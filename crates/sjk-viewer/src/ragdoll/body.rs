//! The Ragdoll toy's body: a limp humanoid of [`JOINTS`] points held together by sticks
//! (Verlet integration, as the sandbox games' first ragdolls), falling and tumbling
//! against the world.
//!
//! Every SJK client that hears of a fall runs this same simulation from the same start
//! ([`Start`]), in fixed steps of [`STEP_MS`] game server milliseconds counted from the
//! fall's moment, so all of them show the body in the same place without any pose being
//! sent: the start is one short relay event. The body's proportions are the stock
//! humanoid's, not the player model's (a client that lacks the model draws another),
//! at the player's `modelScale` ([`Start::scale`], since 03/11/2026: a youngling or a
//! jawa made small by the server fell full size, hub report #223). The world's brushes
//! and its movers are in its way: a mover stands where its trajectory puts it at the
//! step's own moment ([`World::at`]), which every client that sees the mover reads the
//! same, so a lift carries the body and a closing door pushes it the same way everywhere.
//! Once every point has lain still for [`SLEEP_STEPS`] steps the body sleeps, until a
//! mover carries or pushes it again, or something strikes it.
//!
//! Since 26/10/2026 (the Monitor toy) the points weigh what the body's parts weigh
//! ([`MASSES`]: the sticks move a hand more than the pelvis), a surface holds a point by
//! Coulomb friction (what it presses into the surface sets how much of its slide it
//! loses, so a body thrown hard skids and tumbles on rather than stopping dead), the limbs
//! keep out of each other ([`Body::apart`]) and a [`Strike`] gives the whole body a push
//! at one of its points, at a given step, as a solid would take it: a blow to the head
//! turns it heels over head, one to the legs sweeps them from under it.

use glam::Vec3;

/// The points, by name.
pub(crate) const PELVIS: usize = 0;
pub(crate) const CHEST: usize = 1;
pub(crate) const HEAD: usize = 2;
pub(crate) const L_SHOULDER: usize = 3;
pub(crate) const L_ELBOW: usize = 4;
pub(crate) const L_HAND: usize = 5;
pub(crate) const R_SHOULDER: usize = 6;
pub(crate) const R_ELBOW: usize = 7;
pub(crate) const R_HAND: usize = 8;
pub(crate) const L_HIP: usize = 9;
pub(crate) const L_KNEE: usize = 10;
pub(crate) const L_FOOT: usize = 11;
pub(crate) const R_HIP: usize = 12;
pub(crate) const R_KNEE: usize = 13;
pub(crate) const R_FOOT: usize = 14;
pub(crate) const JOINTS: usize = 15;

/// Where each point stands on a body upright at the origin (its feet at height 0),
/// facing +X with its left at +Y: the stock `_humanoid` skeleton's joints (pelvis,
/// thoracic, cranium, humerus, radius, hand, femur, tibia, talus), arms hanging.
const STANDING: [[f32; 3]; JOINTS] = [
    [0.0, 0.0, 37.0],
    [0.0, 0.0, 51.5],
    [1.0, 0.0, 61.5],
    [-1.0, 6.0, 54.5],
    [-1.5, 7.0, 43.8],
    [0.5, 7.5, 35.2],
    [-1.0, -6.0, 54.5],
    [-1.5, -7.0, 43.8],
    [0.5, -7.5, 35.2],
    [0.0, 3.6, 35.0],
    [1.5, 4.0, 21.3],
    [0.0, 4.5, 7.5],
    [0.0, -3.6, 35.0],
    [1.5, -4.0, 21.3],
    [0.0, -4.5, 7.5],
];

/// The sticks: pairs kept at their standing length. Bones first, then the braces that
/// keep the trunk a solid block (shoulders and hips crossed) and the head on the
/// shoulders.
const STICKS: [(usize, usize); 26] = [
    (PELVIS, CHEST),
    (CHEST, HEAD),
    (CHEST, L_SHOULDER),
    (CHEST, R_SHOULDER),
    (L_SHOULDER, L_ELBOW),
    (L_ELBOW, L_HAND),
    (R_SHOULDER, R_ELBOW),
    (R_ELBOW, R_HAND),
    (PELVIS, L_HIP),
    (PELVIS, R_HIP),
    (L_HIP, L_KNEE),
    (L_KNEE, L_FOOT),
    (R_HIP, R_KNEE),
    (R_KNEE, R_FOOT),
    (L_SHOULDER, R_SHOULDER),
    (L_HIP, R_HIP),
    (L_SHOULDER, L_HIP),
    (R_SHOULDER, R_HIP),
    (L_SHOULDER, R_HIP),
    (R_SHOULDER, L_HIP),
    (L_SHOULDER, PELVIS),
    (R_SHOULDER, PELVIS),
    (HEAD, L_SHOULDER),
    (HEAD, R_SHOULDER),
    (CHEST, L_HIP),
    (CHEST, R_HIP),
];

/// What each point weighs, in kilograms or near enough: the trunk heavy, the hands light.
const MASSES: [f32; JOINTS] = [
    12.0, 14.0, 6.0, 4.0, 2.5, 1.2, 4.0, 2.5, 1.2, 6.0, 4.0, 2.0, 6.0, 4.0, 2.0,
];

/// Pairs that may come closer than they stand but not further, and not closer than this
/// share of it: a limb folds at its middle joint, never inside out to nothing; a hand
/// never sinks into its own shoulder.
const REACH: [(usize, usize, f32); 6] = [
    (L_HIP, L_FOOT, 0.45),
    (R_HIP, R_FOOT, 0.45),
    (L_SHOULDER, L_HAND, 0.25),
    (R_SHOULDER, R_HAND, 0.25),
    (HEAD, PELVIS, 0.92),
    (L_FOOT, R_FOOT, 0.25),
];

/// The hinges: (upper, middle, lower) of a limb, and whether the middle bends forward
/// (a knee) or back (an elbow) of the line from upper to lower.
const HINGES: [(usize, usize, usize, bool); 4] = [
    (L_HIP, L_KNEE, L_FOOT, true),
    (R_HIP, R_KNEE, R_FOOT, true),
    (L_SHOULDER, L_ELBOW, L_HAND, false),
    (R_SHOULDER, R_ELBOW, R_HAND, false),
];

