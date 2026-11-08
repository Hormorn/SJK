//! A blade's motion, for the Blade Lab designs that answer it (`saber.wgsl` `motion`,
//! [`crate::blade_lab_designs`]): how hard it swings, and the seconds since it was lit,
//! since it last clashed and since its last swing began. Kept per blade with its trail
//! ([`crate::saber_trail::BladeState`]); presentation only.

use crate::saber::Blade;
use std::sync::Mutex;

/// Seconds reported for something that has not happened, or long enough ago to be
/// settled (a blade lit before it was first seen, a swing long over).
pub(crate) const SETTLED: f32 = 1_000.0;
/// The tip's speed about the hilt (units a second) of a full swing: the lab's 5.5 m/s.
/// About the hilt, so running with the blade held still is no swing.
const FULL_SWING: f32 = 5.5 / 0.027;
/// The lab's swing rises half the way to the tip's speed each frame and falls 8% of the
/// way, at 60 frames a second.
const RISE: f32 = 0.5;
const FALL: f32 = 0.08;
/// A swing begins when the swing rises past this and ends when it falls under the other,
/// above an idle stance's sway; a chain of swings without a pause between them begins a
/// new one every `SWING_EVERY` milliseconds (each swing of a combo is one).
/// (Round 13: up from 0.4 and 0.25, a run's arm sway set the Clone Wars blade's words off.)
const SWING_BEGINS: f32 = 0.55;
const SWING_ENDS: f32 = 0.3;
/// The length the swing's tip is measured at: a stock blade's (units).
const SWING_REACH: f32 = 40.0;
/// A swing also has to turn the blade: its tip (at [`SWING_REACH`]) must have moved this
/// far (units, about 40 degrees) from where it was within the last [`SWING_SPAN`]
/// milliseconds (Creyon's round 14: a run's arm sway, fast but short, still set the Clone
/// Wars blade's words off; it moves the tip some 20 units, a swing 40 and more).
const SWING_TURN: f32 = 27.0;
const SWING_SPAN: i64 = 450;
/// The tips kept for [`SWING_TURN`].
const RECENT: usize = 16;
const SWING_EVERY: i64 = 1_000;
/// A blade unseen this long (milliseconds) was put away; seen again, it is lit anew.
const UNSEEN: i64 = 250;

/// The tip's speed in the world (units a second, at [`SWING_REACH`]) that stirs a blade fully
/// ([`BladeMotion::stir`]): about a run's (JKA runs at 250), and a speed under
/// [`STIR_IDLE`] (an idle stance's sway) stirs it not at all.
const STIR_FULL: f32 = 200.0;
const STIR_IDLE: f32 = 40.0;
/// Seconds the stir takes to rise most of the way to a new speed, and to fall back once the
/// blade is still (slower, so its effects stay out a moment before settling).
const STIR_RISE: f32 = 0.3;
const STIR_FALL: f32 = 1.4;

/// How near (units) a clash must be to a blade for it to be that blade's.
const CLASH_REACH: f32 = 12.0;

/// The latest clashes the snapshots brought (where, numbered in order), for the blades
/// to find theirs ([`note_clash`]); the newest eight.
static CLASHES: Mutex<Clashes> = Mutex::new(NO_CLASHES);

/// The newest eight clashes (where, and their numbers) and the newest's number.
type Clashes = ([([f32; 3], u64); 8], u64);
const NO_CLASHES: Clashes = ([([0.0; 3], 0); 8], 0);

/// A saber clash at `origin` (`impact_spawn.rs`, as its flare latches).
pub(crate) fn note_clash(origin: [f32; 3]) {
    if let Ok(mut clashes) = CLASHES.lock() {
        clashes.1 += 1;
        let number = clashes.1;
        clashes.0[(number % 8) as usize] = (origin, number);
    }
}

/// The motion `saber.wgsl` reads: swing (0 to 1), seconds since the last clash, since
/// the blade was lit and since the last swing began.
pub(crate) const STILL: [f32; 4] = [0.0, SETTLED, SETTLED, SETTLED];

