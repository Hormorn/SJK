//! Bounded, load-only mip preparation. Uploads keep the material forge's texture keys.
use super::{PendingMaterial, forge::Forge};
use image::RgbaImage;
use std::{collections::HashSet, error::Error, sync::Arc, time::Instant};

use super::filtering::mips;

// Bound prepared chains per batch, rather than retaining every map texture twice.
// A single larger animated texture is handled alone, as in the serial path.
const BATCH_BYTES: u64 = 64 << 20;
const MAX_WORKERS: usize = 4;

/// Each texture key a world build prepares, with its images.
#[cfg(test)]
pub(crate) type Captured = Vec<(String, Vec<Arc<RgbaImage>>)>;

/// The textures world builds prepare, kept while `Some` for the compression measurements
/// (`world_shot_texture_compression.rs`).
#[cfg(test)]
pub(crate) static CAPTURED: std::sync::Mutex<Option<Captured>> = std::sync::Mutex::new(None);

struct Job<'a> {
    key: &'a str,
    images: &'a [Arc<RgbaImage>],
    width: u32,
    height: u32,
    /// The texture detail it takes ([`mips::picmip_for`]).
    picmip: u32,
    bytes: u64,
}

impl<'a> Job<'a> {
    fn new(key: &'a str, images: &'a [Arc<RgbaImage>]) -> Self {
        let (width, height) = mips::extent(images);
        let bytes = (0..mips::levels(width, height))
            .map(|level| {
                u64::from((width >> level).max(1)) * u64::from((height >> level).max(1)) * 4
            })
            .sum::<u64>()
            .saturating_mul(images.len() as u64);
        Self {
            key,
            images,
            width,
            height,
            picmip: mips::picmip_for(key),
            bytes,
        }
    }
}

/// Prepare each unique texture once before stage assembly, with unchanged mip pixels.
/// Workers only resize pixels (and encode them, with `r_textureCompression`); GPU
/// creation and forge mutation stay on the installer.
pub(super) fn upload(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    forge: &mut Forge,
    materials: &[PendingMaterial],
) -> Result<(), Box<dyn Error>> {
    if !forge.filtering.mipmapped() {
        return Ok(());
    }
    let started = Instant::now();
    let mut seen = HashSet::new();
    let mut jobs = Vec::new();
    for stage in materials.iter().flat_map(|material| &material.stages) {
        for (key, images) in
            std::iter::once((stage.primary_key.as_str(), stage.primary_pixels.as_slice())).chain(
                stage
                    .secondary_key
                    .as_deref()
                    .zip(stage.secondary_pixels.as_deref()),
            )
        {
            if !forge.texture_cache.contains_key(key)
                && !key.starts_with(super::videos::KEY_PREFIX)
                // A model's normal map is uploaded linear by the forge itself.
                && !key.starts_with(super::material_maps::models::KEY_PREFIX)
                && seen.insert(key)
            {
                // The last map's world, still alive during a map change, may hold it.
                if let Some(view) = mips::find_shared(images, mips::picmip_for(key)) {
                    forge.texture_cache.insert(key.to_owned(), view);
                } else {
                    jobs.push(Job::new(key, images));
                }
            }
        }
    }
    #[cfg(test)]
    if let Some(captured) = CAPTURED.lock().expect("capture lock").as_mut() {
        captured.extend(
            jobs.iter()
                .map(|job| (job.key.to_owned(), job.images.to_vec())),
        );
    }
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .saturating_sub(1)
        .clamp(1, MAX_WORKERS);
    let mut compressed = 0_usize;
    let mut remaining = jobs.as_slice();
    while !remaining.is_empty() {
        let mut bytes = 0_u64;
        let mut count = 0;
        for job in remaining {
            if count > 0 && bytes.saturating_add(job.bytes) > BATCH_BYTES {
                break;
            }
            bytes = bytes.saturating_add(job.bytes);
            count += 1;
        }
        let (batch, tail) = remaining.split_at(count);
        let next = std::sync::atomic::AtomicUsize::new(0);
        std::thread::scope(|scope| -> Result<(), Box<dyn Error>> {
            let (sender, receiver) = std::sync::mpsc::sync_channel(workers);
            for _ in 0..workers.min(batch.len()) {
                let sender = sender.clone();
                let next = &next;
                scope.spawn(move || {
                    while let Some(job) =
                        batch.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                    {
                        let levels = mips::cache::prepare_levels(
                            job.images, job.width, job.height, job.picmip,
                        );
                        if sender.send((job, levels)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(sender);
            for (job, levels) in receiver {
                compressed += usize::from(matches!(levels, mips::Levels::Blocks(_)));
                let texture = mips::upload_shared(
                    device,
                    queue,
                    job.images,
                    job.width,
                    job.height,
                    job.picmip,
                    || levels,
                )?;
                forge.texture_cache.insert(job.key.to_owned(), texture);
            }
            Ok(())
        })?;
        remaining = tail;
    }
    crate::log::progress(format_args!(
        "texture preparation: {} arrays{}, {workers} workers, {:.1} ms",
        jobs.len(),
        if mips::compress::active() {
            format!(" ({compressed} block-compressed)")
        } else {
            String::new()
        },
        started.elapsed().as_secs_f64() * 1_000.0
    ));
    Ok(())
}