/// One step of the simulation, in game server milliseconds, and in seconds.
pub(crate) const STEP_MS: i32 = 16;
const STEP: f32 = STEP_MS as f32 / 1000.0;
/// `g_gravity`.
const GRAVITY: f32 = 800.0;
/// How much of its speed a point keeps from one step to the next, in the air.
const DRAG: f32 = 0.995;
/// Rounds of the sticks per step: more is stiffer.
const ROUNDS: usize = 6;
/// Half the size of each point's box in the world's collision: about how thick the body
/// is there, so that it lies on a floor rather than in it.
const HALVES: [f32; JOINTS] = [
    4.5, 5.0, 5.0, 3.0, 2.5, 2.0, 3.0, 2.5, 2.0, 3.5, 3.0, 2.5, 3.5, 3.0, 2.5,
];
/// Coulomb's friction of a point on a surface: it loses this much of its slide for what
/// it presses into the surface; and the share of its slide it keeps besides, each step
/// it touches (a body rolls and rubs, it does not skate). It bounces back with this much
/// of its speed into the surface.
const FRICTION: f32 = 0.8;
const CONTACT_KEPT: f32 = 0.985;
const BOUNCE: f32 = 0.2;
/// Slower into a surface than this (units a second), a point does not bounce, and its
/// knock is not heard.
const BOUNCE_SPEED: f32 = 60.0;
/// The fastest a point bounces back off a surface, units a second: flesh takes a hard
/// blow in, it does not spring a body back across the room.
const MAX_BOUNCE: f32 = 120.0;
/// A point that moves less than this in a step is still (the sticks and the floor keep
/// a lying body trembling by a tenth of a unit); the body sleeps once all are, for this
/// many steps in a row (a second).
const STILL: f32 = 0.2;
pub(crate) const SLEEP_STEPS: u32 = 60;
/// The body sleeps after this many steps whatever it does (40 seconds).
pub(crate) const MAX_STEPS: u32 = 2_500;
/// A point that falls this far below where the body started stops: the void under a map.
const FLOOR_DEPTH: f32 = 8_192.0;
/// The fastest a point may go, in units a step (3,500 units a second: the traces sweep
/// the whole way, so a fast point does not pass through a wall).
const TOP_SPEED: f32 = 56.0;
/// The hardest push a [`Strike`] gives, in units a second, and the fastest it may turn
/// the body (radians a second).
pub(crate) const MAX_PUSH: i32 = 3_000;
const MAX_TURN: f32 = 24.0;
/// The share of a strike's push its own point and the points next to it take again, on
/// top of the whole body's: the limb struck whips.
const WHIP: f32 = 0.35;
/// A point sliding along a surface faster than this (units a second) scrapes.
const SCRAPE_SPEED: f32 = 80.0;

/// How a fall starts: where the player stood (their origin, 24 units above their feet,
/// as the game keeps it), how fast they went, which way they faced (degrees), and the
/// moment (game server milliseconds), which also varies how the body crumples; and how
/// big the player is drawn (`scale`, a percentage, [`STOCK_SCALE`] the stock size).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Start {
    pub(crate) moment: i32,
    pub(crate) origin: [i32; 3],
    pub(crate) velocity: [i32; 3],
    pub(crate) yaw: u16,
    /// The player's `iModelScale` as [`scale_percent`] reads it: the body stands this
    /// share of the stock humanoid's size, its feet where the scaled model's are
    /// (`CG_Player` lowers a scaled model by `24 * (scale - 1)`, so they stay on the
    /// floor), its boxes and sticks scaled alike so it lies on the floor, not in or over it.
    pub(crate) scale: u16,
}

/// The stock size, as a percentage: what a fall with no `modelScale` takes.
pub(crate) const STOCK_SCALE: u16 = 100;
/// The largest scale a fall takes: `iModelScale` is sent in 10 bits.
const MAX_SCALE: u16 = 1_023;

/// A player's `iModelScale` (entity or player state) as a fall's [`Start::scale`]: zero,
/// the network's "no custom scale", is the stock size, as `CG_Player` draws it.
pub(crate) fn scale_percent(model_scale: i32) -> u16 {
    match model_scale {
        0 => STOCK_SCALE,
        scale => scale.clamp(1, i32::from(MAX_SCALE)) as u16,
    }
}

/// A blow to the body: at the start of step `step` since the fall's moment, the body
/// takes a push of `push` (units a second, what its centre of mass gains) at point
/// `joint`, as a solid would (the push off the centre turns it), the struck limb whipping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Strike {
    pub(crate) step: u32,
    pub(crate) joint: u8,
    pub(crate) push: [i32; 3],
}

/// What a point met: where it stopped, the surface's normal, what the surface is made
/// of (a footstep family, for its sound), and the mover it is, if one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    pub(crate) stop: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) surface: u8,
    pub(crate) mover: Option<u16>,
}

/// What the world is like round a point.
pub(crate) trait World {
    /// What a box of half size `half` meets moving from one point to another.
    fn trace(&mut self, from: Vec3, to: Vec3, half: f32) -> Option<Hit>;
    /// Stand the movers near the body (its points between `low` and `high`) where they
    /// are at `time` (game server milliseconds), the end of a step, and a step before.
    fn at(&mut self, _time: i32, _low: Vec3, _high: Vec3) {}
    /// Where mover `mover`'s move over the step takes a point at `at` that rides it.
    fn carry(&mut self, _mover: u16, at: Vec3) -> Vec3 {
        at
    }
    /// The mover that moved into a box of half size `half` at `at` over the step.
    fn pusher(&mut self, _at: Vec3, _half: f32) -> Option<u16> {
        None
    }
}

/// A world of brushes alone, giving the stop and the normal.
impl<F: FnMut(Vec3, Vec3, f32) -> Option<(Vec3, Vec3)>> World for F {
    fn trace(&mut self, from: Vec3, to: Vec3, half: f32) -> Option<Hit> {
        self(from, to, half).map(|(stop, normal)| Hit {
            stop,
            normal,
            surface: 0,
            mover: None,
        })
    }
}

/// A point that hit a surface (how hard, into it) or slid along one (how fast), in units
/// a second, and what the surface is made of ([`Hit::surface`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Knock {
    pub(crate) joint: usize,
    pub(crate) speed: f32,
    pub(crate) surface: u8,
}