/// Frames in a row a blade's length must fall before it counts as going out.
const SHRINKING: u8 = 3;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BladeMotion {
    tip: Option<[f32; 3]>,
    seen: i64,
    swing: f32,
    swinging: bool,
    lit: Option<i64>,
    swung: Option<i64>,
    clashed: Option<i64>,
    /// The newest clash this blade has looked at.
    clash_seen: u64,
    /// Its length last seen, and when it began going out (switched off), if it is.
    length: f32,
    going_out: Option<i64>,
    /// How many frames in a row its length has fallen.
    shrinking: u8,
    /// Its tips of the last frames and when (a ring, [`SWING_TURN`]).
    recent: [Option<([f32; 3], i64)>; RECENT],
    recent_next: usize,
    /// Its tip in the world last seen, and how stirred it is ([`Self::stir`]).
    world_tip: Option<[f32; 3]>,
    stir: f32,
}

/// What the lit lane reports while the blade goes out: this plus the seconds since it
/// began to (Creyon's round 13: the Clone Wars blade's word on switching off). Above
/// [`SETTLED`], so whatever reads the lane as "long lit" still does.
pub(crate) const GOING_OUT: f32 = 2_000.0;

impl BladeMotion {
    /// The blade drawn at `now` (presentation milliseconds), `growing` while it is still
    /// coming out of the hilt; its motion for the shader.
    #[cfg(test)]
    pub(crate) fn record(&mut self, blade: Blade, now: i64, growing: bool) -> [f32; 4] {
        self.record_turned(blade, blade.direction, now, growing, None, true)
    }

    /// [`Self::record`] with the swing measured along `body`, the blade's direction in its
    /// wearer's own frame (Creyon's round 11: turning the view or running is no swing).
    /// `each_swing`: a chain's swings are marked one by one ([`SWING_EVERY`]), else the chain
    /// counts from its first. `attacking` is whether its wearer's saber is in an attack (`saberMove`), when known:
    /// then only an attack begins a swing (Creyon's round 14: whatever the arm does while
    /// running is none); unknown, the blade must also have turned ([`SWING_TURN`]).
    pub(crate) fn record_turned(
        &mut self,
        blade: Blade,
        body: [f32; 3],
        now: i64,
        growing: bool,
        attacking: Option<bool>,
        each_swing: bool,
    ) -> [f32; 4] {
        // The tip from the hilt: how the blade turns, not how its wearer moves, nor how long
        // it is now (Creyon's round 13: switching a blade off read as a swing, and the Clone
        // Wars blade's swing words came then), so a stock blade's length.
        let tip = [
            body[0] * SWING_REACH,
            body[1] * SWING_REACH,
            body[2] * SWING_REACH,
        ];
        let gap = now - self.seen;
        let fresh = self.tip.is_none() || !(0..=UNSEEN).contains(&gap);
        let clashes = CLASHES.lock().map(|clashes| *clashes).unwrap_or(NO_CLASHES);
        if fresh {
            *self = Self {
                lit: growing.then_some(now),
                clash_seen: clashes.1,
                ..Self::default()
            };
        } else if gap > 0 {
            let seconds = gap as f32 * 0.001;
            let last = self.tip.unwrap_or(tip);
            let moved = ((tip[0] - last[0]).powi(2)
                + (tip[1] - last[1]).powi(2)
                + (tip[2] - last[2]).powi(2))
            .sqrt();
            let target = (moved / seconds / FULL_SWING).clamp(0.0, 1.0);
            let frames = seconds * 60.0;
            let rate = if target > self.swing { RISE } else { FALL };
            self.swing += (target - self.swing) * (1.0 - (1.0 - rate).powf(frames));
            // How much it moves in the world: running carries it as a swing does.
            let world = [
                blade.base[0] + blade.direction[0] * SWING_REACH,
                blade.base[1] + blade.direction[1] * SWING_REACH,
                blade.base[2] + blade.direction[2] * SWING_REACH,
            ];
            let speed = self.world_tip.map_or(0.0, |last| {
                ((world[0] - last[0]).powi(2)
                    + (world[1] - last[1]).powi(2)
                    + (world[2] - last[2]).powi(2))
                .sqrt()
                    / seconds
            });
            let stirred = ((speed - STIR_IDLE) / (STIR_FULL - STIR_IDLE))
                .clamp(0.0, 1.0)
                .max(self.swing);
            let tau = if stirred > self.stir {
                STIR_RISE
            } else {
                STIR_FALL
            };
            self.stir += (stirred - self.stir) * (1.0 - (-seconds / tau).exp());
            let attack = attacking.unwrap_or_else(|| self.turned(tip, now));
            if !self.swinging && self.swing > SWING_BEGINS && attack {
                self.swinging = true;
                self.swung = Some(now);
            } else if self.swinging && self.swing < SWING_ENDS {
                self.swinging = false;
            } else if each_swing
                && self.swinging
                && attack
                && self.swung.is_some_and(|at| now - at >= SWING_EVERY)
            {
                self.swung = Some(now);
            }
        }
        if clashes.1 > self.clash_seen {
            let near = clashes.0.iter().any(|&(origin, number)| {
                number > self.clash_seen && distance_to(origin, blade) < CLASH_REACH + blade.radius
            });
            if near {
                self.clash(now);
            }
            self.clash_seen = clashes.1;
        }
        if !fresh {
            // Only a blade that keeps shrinking is going out: a length that dips for a frame or two
            // made its effects vanish and snap back in game (Creyon, 4/11/2026).
            if blade.length < self.length - 0.01 {
                self.shrinking = self.shrinking.saturating_add(1);
                if self.shrinking >= SHRINKING
                    || blade.length < self.length - 1.0
                    || blade.length <= 0.01
                {
                    self.going_out.get_or_insert(now);
                }
            } else {
                self.shrinking = 0;
                if blade.length > 0.01 {
                    self.going_out = None;
                }
            }
        }
        self.length = blade.length;
        self.tip = Some(tip);
        self.world_tip = Some([
            blade.base[0] + blade.direction[0] * SWING_REACH,
            blade.base[1] + blade.direction[1] * SWING_REACH,
            blade.base[2] + blade.direction[2] * SWING_REACH,
        ]);
        self.seen = now;
        self.recent[self.recent_next] = Some((tip, now));
        self.recent_next = (self.recent_next + 1) % RECENT;
        self.motion(now)
    }

