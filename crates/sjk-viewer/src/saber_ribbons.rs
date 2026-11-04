//! Smoke as ribbons from a blade skin's emitter (`ribbon` in its file,
//! [`crate::blade_skin_file::smoke::Ribbon`]): each blade's emitter lays points along its
//! way at the skin's rate while the blade is lit and, fading, a while after it is put out,
//! one strand through them. Each point is left in the air: it rises, keeps a share of the
//! emitter's speed (dying away), the stream sways round as it is let go and the point
//! curls as it goes, so the strand bends into a wisp; it widens and fades as it ages. The
//! strand is drawn facing the camera through its points ([`Gpu`], `saber_ribbons.wgsl`):
//! soft across, torn along by noise that rides with each point. A swing keeps it smooth:
//! points are laid a unit apart however fast the emitter goes, a frame behind on the
//! centripetal Catmull-Rom spline through its last places rather than on each frame's
//! chord, and the strand is drawn through them on that spline too, each span cut in
//! [`CUTS`]. A leap of the emitter (the model put elsewhere) breaks the strand. A strand the emitter left
//! (put out and done, or not drawn for a moment) lives out its points. A bounded pool:
//! a full one takes no more points.

use glam::Vec3;
use std::collections::HashMap;

/// Most points in all strands, and most strands.
const MOST_POINTS: usize = 8_000;
const MOST_STRANDS: usize = 256;
/// Most points one emitter lays in a frame: a fast swing's, a unit apart.
const MOST_A_FRAME: u32 = 32;
/// The farthest apart (units) the points of a moving emitter are laid, whatever the rate:
/// a swing laid at the rate alone left straight spans with sharp corners between them.
const SPACING: f32 = 1.0;
/// The pieces each span between two points is drawn in, along the spline through them.
const CUTS: usize = 3;
/// A frame further from the last than this (the blade not drawn meanwhile) ends the
/// strand: the next begins where the emitter is then.
const GAP_MILLIS: i64 = 250;
/// A frame's move farther than this (units) is a leap, not a swing (the model put
/// somewhere else): it ends the strand too, rather than laying one straight across.
const LEAP: f32 = 64.0;
/// How fast a point's own speed (the emitter's share and the sway) dies away, a share a
/// second, so that the point comes to rise and curl alone, and the most of the emitter's
/// speed (units a second) a point keeps: air stops smoke at once. Kept slower and
/// uncapped, a strong swing threw its points down to the floor in a straight line.
const DRAG: f32 = 5.0;
const MOST_FOLLOW: f32 = 40.0;
/// A place nearer the last than this (units) is the same place: the stage's pose holds
/// a frame now and then (it is skinned in steps), and a place repeated made a corner.
const SAME_PLACE: f32 = 0.01;
/// How long it holds the same place before it is held still (and smokes there at its
/// rate) rather than paused a frame in a swing.
const HOLD_MILLIS: i64 = 100;
/// Seconds the emitter's speed is eased over, for how much it swings: a swing's smoke
/// thins and comes back softly, not at each pause.
const PACE_EASE: f32 = 0.25;
/// Milliseconds a strand takes to fade in from the emitter, so it does not start hard.
const FADE_IN_MILLIS: f32 = 60.0;

/// A skin's ribbon, small and `Copy` for the blades to carry ([`crate::saber_skins::SkinColor`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RibbonSpec {
    pub(crate) color: [f32; 3],
    pub(crate) opacity: f32,
    pub(crate) rate: f32,
    pub(crate) after: f32,
    pub(crate) life: f32,
    pub(crate) width: f32,
    pub(crate) growth: f32,
    pub(crate) rise: f32,
    pub(crate) follow: f32,
    pub(crate) sway: f32,
    pub(crate) sway_rate: f32,
    pub(crate) curl: f32,
    pub(crate) detail: f32,
    pub(crate) softness: f32,
    pub(crate) idle: f32,
    pub(crate) swinging: f32,
    pub(crate) swing_speed: f32,
    pub(crate) ignite: f32,
    pub(crate) along: f32,
}

impl RibbonSpec {
    /// The section read from a skin's file.
    pub(crate) fn of(ribbon: &crate::blade_skin_file::smoke::Ribbon) -> Self {
        Self {
            color: ribbon.color,
            opacity: ribbon.opacity,
            rate: ribbon.rate,
            after: ribbon.after,
            life: ribbon.life,
            width: ribbon.width,
            growth: ribbon.growth,
            rise: ribbon.rise,
            follow: ribbon.follow,
            sway: ribbon.sway,
            sway_rate: ribbon.sway_rate,
            curl: ribbon.curl,
            detail: ribbon.detail,
            softness: ribbon.softness,
            idle: ribbon.idle,
            swinging: ribbon.swinging,
            swing_speed: ribbon.swing_speed,
            ignite: ribbon.ignite,
            along: ribbon.along,
        }
    }
}

