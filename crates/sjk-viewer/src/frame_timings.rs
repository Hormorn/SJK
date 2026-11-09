//! The frame timings readout and log (Sol's request, 09/11/2026, to work on the
//! graphics settings and make them cheaper): how long the graphics card spends on each
//! section of the frame ([`crate::gpu_phases`], timestamps between the encoder's
//! sections) and the processor on each host phase ([`crate::frame_pacing::budget`]).
//!
//! `r_drawTimings` shows them in a panel at the top right of the screen (the HUD edit
//! mode moves it), averaged over half a second with each one's worst frame: 1 the graphics card's sections, 2 the
//! processor's phases too. `r_logTimings` writes them averaged over each second to
//! `logs/timings.log` beside the configuration (the session log's folder), with the
//! window, the map, the graphics settings (every Video and renderer row) and each
//! change to them, and `perfmark` lines, so a run that turns settings on and off reads
//! back as before and after. A session's first log keeps the one before as
//! `logs/previous-timings.log`.
//!
//! Both off, the profiler samples as before and nothing here runs past two cvar reads.
//! On, it samples every frame whose previous readback has landed; nothing allocates per
//! frame (the panel's text has fixed storage, a log line is formatted once a second).

use crate::frame_pacing::budget::{self, NAMES};
use crate::menu::sjk::{Frame, color, text};
use crate::menu_widgets::{MenuCanvas, TextFamily};
use sjk_ui::{Color, DrawCommand, FontWeight, Rect, TextAlign};
use std::fmt::{self, Write as _};
use std::fs::{File, OpenOptions};
use std::io::{LineWriter, Seek, SeekFrom, Write as _};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The readout: 0 off, 1 the graphics card's sections, 2 the processor's phases too.
pub(crate) const DRAW_CVAR: &str = "r_drawTimings";
/// The log, on or off.
pub(crate) const LOG_CVAR: &str = "r_logTimings";
/// The log's file in the session log's folder, and the one before it.
const FILE: &str = "timings.log";
const PREVIOUS: &str = "previous-timings.log";
/// Past this many bytes the log starts again, as the session log does.
const FILE_MAX: u64 = 16 * 1024 * 1024;

/// How long the panel's numbers are averaged over, and the log's.
const SHOWN_PERIOD: Duration = Duration::from_millis(500);
const LOG_PERIOD: Duration = Duration::from_secs(1);
/// Sections a frame can have: the profiler's timestamps less one.
const SECTIONS: usize = 32;
/// The processor's heaviest phases the panel lists.
const CPU_ROWS: usize = 8;
/// The panel leaves out sections that average less than this, in milliseconds.
const SHOWN_MIN_MS: f32 = 0.005;
/// The log leaves out sections and phases that average less than this.
const LOGGED_MIN_MS: f64 = 0.0005;

/// The panel's place and rows, in window pixels at 1080 lines ([`Frame::s`]).
const PANEL_MARGIN: f32 = 16.0;
const PANEL_TOP: f32 = 150.0;
const PANEL_WIDTH: f32 = 360.0;
const PAD: f32 = 14.0;
const TITLE_HEIGHT: f32 = 30.0;
const LEGEND_HEIGHT: f32 = 16.0;
const HEADING_HEIGHT: f32 = 24.0;
const ROW_HEIGHT: f32 = 17.0;
const GAP: f32 = 8.0;
/// The mean's and the worst's right edges, from the panel's right edge.
const MEAN_RIGHT: f32 = 66.0;
const MAX_RIGHT: f32 = PAD;
/// A section at least this share of the frame is named in gold.
const HEAVY_SHARE: f32 = 0.25;
/// The panel's text runs at most (three a row) and its draw commands (every run is
/// one, with the glass and a bar a row).
const PANEL_TEXTS: usize = 160;
const PANEL_DRAWS: usize = 256;

/// The readout's level: 0 off, 1 the graphics card's sections, 2 the processor's too.
fn draw_level(console: Option<&crate::console::ViewerConsole>) -> i64 {
    console
        .and_then(|console| console.integer_cvar(DRAW_CVAR))
        .unwrap_or(0)
        .clamp(0, 2)
}

/// One section's or phase's mean and worst frame, in milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Row {
    pub(crate) name: &'static str,
    pub(crate) mean: f32,
    pub(crate) max: f32,
}

/// What the panel shows: the last half second's means.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Readout {
    gpu: [Row; SECTIONS],
    gpu_len: usize,
    gpu_total: Row,
    gpu_samples: u32,
    cpu: [Row; CPU_ROWS],
    cpu_len: usize,
    cpu_total: Row,
    fps: f32,
    ready: bool,
}

impl Readout {
    /// The graphics card's sections, in frame order.
    fn gpu(&self) -> &[Row] {
        &self.gpu[..self.gpu_len]
    }

