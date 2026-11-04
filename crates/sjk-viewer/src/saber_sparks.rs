//! Sparks from a blade skin's emitter (`sparks` in its file,
//! [`crate::blade_skin_file::smoke::Sparks`]): each blade's emitter crackles, letting go
//! bursts of sparks at random moments (a Poisson stream, `rate` bursts a second while lit,
//! more for the `ignite` seconds once lit, fewer and fewer for the `after` seconds once put
//! out). A spark is one of the game's particles shot out of the emitter in a cone round
//! the world's up, the blade's way or between (`aim`), falling under gravity, its streak drawn along the way it goes
//! ([`PrimitiveShape::Spark`]), shrinking and fading as it dies. Only a skin with the
//! section sparks; a full particle pool takes no more.

use crate::effect_library::EffectLibrary;
use crate::particle_types::{MAX_PARTICLES, Particle, PrimitiveShape};
use glam::Vec3;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Most spark sets (and bolts) one skin's blades carry: three sets of sparks and bolts.
pub(crate) const MOST_SETS: usize = crate::blade_skin_file::smoke::MAX_SPARK_SETS + 1;

/// Most bursts one emitter lets go in a frame.
const MOST_A_FRAME: u32 = 4;
/// Milliseconds an emitter is not told of before it is forgotten, and before a frame
/// after a gap lets no bursts go for the gap.
const FORGET_MILLIS: i64 = 500;
const GAP_MILLIS: i64 = 250;

/// A skin's sparks, small and `Copy` for the blades to carry
/// ([`crate::saber_skins::SkinColor`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SparkSpec {
    pub(crate) shader: &'static str,
    pub(crate) color: [f32; 3],
    pub(crate) opacity: f32,
    pub(crate) rate: f32,
    pub(crate) ignite: f32,
    pub(crate) ignite_rate: f32,
    pub(crate) after: f32,
    pub(crate) after_rate: f32,
    pub(crate) burst: u32,
    pub(crate) speed: f32,
    pub(crate) speed_jitter: f32,
    pub(crate) spread: f32,
    pub(crate) aim: f32,
    pub(crate) gravity: f32,
    pub(crate) life: f32,
    pub(crate) size: f32,
    pub(crate) length: f32,
    pub(crate) along: f32,
    /// The wandering force on each spark (units a second², 0: none).
    pub(crate) wander: f32,
    /// Bursts let go at once when the blade clashes ([`Sparks::kick`]).
    pub(crate) clash: u32,
    /// What a burst lets go: sparks, or bolts of electricity.
    pub(crate) shape: Shape,
}

/// What a burst lets go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Shape {
    /// Sparks: streaks along their way, or round points (embers) with no `length`.
    Sparks,
    /// Bolts of electricity from the emitter to a point `reach` units off (each up to
    /// `reach_jitter` of it shorter), as jagged as `chaos`, forking when `branch`.
    Bolts {
        reach: f32,
        reach_jitter: f32,
        chaos: f32,
        branch: bool,
    },
}

impl SparkSpec {
    /// The section read from a skin's file.
    pub(crate) fn of(sparks: &crate::blade_skin_file::smoke::Sparks) -> Self {
        Self {
            shader: crate::saber_smoke::intern(&sparks.shader),
            color: sparks.color,
            opacity: sparks.opacity,
            rate: sparks.rate,
            ignite: sparks.ignite,
            ignite_rate: sparks.ignite_rate,
            after: sparks.after,
            after_rate: sparks.after_rate,
            burst: sparks.burst,
            speed: sparks.speed,
            speed_jitter: sparks.speed_jitter,
            spread: sparks.spread,
            aim: sparks.aim,
            gravity: sparks.gravity,
            life: sparks.life,
            size: sparks.size,
            length: sparks.length,
            along: sparks.along,
            wander: sparks.wander,
            clash: sparks.clash,
            shape: Shape::Sparks,
        }
    }

