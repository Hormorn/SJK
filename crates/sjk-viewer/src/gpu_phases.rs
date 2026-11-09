//! Per-pass GPU timing of the main frame: timestamps between the encoder's sections,
//! resolved on a sampled frame. They are printed with the frame-budget report
//! (`SJK_FRAME_BUDGET` or `SJK_GPU_PHASES`), kept for the low-FPS help
//! ([`crate::fps_help`]) while it watches, and handed section by section to the frame
//! timings readout and log ([`crate::frame_timings`]) while they run. None on, nothing
//! is encoded.
use std::cell::{Cell, RefCell};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const CAPACITY: u32 = 32;

/// Frames between sampled frames for the report and the low-FPS help; the readback of
/// one must finish before the next.
const INTERVAL: u32 = 32;
/// Frames between sampled frames while the frame timings run ([`Profiler::show`]):
/// every frame whose previous readback has landed, about one in two or three.
const SHOWN_INTERVAL: u32 = 1;

/// Timestamp features when the adapter has them; requesting them costs nothing.
pub(crate) fn features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let wanted = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    if adapter.features().contains(wanted) {
        wanted
    } else {
        wgpu::Features::empty()
    }
}

pub(crate) struct Profiler {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    read: wgpu::Buffer,
    nanoseconds_per_tick: f32,
    frame: Cell<u32>,
    active: Cell<bool>,
    index: Cell<u32>,
    names: RefCell<Vec<&'static str>>,
    pending: Cell<bool>,
    mapped: Arc<AtomicBool>,
    /// Requested by environment: every sampled frame is printed.
    print: bool,
    /// The low-FPS help wants samples.
    watch: Cell<bool>,
    /// The last sampled frame's times, until taken.
    sample: Cell<Option<Sample>>,
    /// The frame timings readout or log wants every section's time.
    shown: Cell<bool>,
    /// The frame being sampled, and the last one sampled at [`INTERVAL`]'s pace for the
    /// report and the low-FPS help, which keep that pace while the timings run.
    sampled: Cell<u32>,
    paced: Cell<u32>,
    /// The last sampled frame's sections and their milliseconds, in frame order, and
    /// the whole frame's, until taken ([`Profiler::take_sections`]).
    sections: RefCell<Vec<(&'static str, f32)>>,
    sections_total: Cell<Option<f32>>,
}

/// One sampled frame's GPU time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    /// The whole main frame, milliseconds.
    pub(crate) frame_ms: f32,
    /// SJK's real-time lighting: the sections named `light-...`.
    pub(crate) light_ms: f32,
}