    /// The processor's heaviest phases, heaviest first.
    fn cpu(&self) -> &[Row] {
        &self.cpu[..self.cpu_len]
    }
}

/// One section's sums over a window.
#[derive(Clone, Copy, Debug, Default)]
struct Section {
    name: &'static str,
    sum: f64,
    max: f64,
    /// This sample's time, a name that comes twice in a frame summed.
    frame: f64,
}

/// Sums over the frames of one period, in fixed storage.
#[derive(Clone, Copy, Debug)]
struct Window {
    started: Option<Instant>,
    sections: [Section; SECTIONS],
    sections_len: usize,
    gpu_samples: u32,
    gpu_sum: f64,
    gpu_max: f64,
    phases: [f64; NAMES.len()],
    phases_max: [f64; NAMES.len()],
    frames: u32,
    work_sum: f64,
    work_max: f64,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            started: None,
            sections: [Section::default(); SECTIONS],
            sections_len: 0,
            gpu_samples: 0,
            gpu_sum: 0.0,
            gpu_max: 0.0,
            phases: [0.0; NAMES.len()],
            phases_max: [0.0; NAMES.len()],
            frames: 0,
            work_sum: 0.0,
            work_max: 0.0,
        }
    }
}

impl Window {
    /// Start counting at `now`, the sums cleared; the sections keep their order.
    fn restart(&mut self, now: Instant) {
        let sections = self.sections;
        let len = self.sections_len;
        *self = Self::default();
        for (slot, section) in self.sections.iter_mut().zip(&sections[..len]) {
            slot.name = section.name;
        }
        self.sections_len = len;
        self.started = Some(now);
    }

    /// Whether `period` has passed since the window started; it starts at `now` if
    /// it had not.
    fn due(&mut self, now: Instant, period: Duration) -> bool {
        let started = *self.started.get_or_insert(now);
        now.duration_since(started) >= period
    }

    /// One frame's host phases.
    fn add_cpu(&mut self, sample: &budget::Sample) {
        self.frames += 1;
        self.work_sum += sample.work;
        self.work_max = self.work_max.max(sample.work);
        for ((sum, max), ms) in self
            .phases
            .iter_mut()
            .zip(self.phases_max.iter_mut())
            .zip(sample.phases)
        {
            *sum += ms;
            *max = max.max(ms);
        }
    }

    /// One sampled frame's sections, in frame order, and its whole time.
    fn add_gpu(&mut self, sections: &[(&'static str, f32)], total: f32) {
        self.gpu_samples += 1;
        self.gpu_sum += f64::from(total);
        self.gpu_max = self.gpu_max.max(f64::from(total));
        for section in &mut self.sections[..self.sections_len] {
            section.frame = 0.0;
        }
        for &(name, ms) in sections {
            let found = self.sections[..self.sections_len]
                .iter()
                .position(|section| section.name == name);
            let index = match found {
                Some(index) => index,
                None if self.sections_len < SECTIONS => {
                    self.sections[self.sections_len].name = name;
                    self.sections_len += 1;
                    self.sections_len - 1
                }
                None => continue,
            };
            self.sections[index].frame += f64::from(ms);
        }
        for section in &mut self.sections[..self.sections_len] {
            section.sum += section.frame;
            section.max = section.max.max(section.frame);
        }
    }

    /// The window's frames per second at `now`.
    fn fps(&self, now: Instant) -> f32 {
        let seconds = self
            .started
            .map_or(0.0, |started| now.duration_since(started).as_secs_f64());
        if seconds > 0.0 {
            (f64::from(self.frames) / seconds) as f32
        } else {
            0.0
        }
    }

    /// The sections' means, in frame order, those under `minimum` left out.
    fn gpu_rows(&self, minimum: f64) -> impl Iterator<Item = Row> + '_ {
        let samples = f64::from(self.gpu_samples.max(1));
        let sampled = self.gpu_samples > 0;
        self.sections[..self.sections_len]
            .iter()
            .filter(move |section| sampled && section.sum / samples >= minimum)
            .map(move |section| Row {
                name: section.name,
                mean: (section.sum / samples) as f32,
                max: section.max as f32,
            })
    }

    /// The host phases' means, heaviest first, those under `minimum` left out.
    fn cpu_rows(&self, minimum: f64) -> impl Iterator<Item = Row> + '_ {
        let frames = f64::from(self.frames.max(1));
        let counted = self.frames > 0;
        let mut order: [usize; NAMES.len()] = std::array::from_fn(|index| index);
        order.sort_by(|a, b| self.phases[*b].total_cmp(&self.phases[*a]));
        order
            .into_iter()
            .filter(move |index| counted && self.phases[*index] / frames >= minimum)
            .map(move |index| Row {
                name: NAMES[index],
                mean: (self.phases[index] / frames) as f32,
                max: self.phases_max[index] as f32,
            })
    }

