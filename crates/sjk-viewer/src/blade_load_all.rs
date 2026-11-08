//! `saber_loadall` (Sol, 06/11/2026, to class the tiers of Creyon's 130 new blades): build
//! every blade's own shader now instead of when a blade is first shown.
//!
//! A signature blade draws with the shared pipelines, from its skin's file, until its own
//! are built off the render thread (`saber_gpu.rs`, `BuildQueue`); they are built when a
//! blade first draws it (worn, in the preview, in the Collection's pictures), so going
//! through the Collection, each new blade shows its plain look first and its real one a few
//! seconds later. The command asks for every loaded blade's shader at once, after the ones
//! worn or shown: the Collection's pictures and the preview then show each blade as it
//! really looks the moment it comes up. Typed again, it says how far the builds are. For
//! this PC and this run only: nothing is stored, and the next start builds on demand again.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

/// Console command name.
pub(crate) const COMMAND: &str = "saber_loadall";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Build every blade's shader now, so each shows its real look at once; again to see how far";

/// How many times the command asked for every shader (0: never this run): the blade
/// renderers ask once more whenever it grows ([`asked`]).
static ASKED: AtomicU64 = AtomicU64::new(0);
/// The loaded blades with a shader of their own, and how many of those are built, as the
/// blade renderer last counted them ([`count`]).
static TOTAL: AtomicUsize = AtomicUsize::new(0);
static BUILT: AtomicUsize = AtomicUsize::new(0);
/// When the command first asked, and when every shader was found built.
static TIMES: Mutex<(Option<Instant>, Option<Instant>)> = Mutex::new((None, None));

/// How many times every shader was asked for; a renderer that asked fewer times asks again.
pub(crate) fn asked() -> u64 {
    ASKED.load(Ordering::Acquire)
}

/// The blade renderer's count: `built` of the `total` loaded blades with a shader of their
/// own are built.
pub(crate) fn count(built: usize, total: usize) {
    TOTAL.store(total, Ordering::Relaxed);
    BUILT.store(built, Ordering::Relaxed);
    if total > 0 && built >= total {
        let mut times = TIMES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if times.0.is_some() && times.1.is_none() {
            times.1 = Some(Instant::now());
        }
    }
}

/// `saber_loadall`: ask for every shader the first time, then say how far the builds are.
pub(crate) fn command() -> Vec<String> {
    let mut times = TIMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (built, total) = (BUILT.load(Ordering::Relaxed), TOTAL.load(Ordering::Relaxed));
    let Some(started) = times.0 else {
        *times = (Some(Instant::now()), None);
        ASKED.fetch_add(1, Ordering::AcqRel);
        return vec![
            format!(
                "Building every blade's shader in the background ({}), after the blades worn and \
                 shown; each shows its real look once built. The game may stutter a little \
                 meanwhile.",
                if total > 0 {
                    format!("{} of {total} built so far", built.min(total))
                } else {
                    "none counted yet".to_owned()
                }
            ),
            format!("Type {COMMAND} again to see how far it is."),
        ];
    };
    // Asked again: ask once more (blades loaded since, a new map's renderer), and tell.
    ASKED.fetch_add(1, Ordering::AcqRel);
    let done = times
        .1
        .filter(|_| built >= total)
        .map(|done| done - started);
    vec![how_far(built, total, started.elapsed(), done)]
}

/// How far the builds are: `built` of `total`, `elapsed` since the command asked, `done`
/// how long they took once all are built.
fn how_far(
    built: usize,
    total: usize,
    elapsed: std::time::Duration,
    done: Option<std::time::Duration>,
) -> String {
    match done {
        Some(done) => format!(
            "All {total} blade shaders are built (in {}).",
            seconds(done)
        ),
        None => format!(
            "Blade shaders: {} of {total} built ({} so far).",
            built.min(total),
            seconds(elapsed)
        ),
    }
}

/// `elapsed` as players read it: "42 s", "3 min 05 s".
fn seconds(elapsed: std::time::Duration) -> String {
    let secs = elapsed.as_secs();
    if secs < 60 {
        format!("{secs} s")
    } else {
        format!("{} min {:02} s", secs / 60, secs % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_tells_how_far_then_how_long_it_took() {
        let secs = std::time::Duration::from_secs;
        assert_eq!(
            how_far(120, 262, secs(48), None),
            "Blade shaders: 120 of 262 built (48 s so far)."
        );
        assert_eq!(
            how_far(262, 262, secs(130), Some(secs(100))),
            "All 262 blade shaders are built (in 1 min 40 s)."
        );
    }

    #[test]
    fn a_time_reads_in_seconds_then_minutes() {
        assert_eq!(seconds(std::time::Duration::from_secs(42)), "42 s");
        assert_eq!(seconds(std::time::Duration::from_secs(185)), "3 min 05 s");
    }
}
