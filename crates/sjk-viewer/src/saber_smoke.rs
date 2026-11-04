//! Smoke from a blade skin's emitter (`smoke` in its file,
//! [`crate::blade_skin_file::smoke`]): each blade keeps an [`Emitter`] that lets puffs of
//! the game's particles go where the emitter is, at the skin's rate while the blade is
//! lit and, once it is put out, at a rate falling to none over the skin's `after`
//! seconds. A puff is let go into the air, not tied to the hilt: it rises, drifts,
//! grows, turns and fades on its own, so a swing leaves its puffs strung out behind it
//! and the hilt put away leaves its last ones hanging. The puffs of a frame are spread
//! along the way the emitter came, so a fast swing leaves a line, not clumps; the stream
//! sways round as it is let go, so the puffs curl into wisps. A puff fades as its
//! shader's blend asks: an additive one (the game's wisp) its colour, else its opacity.
//! Only a skin with the section smokes; a full particle pool takes no more puffs.

use crate::effect_library::EffectLibrary;
use crate::particle_types::{MAX_PARTICLES, Particle, PrimitiveShape};
use glam::Vec3;
use std::collections::HashSet;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Most puffs one emitter lets go in a frame: a dense wisp at 200 a second, at 30
/// frames a second.
const MOST_A_FRAME: u32 = 8;
/// A frame further from the last than this (the blade not drawn meanwhile) lets no puffs
/// go for the time between: they would all come out at once.
const GAP_MILLIS: i64 = 250;

/// A skin's smoke, small and `Copy` for the blades to carry ([`crate::saber_skins::SkinColor`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SmokeSpec {
    pub(crate) shader: &'static str,
    pub(crate) color: [f32; 3],
    pub(crate) opacity: f32,
    pub(crate) rate: f32,
    pub(crate) after_rate: f32,
    pub(crate) after: f32,
    pub(crate) life: f32,
    pub(crate) size: f32,
    pub(crate) growth: f32,
    pub(crate) rise: f32,
    pub(crate) spread: f32,
    pub(crate) follow: f32,
    pub(crate) spin: f32,
    pub(crate) turn: f32,
    pub(crate) sway: f32,
    pub(crate) sway_rate: f32,
    pub(crate) along: f32,
}

impl SmokeSpec {
    /// The section read from a skin's file.
    pub(crate) fn of(smoke: &crate::blade_skin_file::smoke::Smoke) -> Self {
        Self {
            shader: intern(&smoke.shader),
            color: smoke.color,
            opacity: smoke.opacity,
            rate: smoke.rate,
            after_rate: smoke.after_rate,
            after: smoke.after,
            life: smoke.life,
            size: smoke.size,
            growth: smoke.growth,
            rise: smoke.rise,
            spread: smoke.spread,
            follow: smoke.follow,
            spin: smoke.spin,
            turn: smoke.turn,
            sway: smoke.sway,
            sway_rate: smoke.sway_rate,
            along: smoke.along,
        }
    }

    /// Puffs a second: while `lit`, else `out` seconds after the blade went out (`None`:
    /// long enough ago that it no longer smokes).
    fn rate(&self, lit: bool, out: Option<f32>) -> f32 {
        match out {
            _ if lit => self.rate,
            Some(out) if out < self.after => self.after_rate * (1.0 - out / self.after),
            _ => 0.0,
        }
    }
}

/// `shader` kept for the program's life, once per name: the skins' shader names,
/// read again on each load (and each edit), for [`SmokeSpec`] to be `Copy`.
pub(crate) fn intern(shader: &str) -> &'static str {
    static NAMES: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut names = NAMES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let names = names.get_or_insert_with(HashSet::new);
    if let Some(name) = names.get(shader) {
        return name;
    }
    let name: &'static str = Box::leak(shader.to_owned().into_boxed_str());
    names.insert(name);
    name
}