    /// Whether the tip at `tip` has turned [`SWING_TURN`] from a tip of the last
    /// [`SWING_SPAN`] milliseconds.
    fn turned(&self, tip: [f32; 3], now: i64) -> bool {
        self.recent.iter().flatten().any(|&(past, at)| {
            now - at <= SWING_SPAN
                && ((tip[0] - past[0]).powi(2)
                    + (tip[1] - past[1]).powi(2)
                    + (tip[2] - past[2]).powi(2))
                .sqrt()
                    >= SWING_TURN
        })
    }

    /// While the blade, gone in, lingers (its words float on: [`crate::saber_skins::SkinColor::linger`]):
    /// the motion its instance carries, a swing of -1 telling `saber.wgsl` to draw no
    /// blade, `None` once `linger` seconds have passed since it began going out.
    pub(crate) fn lingering(&self, now: i64, linger: f32) -> Option<[f32; 4]> {
        let since = (now - self.going_out?).max(0) as f32 * 0.001;
        (since < linger).then_some([-1.0, SETTLED, GOING_OUT + since, SETTLED])
    }

    /// How long the blade has been lit, in seconds; none when it is not.
    pub(crate) fn lit_for(&self, now: i64) -> Option<f32> {
        self.lit.map(|lit| (now - lit).max(0) as f32 * 0.001)
    }

    /// How stirred the blade is, 0 (still) to 1, rising quickly while it moves in the world,
    /// running or swinging, and falling back slowly once it is still ([`STIR_FALL`]); smooth
    /// either way, so what answers it never jumps.
    pub(crate) fn stir(&self) -> f32 {
        self.stir
    }

    /// A clash at `now`.
    pub(crate) fn clash(&mut self, now: i64) {
        self.clashed = Some(now);
    }

    fn motion(&self, now: i64) -> [f32; 4] {
        let since = |at: Option<i64>| {
            at.map_or(SETTLED, |at| {
                ((now - at).max(0) as f32 * 0.001).min(SETTLED)
            })
        };
        let lit = match self.going_out {
            Some(at) => GOING_OUT + ((now - at).max(0) as f32 * 0.001).min(SETTLED),
            None => since(self.lit),
        };
        [self.swing, since(self.clashed), lit, since(self.swung)]
    }
}

