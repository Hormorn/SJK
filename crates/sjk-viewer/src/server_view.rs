//! What the server sees of our commands, written to the log (Heimdall kicks).
//!
//! JoF's anti-cheat, Heimdall (closed source), drops SJK players with "Set
//! /cl_timeNudge 0" at random moments, and three guesses at the cause did not
//! stop it ([networking.md](../../../docs/networking.md#user-commands-and-move-packets)).
//! A server can only judge a time nudge from what reaches it: how old the
//! newest command it ran is against its own time. Every snapshot carries both
//! sides of that, `serverTime` and the player state's `commandTime` (the stamp
//! of the last command the server ran), so their difference is the command's
//! age at the server when it built the snapshot. Minus that command's round
//! trip (from the packet that carried it to the snapshot reflecting it) it
//! leaves about how far the stamps trail the server, which a stock client's
//! `cl_timeNudge` moves; a gap with no command (a frozen frame, a stalled
//! packet) shows as an age that grows until the next command lands.
//!
//! [`ServerView`] keeps the last [`WINDOW_MS`] of these, in storage allocated
//! once, and writes a line when the server drops the player ([`ServerView::drop_report`])
//! and one a minute while playing ([`ServerView::frame`]), so a kicked player's
//! logs show what the server measured before the kick.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// How much history a drop report covers.
pub(crate) const WINDOW_MS: u32 = 10_000;
/// Snapshots kept: [`WINDOW_MS`] at 100 snapshots a second.
const SNAPSHOTS: usize = 1_024;
/// Sent commands waiting for the snapshot that runs them: 2 s at 125 a second.
const COMMANDS: usize = 256;
/// Frames and packet gaps longer than this are kept for the drop report.
const STALL_MS: u32 = 40;
/// Stalls kept: far more than [`WINDOW_MS`] holds at one every [`STALL_MS`].
const STALLS: usize = 256;
/// Time between the summary lines.
const SUMMARY_EVERY: Duration = Duration::from_secs(60);
/// An age minus round trip above this is counted as late in the summary.
const LATE_MS: i32 = 100;
/// An age outside this range is a different time line (a map change), not lag.
const AGE_RANGE: std::ops::RangeInclusive<i32> = -1_000..=60_000;

#[derive(Clone, Copy, Debug, Default)]
struct Snapshot {
    /// Milliseconds since [`ServerView::epoch`] when it was read.
    at: u32,
    /// `serverTime - ps.commandTime`.
    age: i32,
    /// From the packet carrying `ps.commandTime`'s command to this read, if known.
    round_trip: Option<i32>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Sent {
    stamp: i32,
    /// When the first packet carrying it left.
    sent: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum StallKind {
    #[default]
    Frame,
    Packet,
}

#[derive(Clone, Copy, Debug, Default)]
struct Stall {
    at: u32,
    ms: u32,
    kind: StallKind,
}

/// A fixed-size ring, newest last.
struct Ring<T> {
    items: Box<[T]>,
    next: usize,
    len: usize,
}

impl<T: Copy + Default> Ring<T> {
    fn new(capacity: usize) -> Self {
        Self {
            items: vec![T::default(); capacity].into_boxed_slice(),
            next: 0,
            len: 0,
        }
    }

    fn push(&mut self, item: T) {
        self.items[self.next] = item;
        self.next = (self.next + 1) % self.items.len();
        self.len = (self.len + 1).min(self.items.len());
    }

    fn clear(&mut self) {
        self.next = 0;
        self.len = 0;
    }

    /// Index of the `back`th newest item (0 is the newest).
    fn index(&self, back: usize) -> usize {
        (self.next + self.items.len() - 1 - back) % self.items.len()
    }

    fn newest_first(&self) -> impl Iterator<Item = &T> {
        (0..self.len).map(|back| &self.items[self.index(back)])
    }
}

/// Totals since the last summary line.
#[derive(Clone, Copy, Debug, Default)]
struct Period {
    snapshots: u32,
    age_sum: i64,
    age_max: i32,
    trip_count: u32,
    trip_sum: i64,
    trail_sum: i64,
    trail_min: i32,
    trail_max: i32,
    late: u32,
    packet_gap_max: u32,
    frame_max: u32,
}

impl Period {
    fn add(&mut self, sample: Snapshot) {
        self.snapshots += 1;
        self.age_sum += i64::from(sample.age);
        self.age_max = if self.snapshots == 1 {
            sample.age
        } else {
            self.age_max.max(sample.age)
        };
        if let Some(trip) = sample.round_trip {
            let trail = sample.age - trip;
            self.trail_min = if self.trip_count == 0 {
                trail
            } else {
                self.trail_min.min(trail)
            };
            self.trail_max = if self.trip_count == 0 {
                trail
            } else {
                self.trail_max.max(trail)
            };
            self.trip_count += 1;
            self.trip_sum += i64::from(trip);
            self.trail_sum += i64::from(trail);
            self.late += u32::from(trail > LATE_MS);
        }
    }
}

fn average(sum: i64, count: u32) -> i64 {
    if count == 0 {
        0
    } else {
        sum / i64::from(count)
    }
}

/// The server's view of our commands over the last seconds (module docs).
pub(crate) struct ServerView {
    epoch: Instant,
    snapshots: Ring<Snapshot>,
    commands: Ring<Sent>,
    stalls: Ring<Stall>,
    last_frame: Option<Instant>,
    last_packet: Option<Instant>,
    period: Period,
    next_summary: Option<Instant>,
}

impl Default for ServerView {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            snapshots: Ring::new(SNAPSHOTS),
            commands: Ring::new(COMMANDS),
            stalls: Ring::new(STALLS),
            last_frame: None,
            last_packet: None,
            period: Period::default(),
            next_summary: None,
        }
    }
}