/// One point of a strand: where it is and how it moves, when it was let go, how strong
/// (the strand fading after the blade went out), and its number along the strand (the
/// noise rides with it).
#[derive(Clone, Copy, Debug)]
struct Point {
    at: Vec3,
    velocity: Vec3,
    born: i64,
    /// The time `at` is for: a point is laid a frame late and moved on to the frame it is
    /// laid in at once ([`drift`]), so that every point is where its age puts it. Moved
    /// only from its laying, the points of each frame lagged those before: a sawtooth.
    placed: i64,
    strength: f32,
    ordinal: u32,
}

/// Move `point` of `spec`'s strand (`seed` its randomness) on to `time`: its own speed
/// dying away ([`DRAG`]), rising and curling, in steps of at most 50 ms.
fn drift(point: &mut Point, spec: &RibbonSpec, seed: f32, time: i64) {
    while point.placed < time {
        let step = (time - point.placed).min(50);
        let seconds = step as f32 / 1_000.0;
        let age = (point.placed - point.born) as f32 / 1_000.0;
        let curl = Vec3::new(
            (point.at.z * 0.11 + seed + age * 0.9).sin(),
            (point.at.z * 0.13 + seed * 1.7 + age * 0.7).cos(),
            0.0,
        ) * spec.curl;
        // Exactly as `v' = −DRAG v` has it over the step.
        let keep = (-DRAG * seconds).exp();
        point.at += point.velocity * ((1.0 - keep) / DRAG) + (curl + Vec3::Z * spec.rise) * seconds;
        point.velocity *= keep;
        point.placed += step;
    }
}

/// One strand: its skin's ribbon, its points from the oldest, whether its emitter still
/// lays points on it, and its randomness.
#[derive(Clone, Debug)]
struct Strand {
    spec: RibbonSpec,
    points: std::collections::VecDeque<Point>,
    open: bool,
    seed: u32,
    /// The emitter's place this frame, while open: the strand reaches it.
    head: Option<Vec3>,
}

/// One blade's emitter: the strand it lays, the points owed (a share of one carried to
/// the next frame, by the rate and by the way), where it was the last three frames and
/// when (the oldest first), when the blade went out while it still smokes after, and the
/// points laid.
#[derive(Clone, Copy, Debug, Default)]
struct Emitter {
    strand: Option<usize>,
    owed: f32,
    /// The points owed by the way it came (one a [`SPACING`]), apart from the rate's.
    owed_far: f32,
    history: [Option<(Vec3, i64)>; 3],
    /// How fast it went over the way laid last frame (units a second), for this frame's
    /// points to change speed from smoothly.
    speed: Option<f32>,
    out_at: Option<i64>,
    lit: bool,
    /// When it was last lit (from out; not when first seen lit), for the ignition's smoke.
    lit_at: Option<i64>,
    /// Whether it has been seen before: one first seen lit was not lit just now.
    known: bool,
    /// Its speed eased over [`PACE_EASE`] seconds (units a second): how much it swings.
    pace: f32,
    laid: u32,
    /// The frame it was last told of, to forget it.
    seen: i64,
}

impl Emitter {
    /// Where it was last frame, and when.
    fn last(&self) -> Option<(Vec3, i64)> {
        self.history[2]
    }

    /// Forget where it was: the next frame starts a fresh way.
    fn forget_way(&mut self) {
        self.history = [None; 3];
        self.speed = None;
        self.owed = 0.0;
        self.owed_far = 0.0;
    }
}

/// Every strand and every emitter laying one.
#[derive(Default)]
pub(crate) struct Ribbons {
    strands: Vec<Option<Strand>>,
    emitters: HashMap<u64, Emitter>,
    points: usize,
}

