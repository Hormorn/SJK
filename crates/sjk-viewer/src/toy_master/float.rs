//! The players who float in a toy's show (Sol, 02/11/2026: "everyone and the toy
//! gravitation less, roaming around in space randomly maybe with the ragdoll system"; the
//! players' looks alone, their view and where the game has them unchanged). As a `float`
//! step begins, every player within the show's reach goes limp in the Ragdoll toy's body
//! ([`crate::ragdoll::Body`]) with no gravity, given a push up and away that sets it
//! tumbling, and a gentle push now and then to keep it drifting; while the toy spins,
//! the bodies turn with it. When the floating ends they fall back to the floor, and a
//! little later they are drawn standing again. Each client floats them by itself: the
//! drift is the same everywhere (it comes from the show's start and the slot), not where
//! a body bumps into another player.

use crate::ragdoll::{Body, JOINTS, STEP_MS, Start, Strike, World, unit};
use glam::Vec3;

/// The push up a body takes as it floats off, in units a second, and the most it takes
/// across; and the gentle pushes after, every [`NUDGE_EVERY`] steps.
const LIFT: (f32, f32) = (90.0, 60.0);
const ACROSS: f32 = 60.0;
const NUDGE: f32 = 45.0;
const NUDGE_EVERY: u32 = 160;
/// How long the bodies lie once they fell back, before the players are drawn standing.
pub(super) const FALLEN: f32 = 3_500.0;
/// The joints a push may strike, so that it turns the body one way or another.
const STRUCK: [u8; 6] = [0, 1, 2, 5, 8, 11];

/// One floating player.
#[derive(Clone, Debug, PartialEq)]
struct Floater {
    slot: u8,
    body: Body,
    /// The show's milliseconds when it floated off.
    began: f32,
    /// The steps whose pushes were given the body.
    nudged: u32,
    seed: u32,
}

/// The floating players of a show.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Floats {
    list: Vec<Floater>,
    /// The show's milliseconds when the floating ended (the bodies fall).
    ended: Option<f32>,
}

