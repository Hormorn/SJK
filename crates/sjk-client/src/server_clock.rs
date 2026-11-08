//! The client's running estimate of the server's clock.
//!
//! User commands carry the server time the client believes it is; the server
//! clamps that stamp to `[level.time - 1000, level.time + 200]` and drops a
//! command whose stamp does not advance `ps.commandTime` (`g_active.c`
//! `ClientThink_real`). A stamp the server rewrote never matches the one the
//! client predicted, so every pending command is discarded on the next
//! snapshot and the player sees raw, unpredicted snapshots. The stamp
//! therefore has to track real server time, not count issued commands.
//!
//! This mirrors `codemp/client/cl_cgame.cpp`: `CL_FirstSnapshot` anchors
//! `serverTimeDelta` on the first snapshot, `CL_AdjustTimeDelta` (lines
//! 626-673) re-anchors on a 500 ms jump, halves a 100 ms error and otherwise
//! drifts by one or two milliseconds per snapshot, and `CL_SetCGameTime`
//! (lines 779-792) derives a never-decreasing `cl.serverTime`.
//!
//! A clock is *anchored* only once it has seen a snapshot of the timeline it
//! stamps for. Until then it stamps 0, exactly as stock does: `CL_ClearState`
//! zeroes `cl.serverTime` with every gamestate and `CL_SetCGameTime` only
//! runs once the client is `CA_ACTIVE`, so the whole map load is stamped 0.
//!
//! The delta is adjusted once a frame, on the newest snapshot, after every
//! queued packet has been read (`CL_ReadPackets` drains the socket, then
//! `CL_SetCGameTime` calls `CL_AdjustTimeDelta` on `cl.snap` when
//! `cl.newSnapshots`): [`ServerClock::receive_snapshot`] notes each snapshot
//! read and [`ServerClock::adjust`] runs once the frame has read them all. A
//! snapshot that waited in the socket through a hitch is old by the time it is
//! read; adjusting on it as if it had just arrived would put the clock back by
//! the hitch, and the commands stamped from it would reach the server a second
//! or more behind its time, which servers read as a huge `cl_timeNudge`
//! (JoF's Heimdall kicked a player whose game hitched, Slash, 23/10/2026).
//! That matters because a server that has not yet seen the client
//! acknowledge the new `serverId` deliberately reports inflated snapshot
//! times (`sv.time + oldServerTime`, `sv_snapshot.cpp:190-198`) and drops to
//! raw `sv.time` afterwards (`sv_client.cpp:1517-1519`). A command stamped
//! from the inflated era is latched raw by `SV_ClientEnterWorld`
//! (`sv_client.cpp:576`) and then outranks every later command forever
//! (`:1436`), which strands the player at the spawn point.
//!
//! Commands carry the clock as the previous frame read it
//! ([`ServerClock::command_time`]): stock's `CL_Frame` runs `CL_SendCmd`, whose
//! `CL_CreateCmd` stamps `cl.serverTime`, before `CL_SetCGameTime` reads the clock
//! for the frame (`codemp/client/cl_main.cpp`, JoF EJK the same), so a stock
//! command is a frame older than the time its frame draws at: 8 ms at 125 FPS.
//! Until 07/11/2026 SJK stamped this frame's reading, a frame fresher than every
//! other client, and JoF's Heimdall reads a command stamped ahead of a stock
//! client's as a negative `cl_timeNudge`, which it kicks for. A new timeline (a
//! restart, a new connection, a re-anchor or a reset) forgets the previous
//! reading, so no command carries the old timeline's time, and after a hitch a
//! command is at most [`MAX_COMMAND_LAG_MILLIS`] older than the frame's reading.

use std::time::Instant;

/// `RESET_TIME`: a delta jump this large is a timeline change, not latency.
const RESET_MILLIS: i64 = 500;
/// Above this the delta is halved instead of nudged.
const FAST_ADJUST_MILLIS: i64 = 100;
/// `cl.extrapolatedSnapshot` margin before the newest snapshot.
const EXTRAPOLATION_MARGIN: i64 = 5;
/// The most a command's time trails the frame's reading
/// ([`ServerClock::command_time`]): a frame at the slowest gameplay FPS (20, a
/// 50 ms step). A stock client's commands trail by its whole frame, so a hitch
/// sends one stamped before it; SJK's stay this close to the clock.
const MAX_COMMAND_LAG_MILLIS: i64 = 1000 / crate::command_rate::MIN_GAMEPLAY_FPS;