    fn gpu_total(&self) -> Row {
        Row {
            name: "gpu",
            mean: (self.gpu_sum / f64::from(self.gpu_samples.max(1))) as f32,
            max: self.gpu_max as f32,
        }
    }

    fn cpu_total(&self) -> Row {
        Row {
            name: "cpu",
            mean: (self.work_sum / f64::from(self.frames.max(1))) as f32,
            max: self.work_max as f32,
        }
    }

    /// The panel's numbers for this window.
    fn readout(&self, now: Instant) -> Readout {
        let mut readout = Readout {
            gpu_total: self.gpu_total(),
            gpu_samples: self.gpu_samples,
            cpu_total: self.cpu_total(),
            fps: self.fps(now),
            ready: true,
            ..Readout::default()
        };
        for row in self.gpu_rows(f64::from(SHOWN_MIN_MS)) {
            readout.gpu[readout.gpu_len] = row;
            readout.gpu_len += 1;
        }
        for row in self.cpu_rows(f64::from(SHOWN_MIN_MS)).take(CPU_ROWS) {
            readout.cpu[readout.cpu_len] = row;
            readout.cpu_len += 1;
        }
        readout
    }

    /// The log's line for this window, `seconds` into the log.
    fn log_line(&self, seconds: f64, now: Instant, line: &mut String) {
        line.clear();
        let gpu = self.gpu_total();
        let cpu = self.cpu_total();
        let _ = write!(
            line,
            "t={seconds:.1} fps={:.0} frames={} cpu={:.3}/{:.3}",
            self.fps(now),
            self.frames,
            cpu.mean,
            cpu.max
        );
        if self.gpu_samples > 0 {
            let _ = write!(
                line,
                " gpu={:.3}/{:.3} samples={} | gpu:",
                gpu.mean, gpu.max, self.gpu_samples
            );
            for row in self.gpu_rows(LOGGED_MIN_MS) {
                let _ = write!(line, " {}={:.3}", row.name, row.mean);
            }
        }
        line.push_str(" | cpu:");
        for row in self.cpu_rows(LOGGED_MIN_MS) {
            let _ = write!(line, " {}={:.3}", row.name, row.mean);
        }
    }
}