/// Where the particles go, the game's shaders come from and their blends are known, this
/// frame.
pub(crate) struct Sink<'a> {
    pub(crate) particles: &'a mut Vec<Particle>,
    pub(crate) effects: &'a mut EffectLibrary,
    /// `None` in a test without one: the puffs fade their opacity.
    pub(crate) atlas: Option<&'a crate::ParticleAtlas>,
    /// The smoke ribbons ([`crate::saber_ribbons`]), and the sparks' emitters
    /// ([`crate::saber_sparks`]).
    pub(crate) ribbons: &'a mut crate::saber_ribbons::Ribbons,
    pub(crate) sparks: &'a mut crate::saber_sparks::Sparks,
    /// Where blades clashed this frame (`impacts::Pool::clashes`): another blade, a wall.
    pub(crate) clashes: &'a [[f32; 3]],
    /// What a match's blades of a skin with a crackle are heard doing ([`Heard`]); none
    /// on the menus' stage, whose Saber Lab plays its own.
    pub(crate) heard: Option<&'a mut Heard>,
    pub(crate) now: Instant,
}

/// What a match's blades of skins with a crackle (`crackle` in their files) are heard
/// doing this frame, for `GameAudio::hear_sabers`: their bolts' snaps, and their lit
/// emitters' buzz.
#[derive(Debug, Default)]
pub(crate) struct Heard {
    pub(crate) snaps: Vec<Sound>,
    pub(crate) buzzes: Vec<Sound>,
}

/// A blade's crackle heard: where (its emitter), whose (an entity number), the skin
/// numbered `skin`, and how many times (a snap's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sound {
    pub(crate) at: [f32; 3],
    pub(crate) entity: u32,
    pub(crate) skin: u8,
    pub(crate) count: u32,
}

/// The most sounds of each kind kept a frame.
const MOST_HEARD: usize = 64;

/// How far (units) past its radius from a blade's line a clash is still the blade's.
const CLASH_REACH: f32 = 6.0;

/// One blade's emitter: the puffs owed (a share of one carried to the next frame), where
/// it was last frame and when, when the blade went out while it still smokes after, and
/// how many puffs it let go (each its own randomness).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Emitter {
    owed: f32,
    last: Option<(Vec3, i64)>,
    out_at: Option<i64>,
    lit: bool,
    puffs: u32,
}

impl Emitter {
    /// This frame, at `time` (milliseconds): the emitter is at `at`, the blade `lit` or
    /// not; let go the puffs `smoke` owes since the last frame. `seed` tells blades apart.
    pub(crate) fn emit(
        &mut self,
        smoke: &SmokeSpec,
        at: Vec3,
        lit: bool,
        time: i64,
        seed: u32,
        sink: &mut Sink<'_>,
    ) {
        let previous = self.last.replace((at, time));
        if self.lit && !lit {
            self.out_at = Some(time);
        }
        if lit {
            self.out_at = None;
        }
        self.lit = lit;
        let Some((from, then)) = previous else {
            return;
        };
        let millis = time - then;
        if millis <= 0 {
            return;
        }
        if millis > GAP_MILLIS {
            self.owed = 0.0;
            return;
        }
        let seconds = millis as f32 / 1_000.0;
        let out = self.out_at.map(|out| (time - out) as f32 / 1_000.0);
        if out.is_some_and(|out| out >= smoke.after) {
            self.out_at = None;
        }
        self.owed += smoke.rate(lit, out) * seconds;
        let count = (self.owed.floor() as u32).min(MOST_A_FRAME);
        self.owed = (self.owed - count as f32).min(1.0);
        let moving = (at - from) / seconds;
        for puff in 0..count {
            // Spread along the way the emitter came this frame.
            let share = (puff as f32 + 1.0) / count as f32;
            self.puffs = self.puffs.wrapping_add(1);
            let seed = seed
                .wrapping_mul(0x9e37_79b9)
                .wrapping_add(self.puffs.wrapping_mul(0x85eb_ca6b));
            // The sway's way at the moment this puff is let go.
            let moment = (then as f32 + millis as f32 * share) / 1_000.0;
            let let_go_at = Puff {
                at: from.lerp(at, share),
                moving,
                moment,
                seed,
            };
            let_go(smoke, let_go_at, sink);
        }
    }
}

/// One puff to let go: where, the emitter moving at `moving` units a second, at `moment`
/// (seconds, for the sway), with its own randomness.
struct Puff {
    at: Vec3,
    moving: Vec3,
    moment: f32,
    seed: u32,
}