/// Server-time estimate for stamping user commands (stock `cl.serverTime`).
#[derive(Clone, Debug)]
pub struct ServerClock {
    epoch: Instant,
    /// `cl.serverTimeDelta`: server time minus local milliseconds.
    delta: i64,
    /// Newest snapshot's server time (`cl.snap.serverTime`).
    latest_snapshot: i64,
    /// `cl.oldServerTime`: the monotonic floor.
    floor: i64,
    /// `cl.extrapolatedSnapshot`: a sample ran past the newest snapshot.
    extrapolated: bool,
    /// A snapshot of the timeline being stamped for has been seen
    /// (`CL_FirstSnapshot` has run). Until then every stamp is 0.
    anchored: bool,
    /// The client presents this timeline (stock `CA_ACTIVE`). While it is
    /// still loading the new map, snapshots must not anchor the clock: they
    /// can still carry the server's inflated transition times.
    active: bool,
    /// The newest snapshot read since the last [`Self::adjust`]
    /// (`cl.newSnapshots` with `cl.snap.serverTime`).
    pending: Option<i64>,
    /// The time the previous frame read for its commands, which this frame's
    /// carry ([`Self::command_time`]); `None` on a timeline not read yet.
    previous_frame: Option<i64>,
}

impl ServerClock {
    /// Anchor on the first snapshot (`CL_FirstSnapshot`).
    pub fn new(snapshot_server_time: i32, now: Instant) -> Self {
        let server_time = i64::from(snapshot_server_time);
        Self {
            epoch: now,
            delta: server_time,
            latest_snapshot: server_time,
            floor: server_time,
            extrapolated: false,
            anchored: true,
            active: true,
            pending: None,
            previous_frame: None,
        }
    }

    /// A clock for a world that is live but has not seen a snapshot yet: it
    /// stamps 0 until the next snapshot anchors it (`CL_FirstSnapshot`).
    pub fn unanchored(now: Instant) -> Self {
        Self {
            anchored: false,
            ..Self::new(0, now)
        }
    }

    /// Forget a departed connection, including its monotonic floor.
    ///
    /// Stock `CL_Disconnect` calls `CL_ClearState` (`codemp/client/cl_main.cpp`:
    /// 726,823). A new server must never receive the previous server's time,
    /// even if input is due before its next snapshot arrives. Stamps stay 0
    /// and snapshots are ignored until the new world calls [`Self::activate`].
    /// Use [`Self::restart`] instead for a gamestate on the same connection:
    /// that server may still remember the highest command already sent.
    pub fn reset_connection(&mut self, now: Instant) {
        *self = Self {
            active: false,
            ..Self::unanchored(now)
        };
    }

    /// Drop the anchor because the server restarted its timeline (a new
    /// gamestate: map change, same-map restart or `map_restart`). Stamps are
    /// 0 and snapshots are ignored until [`Self::activate`], so the inflated
    /// times the server sends during the transition never become an anchor.
    pub fn restart(&mut self) {
        self.anchored = false;
        self.active = false;
        self.extrapolated = false;
        self.pending = None;
        self.previous_frame = None;
    }

    /// The new timeline's world is live (stock `CA_ACTIVE`): let the next
    /// snapshot anchor the clock.
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Whether stamps track the server (false while stamping 0).
    pub fn anchored(&self) -> bool {
        self.anchored
    }

    fn realtime(&self, now: Instant) -> i64 {
        i64::try_from(now.saturating_duration_since(self.epoch).as_millis()).unwrap_or(i64::MAX)
    }

    /// Read a snapshot and adjust on it at once: [`Self::receive_snapshot`]
    /// then [`Self::adjust`], for a frame that read one snapshot and no more.
    pub fn observe_snapshot(&mut self, snapshot_server_time: i32, now: Instant) {
        self.receive_snapshot(snapshot_server_time);
        self.adjust(now);
    }

    /// A snapshot was read (stock `cl.snap`, `cl.newSnapshots`). The delta waits
    /// for [`Self::adjust`], once the frame has read every queued packet.
    pub fn receive_snapshot(&mut self, snapshot_server_time: i32) {
        if !self.active {
            return;
        }
        let server_time = i64::from(snapshot_server_time);
        self.latest_snapshot = server_time;
        self.pending = Some(server_time);
    }