/// What the log last wrote of the context, to write each change once.
#[derive(Debug, Default)]
struct Context {
    /// The console's archive stamp the settings were last read at.
    revision: Option<u64>,
    settings: Vec<(&'static str, String)>,
    preset: &'static str,
    size: [u32; 2],
    map: String,
}

/// The lines that say how `now` differs from `before`: every setting the first time,
/// then `set <cvar> <old> -> <new>` for each one that changed.
fn settings_lines(
    before: &[(&'static str, String)],
    now: &[(&'static str, String)],
    mut line: impl FnMut(fmt::Arguments<'_>),
) {
    if before.is_empty() {
        let mut all = String::with_capacity(now.len() * 16);
        for (cvar, value) in now {
            let _ = write!(all, " {cvar}={value}");
        }
        line(format_args!("settings{all}"));
        return;
    }
    for (cvar, value) in now {
        let old = before
            .iter()
            .find(|(name, _)| name == cvar)
            .map(|(_, value)| value.as_str());
        if old != Some(value.as_str()) {
            line(format_args!(
                "set {cvar} {} -> {value}",
                old.unwrap_or("(none)")
            ));
        }
    }
}

/// The readout's and the log's state, kept with the renderer.
pub(crate) struct State {
    shown: Window,
    logged: Window,
    readout: Readout,
    /// Whether the readout or the log ran last frame, and the log.
    running: bool,
    logging: bool,
    /// The log's file opened when it was turned on.
    log_open: bool,
    /// The panel was laid out this frame.
    drawn: bool,
    /// Where it was, in pixels (the HUD edit mode's box).
    rect: Option<Rect>,
    log_started: Option<Instant>,
    context: Context,
    line: String,
    panel: MenuCanvas,
}

impl State {
    /// The panel's shapes, after [`crate::GpuState::append_frame_timings`] drew it.
    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.panel.draw_list()
    }

    /// Whether the panel is drawn this frame: the 2D pass then runs for it even with
    /// no session, menu or console (a map explored offline).
    pub(crate) fn drawn(&self) -> bool {
        self.drawn
    }

    /// The panel's glass this frame, when drawn.
    pub(crate) fn panel_rect(&self) -> Option<Rect> {
        self.rect.filter(|_| self.drawn)
    }

    /// Whether the panel has a half second's numbers (world shots).
    #[cfg(test)]
    pub(crate) fn readout_ready(&self) -> bool {
        self.readout.ready
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            shown: Window::default(),
            logged: Window::default(),
            readout: Readout::default(),
            running: false,
            logging: false,
            log_open: false,
            drawn: false,
            rect: None,
            log_started: None,
            context: Context::default(),
            line: String::with_capacity(1024),
            panel: MenuCanvas::with_capacities(PANEL_TEXTS, 48, PANEL_DRAWS),
        }
    }
}

/// The log's open file.
struct LogFile {
    file: LineWriter<File>,
    written: u64,
}

static LOG: Mutex<Option<LogFile>> = Mutex::new(None);
/// The session's first log keeps the last one as [`PREVIOUS`]; later ones add to it.
static ROTATED: AtomicBool = AtomicBool::new(false);

fn log() -> std::sync::MutexGuard<'static, Option<LogFile>> {
    LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Open the log in the session log's folder; its path, or `None` when there is no
/// folder (tests, a configuration that could not be written) or it cannot be opened.
fn open_log() -> Option<PathBuf> {
    let dir = crate::session_log::folder()?;
    let path = dir.join(FILE);
    let file = if ROTATED.swap(true, Ordering::Relaxed) {
        OpenOptions::new().create(true).append(true).open(&path)
    } else {
        if path.exists() {
            let _ = std::fs::rename(&path, dir.join(PREVIOUS));
        }
        File::create(&path)
    }
    .ok()?;
    let written = file.metadata().map_or(0, |metadata| metadata.len());
    *log() = Some(LogFile {
        file: LineWriter::new(file),
        written,
    });
    Some(path)
}

fn close_log() {
    *log() = None;
}

/// Write one line to the frame timings log while it is open (a `perfmark` too).
pub(crate) fn note(arguments: fmt::Arguments<'_>) {
    let mut guard = log();
    let Some(log) = guard.as_mut() else {
        return;
    };
    if log.written > FILE_MAX {
        let restarted = log.file.get_mut().set_len(0).and_then(|()| {
            log.file.get_mut().seek(SeekFrom::Start(0))?;
            writeln!(log.file, "[earlier lines of this log left out: over 16 MB]")
        });
        log.written = 0;
        if restarted.is_err() {
            *guard = None;
            return;
        }
    }
    let line = arguments.to_string();
    if writeln!(log.file, "{line}").is_err() {
        *guard = None;
        return;
    }
    log.written += line.len() as u64 + 1;
}

impl crate::GpuState {
    /// Once a rendered frame, after its host phases are recorded: take its times into
    /// the readout's and the log's windows, and publish or write each window as it ends.
    pub(crate) fn observe_frame_timings(&mut self) {
        let console = self.console.as_ref();
        let level = draw_level(console);
        let logging = console
            .and_then(|console| console.bool_cvar(LOG_CVAR))
            .unwrap_or(false);
        let running = level > 0 || logging;
        if let Some(phases) = &self.gpu_phases {
            phases.show(running);
        }
        if logging != self.frame_timings.logging {
            self.switch_timings_log(logging);
        }
        let state = &mut self.frame_timings;
        if !running {
            if std::mem::take(&mut state.running) {
                state.readout = Readout::default();
                state.shown = Window::default();
            }
            return;
        }
        state.running = true;
        let now = Instant::now();
        let mut sections = [("", 0.0_f32); SECTIONS];
        let mut count = 0;
        let gpu_total = self.gpu_phases.as_ref().and_then(|phases| {
            phases.take_sections(|name, ms| {
                if count < SECTIONS {
                    sections[count] = (name, ms);
                    count += 1;
                }
            })
        });
        let sample = self.frame_pacer.last;
        for window in [&mut state.shown, &mut state.logged] {
            window.started.get_or_insert(now);
            window.add_cpu(&sample);
            if let Some(total) = gpu_total {
                window.add_gpu(&sections[..count], total);
            }
        }
        if state.shown.due(now, SHOWN_PERIOD) {
            state.readout = state.shown.readout(now);
            state.shown.restart(now);
        }
        if logging && state.logged.due(now, LOG_PERIOD) {
            self.write_timings_context();
            let state = &mut self.frame_timings;
            let seconds = state
                .log_started
                .map_or(0.0, |started| now.duration_since(started).as_secs_f64());
            state.logged.log_line(seconds, now, &mut state.line);
            note(format_args!("{}", state.line));
            state.logged.restart(now);
        }
    }

    /// Open or close the log as `r_logTimings` turns on or off, and say where it is.
    fn switch_timings_log(&mut self, on: bool) {
        self.frame_timings.logging = on;
        self.frame_timings.log_open = false;
        if !on {
            note(format_args!("log off"));
            close_log();
            return;
        }
        let path = open_log();
        self.frame_timings.log_open = path.is_some();
        let message = match &path {
            Some(path) => format!("Frame timings log: {}", path.display()),
            None => "^3Frame timings log: no logs folder to write it in".to_owned(),
        };
        crate::log::progress(format_args!("{message}"));
        if let Some(console) = self.console.as_mut() {
            console.push_log(message);
        }
        let state = &mut self.frame_timings;
        state.context = Context::default();
        state.log_started = Some(Instant::now());
        state.logged.restart(Instant::now());
        if path.is_none() {
            return;
        }
        note(format_args!(
            "SJK {} frame timings log {}: one line a second, mean milliseconds (totals \
             mean/worst); gpu: the graphics card's sections in frame order, cpu: the \
             processor's phases heaviest first",
            crate::build_info::VERSION,
            crate::session_log::now_stamp()
        ));
        note(format_args!(
            "adapter {} ({}), gpu timestamps {}",
            self.adapter_name,
            self.adapter_backend,
            if self.gpu_phases.is_some() {
                "yes"
            } else {
                "no"
            }
        ));
    }

    /// Before a log line: the window's size, the map and the graphics settings, each
    /// time one changed.
    fn write_timings_context(&mut self) {
        let size = [self.configuration.width, self.configuration.height];
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let context = &mut self.frame_timings.context;
        if context.size != size {
            context.size = size;
            note(format_args!("window {}x{}", size[0], size[1]));
        }
        if context.map != map {
            context.map.clear();
            context.map.push_str(map);
            note(format_args!(
                "map {}{}",
                if map.is_empty() { "(none)" } else { map },
                if self.is_menu_world { " (menu)" } else { "" }
            ));
        }
        let Some(console) = self.console.as_ref() else {
            return;
        };
        let revision = console.archive_revision();
        if context.revision == Some(revision) {
            return;
        }
        context.revision = Some(revision);
        let preset = crate::graphics_quality::Level::current(console)
            .map_or("Custom", crate::graphics_quality::Level::label);
        if context.preset != preset {
            context.preset = preset;
            note(format_args!("preset {preset}"));
        }
        let mut seen: Vec<&'static str> = Vec::new();
        let settings: Vec<(&'static str, String)> = crate::settings::graphics_cvars()
            .filter(|cvar| {
                let first = !seen.iter().any(|seen| seen.eq_ignore_ascii_case(cvar));
                seen.push(cvar);
                first
            })
            .filter_map(|cvar| console.cvar(cvar).map(|value| (cvar, value.as_text())))
            .collect();
        settings_lines(&context.settings, &settings, note);
        context.settings = settings;
    }

    /// Draw the panel when `r_drawTimings` asks for it and nothing covers the frame;
    /// whether it was drawn, so its shapes go into the frame's layers.
    pub(crate) fn append_frame_timings(&mut self, viewport: [f32; 2], covered: bool) -> bool {
        let level = draw_level(self.console.as_ref());
        self.frame_timings.drawn = level > 0 && !covered;
        if !self.frame_timings.drawn {
            return false;
        }
        let logging = self.frame_timings.log_open;
        let gpu = self.gpu_phases.is_some();
        let offset = self
            .hud
            .offsets()
            .pixels(crate::hud::placement::Element::Timings, viewport);
        let state = &mut self.frame_timings;
        state.rect = Some(build_panel(
            &mut state.panel,
            &state.readout,
            viewport,
            offset,
            level,
            gpu,
            logging,
        ));
        let target = crate::ingame_menu::sjk_view::text_target(
            &mut self.game_fonts,
            &mut self.text_vertices,
            &self.ui_font,
        );
        target.append(&self.frame_timings.panel, viewport);
        true
    }
}

/// The legend's count of the frames the graphics card's times are from, when there are.
struct Sampled(Option<u32>);

impl fmt::Display for Sampled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(frames) => write!(f, " ({frames} GPU frames)"),
            None => Ok(()),
        }
    }
}

/// The panel's height for these rows, in pixels at 1080 lines.
fn panel_height(gpu_rows: usize, cpu_rows: Option<usize>, footer: bool) -> f32 {
    let mut height =
        PAD + TITLE_HEIGHT + LEGEND_HEIGHT + HEADING_HEIGHT + ROW_HEIGHT * gpu_rows.max(1) as f32;
    if let Some(rows) = cpu_rows {
        height += GAP + HEADING_HEIGHT + ROW_HEIGHT * rows.max(1) as f32;
    }
    if footer {
        height += GAP + ROW_HEIGHT;
    }
    height + PAD
}

/// Lay out the panel: its glass, the title and frame rate, the graphics card's
/// sections (or why there are none), the processor's heaviest phases at level 2 or
/// without the graphics card's, and the log's file while it writes. It hangs from the
/// top right corner, moved by `offset` pixels (the HUD edit mode's); returns its glass.
fn build_panel(
    ui: &mut MenuCanvas,
    readout: &Readout,
    viewport: [f32; 2],
    offset: [f32; 2],
    level: i64,
    gpu: bool,
    logging: bool,
) -> Rect {
    let s = Frame::new(viewport).s;
    let show_cpu = level >= 2 || !gpu;
    let gpu_rows = if gpu { readout.gpu().len() } else { 1 };
    let cpu_rows = show_cpu.then(|| readout.cpu().len());
    let height = panel_height(gpu_rows, cpu_rows, logging);
    let width = PANEL_WIDTH * s;
    let left = viewport[0] - (PANEL_MARGIN + PANEL_WIDTH) * s + offset[0];
    let right = left + width;
    let mut y = PANEL_TOP * s + offset[1];
    ui.begin_transparent(viewport);
    let glass = Rect::new(left, y, width, height * s);
    let _ = ui.draw_list_mut().push(DrawCommand::RoundedRect {
        rect: glass,
        radius: 12.0 * s,
        color: Color::new(0.024, 0.039, 0.078, 0.82),
    });
    let _ = ui.draw_list_mut().push(DrawCommand::Border {
        rect: glass,
        radius: 12.0 * s,
        width: 1.5 * s,
        color: color::alpha(color::HOLO, 0.35),
    });
    y += PAD * s;
    let title = Rect::new(left + PAD * s, y, width - PAD * 2.0 * s, TITLE_HEIGHT * s);
    text(
        ui,
        TextFamily::Display,
        format_args!("FRAME TIMINGS"),
        title,
        17.0 * s,
        color::GOLD_BRIGHT,
        FontWeight::Semibold,
        TextAlign::Start,
    );
    if readout.ready {
        text(
            ui,
            TextFamily::Code,
            format_args!("{:.0} fps", readout.fps),
            title,
            13.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::End,
        );
    }
    y += TITLE_HEIGHT * s;
    let legend = Rect::new(left + PAD * s, y, width - PAD * 2.0 * s, LEGEND_HEIGHT * s);
    // The graphics card's numbers are from the frames sampled, about one in two or three.
    let sampled = (gpu && readout.ready).then_some(readout.gpu_samples);
    text(
        ui,
        TextFamily::Body,
        format_args!("ms over half a second{}", Sampled(sampled),),
        legend,
        11.5 * s,
        color::QUIET,
        FontWeight::Regular,
        TextAlign::Start,
    );
    column_labels(ui, legend.y, legend.height, right, s);
    y += LEGEND_HEIGHT * s;
    let heading = |ui: &mut MenuCanvas, y: f32, label: &str, total: Option<Row>| {
        let rect = Rect::new(left + PAD * s, y, width - PAD * 2.0 * s, HEADING_HEIGHT * s);
        text(
            ui,
            TextFamily::Display,
            format_args!("{label}"),
            rect,
            15.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Start,
        );
        if let Some(total) = total {
            numbers(ui, rect.y, rect.height, right, s, total, color::TEXT);
        }
    };
    let gpu_total = (gpu && readout.ready).then_some(readout.gpu_total);
    heading(ui, y, "GPU", gpu_total);
    y += HEADING_HEIGHT * s;
    if !gpu {
        line(
            ui,
            left,
            y,
            width,
            s,
            format_args!("No timestamps on this graphics driver"),
            color::MUTED,
        );
        y += ROW_HEIGHT * s;
    } else if readout.gpu().is_empty() {
        line(
            ui,
            left,
            y,
            width,
            s,
            format_args!("Measuring..."),
            color::MUTED,
        );
        y += ROW_HEIGHT * s;
    }
    if gpu {
        for row in readout.gpu() {
            let share = row.mean / readout.gpu_total.mean.max(0.001);
            section_row(ui, left, y, width, s, *row, share);
            y += ROW_HEIGHT * s;
        }
    }
    if show_cpu {
        y += GAP * s;
        let cpu_total = readout.ready.then_some(readout.cpu_total);
        heading(ui, y, "CPU", cpu_total);
        y += HEADING_HEIGHT * s;
        if readout.cpu().is_empty() {
            line(
                ui,
                left,
                y,
                width,
                s,
                format_args!("Measuring..."),
                color::MUTED,
            );
            y += ROW_HEIGHT * s;
        }
        for row in readout.cpu() {
            let share = row.mean / readout.cpu_total.mean.max(0.001);
            section_row(ui, left, y, width, s, *row, share);
            y += ROW_HEIGHT * s;
        }
    }
    if logging {
        y += GAP * s;
        line(
            ui,
            left,
            y,
            width,
            s,
            format_args!("Logging to logs/{FILE}"),
            color::QUIET,
        );
    }
    ui.finish(0);
    glass
}

/// The mean's and the worst's columns on a line at `y`, `height` tall, from `right`,
/// the panel's right edge; their text is set against the rectangles' right edges.
fn columns(y: f32, height: f32, right: f32, s: f32) -> [Rect; 2] {
    [
        Rect::new(right - 160.0 * s, y, (160.0 - MEAN_RIGHT) * s, height),
        Rect::new(right - 60.0 * s, y, (60.0 - MAX_RIGHT) * s, height),
    ]
}

/// The columns' names over the numbers.
fn column_labels(ui: &mut MenuCanvas, y: f32, height: f32, right: f32, s: f32) {
    for (rect, label) in columns(y, height, right, s)
        .into_iter()
        .zip(["mean", "worst"])
    {
        text(
            ui,
            TextFamily::Body,
            format_args!("{label}"),
            rect,
            11.5 * s,
            color::QUIET,
            FontWeight::Regular,
            TextAlign::End,
        );
    }
}

/// A row's mean and worst at their right edges, from `right`, the panel's.
fn numbers(ui: &mut MenuCanvas, y: f32, height: f32, right: f32, s: f32, row: Row, mean: Color) {
    let [mean_rect, max_rect] = columns(y, height, right, s);
    text(
        ui,
        TextFamily::Code,
        format_args!("{:.2}", row.mean),
        mean_rect,
        12.5 * s,
        mean,
        FontWeight::Regular,
        TextAlign::End,
    );
    text(
        ui,
        TextFamily::Code,
        format_args!("{:.2}", row.max),
        max_rect,
        12.5 * s,
        color::QUIET,
        FontWeight::Regular,
        TextAlign::End,
    );
}

/// One section's or phase's row: a bar for its share of the frame behind its name,
/// gold when it is a heavy one, then its mean and worst.
fn section_row(ui: &mut MenuCanvas, left: f32, y: f32, width: f32, s: f32, row: Row, share: f32) {
    let inner = width - PAD * 2.0 * s;
    let bar = Rect::new(
        left + PAD * s,
        y + 2.0 * s,
        inner * share.clamp(0.0, 1.0),
        (ROW_HEIGHT - 4.0) * s,
    );
    let _ = ui.draw_list_mut().push(DrawCommand::SolidRect {
        rect: bar,
        color: color::alpha(color::HOLO, 0.14),
    });
    let name = if share >= HEAVY_SHARE {
        color::GOLD_BRIGHT
    } else {
        color::MUTED
    };
    let rect = Rect::new(left + PAD * s, y, inner, ROW_HEIGHT * s);
    text(
        ui,
        TextFamily::Code,
        format_args!("{}", row.name),
        rect,
        12.5 * s,
        name,
        FontWeight::Regular,
        TextAlign::Start,
    );
    numbers(ui, y, ROW_HEIGHT * s, left + width, s, row, color::TEXT);
}

/// A line of small text across the panel.
fn line(
    ui: &mut MenuCanvas,
    left: f32,
    y: f32,
    width: f32,
    s: f32,
    value: fmt::Arguments<'_>,
    colour: Color,
) {
    let rect = Rect::new(left + PAD * s, y, width - PAD * 2.0 * s, ROW_HEIGHT * s);
    text(
        ui,
        TextFamily::Body,
        value,
        rect,
        12.5 * s,
        colour,
        FontWeight::Regular,
        TextAlign::Start,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu_sample(work: f64, phases: &[(budget::Phase, f64)]) -> budget::Sample {
        let mut sample = budget::Sample {
            work,
            ..budget::Sample::default()
        };
        for (phase, ms) in phases {
            sample.phases[*phase as usize] = *ms;
        }
        sample
    }

    /// Sections keep their frame order, a name that comes twice in a frame counts as
    /// one section, and the worst is the worst frame's, not the worst piece's.
    #[test]
    fn sections_average_over_the_samples_in_frame_order() {
        let mut window = Window::default();
        window.add_gpu(&[("uploads", 0.5), ("water", 1.0), ("water", 1.0)], 2.5);
        window.add_gpu(&[("uploads", 1.5)], 1.5);
        let rows: Vec<Row> = window.gpu_rows(0.0).collect();
        assert_eq!(
            rows,
            [
                Row {
                    name: "uploads",
                    mean: 1.0,
                    max: 1.5
                },
                Row {
                    name: "water",
                    mean: 1.0,
                    max: 2.0
                },
            ]
        );
        assert_eq!(window.gpu_total().mean, 2.0);
        assert_eq!(window.gpu_total().max, 2.5);
    }

    /// A new window starts its sums again but keeps the sections' order, so the
    /// panel's rows do not jump about from one half second to the next.
    #[test]
    fn a_new_window_keeps_the_order_and_drops_the_sums() {
        let mut window = Window::default();
        window.add_gpu(&[("views", 0.2), ("glow", 0.3)], 0.5);
        window.restart(Instant::now());
        assert_eq!(window.gpu_rows(0.0).count(), 0);
        window.add_gpu(&[("glow", 0.4), ("views", 0.1)], 0.5);
        let names: Vec<&str> = window.gpu_rows(0.0).map(|row| row.name).collect();
        assert_eq!(names, ["views", "glow"]);
    }

    /// The processor's phases come heaviest first, the ones under the floor left out.
    #[test]
    fn host_phases_come_heaviest_first() {
        use budget::Phase;
        let mut window = Window::default();
        window.add_cpu(&cpu_sample(3.0, &[(Phase::Hud, 0.5), (Phase::Pose, 2.0)]));
        window.add_cpu(&cpu_sample(1.0, &[(Phase::Hud, 0.5)]));
        let rows: Vec<Row> = window.cpu_rows(0.01).collect();
        assert_eq!(
            rows,
            [
                Row {
                    name: "pose+skin+upload",
                    mean: 1.0,
                    max: 2.0
                },
                Row {
                    name: "hud",
                    mean: 0.5,
                    max: 0.5
                },
            ]
        );
        assert_eq!(window.cpu_total().mean, 2.0);
        assert_eq!(window.cpu_total().max, 3.0);
    }

    /// A log line carries the totals, then the sections and phases as name=mean, so a
    /// script can read it back.
    #[test]
    fn a_log_line_reads_as_names_and_means() {
        use budget::Phase;
        let mut window = Window::default();
        let now = Instant::now();
        window.restart(now);
        window.add_cpu(&cpu_sample(2.0, &[(Phase::World, 1.25)]));
        window.add_gpu(&[("world-opaque", 1.5), ("glow", 0.0001)], 1.5);
        let mut line = String::new();
        window.log_line(3.0, now, &mut line);
        assert_eq!(
            line,
            "t=3.0 fps=0 frames=1 cpu=2.000/2.000 gpu=1.500/1.500 samples=1 | gpu: \
             world-opaque=1.500 | cpu: world+actor-submit=1.250"
        );
    }

    /// Without the graphics card's times, the line still has the processor's.
    #[test]
    fn a_log_line_without_timestamps_has_the_processor_only() {
        let mut window = Window::default();
        let now = Instant::now();
        window.restart(now);
        window.add_cpu(&cpu_sample(1.0, &[(budget::Phase::Hud, 1.0)]));
        let mut line = String::new();
        window.log_line(0.0, now, &mut line);
        assert_eq!(
            line,
            "t=0.0 fps=0 frames=1 cpu=1.000/1.000 | cpu: hud=1.000"
        );
    }

    /// The first time, every setting; after, only what changed, old and new.
    #[test]
    fn the_log_writes_every_setting_once_then_each_change() {
        let first = vec![("r_ssao", "1".to_owned()), ("r_fxaa", "1".to_owned())];
        let mut lines = Vec::new();
        settings_lines(&[], &first, |line| lines.push(line.to_string()));
        assert_eq!(lines, ["settings r_ssao=1 r_fxaa=1"]);
        let second = vec![("r_ssao", "0".to_owned()), ("r_fxaa", "1".to_owned())];
        lines.clear();
        settings_lines(&first, &second, |line| lines.push(line.to_string()));
        assert_eq!(lines, ["set r_ssao 1 -> 0"]);
        lines.clear();
        settings_lines(&second, &second, |line| lines.push(line.to_string()));
        assert!(lines.is_empty());
    }

    /// The panel fits every section the profiler can time in its fixed storage, with
    /// the processor's rows and the log's line.
    #[test]
    fn the_full_panel_fits_its_storage() {
        let mut readout = Readout {
            gpu_len: SECTIONS,
            cpu_len: CPU_ROWS,
            ready: true,
            ..Readout::default()
        };
        for row in readout.gpu.iter_mut().chain(readout.cpu.iter_mut()) {
            *row = Row {
                name: "encode-instance-pack+uploads",
                mean: 12.34,
                max: 56.78,
            };
        }
        let mut ui = MenuCanvas::with_capacities(PANEL_TEXTS, 48, PANEL_DRAWS);
        build_panel(&mut ui, &readout, [1920.0, 1080.0], [0.0; 2], 2, true, true);
        assert!(!ui.overflowed());
        build_panel(
            &mut ui,
            &readout,
            [1920.0, 1080.0],
            [0.0; 2],
            1,
            false,
            false,
        );
        assert!(!ui.overflowed());
    }

    /// The level is 0, 1 or 2, whatever is typed.
    #[test]
    fn the_level_stays_in_range() {
        assert_eq!(draw_level(None), 0);
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar(DRAW_CVAR, "2");
        assert_eq!(draw_level(Some(&console)), 2);
        console.set_cvar(DRAW_CVAR, "7");
        assert_eq!(draw_level(Some(&console)), 2);
        console.set_cvar(DRAW_CVAR, "-1");
        assert_eq!(draw_level(Some(&console)), 0);
    }
}