/// Let `puff` of `smoke` go.
fn let_go(smoke: &SmokeSpec, puff: Puff, sink: &mut Sink<'_>) {
    if sink.particles.len() >= MAX_PARTICLES {
        return;
    }
    let Puff {
        at,
        moving,
        moment,
        seed,
    } = puff;
    let random = |salt: u32| crate::effect_envelope::random_unit(seed.wrapping_add(salt));
    let either = |salt: u32| random(salt) * 2.0 - 1.0;
    let drift = Vec3::new(either(1), either(2), either(3)) * smoke.spread;
    // The sway goes round, a little out of step across, so the stream wanders.
    let phase = moment * smoke.sway_rate * std::f32::consts::TAU;
    let sway = Vec3::new(phase.sin(), (phase * 0.73).cos(), 0.0) * smoke.sway;
    let velocity = moving * smoke.follow + drift + sway + Vec3::Z * smoke.rise;
    // An additive shader shows no opacity: it fades the colour instead.
    let additive = sink.atlas.and_then(|atlas| atlas.blend_of(smoke.shader))
        == Some(crate::particle_types::ParticleBlend::Add);
    let life = smoke.life * (0.75 + 0.5 * random(4));
    let linear = sjk_effect::CurveFlags {
        linear: true,
        ..Default::default()
    };
    let envelope =
        |start, end| crate::effect_envelope::Envelope::from_values(start, end, 0.0, linear);
    let spin = smoke.spin * if random(5) < 0.5 { -1.0 } else { 1.0 };
    sink.particles.push(Particle {
        motion: crate::particle_motion::Motion::new(
            at,
            velocity,
            Vec3::ZERO,
            either(6) * smoke.turn,
            spin,
            sink.now,
        ),
        spawned_at: sink.now,
        delay: Duration::ZERO,
        lifetime: Duration::from_secs_f32(life),
        size: envelope(smoke.size, smoke.growth),
        length: None,
        streak: None,
        trace_streak: false,
        normal: None,
        alpha: envelope(smoke.opacity, 0.0),
        use_alpha: !additive,
        set_shader_time: false,
        rgb: smoke.color.map(|channel| envelope(channel, channel)),
        seed,
        shader: sink.effects.shader(smoke.shader),
        physics: crate::particle_physics::State::new(
            sink.effects.code_primitive_definition(),
            0,
            0,
            0.0,
            sjk_effect::PrimitiveFlags::default(),
        ),
        shape: PrimitiveShape::Billboard,
    });
}

impl Sink<'_> {
    /// Let emitter `key`'s sparks of `spec` go at `at`, its blade pointing `blade` and
    /// `lit` or not, at `time`.
    pub(crate) fn spark(
        &mut self,
        key: u64,
        spec: &crate::saber_sparks::SparkSpec,
        at: Vec3,
        blade: Vec3,
        lit: bool,
        time: i64,
    ) -> u32 {
        let mut target = crate::saber_sparks::Target {
            particles: self.particles,
            effects: self.effects,
            atlas: self.atlas,
            now: self.now,
        };
        self.sparks
            .emit(key, spec, at, blade, lit, time, &mut target)
    }

    /// Whether a clash this frame was where `blade` is: within its radius and
    /// [`CLASH_REACH`] of its line.
    pub(crate) fn clashed(&self, blade: crate::saber::Blade) -> bool {
        let base = Vec3::from_array(blade.base);
        let way = Vec3::from_array(blade.direction) * blade.length;
        self.clashes.iter().any(|at| {
            let at = Vec3::from_array(*at);
            let along = ((at - base).dot(way) / way.length_squared().max(1e-6)).clamp(0.0, 1.0);
            (base + way * along).distance(at) <= blade.radius + CLASH_REACH
        })
    }

    /// Note a crackle `sound` heard: a snap, or a buzz (`buzz`).
    pub(crate) fn hear(&mut self, sound: Sound, buzz: bool) {
        let Some(heard) = self.heard.as_deref_mut() else {
            return;
        };
        let list = if buzz {
            &mut heard.buzzes
        } else {
            &mut heard.snaps
        };
        if list.len() < MOST_HEARD {
            list.push(sound);
        }
    }

    /// Let `bursts` of spark emitter `key`'s go at once (its blade clashed).
    pub(crate) fn kick(&mut self, key: u64, bursts: u32) {
        if bursts > 0 {
            self.sparks.kick(key, bursts);
        }
    }
}