    /// Drift the estimate toward the newest snapshot read since the last call
    /// (`CL_AdjustTimeDelta`). Call it once a frame, and only when no packet is
    /// left queued: the newest snapshot then is the one that just arrived.
    pub fn adjust(&mut self, now: Instant) {
        if !self.active {
            return;
        }
        let Some(server_time) = self.pending.take() else {
            return;
        };
        if !self.anchored {
            // `CL_FirstSnapshot`: sit exactly on this frame.
            self.anchored = true;
            self.delta = server_time - self.realtime(now);
            // Keep the connection's monotonic floor across a re-anchor. The
            // server remembers the newest usercmd it has seen and ignores
            // anything not newer (`sv_client.cpp:1436`), and it only forgets
            // that when it re-enters the client into the world — which a
            // gamestate sent to an already-active client does not do. A stamp
            // below one already sent is therefore dead on arrival, and a
            // clock that re-anchors a few hundred milliseconds behind the
            // stamps it was just issuing strands the player. The floor drops
            // only when this really is a different timeline, the same
            // `RESET_TIME` judgement `CL_AdjustTimeDelta` makes.
            let restarted = (server_time - self.floor).abs() > RESET_MILLIS;
            self.floor = if restarted {
                server_time
            } else {
                self.floor.max(server_time)
            };
            self.latest_snapshot = server_time;
            self.extrapolated = false;
            self.previous_frame = None;
            return;
        }
        let new_delta = server_time - self.realtime(now);
        let error = (new_delta - self.delta).abs();
        if error > RESET_MILLIS {
            self.delta = new_delta;
            self.floor = server_time;
            // The previous frame read the old timeline: a command carrying it
            // would be half a second or more off the server's time.
            self.previous_frame = None;
        } else if error > FAST_ADJUST_MILLIS {
            self.delta = (self.delta + new_delta) >> 1;
        } else if self.extrapolated {
            self.extrapolated = false;
            self.delta -= 2;
        } else {
            self.delta += 1;
        }
    }

    /// The server time to stamp a command issued now (`CL_SetCGameTime`).
    pub fn server_time(&mut self, now: Instant) -> i32 {
        if !self.anchored {
            return 0;
        }
        let raw = self.realtime(now) + self.delta;
        let server_time = raw.max(self.floor);
        self.floor = server_time;
        if raw >= self.latest_snapshot - EXTRAPOLATION_MARGIN {
            self.extrapolated = true;
        }
        i32::try_from(server_time).unwrap_or(i32::MAX)
    }

    /// The server time to stamp this frame's commands with: the clock as the
    /// previous call read it (stock's `cl.serverTime`, set by the previous
    /// frame's `CL_SetCGameTime` before this frame's `CL_SendCmd` stamps it),
    /// never more than [`MAX_COMMAND_LAG_MILLIS`] behind this frame's reading,
    /// which the next call returns. Call it once a frame, after [`Self::adjust`].
    /// The first call on a timeline returns its own reading. 0 while unanchored,
    /// as [`Self::server_time`].
    pub fn command_time(&mut self, now: Instant) -> i32 {
        if !self.anchored {
            return 0;
        }
        let reading = i64::from(self.server_time(now));
        let time = self.previous_frame.map_or(reading, |previous| {
            previous.max(reading - MAX_COMMAND_LAG_MILLIS)
        });
        self.previous_frame = Some(reading);
        i32::try_from(time).unwrap_or(i32::MAX)
    }

    /// The newest time read (what the next frame's commands carry, see
    /// [`Self::command_time`]), for diagnostics.
    pub fn last_stamp(&self) -> i32 {
        if !self.anchored {
            return 0;
        }
        i32::try_from(self.floor).unwrap_or(i32::MAX)
    }