/// The body, as it falls.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Body {
    /// Each point now, and a step ago (the speed is the difference).
    points: [Vec3; JOINTS],
    previous: [Vec3; JOINTS],
    /// Each point before the last step, where it was shown (unlike `previous`, which
    /// a hit changes to change the speed).
    earlier: [Vec3; JOINTS],
    lengths: [f32; STICKS.len()],
    reach: [f32; REACH.len()],
    /// The blows the body takes, by step (those taken and to come).
    strikes: Vec<Strike>,
    /// The fall's moment (game server milliseconds).
    moment: i32,
    /// Steps taken since the moment, how many of the last in a row all were still, and
    /// how many since the start or since a mover last woke the body.
    steps: u32,
    still: u32,
    restless: u32,
    /// The mover each point lay on at the last step, and how far that step carried it.
    riding: [Option<u16>; JOINTS],
    ride: [Vec3; JOINTS],
    /// Since [`Body::run_to`] began: the points that hit the world, and that slid along it.
    pub(crate) knocks: Vec<Knock>,
    pub(crate) scrapes: Vec<Knock>,
    lowest: f32,
    /// How hard it falls, units a second squared: [`GRAVITY`], or none while it floats
    /// (the Toy Master's shows, `toy_master::float`).
    gravity: f32,
    /// [`Start::scale`] as a factor: every length of the body (its points standing,
    /// sticks, reaches, boxes) is the stock one times this.
    scale: f32,
}

/// A number in 0..1 from `seed` and `salt`, the same on every client.
pub(crate) fn unit(seed: u32, salt: u32) -> f32 {
    let mut mixed = seed ^ salt.wrapping_mul(0x9e37_79b9);
    mixed ^= mixed >> 16;
    mixed = mixed.wrapping_mul(0x85eb_ca6b);
    mixed ^= mixed >> 13;
    mixed = mixed.wrapping_mul(0xc2b2_ae35);
    mixed ^= mixed >> 16;
    (mixed >> 8) as f32 / (1 << 24) as f32
}

/// The cosine and sine of `yaw` (a whole number of degrees), from a table of whole
/// degrees so that every client gets the same, whatever its maths library.
fn turn(yaw: u16) -> (f32, f32) {
    let degrees = i32::from(yaw % 360);
    // Quarter turns are exact; within one, Bhaskara's approximation is enough to
    // place a body, and it is plain arithmetic.
    let quarter = degrees / 90;
    let within = (degrees % 90) as f32;
    let sine = |d: f32| {
        // sin(d degrees) for 0 <= d <= 180.
        4.0 * d * (180.0 - d) / (40_500.0 - d * (180.0 - d))
    };
    let (s, c) = (sine(within), sine(90.0 - within));
    match quarter {
        0 => (c, s),
        1 => (-s, c),
        2 => (-c, -s),
        _ => (s, -c),
    }
}

impl Body {
    /// The body at the start of a fall: standing where the player stood, turned their
    /// way, with their speed, given a push off balance (back and to one side, the knees
    /// buckling) that the moment varies.
    pub(crate) fn new(start: &Start) -> Self {
        let (cos, sin) = turn(start.yaw);
        let scale = f32::from(start.scale) / f32::from(STOCK_SCALE);
        let feet = Vec3::new(
            start.origin[0] as f32,
            start.origin[1] as f32,
            start.origin[2] as f32 - 24.0,
        );
        let speed = Vec3::new(
            start.velocity[0] as f32,
            start.velocity[1] as f32,
            start.velocity[2] as f32,
        ) * STEP;
        let seed = start.moment as u32;
        let forward = Vec3::new(cos, sin, 0.0);
        let left = Vec3::new(-sin, cos, 0.0);
        // The topple, in units a second: the trunk goes back, the knees forward.
        let back = -forward * (70.0 + 50.0 * unit(seed, 1)) + left * (60.0 * unit(seed, 2) - 30.0);
        let buckle = forward * (40.0 + 30.0 * unit(seed, 3));
        let mut points = [Vec3::ZERO; JOINTS];
        let mut previous = [Vec3::ZERO; JOINTS];
        for (joint, standing) in STANDING.iter().enumerate() {
            let [x, y, z] = standing.map(|c| c * scale);
            let at = feet + forward * x + left * y + Vec3::Z * z;
            let push = match joint {
                CHEST | HEAD | L_SHOULDER | R_SHOULDER => back,
                L_ELBOW | R_ELBOW | L_HAND | R_HAND => back * 0.6,
                L_KNEE | R_KNEE => buckle,
                _ => Vec3::ZERO,
            } * STEP;
            points[joint] = at;
            previous[joint] = at - speed - push;
        }
        let standing = |joint: usize| Vec3::from_array(STANDING[joint]) * scale;
        let lengths = STICKS.map(|(a, b)| standing(a).distance(standing(b)));
        let reach = REACH.map(|(a, b, share)| standing(a).distance(standing(b)) * share);
        Self {
            points,
            previous,
            earlier: points,
            lengths,
            reach,
            strikes: Vec::new(),
            moment: start.moment,
            steps: 0,
            still: 0,
            restless: 0,
            riding: [None; JOINTS],
            ride: [Vec3::ZERO; JOINTS],
            knocks: Vec::with_capacity(JOINTS),
            scrapes: Vec::with_capacity(JOINTS),
            lowest: feet.z - FLOOR_DEPTH,
            gravity: GRAVITY,
            scale,
        }
    }

    /// Where each point is now.
    pub(crate) fn points(&self) -> &[Vec3; JOINTS] {
        &self.points
    }

    /// Where each point is `fraction` of the way from the last step to the next, for
    /// a frame between two steps (from the last step's start to its end: the body is
    /// shown a step late, which no one sees).
    pub(crate) fn between(&self, fraction: f32) -> [Vec3; JOINTS] {
        std::array::from_fn(|joint| self.earlier[joint].lerp(self.points[joint], fraction))
    }

    /// Float weightless, or fall again (waking the body).
    pub(crate) fn weightless(&mut self, floating: bool) {
        self.gravity = if floating { 0.0 } else { GRAVITY };
        self.still = 0;
        self.restless = 0;
    }

    /// Steps taken since the moment.
    pub(crate) fn steps(&self) -> u32 {
        self.steps
    }