impl Ribbons {
    /// This frame, at `time` (milliseconds): emitter `key` is at `at`, its blade `lit` or
    /// not; lay the points `ribbon` owes on its strand. They lie a frame behind, on the way
    /// between the last two frames, on the centripetal spline through the last four
    /// places: it bends round a swing turning back without the loops a curve carrying on
    /// the last way made; the strand reaches the emitter through its head.
    pub(crate) fn emit(&mut self, key: u64, ribbon: &RibbonSpec, at: Vec3, lit: bool, time: i64) {
        let mut emitter = self.emitters.get(&key).copied().unwrap_or_default();
        emitter.seen = time;
        if emitter.lit && !lit {
            emitter.out_at = Some(time);
        }
        if lit {
            emitter.out_at = None;
        }
        if lit && !emitter.lit && emitter.known {
            emitter.lit_at = Some(time);
        }
        emitter.known = true;
        emitter.lit = lit;
        // A long wait (the blade not drawn) or a leap (the model put somewhere else)
        // breaks the way: no strand across it.
        let broken = emitter.last().is_none_or(|(then_at, then)| {
            time - then > GAP_MILLIS || time < then || then_at.distance(at) > LEAP
        });
        // How fast it goes, eased: a pose held a frame in a swing does not slow it.
        if let Some((place, then)) = emitter.last().filter(|_| !broken) {
            let held = place.distance(at) < SAME_PLACE && time - then < HOLD_MILLIS;
            let seconds = (time - then) as f32 / 1_000.0;
            if !held && seconds > 0.0 {
                let speed = place.distance(at) / seconds;
                let ease = 1.0 - (-seconds / PACE_EASE).exp();
                emitter.pace += (speed - emitter.pace) * ease;
            }
        } else {
            emitter.pace = 0.0;
        }
        let strength = strength(ribbon, &emitter, time);
        if broken || strength <= 0.0 {
            self.close(&mut emitter);
        }
        if broken {
            emitter.forget_way();
        }
        // Held still (the stance, or a pose held a frame: the stage skins in steps): the
        // same place again is no new place on the way, but the emitter goes on smoking
        // there at its rate. The way laid a frame behind is laid out to it first, once.
        if !broken
            && let Some((place, then)) = emitter.last()
            && place.distance(at) < SAME_PLACE
        {
            // A pose held a frame or two in a swing: the way goes on from it once it moves,
            // the time held counted then.
            if time - then < HOLD_MILLIS && emitter.history[1].is_some() {
                self.emitters.insert(key, emitter);
                return;
            }
            if strength > 0.0 {
                let [before, from, to] = emitter.history;
                if let (Some(from), Some(to)) = (from, to) {
                    let before = before.map_or(from.0, |(place, _)| place);
                    self.lay_way(
                        &mut emitter,
                        key,
                        ribbon,
                        strength,
                        [before, to.0],
                        from,
                        to,
                        time,
                    );
                }
                let seconds = (time - then).max(1) as f32 / 1_000.0;
                emitter.owed += ribbon.rate * seconds;
                emitter.owed_far = 0.0;
                emitter.speed = Some(0.0);
                let count = (emitter.owed.floor() as u32).min(MOST_A_FRAME);
                emitter.owed = (emitter.owed - count as f32).clamp(0.0, 1.0);
                self.lay_still(
                    &mut emitter,
                    key,
                    ribbon,
                    strength,
                    place,
                    then,
                    time,
                    count,
                );
            }
            // Its time goes on there; the next move starts a fresh way from it.
            emitter.history = [None, None, Some((place, time))];
            if let Some(Some(strand)) = emitter.strand.and_then(|at| self.strands.get_mut(at)) {
                strand.head = Some(at);
                strand.spec = *ribbon;
            }
            self.emitters.insert(key, emitter);
            return;
        }
        let [before, from, to] = emitter.history;
        emitter.history = [from, to, Some((at, time))];
        // The way laid this frame: last frame's, between `from` and `to`, with the places
        // either side of it.
        if let (Some(from), Some(to), true) = (from, to, strength > 0.0) {
            let before = before.map_or(from.0, |(place, _)| place);
            self.lay_way(
                &mut emitter,
                key,
                ribbon,
                strength,
                [before, at],
                from,
                to,
                time,
            );
        }
        if let Some(Some(strand)) = emitter.strand.and_then(|at| self.strands.get_mut(at)) {
            strand.head = Some(at);
            strand.spec = *ribbon;
        }
        self.emitters.insert(key, emitter);
    }

    /// The emitter's strand, opened when it has none; `None` when the pool is full.
    fn strand_of(&mut self, emitter: &mut Emitter, ribbon: &RibbonSpec, key: u64) -> Option<usize> {
        let strand = match emitter.strand {
            Some(strand) => Some(strand),
            None => self.open_strand(ribbon, key),
        };
        emitter.strand = strand;
        strand
    }

    /// Lay the points owed on the way from `from` to `to` (each a place and its time) on
    /// the centripetal spline through `before` and `after` round them; each keeps the
    /// emitter's speed where it was let go, going smoothly from the last way's, and is
    /// moved on to `time`.
    #[allow(clippy::too_many_arguments)]
    fn lay_way(
        &mut self,
        emitter: &mut Emitter,
        key: u64,
        ribbon: &RibbonSpec,
        strength: f32,
        [before, after]: [Vec3; 2],
        (from, then): (Vec3, i64),
        (to, now): (Vec3, i64),
        time: i64,
    ) {
        let millis = (now - then).max(1);
        let seconds = millis as f32 / 1_000.0;
        // The rate's points, or one a unit of the way when that is more.
        emitter.owed += ribbon.rate * seconds;
        emitter.owed_far += from.distance(to) / SPACING;
        let owed = emitter.owed.max(emitter.owed_far);
        let count = (owed.floor() as u32).min(MOST_A_FRAME);
        emitter.owed = (emitter.owed - count as f32).clamp(0.0, 1.0);
        emitter.owed_far = (emitter.owed_far - count as f32).clamp(0.0, 1.0);
        // Each point keeps the emitter's speed where it was let go: the spline's way there,
        // at a speed going smoothly from the last way's to this one's. A frame's one chord
        // for all its points turned their speed at each frame, and they drifted apart into
        // corners.
        let speed = from.distance(to) / seconds;
        let speed_before = emitter.speed.replace(speed).unwrap_or(speed);
        let Some(strand) = self.strand_of(emitter, ribbon, key) else {
            return;
        };
        let place = |share: f32| centripetal(before, from, to, after, share);
        for laid in 0..count {
            let share = (laid as f32 + 1.0) / count as f32;
            let way = (place((share + 0.02).min(1.0)) - place((share - 0.02).max(0.0)))
                .normalize_or_zero();
            let moving = way * (speed_before + (speed - speed_before) * share);
            let born = then + (millis as f32 * share) as i64;
            let velocity = (moving * ribbon.follow).clamp_length_max(MOST_FOLLOW);
            self.let_go(
                emitter,
                strand,
                ribbon,
                strength,
                place(share),
                velocity,
                born,
                time,
            );
        }
    }

