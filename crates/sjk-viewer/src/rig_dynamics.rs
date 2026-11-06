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
/// A bone shorter than this, in game units, has no direction to aim along: some rigs
/// stack joints on one spot, and their offsets are only the bind matrices' rounding.
const MIN_BONE: f32 = 0.01;
/// How fast a limited joint takes up a push from the body or a chain under it, per second.
/// Its short bones would turn far in a frame at the push's full step.
const PUSH_RATE: f32 = 12.0;
/// How fast a limited joint's bone takes up its spring's lag, per second. A change of
/// animation moves the body far in a tenth of a second; at once, every bone down a chain
/// would tilt with it and the tip would jump.
const TILT_RATE: f32 = 14.0;
/// A [`Settings::limit`] that never holds a bone back.
pub(super) const NO_LIMIT: f32 = 180.0;
/// How far a limited joint's spring lags, game units, to tilt its bone half its limit.
const LIMIT_HALF_LAG: f32 = 2.0;

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
    /// The angle a bone's tilt from its animated direction under its parent approaches,
    /// degrees, never reaching it (the default, [`NO_LIMIT`], lets it swing freely, its
    /// spring's point at its bone's length). The spring's lag is in game units, so a
    /// short bone would otherwise swing far on a small step.
    pub(crate) limit: f32,
    /// How far outside the body's capsules the chain stays, game units: they sit under
    /// the skin, so cloth that should clear it asks for a little more.
    pub(crate) clearance: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            gravity: GRAVITY,
            wag: WAG_UNITS,
            stiffness: STIFFNESS,
            damping: DAMPING,
            collide: true,
            limit: NO_LIMIT,
            clearance: 0.0,
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
pub(super) fn closest(a: Vec3A, b: Vec3A, point: Vec3A) -> Vec3A {
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
    pub(super) ignored: Vec<u64>,
    /// The body's parts the chains are kept out of.
    pub(super) capsules: Vec<Capsule>,
    /// How far out of each capsule each joint stays: a row of [`Self::capsules`] per joint.
    reach: Vec<f32>,
    /// For a limited joint with no bone of its own to aim (stacked on its parent, with no
    /// children): from its pivot to the middle of the vertices it carries, in its own
    /// frame. It swings about the pivot along this. A rig's single-joint hair strand.
    levers: Vec<Option<Vec3A>>,
    /// The other chain's bone each joint stays on one side of (`layer` lines).
    layers: Vec<Option<Layer>>,
}