/// How far `point` is from `blade` (the segment from its base to its tip).
fn distance_to(point: [f32; 3], blade: Blade) -> f32 {
    let v = [
        point[0] - blade.base[0],
        point[1] - blade.base[1],
        point[2] - blade.base[2],
    ];
    let d = blade.direction;
    let along = (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]).clamp(0.0, blade.length);
    ((v[0] - d[0] * along).powi(2) + (v[1] - d[1] * along).powi(2) + (v[2] - d[2] * along).powi(2))
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blade(angle: f32) -> Blade {
        Blade {
            base: [0.0; 3],
            direction: [angle.cos(), angle.sin(), 0.0],
            length: 40.0,
            radius: 3.0,
        }
    }

    #[test]
    fn a_blade_held_still_is_settled_and_one_seen_lighting_is_young() {
        let mut motion = BladeMotion::default();
        assert_eq!(motion.record(blade(0.0), 1_000, false), STILL);
        let mut lighting = BladeMotion::default();
        assert_eq!(lighting.record(blade(0.0), 1_000, true)[2], 0.0);
        // Half a second later, by 100 ms steps (never unseen).
        let mut lighting = BladeMotion::default();
        lighting.record(blade(0.0), 1_000, true);
        let mut last = [0.0; 4];
        for step in 1..=5 {
            last = lighting.record(blade(0.0), 1_000 + step * 100, false);
        }
        assert!((last[2] - 0.5).abs() < 1e-4);
        assert_eq!(last[0], 0.0);
    }

    #[test]
    fn a_fast_turn_swings_and_marks_its_start_then_settles() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        // A quarter turn in 100 ms, 16 ms a frame: the tip moves about 630 units a second.
        let mut at = 0;
        let mut began = None;
        for frame in 1..=6 {
            at = frame * 16;
            let m = motion.record(blade(frame as f32 * 0.25), at, false);
            if m[3] < SETTLED && began.is_none() {
                began = Some(at);
            }
        }
        let swung = motion.record(blade(1.5), at + 16, false);
        assert!(swung[0] > 0.8, "{swung:?}");
        assert!(began.is_some());
        // Held still for two seconds, it settles and the swing's start keeps counting.
        let mut still = swung;
        for frame in 1..=120 {
            still = motion.record(blade(1.5), at + 16 + frame * 16, false);
        }
        assert!(still[0] < 0.01, "{still:?}");
        assert!(still[3] > 1.9 && still[3] < 2.1, "{still:?}");
    }

    #[test]
    fn running_with_the_blade_held_still_is_no_swing() {
        let mut motion = BladeMotion::default();
        for frame in 0..60 {
            // 400 units a second along x, the blade upright.
            let moving = Blade {
                base: [frame as f32 * 6.4, 0.0, 0.0],
                ..blade(1.5)
            };
            let m = motion.record(moving, frame * 16, false);
            assert!(m[0] < 0.01 && m[3] == SETTLED, "{m:?}");
        }
    }

    #[test]
    fn a_runs_arm_sway_is_no_swing() {
        // The blade rocking 15 degrees each way three times a second, fast enough to pass
        // the swing's speed, but never turning far (Creyon's round 14).
        let mut motion = BladeMotion::default();
        for frame in 0..180 {
            let angle = 0.26 * (frame as f32 * 16.0 * 0.001 * 3.0 * std::f32::consts::TAU).sin();
            let m = motion.record(blade(angle), frame * 16, false);
            assert_eq!(m[3], SETTLED, "{frame}: {m:?}");
        }
    }

    #[test]
    fn only_an_attack_swings_when_the_saber_move_is_known() {
        // A fast quarter turn, not in an attack: no swing; the same in one: a swing.
        for attacking in [false, true] {
            let mut motion = BladeMotion::default();
            motion.record_turned(
                blade(0.0),
                blade(0.0).direction,
                0,
                false,
                Some(attacking),
                true,
            );
            let mut began = false;
            for frame in 1..=8 {
                let b = blade(frame as f32 * 0.25);
                let m =
                    motion.record_turned(b, b.direction, frame * 16, false, Some(attacking), true);
                began |= m[3] < SETTLED;
            }
            assert_eq!(began, attacking);
        }
    }

    #[test]
    fn a_blade_gone_in_lingers_for_its_words_then_not() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.5), 0, false);
        motion.record(
            Blade {
                length: 20.0,
                ..blade(0.5)
            },
            16,
            false,
        );
        let lingering = motion.lingering(1_000, 1.4).unwrap();
        assert_eq!(lingering[0], -1.0);
        assert!(lingering[2] >= GOING_OUT);
        assert_eq!(motion.lingering(1_500, 1.4), None);
        assert_eq!(BladeMotion::default().lingering(10, 1.4), None);
    }

    #[test]
    fn switching_off_or_lighting_is_no_swing() {
        // The blade going out over a third of a second, then coming back: its length
        // changes, its direction does not (Creyon's round 13).
        let mut motion = BladeMotion::default();
        motion.record(blade(0.5), 0, false);
        for frame in 1..=40 {
            let length = 40.0 * (1.0 - (frame as f32 / 20.0)).abs();
            let m = motion.record(
                Blade {
                    length,
                    ..blade(0.5)
                },
                frame * 16,
                true,
            );
            assert!(m[0] < 0.01 && m[3] == SETTLED, "{frame}: {m:?}");
            // Going out it says so, and coming back it no longer does.
            assert_eq!(m[2] >= GOING_OUT, frame <= 20, "{frame}: {m:?}");
        }
    }

    #[test]
    fn a_long_chain_of_swings_begins_one_every_so_often() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        let mut starts = 0;
        let mut last = SETTLED;
        // Two seconds of turning without a pause.
        for frame in 1..=125 {
            let m = motion.record(blade(frame as f32 * 0.25), frame * 16, false);
            if m[3] < last {
                starts += 1;
            }
            last = m[3];
        }
        assert_eq!(starts, 2);
    }

    #[test]
    fn a_clash_counts_from_when_it_happened() {
        let mut motion = BladeMotion::default();
        motion.record(blade(0.0), 0, false);
        motion.clash(100);
        assert!((motion.record(blade(0.0), 200, false)[1] - 0.1).abs() < 1e-4);
    }

    #[test]
    fn a_clash_on_the_blade_is_its_own_and_one_far_off_is_not() {
        // Far along another axis, so other tests' clashes (the list is shared) are not ours.
        let place = |angle: f32| Blade {
            base: [5_000.0, 0.0, 0.0],
            ..blade(angle)
        };
        let mut near = BladeMotion::default();
        near.record(place(0.0), 0, false);
        note_clash([5_020.0, 4.0, 0.0]);
        assert_eq!(near.record(place(0.0), 16, false)[1], 0.0);
        let far_blade = Blade {
            base: [9_000.0, 0.0, 0.0],
            ..blade(0.0)
        };
        let mut other = BladeMotion::default();
        other.record(far_blade, 0, false);
        note_clash([5_020.0, 4.0, 0.0]);
        assert_eq!(other.record(far_blade, 16, false)[1], SETTLED);
    }

    /// Round 16: running stirs a blade as a swing does, smoothly, and standing still lets it
    /// settle back slowly; the swing itself is not set off by running.
    #[test]
    fn running_stirs_the_blade_and_standing_still_settles_it() {
        let mut motion = BladeMotion::default();
        let mut at = 0;
        motion.record(blade(0.0), at, false);
        let mut last = 0.0;
        for _ in 0..60 {
            at += 16;
            let mut running = blade(0.0);
            running.base = [at as f32 * 0.25, 0.0, 0.0];
            let m = motion.record(running, at, false);
            assert!(motion.stir() >= last, "it rises without jumping back");
            last = motion.stir();
            assert!(m[0] < 0.1, "running is no swing");
        }
        assert!(
            motion.stir() > 0.9,
            "a run stirs it fully: {}",
            motion.stir()
        );
        let base = [at as f32 * 0.25, 0.0, 0.0];
        for frame in 0..30 {
            at += 16;
            let mut still = blade(0.0);
            still.base = base;
            motion.record(still, at, false);
            assert!(motion.stir() <= last, "it falls without jumping");
            if frame == 0 {
                assert!(motion.stir() > 0.8, "and slowly");
            }
            last = motion.stir();
        }
        for _ in 0..300 {
            at += 16;
            let mut still = blade(0.0);
            still.base = base;
            motion.record(still, at, false);
        }
        assert!(motion.stir() < 0.05, "standing still settles it");
    }
}