    /// Lay `count` points at `place`, where the emitter holds still, born between `then`
    /// and `time`.
    #[allow(clippy::too_many_arguments)]
    fn lay_still(
        &mut self,
        emitter: &mut Emitter,
        key: u64,
        ribbon: &RibbonSpec,
        strength: f32,
        place: Vec3,
        then: i64,
        time: i64,
        count: u32,
    ) {
        if count == 0 {
            return;
        }
        let Some(strand) = self.strand_of(emitter, ribbon, key) else {
            return;
        };
        let millis = (time - then).max(1);
        for laid in 0..count {
            let share = (laid as f32 + 1.0) / count as f32;
            let born = then + (millis as f32 * share) as i64;
            self.let_go(
                emitter,
                strand,
                ribbon,
                strength,
                place,
                Vec3::ZERO,
                born,
                time,
            );
        }
    }

    /// Let one point go on `strand` at `place`, born at `born` with `velocity` and the
    /// stream's sway then, moved on to `time` as if it had been let go then.
    #[allow(clippy::too_many_arguments)]
    fn let_go(
        &mut self,
        emitter: &mut Emitter,
        strand: usize,
        ribbon: &RibbonSpec,
        strength: f32,
        place: Vec3,
        velocity: Vec3,
        born: i64,
        time: i64,
    ) {
        emitter.laid = emitter.laid.wrapping_add(1);
        let mut point = Point {
            at: place,
            velocity: velocity + sway(ribbon, born as f32 / 1_000.0),
            born,
            placed: born,
            strength,
            ordinal: emitter.laid,
        };
        let seed = self.strands[strand]
            .as_ref()
            .map_or(0.0, |strand| strand.seed as f32 * 1e-4);
        drift(&mut point, ribbon, seed, time);
        self.lay(strand, point);
    }

    /// Stop `emitter`'s strand being laid: it lives out its points.
    fn close(&mut self, emitter: &mut Emitter) {
        if let Some(Some(strand)) = emitter
            .strand
            .take()
            .and_then(|at| self.strands.get_mut(at))
        {
            strand.open = false;
            strand.head = None;
        }
        emitter.owed = 0.0;
        emitter.owed_far = 0.0;
    }

    /// A new strand of `ribbon` for emitter `key`, in a free slot; `None` when full.
    fn open_strand(&mut self, ribbon: &RibbonSpec, key: u64) -> Option<usize> {
        let strand = Strand {
            spec: *ribbon,
            points: std::collections::VecDeque::new(),
            open: true,
            seed: (key as u32).wrapping_mul(0x9e37_79b9) ^ (key >> 32) as u32,
            head: None,
        };
        if let Some(free) = self.strands.iter().position(Option::is_none) {
            self.strands[free] = Some(strand);
            return Some(free);
        }
        (self.strands.len() < MOST_STRANDS).then(|| {
            self.strands.push(Some(strand));
            self.strands.len() - 1
        })
    }

    fn lay(&mut self, strand: usize, point: Point) {
        if self.points >= MOST_POINTS {
            return;
        }
        if let Some(Some(strand)) = self.strands.get_mut(strand) {
            strand.points.push_back(point);
            self.points += 1;
        }
    }

    /// Move every point from `then` to `time` (milliseconds), let go the ones past their
    /// life, the strands left with none, and the emitters not heard from for a while.
    pub(crate) fn advance(&mut self, then: i64, time: i64) {
        let seconds = ((time - then) as f32 / 1_000.0).clamp(0.0, 0.1);
        for slot in &mut self.strands {
            let Some(strand) = slot else { continue };
            let spec = strand.spec;
            let life_millis = (spec.life * 1_000.0) as i64;
            while strand
                .points
                .front()
                .is_some_and(|point| time - point.born >= life_millis)
            {
                strand.points.pop_front();
                self.points -= 1;
            }
            let seed = strand.seed as f32 * 1e-4;
            // A long frame moves them as far as a tenth of a second would.
            let from = time - (seconds * 1_000.0) as i64;
            for point in &mut strand.points {
                point.placed = point.placed.max(from);
                drift(point, &spec, seed, time);
            }
            if !strand.open && strand.points.is_empty() {
                *slot = None;
            }
        }
        // Emitters not told of for twice the gap (their blade gone) are forgotten.
        let strands = &mut self.strands;
        self.emitters.retain(|_, emitter| {
            let kept = time - emitter.seen <= GAP_MILLIS * 2;
            if !kept && let Some(Some(strand)) = emitter.strand.and_then(|at| strands.get_mut(at)) {
                strand.open = false;
                strand.head = None;
            }
            kept
        });
    }

