//! Blossoms a swung blade sheds (a signature's `// @petals` line, `docs/unlockables.md`,
//! "Falling blossoms"; first the Varykino Blossom, Kushina's idea): where the Spectral leaves
//! afterimages of its swing, this blade leaves petals that flutter down behind it, settle on
//! the floor and fade. They are particles in the world, not part of the blade's quad: each
//! blade keeps where its tip was ([`PetalEmitter`]), every unit its tip travels in its
//! wearer's frame (a swing, not a turn of the view or a run) queues a share of a petal along
//! the blade ([`crate::saber_trail::StateSlab::petals`]), and [`frame`] turns the queue into
//! particles with the bundled petal pictures. Nothing is allocated per frame. The Thousand
//! Cranes (25/10/2026) releases paper cranes the same way (`"kind": "crane"`): they rise and
//! fly off instead of settling, each in a colour of paper. The Hearth (25/10/2026) sheds ash
//! (`"kind": "ash"`): grey flakes, some still glowing at their edge, that drift down and settle.
//! The Sun's Swing embers (26/10/2026, one of its extras, `blade_extras.rs`) are sparks
//! (`"kind": "ember"`) flung off along the swing, rising as hot air does, cooling from white
//! gold to deep red and going out in the air. The Moon (26/10/2026, Creyon's) sheds moondust
//! (`"kind": "moon"`): glittering specks in silver, lilac and purple that float up and fade, as
//! its blade swings and as its wearer moves (`moving`); on a chroma the coloured dust takes its
//! wearer's saber colour (`PetalSpec::turn`).

use glam::Vec3;
use serde::Deserialize;
use std::time::{Duration, Instant};

/// What a swing sheds: blossom petals (the default) or paper cranes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PetalKind {
    /// Petals that flutter down and settle on the floor.
    #[default]
    Blossom,
    /// Paper cranes that rise and fly off through the world, each tinted a colour of paper.
    Crane,
    /// Flakes of ash that drift down and settle, grey or still glowing at their edge.
    Ash,
    /// Sparks flung off the swing that rise, cool from white gold to red and go out in the
    /// air, glowing (drawn added to what is behind them).
    Ember,
    /// Moondust that floats up and fades: specks of added light in the Moon's silver, lilac and
    /// purple.
    Moon,
    /// An angel's feathers, turning as they sink slowly and settle.
    Feather,
}

/// A skin's falling blossoms, as its signature's `@petals` line gives them.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PetalSpec {
    /// The signature's own part (its `// @part` line, from 0) that switches them; not read
    /// for an extra's (`blade_extras.rs`), which its own switch turns on.
    #[serde(default)]
    pub(crate) part: u8,
    /// What they are.
    #[serde(default)]
    pub(crate) kind: PetalKind,
    /// Petals for each unit the tip travels.
    pub(crate) per_unit: f32,
    /// How long one lasts (seconds), and its size (units).
    pub(crate) life: f32,
    pub(crate) size: f32,
    /// How fast it sinks (units a second) and how far it may drift sideways (units a second).
    pub(crate) fall: f32,
    pub(crate) drift: f32,
    /// How fast it climbs (units a second): a crane's wings carry it up.
    #[serde(default)]
    pub(crate) rise: f32,
    /// The share of the tip's speed it leaves with ([`DEFAULT_FLING`] when absent): an
    /// ember is flung off far harder than a petal is let go.
    #[serde(default = "default_fling")]
    pub(crate) fling: f32,
    /// Petals for each unit the tip travels through the world beyond its swing (the wearer
    /// running, jumping, riding): the Moon sheds moondust as its wearer moves. 0 for none.
    #[serde(default)]
    pub(crate) moving: f32,
    /// Turns from the skin's own hue to its wearer's saber colour (a chroma's
    /// [`crate::saber_skins::Chroma::turn`]), set where the blade sheds them, never in the
    /// signature's line: the Moon's coloured moondust takes the wearer's colour (Creyon's
    /// wish), white dust stays white.
    #[serde(skip)]
    pub(crate) turn: f32,
}

/// A petal's share of the tip's speed (the swing's breath).
pub(crate) const DEFAULT_FLING: f32 = 0.025;

fn default_fling() -> f32 {
    DEFAULT_FLING
}