    /// A skin's `bolts`, let go as a burst's bolts.
    pub(crate) fn of_bolts(bolts: &crate::blade_skin_file::smoke::Bolts) -> Self {
        Self {
            shader: crate::saber_smoke::intern(&bolts.shader),
            color: bolts.color,
            opacity: bolts.opacity,
            rate: bolts.rate,
            ignite: bolts.ignite,
            ignite_rate: bolts.ignite_rate,
            after: bolts.after,
            after_rate: bolts.after_rate,
            burst: bolts.burst,
            speed: 0.0,
            speed_jitter: 0.0,
            spread: bolts.spread,
            aim: bolts.aim,
            gravity: 0.0,
            life: bolts.life,
            size: bolts.width,
            length: 0.0,
            along: bolts.along,
            wander: 0.0,
            clash: bolts.clash,
            shape: Shape::Bolts {
                reach: bolts.reach,
                reach_jitter: bolts.reach_jitter,
                chaos: bolts.chaos,
                branch: bolts.branch,
            },
        }
    }

    /// A skin's spark sets and bolts, as its blades carry them: at most
    /// [`MOST_SETS`], the sparks first; the sparks only while `sparks`, the bolts only
    /// while `bolts` (its wearer's parts).
    pub(crate) fn sets_of(
        def: &crate::blade_skin_file::BladeSkinDef,
        sparks: bool,
        bolts: bool,
    ) -> [Option<Self>; MOST_SETS] {
        let sparks = def
            .sparks
            .iter()
            .filter(|_| sparks)
            .flat_map(|sets| sets.sets())
            .map(Self::of);
        let bolts = def.bolts.iter().filter(|_| bolts).map(Self::of_bolts);
        let mut sets = sparks.chain(bolts);
        std::array::from_fn(|_| sets.next())
    }

    /// Bursts a second at `time` for `emitter`: `rate` lit, `ignite_rate` (when more) for
    /// the `ignite` seconds once lit, put out `after_rate` falling to none over `after`.
    fn bursts(&self, emitter: &Emitter, time: i64) -> f32 {
        let since = |at: i64| (time - at) as f32 / 1_000.0;
        if emitter.lit {
            let lighting = emitter
                .lit_at
                .filter(|lit| since(*lit) < self.ignite)
                .map_or(0.0, |_| self.ignite_rate);
            return self.rate.max(lighting);
        }
        emitter.out_at.map_or(0.0, |out| {
            let out = since(out);
            if out < self.after {
                self.after_rate * (1.0 - out / self.after)
            } else {
                0.0
            }
        })
    }
}

/// One blade's emitter: whether its blade is lit (and since when, once seen before),
/// when it was put out, when its next burst goes, how many it let go, and when it was
/// last told of.
#[derive(Clone, Copy, Debug, Default)]
struct Emitter {
    lit: bool,
    known: bool,
    lit_at: Option<i64>,
    out_at: Option<i64>,
    next: Option<i64>,
    /// The rate (bursts a second) the next burst's wait was drawn at.
    drawn_at: f32,
    bursts: u32,
    seen: i64,
    /// Bursts owed at once, a clash's ([`Sparks::kick`]).
    kicks: u32,
}

/// Where the sparks go, the game's shaders come from and their blends are known.
pub(crate) struct Target<'a> {
    pub(crate) particles: &'a mut Vec<Particle>,
    pub(crate) effects: &'a mut EffectLibrary,
    /// `None` in a test without one: the sparks fade their opacity.
    pub(crate) atlas: Option<&'a crate::ParticleAtlas>,
    pub(crate) now: Instant,
}

/// Every blade's spark emitter, by key.
#[derive(Default)]
pub(crate) struct Sparks {
    emitters: HashMap<u64, Emitter>,
}