    /// Every strand's quads at `time`, as two triangles a span between two points.
    pub(crate) fn append_vertices(&self, time: i64, out: &mut Vec<Vertex>) {
        for strand in self.strands.iter().flatten() {
            let spec = strand.spec;
            let life_millis = spec.life * 1_000.0;
            // The points, and the emitter itself at the strand's young end while it is laid.
            let mut points: Vec<(Vec3, f32, f32, f32)> = strand
                .points
                .iter()
                .map(|point| {
                    let age = (time - point.born) as f32;
                    (point.at, age, point.strength, point.ordinal as f32)
                })
                .collect();
            if let (Some(head), Some(last)) = (strand.head, strand.points.back()) {
                points.push((head, 0.0, last.strength, last.ordinal as f32 + 1.0));
            }
            if points.len() < 2 {
                continue;
            }
            let points = smoothed(&points);
            let vertex = |at: usize, side: f32| {
                let (position, age, strength, ordinal) = points[at];
                let before = points[at.saturating_sub(1)].0;
                let after = points[(at + 1).min(points.len() - 1)].0;
                let share = (age / life_millis).clamp(0.0, 1.0);
                let width = spec.width + (spec.growth - spec.width) * share;
                // Fades with age, and in from the emitter.
                let fade = (1.0 - share) * (age / FADE_IN_MILLIS).clamp(0.0, 1.0);
                Vertex {
                    position: position.to_array(),
                    side: side * width * 0.5,
                    tangent: (after - before).to_array(),
                    along: ordinal / spec.rate.max(1.0) * spec.rise.abs().max(4.0),
                    color: [
                        spec.color[0],
                        spec.color[1],
                        spec.color[2],
                        spec.opacity * strength * fade,
                    ],
                    detail: spec.detail,
                    softness: spec.softness,
                    age: age / 1_000.0 + (strand.seed % 997) as f32,
                    _padding: 0.0,
                }
            };
            for at in 0..points.len() - 1 {
                let quad = [
                    vertex(at, -1.0),
                    vertex(at, 1.0),
                    vertex(at + 1, -1.0),
                    vertex(at + 1, -1.0),
                    vertex(at, 1.0),
                    vertex(at + 1, 1.0),
                ];
                out.extend(quad);
            }
        }
    }

    /// How many points there are, in all strands.
    #[cfg(test)]
    fn point_count(&self) -> usize {
        self.points
    }
}

/// `points` (place, age, strength, number) with [`CUTS`] pieces a span, the places on
/// the centripetal Catmull-Rom spline through them (which neither overshoots nor loops
/// where the points lie unevenly), the rest in between.
fn smoothed(points: &[(Vec3, f32, f32, f32)]) -> Vec<(Vec3, f32, f32, f32)> {
    let last = points.len() - 1;
    let mut out = Vec::with_capacity(last * CUTS + 1);
    for span in 0..last {
        let p0 = points[span.saturating_sub(1)].0;
        let (p1, age1, strength1, ordinal1) = points[span];
        let (p2, age2, strength2, ordinal2) = points[span + 1];
        let p3 = points[(span + 2).min(last)].0;
        for cut in 0..CUTS {
            let t = cut as f32 / CUTS as f32;
            let blend = |a: f32, b: f32| a + (b - a) * t;
            out.push((
                centripetal(p0, p1, p2, p3, t),
                blend(age1, age2),
                blend(strength1, strength2),
                blend(ordinal1, ordinal2),
            ));
        }
    }
    out.push(points[last]);
    out
}

/// The centripetal Catmull-Rom spline through `p1` and `p2` (`p0` before, `p3` after) at
/// `t` of the way (Barry and Goldman's form, knots by the square root of each chord).
fn centripetal(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    let knot = |a: Vec3, b: Vec3| a.distance(b).sqrt().max(1e-4);
    let t1 = knot(p0, p1);
    let t2 = t1 + knot(p1, p2);
    let t3 = t2 + knot(p2, p3);
    let u = t1 + (t2 - t1) * t;
    let lerp = |a: Vec3, b: Vec3, ta: f32, tb: f32| a + (b - a) * ((u - ta) / (tb - ta));
    let a1 = lerp(p0, p1, 0.0, t1);
    let a2 = lerp(p1, p2, t1, t2);
    let a3 = lerp(p2, p3, t2, t3);
    let b1 = lerp(a1, a2, 0.0, t2);
    let b2 = lerp(a2, a3, t1, t3);
    lerp(b1, b2, t1, t2)
}

/// How strong `emitter`'s smoke of `ribbon` is at `time`: lit, `idle` to `swinging` by
/// how fast it goes, and full for the `ignite` seconds once lit, thinning at their end;
/// put out, staying strong a while, then thinning away over `after` seconds.
fn strength(ribbon: &RibbonSpec, emitter: &Emitter, time: i64) -> f32 {
    let since = |at: i64| (time - at) as f32 / 1_000.0;
    let fading = |seconds: f32, over: f32| {
        if seconds < over {
            1.0 - (seconds / over).powi(2)
        } else {
            0.0
        }
    };
    if emitter.lit {
        let swing = (emitter.pace / ribbon.swing_speed).clamp(0.0, 1.0);
        let swing = swing * swing * (3.0 - 2.0 * swing);
        let moving = ribbon.idle + (ribbon.swinging - ribbon.idle) * swing;
        let lighting = emitter
            .lit_at
            .map_or(0.0, |lit| fading(since(lit), ribbon.ignite));
        return moving.max(lighting);
    }
    emitter
        .out_at
        .map_or(0.0, |out| fading(since(out), ribbon.after))
}