impl Profiler {
    /// Present when the device has timestamps.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) -> Option<Self> {
        let print = std::env::var_os("SJK_FRAME_BUDGET").is_some()
            || std::env::var_os("SJK_GPU_PHASES").is_some();
        let supported = device.features().contains(
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
        );
        if !supported {
            return None;
        }
        let set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("SJK gpu phases"),
            ty: wgpu::QueryType::Timestamp,
            count: CAPACITY,
        });
        let size = u64::from(CAPACITY) * 8;
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK gpu phases resolve"),
            size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let read = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK gpu phases read"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Some(Self {
            set,
            resolve,
            read,
            nanoseconds_per_tick: queue.get_timestamp_period(),
            frame: Cell::new(0),
            active: Cell::new(false),
            index: Cell::new(0),
            names: RefCell::new(Vec::with_capacity(CAPACITY as usize)),
            pending: Cell::new(false),
            mapped: Arc::new(AtomicBool::new(false)),
            print,
            watch: Cell::new(false),
            sample: Cell::new(None),
            shown: Cell::new(false),
            sampled: Cell::new(0),
            paced: Cell::new(0),
            sections: RefCell::new(Vec::with_capacity(CAPACITY as usize)),
            sections_total: Cell::new(None),
        })
    }

    /// Sample frames for [`Self::take_sample`] (`true`) or stop.
    pub(crate) fn watch(&self, on: bool) {
        self.watch.set(on);
        if !on {
            self.sample.set(None);
        }
    }

    /// The last sampled frame's times, once.
    pub(crate) fn take_sample(&self) -> Option<Sample> {
        self.sample.take()
    }

    /// Sample frames for [`Self::take_sections`] as often as the readbacks allow
    /// (`true`), or stop.
    pub(crate) fn show(&self, on: bool) {
        self.shown.set(on);
        if !on {
            self.sections_total.set(None);
        }
    }

    /// The last sampled frame's sections, once: `each` gets every section's name and
    /// milliseconds in frame order (a name can come twice), and the whole frame's
    /// milliseconds are returned.
    pub(crate) fn take_sections(&self, mut each: impl FnMut(&'static str, f32)) -> Option<f32> {
        let total = self.sections_total.take()?;
        for &(name, ms) in self.sections.borrow().iter() {
            each(name, ms);
        }
        Some(total)
    }

    /// Start a frame. Without a readback in flight, every `INTERVAL`th frame is sampled
    /// while printing or watched, and every frame while the timings run.
    pub(crate) fn begin(&self, encoder: &mut wgpu::CommandEncoder) {
        let frame = self.frame.get().wrapping_add(1);
        self.frame.set(frame);
        let interval = if self.shown.get() {
            SHOWN_INTERVAL
        } else {
            INTERVAL
        };
        let active = (self.print || self.watch.get() || self.shown.get())
            && frame.is_multiple_of(interval)
            && !self.pending.get();
        self.active.set(active);
        if !active {
            return;
        }
        self.sampled.set(frame);
        self.index.set(0);
        self.names.borrow_mut().clear();
        self.stamp(encoder);
    }

    /// End the section named `name` here.
    pub(crate) fn mark(&self, encoder: &mut wgpu::CommandEncoder, name: &'static str) {
        if !self.active.get() || self.index.get() >= CAPACITY {
            return;
        }
        self.names.borrow_mut().push(name);
        self.stamp(encoder);
    }

    fn stamp(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.write_timestamp(&self.set, self.index.get());
        self.index.set(self.index.get() + 1);
    }

    /// Resolve the sampled frame's stamps into the readback buffer; call before finishing.
    pub(crate) fn finish(&self, encoder: &mut wgpu::CommandEncoder) {
        if !self.active.get() || self.index.get() < 2 {
            return;
        }
        let count = self.index.get();
        encoder.resolve_query_set(&self.set, 0..count, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.read, 0, u64::from(count) * 8);
        self.pending.set(true);
        self.mapped.store(false, Ordering::Release);
        self.active.set(false);
    }

    /// Map the readback after the sampled frame was submitted; the callback lands later.
    pub(crate) fn after_submit(&self) {
        if !self.pending.get() || self.index.get() < 2 {
            return;
        }
        // `active` is already false; a second call while pending must not map twice.
        if self.index.get() == u32::MAX {
            return;
        }
        let mapped = self.mapped.clone();
        let count = self.index.get();
        self.index.set(u32::MAX);
        self.read
            .slice(..u64::from(count) * 8)
            .map_async(wgpu::MapMode::Read, move |result| {
                if result.is_ok() {
                    mapped.store(true, Ordering::Release);
                }
            });
    }

    /// Print or keep the sampled frame once its readback has landed; safe to call every
    /// frame.
    pub(crate) fn report(&self) {
        if !self.pending.get() || !self.mapped.load(Ordering::Acquire) {
            return;
        }
        let names = self.names.borrow();
        let count = names.len() + 1;
        let mut stamps = [0_u64; CAPACITY as usize];
        {
            let view = self
                .read
                .slice(..(count as u64) * 8)
                .get_mapped_range()
                .expect("mapped readback range");
            stamps[..count].copy_from_slice(bytemuck::cast_slice::<u8, u64>(&view));
        }
        self.read.unmap();
        self.pending.set(false);
        let ms = |a: u64, b: u64| {
            b.saturating_sub(a) as f64 * f64::from(self.nanoseconds_per_tick) * 1e-6
        };
        let total_ms = ms(stamps[0], stamps[count - 1]);
        if self.shown.get() {
            let mut sections = self.sections.borrow_mut();
            sections.clear();
            sections.extend(
                names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| (*name, ms(stamps[i], stamps[i + 1]) as f32)),
            );
            self.sections_total.set(Some(total_ms as f32));
        }
        // The report and the low-FPS help keep their pace while the timings run.
        let sampled = self.sampled.get();
        if self.shown.get() && !paced(sampled, self.paced.get()) {
            return;
        }
        self.paced.set(sampled);
        let light_ms = names
            .iter()
            .enumerate()
            .filter(|(_, name)| name.starts_with("light-"))
            .map(|(i, _)| ms(stamps[i], stamps[i + 1]))
            .sum::<f64>();
        if self.watch.get() {
            self.sample.set(Some(Sample {
                frame_ms: total_ms as f32,
                light_ms: light_ms as f32,
            }));
        }
        if !self.print {
            return;
        }
        let mut line = format!("gpu-phases frame={sampled} total_ms={total_ms:.3}");
        for (i, name) in names.iter().enumerate() {
            line.push_str(&format!(" {name}={:.3}", ms(stamps[i], stamps[i + 1])));
        }
        eprintln!("{line}");
    }
}

/// Whether frame `sampled` comes [`INTERVAL`] or more frames after `last`, the last
/// sample the report and the low-FPS help took: they keep that pace whatever else samples.
fn paced(sampled: u32, last: u32) -> bool {
    sampled.wrapping_sub(last) >= INTERVAL
}

#[cfg(test)]
mod tests {
    use super::*;

    /// While the timings sample every frame they can, the report and the low-FPS help
    /// still take one sample in [`INTERVAL`] frames, across the counter's wrap too.
    #[test]
    fn the_report_keeps_its_pace_while_the_timings_sample_more() {
        let mut last = 0;
        let taken = (1..=320_u32)
            .filter(|frame| {
                let take = paced(*frame, last);
                if take {
                    last = *frame;
                }
                take
            })
            .count();
        assert_eq!(taken, 10);
        assert!(paced(5, u32::MAX - 40));
        assert!(!paced(5, u32::MAX - 10));
    }
}
