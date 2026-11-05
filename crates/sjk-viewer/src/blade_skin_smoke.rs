//! The blade-skin file's eighth set (`docs/unlockables.md`, "Blade-skin files"), added
//! 18/10/2026: smoke from the emitter. Not drawn by `saber.wgsl` with the blade: puffs
//! of the game's particles ([`crate::saber_smoke`]) left in the air where the emitter
//! passes, rising, spreading and fading on their own, so a swing leaves them trailing
//! behind it and the blade put out goes on smoking for a while; or ribbons
//! ([`crate::saber_ribbons`]), strands drawn through points let go one after another,
//! each drifting on its own, for wisps.
//!
//! A client of the seventh set refuses a field it does not know in its additions files,
//! so it lives in a sixth one (`<id>.bladeextra6`, [`SMOKE_ADDITIONS`]), laid over the
//! others: an older client draws the skin without it. As before, a section left out
//! draws nothing and, when present, every field of it is required.

use super::{Rgb, colour, game_path, within};
use serde::Deserialize;

/// The sixth additions file's extension (the eighth set's): `<id>.bladeextra6`.
pub(crate) const SMOKE_ADDITIONS: &str = ".bladeextra6";

/// Smoke from the emitter, `along` the blade (0: the emitter, 1: the tip): `rate` puffs a
/// second while the blade is lit, then `after_rate` a second falling to none over `after`
/// seconds once it is put out. A puff (the game shader `shader`, tinted `color`) is
/// `size` units across when let go and `growth` when gone `life` seconds later, fading
/// from `opacity`; it rises `rise` units a second, drifts `spread` units a second
/// whichever way, keeps `follow` of the emitter's own speed (a swing drags it along), is
/// turned up to `turn` degrees either way when let go (180: any way; little keeps a
/// wisp's shape upright) and turns `spin` (the game's rotation units, either way at
/// random). The stream sways `sway` units a second to the side, its way going round
/// `sway_rate` times a second, so the puffs let go one after another curl into a wisp.
/// Small faint puffs let go many a second (`rate` up to 200) lie in a chain and read as a
/// thin wisp. An additive shader fades the puff's colour, an alpha-blended one its
/// opacity, as its blend asks.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Smoke {
    pub(crate) shader: String,
    pub(crate) color: Rgb,
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

/// Smoke as ribbons from the emitter, `along` the blade: a strand through points let go
/// `rate` a second while the blade is lit and for `after` seconds once it is put out,
/// staying strong a while then thinning away. Lit, it is as strong as `idle` held still
/// and `swinging` going `swing_speed` units a second or faster (the emitter's speed eased
/// over a quarter of a second), in between as fast as it goes, and at full strength for
/// `ignite` seconds once lit: 0 swinging smokes only standing, lighting and put out; 1
/// smokes the whole time. A point lasts `life` seconds: the strand is `width` units across there when let go
/// and `growth` when it is gone, in `color` at `opacity` fading to nothing; it rises `rise`
/// units a second, keeps `follow` of the emitter's own speed (dying away as it rises), the
/// stream swaying `sway` units a second to the side round `sway_rate` times a second, and
/// it curls `curl` units a second as it goes. The strand is torn by noise `detail` cycles
/// a unit along it, and fades over `softness` of its half-width to its edges.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ribbon {
    pub(crate) color: Rgb,
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

impl Ribbon {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        colour("ribbon.color", self.color, 1.0)?;
        within("ribbon.opacity", self.opacity, 0.0, 1.0)?;
        within("ribbon.rate", self.rate, 5.0, 120.0)?;
        within("ribbon.after", self.after, 0.0, 20.0)?;
        within("ribbon.life", self.life, 0.2, 10.0)?;
        within("ribbon.width", self.width, 0.05, 16.0)?;
        within("ribbon.growth", self.growth, 0.05, 48.0)?;
        within("ribbon.rise", self.rise, -50.0, 100.0)?;
        within("ribbon.follow", self.follow, 0.0, 1.0)?;
        within("ribbon.sway", self.sway, 0.0, 50.0)?;
        within("ribbon.sway_rate", self.sway_rate, 0.0, 10.0)?;
        within("ribbon.curl", self.curl, 0.0, 50.0)?;
        within("ribbon.detail", self.detail, 0.0, 4.0)?;
        within("ribbon.softness", self.softness, 0.05, 1.0)?;
        within("ribbon.idle", self.idle, 0.0, 1.0)?;
        within("ribbon.swinging", self.swinging, 0.0, 1.0)?;
        within("ribbon.swing_speed", self.swing_speed, 10.0, 1_000.0)?;
        within("ribbon.ignite", self.ignite, 0.0, 10.0)?;
        within("ribbon.along", self.along, 0.0, 1.0)
    }
}