impl Sparks {
    /// This frame, at `time` (milliseconds): emitter `key` is at `at`, its blade pointing
    /// `blade` and `lit` or not; let go the bursts `spec` has come due since the last frame,
    /// and say how many (a skin's crackle snaps with its bolts').
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn emit(
        &mut self,
        key: u64,
        spec: &SparkSpec,
        at: Vec3,
        blade: Vec3,
        lit: bool,
        time: i64,
        target: &mut Target<'_>,
    ) -> u32 {
        let mut emitter = self.emitters.get(&key).copied().unwrap_or_default();
        let gap = !emitter.known || time - emitter.seen > GAP_MILLIS || time < emitter.seen;
        emitter.seen = time;
        if emitter.lit && !lit {
            emitter.out_at = Some(time);
        }
        if lit && !emitter.lit && emitter.known {
            emitter.lit_at = Some(time);
        }
        if lit {
            emitter.out_at = None;
        }
        emitter.known = true;
        emitter.lit = lit;
        // A clash's bursts go at once.
        let kicked = std::mem::take(&mut emitter.kicks).min(MOST_A_FRAME);
        for _ in 0..kicked {
            emitter.bursts = emitter.bursts.wrapping_add(1);
            let seed =
                (key as u32).wrapping_mul(0x2545_f491) ^ emitter.bursts.wrapping_mul(0x9e37_79b9);
            burst(spec, at, axis(spec.aim, blade), seed, target);
        }
        let rate = spec.bursts(&emitter, time);
        if rate <= 0.0 {
            emitter.next = None;
            emitter.drawn_at = 0.0;
            self.emitters.insert(key, emitter);
            return kicked;
        }
        let interval = |bursts: u32| {
            // An exponential wait: bursts at random, `rate` a second on the whole.
            let draw = crate::effect_envelope::random_unit(
                (key as u32).wrapping_mul(0x9e37_79b9) ^ bursts.wrapping_mul(0x85eb_ca6b),
            );
            ((-draw.max(1e-3).ln() / rate) * 1_000.0) as i64
        };
        // A wait drawn at a slower rate (before the ignition), or due during a gap, is
        // drawn again from now.
        let slower = emitter.drawn_at * 1.5 < rate;
        if gap || slower || emitter.next.is_none_or(|next| next < time - GAP_MILLIS) {
            emitter.next = Some(time + interval(emitter.bursts));
            emitter.drawn_at = rate;
        }
        let mut fired = 0;
        while let Some(next) = emitter
            .next
            .filter(|next| *next <= time && fired < MOST_A_FRAME)
        {
            emitter.bursts = emitter.bursts.wrapping_add(1);
            let seed =
                (key as u32).wrapping_mul(0x2545_f491) ^ emitter.bursts.wrapping_mul(0x9e37_79b9);
            burst(spec, at, axis(spec.aim, blade), seed, target);
            fired += 1;
            emitter.next = Some(next + interval(emitter.bursts).max(1));
            emitter.drawn_at = rate;
        }
        self.emitters.insert(key, emitter);
        fired + kicked
    }

    /// Owe emitter `key` `bursts` at once, let go the next time it is told of: its blade
    /// clashed.
    pub(crate) fn kick(&mut self, key: u64, bursts: u32) {
        let emitter = self.emitters.entry(key).or_default();
        emitter.kicks = emitter.kicks.saturating_add(bursts);
    }

    /// Forget the emitters not told of for a while (their blade gone).
    pub(crate) fn forget(&mut self, time: i64) {
        self.emitters
            .retain(|_, emitter| time - emitter.seen <= FORGET_MILLIS);
    }
}

/// The axis of the sparks' cone for `aim`: the world's up at 0, `blade`'s way at 1, back
/// out of the hilt at −1, turning between (up where the two would cancel).
fn axis(aim: f32, blade: Vec3) -> Vec3 {
    let toward = blade.normalize_or_zero() * aim.signum();
    Vec3::Z
        .lerp(toward, aim.abs().min(1.0))
        .try_normalize()
        .unwrap_or(Vec3::Z)
}