impl ServerView {
    fn millis(&self, now: Instant) -> u32 {
        u32::try_from(now.saturating_duration_since(self.epoch).as_millis()).unwrap_or(u32::MAX)
    }

    /// A frame of a live remote session began at `now`. Returns the summary line
    /// once [`SUMMARY_EVERY`] has passed since the first frame or the last line.
    pub(crate) fn frame(&mut self, now: Instant) -> Option<String> {
        if let Some(last) = self.last_frame.replace(now) {
            let ms =
                u32::try_from(now.saturating_duration_since(last).as_millis()).unwrap_or(u32::MAX);
            self.stall(now, ms, StallKind::Frame);
        }
        let due = *self.next_summary.get_or_insert(now + SUMMARY_EVERY);
        if now < due {
            return None;
        }
        self.next_summary = Some(now + SUMMARY_EVERY);
        let period = std::mem::take(&mut self.period);
        Some(summary_line(&period))
    }

    fn stall(&mut self, now: Instant, ms: u32, kind: StallKind) {
        match kind {
            StallKind::Frame => self.period.frame_max = self.period.frame_max.max(ms),
            StallKind::Packet => self.period.packet_gap_max = self.period.packet_gap_max.max(ms),
        }
        if ms > STALL_MS {
            let at = self.millis(now);
            self.stalls.push(Stall { at, ms, kind });
        }
    }

    /// A command stamped `stamp` was queued for the next packet.
    pub(crate) fn command(&mut self, stamp: i32) {
        self.commands.push(Sent { stamp, sent: None });
    }

    /// A move packet left at `now`, carrying every command queued since the last.
    pub(crate) fn packet_sent(&mut self, now: Instant) {
        if let Some(last) = self.last_packet.replace(now) {
            let ms =
                u32::try_from(now.saturating_duration_since(last).as_millis()).unwrap_or(u32::MAX);
            self.stall(now, ms, StallKind::Packet);
        }
        let at = self.millis(now);
        for back in 0..self.commands.len {
            let index = self.commands.index(back);
            let sent = &mut self.commands.items[index];
            if sent.sent.is_some() {
                break;
            }
            sent.sent = Some(at);
        }
    }

    /// A snapshot of the timeline being stamped for was read at `now`:
    /// `server_time` and its player state's `command_time`.
    pub(crate) fn snapshot(&mut self, server_time: i32, command_time: i32, now: Instant) {
        if command_time <= 0 {
            return;
        }
        let age = server_time.saturating_sub(command_time);
        if !AGE_RANGE.contains(&age) {
            return;
        }
        let at = self.millis(now);
        let round_trip = self
            .commands
            .newest_first()
            .find(|sent| sent.stamp == command_time)
            .and_then(|sent| sent.sent)
            .map(|sent| i32::try_from(at.saturating_sub(sent)).unwrap_or(i32::MAX));
        let sample = Snapshot {
            at,
            age,
            round_trip,
        };
        self.snapshots.push(sample);
        self.period.add(sample);
    }