/// Where a joint lies against a chain it is layered over: the plane through that chain's
/// nearest bone, which it stays outside. A fixed bone and a plane rather than the nearest
/// point each frame, so the push never jumps as the two slide along each other.
#[derive(Clone, Copy, Debug)]
struct Layer {
    /// The bone's parent joint: the plane goes through it, and turns with it.
    from: usize,
    /// The plane's normal, towards the joint's side, in `from`'s frame at rest.
    outward: Vec3A,
    /// How far out it stays: its chain's clearance, or as far as it lies at rest if less.
    gap: f32,
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
            reach: Vec::new(),
            levers: vec![None; found.len()],
            layers: vec![None; found.len()],
        }
    }

    /// Keep the chains under each pair's first anchor outside those under its second, as
    /// hair lying over a cape: `(outer, inner)`. Several pairs with one outer anchor make
    /// one sheet of the inner chains, side by side. The inner chains move first, so the
    /// outer ones meet them where they are drawn. Call this before the other steps.
    pub(super) fn layer(&mut self, joints: &[Joint], pairs: &[(usize, usize)]) {
        let is_inner = |anchor: usize| pairs.iter().any(|(_, inner)| *inner == anchor);
        let mut slots: Vec<usize> = (0..self.joints.len()).collect();
        // A stable sort: each chain stays parents first.
        slots.sort_by_key(|slot| !is_inner(self.anchor[*slot]));
        self.joints = slots.iter().map(|slot| self.joints[*slot]).collect();
        self.along = slots.iter().map(|slot| self.along[*slot]).collect();
        self.settings = slots.iter().map(|slot| self.settings[*slot]).collect();
        self.anchor = slots.iter().map(|slot| self.anchor[*slot]).collect();
        for slot in 0..self.joints.len() {
            let inners: Vec<usize> = pairs
                .iter()
                .filter(|(outer, _)| *outer == self.anchor[slot])
                .map(|(_, inner)| *inner)
                .collect();
            self.layers[slot] = self.layer_of(joints, slot, &inners);
        }
    }

    /// The layer of `slot` over the chains under `inners`, on the side it lies at rest;
    /// `None` when it lies beyond their ends or right on them.
    fn layer_of(&self, joints: &[Joint], slot: usize, inners: &[usize]) -> Option<Layer> {
        let columns: Vec<Vec<usize>> = inners
            .iter()
            .map(|inner| {
                (0..self.joints.len())
                    .filter(|other| self.anchor[*other] == *inner)
                    .map(|other| self.joints[other])
                    .filter(|joint| {
                        joints[*joint].parent.is_some_and(|parent| {
                            let bone =
                                joints[*joint].bind.translation - joints[parent].bind.translation;
                            bone.length() >= MIN_BONE
                        })
                    })
                    .collect()
            })
            .collect();
        let point = joints[self.joints[slot]].bind.translation;
        let mut nearest: Option<(f32, usize, usize)> = None;
        for (column, bones) in columns.iter().enumerate() {
            for row in 0..bones.len() {
                let to = joints[bones[row]].bind.translation;
                let from = joints[joints[bones[row]].parent?].bind.translation;
                let distance = (point - closest(from, to, point)).length();
                if nearest.is_none_or(|(best, ..)| distance < best) {
                    nearest = Some((distance, column, row));
                }
            }
        }
        let (_, column, row) = nearest?;
        let joint = columns[column][row];
        let from = joints[joint].parent?;
        let start = joints[from].bind.translation;
        let along = joints[joint].bind.translation - start;
        let share = (point - start).dot(along) / along.length_squared();
        if !(0.0..=1.0).contains(&share) {
            return None;
        }
        // Across the sheet where it has a width; otherwise straight at the joint.
        let across = columns
            .first()
            .zip(columns.last())
            .filter(|(first, last)| first.len() > row && last.len() > row && columns.len() > 1)
            .and_then(|(first, last)| {
                let side = joints[last[row]].bind.translation - joints[first[row]].bind.translation;
                along.cross(side).try_normalize()
            })
            .or_else(|| (point - (start + along * share)).try_normalize())?;
        let depth = (point - start).dot(across);
        let (outward, depth) = if depth >= 0.0 {
            (across, depth)
        } else {
            (-across, -depth)
        };
        (depth >= MIN_BONE).then(|| Layer {
            from,
            outward: joints[from].inverse_bind.matrix3 * outward,
            gap: self.settings[slot].clearance.min(depth),
        })
    }

    /// The joints that swing on a lever, with it.
    #[cfg(test)]
    pub(super) fn swings(&self) -> Vec<(usize, Vec3A)> {
        self.joints
            .iter()
            .zip(&self.levers)
            .filter_map(|(&joint, lever)| Some((joint, (*lever)?)))
            .collect()
    }

    /// Find the levers of the limited joints that have no bone of their own, from each
    /// skin weight's joint, bind-pose position and share.
    pub(super) fn swing_from(
        &mut self,
        joints: &[Joint],
        vertices: impl Iterator<Item = (usize, Vec3A, f32)>,
    ) {
        // Only limited chains swing, so a rig without any does not walk its vertices.
        if self
            .settings
            .iter()
            .all(|settings| settings.limit >= NO_LIMIT)
        {
            return;
        }
        let mut slot_of = vec![None; joints.len()];
        for (slot, &joint) in self.joints.iter().enumerate() {
            slot_of[joint] = Some(slot);
        }
        let mut carried = vec![(Vec3A::ZERO, 0.0f32); self.joints.len()];
        for (joint, position, weight) in vertices {
            if let Some(slot) = slot_of.get(joint).copied().flatten() {
                carried[slot].0 += position * weight;
                carried[slot].1 += weight;
            }
        }
        for (slot, &(sum, count)) in carried.iter().enumerate() {
            let index = self.joints[slot];
            let joint = &joints[index];
            let stacked = joint.parent.is_some_and(|parent| {
                (joint.bind.translation - joints[parent].bind.translation).length() < MIN_BONE
            });
            let leaf = !joints.iter().any(|other| other.parent == Some(index));
            self.levers[slot] =
                (self.settings[slot].limit < NO_LIMIT && stacked && leaf && count > 0.0)
                    .then(|| sum / count - joint.bind.translation)
                    .filter(|lever| lever.length() >= MIN_BONE)
                    .map(|lever| joint.inverse_bind.matrix3 * lever);
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    /// How far each layered joint stands outside its plane in `pose`, less its gap:
    /// below zero when it has got through.
    #[cfg(test)]
    pub(super) fn layer_margins(&self, pose: &[Affine3A]) -> Vec<f32> {
        self.joints
            .iter()
            .zip(&self.layers)
            .filter_map(|(joint, layer)| {
                let layer = (*layer)?;
                let outward = (pose[layer.from].matrix3 * layer.outward).normalize_or_zero();
                let offset = pose[*joint].translation - pose[layer.from].translation;
                Some(offset.dot(outward) - layer.gap)
            })
            .collect()
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
        self.reach.clear();
        for slot in 0..self.joints.len() {
            let joint = &joints[self.joints[slot]];
            let point = joint.bind.translation;
            let middle = joint.parent.map_or(point, |parent| {
                (joints[parent].bind.translation + point) / 2.0
            });
            let mut ignored = 0u64;
            for (bit, capsule) in self.capsules.iter().enumerate() {
                let a = joints[capsule.from].bind.translation;
                let b = joints[capsule.to].bind.translation;
                let away = |point: Vec3A| (point - closest(a, b, point)).length();
                let rest = away(point).min(away(middle));
                if capsule.from == self.anchor[slot] || rest < capsule.radius {
                    ignored |= 1 << bit;
                }
                // Out by the clearance, but no further than it lies at rest: standing
                // still should not move it.
                let wanted = capsule.radius + self.settings[slot].clearance;
                self.reach.push(wanted.min(rest.max(capsule.radius)));
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
    /// The push each limited joint has taken up so far.
    pushes: Vec<Vec3A>,
    /// How far each limited joint's spring lags, as its bone has taken it up so far.
    lags: Vec<Vec3A>,
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
        state.pushes = vec![Vec3A::ZERO; count];
        state.lags = vec![Vec3A::ZERO; count];
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
    state.animated.clear();
    state.animated.extend_from_slice(pose);
    state.aimed.clear();
    state.aimed.resize(pose.len(), false);
    // Parents first, so each joint's parent already stands where it will be drawn.
    for slot in 0..count {
        let index = chains.joints[slot];
        let Some(parent) = joints[index].parent else {
            continue;
        };
        let local = state.animated[parent].inverse() * state.animated[index];
        let length = local.translation.length();
        let origin = pose[parent].translation;
        // Each point at its bone's length from where its parent stands, pushed out of
        // the body, then at its length again.
        let animated = (pose[parent] * local).translation - origin;
        let at_length = |point: Vec3A| {
            let towards = (point - origin)
                .try_normalize()
                .or_else(|| animated.try_normalize())
                .unwrap_or(Vec3A::Z);
            origin + towards * length
        };
        // A limited joint's spring runs free of the bone: the bone only tilts by how far
        // the spring lags, so it cannot snap from side to side as the lag turns round.
        let limit = chains.settings[slot].limit;
        let limited = limit < NO_LIMIT;
        let lag = if limited {
            let raw = state.points[slot] - state.animated[index].translation;
            let share = if restart {
                1.0
            } else {
                1.0 - (-step * TILT_RATE).exp()
            };
            let taken = &mut state.lags[slot];
            *taken += (raw - *taken) * share;
            *taken
        } else {
            Vec3A::ZERO
        };
        let mut point = if limited {
            origin + tilted(animated, lag, limit)
        } else {
            at_length(state.points[slot])
        };
        let free = point;
        if chains.settings[slot].collide
            && push_out(
                chains,
                slot,
                pose,
                origin,
                &mut point,
                &mut state.velocities[slot],
            )
        {
            point = at_length(point);
        }
        if let Some(layer) = chains.layers[slot] {
            let outward = (pose[layer.from].matrix3 * layer.outward).normalize_or_zero();
            let depth = (point - pose[layer.from].translation).dot(outward);
            if depth < layer.gap {
                point = at_length(point + outward * (layer.gap - depth));
                let velocity = &mut state.velocities[slot];
                let into = velocity.dot(outward);
                if into < 0.0 {
                    *velocity -= outward * into;
                }
            }
        }
        if limited {
            // At once on a restart; otherwise eased in, so a short bone does not jump.
            let share = if restart {
                1.0
            } else {
                1.0 - (-step * PUSH_RATE).exp()
            };
            let push = &mut state.pushes[slot];
            *push += (point - free - *push) * share;
            point = at_length(free + *push);
        }
        // The parent turned to point at its first real child. A bone with no length
        // gives no direction, only rounding noise, so it turns nothing and leaves the
        // parent to the next child.
        if length >= MIN_BONE && !state.aimed[parent] {
            let now = animated.try_normalize();
            let wanted = (point - origin).try_normalize();
            if let (Some(now), Some(wanted)) = (now, wanted) {
                let turn = Mat3A::from_quat(Quat::from_rotation_arc(now.into(), wanted.into()));
                pose[parent].matrix3 = turn * pose[parent].matrix3;
            }
            state.aimed[parent] = true;
        }
        pose[index] = pose[parent] * local;
        // A joint with no bone to aim swings itself about its pivot.
        if let Some(lever) = chains.levers[slot] {
            let now = pose[index].matrix3 * lever;
            let wanted = tilted(now, lag, limit);
            if let (Some(now), Some(wanted)) = (now.try_normalize(), wanted.try_normalize()) {
                let turn = Mat3A::from_quat(Quat::from_rotation_arc(now.into(), wanted.into()));
                pose[index].matrix3 = turn * pose[index].matrix3;
            }
        }
        // The joint's point is where it now stands: another branch of a fork follows
        // its parent's turn, and its own chain goes on from there.
        if !limited {
            state.points[slot] = pose[index].translation;
        }
    }
}

/// The bone `animated` tilted towards `lag`, at its length. The tilt grows smoothly with
/// the lag across the bone, from none to nearly `limit` degrees, so it passes through no
/// jump as the lag changes direction.
fn tilted(animated: Vec3A, lag: Vec3A, limit: f32) -> Vec3A {
    let Some(along) = animated.try_normalize() else {
        return animated;
    };
    let across = lag - along * lag.dot(along);
    let Some(side) = across.try_normalize() else {
        return animated;
    };
    let lagging = across.length();
    let angle = limit.to_radians() * lagging / (lagging + LIMIT_HALF_LAG);
    let (sin, cos) = angle.sin_cos();
    (along * cos + side * sin) * animated.length()
}

/// Push `point`, and the middle of its bone from `origin`, out of the capsules `slot` does
/// not ignore, in `pose`, and stop its `velocity` going further in. Whether it moved.
fn push_out(
    chains: &Chains,
    slot: usize,
    pose: &[Affine3A],
    origin: Vec3A,
    point: &mut Vec3A,
    velocity: &mut Vec3A,
) -> bool {
    let count = chains.capsules.len();
    let reach = &chains.reach[slot * count..(slot + 1) * count];
    let mut moved = false;
    for (bit, capsule) in chains.capsules.iter().enumerate() {
        if chains.ignored[slot] & (1 << bit) != 0 {
            continue;
        }
        let radius = reach[bit];
        let from = pose[capsule.from].translation;
        let to = pose[capsule.to].translation;
        // The tip, then the middle: moving the tip moves the middle half as far, so the
        // middle's way out asks for twice its depth.
        for (share, leverage) in [(1.0, 1.0), (0.5, 2.0)] {
            let sample = origin + (*point - origin) * share;
            let offset = sample - closest(from, to, sample);
            let distance = offset.length();
            if distance >= radius || distance <= 1e-4 {
                continue;
            }
            let normal = offset / distance;
            *point += normal * (radius - distance) * leverage;
            let into = velocity.dot(normal);
            if into < 0.0 {
                *velocity -= normal * into;
            }
            moved = true;
        }
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
    fn a_chains_clearance_holds_it_further_from_the_body() {
        let (tip, hip, knee) = tip_after_the_leg_swings(Settings {
            gravity: 0.0,
            wag: 0.0,
            clearance: 3.0,
            ..Settings::default()
        });
        let distance = (tip - closest(hip, knee, tip)).length();
        assert!(distance > 8.5, "kept out: {distance} {tip:?}");
    }

    #[test]
    fn a_chains_clearance_does_not_move_it_at_rest() {
        let joints = leg_and_pendulum();
        let mut chains = Chains::under(
            &joints,
            &[0, 1, 2, 3],
            &[(
                2,
                Settings {
                    gravity: 0.0,
                    wag: 0.0,
                    clearance: 5.0,
                    ..Settings::default()
                },
            )],
        );
        // The pendulum hangs 12 units from the leg: inside radius and clearance.
        let capsule = Capsule {
            from: 0,
            to: 1,
            radius: 10.0,
        };
        chains.collide_with(&joints, vec![capsule]);
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        simulate(&joints, &chains, &mut pose, &mut state, None);
        for _ in 0..30 {
            pose.clone_from(&rest);
            simulate(&joints, &chains, &mut pose, &mut state, Some(1.0 / 60.0));
        }
        assert!((pose[3].translation - rest[3].translation).length() < 1e-2);
    }

    #[test]
    fn a_long_bones_middle_stays_out_of_a_capsule_its_ends_miss() {
        // A rod across the pendulum's middle: both its ends lie well clear of it.
        let anchor = Vec3A::new(12.0, 0.0, 40.0);
        let tip = Vec3A::new(12.0, 0.0, 20.0);
        let joints = vec![
            joint(None, Vec3A::ZERO, Vec3A::new(0.0, -10.0, 30.0)),
            joint(
                Some(0),
                Vec3A::new(0.0, -10.0, 30.0),
                Vec3A::new(0.0, 10.0, 30.0),
            ),
            joint(Some(0), Vec3A::new(0.0, -10.0, 30.0), anchor),
            joint(Some(2), anchor, tip),
        ];
        let mut chains = Chains::under(
            &joints,
            &[0, 1, 2, 3],
            &[(
                2,
                Settings {
                    gravity: 0.0,
                    wag: 0.0,
                    ..Settings::default()
                },
            )],
        );
        let capsule = Capsule {
            from: 0,
            to: 1,
            radius: 3.0,
        };
        chains.collide_with(&joints, vec![capsule]);
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        simulate(&joints, &chains, &mut pose, &mut state, None);
        let mut moved = rest;
        moved[0].translation.x = 13.0;
        moved[1].translation.x = 13.0;
        for _ in 0..60 {
            pose.clone_from(&moved);
            simulate(&joints, &chains, &mut pose, &mut state, Some(1.0 / 60.0));
        }
        let middle = (pose[2].translation + pose[3].translation) / 2.0;
        let near = closest(pose[0].translation, pose[1].translation, middle);
        assert!((middle - near).length() > 2.5, "{middle:?}");
    }

    #[test]
    fn a_chain_layered_over_another_stays_on_its_side() {
        let inner = Vec3A::new(0.0, 0.0, 40.0);
        let outer = Vec3A::new(0.0, 3.0, 40.0);
        let joints = vec![
            joint(None, Vec3A::ZERO, Vec3A::ZERO),
            joint(Some(0), Vec3A::ZERO, inner),
            joint(Some(1), inner, Vec3A::new(0.0, 0.0, 20.0)),
            joint(Some(0), Vec3A::ZERO, outer),
            joint(Some(3), outer, Vec3A::new(0.0, 3.0, 20.0)),
        ];
        let still = Settings {
            gravity: 0.0,
            wag: 0.0,
            collide: false,
            clearance: 2.0,
            ..Settings::default()
        };
        let mut chains = Chains::under(&joints, &[0, 1, 2, 3, 4], &[(3, still), (1, still)]);
        chains.layer(&joints, &[(3, 1)]);
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        simulate(&joints, &chains, &mut pose, &mut state, None);
        // The inner chain's anchor steps past the outer's.
        let mut moved = rest;
        moved[1].translation.y = 8.0;
        moved[2].translation.y = 8.0;
        for _ in 0..120 {
            pose.clone_from(&moved);
            simulate(&joints, &chains, &mut pose, &mut state, Some(1.0 / 60.0));
        }
        let gap = pose[4].translation.y - pose[2].translation.y;
        assert!(gap > 1.9, "{gap} {:?}", pose[4].translation);
        // Its length held.
        let length = (pose[4].translation - pose[3].translation).length();
        assert!((length - 20.0).abs() < 1e-2, "{length}");
    }

    /// A strand stacked on its anchor with its vertices 10 units below, the anchor
    /// stepped sideways: the strand's angle from straight down, in degrees.
    fn strand_tilt(limit: f32) -> f32 {
        let anchor = Vec3A::new(0.0, 0.0, 40.0);
        let joints = vec![
            joint(None, Vec3A::ZERO, anchor),
            joint(Some(0), anchor, anchor),
        ];
        let settings = Settings {
            gravity: 0.0,
            wag: 0.0,
            limit,
            ..Settings::default()
        };
        let mut chains = Chains::under(&joints, &[0, 1], &[(0, settings)]);
        chains.swing_from(&joints, std::iter::once((1, anchor - Vec3A::Z * 10.0, 1.0)));
        let rest: Vec<Affine3A> = joints.iter().map(|joint| joint.bind).collect();
        let mut state = State::default();
        let mut pose = rest.clone();
        simulate(&joints, &chains, &mut pose, &mut state, None);
        let mut moved = rest;
        for joint in &mut moved {
            joint.translation.x += 5.0;
        }
        pose.clone_from(&moved);
        simulate(&joints, &chains, &mut pose, &mut state, Some(1.0 / 60.0));
        let down = (pose[1].matrix3 * Vec3A::NEG_Z).normalize();
        down.dot(Vec3A::NEG_Z).clamp(-1.0, 1.0).acos().to_degrees()
    }

    #[test]
    fn a_limited_strand_with_no_bone_swings_about_its_pivot_within_its_limit() {
        let tilt = strand_tilt(12.0);
        assert!(tilt > 3.0 && tilt < 12.0, "{tilt}");
        // Without a limit it stays as it was.
        assert!(strand_tilt(NO_LIMIT) < 0.01);
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