    /// Newest observed snapshot time, for diagnostics.
    pub fn latest_snapshot(&self) -> i32 {
        i32::try_from(self.latest_snapshot).unwrap_or(i32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_rate::{CommandSchedule, Pacing};
    use std::time::Duration;

    const START: i64 = 10_000;
    const SNAPSHOT_MILLIS: i64 = 25;
    const FRAME_MILLIS: i64 = 17;
    const READ_PER_FRAME: usize = 8;

    /// Play a 59 FPS client on a 40 snapshots a second server with no latency, a
    /// hitch of `hitch` ms at 2 s, and return the largest lag behind the
    /// server's time of any command stamped after it. `per_snapshot` adjusts on
    /// every snapshot read, as the client did until 23/10/2026; otherwise once a
    /// frame after the queue is read, as stock does. `pacing` spreads the commands,
    /// which carry the previous frame's reading ([`ServerClock::command_time`]).
    fn worst_lag_after_hitch(step: i32, hitch: i64, per_snapshot: bool, pacing: Pacing) -> i64 {
        let base = Instant::now();
        let at = |ms: i64| base + Duration::from_millis(ms as u64);
        let mut clock = ServerClock::new(START as i32, at(0));
        let mut schedule = CommandSchedule::default();
        let mut next_snapshot = SNAPSHOT_MILLIS;
        let mut worst = 0;
        let mut real = 0;
        let mut hitched = None;
        while real < 6_000 {
            real += if hitched.is_none() && real >= 2_000 {
                hitched = Some(real);
                hitch
            } else {
                FRAME_MILLIS
            };
            let now = at(real);
            let mut read = 0;
            while next_snapshot <= real && read < READ_PER_FRAME {
                let time = (START + next_snapshot) as i32;
                if per_snapshot {
                    clock.observe_snapshot(time, now);
                } else {
                    clock.receive_snapshot(time);
                }
                next_snapshot += SNAPSHOT_MILLIS;
                read += 1;
            }
            if !per_snapshot && next_snapshot > real {
                clock.adjust(now);
            }
            let stamp = clock.command_time(now);
            for &command in schedule.due(stamp, true, now, step, pacing).as_slice() {
                if hitched.is_some() {
                    worst = worst.max(START + real - i64::from(command));
                }
            }
        }
        worst
    }

    #[test]
    fn a_hitch_keeps_commands_on_the_servers_time() {
        for step in [8, 7, 4, 3] {
            for hitch in [150, 400, 1_346] {
                let lag = worst_lag_after_hitch(step, hitch, false, Pacing::Grid);
                // The oldest of a frame's catch-up commands (32 ms back), the
                // grid, the snapshot interval the clock trails by and the frame
                // the commands' time trails the clock by, capped after the hitch.
                assert!(
                    lag <= i64::from(crate::command_rate::CATCH_UP_MILLIS)
                        + SNAPSHOT_MILLIS
                        + 8
                        + MAX_COMMAND_LAG_MILLIS,
                    "step {step} ms, hitch {hitch} ms: commands {lag} ms behind"
                );
            }
        }
    }

    #[test]
    fn one_per_frame_trails_the_clock_by_a_frame_at_most() {
        // JoF's pacing (`cl_cmdratecap`): no catch-up command and no rounding down,
        // so commands trail the server only as much as the clock does (a snapshot
        // interval and a frame) and the frame their time is older (as stock, up
        // to 50 ms after the hitch), where the grid's catch-up adds up to 32 ms
        // more (02/11/2026).
        for hitch in [17, 150, 400, 1_346] {
            let lag = worst_lag_after_hitch(8, hitch, false, Pacing::OnePerFrame);
            let grid = worst_lag_after_hitch(8, hitch, false, Pacing::Grid);
            assert!(
                lag <= SNAPSHOT_MILLIS + FRAME_MILLIS + hitch.min(MAX_COMMAND_LAG_MILLIS),
                "hitch {hitch} ms: commands {lag} ms behind"
            );
            assert!(
                grid - lag >= 8,
                "hitch {hitch} ms: grid {grid} ms, one per frame {lag} ms"
            );
        }
    }

    #[test]
    fn adjusting_on_every_queued_snapshot_put_commands_a_second_behind() {
        for step in [8, 7, 4, 3] {
            let lag = worst_lag_after_hitch(step, 1_346, true, Pacing::Grid);
            assert!(lag > 1_000, "step {step} ms: {lag} ms");
        }
    }

    #[test]
    fn adjust_waits_for_the_frame_and_takes_the_newest_snapshot() {
        let base = Instant::now();
        let at = |ms: u64| base + Duration::from_millis(ms);
        let mut clock = ServerClock::new(10_000, at(0));
        // Snapshots that waited 1 s in the socket, then the one sent now.
        for time in [10_025, 10_050, 10_075, 11_000] {
            clock.receive_snapshot(time);
        }
        assert_eq!(clock.server_time(at(1_000)), 11_000);
        clock.adjust(at(1_000));
        assert!((clock.server_time(at(1_000)) - 11_000).abs() <= 2);
        // Nothing new read: no second adjustment.
        clock.adjust(at(1_000));
        assert!((clock.server_time(at(1_000)) - 11_000).abs() <= 2);
    }

    #[test]
    fn commands_carry_the_previous_frames_reading() {
        // 125, 142, 250 and 333 FPS: each frame's commands carry the time the frame
        // before read, a whole frame older than this frame's reading, as stock's
        // `CL_SendCmd` stamps the `cl.serverTime` the previous `CL_SetCGameTime` set.
        for frame in [8_u64, 7, 4, 3] {
            let base = Instant::now();
            let mut clock = ServerClock::new(10_000, base);
            let mut previous = None;
            for n in 0..64 {
                let now = base + Duration::from_millis(n * frame);
                let stamp = clock.command_time(now);
                let reading = clock.server_time(now);
                match previous {
                    // The first frame of a timeline has no earlier reading.
                    None => assert_eq!(stamp, reading),
                    Some(previous) => {
                        assert_eq!(stamp, previous, "{frame} ms frames, frame {n}");
                        assert_eq!(i64::from(reading - stamp), frame as i64);
                    }
                }
                previous = Some(reading);
            }
        }
    }

    #[test]
    fn a_hitch_stamps_no_further_back_than_the_slowest_step() {
        let base = Instant::now();
        let at = |ms: u64| base + Duration::from_millis(ms);
        let mut clock = ServerClock::new(10_000, at(0));
        clock.command_time(at(0));
        clock.command_time(at(8));
        let stamp = clock.command_time(at(8 + 1_346));
        let reading = clock.server_time(at(8 + 1_346));
        assert_eq!(i64::from(reading - stamp), MAX_COMMAND_LAG_MILLIS);
        // The next frame carries the hitch frame's own reading.
        assert_eq!(clock.command_time(at(8 + 1_346 + 8)), reading);
    }

    #[test]
    fn a_new_timeline_never_carries_the_old_ones_time() {
        let base = Instant::now();
        let at = |ms: u64| base + Duration::from_millis(ms);

        // A restart (map change): unanchored stamps are 0, then the new map's first
        // commands carry its own time, not the old map's last reading.
        let mut clock = ServerClock::new(90_000, at(0));
        clock.command_time(at(0));
        clock.command_time(at(8));
        clock.restart();
        assert_eq!(clock.command_time(at(16)), 0);
        clock.activate();
        clock.receive_snapshot(2_000);
        clock.adjust(at(24));
        assert_eq!(clock.command_time(at(24)), clock.server_time(at(24)));
        assert_eq!(clock.command_time(at(32)), 2_000);

        // A reset (the server's time jumped back past `RESET_TIME`): a command
        // carrying the frame before it would outrank every later one.
        let mut clock = ServerClock::new(10_000, at(0));
        clock.command_time(at(0));
        clock.command_time(at(8));
        clock.receive_snapshot(4_000);
        clock.adjust(at(16));
        let stamp = clock.command_time(at(16));
        assert_eq!(stamp, clock.server_time(at(16)));
        assert!(stamp < 5_000, "stamped {stamp}");

        // A new connection: 0 until its world is live and anchored, then its time.
        let mut clock = ServerClock::new(10_000, at(0));
        clock.command_time(at(0));
        clock.reset_connection(at(8));
        clock.receive_snapshot(3_000);
        clock.adjust(at(8));
        assert_eq!(clock.command_time(at(8)), 0);
        clock.activate();
        clock.receive_snapshot(3_000);
        clock.adjust(at(16));
        assert_eq!(clock.command_time(at(16)), 3_000);
    }

    #[test]
    fn a_restart_drops_the_snapshot_waiting_to_adjust() {
        let base = Instant::now();
        let mut clock = ServerClock::new(10_000, base);
        clock.receive_snapshot(10_025);
        clock.restart();
        clock.activate();
        clock.adjust(base);
        assert!(!clock.anchored());
        assert_eq!(clock.server_time(base), 0);
    }
}