    /// The server started a new time line (a map change): stamps from before it
    /// can no longer match a `commandTime`.
    pub(crate) fn restart(&mut self) {
        self.commands.clear();
    }

    /// The session is over: forget everything.
    pub(crate) fn end(&mut self) {
        self.snapshots.clear();
        self.commands.clear();
        self.stalls.clear();
        self.last_frame = None;
        self.last_packet = None;
        self.period = Period::default();
        self.next_summary = None;
    }

    /// The line written when the server drops the player: the last [`WINDOW_MS`]
    /// before `now`, or nothing if no snapshot was kept in that time.
    pub(crate) fn drop_report(&self, now: Instant) -> Option<String> {
        let end = self.millis(now);
        let start = end.saturating_sub(WINDOW_MS);
        let mut period = Period::default();
        // The oldest of the worst ages, and when it was read.
        let mut worst = (i32::MIN, 0_u32);
        let mut seconds = [None::<i32>; (WINDOW_MS / 1_000) as usize];
        for sample in self.snapshots.newest_first().take_while(|s| s.at >= start) {
            period.add(*sample);
            if sample.age >= worst.0 {
                worst = (sample.age, sample.at);
            }
            let second = ((end - sample.at) / 1_000) as usize;
            if let Some(slot) = seconds.get_mut(second) {
                *slot = Some(slot.map_or(sample.age, |worst| worst.max(sample.age)));
            }
        }
        if period.snapshots == 0 {
            return None;
        }
        let worst_stall = |kind| {
            self.stalls
                .newest_first()
                .take_while(|stall| stall.at >= start)
                .filter(|stall| stall.kind == kind)
                .max_by_key(|stall| stall.ms)
                .map(|stall| (stall.ms, end - stall.at))
        };
        let before = |ms: u32| f64::from(ms) / 1_000.0;
        let mut line = format!(
            "server view, last {} s before the drop: {} snapshots, command age at the server avg {} / max {} ms ({:.1} s before)",
            WINDOW_MS / 1_000,
            period.snapshots,
            average(period.age_sum, period.snapshots),
            period.age_max,
            before(end - worst.1),
        );
        if period.trip_count > 0 {
            let _ = write!(
                line,
                ", round trip avg {} ms, age minus round trip avg {} / min {} / max {} ms",
                average(period.trip_sum, period.trip_count),
                average(period.trail_sum, period.trip_count),
                period.trail_min,
                period.trail_max,
            );
        }
        for (kind, name) in [
            (StallKind::Packet, "longest gap between packets"),
            (StallKind::Frame, "longest frame"),
        ] {
            match worst_stall(kind) {
                Some((ms, ago)) => {
                    let _ = write!(line, ", {name} {ms} ms ({:.1} s before)", before(ago));
                }
                None => {
                    let _ = write!(line, ", {name} under {STALL_MS} ms");
                }
            }
        }
        line.push_str("; worst age each second, oldest first:");
        for worst in seconds.iter().rev() {
            match worst {
                Some(age) => {
                    let _ = write!(line, " {age}");
                }
                None => line.push_str(" -"),
            }
        }
        Some(line)
    }
}