/// Sparks crackling out of the emitter, `along` the blade: bursts at random moments,
/// `rate` a second while lit, `ignite_rate` (when more) for `ignite` seconds once lit, put
/// out `after_rate` falling to none over `after` seconds. A burst is one to `burst` sparks
/// (the game shader `shader`, tinted `color` at `opacity`, fading as they die), shot out
/// at `speed` units a second (each up to `speed_jitter` of it slower) in a cone `spread`
/// degrees round its axis, falling `gravity` units a second²; the axis is the world's up
/// (`aim` 0), the way the blade points (1), back out of the hilt (−1), or between; a spark lasts about
/// `life` seconds, `size` across, streaking `length` units behind it along its way.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Sparks {
    pub(crate) shader: String,
    pub(crate) color: Rgb,
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
    /// How hard a wandering force pushes each one about (units a second²; 0, the default,
    /// none): embers darting and swirling instead of drifting straight.
    #[serde(default)]
    pub(crate) wander: f32,
    /// How many bursts go at once when the blade clashes (0, the default, to
    /// [`MAX_CLASH_BURSTS`]).
    #[serde(default)]
    pub(crate) clash: u32,
}

/// The most bursts a clash lets go at once, of a spark set or of the bolts.
pub(crate) const MAX_CLASH_BURSTS: u32 = 8;

/// `count`, named `field`, at most [`MAX_CLASH_BURSTS`].
fn clash_bursts(field: &str, count: u32) -> Result<(), String> {
    if count > MAX_CLASH_BURSTS {
        return Err(format!(
            "{field} must be at most {MAX_CLASH_BURSTS}, not {count}"
        ));
    }
    Ok(())
}

impl Sparks {
    /// Each value within its range, named under `at` (`sparks`, or `sparks[1]` in a list).
    pub(crate) fn check(&self, at: &str) -> Result<(), String> {
        let name = |field: &str| format!("{at}.{field}");
        game_path(&name("shader"), &self.shader)?;
        colour(&name("color"), self.color, 4.0)?;
        within(&name("opacity"), self.opacity, 0.0, 1.0)?;
        within(&name("rate"), self.rate, 0.0, 30.0)?;
        within(&name("ignite"), self.ignite, 0.0, 10.0)?;
        within(&name("ignite_rate"), self.ignite_rate, 0.0, 60.0)?;
        within(&name("after"), self.after, 0.0, 10.0)?;
        within(&name("after_rate"), self.after_rate, 0.0, 60.0)?;
        if !(1..=16).contains(&self.burst) {
            return Err(format!(
                "{} must be 1 to 16, not {}",
                name("burst"),
                self.burst
            ));
        }
        within(&name("speed"), self.speed, 0.0, 500.0)?;
        within(&name("speed_jitter"), self.speed_jitter, 0.0, 1.0)?;
        within(&name("spread"), self.spread, 0.0, 180.0)?;
        within(&name("aim"), self.aim, -1.0, 1.0)?;
        within(&name("gravity"), self.gravity, -800.0, 2_000.0)?;
        within(&name("life"), self.life, 0.02, 6.0)?;
        within(&name("size"), self.size, 0.05, 8.0)?;
        within(&name("length"), self.length, 0.0, 32.0)?;
        within(&name("along"), self.along, 0.0, 1.0)?;
        within(&name("wander"), self.wander, 0.0, 400.0)?;
        clash_bursts(&name("clash"), self.clash)
    }
}

