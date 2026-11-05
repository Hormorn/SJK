//! Dynamic joints: the chains a rig model's manifest names (`dynamic <joint>`, the
//! joints under that one, such as a tail) move on their own. Each simulated joint is
//! a point pulled by a spring towards where the animation puts it, so it lags and
//! bounces when the body moves, held at its bone's length from its parent; a gentle
//! side-to-side wag, growing towards the tip, keeps it alive while the player stands,
//! and gravity, more towards the tip, makes it hang (a tail's rest pose sticks out
//! straight; the game it comes from lets physics pull it down).
//! Then each parent is turned to point at its simulated child. The state is the
//! actor's ([`super::Scratch`]); it all happens in the model's space, so the chain
//! answers the animation (steps, jumps, turns of the hips), not the player's
//! movement through the world.
//!
//! Each chain has its own [`Settings`] (a cape: no wag, stiffer), and its points are
//! pushed out of the body's [`Capsule`]s (trunk and legs, measured once from the mesh)
//! so a tail or a cape swings past the legs rather than through them.

use super::Joint;
use glam::{Affine3A, Mat3A, Quat, Vec3A};

/// The spring pulling a joint towards its animated place, per second squared.
pub(super) const STIFFNESS: f32 = 70.0;
/// How fast the swing dies down, per second.
pub(super) const DAMPING: f32 = 7.0;
/// The wag at the chain's tip, game units to each side, and how many a second.
pub(super) const WAG_UNITS: f32 = 2.5;
const WAG_HERTZ: f32 = 0.7;
/// The pull down on a chain's tip, game units per second squared, unless its
/// `dynamic` line gives one: twice the game's (`g_gravity` 800), which only levelled
/// the test model's tail (its rest pose rises 22 degrees); this hangs it about 29 degrees
/// below level, the tip some 12 units under its anchor. Less towards the anchor.
pub(super) const GRAVITY: f32 = 1600.0;
/// A frame step longer than this (a hitch, a teleport, the first frame) starts the
/// chain again at the animated pose.
const MAX_STEP: f32 = 0.1;

/// One `dynamic` line's chain: how it hangs, swings and wags, and whether the body
/// pushes it away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Settings {
    /// The pull down at the tip, game units per second squared.
    pub(crate) gravity: f32,
    /// The side-to-side wag at the tip, game units (0: none, as for a cape).
    pub(crate) wag: f32,
    /// The spring towards the animated place, per second squared.
    pub(crate) stiffness: f32,
    /// How fast the swing dies down, per second.
    pub(crate) damping: f32,
    /// Whether the body's capsules push the chain's points out.
    pub(crate) collide: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gravity: GRAVITY,
            wag: WAG_UNITS,
            stiffness: STIFFNESS,
            damping: DAMPING,
            collide: true,
        }
    }
}

/// A part of the body dynamic chains are kept out of: the segment between two
/// joints, thickened to `radius` (game units), measured in the bind pose from the
/// vertices the first joint carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Capsule {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) radius: f32,
}

/// At most this many capsules (one bit each in a chain joint's mask).
const MAX_CAPSULES: usize = 64;
/// The share of a capsule's vertices that lie inside it: low, so it stays under the
/// skin (clothes, fur tufts and muscles stick out of a round section).
const RADIUS_PERCENTILE: f32 = 0.25;
/// Fewer vertices than this on a joint make no capsule.
const MIN_SAMPLES: usize = 8;

impl Capsule {
    /// The capsule from joint `from` to `to` in the bind pose round `vertices`
    /// (bind-pose positions), `None` when there are too few or it is degenerate.
    pub(super) fn measure(
        joints: &[Joint],
        from: usize,
        to: usize,
        vertices: &[Vec3A],
    ) -> Option<Self> {
        if vertices.len() < MIN_SAMPLES || from == to {
            return None;
        }
        let a = joints[from].bind.translation;
        let b = joints[to].bind.translation;
        let mut distances: Vec<f32> = vertices
            .iter()
            .map(|&point| (point - closest(a, b, point)).length())
            .collect();
        distances.sort_by(f32::total_cmp);
        let at = ((distances.len() - 1) as f32 * RADIUS_PERCENTILE) as usize;
        let radius = distances[at];
        (radius > 0.0).then_some(Self { from, to, radius })
    }
}