/// The sway of the stream at `moment` (seconds): round, a little out of step across.
fn sway(ribbon: &RibbonSpec, moment: f32) -> Vec3 {
    let phase = moment * ribbon.sway_rate * std::f32::consts::TAU;
    Vec3::new(phase.sin(), (phase * 0.73).cos(), 0.0) * ribbon.sway
}

/// One ribbon vertex: its point, how far to its side (signed half-width, widened facing
/// the camera in the shader), the strand's way there, how far along it the noise is, its
/// colour, the noise's detail and the edge's softness, and its age (seconds, the noise
/// turning with it).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) side: f32,
    pub(crate) tangent: [f32; 3],
    pub(crate) along: f32,
    pub(crate) color: [f32; 4],
    pub(crate) detail: f32,
    pub(crate) softness: f32,
    pub(crate) age: f32,
    pub(crate) _padding: f32,
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32,
            2 => Float32x3,
            3 => Float32,
            4 => Float32x4,
            5 => Float32x4
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

/// Most vertices drawn a frame: every point's span, in its pieces.
const MOST_VERTICES: usize = (MOST_POINTS + MOST_STRANDS) * CUTS * 6;

/// The ribbons' pipeline (alpha-blended, depth-tested, not writing depth) and buffer.
pub(crate) struct Gpu {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    vertices: Vec<Vertex>,
    count: u32,
}