fn summary_line(period: &Period) -> String {
    let mut line = format!(
        "server view, last {} s: {} snapshots, command age at the server avg {} / max {} ms",
        SUMMARY_EVERY.as_secs(),
        period.snapshots,
        average(period.age_sum, period.snapshots),
        period.age_max,
    );
    if period.trip_count > 0 {
        let _ = write!(
            line,
            ", round trip avg {} ms, age minus round trip avg {} / min {} / max {} ms ({} over {LATE_MS} ms)",
            average(period.trip_sum, period.trip_count),
            average(period.trail_sum, period.trip_count),
            period.trail_min,
            period.trail_max,
            period.late,
        );
    }
    let _ = write!(
        line,
        ", longest gap between packets {} ms, longest frame {} ms",
        period.packet_gap_max, period.frame_max
    );
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, ms: u64) -> Instant {
        start + Duration::from_millis(ms)
    }

    /// A 125 FPS client on a 30 snapshots a second server, 40 ms each way, with a
    /// `freeze` ms frame at 5 s, played for `seconds`.
    fn play(view: &mut ServerView, start: Instant, seconds: u64, freeze: u64) -> u64 {
        let mut real = 0;
        let mut stamps: Vec<(u64, i32)> = Vec::new();
        let mut next_snapshot = 33;
        while real < seconds * 1_000 {
            real += if (5_000..5_008).contains(&real) {
                freeze
            } else {
                8
            };
            let now = at(start, real);
            view.frame(now);
            // Snapshots the server built 40 ms ago, reflecting the newest command
            // that had reached it (sent 40 ms before that).
            while next_snapshot + 40 <= real {
                let built = next_snapshot;
                let ran = stamps
                    .iter()
                    .rev()
                    .find(|(sent, _)| sent + 40 <= built)
                    .map(|(_, stamp)| *stamp);
                if let Some(stamp) = ran {
                    view.snapshot(10_000 + built as i32, stamp, now);
                }
                next_snapshot += 33;
            }
            // Stamped 30 ms behind the server, as the clock trails it.
            let stamp = 10_000 + real as i32 - 30;
            view.command(stamp);
            view.packet_sent(now);
            stamps.push((real, stamp));
        }
        real
    }

    #[test]
    fn a_smooth_session_shows_the_clock_lag_and_no_stall() {
        let start = Instant::now();
        let mut view = ServerView {
            epoch: start,
            ..Default::default()
        };
        let end = play(&mut view, start, 12, 8);
        let line = view.drop_report(at(start, end)).unwrap();
        // Age: 30 ms of clock lag, 40 ms up, up to a snapshot interval of waiting.
        assert!(line.contains("round trip avg"), "{line}");
        assert!(
            line.contains("longest gap between packets under 40 ms"),
            "{line}"
        );
        assert!(line.contains("longest frame under 40 ms"), "{line}");
        let trail = view
            .snapshots
            .newest_first()
            .filter_map(|s| s.round_trip.map(|trip| s.age - trip))
            .max()
            .unwrap();
        assert!(
            (-50..=40).contains(&trail),
            "age minus round trip {trail} ms"
        );
    }

    #[test]
    fn a_frozen_frame_shows_as_an_old_command_and_a_gap() {
        let start = Instant::now();
        let mut view = ServerView {
            epoch: start,
            ..Default::default()
        };
        let end = play(&mut view, start, 12, 480);
        let worst = view.snapshots.newest_first().map(|s| s.age).max().unwrap();
        assert!(worst >= 480, "worst age {worst} ms");
        let line = view.drop_report(at(start, end)).unwrap();
        assert!(line.contains("longest frame 480 ms"), "{line}");
        assert!(
            line.contains("longest gap between packets 480 ms"),
            "{line}"
        );
    }

    #[test]
    fn the_summary_comes_once_a_minute_and_starts_over() {
        let start = Instant::now();
        let mut view = ServerView {
            epoch: start,
            ..Default::default()
        };
        assert!(view.frame(at(start, 0)).is_none());
        view.command(1_000);
        view.packet_sent(at(start, 10));
        view.snapshot(1_060, 1_000, at(start, 70));
        assert!(view.frame(at(start, 59_999)).is_none());
        let line = view.frame(at(start, 60_000)).unwrap();
        assert!(
            line.contains("1 snapshots, command age at the server avg 60 / max 60 ms"),
            "{line}"
        );
        assert!(
            line.contains("round trip avg 60 ms, age minus round trip avg 0"),
            "{line}"
        );
        assert!(view.frame(at(start, 60_001)).is_none());
        let next = view.frame(at(start, 120_000)).unwrap();
        assert!(next.contains(": 0 snapshots"), "{next}");
    }

    #[test]
    fn other_time_lines_and_an_empty_window_are_left_out() {
        let start = Instant::now();
        let mut view = ServerView {
            epoch: start,
            ..Default::default()
        };
        // A map change: the old timeline's commandTime against the new serverTime.
        view.snapshot(400, 90_000, at(start, 0));
        view.snapshot(400, 0, at(start, 0));
        assert!(view.drop_report(at(start, 0)).is_none());
        view.snapshot(1_050, 1_000, at(start, 0));
        assert!(view.drop_report(at(start, 5_000)).is_some());
        assert!(
            view.drop_report(at(start, 10_001)).is_none(),
            "older than the window"
        );
        view.end();
        assert!(view.drop_report(at(start, 0)).is_none());
        assert!(
            view.frame(at(start, 0)).is_none(),
            "the summary clock starts again"
        );
    }
}