impl PetalSpec {
    /// Whether its numbers are in reason (checked with the signature's other lines).
    #[cfg(test)]
    pub(crate) fn valid(&self) -> bool {
        self.part < 6
            && (0.0..=2.0).contains(&self.per_unit)
            && (0.2..=6.0).contains(&self.life)
            && (0.2..=6.0).contains(&self.size)
            && (0.0..=200.0).contains(&self.fall)
            && (0.0..=100.0).contains(&self.drift)
            && (0.0..=200.0).contains(&self.rise)
            && (0.0..=0.5).contains(&self.fling)
            && (0.0..=2.0).contains(&self.moving)
    }
}

/// The most petals one blade sheds in a frame, and the most waiting in the queue.
const MOST_A_FRAME: u32 = 6;
pub(crate) const MOST_QUEUED: usize = 256;

/// A petal to spawn: where, its first velocity, its seed and its skin's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PetalSpawn {
    pub(crate) origin: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) seed: u32,
    pub(crate) spec: PetalSpec,
}

/// One blade's tip as last seen, in its wearer's frame and in the world.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PetalEmitter {
    last: Option<(Vec3, Vec3)>,
    last_millis: i64,
    /// The share of a petal owed from the last frames.
    carry: f32,
}

/// 0 to 1 from `seed` and `salt`.
fn unit(seed: u32, salt: u32) -> f32 {
    crate::effect_envelope::random_unit(
        seed.wrapping_mul(0x9e37_79b9) ^ salt.wrapping_mul(0x85eb_ca6b),
    )
}

impl PetalEmitter {
    /// The blade from `base` along `direction` for `length` units (in the world), its tip at
    /// `body_tip` in its wearer's frame, drawn at `now`: queue the petals its swing sheds. A
    /// jump in time (a new map, a demo seek) or a pause forgets where it was.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn shed(
        &mut self,
        base: Vec3,
        direction: Vec3,
        length: f32,
        body_tip: Vec3,
        now: i64,
        key: u32,
        spec: PetalSpec,
        queue: &mut Vec<PetalSpawn>,
    ) {
        let tip = base + direction * length;
        let since = now - self.last_millis;
        let last = self.last.replace((body_tip, tip));
        self.last_millis = now;
        let Some((last_body, last_tip)) = last.filter(|_| (1..=200).contains(&since)) else {
            self.carry = 0.0;
            return;
        };
        let swung = body_tip.distance(last_body);
        // What the tip travelled through the world beyond its swing: the wearer moving.
        let moved = (tip.distance(last_tip) - swung).max(0.0);
        self.carry += swung * spec.per_unit + moved * spec.moving;
        let count = (self.carry.floor() as u32).min(MOST_A_FRAME);
        self.carry -= self.carry.floor();
        // A share of the tip's speed carries them on (a little: the swing's breath; an
        // ember far more).
        let carried = (tip - last_tip) / (since as f32 * 0.001) * spec.fling;
        let most = 40.0_f32.max(spec.fling * 1_600.0);
        for i in 0..count {
            if queue.len() >= MOST_QUEUED {
                return;
            }
            let seed = key ^ (now as u32).wrapping_mul(2_654_435_761) ^ i.wrapping_mul(40_503);
            // Along the blade's outer two thirds, a little to either side.
            let along = 0.35 + 0.65 * unit(seed, 1);
            let side = Vec3::new(
                unit(seed, 2) - 0.5,
                unit(seed, 3) - 0.5,
                unit(seed, 4) - 0.5,
            );
            let drift = Vec3::new(unit(seed, 5) - 0.5, unit(seed, 6) - 0.5, 0.0) * 2.0 * spec.drift;
            queue.push(PetalSpawn {
                origin: base + direction * (length * along) + side * 2.0,
                velocity: carried.clamp_length_max(most)
                    + drift
                    + Vec3::Z * (spec.rise - spec.fall) * 0.5,
                seed,
                spec,
            });
        }
    }
}