    /// Whether it lies still (or has run for [`MAX_STEPS`] since it last woke).
    pub(crate) fn asleep(&self) -> bool {
        self.still >= SLEEP_STEPS || self.restless >= MAX_STEPS
    }

    /// Where the body is, for the camera and the player's own body: its pelvis.
    pub(crate) fn centre(&self) -> Vec3 {
        self.points[PELVIS]
    }

    /// Add `strike` to the blows the body takes; false (and not added) when its step has
    /// gone by: the body must then be run again from its start with it
    /// ([`Body::rerun`]), as every client that heard of it in time ran it.
    pub(crate) fn strike(&mut self, strike: Strike) -> bool {
        if strike.step < self.steps {
            return false;
        }
        if !self.strikes.contains(&strike) {
            let at = self.strikes.partition_point(|s| s.step <= strike.step);
            self.strikes.insert(at, strike);
        }
        true
    }

    /// Whether the body took or will take `strike`.
    pub(crate) fn has_strike(&self, strike: &Strike) -> bool {
        self.strikes.contains(strike)
    }

    /// How many blows the body takes, those taken and to come.
    pub(crate) fn strike_count(&self) -> usize {
        self.strikes.len()
    }

    /// The body from `start` again, with every blow it took and `strike`, not yet run
    /// (the next [`Body::run_to`] fast-forwards it).
    pub(crate) fn rerun(&self, start: &Start, strike: Strike) -> Self {
        let mut body = Self::new(start);
        body.strikes.clone_from(&self.strikes);
        body.strike(strike);
        body
    }

    /// The body from `start` again, with every blow it took but `strike` (one this
    /// client foresaw that did not come so), not yet run.
    pub(crate) fn without(&self, start: &Start, strike: &Strike) -> Self {
        let mut body = Self::new(start);
        body.strikes = self
            .strikes
            .iter()
            .filter(|taken| *taken != strike)
            .copied()
            .collect();
        body
    }

    /// Step on to `steps` steps since the moment, against `world`. Asleep, the steps go
    /// by with the body lying still, unless a mover wakes it or a blow comes.
    pub(crate) fn run_to(&mut self, steps: u32, world: &mut impl World) {
        self.knocks.clear();
        self.scrapes.clear();
        while self.steps < steps {
            if self.asleep() && !self.struck_now() && !self.stirred(world) {
                self.steps += 1;
                continue;
            }
            self.step(world);
        }
        if self.asleep() {
            self.earlier = self.points;
        }
    }

    /// The game server time at the end of step `steps`.
    fn time_of(&self, steps: u32) -> i32 {
        self.moment
            .wrapping_add((steps as i32).wrapping_mul(STEP_MS))
    }

    /// The box round every point.
    fn bounds(&self) -> (Vec3, Vec3) {
        self.points
            .iter()
            .fold((self.points[0], self.points[0]), |(low, high), point| {
                (low.min(*point), high.max(*point))
            })
    }

    /// Whether a mover wakes the sleeping body at its next step: one carrying a point
    /// that lies on it, or moving into one.
    fn stirred(&mut self, world: &mut impl World) -> bool {
        let (low, high) = self.bounds();
        world.at(self.time_of(self.steps + 1), low, high);
        let stirred = (0..JOINTS).any(|joint| {
            let at = self.points[joint];
            self.riding[joint].is_some_and(|mover| world.carry(mover, at) != at)
                || world.pusher(at, self.half(joint)).is_some()
        });
        if stirred {
            self.still = 0;
            self.restless = 0;
        }
        stirred
    }

    /// Whether a blow comes at the start of the next step.
    fn struck_now(&self) -> bool {
        self.strikes.iter().any(|strike| strike.step == self.steps)
    }

    /// The centre of the points' masses.
    fn centre_of_mass(&self) -> Vec3 {
        let total: f32 = MASSES.iter().sum();
        self.points
            .iter()
            .zip(MASSES)
            .fold(Vec3::ZERO, |sum, (point, mass)| sum + *point * mass)
            / total
    }

    /// Give the body the blows of the step about to begin: each pushes the centre of
    /// mass and turns the body by its push off the centre (the inertia of the points as
    /// they lie), the struck limb whipping on top.
    fn take_strikes(&mut self) {
        let first = self.strikes.partition_point(|s| s.step < self.steps);
        let end = self.strikes.partition_point(|s| s.step <= self.steps);
        if first == end {
            return;
        }
        let total: f32 = MASSES.iter().sum();
        for index in first..end {
            let strike = self.strikes[index];
            let push = Vec3::new(
                strike.push[0] as f32,
                strike.push[1] as f32,
                strike.push[2] as f32,
            );
            let joint = usize::from(strike.joint).min(JOINTS - 1);
            let centre = self.centre_of_mass();
            // The inertia about the centre, a ball's share added so that a body lying
            // straight still has some about its length.
            let mut inertia =
                glam::Mat3::from_diagonal(Vec3::splat(total * 20.0 * self.scale * self.scale));
            for (point, mass) in self.points.iter().zip(MASSES) {
                let r = *point - centre;
                let rr = r.length_squared();
                inertia += glam::Mat3::from_cols(
                    Vec3::new(rr - r.x * r.x, -r.x * r.y, -r.x * r.z),
                    Vec3::new(-r.y * r.x, rr - r.y * r.y, -r.y * r.z),
                    Vec3::new(-r.z * r.x, -r.z * r.y, rr - r.z * r.z),
                ) * mass;
            }
            let lever = self.points[joint] - centre;
            let turn = (inertia.inverse() * lever.cross(push * total)).clamp_length_max(MAX_TURN);
            for point in 0..JOINTS {
                let mut gain = push + turn.cross(self.points[point] - centre);
                if point == joint || boned(joint, point) {
                    gain += push * WHIP;
                }
                self.previous[point] -= gain * STEP;
            }
        }
        self.still = 0;
        self.restless = 0;
    }