impl Floats {
    /// Whether any player floats or lies fallen.
    pub(crate) fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The players `players` (slot, origin, yaw in degrees, scale as
    /// [`Start::scale`]) float off at `elapsed_ms` into a show of `seed`.
    pub(crate) fn begin(&mut self, players: &[(u8, Vec3, f32, u16)], elapsed_ms: f32, seed: u32) {
        self.ended = None;
        for &(slot, origin, yaw, scale) in players {
            if self.list.iter().any(|floater| floater.slot == slot) {
                continue;
            }
            let seed = seed ^ (u32::from(slot) + 1).wrapping_mul(0x2545_f491);
            let start = Start {
                moment: seed as i32,
                origin: origin.round().as_ivec3().to_array(),
                velocity: [0; 3],
                yaw: yaw.rem_euclid(360.0) as u16,
                scale,
            };
            let mut body = Body::new(&start);
            body.weightless(true);
            let across = Vec3::new(unit(seed, 11) - 0.5, unit(seed, 12) - 0.5, 0.0) * 2.0 * ACROSS;
            let push = across + Vec3::Z * (LIFT.0 + LIFT.1 * unit(seed, 13));
            body.strike(Strike {
                step: 0,
                joint: STRUCK[(unit(seed, 14) * STRUCK.len() as f32) as usize % STRUCK.len()],
                push: push.round().as_ivec3().to_array(),
            });
            self.list.push(Floater {
                slot,
                body,
                began: elapsed_ms,
                nudged: 0,
                seed,
            });
        }
    }

    /// Run the bodies on to `elapsed_ms` against `world`: floating while `floating`, else
    /// falling, and gone [`FALLEN`] after the floating ended.
    pub(crate) fn run(&mut self, elapsed_ms: f32, floating: bool, world: &mut impl World) {
        if self.list.is_empty() {
            return;
        }
        if !floating && self.ended.is_none() {
            self.ended = Some(elapsed_ms);
            for floater in &mut self.list {
                floater.body.weightless(false);
            }
        }
        if self.ended.is_some_and(|ended| elapsed_ms - ended > FALLEN) {
            self.list.clear();
            self.ended = None;
            return;
        }
        for floater in &mut self.list {
            let steps = ((elapsed_ms - floater.began).max(0.0) / STEP_MS as f32) as u32;
            // The gentle pushes due by then, while floating.
            while floating && floater.nudged + NUDGE_EVERY <= steps {
                floater.nudged += NUDGE_EVERY;
                let n = floater.nudged / NUDGE_EVERY;
                let seed = floater.seed;
                let way = Vec3::new(
                    unit(seed, 100 + n * 4) - 0.5,
                    unit(seed, 101 + n * 4) - 0.5,
                    (unit(seed, 102 + n * 4) - 0.5) * 0.6,
                )
                .normalize_or(Vec3::Z);
                let joint =
                    STRUCK[(unit(seed, 103 + n * 4) * STRUCK.len() as f32) as usize % STRUCK.len()];
                floater.body.strike(Strike {
                    step: floater.nudged.max(floater.body.steps()),
                    joint,
                    push: (way * NUDGE).round().as_ivec3().to_array(),
                });
            }
            floater.body.run_to(steps, world);
        }
    }

    /// The bodies to draw, by slot, each turned `twirl` degrees about its middle (while
    /// the toy spins as they float).
    pub(crate) fn shown(&self, twirl: f32) -> impl Iterator<Item = (u8, [Vec3; JOINTS])> + '_ {
        let turn = glam::Quat::from_rotation_z(twirl.to_radians());
        self.list.iter().map(move |floater| {
            let points = *floater.body.points();
            let middle = points.iter().copied().sum::<Vec3>() / JOINTS as f32;
            (
                floater.slot,
                points.map(|point| middle + turn * (point - middle)),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ragdoll::STOCK_SCALE;

    /// A floor at z = 0 and nothing else.
    fn floor(from: Vec3, to: Vec3, half: f32) -> Option<(Vec3, Vec3)> {
        (to.z - half < 0.0 && from.z - half >= 0.0).then(|| {
            let u = (from.z - half) / (from.z - to.z);
            (from.lerp(to, u), Vec3::Z)
        })
    }

    fn middle(floats: &Floats) -> Vec3 {
        let (_, points) = floats.shown(0.0).next().unwrap();
        points.iter().copied().sum::<Vec3>() / JOINTS as f32
    }

    /// A player floats up off the floor and drifts, then falls back once the floating
    /// ends, and is drawn standing again a little later.
    #[test]
    fn a_player_floats_up_falls_back_and_stands_again() {
        let mut floats = Floats::default();
        floats.begin(
            &[(3, Vec3::new(0.0, 0.0, 24.0), 90.0, STOCK_SCALE)],
            1_000.0,
            42,
        );
        floats.begin(
            &[(3, Vec3::new(0.0, 0.0, 24.0), 90.0, STOCK_SCALE)],
            1_100.0,
            42,
        );
        assert_eq!(floats.list.len(), 1);
        floats.run(4_000.0, true, &mut floor);
        let up = middle(&floats).z;
        assert!(up > 60.0, "{up}");
        floats.run(20_000.0, true, &mut floor);
        assert!(middle(&floats).z > 20.0);
        floats.run(22_000.0, false, &mut floor);
        floats.run(23_000.0, false, &mut floor);
        assert!(middle(&floats).z < 15.0, "{}", middle(&floats).z);
        floats.run(22_000.0 + FALLEN + 1.0, false, &mut floor);
        assert!(floats.is_empty());
    }

    /// The bodies turn about their middle with the twirl.
    #[test]
    fn a_twirl_turns_the_body_about_its_middle() {
        let mut floats = Floats::default();
        floats.begin(&[(1, Vec3::new(10.0, 0.0, 24.0), 0.0, STOCK_SCALE)], 0.0, 7);
        let before = middle(&floats);
        let (_, turned) = floats.shown(90.0).next().unwrap();
        let after = turned.iter().copied().sum::<Vec3>() / JOINTS as f32;
        assert!((before - after).length() < 1e-3);
        let (_, still) = floats.shown(0.0).next().unwrap();
        assert_ne!(still, turned);
    }
}