impl crate::GpuState {
    /// Put the loaded skins' smoke shaders in the effect atlas, which holds only the
    /// shaders of the map's effects when it is made: each frame (an atlas made for a new
    /// map lacks them), at the cost of a lookup a smoking skin. A missing one is asked
    /// for once an atlas; the atlas is rebuilt on a worker with it, keeping its shaders
    /// ([`crate::particle_atlas::Pending`]), and the smoke shows once it is in.
    pub(crate) fn admit_smoke_shaders(&mut self) {
        let atlas = &self.particle_atlas;
        let missing: Vec<String> = self
            .blade_skins
            .iter()
            .flat_map(|(_, skin)| {
                let smoke = skin.def.smoke.as_ref().map(|smoke| smoke.shader.as_str());
                let sparks = skin
                    .def
                    .sparks
                    .iter()
                    .flat_map(|sets| sets.sets())
                    .map(|sparks| sparks.shader.as_str());
                let bolts = skin.def.bolts.as_ref().map(|bolts| bolts.shader.as_str());
                smoke.into_iter().chain(sparks).chain(bolts)
            })
            .map(str::to_ascii_lowercase)
            .filter(|name| !atlas.animations.contains_key(name) && !atlas.admitted.contains(name))
            .collect();
        if missing.is_empty() {
            return;
        }
        self.particle_atlas.admitted.extend(missing);
        if let Err(error) = self.refresh_effect_atlas() {
            crate::log::progress(format_args!("blade skin smoke: {error}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> SmokeSpec {
        SmokeSpec {
            shader: "gfx/effects/alpha_smoke2",
            color: [0.5; 3],
            opacity: 0.5,
            rate: 20.0,
            after_rate: 10.0,
            after: 2.0,
            life: 1.5,
            size: 2.0,
            growth: 8.0,
            rise: 10.0,
            spread: 2.0,
            follow: 0.3,
            spin: 1.0,
            turn: 180.0,
            sway: 3.0,
            sway_rate: 0.5,
            along: 0.0,
        }
    }

    /// While lit the emitter lets puffs go at the skin's rate; put out, it goes on at
    /// the falling rate, and stops after `after` seconds; a long gap lets none go.
    #[test]
    fn it_smokes_while_lit_and_for_a_while_after() {
        let mut particles = Vec::new();
        let mut effects = EffectLibrary::default();
        let mut ribbons = crate::saber_ribbons::Ribbons::default();
        let mut sparks = crate::saber_sparks::Sparks::default();
        let mut sink = Sink {
            particles: &mut particles,
            effects: &mut effects,
            atlas: None,
            ribbons: &mut ribbons,
            sparks: &mut sparks,
            clashes: &[],
            heard: None,
            now: Instant::now(),
        };
        let mut emitter = Emitter::default();
        let smoke = spec();
        let mut time = 0;
        let run =
            |emitter: &mut Emitter, time: &mut i64, lit: bool, frames: i64, sink: &mut Sink<'_>| {
                let before = sink.particles.len();
                for _ in 0..frames {
                    *time += 16;
                    let at = Vec3::new(*time as f32 * 0.1, 0.0, 0.0);
                    emitter.emit(&smoke, at, lit, *time, 7, sink);
                }
                sink.particles.len() - before
            };
        // A second lit: about twenty puffs.
        let lit = run(&mut emitter, &mut time, true, 62, &mut sink);
        assert!((18..=21).contains(&lit), "{lit}");
        // The first second out: about seven and a half (ten a second falling to five).
        let first = run(&mut emitter, &mut time, false, 62, &mut sink);
        assert!((6..=9).contains(&first), "{first}");
        let second = run(&mut emitter, &mut time, false, 62, &mut sink);
        assert!(second < first && second > 0, "{second}");
        // Two seconds out: done.
        assert_eq!(run(&mut emitter, &mut time, false, 62, &mut sink), 0);
        // Lit again after a long gap: no burst for the time between.
        time += 5_000;
        assert_eq!(run(&mut emitter, &mut time, true, 1, &mut sink), 0);
        let puff = &sink.particles[0];
        assert_eq!(&*puff.shader, "gfx/effects/alpha_smoke2");
        assert!(matches!(puff.shape, PrimitiveShape::Billboard));
    }

    #[test]
    fn a_name_is_kept_once() {
        assert!(std::ptr::eq(intern("gfx/a"), intern("gfx/a")));
    }
}