    /// One step: the blows due land, the movers carry or push the points they hold,
    /// then every point moves on by its speed and gravity, out of the world's way, and
    /// the sticks hold.
    fn step(&mut self, world: &mut impl World) {
        self.take_strikes();
        self.steps += 1;
        self.restless += 1;
        let began = self.points;
        self.earlier = began;
        let (low, high) = self.bounds();
        world.at(self.time_of(self.steps), low, high);
        // A point on a mover goes with it, and one a mover moves into is pushed along:
        // both keep their own speed on top (the previous point moves as well).
        let mut carried = false;
        for joint in 0..JOINTS {
            let at = self.points[joint];
            let mover = self.riding[joint].or_else(|| world.pusher(at, self.half(joint)));
            let shift = mover.map_or(Vec3::ZERO, |mover| world.carry(mover, at) - at);
            self.points[joint] += shift;
            self.previous[joint] += shift;
            self.ride[joint] = shift;
            carried |= shift != Vec3::ZERO;
        }
        let fall = Vec3::new(0.0, 0.0, -self.gravity * STEP * STEP);
        let mut touched = [None; JOINTS];
        for (joint, touch) in touched.iter_mut().enumerate() {
            let at = self.points[joint];
            let speed = ((at - self.previous[joint]) * DRAG + fall).clamp_length_max(TOP_SPEED);
            let mut to = at + speed;
            to.z = to.z.max(self.lowest);
            self.previous[joint] = at;
            let (point, hit) = self.collide(joint, at, to, world);
            self.points[joint] = point;
            *touch = hit;
        }
        let moved_to = self.points;
        for _ in 0..ROUNDS {
            self.hold();
        }
        // The sticks may have pushed points into the world: trace each from where it
        // was before them.
        let mut moved = 0.0_f32;
        for (joint, &from) in moved_to.iter().enumerate() {
            let to = self.points[joint];
            if let Some(hit) = world.trace(from, to, self.half(joint)) {
                touched[joint] = touched[joint].or(Some(hit));
                // What is left slides along the surface, as far as the world lets it.
                let rest = to - hit.stop;
                let slide = rest - hit.normal * rest.dot(hit.normal);
                let base = hit.stop + hit.normal * 0.125;
                self.points[joint] = match world.trace(base, base + slide, self.half(joint)) {
                    None => base + slide,
                    Some(hit) => hit.stop + hit.normal * 0.125,
                };
                // Put back out of the surface, the point keeps no speed away from it:
                // else the way out becomes a bounce, and a body thrown at a wall flies
                // back off it.
                self.previous[joint] = self.points[joint]
                    - unbounced(self.points[joint] - self.previous[joint], hit.normal);
            }
            moved = moved.max(self.points[joint].distance(began[joint]));
        }
        // A point that leaves the mover it lay on takes the mover's speed with it.
        for (joint, touch) in touched.iter().enumerate() {
            if self.riding[joint].is_some() && touch.is_none() {
                self.previous[joint] -= self.ride[joint];
            }
            self.riding[joint] = touch.and_then(|hit| hit.mover);
        }
        if moved < STILL && !carried {
            self.still += 1;
        } else {
            self.still = 0;
        }
    }

    /// Where a point going from `from` to `to` ends up, and what it met: where it meets
    /// the world, its speed off the surface kept in part ([`FRICTION_KEPT`], [`BOUNCE`]).
    fn collide(
        &mut self,
        joint: usize,
        from: Vec3,
        to: Vec3,
        world: &mut impl World,
    ) -> (Vec3, Option<Hit>) {
        let Some(hit) = world.trace(from, to, self.half(joint)) else {
            return (to, None);
        };
        let normal = hit.normal;
        let speed = to - from;
        let into = speed.dot(normal);
        let along = speed - normal * into;
        let hard = -into.min(0.0) / STEP;
        let back = if hard > BOUNCE_SPEED {
            (hard * BOUNCE).min(MAX_BOUNCE) * STEP
        } else {
            0.0
        };
        let after = friction(along, -into.min(0.0), FRICTION) * CONTACT_KEPT + normal * back;
        let at = hit.stop + normal * 0.125;
        self.previous[joint] = at - after;
        let surface = hit.surface;
        if hard > BOUNCE_SPEED {
            self.knocks.push(Knock {
                joint,
                speed: hard,
                surface,
            });
        }
        let slide = along.length() / STEP;
        if slide > SCRAPE_SPEED && normal != Vec3::ZERO {
            self.scrapes.push(Knock {
                joint,
                speed: slide,
                surface,
            });
        }
        (at, Some(hit))
    }

    /// One round of the sticks, the reaches, the limbs kept apart and the hinges.
    fn hold(&mut self) {
        for (index, &(a, b)) in STICKS.iter().enumerate() {
            self.keep(a, b, self.lengths[index], self.lengths[index]);
        }
        for (index, &(a, b, _)) in REACH.iter().enumerate() {
            let standing = Vec3::from_array(STANDING[a]).distance(Vec3::from_array(STANDING[b]));
            self.keep(a, b, self.reach[index], standing * self.scale);
        }
        self.apart();
        let forward = self.facing();
        for &(upper, middle, lower, forth) in &HINGES {
            let line = (self.points[upper] + self.points[lower]) * 0.5;
            let out = (self.points[middle] - line).dot(forward);
            let wrong = if forth { -out } else { out - 1.5 * self.scale };
            if wrong > 0.0 {
                let push = forward * if forth { wrong } else { -wrong };
                self.points[middle] += push * 0.5;
                self.points[upper] -= push * 0.25;
                self.points[lower] -= push * 0.25;
            }
        }
    }

    /// Keep the points no stick or reach holds out of each other: no closer than their
    /// boxes' sizes together, or than they stand if that is closer (a hand hangs by the
    /// hip). A leg no longer passes through the other, nor a hand through the head.
    fn apart(&mut self) {
        for &(a, b, shortest) in APART.iter() {
            self.keep(a, b, shortest * self.scale, f32::MAX);
        }
    }

    /// Hold `a` and `b` between `shortest` and `longest` apart, each moved by its share
    /// of their weight (the lighter further).
    fn keep(&mut self, a: usize, b: usize, shortest: f32, longest: f32) {
        let gap = self.points[b] - self.points[a];
        let length = gap.length();
        if length < 1e-4 {
            return;
        }
        let target = length.clamp(shortest, longest);
        if target == length {
            return;
        }
        let shift = gap * ((length - target) / length);
        let (light_a, light_b) = (1.0 / MASSES[a], 1.0 / MASSES[b]);
        let share = light_a / (light_a + light_b);
        self.points[a] += shift * share;
        self.points[b] -= shift * (1.0 - share);
    }