/// Where the blade is unsettled: its unrest (granulation, flares, shimmer, the frayed edge,
/// the tongues' flicker and the arcs) is whole for `reach` units from the emitter, then
/// calms over `fade` units to `far` of itself: a blade wild at the emitter and steady up
/// its length. The arcs hug the blade as it calms, bulging out less. `lean` (default 0)
/// slants the flame tongues' spikes toward the tip: units along a capsule radius out.
/// `swing` (default 0) reaches the unrest that many units further up the blade as it
/// swings, by the blade's swing (`saber_motion.rs`, 0 to 1). `haze` (default 0, 0 to 1)
/// calms the heat haze with it: 1 bends the scene only where the blade is unsettled, 0 all
/// along as before. `clash` (default 0) reaches the arcs' unrest alone (not the blade's
/// edge and flicker) that many units further at once when the blade clashes (another
/// blade, a wall, the Saber Lab's Clash), settling back over `clash_time` seconds (default
/// 0.6).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Unrest {
    pub(crate) reach: f32,
    pub(crate) fade: f32,
    pub(crate) far: f32,
    #[serde(default)]
    pub(crate) lean: f32,
    #[serde(default)]
    pub(crate) swing: f32,
    #[serde(default)]
    pub(crate) haze: f32,
    #[serde(default)]
    pub(crate) clash: f32,
    #[serde(default = "Unrest::default_clash_time")]
    pub(crate) clash_time: f32,
}

impl Unrest {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        within("unrest.reach", self.reach, 0.0, 64.0)?;
        within("unrest.fade", self.fade, 0.5, 64.0)?;
        within("unrest.far", self.far, 0.0, 1.0)?;
        within("unrest.lean", self.lean, -16.0, 16.0)?;
        within("unrest.swing", self.swing, 0.0, 64.0)?;
        within("unrest.haze", self.haze, 0.0, 1.0)?;
        within("unrest.clash", self.clash, 0.0, 64.0)?;
        within("unrest.clash_time", self.clash_time, 0.05, 5.0)
    }

    fn default_clash_time() -> f32 {
        0.6
    }
}

/// A crackling star at the emitter, `along` the blade: the Sun's glint gone wild. `count`
/// stars at once, each `rays` thin rays `length` units long (`width` a share of it thick)
/// round a hot heart (`halo`), in `color` at `brightness`; `rate` times a second each
/// shuffles anew: its rays' angles and lengths thrown about by `chaos` (0 even and whole, 1
/// wild), its light flickering by it too, and the star jumping up to `jitter` units about
/// its place. `lean` (0 to 1) stretches the rays that point up the blade and shortens the
/// ones that point back, so it sprays toward the tip. `light` (default 0) makes the
/// blade's light crackle with it: at a shuffle's brightest the stars add that share of the
/// light (1 doubles it), dimmer shuffles less. For `ignite` seconds once lit (default 0:
/// never) the stars surge: up to `surge` more bright (1 twice as bright) and `grow` more
/// big (1 twice as big) the moment it lights, falling off to themselves, the light with
/// them. For `die` seconds once put out (default 0: never) they sputter on without the
/// blade, now out, now flaring, out more often and dimmer as they die. `burst` (none by
/// default) sends a ring of heat out from the emitter as the blade lights ([`Burst`]).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Flash {
    pub(crate) color: Rgb,
    pub(crate) brightness: f32,
    pub(crate) count: u32,
    pub(crate) rays: u32,
    pub(crate) length: f32,
    pub(crate) width: f32,
    pub(crate) halo: f32,
    pub(crate) rate: f32,
    pub(crate) chaos: f32,
    pub(crate) jitter: f32,
    pub(crate) lean: f32,
    pub(crate) along: f32,
    #[serde(default)]
    pub(crate) light: f32,
    #[serde(default)]
    pub(crate) ignite: f32,
    #[serde(default)]
    pub(crate) surge: f32,
    #[serde(default)]
    pub(crate) grow: f32,
    #[serde(default)]
    pub(crate) die: f32,
    #[serde(default)]
    pub(crate) burst: Option<Burst>,
}

/// A flash's heat burst: as the blade lights, a ring of bent air `width` units thick
/// spreads from the emitter to `reach` units over `time` seconds, bending the scene behind
/// it up to `strength` units at first and less as it goes (drawn with the heat haze).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Burst {
    pub(crate) strength: f32,
    pub(crate) reach: f32,
    pub(crate) time: f32,
    pub(crate) width: f32,
}

/// The most stars a flash may have at once, and rays a star.
pub(crate) const MAX_FLASHES: u32 = 4;
pub(crate) const MAX_FLASH_RAYS: u32 = 12;