/// The point of segment `a`-`b` nearest `point`.
fn closest(a: Vec3A, b: Vec3A, point: Vec3A) -> Vec3A {
    let ab = b - a;
    let length = ab.length_squared();
    if length <= f32::EPSILON {
        return a;
    }
    a + ab * ((point - a).dot(ab) / length).clamp(0.0, 1.0)
}

/// The simulated joints of a rig, parents before children.
#[derive(Clone, Debug, Default)]
pub(super) struct Chains {
    pub(super) joints: Vec<usize>,
    /// How far along its chain each one is, 0 at the anchor to 1 at the tip.
    along: Vec<f32>,
    /// Its chain's settings.
    settings: Vec<Settings>,
    /// Its chain's anchor joint.
    anchor: Vec<usize>,
    /// The capsules each one ignores (a bit per capsule): its anchor's, and those it
    /// already lies in at rest.
    ignored: Vec<u64>,
    /// The body's parts the chains are kept out of.
    pub(super) capsules: Vec<Capsule>,
}

impl Chains {
    /// The joints under each of `anchors` (not the anchors: they follow the
    /// animation and hold the chain), in `order`, each anchor with its chain's
    /// settings.
    pub(super) fn under(joints: &[Joint], order: &[usize], anchors: &[(usize, Settings)]) -> Self {
        let below = |mut index: usize| -> Option<(usize, usize, Settings)> {
            let mut depth = 0;
            while let Some(parent) = joints[index].parent {
                depth += 1;
                if let Some((anchor, settings)) =
                    anchors.iter().find(|(anchor, _)| *anchor == parent)
                {
                    return Some((depth, *anchor, *settings));
                }
                index = parent;
            }
            None
        };
        let found: Vec<(usize, usize, usize, Settings)> = order
            .iter()
            .filter_map(|&index| {
                below(index).map(|(depth, anchor, settings)| (index, depth, anchor, settings))
            })
            .collect();
        let deepest = found.iter().map(|(_, depth, ..)| *depth).max().unwrap_or(1) as f32;
        Self {
            joints: found.iter().map(|(index, ..)| *index).collect(),
            along: found
                .iter()
                .map(|(_, depth, ..)| *depth as f32 / deepest)
                .collect(),
            settings: found.iter().map(|(.., settings)| *settings).collect(),
            anchor: found.iter().map(|(_, _, anchor, _)| *anchor).collect(),
            ignored: vec![0; found.len()],
            capsules: Vec::new(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// Keep the chains out of `capsules` (the first [`MAX_CAPSULES`]; those touching
    /// a chain's joints are dropped). Each joint ignores its anchor's capsules and
    /// those it already lies in at rest, so a cape hanging from the back is not
    /// shoved off it.
    pub(super) fn collide_with(&mut self, joints: &[Joint], capsules: Vec<Capsule>) {
        self.capsules = capsules
            .into_iter()
            .filter(|capsule| {
                !self.joints.contains(&capsule.from) && !self.joints.contains(&capsule.to)
            })
            .take(MAX_CAPSULES)
            .collect();
        for slot in 0..self.joints.len() {
            let point = joints[self.joints[slot]].bind.translation;
            let mut ignored = 0u64;
            for (bit, capsule) in self.capsules.iter().enumerate() {
                let a = joints[capsule.from].bind.translation;
                let b = joints[capsule.to].bind.translation;
                let inside = (point - closest(a, b, point)).length() < capsule.radius;
                if capsule.from == self.anchor[slot] || inside {
                    ignored |= 1 << bit;
                }
            }
            self.ignored[slot] = ignored;
        }
    }
}

/// One actor's chain state.
#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    points: Vec<Vec3A>,
    velocities: Vec<Vec3A>,
    /// Seconds simulated, for the wag.
    time: f32,
    /// The animated pose before the chain moved it (scratch).
    animated: Vec<Affine3A>,
    aimed: Vec<bool>,
}

/// Move `chains` in `pose` by `step` seconds (`None`: start again at the animated
/// pose).
pub(super) fn simulate(
    joints: &[Joint],
    chains: &Chains,
    pose: &mut [Affine3A],
    state: &mut State,
    step: Option<f32>,
) {
    let count = chains.joints.len();
    let step = step.filter(|step| *step > 0.0 && *step <= MAX_STEP);
    let restart = step.is_none() || state.points.len() != count;
    let step = step.unwrap_or(0.0);
    state.time += step;
    let phase = std::f32::consts::TAU * WAG_HERTZ * state.time;
    let target = |slot: usize, pose: &[Affine3A]| {
        let along = chains.along[slot];
        let wag = chains.settings[slot].wag;
        // Model space: +X is the player's side (-Y the front, +Z up). The tip trails
        // the base a little, so the wag runs down the chain.
        pose[chains.joints[slot]].translation + Vec3A::X * wag * along * (phase - along * 0.6).sin()
    };
    // Gravity on each point, more towards the tip.
    let weight = |slot: usize| Vec3A::NEG_Z * chains.settings[slot].gravity * chains.along[slot];
    if restart {
        // Already hanging: where the spring holds the weight.
        state.points = (0..count)
            .map(|slot| target(slot, pose) + weight(slot) / chains.settings[slot].stiffness)
            .collect();
        state.velocities = vec![Vec3A::ZERO; count];
    } else {
        for slot in 0..count {
            let pull = target(slot, pose) - state.points[slot];
            let Settings {
                stiffness, damping, ..
            } = chains.settings[slot];
            let velocity = &mut state.velocities[slot];
            *velocity += (pull * stiffness - *velocity * damping + weight(slot)) * step;
            state.points[slot] += *velocity * step;
        }
    }
    // Each point at its bone's length from its parent's (simulated or animated),
    // pushed out of the body, then at its length again.
    for slot in 0..count {
        let index = chains.joints[slot];
        let Some(parent) = joints[index].parent else {
            continue;
        };
        let anchor = chains
            .joints
            .iter()
            .position(|&joint| joint == parent)
            .map_or(pose[parent].translation, |at| state.points[at]);
        let bone = pose[index].translation - pose[parent].translation;
        let at_length = |point: Vec3A| {
            let towards = (point - anchor)
                .try_normalize()
                .or_else(|| bone.try_normalize())
                .unwrap_or(Vec3A::Z);
            anchor + towards * bone.length()
        };
        state.points[slot] = at_length(state.points[slot]);
        if chains.settings[slot].collide
            && push_out(
                chains,
                slot,
                pose,
                &mut state.points[slot],
                &mut state.velocities[slot],
            )
        {
            state.points[slot] = at_length(state.points[slot]);
        }
    }
    // Each parent turned to point at its simulated child; the children keep their
    // animated place under it.
    state.animated.clear();
    state.animated.extend_from_slice(pose);
    state.aimed.clear();
    state.aimed.resize(pose.len(), false);
    for slot in 0..count {
        let index = chains.joints[slot];
        let Some(parent) = joints[index].parent else {
            continue;
        };
        let local = state.animated[parent].inverse() * state.animated[index];
        if !state.aimed[parent] {
            let origin = pose[parent].translation;
            let now = ((pose[parent] * local).translation - origin).try_normalize();
            let wanted = (state.points[slot] - origin).try_normalize();
            if let (Some(now), Some(wanted)) = (now, wanted) {
                let turn = Mat3A::from_quat(Quat::from_rotation_arc(now.into(), wanted.into()));
                pose[parent].matrix3 = turn * pose[parent].matrix3;
            }
            state.aimed[parent] = true;
        }
        pose[index] = pose[parent] * local;
    }
}

/// Push `point` out of the capsules `slot` does not ignore, in `pose`, and stop its
/// `velocity` going further in. Whether it moved.
fn push_out(
    chains: &Chains,
    slot: usize,
    pose: &[Affine3A],
    point: &mut Vec3A,
    velocity: &mut Vec3A,
) -> bool {
    let mut moved = false;
    for (bit, capsule) in chains.capsules.iter().enumerate() {
        if chains.ignored[slot] & (1 << bit) != 0 {
            continue;
        }
        let near = closest(
            pose[capsule.from].translation,
            pose[capsule.to].translation,
            *point,
        );
        let offset = *point - near;
        let distance = offset.length();
        if distance >= capsule.radius || distance <= 1e-4 {
            continue;
        }
        let normal = offset / distance;
        *point = near + normal * capsule.radius;
        let into = velocity.dot(normal);
        if into < 0.0 {
            *velocity -= normal * into;
        }
        moved = true;
    }
    moved
}

#[cfg(test)]
mod tests {
    use super::super::clips::Rest;
    use super::*;

    /// A joint at `at` under `parent` (whose bind is `above`).
    fn joint(parent: Option<usize>, above: Vec3A, at: Vec3A) -> Joint {
        let bind = Affine3A::from_translation(at.into());
        let local = Affine3A::from_translation((at - above).into());
        Joint {
            name: String::new(),
            parent,
            bind,
            inverse_bind: bind.inverse(),
            local,
            rest: Rest::of(&local),
            above: Affine3A::IDENTITY,
        }
    }

    /// A leg from a hip 40 units up to a knee on the ground, and a 20-unit pendulum
    /// hanging 12 units to its side from a hip-held anchor.
    fn leg_and_pendulum() -> Vec<Joint> {
        let hip = Vec3A::new(0.0, 0.0, 40.0);
        let anchor = Vec3A::new(12.0, 0.0, 40.0);
        vec![
            joint(None, Vec3A::ZERO, hip),
            joint(Some(0), hip, Vec3A::ZERO),
            joint(Some(0), hip, anchor),
            joint(Some(2), anchor, Vec3A::new(12.0, 0.0, 20.0)),
        ]
    }

    fn ring(centre: Vec3A, radius: f32) -> Vec<Vec3A> {
        (0..12)
            .map(|step| {
                let angle = step as f32 * std::f32::consts::TAU / 12.0;
                centre + Vec3A::new(angle.cos(), angle.sin(), 0.0) * radius
            })
            .collect()
    }

    #[test]
    fn a_capsule_sits_inside_the_vertices_round_its_joint() {
        let joints = leg_and_pendulum();
        let mut around = ring(Vec3A::new(0.0, 0.0, 30.0), 6.0);
        around.extend(ring(Vec3A::new(0.0, 0.0, 10.0), 6.0));
        let capsule = Capsule::measure(&joints, 0, 1, &around).expect("a capsule");
        assert!((capsule.radius - 6.0).abs() < 1e-3, "{capsule:?}");
        assert!(Capsule::measure(&joints, 0, 1, &around[..3]).is_none());
    }

    /// The pendulum's tip after the knee swings out under it, with `settings`.
    fn tip_after_the_leg_swings(settings: Settings) -> (Vec3A, Vec3A, Vec3A) {
        let joints = leg_and_pendulum();
        let order = [0, 1, 2, 3];
        let mut chains = Chains::under(&joints, &order, &[(2, settings)]);
        chains.collide_with(
            &joints,
            vec![Capsule {
                from: 0,
                to: 1,
                radius: 6.0,
            }],
        );
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        simulate(&joints, &chains, &mut pose, &mut state, None);
        // The knee kicks out sideways: the leg now runs through the pendulum's tip.
        let mut kicked = rest.clone();
        kicked[1].translation = Vec3A::new(12.0, 0.0, 10.0);
        for _ in 0..30 {
            pose.clone_from(&kicked);
            simulate(&joints, &chains, &mut pose, &mut state, Some(1.0 / 60.0));
        }
        (
            pose[3].translation,
            kicked[0].translation,
            kicked[1].translation,
        )
    }

    #[test]
    fn the_body_pushes_a_chain_out_unless_it_says_nocollide() {
        let still = Settings {
            gravity: 0.0,
            wag: 0.0,
            ..Settings::default()
        };
        let (tip, hip, knee) = tip_after_the_leg_swings(still);
        let distance = (tip - closest(hip, knee, tip)).length();
        assert!(distance > 5.5, "pushed out: {distance} {tip:?}");
        // The bone keeps its length.
        assert!(((tip - Vec3A::new(12.0, 0.0, 40.0)).length() - 20.0).abs() < 1e-3);
        let (tip, hip, knee) = tip_after_the_leg_swings(Settings {
            collide: false,
            ..still
        });
        let distance = (tip - closest(hip, knee, tip)).length();
        assert!(distance < 4.5, "left inside: {distance} {tip:?}");
    }

    #[test]
    fn a_joint_ignores_its_anchors_capsule_and_those_it_starts_in() {
        let joints = leg_and_pendulum();
        let mut chains = Chains::under(&joints, &[0, 1, 2, 3], &[(2, Settings::default())]);
        let capsule = |from, to, radius| Capsule { from, to, radius };
        chains.collide_with(
            &joints,
            vec![capsule(0, 1, 6.0), capsule(2, 0, 2.0), capsule(0, 1, 15.0)],
        );
        // The second is the anchor's, the third already holds the tip at rest.
        assert_eq!(chains.ignored, [0b110]);
        // A capsule on a chain joint is dropped.
        chains.collide_with(&joints, vec![capsule(3, 1, 6.0)]);
        assert!(chains.capsules.is_empty());
    }
}