    /// Half the size of `joint`'s box in the world's collision, at the body's scale.
    fn half(&self, joint: usize) -> f32 {
        HALVES[joint] * self.scale
    }

    /// The way the trunk faces: its left (hips, right to left) crossed with its up.
    pub(crate) fn facing(&self) -> Vec3 {
        let left = self.points[L_HIP] - self.points[R_HIP];
        let up = self.points[CHEST] - self.points[PELVIS];
        left.cross(up).normalize_or_zero()
    }
}

/// The bones: the sticks before the trunk's braces.
const BONES: usize = 14;

/// Whether a bone joins `a` and `b`.
fn boned(a: usize, b: usize) -> bool {
    STICKS[..BONES]
        .iter()
        .any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
}

/// A step's move `speed` of a point put back out of a surface of `normal`, without what
/// takes it away from the surface.
pub(crate) fn unbounced(speed: Vec3, normal: Vec3) -> Vec3 {
    let out = speed.dot(normal);
    if out > 0.0 {
        speed - normal * out
    } else {
        speed
    }
}

/// What is left of a slide `along` (a step's move along a surface) once friction of
/// `coefficient` took its share for a press of `press` (the step's move into it): none
/// when the press holds the point still (static friction).
pub(crate) fn friction(along: Vec3, press: f32, coefficient: f32) -> Vec3 {
    let slide = along.length();
    let lost = press * coefficient;
    if slide <= lost {
        Vec3::ZERO
    } else {
        along * (1.0 - lost / slide)
    }
}

/// The pairs [`Body::apart`] keeps apart: every two points no stick or reach holds, and
/// how close they may come.
static APART: std::sync::LazyLock<Vec<(usize, usize, f32)>> = std::sync::LazyLock::new(|| {
    let held = |a: usize, b: usize| {
        STICKS
            .iter()
            .any(|&(x, y)| (x, y) == (a, b) || (x, y) == (b, a))
            || REACH
                .iter()
                .any(|&(x, y, _)| (x, y) == (a, b) || (x, y) == (b, a))
    };
    let mut pairs = Vec::new();
    for a in 0..JOINTS {
        for b in a + 1..JOINTS {
            if held(a, b) {
                continue;
            }
            let standing = Vec3::from_array(STANDING[a]).distance(Vec3::from_array(STANDING[b]));
            pairs.push((a, b, (HALVES[a] + HALVES[b]).min(standing * 0.95)));
        }
    }
    pairs
});

/// `Start` as one relay event's text after its prefix: the moment, the origin, the
/// speed and the yaw, whole numbers parted by `_` (the relay takes 64 characters), then
/// the scale when it is not the stock size: a stock-sized fall reads as it did before
/// the scale was sent, so clients from before then still show it.
pub(crate) fn encode(start: &Start) -> String {
    let [x, y, z] = start.origin;
    let [vx, vy, vz] = start.velocity;
    let mut text = format!("{}_{x}_{y}_{z}_{vx}_{vy}_{vz}_{}", start.moment, start.yaw);
    if start.scale != STOCK_SCALE {
        text.push_str(&format!("_{}", start.scale));
    }
    text
}