impl Flash {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        colour("flash.color", self.color, 4.0)?;
        within("flash.brightness", self.brightness, 0.0, 10.0)?;
        if !(1..=MAX_FLASHES).contains(&self.count) {
            return Err(format!(
                "flash.count must be 1 to {MAX_FLASHES}, not {}",
                self.count
            ));
        }
        if !(2..=MAX_FLASH_RAYS).contains(&self.rays) {
            return Err(format!(
                "flash.rays must be 2 to {MAX_FLASH_RAYS}, not {}",
                self.rays
            ));
        }
        within("flash.length", self.length, 0.2, 24.0)?;
        within("flash.width", self.width, 0.01, 0.5)?;
        within("flash.halo", self.halo, 0.0, 4.0)?;
        within("flash.rate", self.rate, 0.0, 60.0)?;
        within("flash.chaos", self.chaos, 0.0, 1.0)?;
        within("flash.jitter", self.jitter, 0.0, 8.0)?;
        within("flash.lean", self.lean, 0.0, 1.0)?;
        within("flash.along", self.along, 0.0, 1.0)?;
        within("flash.light", self.light, 0.0, 4.0)?;
        within("flash.ignite", self.ignite, 0.0, 10.0)?;
        within("flash.surge", self.surge, 0.0, 8.0)?;
        within("flash.grow", self.grow, 0.0, 2.0)?;
        within("flash.die", self.die, 0.0, 5.0)?;
        if let Some(b) = &self.burst {
            within("flash.burst.strength", b.strength, 0.0, 4.0)?;
            within("flash.burst.reach", b.reach, 1.0, 64.0)?;
            within("flash.burst.time", b.time, 0.05, 3.0)?;
            within("flash.burst.width", b.width, 0.2, 16.0)?;
        }
        Ok(())
    }
}

/// The skin is a chroma (`docs/unlockables.md`): it takes its wearer's saber colour
/// (`color1` for the first saber, `color2` the second, the RGB ones too), its colours turned
/// round to that hue from its corona's rim's, as a catalogue chroma's are. `effects`
/// (default true) turns its emitter's smoke, ribbons, sparks and bolts with it.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Chroma {
    #[serde(default = "Chroma::turned")]
    pub(crate) effects: bool,
}

impl Chroma {
    fn turned() -> bool {
        true
    }
}

/// The emitter's crackle heard: `buzz`, a sound looped under the hum while the blade is
/// lit, at `buzz_volume`, and `snaps`, one of which (at random) sounds at `snap_volume`
/// each time a burst of the skin's `bolts` leaves the emitter. Game paths; an empty `buzz`
/// or no `snaps` leaves that part out.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Crackle {
    pub(crate) buzz: String,
    pub(crate) buzz_volume: f32,
    pub(crate) snaps: Vec<String>,
    pub(crate) snap_volume: f32,
}

/// The most snaps a crackle may name.
pub(crate) const MAX_SNAPS: usize = 8;

impl Crackle {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        if !self.buzz.is_empty() {
            game_path("crackle.buzz", &self.buzz)?;
        }
        within("crackle.buzz_volume", self.buzz_volume, 0.0, 2.0)?;
        if self.snaps.len() > MAX_SNAPS {
            return Err(format!(
                "crackle.snaps must name at most {MAX_SNAPS} sounds, not {}",
                self.snaps.len()
            ));
        }
        for (index, snap) in self.snaps.iter().enumerate() {
            game_path(&format!("crackle.snaps[{index}]"), snap)?;
        }
        within("crackle.snap_volume", self.snap_volume, 0.0, 2.0)
    }
}

/// The most spark sets a skin may have ([`SparkSets`]).
pub(crate) const MAX_SPARK_SETS: usize = 3;

/// A skin's `sparks`: one set (an object, as first written) or several (a list of them:
/// a welder's spray and embers drifting off, say), each its own emitter.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SparkSets {
    One(Sparks),
    Many(Vec<Sparks>),
}

impl SparkSets {
    /// The sets, in order.
    pub(crate) fn sets(&self) -> &[Sparks] {
        match self {
            Self::One(set) => std::slice::from_ref(set),
            Self::Many(sets) => sets,
        }
    }

    /// Each set within its ranges, and at most [`MAX_SPARK_SETS`] of them.
    pub(crate) fn check(&self) -> Result<(), String> {
        match self {
            Self::One(set) => set.check("sparks"),
            Self::Many(sets) => {
                if sets.is_empty() || sets.len() > MAX_SPARK_SETS {
                    return Err(format!(
                        "sparks must hold 1 to {MAX_SPARK_SETS} sets, not {}",
                        sets.len()
                    ));
                }
                sets.iter()
                    .enumerate()
                    .try_for_each(|(index, set)| set.check(&format!("sparks[{index}]")))
            }
        }
    }
}