/// The petal pictures, at their game paths, and their shader (made by
/// `scripts/petal_assets.py`, but the shader).
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/petals/", $name))
    };
}
/// Their shaders, which the frame's effect materials must load (`effect_assets::CODE_SHADERS`):
/// the blossoms' three tints, then the crane flying right and flying left, then grey ash and
/// ash still glowing, then the ember, then the Moon's moondust.
pub(crate) const SHADERS: [&str; 11] = [
    "gfx/sjk/petals/blossom_pink",
    "gfx/sjk/petals/blossom_pale",
    "gfx/sjk/petals/blossom_white",
    "gfx/sjk/petals/crane",
    "gfx/sjk/petals/crane_left",
    "gfx/sjk/petals/ash_grey",
    "gfx/sjk/petals/ash_ember",
    "gfx/sjk/petals/ember",
    "gfx/sjk/petals/moon_dust",
    "gfx/sjk/petals/feather",
    "gfx/sjk/petals/puff",
];
const BLOSSOMS: usize = 3;
const CRANES: usize = 2;
const ASHES: usize = 2;
/// An ember's colour as it leaves the blade and as it goes out.
const EMBER_HOT: [f32; 3] = [1.0, 0.92, 0.62];
const EMBER_COOL: [f32; 3] = [0.95, 0.22, 0.04];
/// The Moon's moondust (`moon.glsl`'s `MO_WHITE`, `MO_SILVER`, `MO_LILAC` and `MO_PURPLE`).
const MOONLIGHT: [[f32; 3]; 4] = [
    [0.94, 0.97, 1.0],
    [0.78, 0.85, 0.98],
    [0.8, 0.62, 1.0],
    [0.62, 0.3, 1.0],
];

/// A speck of moondust's colour for `seed`, turned `turn` round to its wearer's saber colour
/// (a chroma's turn; 0 keeps the Moon's own).
fn moonlight(seed: u32, turn: f32) -> [f32; 3] {
    crate::saber_skins::turn_hue(
        MOONLIGHT[((seed >> 3) % MOONLIGHT.len() as u32) as usize],
        turn,
    )
}
/// The cranes' paper, as the Thousand Cranes' flock wears it (`tc_paper`): vermilion, indigo,
/// sakura, jade, gold, sky, plum and cream.
const PAPER: [[f32; 3]; 8] = [
    [0.9, 0.24, 0.18],
    [0.22, 0.3, 0.62],
    [0.98, 0.66, 0.74],
    [0.3, 0.66, 0.5],
    [0.96, 0.76, 0.3],
    [0.46, 0.68, 0.9],
    [0.6, 0.32, 0.56],
    [0.97, 0.93, 0.84],
];
const FILES: [(&str, &[u8]); 12] = [
    ("shaders/sjk_petals.shader", bundled!("petals.shader")),
    (
        "gfx/sjk/petals/blossom_pink.png",
        bundled!("blossom_pink.png"),
    ),
    (
        "gfx/sjk/petals/blossom_pale.png",
        bundled!("blossom_pale.png"),
    ),
    (
        "gfx/sjk/petals/blossom_white.png",
        bundled!("blossom_white.png"),
    ),
    ("gfx/sjk/petals/crane.png", bundled!("crane.png")),
    ("gfx/sjk/petals/crane_left.png", bundled!("crane_left.png")),
    ("gfx/sjk/petals/ash_grey.png", bundled!("ash_grey.png")),
    ("gfx/sjk/petals/ash_ember.png", bundled!("ash_ember.png")),
    ("gfx/sjk/petals/ember.png", bundled!("ember.png")),
    ("gfx/sjk/petals/moon_dust.png", bundled!("moon_dust.png")),
    ("gfx/sjk/petals/feather.png", bundled!("feather.png")),
    ("gfx/sjk/petals/puff.png", bundled!("puff.png")),
];

/// Mount the bundled petal files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK petals", FILES.iter().copied())?;
    Ok(())
}