/// The start [`encode`] wrote; `None` for anything else or out of bounds.
pub(crate) fn decode(text: &str) -> Option<Start> {
    let mut parts = text.split('_');
    let mut number = || parts.next()?.parse::<i32>().ok();
    let moment = number()?;
    let origin = [number()?, number()?, number()?];
    let velocity = [number()?, number()?, number()?];
    let yaw = u16::try_from(number()?).ok().filter(|yaw| *yaw < 360)?;
    let scale = match parts.next() {
        None => STOCK_SCALE,
        Some(scale) => scale
            .parse::<u16>()
            .ok()
            .filter(|scale| (1..=MAX_SCALE).contains(scale))?,
    };
    if parts.next().is_some()
        || origin.iter().any(|c| c.abs() > 65_536)
        || velocity.iter().any(|c| c.abs() > 4_000)
    {
        return None;
    }
    Some(Start {
        moment,
        origin,
        velocity,
        yaw,
        scale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor at height 0, and a wall at x = 100 if asked.
    fn floor(wall: bool) -> impl FnMut(Vec3, Vec3, f32) -> Option<(Vec3, Vec3)> {
        move |from: Vec3, to: Vec3, half: f32| {
            let mut best: Option<(f32, Vec3)> = None;
            let low = half;
            if to.z < low && from.z >= low - 1e-3 {
                let t = ((from.z - low) / (from.z - to.z)).clamp(0.0, 1.0);
                best = Some((t, Vec3::Z));
            } else if to.z < low {
                best = Some((0.0, Vec3::Z));
            }
            let face = 100.0 - half;
            if wall && to.x > face {
                let t = if from.x <= face {
                    ((face - from.x) / (to.x - from.x)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if best.is_none_or(|(bt, _)| t < bt) {
                    best = Some((t, -Vec3::X));
                }
            }
            best.map(|(t, normal)| {
                let mut at = from + (to - from) * t;
                if normal == Vec3::Z {
                    at.z = at.z.max(low);
                } else {
                    at.x = at.x.min(face);
                }
                (at, normal)
            })
        }
    }

    fn start(moment: i32) -> Start {
        Start {
            moment,
            origin: [0, 0, 24],
            velocity: [0, 0, 0],
            yaw: 90,
            scale: STOCK_SCALE,
        }
    }

    #[test]
    fn a_start_reads_back_and_fits_a_relay_event() {
        let start = Start {
            moment: 2_147_000_000,
            origin: [-65_000, 12_345, -4_000],
            velocity: [-3_999, 250, -1_200],
            yaw: 359,
            scale: STOCK_SCALE,
        };
        let text = format!("fall_{}", encode(&start));
        assert!(text.len() <= 64, "{text}");
        assert_eq!(decode(text.strip_prefix("fall_").unwrap()), Some(start));
        assert_eq!(decode("1_2_3"), None);
        assert_eq!(decode("1_0_0_0_0_0_0_360"), None);
        assert_eq!(decode("1_0_0_0_0_0_0_10_5_7"), None);
    }

    #[test]
    fn a_scaled_start_sends_its_scale_and_a_stock_one_reads_as_before() {
        // The stock size writes no scale: clients from before it was sent read it.
        assert_eq!(encode(&start(5)), "5_0_0_24_0_0_0_90");
        let worst = Start {
            moment: i32::MIN,
            origin: [-65_536; 3],
            velocity: [-4_000; 3],
            yaw: 359,
            scale: MAX_SCALE,
        };
        let small = Start {
            scale: 60,
            ..start(5)
        };
        for start in [worst, small] {
            let text = format!("fall_{}", encode(&start));
            assert!(text.len() <= 64, "{text}");
            assert_eq!(decode(text.strip_prefix("fall_").unwrap()), Some(start));
        }
        assert_eq!(encode(&small), "5_0_0_24_0_0_0_90_60");
        assert_eq!(decode("1_0_0_0_0_0_0_10_0"), None, "no zero scale");
        assert_eq!(decode("1_0_0_0_0_0_0_10_1024"), None, "past 10 bits");
        // `iModelScale` 0 is the stock size; anything else is its percentage.
        assert_eq!(scale_percent(0), STOCK_SCALE);
        assert_eq!(scale_percent(60), 60);
        assert_eq!(scale_percent(-5), 1);
        assert_eq!(scale_percent(5_000), MAX_SCALE);
    }

    /// Hub report #223: a youngling or a jawa the server made small (`modelScale`)
    /// fell as a full-sized body. A body at 60 % stands 60 % as tall on the same feet,
    /// and lies on the floor as the stock one does, its boxes touching it: neither
    /// floating over it nor sunk into it.
    #[test]
    fn a_small_player_falls_as_a_small_body_lying_on_the_floor() {
        let stock = Body::new(&start(1_000));
        let small = Body::new(&Start {
            scale: 60,
            ..start(1_000)
        });
        let feet = Vec3::new(0.0, 0.0, 0.0);
        for (big, little) in stock.points().iter().zip(small.points()) {
            let expected = feet + (*big - feet) * 0.6;
            assert!(little.distance(expected) < 1e-3, "{little} {expected}");
        }
        let mut world = floor(false);
        for (mut body, scale) in [(stock, 1.0), (small, 0.6)] {
            body.run_to(MAX_STEPS, &mut world);
            assert!(body.asleep());
            let points = *body.points();
            // Every box rests on or above the floor, and the lowest lies on it.
            let gaps: Vec<f32> = (0..JOINTS)
                .map(|joint| points[joint].z - body.half(joint))
                .collect();
            let lowest = gaps.iter().copied().fold(f32::MAX, f32::min);
            assert!(gaps.iter().all(|gap| *gap > -0.5), "{gaps:?}");
            assert!(lowest < 0.5, "floating {lowest}");
            assert!(
                points[PELVIS].z < 10.0 * scale,
                "{scale}: pelvis {}",
                points[PELVIS]
            );
            // The sticks held at the body's size.
            let bone = points[L_HIP].distance(points[L_KNEE]);
            let standing = Vec3::from_array(STANDING[L_HIP])
                .distance(Vec3::from_array(STANDING[L_KNEE]))
                * scale;
            assert!((bone - standing).abs() < 1.0 * scale, "{bone} {standing}");
        }
    }

    #[test]
    fn the_turn_table_is_close_to_the_real_one() {
        for yaw in 0..360_u16 {
            let (c, s) = turn(yaw);
            let r = f32::from(yaw).to_radians();
            assert!(
                (c - r.cos()).abs() < 0.002 && (s - r.sin()).abs() < 0.002,
                "{yaw}"
            );
        }
    }

    #[test]
    fn the_body_falls_onto_the_floor_and_falls_asleep_lying_down() {
        let mut body = Body::new(&start(1_000));
        let mut world = floor(false);
        body.run_to(MAX_STEPS, &mut world);
        assert!(body.asleep());
        assert!(body.still >= SLEEP_STEPS, "never came to rest");
        let points = body.points();
        for point in points {
            assert!(point.z >= 1.9 && point.z < 20.0, "{point}");
        }
        // Lying, not standing: the head is low, and away from the feet's spot.
        assert!(points[HEAD].z < 15.0);
        // The sticks held.
        let bone = points[L_HIP].distance(points[L_KNEE]);
        let standing =
            Vec3::from_array(STANDING[L_HIP]).distance(Vec3::from_array(STANDING[L_KNEE]));
        assert!((bone - standing).abs() < 1.0, "{bone} {standing}");
    }

    #[test]
    fn the_same_start_falls_the_same_way_however_the_steps_are_taken() {
        let mut world = floor(true);
        let mut once = Body::new(&start(77));
        once.run_to(400, &mut world);
        let mut piecemeal = Body::new(&start(77));
        for steps in (0..=400).step_by(7) {
            piecemeal.run_to(steps, &mut world);
        }
        piecemeal.run_to(400, &mut world);
        assert_eq!(once.points(), piecemeal.points());
        let mut other = Body::new(&start(78));
        other.run_to(400, &mut world);
        assert_ne!(once.points(), other.points());
    }

    /// A lift, mover 7: a floor whose top rises from 0 to 200 between 6 and 8 seconds.
    #[derive(Default)]
    struct Lift {
        top: f32,
        before: f32,
    }

    impl Lift {
        fn height(time: i32) -> f32 {
            ((time - 6_000) as f32 / 10.0).clamp(0.0, 200.0)
        }
    }

    impl World for Lift {
        fn trace(&mut self, from: Vec3, to: Vec3, half: f32) -> Option<Hit> {
            let low = self.top + half;
            if to.z >= low {
                return None;
            }
            let t = if from.z > low {
                (from.z - low) / (from.z - to.z)
            } else {
                0.0
            };
            let mut stop = from + (to - from) * t;
            stop.z = stop.z.max(low);
            Some(Hit {
                stop,
                normal: Vec3::Z,
                surface: 6,
                mover: Some(7),
            })
        }

        fn at(&mut self, time: i32, _low: Vec3, _high: Vec3) {
            self.before = Self::height(time - STEP_MS);
            self.top = Self::height(time);
        }

        fn carry(&mut self, mover: u16, at: Vec3) -> Vec3 {
            assert_eq!(mover, 7);
            at + Vec3::Z * (self.top - self.before)
        }

        fn pusher(&mut self, at: Vec3, half: f32) -> Option<u16> {
            let bottom = at.z - half;
            (bottom < self.top && bottom >= self.before).then_some(7)
        }
    }

    #[test]
    fn a_lift_carries_the_sleeping_body_up_and_it_sleeps_again_on_top() {
        let mut body = Body::new(&start(1_000));
        let mut lift = Lift::default();
        // Lying still on the lift before it moves.
        body.run_to(300, &mut lift);
        assert!(body.asleep());
        assert!(body.points().iter().all(|point| point.z < 20.0));
        assert!(body.knocks.iter().any(|knock| knock.surface == 6));
        // Up with it, still lying (no point left behind or thrown off), and asleep
        // again once it has stopped; the same however the steps are taken.
        let mut piecemeal = body.clone();
        body.run_to(600, &mut lift);
        for point in body.points() {
            assert!(point.z > 200.0 && point.z < 220.0, "{point}");
        }
        assert!(body.asleep());
        for steps in (300..=600).step_by(5) {
            piecemeal.run_to(steps, &mut lift);
        }
        assert_eq!(body.points(), piecemeal.points());
    }

    #[test]
    fn a_strike_sends_the_body_flying_its_way_and_it_lands_in_one_piece() {
        let mut body = Body::new(&start(9));
        assert!(body.strike(Strike {
            step: 0,
            joint: CHEST as u8,
            push: [1_650, 0, 260],
        }));
        let mut world = floor(false);
        body.run_to(15, &mut world);
        // A quarter of a second on, it is well on its way, off the ground.
        let centre = body.centre_of_mass();
        assert!(centre.x > 250.0, "{centre}");
        assert!(body.points().iter().all(|p| p.z > 2.0));
        body.run_to(MAX_STEPS, &mut world);
        assert!(body.still >= SLEEP_STEPS, "came to rest");
        assert!(
            body.centre().x > 600.0,
            "slid and tumbled on: {}",
            body.centre()
        );
        for (index, &(a, b)) in STICKS.iter().enumerate() {
            let length = body.points[a].distance(body.points[b]);
            assert!(
                (length - body.lengths[index]).abs() < 1.5,
                "{a} {b} {length}"
            );
        }
    }

    #[test]
    fn a_blow_to_the_head_turns_the_body_the_other_way_from_one_to_the_legs() {
        // Struck along +X: at the head, the head goes first and the feet trail; at the
        // knees, the feet go first and the head trails.
        let spin = |joint: usize| {
            let mut body = Body::new(&Start {
                velocity: [0, 0, 0],
                ..start(3)
            });
            body.strike(Strike {
                step: 0,
                joint: joint as u8,
                push: [900, 0, 0],
            });
            body.run_to(6, &mut |_: Vec3, _: Vec3, _: f32| None);
            let head = body.points()[HEAD].x - body.centre_of_mass().x;
            let feet =
                (body.points()[L_FOOT].x + body.points()[R_FOOT].x) * 0.5 - body.centre_of_mass().x;
            head - feet
        };
        assert!(spin(HEAD) > 4.0, "{}", spin(HEAD));
        assert!(spin(L_KNEE) < -4.0, "{}", spin(L_KNEE));
    }

    #[test]
    fn a_strike_heard_late_gives_the_body_one_heard_in_time() {
        let mut world = floor(true);
        let strike = Strike {
            step: 30,
            joint: HEAD as u8,
            push: [-700, 300, 500],
        };
        let mut in_time = Body::new(&start(41));
        assert!(in_time.strike(strike));
        in_time.run_to(400, &mut world);
        let mut late = Body::new(&start(41));
        late.run_to(120, &mut world);
        assert!(!late.strike(strike), "its step went by");
        let mut late = late.rerun(&start(41), strike);
        late.run_to(400, &mut world);
        assert_eq!(in_time.points(), late.points());
        // And a sleeping body wakes for a blow.
        let mut asleep = Body::new(&start(41));
        asleep.run_to(MAX_STEPS, &mut world);
        assert!(asleep.asleep());
        let lying = asleep.centre();
        let steps = asleep.steps();
        asleep.strike(Strike {
            step: steps,
            ..strike
        });
        asleep.run_to(steps + 20, &mut world);
        assert!(asleep.centre().distance(lying) > 20.0);
    }

    #[test]
    fn friction_holds_a_slow_slide_and_slows_a_fast_one() {
        let slow = friction(Vec3::new(0.1, 0.0, 0.0), 0.2, 0.8);
        assert_eq!(slow, Vec3::ZERO);
        let fast = friction(Vec3::new(10.0, 0.0, 0.0), 0.2, 0.8);
        assert!((fast.x - 9.84).abs() < 1e-4, "{fast}");
    }

    #[test]
    fn the_limbs_keep_out_of_each_other() {
        // A body dropped folded: its feet thrown into each other and its hand at its head.
        let mut body = Body::new(&start(12));
        body.points[R_FOOT] = body.points[L_FOOT];
        body.points[L_HAND] = body.points[HEAD];
        for _ in 0..ROUNDS * 4 {
            body.hold();
        }
        assert!(body.points[R_FOOT].distance(body.points[L_FOOT]) > 2.0);
        assert!(body.points[L_HAND].distance(body.points[HEAD]) > 5.0);
    }

    #[test]
    fn a_body_slammed_into_a_wall_drops_by_it_rather_than_flying_back() {
        let mut body = Body::new(&start(21));
        body.strike(Strike {
            step: 0,
            joint: CHEST as u8,
            push: [2_400, 0, 200],
        });
        let mut world = floor(true);
        body.run_to(MAX_STEPS, &mut world);
        assert!(body.asleep());
        // The wall is at x = 100: it lies within a body's length of it.
        assert!(body.centre().x > 40.0, "flew back to {}", body.centre());
        assert!(body.points().iter().all(|point| point.x <= 100.0));
    }

    #[test]
    fn a_body_thrown_at_a_wall_stops_against_it() {
        let mut body = Body::new(&Start {
            velocity: [600, 0, 0],
            ..start(5)
        });
        let mut world = floor(true);
        body.run_to(MAX_STEPS, &mut world);
        for point in body.points() {
            assert!(point.x <= 100.0, "{point}");
        }
        assert!(body.knocks.is_empty() || body.asleep());
    }
}