/// One burst of `spec`'s sparks at `at`, round `axis`: one to `burst` of them.
fn burst(spec: &SparkSpec, at: Vec3, axis: Vec3, seed: u32, target: &mut Target<'_>) {
    // The cone's frame: two ways across the axis.
    let (across, over) = axis.any_orthonormal_pair();
    let random = |salt: u32| crate::effect_envelope::random_unit(seed.wrapping_add(salt));
    let count = 1 + (random(0) * spec.burst as f32) as u32;
    let additive = target.atlas.and_then(|atlas| atlas.blend_of(spec.shader))
        == Some(crate::particle_types::ParticleBlend::Add);
    let linear = sjk_effect::CurveFlags {
        linear: true,
        ..Default::default()
    };
    let envelope =
        |start, end| crate::effect_envelope::Envelope::from_values(start, end, 0.0, linear);
    for spark in 0..count.min(spec.burst.max(1)) {
        if target.particles.len() >= MAX_PARTICLES {
            return;
        }
        let salt = spark * 16;
        // A cone round the world's up, even over its area.
        let tilt = spec.spread.to_radians() * random(salt + 1).sqrt();
        let turn = random(salt + 2) * std::f32::consts::TAU;
        let way = (across * turn.cos() + over * turn.sin()) * tilt.sin() + axis * tilt.cos();
        let speed = spec.speed * (1.0 - spec.speed_jitter * random(salt + 3));
        let velocity = way * speed;
        let life = spec.life * (0.6 + 0.8 * random(salt + 4));
        let spark_seed = seed.wrapping_add(salt.wrapping_mul(7919));
        // A bolt from the emitter to its end; a spark streaking along its way, or an
        // ember: a round point, its streak none.
        let (shape, streak, length, velocity, gravity) = match spec.shape {
            Shape::Bolts {
                reach,
                reach_jitter,
                chaos,
                branch,
            } => {
                let reach = reach * (1.0 - reach_jitter * random(salt + 5));
                let bolt = PrimitiveShape::Electricity {
                    end: at + way * reach,
                    chaos,
                    tapered: true,
                    branched: branch,
                    grow: false,
                    trace_end: false,
                    depth_hack: false,
                };
                (bolt, None, None, Vec3::ZERO, 0.0)
            }
            Shape::Sparks if spec.length <= 0.0 => (
                PrimitiveShape::Billboard,
                None,
                None,
                velocity,
                spec.gravity,
            ),
            Shape::Sparks => (
                PrimitiveShape::Spark,
                Some(-velocity),
                Some(envelope(spec.length, spec.length * 0.5)),
                velocity,
                spec.gravity,
            ),
        };
        target.particles.push(Particle {
            motion: crate::particle_motion::Motion::new(
                at,
                velocity,
                Vec3::new(0.0, 0.0, -gravity),
                0.0,
                0.0,
                target.now,
            )
            .with_wander(spec.wander, spark_seed),
            spawned_at: target.now,
            delay: Duration::ZERO,
            lifetime: Duration::from_secs_f32(life.max(0.01)),
            size: envelope(spec.size, spec.size * 0.4),
            length,
            streak,
            trace_streak: false,
            normal: None,
            alpha: envelope(spec.opacity, 0.0),
            use_alpha: !additive,
            set_shader_time: false,
            rgb: spec.color.map(|channel| envelope(channel, channel)),
            seed: spark_seed,
            shader: target.effects.shader(spec.shader),
            physics: crate::particle_physics::State::new(
                target.effects.code_primitive_definition(),
                0,
                0,
                0.0,
                sjk_effect::PrimitiveFlags::default(),
            ),
            shape,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> SparkSpec {
        SparkSpec {
            shader: "gfx/misc/spark2",
            color: [1.0, 0.6, 0.3],
            opacity: 1.0,
            rate: 3.0,
            ignite: 1.0,
            ignite_rate: 15.0,
            after: 1.0,
            after_rate: 8.0,
            burst: 5,
            speed: 70.0,
            speed_jitter: 0.5,
            spread: 35.0,
            aim: 0.0,
            gravity: 400.0,
            life: 0.4,
            size: 0.6,
            length: 4.0,
            along: 0.0,
            wander: 0.0,
            clash: 0,
            shape: Shape::Sparks,
        }
    }

    /// Lit a while it crackles at its rate; lit again after being out it crackles far
    /// more for its ignition; put out it sparks on, then stops. Every spark goes upward
    /// within the cone and streaks along its way.
    /// Aimed along the blade, the sparks fly within the cone round the blade's way; aimed
    /// half, round the way between it and up; aimed back, out of the hilt.
    #[test]
    fn aimed_they_fly_along_the_blade() {
        let blade = Vec3::new(1.0, 0.0, -1.0).normalize();
        assert!(axis(1.0, blade).distance(blade) < 1e-5);
        assert!(axis(0.0, blade).distance(Vec3::Z) < 1e-5);
        assert!(axis(-1.0, blade).distance(-blade) < 1e-5);
        let half = axis(0.5, blade);
        assert!(half.angle_between(Vec3::Z) < half.angle_between(blade) + 1e-4 || half.z > 0.0);
        assert_eq!(axis(1.0, Vec3::ZERO), Vec3::Z, "no blade: up");
        let mut particles = Vec::new();
        let mut effects = EffectLibrary::default();
        let mut target = Target {
            particles: &mut particles,
            effects: &mut effects,
            atlas: None,
            now: Instant::now(),
        };
        let spec = SparkSpec { aim: 1.0, ..spec() };
        for seed in 0..40 {
            burst(&spec, Vec3::ZERO, axis(spec.aim, blade), seed, &mut target);
        }
        assert!(target.particles.len() > 40);
        for spark in target.particles.iter() {
            let way = spark.motion.sample().velocity.normalize();
            assert!(
                way.angle_between(blade).to_degrees() <= 35.0 + 1e-3,
                "{way}"
            );
        }
    }

    #[test]
    fn it_crackles_lit_more_lighting_and_fizzles_out() {
        let mut particles = Vec::new();
        let mut effects = EffectLibrary::default();
        let mut target = Target {
            particles: &mut particles,
            effects: &mut effects,
            atlas: None,
            now: Instant::now(),
        };
        let mut sparks = Sparks::default();
        let spec = spec();
        let mut time = 0;
        let mut run = |sparks: &mut Sparks, target: &mut Target<'_>, lit: bool, frames: i64| {
            let before = target.particles.len();
            let mut bursts = 0;
            for _ in 0..frames {
                time += 16;
                let had = target.particles.len();
                sparks.emit(1, &spec, Vec3::ZERO, Vec3::X, lit, time, target);
                bursts += usize::from(target.particles.len() > had);
            }
            (target.particles.len() - before, bursts)
        };
        // Ten seconds lit: about thirty bursts.
        let (_, steady) = run(&mut sparks, &mut target, true, 625);
        assert!(
            (18..=45).contains(&steady),
            "{steady} bursts in ten seconds"
        );
        // Put out: a few bursts in its second, then none.
        let (_, after) = run(&mut sparks, &mut target, false, 70);
        assert!((2..=14).contains(&after), "{after}");
        assert_eq!(run(&mut sparks, &mut target, false, 62).0, 0, "done");
        // Lit again: its ignition's second crackles far more than a steady second.
        let (_, lighting) = run(&mut sparks, &mut target, true, 62);
        assert!(lighting >= 6, "{lighting} bursts lighting");
        for spark in target.particles.iter() {
            let way = spark.motion.sample().velocity.normalize();
            assert!(way.z >= 35_f32.to_radians().cos() - 1e-4, "{way}");
            assert!(matches!(spark.shape, PrimitiveShape::Spark));
        }
    }
}