/// Turn the queued petals into particles: each a picture of a blossom petal facing the
/// camera, turning as it sinks, settling where it lands (it stops on the floor) and fading
/// over its last third. A crane keeps nearly upright as it climbs, flies on through the world
/// rather than landing, and takes a colour of paper. An ember flies on too, shrinking and
/// cooling as it goes; moondust floats up like a crane, turning, in a colour of moonlight.
pub(crate) fn frame(gpu: &mut crate::GpuState, now: Instant) {
    use crate::particle_types::{Particle, PrimitiveShape};
    let queue = gpu.saber_states.petals();
    if queue.is_empty() {
        return;
    }
    let constant = |value: f32| {
        crate::effect_envelope::Envelope::from_values(value, value, 0.0, Default::default())
    };
    for spawn in queue.drain(..) {
        if !crate::particle_room::effect_fits(gpu.particles.len()) {
            continue;
        }
        let seed = spawn.seed;
        let spec = spawn.spec;
        let size = spec.size * (0.75 + 0.5 * unit(seed, 7));
        let crane = spec.kind == PetalKind::Crane;
        let ember = spec.kind == PetalKind::Ember;
        let moon = spec.kind == PetalKind::Moon;
        // Cranes, embers and moondust fly on through the world; petals and ash settle.
        let flies = crane || ember || moon;
        let (angle, spin) = if crane {
            ((unit(seed, 9) - 0.5) * 30.0, (unit(seed, 8) - 0.5) * 12.0)
        } else if moon {
            (unit(seed, 9) * 360.0, (unit(seed, 8) - 0.5) * 120.0)
        } else {
            (unit(seed, 9) * 360.0, (unit(seed, 8) - 0.5) * 50.0)
        };
        let shader = match spec.kind {
            PetalKind::Blossom => SHADERS[(seed % BLOSSOMS as u32) as usize],
            PetalKind::Crane => SHADERS[BLOSSOMS + (seed % CRANES as u32) as usize],
            // One flake in three still glows.
            PetalKind::Ash => SHADERS[BLOSSOMS + CRANES + usize::from(seed % 3 == 0)],
            PetalKind::Ember => SHADERS[BLOSSOMS + CRANES + ASHES],
            PetalKind::Moon => SHADERS[BLOSSOMS + CRANES + ASHES + 1],
            // About every other one a small cloud comes away with the feathers.
            PetalKind::Feather => {
                SHADERS[BLOSSOMS + CRANES + ASHES + 2 + usize::from(seed % 2 == 0)]
            }
        };
        let [r, g, b] = if crane {
            PAPER[(seed % PAPER.len() as u32) as usize]
        } else if moon {
            moonlight(seed, spec.turn)
        } else {
            [1.0; 3]
        };
        let linear = sjk_effect::CurveFlags {
            linear: true,
            ..Default::default()
        };
        let rgb = if ember {
            std::array::from_fn(|channel| {
                crate::effect_envelope::Envelope::from_values(
                    EMBER_HOT[channel],
                    EMBER_COOL[channel],
                    0.0,
                    linear,
                )
            })
        } else {
            [constant(r), constant(g), constant(b)]
        };
        gpu.particles.push(Particle {
            motion: crate::particle_motion::Motion::new(
                spawn.origin,
                spawn.velocity,
                // Sinking slowly, as a petal does in air (a crane climbing).
                Vec3::new(0.0, 0.0, (spec.rise - spec.fall) * 0.6),
                angle,
                spin,
                now,
            ),
            spawned_at: now,
            delay: Duration::ZERO,
            lifetime: Duration::from_secs_f32(spec.life * (0.8 + 0.4 * unit(seed, 10))),
            size: crate::effect_envelope::Envelope::from_values(
                size,
                match spec.kind {
                    PetalKind::Crane => size,
                    PetalKind::Ember => size * 0.35,
                    _ => size * 0.8,
                },
                0.0,
                linear,
            ),
            length: None,
            streak: None,
            trace_streak: false,
            normal: None,
            alpha: crate::effect_envelope::Envelope::from_values(
                1.0,
                0.0,
                65.0,
                sjk_effect::CurveFlags {
                    modifier: sjk_effect::CurveModifier::NonLinear,
                    ..Default::default()
                },
            ),
            use_alpha: true,
            set_shader_time: false,
            rgb,
            seed,
            shader: gpu.effects.shader(shader),
            // Falling onto the floor, it stays there (no bounce) until it fades; a crane, an
            // ember or moondust flies on through.
            physics: crate::particle_physics::State::new(
                gpu.effects.code_primitive_definition(),
                0,
                0,
                0.0,
                sjk_effect::PrimitiveFlags {
                    apply_physics: !flies,
                    ..Default::default()
                },
            ),
            shape: PrimitiveShape::Billboard,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: PetalSpec = PetalSpec {
        part: 3,
        kind: PetalKind::Blossom,
        per_unit: 0.2,
        life: 2.5,
        size: 1.3,
        fall: 40.0,
        drift: 10.0,
        rise: 0.0,
        fling: DEFAULT_FLING,
        moving: 0.0,
        turn: 0.0,
    };

    #[test]
    fn a_swing_sheds_petals_and_a_still_blade_none() {
        let mut emitter = PetalEmitter::default();
        let mut queue = Vec::new();
        let base = Vec3::ZERO;
        let up = Vec3::Z;
        // Held still for a second: nothing.
        for step in 0..60 {
            emitter.shed(
                base,
                up,
                40.0,
                Vec3::new(0.0, 0.0, 40.0),
                1_000 + step * 16,
                7,
                SPEC,
                &mut queue,
            );
        }
        assert!(queue.is_empty());
        // Swung 20 units a frame for ten frames: 200 units, 40 petals at 0.2 a unit, but
        // at most six a frame.
        for step in 0..10 {
            let tip = Vec3::new(20.0 * (step + 1) as f32, 0.0, 40.0);
            emitter.shed(base, up, 40.0, tip, 2_000 + step * 16, 7, SPEC, &mut queue);
        }
        assert_eq!(queue.len(), 40);
        // Each on the blade's outer two thirds, near it.
        for petal in &queue {
            assert!(
                petal.origin.z >= 40.0 * 0.35 - 1.5 && petal.origin.z <= 41.5,
                "{petal:?}"
            );
            assert!(petal.velocity.z < 0.0);
        }
    }

    #[test]
    fn cranes_rise_where_blossoms_sink() {
        let crane = PetalSpec {
            kind: PetalKind::Crane,
            fall: 0.0,
            rise: 40.0,
            drift: 0.0,
            ..SPEC
        };
        let mut emitter = PetalEmitter::default();
        let mut queue = Vec::new();
        for step in 0..10 {
            let tip = Vec3::new(20.0 * step as f32, 0.0, 40.0);
            emitter.shed(
                Vec3::ZERO,
                Vec3::Z,
                40.0,
                tip,
                2_000 + step * 16,
                7,
                crane,
                &mut queue,
            );
        }
        assert!(!queue.is_empty());
        assert!(queue.iter().all(|spawn| spawn.velocity.z > 0.0));
        let line = r#"{"part":2,"kind":"ash","per_unit":0.04,"life":3.2,"size":0.9,"fall":10.0,"drift":8.0}"#;
        let ash: PetalSpec = serde_json::from_str(line).unwrap();
        assert!(ash.valid() && ash.kind == PetalKind::Ash);
        // A line without a kind or a rise is a blossom's, as the Varykino Blossom's is.
        let line = r#"{"part":3,"per_unit":0.07,"life":2.6,"size":1.6,"fall":38.0,"drift":14.0}"#;
        let spec: PetalSpec = serde_json::from_str(line).unwrap();
        assert_eq!((spec.kind, spec.rise), (PetalKind::Blossom, 0.0));
        assert_eq!(spec.fling, DEFAULT_FLING);
        // The Moon's moondust rises as cranes do.
        let moon = crate::blade_signatures::of("saber_moon")
            .and_then(|signature| signature.petals())
            .expect("the Moon sheds moondust");
        assert!(moon.valid() && moon.kind == PetalKind::Moon && moon.rise > moon.fall);
    }

    /// The Moon's purple moondust takes its wearer's colour on a chroma (here a third of a turn
    /// round), its white stays white, and no signature line may set the turn itself.
    #[test]
    fn moondust_takes_its_wearers_colour() {
        let purple = (0..64)
            .find(|&seed| moonlight(seed, 0.0) == MOONLIGHT[3])
            .unwrap();
        let hue = |rgb| crate::saber_skins::hue_of(rgb).unwrap();
        let moved = (hue(moonlight(purple, 1.0 / 3.0)) - hue(MOONLIGHT[3])).rem_euclid(1.0);
        assert!((moved - 1.0 / 3.0).abs() < 0.01, "{moved}");
        let white = (0..64)
            .find(|&seed| moonlight(seed, 0.0) == MOONLIGHT[0])
            .unwrap();
        let turned = moonlight(white, 1.0 / 3.0);
        assert!(
            turned
                .iter()
                .zip(MOONLIGHT[0])
                .all(|(a, b)| (a - b).abs() < 0.08),
            "{turned:?}"
        );
        let line = r#"{"part":1,"kind":"moon","per_unit":0.3,"life":2.6,"size":1.1,"fall":0.0,"drift":14.0,"turn":0.5}"#;
        assert!(serde_json::from_str::<PetalSpec>(line).is_err());
    }

    /// A blade held still while its wearer runs sheds nothing, unless its skin sheds as it
    /// moves (the Moon's moondust).
    #[test]
    fn moving_sheds_only_for_a_skin_that_asks() {
        let run = |spec: PetalSpec| {
            let mut emitter = PetalEmitter::default();
            let mut queue = Vec::new();
            for step in 0..20 {
                // The body carries the blade 10 units a frame; the tip keeps its place on it.
                let base = Vec3::new(10.0 * step as f32, 0.0, 0.0);
                emitter.shed(
                    base,
                    Vec3::Z,
                    40.0,
                    Vec3::new(0.0, 0.0, 40.0),
                    1_000 + step * 16,
                    3,
                    spec,
                    &mut queue,
                );
            }
            queue.len()
        };
        assert_eq!(run(SPEC), 0);
        // 190 units moved at 0.05 a unit: nine.
        assert_eq!(
            run(PetalSpec {
                moving: 0.05,
                ..SPEC
            }),
            9
        );
    }

    #[test]
    fn embers_are_flung_off_the_swing_and_rise() {
        let ember = PetalSpec {
            kind: PetalKind::Ember,
            fall: 0.0,
            rise: 36.0,
            drift: 0.0,
            fling: 0.16,
            ..SPEC
        };
        let mut emitter = PetalEmitter::default();
        let mut queue = Vec::new();
        // The blade sweeps along x at 20 units a 16 ms frame (1250 units a second), in the
        // world and in its wearer's frame.
        for step in 0..10 {
            let tip = Vec3::new(20.0 * step as f32, 0.0, 40.0);
            emitter.shed(
                Vec3::new(tip.x, 0.0, 0.0),
                Vec3::Z,
                40.0,
                tip,
                2_000 + step * 16,
                7,
                ember,
                &mut queue,
            );
        }
        assert!(!queue.is_empty());
        for spawn in &queue {
            // Flung along the swing far faster than a petal's breath (0.025 of it, 31), and
            // rising.
            assert!(spawn.velocity.x > 150.0, "{spawn:?}");
            assert!(spawn.velocity.z > 0.0, "{spawn:?}");
        }
        assert!(ember.valid());
        assert!(
            !PetalSpec {
                fling: 0.9,
                ..ember
            }
            .valid()
        );
    }

    #[test]
    fn a_jump_in_time_sheds_nothing() {
        let mut emitter = PetalEmitter::default();
        let mut queue = Vec::new();
        emitter.shed(
            Vec3::ZERO,
            Vec3::Z,
            40.0,
            Vec3::ZERO,
            1_000,
            1,
            SPEC,
            &mut queue,
        );
        // A second later and far away: a new map, a seek or a pause.
        emitter.shed(
            Vec3::ZERO,
            Vec3::Z,
            40.0,
            Vec3::splat(500.0),
            2_000,
            1,
            SPEC,
            &mut queue,
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn the_queue_is_bounded() {
        let mut emitter = PetalEmitter::default();
        let mut queue = Vec::new();
        let fast = PetalSpec {
            per_unit: 2.0,
            ..SPEC
        };
        for step in 0..200 {
            let tip = Vec3::new(30.0 * step as f32, 0.0, 0.0);
            emitter.shed(
                Vec3::ZERO,
                Vec3::Z,
                40.0,
                tip,
                1_000 + step * 16,
                1,
                fast,
                &mut queue,
            );
        }
        assert_eq!(queue.len(), MOST_QUEUED);
    }

    #[test]
    fn their_shaders_are_loaded_with_the_effects() {
        // Missing from the list, the particles drew the stand-in sprite (yellow dots).
        for shader in SHADERS {
            assert!(
                crate::effect_assets::CODE_SHADERS.contains(&shader),
                "{shader}"
            );
        }
    }

    #[test]
    fn the_pictures_are_bundled() {
        for (path, bytes) in FILES {
            assert!(!bytes.is_empty(), "{path}");
        }
        let shader = std::str::from_utf8(FILES[0].1).unwrap();
        for name in SHADERS {
            assert!(shader.contains(name), "{name}");
        }
    }
}