impl<'de> Deserialize<'de> for SparkSets {
    /// An object is one set, a list several; each read strictly, so a mistake names its
    /// field (an untagged enum would only say that nothing matched).
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_array() {
            serde_json::from_value(value)
                .map(Self::Many)
                .map_err(D::Error::custom)
        } else {
            serde_json::from_value(value)
                .map(Self::One)
                .map_err(D::Error::custom)
        }
    }
}

/// Bolts crackling out of the emitter, `along` the blade: strikes at random moments, as
/// [`Sparks`]' bursts come (`rate`, `ignite` and `ignite_rate`, `after` and `after_rate`),
/// each one to `burst` jagged bolts of the game's electricity (the game shader `shader`,
/// tinted `color` at `opacity`) from the emitter to a point `reach` units off (each up to
/// `reach_jitter` of it shorter) in a cone `spread` degrees round its axis (`aim` as for
/// sparks), `width` units across, as jagged as `chaos`, forking when `branch`; a bolt
/// lasts about `life` seconds, crackling into a new shape every frame.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Bolts {
    pub(crate) shader: String,
    pub(crate) color: Rgb,
    pub(crate) opacity: f32,
    pub(crate) rate: f32,
    pub(crate) ignite: f32,
    pub(crate) ignite_rate: f32,
    pub(crate) after: f32,
    pub(crate) after_rate: f32,
    pub(crate) burst: u32,
    pub(crate) reach: f32,
    pub(crate) reach_jitter: f32,
    pub(crate) spread: f32,
    pub(crate) aim: f32,
    pub(crate) width: f32,
    pub(crate) chaos: f32,
    pub(crate) branch: bool,
    pub(crate) life: f32,
    pub(crate) along: f32,
    /// Bursts at once when the blade clashes (default 0).
    #[serde(default)]
    pub(crate) clash: u32,
}

impl Bolts {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        game_path("bolts.shader", &self.shader)?;
        colour("bolts.color", self.color, 4.0)?;
        within("bolts.opacity", self.opacity, 0.0, 1.0)?;
        within("bolts.rate", self.rate, 0.0, 30.0)?;
        within("bolts.ignite", self.ignite, 0.0, 10.0)?;
        within("bolts.ignite_rate", self.ignite_rate, 0.0, 60.0)?;
        within("bolts.after", self.after, 0.0, 10.0)?;
        within("bolts.after_rate", self.after_rate, 0.0, 60.0)?;
        if !(1..=8).contains(&self.burst) {
            return Err(format!("bolts.burst must be 1 to 8, not {}", self.burst));
        }
        within("bolts.reach", self.reach, 0.5, 64.0)?;
        within("bolts.reach_jitter", self.reach_jitter, 0.0, 1.0)?;
        within("bolts.spread", self.spread, 0.0, 180.0)?;
        within("bolts.aim", self.aim, -1.0, 1.0)?;
        within("bolts.width", self.width, 0.05, 8.0)?;
        within("bolts.chaos", self.chaos, 0.0, 2.0)?;
        within("bolts.life", self.life, 0.02, 2.0)?;
        within("bolts.along", self.along, 0.0, 1.0)?;
        clash_bursts("bolts.clash", self.clash)
    }
}

impl Smoke {
    /// Each value within its range.
    pub(crate) fn check(&self) -> Result<(), String> {
        game_path("smoke.shader", &self.shader)?;
        colour("smoke.color", self.color, 1.0)?;
        within("smoke.opacity", self.opacity, 0.0, 1.0)?;
        within("smoke.rate", self.rate, 0.0, 200.0)?;
        within("smoke.after_rate", self.after_rate, 0.0, 200.0)?;
        within("smoke.after", self.after, 0.0, 20.0)?;
        within("smoke.life", self.life, 0.1, 10.0)?;
        within("smoke.size", self.size, 0.2, 32.0)?;
        within("smoke.growth", self.growth, 0.2, 96.0)?;
        within("smoke.rise", self.rise, -50.0, 100.0)?;
        within("smoke.spread", self.spread, 0.0, 50.0)?;
        within("smoke.follow", self.follow, 0.0, 1.0)?;
        within("smoke.spin", self.spin, 0.0, 10.0)?;
        within("smoke.turn", self.turn, 0.0, 180.0)?;
        within("smoke.sway", self.sway, 0.0, 50.0)?;
        within("smoke.sway_rate", self.sway_rate, 0.0, 10.0)?;
        within("smoke.along", self.along, 0.0, 1.0)
    }
}