impl Gpu {
    pub(crate) fn new(
        device: &wgpu::Device,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Self {
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK smoke ribbon pipeline layout"),
            bind_group_layouts: &[Some(camera_layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK smoke ribbon shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("saber_ribbons.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK smoke ribbon pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::layout())],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DepthTarget::FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK smoke ribbon vertices"),
            size: (MOST_VERTICES * std::mem::size_of::<Vertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            buffer,
            vertices: Vec::new(),
            count: 0,
        }
    }

    /// Take this frame's strands of `ribbons` at `time`.
    pub(crate) fn prepare(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        ribbons: &Ribbons,
        time: i64,
    ) {
        self.vertices.clear();
        ribbons.append_vertices(time, &mut self.vertices);
        self.vertices.truncate(MOST_VERTICES / 6 * 6);
        self.count = u32::try_from(self.vertices.len()).unwrap_or(u32::MAX);
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    /// Each vertex's point and how far to its side the strand reaches there, for the
    /// effect layer's bounds.
    pub(crate) fn extents(&self) -> impl Iterator<Item = ([f32; 3], f32)> + '_ {
        self.vertices
            .iter()
            .map(|vertex| (vertex.position, vertex.side.abs()))
    }

    /// Whether there is a strand to draw this frame.
    pub(crate) fn has_draws(&self) -> bool {
        self.count > 0
    }

    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        if self.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, camera, &[]);
        pass.set_vertex_buffer(0, self.buffer.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> RibbonSpec {
        RibbonSpec {
            color: [1.0; 3],
            opacity: 0.5,
            rate: 30.0,
            after: 2.0,
            life: 2.0,
            width: 1.0,
            growth: 4.0,
            rise: 10.0,
            follow: 0.3,
            sway: 3.0,
            sway_rate: 0.5,
            curl: 2.0,
            detail: 1.0,
            softness: 0.5,
            idle: 1.0,
            swinging: 1.0,
            swing_speed: 120.0,
            ignite: 0.0,
            along: 0.0,
        }
    }

    /// Lit, the emitter lays one strand of points that rise; put out, it lays on, fading,
    /// then stops and the strand lives out its points; a gap starts a new strand.
    #[test]
    fn an_emitter_lays_one_strand_lit_and_a_while_after() {
        let mut ribbons = Ribbons::default();
        let ribbon = spec();
        let mut time = 0;
        let run = |ribbons: &mut Ribbons, time: &mut i64, lit: bool, frames: i64| {
            for _ in 0..frames {
                let then = *time;
                *time += 16;
                ribbons.emit(
                    7,
                    &ribbon,
                    Vec3::new(*time as f32 * 0.05, 0.0, 0.0),
                    lit,
                    *time,
                );
                ribbons.advance(then, *time);
            }
        };
        // A second lit, the emitter going 50 units a second: a point a unit (more than the
        // rate's thirty).
        run(&mut ribbons, &mut time, true, 62);
        let lit = ribbons.point_count();
        assert!((45..=51).contains(&lit), "{lit}");
        assert_eq!(ribbons.strands.iter().flatten().count(), 1);
        let first = ribbons.strands.iter().flatten().next().unwrap();
        assert!(first.points.front().unwrap().at.z > first.points.back().unwrap().at.z);
        let mut vertices = Vec::new();
        ribbons.append_vertices(time, &mut vertices);
        assert_eq!(
            vertices.len(),
            lit * CUTS * 6,
            "a span a point, the emitter's included, in its pieces"
        );
        // Put out: laid a while longer, then no more; two seconds on, all gone.
        run(&mut ribbons, &mut time, false, 62);
        let after = ribbons.point_count();
        run(&mut ribbons, &mut time, false, 200);
        assert!(after > 0 && ribbons.point_count() == 0);
        assert!(ribbons.strands.iter().flatten().next().is_none());
        // A gap, then lit again: a new strand.
        time += 5_000;
        run(&mut ribbons, &mut time, true, 30);
        assert_eq!(ribbons.strands.iter().flatten().count(), 1);
    }

    /// A fast swing round a circle (300 units a second, ten degrees a frame) leaves a
    /// strand without corners: its drawn pieces turn little from one to the next, where
    /// the rate's points alone, a frame's chord apart, turned ten degrees at each.
    #[test]
    fn a_swing_leaves_a_smooth_strand() {
        let ribbon = RibbonSpec {
            rise: 0.0,
            follow: 0.0,
            sway: 0.0,
            curl: 0.0,
            ..spec()
        };
        let mut ribbons = Ribbons::default();
        let mut time = 0_i64;
        for frame in 0..30 {
            let then = time;
            time += 16;
            let angle = frame as f32 * 0.16;
            let at = Vec3::new(angle.cos(), angle.sin(), 0.0) * 30.0;
            ribbons.emit(1, &ribbon, at, true, time);
            ribbons.advance(then, time);
        }
        let strand = ribbons.strands.iter().flatten().next().unwrap();
        assert!(
            strand.points.len() > 100,
            "a unit apart: {}",
            strand.points.len()
        );
        let mut vertices = Vec::new();
        ribbons.append_vertices(time, &mut vertices);
        // Each piece's start (the first vertex of its six), the newest first.
        let centres: Vec<Vec3> = vertices
            .chunks(6)
            .map(|quad| Vec3::from_array(quad[0].position))
            .collect();
        let sharpest = centres
            .windows(3)
            .map(|three| {
                let (a, b) = (three[1] - three[0], three[2] - three[1]);
                a.angle_between(b).to_degrees()
            })
            .fold(0.0, f32::max);
        assert!(sharpest < 4.0, "a corner of {sharpest} degrees");
    }

    /// A swing turning back (out along x five frames, then back) bends round where it
    /// turns without looping past it, and a leap (the model put elsewhere) lays no strand
    /// across: it starts a new one.
    #[test]
    fn a_swing_turning_back_does_not_loop_and_a_leap_breaks_the_strand() {
        let ribbon = RibbonSpec {
            rise: 0.0,
            follow: 0.0,
            sway: 0.0,
            curl: 0.0,
            ..spec()
        };
        let mut ribbons = Ribbons::default();
        let mut time = 0_i64;
        let mut step = |ribbons: &mut Ribbons, at: Vec3| {
            let then = time;
            time += 16;
            ribbons.emit(1, &ribbon, at, true, time);
            ribbons.advance(then, time);
        };
        for frame in 0..=5 {
            step(
                &mut ribbons,
                Vec3::new(frame as f32 * 5.0, frame as f32 * 0.5, 0.0),
            );
        }
        for frame in (0..5).rev() {
            step(
                &mut ribbons,
                Vec3::new(frame as f32 * 5.0, 5.0 - frame as f32 * 0.5, 0.0),
            );
        }
        let farthest = ribbons
            .strands
            .iter()
            .flatten()
            .flat_map(|strand| strand.points.iter())
            .map(|point| point.at.x)
            .fold(f32::MIN, f32::max);
        assert!(farthest < 25.5, "looped out to x {farthest}");
        // A frame behind: the new strand is laid from the third frame after the leap.
        for x in [2_000.0, 2_001.0, 2_002.0] {
            step(&mut ribbons, Vec3::new(x, 0.0, 0.0));
        }
        let open = ribbons
            .strands
            .iter()
            .flatten()
            .filter(|strand| strand.open)
            .count();
        assert_eq!(open, 1, "one strand laid, the one after the leap");
        assert!(
            ribbons
                .strands
                .iter()
                .flatten()
                .flat_map(|strand| strand.points.iter())
                .all(|point| point.at.x < 26.0 || point.at.x > 1_999.0),
            "nothing laid across the leap"
        );
    }

    /// The same swing with the pose held every third frame (the stage skins it in
    /// steps) stays as smooth, and a fast swing throws no point far along it.
    #[test]
    fn a_swing_posed_in_steps_stays_smooth_and_its_points_stay_near() {
        let ribbon = RibbonSpec {
            rise: 0.0,
            follow: 1.0,
            sway: 0.0,
            curl: 0.0,
            ..spec()
        };
        let mut ribbons = Ribbons::default();
        let mut time = 0_i64;
        let mut angle = 0.0_f32;
        for frame in 0..30 {
            let then = time;
            time += 16;
            if frame % 3 != 2 {
                angle += 0.16 * 1.5;
            }
            let at = Vec3::new(angle.cos(), angle.sin(), 0.0) * 30.0;
            ribbons.emit(1, &ribbon, at, true, time);
            ribbons.advance(then, time);
        }
        let strand = ribbons.strands.iter().flatten().next().unwrap();
        let farthest = strand
            .points
            .iter()
            .map(|point| point.at.length())
            .fold(0.0, f32::max);
        assert!(farthest < 34.0, "a point thrown out to {farthest}");
        let mut vertices = Vec::new();
        ribbons.append_vertices(time, &mut vertices);
        let centres: Vec<Vec3> = vertices
            .chunks(6)
            .map(|quad| Vec3::from_array(quad[0].position))
            .collect();
        let sharpest = centres
            .windows(3)
            .map(|three| {
                (three[1] - three[0])
                    .angle_between(three[2] - three[1])
                    .to_degrees()
            })
            .filter(|angle| angle.is_finite())
            .fold(0.0, f32::max);
        assert!(sharpest < 8.0, "a corner of {sharpest} degrees");
    }

    /// Held quite still (the stance, its pose the same each frame), the emitter goes on
    /// laying its strand at its rate, a second and more, never broken off; and moving on
    /// again it carries on from where it stood, laying its points in order.
    #[test]
    fn a_still_emitter_goes_on_smoking() {
        let ribbon = spec();
        let mut ribbons = Ribbons::default();
        let mut time = 0_i64;
        let mut step = |ribbons: &mut Ribbons, at: Vec3| {
            let then = time;
            time += 16;
            ribbons.emit(1, &ribbon, at, true, time);
            ribbons.advance(then, time);
        };
        for frame in 0..5 {
            step(&mut ribbons, Vec3::new(frame as f32, 0.0, 0.0));
        }
        let moving = ribbons.point_count();
        for _ in 0..125 {
            step(&mut ribbons, Vec3::new(4.0, 0.0, 0.0));
        }
        let still = ribbons.point_count() - moving;
        assert!(
            (55..=62).contains(&still),
            "{still} in two seconds at 30 a second"
        );
        assert_eq!(
            ribbons.strands.iter().flatten().count(),
            1,
            "never broken off"
        );
        for frame in 5..10 {
            step(&mut ribbons, Vec3::new(frame as f32, 0.0, 0.0));
        }
        let strand = ribbons.strands.iter().flatten().next().unwrap();
        let born: Vec<i64> = strand.points.iter().map(|point| point.born).collect();
        assert!(born.windows(2).all(|two| two[0] <= two[1]), "laid in order");
    }

    /// Smoking standing, lighting and put out but not swinging: still it is at full
    /// strength; swinging fast it thins to nothing (closing its strand); lit after being
    /// out it smokes for its `ignite` seconds even swinging; put out it lingers. With
    /// `swinging` 1 it smokes the whole time.
    #[test]
    fn it_smokes_standing_lighting_and_put_out_but_not_swinging() {
        let ribbon = RibbonSpec {
            idle: 1.0,
            swinging: 0.0,
            swing_speed: 120.0,
            ignite: 1.0,
            ..spec()
        };
        let mut emitter = Emitter {
            lit: true,
            known: true,
            ..Emitter::default()
        };
        assert_eq!(strength(&ribbon, &emitter, 0), 1.0, "standing");
        emitter.pace = 300.0;
        assert_eq!(strength(&ribbon, &emitter, 0), 0.0, "swinging");
        emitter.pace = 60.0;
        let half = strength(&ribbon, &emitter, 0);
        assert!(half > 0.3 && half < 0.7, "between: {half}");
        emitter.pace = 300.0;
        emitter.lit_at = Some(0);
        assert!(strength(&ribbon, &emitter, 500) > 0.7, "lighting");
        assert_eq!(
            strength(&ribbon, &emitter, 1_200),
            0.0,
            "lit a while, swinging"
        );
        let always = RibbonSpec {
            swinging: 1.0,
            ..ribbon
        };
        assert_eq!(strength(&always, &emitter, 1_200), 1.0, "the whole time");
        emitter.lit = false;
        emitter.out_at = Some(1_000);
        assert!(strength(&ribbon, &emitter, 1_500) > 0.9, "put out");
        // Through the emitter: swung round a circle, the strand stops being laid.
        let mut ribbons = Ribbons::default();
        let mut time = 0_i64;
        for frame in 0..60 {
            let then = time;
            time += 16;
            let angle = frame as f32 * 0.2;
            ribbons.emit(
                1,
                &ribbon,
                Vec3::new(angle.cos(), angle.sin(), 0.0) * 40.0,
                true,
                time,
            );
            ribbons.advance(then, time);
        }
        assert!(ribbons.strands.iter().flatten().all(|strand| !strand.open));
    }

    #[test]
    fn the_spline_passes_through_its_points_and_stays_between_them() {
        let (p0, p1, p2, p3) = (
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.1, 0.0, 0.0),
            Vec3::new(5.0, 3.0, 0.0),
        );
        assert!(centripetal(p0, p1, p2, p3, 0.0).distance(p1) < 1e-4);
        assert!(centripetal(p0, p1, p2, p3, 1.0).distance(p2) < 1e-4);
        let middle = centripetal(p0, p1, p2, p3, 0.5);
        assert!(
            middle.x > 0.95 && middle.x < 1.15 && middle.y.abs() < 0.1,
            "{middle}"
        );
    }
}
