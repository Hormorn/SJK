//! Mining, SJK's own toy (`toy_mining`; Slash's design, 04/11/2026, phase 1): the player
//! takes a pickaxe in hand, and boulders come up out of the ground round them, a few at a
//! time ([`ROCKS`]). The player walks up to one and strikes it with `+use`, the whole body
//! in the swing (`BOTH_A3_T__B_`, the strong saber style's blow from overhead, the
//! pickaxe held as a hilt is); ten to fourteen blows
//! break it, and it gives rubble or ore, counted in the player's mining log
//! (`mining_log.txt`). Another boulder comes up elsewhere 4 to 8 seconds later. Now and then
//! a glint flashes on a boulder being broken: a blow that lands while it shines is a clean
//! strike, worth two, and any makes the boulder give better ([`roll_find`]).
//!
//! Unlike Fishing it is no waiting game: the player walks from boulder to boulder, and
//! the pace is theirs, a swing at a time. Attacking, taking damage or dying puts the
//! pickaxe away.
//!
//! Phase 1 changes nothing at the hub: the finds are a local log, no credits, no
//! shards, no achievements. Selling them (phase 2) is to share the Fishing junk's daily
//! cap (`fishing_sell.rs`), one cap for both, so mining earns nothing more than fishing
//! does alone.
//!
//! No game server knows of it: it travels through the SJK hub as a toy in the player's
//! look ([`TOY`]), so other SJK players see the pickaxe in their hand (at rest: swings are
//! not sent); the boulders are the miner's own, seen by no one else. Players on stock
//! clients see a player with empty hands.
//!
//! The models, pictures, shader and icon are bundled ([`FILES`], made by
//! `scripts/mining_assets.py`) and mounted below all game data. The sounds are retail
//! game paths.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_protocol::UserCommand;
use sjk_vfs::{VfsError, VirtualFileSystem};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The toy's console command.
pub(crate) const COMMAND: &str = "toy_mining";
/// The toy's id in a look's `toys` (`PROTOCOL.md`, "Looks").
pub(crate) const TOY: &str = "mining";
pub(crate) const PICK: &str = "models/sjk/mining_pick.md3";
pub(crate) const BOULDER: &str = "models/sjk/mining_boulder.md3";
pub(crate) const GLINT: &str = "models/sjk/mining_glint.md3";
/// What a boulder gives when it gives rubble.
pub(crate) const RUBBLE: &str = "models/sjk/mining_rubble.md3";
pub(crate) const ICON: &str = "gfx/sjk/toy_mining.png";
/// The models, in the order [`GpuState::submit_mining`] finds their meshes (each ore's
/// own in [`ORE_MODELS`]).
pub(crate) const MODELS: [&str; 4] = [PICK, BOULDER, GLINT, RUBBLE];
/// A swing through the air.
const SWING: &str = "sound/weapons/melee/swing1.mp3";
/// A blow on the rock, the two taken in turn.
const HIT: [&str; 2] = [
    "sound/effects/stone_bounce.wav",
    "sound/effects/stone_bounce2.wav",
];
/// A clean strike: the pick rings.
const CLEAN: &str = "sound/weapons/flechette/ricochet1.wav";
/// The glint flashing on a boulder.
const GLINT_SOUND: &str = "sound/ambience/spark1.wav";
/// A boulder breaking.
const BREAK: &str = "sound/effects/stone_break1.mp3";
/// The rarest find.
const RARE: &str = "sound/interface/secret_area.mp3";
const SOUNDS: [&str; 7] = [SWING, HIT[0], HIT[1], CLEAN, GLINT_SOUND, BREAK, RARE];
/// The mining log, in the profile's folder: `<find id> <count>` a line.
pub(crate) const LOG_FILE: &str = "mining_log.txt";

/// A bundled file's bytes, by its name in `assets/mining`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/mining/", $name))
    };
}

/// One ore's own model (`mining_assets.py`'s `ORES`): its find id, its game path and
/// bundled bytes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OreModel {
    pub(crate) id: &'static str,
    pub(crate) model: &'static str,
    bytes: &'static [u8],
}

macro_rules! ore_models {
    ($($id:literal),* $(,)?) => {
        [$(OreModel {
            id: $id,
            model: concat!("models/sjk/mining_ore_", $id, ".md3"),
            bytes: include_bytes!(concat!("../assets/mining/mining_ore_", $id, ".md3")),
        }),*]
    };
}

/// Each ore of [`FINDS`] its own model, in the same order.
pub(crate) const ORE_MODELS: [OreModel; 9] = ore_models![
    "iron_ore",
    "copper_ore",
    "silver_nugget",
    "glowstone",
    "sunstone",
    "moonstone",
    "star_quartz",
    "void_crystal",
    "heart_of_the_mountain",
];

/// The model of the ore `id`, and its place in [`ORE_MODELS`].
fn ore_model(id: &str) -> Option<usize> {
    ORE_MODELS.iter().position(|ore| ore.id == id)
}

/// The bundled files at their game paths (and the ores, [`ORE_MODELS`]).
const FILES: [(&str, &[u8]); 8] = [
    (PICK, bundled!("mining_pick.md3")),
    (BOULDER, bundled!("mining_boulder.md3")),
    (GLINT, bundled!("mining_glint.md3")),
    (RUBBLE, bundled!("mining_rubble.md3")),
    ("models/sjk/mining.png", bundled!("mining.png")),
    ("models/sjk/mining_glint.png", bundled!("mining_glint.png")),
    ("shaders/sjk_mining.shader", bundled!("mining.shader")),
    (ICON, bundled!("toy_mining.png")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    let ores = ORE_MODELS.iter().map(|ore| (ore.model, ore.bytes));
    vfs.mount_memory("SJK mining", FILES.iter().copied().chain(ores))?;
    Ok(())
}

const SLOTS: usize = crate::looks::SLOTS;
const BUTTON_ATTACK: u16 = 1;
const BUTTON_USE: u16 = 32;
const BUTTON_ALT_ATTACK: u16 = 128;
const CONTENTS_SOLID: u32 = 1;
/// Lava, slime and water: no boulder comes up in them.
const CONTENTS_LIQUID: u32 = 8 | 16 | 4;
const SURF_SKY: u32 = 0x2000;
const SURF_NOIMPACT: u32 = 0x0008_0000;
const ET_PLAYER: u8 = 1;
const ET_ITEM: u8 = 2;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
const PMF_FOLLOW: u16 = 4096;
/// Health above this counts down by itself (`STAT_MAX_HEALTH`'s default).
const MAX_HEALTH: i32 = 100;
const WP_SABER: u8 = 3;
/// How long the hands are tried to be emptied for the pickaxe.
const EMPTY_HANDS_FOR: Duration = Duration::from_secs(2);
/// From the player's origin down to their feet (`DEFAULT_MINS_2`).
const FEET: f32 = -24.0;
/// How many boulders wait round the player at once.
pub(crate) const ROCKS: usize = 3;
/// A boulder comes up this far from the player (level distance), at least this far from
/// any other; one farther from them than [`ROCK_FAR`] goes.
const ROCK_RANGE: (f32, f32) = (140.0, 420.0);
const ROCK_APART: f32 = 110.0;
/// Nor nearer than this to an item on the map (a weapon, a pickup).
const ITEM_CLEAR: f32 = 56.0;
const ROCK_FAR: f32 = 1_400.0;
/// The next boulder comes up this long after one broke (seconds), so another always
/// waits (about six a minute, the blows the pace); the first ones, this soon after the
/// pickaxe is taken out.
const ROCK_AGAIN: (f32, f32) = (4.0, 8.0);
const ROCK_FIRST: (f32, f32) = (0.3, 1.0);
/// The first ones come quickly for this long after the pickaxe is out.
const FIRST_WHILE: Duration = Duration::from_secs(4);
/// Spots tried for a boulder each frame.
const PLACE_TRIES: usize = 4;
/// A boulder rises out of the ground this long, from this far below.
const RISE: Duration = Duration::from_millis(600);
const RISE_FROM: f32 = 16.0;
/// A boulder's ground is at most this far below the player's feet (a step down, a slope).
const DROP: f32 = 64.0;
/// A blow reaches a boulder whose middle is this near the player (level distance), this
/// far round from where they face (radians each side), and not far above or below.
pub(crate) const STRIKE_REACH: f32 = 64.0;
const STRIKE_ARC: f32 = 1.0;
const STRIKE_HEIGHT: f32 = 72.0;
/// A swing, from the press to the pick back at rest, and when in it the blow lands
/// (`BOTH_A3_T__B_`: 27 frames at 40 a second).
const SWING_TIME: Duration = Duration::from_millis(700);
const IMPACT: Duration = Duration::from_millis(420);
/// The animation the whole body plays for a swing: the strong saber style's blow from
/// over the head down, both hands on the hilt (here the pickaxe's handle).
const SWING_POSE: &str = "BOTH_A3_T__B_";
/// The torso's pose with the pickaxe in hand: the strong style's stance, both hands on
/// the hilt.
const HOLD_POSE: &str = "BOTH_SABERSLOW_STANCE";
/// The blows a boulder takes, a clean strike counting two.
const ROCK_HITS: (u32, u32) = (10, 14);
/// The first glint on a boulder comes this long after its first blow, the next this long
/// after the last went (seconds); each shines this long.
const GLINT_FIRST: (f32, f32) = (1.0, 2.5);
const GLINT_GAP: (f32, f32) = (1.5, 4.0);
const GLINT_WINDOW: Duration = Duration::from_millis(800);
/// What a broken boulder gives flies out and lies at the player's feet this long.
const SHOW_FIND: Duration = Duration::from_millis(1800);
const FLY: Duration = Duration::from_millis(450);
/// The hand holds the pickaxe this far up from its butt (`mining_assets.py`'s grip).
const GRIP: f32 = 5.0;

/// What a broken boulder gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Rubble,
    Ore,
}

/// One thing in the find table: its id in the mining log, its name, its kind and its
/// weight among its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Find {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    pub(crate) weight: u32,
}

const fn rubble(id: &'static str, name: &'static str) -> Find {
    Find {
        id,
        name,
        kind: Kind::Rubble,
        weight: 1,
    }
}

const fn ore(id: &'static str, name: &'static str, weight: u32) -> Find {
    Find {
        id,
        name,
        kind: Kind::Ore,
        weight,
    }
}

/// How many kinds of ore [`FINDS`] holds (Prospector's goal).
pub(crate) const ORE_KINDS: usize = {
    let mut count = 0;
    let mut index = 0;
    while index < FINDS.len() {
        if matches!(FINDS[index].kind, Kind::Ore) {
            count += 1;
        }
        index += 1;
    }
    count
};

/// The rarest ore's id: bad luck protection raises its weight ([`HEART_PITY`]).

pub(crate) const HEART: &str = "heart_of_the_mountain";

/// Everything a boulder can give: SJK's own names.
pub(crate) const FINDS: [Find; 13] = [
    rubble("rubble", "a handful of rubble"),
    rubble("slag", "a lump of slag"),
    rubble("drill_bit", "a worn drill bit"),
    rubble("fossil", "a fossil shell"),
    ore("iron_ore", "a chunk of iron ore", 96),
    ore("copper_ore", "a chunk of copper ore", 86),
    ore("silver_nugget", "a silver nugget", 57),
    ore("glowstone", "a glowstone", 43),
    ore("sunstone", "a sunstone", 28),
    ore("moonstone", "a moonstone", 19),
    ore("star_quartz", "a star quartz", 11),
    ore("void_crystal", "a void crystal", 9),
    ore(HEART, "the Heart of the Mountain", 1),
];

/// The chance a boulder gives rubble, in percent, less this much for each clean strike,
/// but never under the floor.
const RUBBLE_CHANCE: u32 = 40;
const CLEAN_RUBBLE_CUT: u32 = 10;
const RUBBLE_FLOOR: u32 = 20;
/// Bad luck protection for the Heart of the Mountain, as Fishing's for Old Whiskers: its
/// weight rises by one for every this many ores dug since the last one. With the weights
/// above, one in about 230 boulders (some 40 minutes at six a minute, near Old
/// Whiskers' half hour since Fishing's balance pass).
const HEART_PITY: u32 = 100;
/// The mining log's count of the ores dug since the last Heart: `heart_wait <count>`.
const HEART_WAIT: &str = "heart_wait";

/// The weight of `find` among its kind, `heart_wait` ores after the last Heart.
fn weight(find: &Find, heart_wait: u32) -> u32 {
    if find.id == HEART {
        find.weight.saturating_add(heart_wait / HEART_PITY)
    } else {
        find.weight
    }
}

/// One weighted pick of `kind` from `pick`, a uniform number.
fn pick_of(kind: Kind, pick: u32, heart_wait: u32) -> &'static Find {
    let of_kind = || FINDS.iter().filter(move |f| f.kind == kind);
    let total: u32 = of_kind().map(|f| weight(f, heart_wait)).sum();
    let mut left = pick % total;
    of_kind()
        .find(|f| {
            let weight = weight(f, heart_wait);
            if left < weight {
                true
            } else {
                left -= weight;
                false
            }
        })
        .unwrap_or(&FINDS[0])
}

/// Roll a broken boulder's find from `roll` and `pick`, uniform numbers, after `clean`
/// clean strikes, `heart_wait` ores after the last Heart: rubble less often for each
/// clean strike, and with any clean strike the ore one tier rarer, but never into the
/// Heart, which only its own roll gives.
pub(crate) fn roll_find(roll: u32, pick: u32, clean: u32, heart_wait: u32) -> &'static Find {
    let rubble = RUBBLE_CHANCE
        .saturating_sub(CLEAN_RUBBLE_CUT.saturating_mul(clean))
        .max(RUBBLE_FLOOR);
    if roll % 100 < rubble {
        return pick_of(Kind::Rubble, pick, 0);
    }
    let ore = pick_of(Kind::Ore, pick, heart_wait);
    if clean == 0 || ore.id == HEART {
        return ore;
    }
    FINDS
        .iter()
        .position(|f| f.id == ore.id)
        .and_then(|at| FINDS.get(at + 1))
        .filter(|rarer| rarer.id != HEART)
        .unwrap_or(ore)
}

/// A small fast random source (xorshift64*), as Fishing's.
#[derive(Clone, Debug)]
struct Dice(u64);

impl Dice {
    fn seeded() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);
        Self(nanos | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }

    /// A number from 0 to 1.
    fn unit(&mut self) -> f32 {
        self.next() as f32 / u32::MAX as f32
    }

    /// A number in `range`, seconds.
    fn seconds(&mut self, (low, high): (f32, f32)) -> Duration {
        Duration::from_secs_f32(low + (high - low) * self.unit())
    }

    /// A whole number from `low` to `high`, both in.
    fn between(&mut self, (low, high): (u32, u32)) -> u32 {
        low + self.next() % (high - low + 1)
    }
}

/// A boulder on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Rock {
    /// Where it stands on the ground, and the way the ground faces there.
    at: [f32; 3],
    normal: [f32; 3],
    /// Its turn about the ground's normal (radians), and when it came up.
    spin: f32,
    rose: Instant,
    /// The blows it takes in all, and still.
    hits: u32,
    left: u32,
    /// The clean strikes it took.
    clean: u32,
    /// A glint shines on it until then; or the next comes then (none before its first
    /// blow).
    glint: Option<Instant>,
    next_glint: Option<Instant>,
}

impl Rock {
    fn struck(&self) -> bool {
        self.left < self.hits
    }
}

/// A swing under way: it started then, and its blow has landed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Swing {
    started: Instant,
    landed: bool,
}

/// A broken boulder's find, flying out and lying at the player's feet for a while.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Shown {
    find: &'static Find,
    from: [f32; 3],
    to: [f32; 3],
    since: Instant,
}

/// What the frame tells the toy of the player's own life.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Frame {
    /// The player plays (alive, in their own view): `None` out of a game.
    pub(crate) playing: Option<bool>,
    /// Their health.
    pub(crate) health: i32,
    /// Where they stand (their origin), and the way they face (radians).
    pub(crate) origin: [f32; 3],
    pub(crate) yaw: f32,
}

/// What the toy asks the frame to say or play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Say {
    /// A line on the centre of the screen.
    Centre(&'static str),
    /// A find, with how many the log now holds of it.
    Found(&'static Find, u32),
    Sound(&'static str),
    /// Empty the hands for the pickaxe: the fists if held, else the saber switched off
    /// ([`GpuState::empty_hands`]).
    EmptyHands,
}

/// The mining log: how many of each find the player dug out.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Log {
    counts: Vec<(String, u32)>,
}

impl Log {
    /// Read `text` (`<id> <count>` a line; anything else is skipped).
    pub(crate) fn parse(text: &str) -> Self {
        let mut log = Self::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let (Some(id), Some(value), None) = (words.next(), words.next(), words.next()) else {
                continue;
            };
            if let Ok(count) = value.parse::<u32>() {
                log.counts.retain(|(known, _)| known != id);
                log.counts.push((id.to_owned(), count));
            }
        }
        log
    }

    pub(crate) fn text(&self) -> String {
        self.counts
            .iter()
            .map(|(id, count)| format!("{id} {count}\n"))
            .collect()
    }

    pub(crate) fn count(&self, id: &str) -> u32 {
        self.counts
            .iter()
            .find(|(known, _)| known == id)
            .map_or(0, |(_, count)| *count)
    }

    /// Count one more `id`; returns the new count.
    fn add(&mut self, id: &str) -> u32 {
        if let Some((_, count)) = self.counts.iter_mut().find(|(known, _)| known == id) {
            *count = count.saturating_add(1);
            *count
        } else {
            self.counts.push((id.to_owned(), 1));
            1
        }
    }

    /// The ores dug since the last Heart of the Mountain (all of them before the first).
    pub(crate) fn heart_wait(&self) -> u32 {
        self.count(HEART_WAIT)
    }

    /// Count `find` (and the wait for the Heart); returns how many the log now holds.
    pub(crate) fn found(&mut self, find: &'static Find) -> u32 {
        if find.kind == Kind::Ore {
            if find.id == HEART {
                self.counts.retain(|(known, _)| known != HEART_WAIT);
            } else {
                self.add(HEART_WAIT);
            }
        }
        self.add(find.id)
    }

    /// The different ores dug, of all there are.
    pub(crate) fn kinds(&self) -> (usize, usize) {
        let all = FINDS.iter().filter(|f| f.kind == Kind::Ore);
        let found = all.clone().filter(|f| self.count(f.id) > 0).count();
        (found, all.count())
    }
}

/// Who holds a pickaxe this frame, by slot, and how it is turned in the hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Picks {
    pub(crate) held: [bool; SLOTS],
    pub(crate) turn: Quat,
}

impl Default for Picks {
    fn default() -> Self {
        Self {
            held: [false; SLOTS],
            turn: pick_turn(),
        }
    }
}

/// The toy's state.
pub(crate) struct Mining {
    /// The pickaxe is in hand, since then.
    on: bool,
    on_since: Option<Instant>,
    /// `toy_mining` was asked for.
    asked: bool,
    swing: Option<Swing>,
    /// The boulders round the player (kept while the pickaxe is away, drawn while it is
    /// out), and when the next comes up.
    rocks: Vec<Rock>,
    next_rock: Option<Instant>,
    /// The map the boulders are on.
    map: String,
    shown: Option<Shown>,
    /// The blows struck, for the two hit sounds in turn.
    blows: u32,
    /// `+use` went down since the last frame.
    use_pressed: bool,
    use_held: bool,
    /// The attack button (the pickaxe's swing while it is out) went down since the last
    /// frame, and is held now: held, the swings follow one another.
    strike_pressed: bool,
    strike_held: bool,
    /// The player alt-attacked since the last frame (or attacked, the pickaxe away).
    attacked: bool,
    health: Option<i32>,
    /// The mining log; `log_read`, it was read from the profile's folder (what was dug
    /// before that is counted in when it is).
    log: Option<Log>,
    log_read: bool,
    dice: Dice,
    said: Vec<Say>,
    /// The boulders broken, each find with its clean strikes, for the achievements.
    broken: Vec<(&'static Find, u32)>,
    /// Each holder's pickaxe, for the actors to draw ([`pick_in_hand`]).
    pub(crate) picks: Picks,
    sounds_registered: Option<u64>,
    /// The hands are to be emptied, tried until then (the saber's switch waits while
    /// the weapon is busy).
    empty_hands_until: Option<Instant>,
}

impl Default for Mining {
    fn default() -> Self {
        Self {
            on: false,
            on_since: None,
            asked: false,
            swing: None,
            rocks: Vec::with_capacity(ROCKS),
            next_rock: None,
            map: String::new(),
            shown: None,
            blows: 0,
            use_pressed: false,
            use_held: false,
            strike_pressed: false,
            strike_held: false,
            attacked: false,
            health: None,
            log: None,
            log_read: false,
            dice: Dice::seeded(),
            said: Vec::with_capacity(4),
            broken: Vec::new(),
            picks: Picks::default(),
            sounds_registered: None,
            empty_hands_until: None,
        }
    }
}

/// Whether health going from `before` to `now` is a hit: not health over the maximum
/// counting down a point a second, as it does after a spawn (105 to 100).
fn hurt(before: i32, now: i32) -> bool {
    now < before && (before <= MAX_HEALTH || before - now > 1)
}

/// The level distance from `a` to `b`, and the way from `a` to `b` (radians).
fn level_way(a: Vec3, b: Vec3) -> (f32, f32) {
    let d = (b - a).truncate();
    (d.length(), d.y.atan2(d.x))
}

/// The smallest turn (radians) between two ways.
fn turn_between(a: f32, b: f32) -> f32 {
    let tau = std::f32::consts::TAU;
    let d = (b - a).rem_euclid(tau);
    d.min(tau - d)
}

impl Mining {
    /// `toy_mining`: take the pickaxe in hand, or put it away.
    pub(crate) fn toggle(&mut self) {
        if self.on {
            self.put_away();
        } else {
            self.asked = true;
        }
    }

    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Note `+use` and the attacks in `command`. While the pickaxe is out the attack button
    /// swings it and is taken out of the command, so the weapon in hand does not fire; an
    /// alt attack puts the pickaxe away.
    pub(crate) fn shape_command(&mut self, command: &mut UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
        let striking = self.on && command.buttons & BUTTON_ATTACK != 0;
        if striking {
            command.buttons &= !BUTTON_ATTACK;
            if !self.strike_held {
                self.strike_pressed = true;
            }
        }
        self.strike_held = striking;
        if command.buttons & (BUTTON_ATTACK | BUTTON_ALT_ATTACK) != 0 {
            self.attacked = true;
        }
    }

    /// The boulders broken since the last call, for the achievements: each find and the
    /// clean strikes on its boulder.
    pub(crate) fn take_broken(&mut self) -> std::vec::Drain<'_, (&'static Find, u32)> {
        self.broken.drain(..)
    }

    pub(crate) fn take_said(&mut self) -> std::vec::Drain<'_, Say> {
        self.said.drain(..)
    }

    pub(crate) fn put_away(&mut self) {
        self.on = false;
        self.on_since = None;
        self.asked = false;
        self.swing = None;
        for rock in &mut self.rocks {
            rock.glint = None;
        }
    }

    /// The player chose a weapon (`weapon <n>`, the saber's 1 among them, or the next or
    /// previous one): the pickaxe goes away and the weapon comes out.
    pub(crate) fn weapon_chosen(&mut self) {
        if self.on {
            self.put_away();
            self.said.push(Say::Centre("You put the pickaxe away."));
        }
    }

    /// The map being played is `map`: boulders on another are gone.
    pub(crate) fn set_map(&mut self, map: &str) {
        if self.map != map {
            map.clone_into(&mut self.map);
            self.rocks.clear();
            self.next_rock = None;
            self.shown = None;
        }
    }

    /// How far into its swing the pickaxe is (none at rest).
    fn swinging(&self, now: Instant) -> Option<Duration> {
        self.swing
            .map(|swing| now.saturating_duration_since(swing.started))
    }

    /// Move the toy on to `now`. `ground` says where a boulder may stand near a spot (the
    /// ground under it, in sight, and the way the ground faces there).
    pub(crate) fn update(
        &mut self,
        frame: Frame,
        now: Instant,
        mut ground: impl FnMut([f32; 3]) -> Option<([f32; 3], [f32; 3])>,
    ) {
        let used = std::mem::take(&mut self.use_pressed)
            | std::mem::take(&mut self.strike_pressed)
            | self.strike_held;
        let attacked = std::mem::take(&mut self.attacked);
        let health = self.health.replace(frame.health);
        if self
            .shown
            .is_some_and(|shown| now >= shown.since + SHOW_FIND)
        {
            self.shown = None;
        }
        if std::mem::take(&mut self.asked) && !self.on {
            if frame.playing == Some(true) {
                self.on = true;
                self.on_since = Some(now);
                self.next_rock = None;
                self.said.push(Say::EmptyHands);
                self.said.push(Say::Centre(
                    "Pickaxe out: boulders come up round you. Strike one with Attack, and strike as it glints!",
                ));
            } else {
                self.said.push(Say::Centre("Mining waits until you play."));
            }
            return;
        }
        if !self.on {
            return;
        }
        let hurt = health.is_some_and(|before| hurt(before, frame.health));
        if frame.playing != Some(true) || attacked || hurt {
            self.put_away();
            if frame.playing == Some(true) {
                self.said.push(Say::Centre("You put the pickaxe away."));
            }
            return;
        }
        let origin = Vec3::from_array(frame.origin);
        self.rocks
            .retain(|rock| level_way(origin, Vec3::from_array(rock.at)).0 <= ROCK_FAR);
        self.place_rocks(origin, now, &mut ground);
        // The glints on the boulders being broken: each shines a while, then the next
        // waits.
        for rock in &mut self.rocks {
            match (rock.glint, rock.next_glint) {
                (Some(until), _) if now >= until => {
                    rock.glint = None;
                    rock.next_glint = Some(now + self.dice.seconds(GLINT_GAP));
                }
                (None, Some(next)) if now >= next => {
                    rock.glint = Some(now + GLINT_WINDOW);
                    self.said.push(Say::Sound(GLINT_SOUND));
                }
                _ => {}
            }
        }
        if used && self.swing.is_none() {
            self.swing = Some(Swing {
                started: now,
                landed: false,
            });
            self.said.push(Say::Sound(SWING));
        }
        if let Some(swing) = self.swing {
            if !swing.landed && now >= swing.started + IMPACT {
                self.swing = Some(Swing {
                    landed: true,
                    ..swing
                });
                self.strike(frame, now);
            }
            if now >= swing.started + SWING_TIME {
                self.swing = None;
            }
        }
    }

    /// Bring up a boulder if one is due: a spot [`ROCK_RANGE`] from the player at `origin`,
    /// apart from the others, where `ground` finds the ground.
    fn place_rocks(
        &mut self,
        origin: Vec3,
        now: Instant,
        ground: &mut impl FnMut([f32; 3]) -> Option<([f32; 3], [f32; 3])>,
    ) {
        if self.rocks.len() >= ROCKS {
            self.next_rock = None;
            return;
        }
        let first = self.on_since.is_some_and(|since| now < since + FIRST_WHILE);
        let due = *self.next_rock.get_or_insert_with(|| {
            now + self
                .dice
                .seconds(if first { ROCK_FIRST } else { ROCK_AGAIN })
        });
        if now < due {
            return;
        }
        for _ in 0..PLACE_TRIES {
            let angle = self.dice.unit() * std::f32::consts::TAU;
            let distance = ROCK_RANGE.0 + (ROCK_RANGE.1 - ROCK_RANGE.0) * self.dice.unit();
            let spot = origin + Vec3::new(angle.cos(), angle.sin(), 0.0) * distance;
            let Some((at, normal)) = ground(spot.to_array()) else {
                continue;
            };
            let point = Vec3::from_array(at);
            if self
                .rocks
                .iter()
                .any(|rock| Vec3::from_array(rock.at).distance(point) < ROCK_APART)
            {
                continue;
            }
            let hits = self.dice.between(ROCK_HITS);
            self.rocks.push(Rock {
                at,
                normal,
                spin: self.dice.unit() * std::f32::consts::TAU,
                rose: now,
                hits,
                left: hits,
                clean: 0,
                glint: None,
                next_glint: None,
            });
            self.next_rock = None;
            return;
        }
    }

    /// The boulder a blow from a player standing at `origin` facing `yaw` strikes: the
    /// nearest in reach before them.
    fn target(&self, origin: Vec3, yaw: f32) -> Option<usize> {
        self.rocks
            .iter()
            .enumerate()
            .filter_map(|(index, rock)| {
                let at = Vec3::from_array(rock.at);
                let (distance, way) = level_way(origin, at);
                (distance <= STRIKE_REACH
                    && (at.z - origin.z).abs() <= STRIKE_HEIGHT
                    && (distance < 16.0 || turn_between(yaw, way) <= STRIKE_ARC))
                    .then_some((index, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    /// A blow lands, the player standing and facing as `frame` says. One striking no
    /// boulder says nothing: a word there would cut short what the last boulder gave,
    /// shown in the centre while the attack is held (Slash, 05/11/2026).
    fn strike(&mut self, frame: Frame, now: Instant) {
        let Some(index) = self.target(Vec3::from_array(frame.origin), frame.yaw) else {
            return;
        };
        self.blows = self.blows.wrapping_add(1);
        let hit = HIT[self.blows as usize % HIT.len()];
        let rock = &mut self.rocks[index];
        if rock.next_glint.is_none() {
            rock.next_glint = Some(now + self.dice.seconds(GLINT_FIRST));
        }
        if rock.glint.is_some_and(|until| now < until) {
            rock.clean += 1;
            rock.left = rock.left.saturating_sub(2);
            rock.glint = None;
            rock.next_glint = Some(now + self.dice.seconds(GLINT_GAP));
            self.said.push(Say::Sound(CLEAN));
            self.said.push(Say::Centre("Clean strike!"));
        } else {
            rock.left = rock.left.saturating_sub(1);
            self.said.push(Say::Sound(hit));
        }
        if rock.left == 0 {
            let rock = self.rocks.remove(index);
            self.break_rock(rock, frame.origin, now);
        }
    }

    fn break_rock(&mut self, rock: Rock, origin: [f32; 3], now: Instant) {
        let heart_wait = self.log.as_ref().map_or(0, Log::heart_wait);
        let roll = self.dice.next();
        let pick = self.dice.next();
        let find = roll_find(roll, pick, rock.clean, heart_wait);
        let count = self.log.get_or_insert_with(Log::default).found(find);
        self.broken.push((find, rock.clean));
        self.said.push(Say::Sound(BREAK));
        if find.id == HEART {
            self.said.push(Say::Sound(RARE));
        }
        self.said.push(Say::Found(find, count));
        self.next_rock = None;
        // It jumps out of the broken boulder to the ground before the player.
        let at = Vec3::from_array(rock.at);
        let from = at + Vec3::Z * 10.0;
        let feet = Vec3::from_array(origin) + Vec3::Z * FEET;
        let toward = (feet - at).truncate().normalize_or_zero();
        let to = (at.truncate() + toward * 18.0).extend(feet.z.max(at.z) + 2.0);
        self.shown = Some(Shown {
            find,
            from: from.to_array(),
            to: to.to_array(),
            since: now,
        });
    }

    /// Read the mining log from `directory` once, counting in what was dug before.
    fn read_log(&mut self, directory: &Path) {
        if std::mem::replace(&mut self.log_read, true) {
            return;
        }
        let text = std::fs::read_to_string(directory.join(LOG_FILE)).unwrap_or_default();
        let mut log = Log::parse(&text);
        if let Some(pending) = self.log.take() {
            for (id, count) in pending.counts {
                for _ in 0..count {
                    log.add(&id);
                }
            }
        }
        self.log = Some(log);
    }

    fn save_log(&self, directory: &Path) {
        if let Some(log) = &self.log
            && let Err(error) = std::fs::write(directory.join(LOG_FILE), log.text())
        {
            crate::log::progress(format_args!("mining: cannot write {LOG_FILE}: {error}"));
        }
    }
}

/// Where a boulder may stand by `spot` (a point at the player's height): the ground under
/// it, at most [`DROP`] below the player's feet (not a floor below a ledge), seen from
/// `eye`, flat enough, dry and diggable. `trace` sweeps a point and says
/// whether it started in solid, the fraction, the end, the hit plane's normal, the surface
/// flags and the contents it met.
pub(crate) fn ground_at(
    eye: Vec3,
    spot: [f32; 3],
    mut trace: impl FnMut([f32; 3], [f32; 3], u32) -> Trace,
) -> Option<([f32; 3], [f32; 3])> {
    let top = [spot[0], spot[1], spot[2] + 16.0];
    let sight = trace(eye.to_array(), top, CONTENTS_SOLID);
    if sight.start_solid || sight.fraction < 1.0 {
        return None;
    }
    let below = [top[0], top[1], spot[2] + FEET - DROP];
    let down = trace(top, below, CONTENTS_SOLID | CONTENTS_LIQUID);
    let normal = down.normal?;
    if down.start_solid
        || down.fraction >= 1.0
        || down.contents & CONTENTS_LIQUID != 0
        || down.flags & (SURF_SKY | SURF_NOIMPACT) != 0
        || normal[2] < 0.75
    {
        return None;
    }
    // The boulder itself in sight, not round a corner of the ledge.
    let seen = [down.end[0], down.end[1], down.end[2] + 8.0];
    let sight = trace(eye.to_array(), seen, CONTENTS_SOLID);
    (!sight.start_solid && sight.fraction >= 1.0).then_some((down.end, normal))
}

/// What a point sweep met, for [`ground_at`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Trace {
    pub(crate) start_solid: bool,
    pub(crate) fraction: f32,
    pub(crate) end: [f32; 3],
    pub(crate) normal: Option<[f32; 3]>,
    pub(crate) flags: u32,
    pub(crate) contents: u32,
}

/// Who holds a pickaxe this frame (bits by slot), for the pose ([`holding_pick`]).
static HOLDING: AtomicU64 = AtomicU64::new(0);
/// The own swing this frame: the slot plus one in the high half, how far into it
/// (milliseconds) in the low; 0 with none ([`swinging`]).
static SWINGING: AtomicU64 = AtomicU64::new(0);

/// Whether the player in `slot` holds a pickaxe this frame: their torso takes the rod's
/// pose, both hands before the chest (`fishing::hold_pose`, `actor_pose_steps.rs`).
pub(crate) fn holding_pick(slot: usize) -> bool {
    slot < SLOTS && HOLDING.load(Ordering::Relaxed) & (1 << slot) != 0
}

/// How far (milliseconds) the player in `slot` is into a swing this frame: their whole
/// body plays [`swing_pose`] from its start (`actor_pose_steps.rs`).
pub(crate) fn swinging(slot: usize) -> Option<i64> {
    let swing = SWINGING.load(Ordering::Relaxed);
    (swing >> 32 == slot as u64 + 1).then_some((swing & 0xFFFF_FFFF) as i64)
}

/// The protocol animation of the pose holding the pickaxe.
pub(crate) fn hold_pose() -> Option<(usize, &'static str)> {
    static POSE: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    POSE.get_or_init(|| sjk_game_jka::legacy_animation_index(HOLD_POSE))
        .map(|index| (index, HOLD_POSE))
}

/// The protocol animation of the swing.
pub(crate) fn swing_pose() -> Option<(usize, &'static str)> {
    static POSE: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    POSE.get_or_init(|| sjk_game_jka::legacy_animation_index(SWING_POSE))
        .map(|index| (index, SWING_POSE))
}

/// The pickaxe in the hand's frame, as a saber's hilt is held: its handle (the model's
/// +X) out of the top of the fist along the hand bolt's +Z, its spike (the model's -Z)
/// forward along -Y, where a gun's barrel points (`propgun_frame::HAND_TURN`). Fitted in
/// game against a held hilt (04/11/2026).
pub(crate) fn pick_turn() -> Quat {
    Quat::from_mat3(&glam::Mat3::from_cols(Vec3::Z, Vec3::X, Vec3::Y))
}

/// The pickaxe in the right hand of a body posed with `matrices`, placed at `transform`:
/// gripped near its butt, turned with the hand (so it follows every pose and swing as a
/// hilt does), `turn` from the hand's frame ([`pick_turn`]).
pub(crate) fn pick_in_hand(
    preview: &crate::PlayerPreview,
    matrices: &[[[f32; 4]; 3]],
    transform: sjk_runtime::Transform,
    turn: Quat,
) -> Option<ActorInstance> {
    let hand = crate::saber::attachment_from_matrices(preview, matrices, crate::saber::HAND_BOLT)?;
    let rotation = crate::weapon_view::actor_world_rotation(transform.rotation);
    let (grip, hand) = crate::saber::world_attachment(
        Vec3::from_array(transform.translation),
        rotation,
        hand.scaled(transform.scale),
    );
    let forward = Quat::from_array(transform.rotation) * Vec3::X;
    let held = straightened((hand * turn).normalize(), forward);
    let scale = Vec3::from_array(transform.scale);
    let butt = grip - held * (Vec3::X * GRIP * scale.x);
    Some(ActorInstance::new(
        butt.to_array(),
        held.to_array(),
        scale.to_array(),
    ))
}

/// The pickaxe turned `held` (its handle the model's +X, its spike -Z) laid straight in
/// the upright plane the body faces along `forward`: the handle and the spike kept in it,
/// so the hands' roll in a saber move's swing does not tip the pickaxe to a side, and the
/// head stays level across the swing (Slash, 05/11/2026: "rovno"). Unchanged when the
/// handle points along the body's side.
fn straightened(held: Quat, forward: Vec3) -> Quat {
    let side = forward
        .truncate()
        .extend(0.0)
        .normalize_or_zero()
        .cross(Vec3::Z);
    if side == Vec3::ZERO {
        return held;
    }
    let side = side.normalize();
    let handle = held * Vec3::X;
    let Some(handle) = (handle - side * side.dot(handle)).try_normalize() else {
        return held;
    };
    // The spike, square to the handle in the plane, on the side it pointed.
    let spike = held * Vec3::NEG_Z;
    let mut straight = side.cross(handle);
    if straight.dot(spike) < 0.0 {
        straight = -straight;
    }
    let z = -straight;
    Quat::from_mat3(&glam::Mat3::from_cols(handle, z.cross(handle), z)).normalize()
}

/// The boulder `rock` at `now`: rising out of the ground, then shrinking as it is broken,
/// shaken a moment by a blow.
fn rock_instance(rock: &Rock, now: Instant, shake: f32) -> ActorInstance {
    let normal = Vec3::from_array(rock.normal).normalize_or(Vec3::Z);
    let rotation =
        Quat::from_axis_angle(normal, rock.spin) * Quat::from_rotation_arc(Vec3::X, normal);
    let risen =
        (now.saturating_duration_since(rock.rose).as_secs_f32() / RISE.as_secs_f32()).min(1.0);
    let eased = risen * risen * (3.0 - 2.0 * risen);
    let scale = 0.6 + 0.4 * rock.left as f32 / rock.hits.max(1) as f32;
    let at = Vec3::from_array(rock.at) - normal * (1.0 - eased) * RISE_FROM
        + Vec3::new(shake, -shake, 0.0);
    ActorInstance::new(at.to_array(), rotation.to_array(), [scale; 3])
}

/// The glint on `rock`, turning and swelling `t` seconds into its shine.
fn glint_instance(rock: &Rock, t: f32) -> ActorInstance {
    let normal = Vec3::from_array(rock.normal).normalize_or(Vec3::Z);
    let scale = 0.6 + 0.4 * rock.left as f32 / rock.hits.max(1) as f32;
    let at = Vec3::from_array(rock.at) + normal * 16.0 * scale;
    let window = GLINT_WINDOW.as_secs_f32();
    let swell = (t / window * std::f32::consts::PI).sin().max(0.0);
    ActorInstance::new(
        at.to_array(),
        Quat::from_rotation_z(t * 3.0).to_array(),
        [0.5 + 1.4 * swell; 3],
    )
}

/// A broken boulder's find `t` seconds after it broke: jumping out in an arc, then lying
/// still.
fn shown_instance(shown: &Shown, t: f32) -> ActorInstance {
    let along = (t / FLY.as_secs_f32()).min(1.0);
    let from = Vec3::from_array(shown.from);
    let to = Vec3::from_array(shown.to);
    let at = from.lerp(to, along) + Vec3::Z * (along * (1.0 - along) * 24.0);
    let turn = Quat::from_rotation_z(t * 2.0) * Quat::from_rotation_x((1.0 - along) * t * 8.0);
    ActorInstance::new(at.to_array(), turn.to_array(), [1.0; 3])
}

fn drawn(entity: &sjk_protocol::EntityState) -> bool {
    entity.entity_type() == ET_PLAYER && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
}

impl GpuState {
    /// `toy_mining`: take the pickaxe in hand (the fishing rod put away: `+use` is both
    /// toys'), or put it away.
    pub(crate) fn toggle_mining(&mut self) {
        if !self.mining.on() && self.fishing.on() {
            self.fishing.toggle();
        }
        self.mining.toggle();
    }

    /// The toy's frame: the own pickaxe's logic, then the boulders and every pickaxe (the
    /// own and other SJK players' whose look has it), and what it says.
    pub(crate) fn submit_mining(&mut self, now: Instant) {
        self.mining_frame(now);
        self.say_mining();
        if let Some(until) = self.mining.empty_hands_until {
            if !self.mining.on || now >= until || self.empty_hands() {
                self.mining.empty_hands_until = None;
            }
        }
    }

    /// Empty the hands for the pickaxe, once: the fists if the player holds them (as
    /// `weapmelee`), else a lit saber switched off (the saber switch, as `weapon 1` with
    /// the saber out). `false` while the weapon is busy, to try again the next frame.
    fn empty_hands(&mut self) -> bool {
        let Some(session) = &self.live_session else {
            return true;
        };
        let player = &session.latest_snapshot().player;
        let (weapon, weapon_time) = self
            .local_prediction
            .predicted_state()
            .map_or((player.weapon(), player.weapon_time()), |state| {
                (state.weapon, state.weapon_time)
            });
        let current = self.selected_weapon.unwrap_or(weapon);
        let inventory = sjk_client::LegacyWeaponInventory::from_player_state(player);
        if sjk_client::legacy_melee_weapon(&inventory, current).is_some() {
            if let Some(console) = self.console.as_mut()
                && let Err(error) = console.queue_command("weapmelee")
            {
                crate::log::progress(format_args!("mining: weapmelee: {error}"));
            }
            return true;
        }
        if weapon != WP_SABER || player.saber_holstered() != 0 {
            return true;
        }
        if weapon_time >= 1 {
            return false;
        }
        self.pending_generic_command = crate::input::generic_commands::GENCMD_SABER_SWITCH;
        true
    }

    fn mining_frame(&mut self, now: Instant) {
        self.mining.picks = Picks::default();
        let Some(session) = self.live_session.as_ref() else {
            self.mining.update(Frame::default(), now, |_| None);
            HOLDING.store(0, Ordering::Relaxed);
            SWINGING.store(0, Ordering::Relaxed);
            return;
        };
        let snapshot = session.latest_snapshot();
        let player = &snapshot.player;
        let spectating = player.movement_flags() & PMF_FOLLOW != 0
            || matches!(player.movement_type(), 4 | 7 | 8);
        let alive = player.health() > 0;
        let slots = crate::looks::ViewSlots::of(session.game_state(), player);
        let own = slots.own.filter(|&slot| slot < SLOTS);
        let origin = self
            .local_prediction
            .predicted_state()
            .map_or_else(|| player.origin(), |state| state.origin);
        let map = session
            .game_state()
            .config_string(0)
            .map(sjk_client::LegacyClientInfo::new)
            .and_then(|info| info.bytes("mapname"))
            .map(|bytes| String::from_utf8_lossy(bytes).to_ascii_lowercase())
            .unwrap_or_default();
        self.mining.set_map(&map);
        // A rod got out puts the pickaxe away (`toggle_mining` does the other way).
        if self.fishing.on() && self.mining.on() {
            self.mining.put_away();
        }
        let frame = Frame {
            playing: Some(alive && !spectating),
            health: player.health(),
            origin,
            yaw: self.camera_yaw,
        };
        let eye = Vec3::from_array(origin) + Vec3::Z * player.view_height() as f32;
        let bsp = &self.bsp;
        let items: Vec<Vec3> = snapshot
            .entities
            .iter()
            .filter(|entity| entity.entity_type() == ET_ITEM)
            .map(|entity| Vec3::from_array(entity.trajectory_base()))
            .collect();
        self.mining.update(frame, now, |spot| {
            let near_item = |at: [f32; 3]| {
                items
                    .iter()
                    .any(|item| item.distance(Vec3::from_array(at)) < ITEM_CLEAR)
            };
            ground_at(eye, spot, |start, end, mask| {
                let trace = bsp.trace_box(start, end, sjk_bsp::Aabb::POINT, mask);
                Trace {
                    start_solid: trace.start_solid,
                    fraction: trace.fraction,
                    end: trace.end_position,
                    normal: trace.plane.map(|plane| plane.normal),
                    flags: trace.surface_flags,
                    contents: trace.content_flags,
                }
            })
            .filter(|(at, _)| !near_item(*at))
        });
        let broke = self
            .mining
            .said
            .iter()
            .any(|say| matches!(say, Say::Found(..)));
        // The achievements count boulders on servers, as the rest: never a demo, a game
        // this client hosts or a server on this machine (`achievements::tracker`).
        let counts = !session.is_local() && !session.server().ip().is_loopback();
        for (find, clean) in self.mining.take_broken().collect::<Vec<_>>() {
            if counts {
                crate::achievements::update(|record| record.mined(find, clean));
            }
        }
        if let Some(directory) = self
            .console
            .as_ref()
            .map(|c| c.config_directory().to_owned())
        {
            if self.mining.on || broke {
                self.mining.read_log(&directory);
            }
            if broke {
                self.mining.save_log(&directory);
            }
        }
        let mesh_of = |model: &str| {
            self.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == model)
        };
        let [_, rock_mesh, glint_mesh, rubble_mesh] = MODELS.map(mesh_of);
        let ore_meshes = ORE_MODELS.map(|ore| mesh_of(ore.model));
        let mut holding = 0_u64;
        let mut swinging = 0_u64;
        if self.mining.on
            && let Some(own) = own
        {
            holding |= 1 << own;
            self.mining.picks.held[own] = true;
            if let Some(since) = self.mining.swinging(now) {
                swinging = (own as u64 + 1) << 32 | since.as_millis().min(0xFFFF_FFFF) as u64;
            }
        }
        if self.mining.on {
            // The boulder just struck shakes a moment.
            let shake_on = self
                .mining
                .swinging(now)
                .filter(|since| *since >= IMPACT && *since < IMPACT + Duration::from_millis(120))
                .and_then(|_| {
                    self.mining
                        .target(Vec3::from_array(origin), self.camera_yaw)
                });
            for (index, rock) in self.mining.rocks.iter().enumerate() {
                let shake = if shake_on == Some(index) {
                    (now.elapsed().as_secs_f32() * 90.0).sin() * 0.8
                } else {
                    0.0
                };
                if let Some(mesh) = rock_mesh {
                    self.object_groups[mesh].push(rock_instance(rock, now, shake));
                }
                if let (Some(glint), Some(until)) = (glint_mesh, rock.glint)
                    && rock.struck()
                {
                    let t = GLINT_WINDOW
                        .saturating_sub(until.saturating_duration_since(now))
                        .as_secs_f32();
                    self.object_groups[glint].push(glint_instance(rock, t));
                }
            }
        }
        if let Some(shown) = self.mining.shown.as_ref() {
            let mesh = match shown.find.kind {
                Kind::Rubble => rubble_mesh,
                Kind::Ore => ore_model(shown.find.id).and_then(|index| ore_meshes[index]),
            };
            if let Some(mesh) = mesh {
                let t = now.saturating_duration_since(shown.since).as_secs_f32();
                self.object_groups[mesh].push(shown_instance(shown, t));
            }
        }
        // Other players' pickaxes, in their hands: their swings are not sent.
        let followed = slots.followed().filter(|&slot| slot < SLOTS);
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot >= SLOTS
                || Some(slot) == own
                || Some(slot) == followed
                || !drawn(entity)
                || !self.looks.toy(slot, TOY)
            {
                continue;
            }
            holding |= 1 << slot;
            self.mining.picks.held[slot] = true;
        }
        HOLDING.store(holding, Ordering::Relaxed);
        SWINGING.store(swinging, Ordering::Relaxed);
    }

    /// Say on the centre of the screen what the toy has to say; its sounds wait for
    /// [`Self::play_mining_sounds`].
    fn say_mining(&mut self) {
        let said: Vec<Say> = self.mining.take_said().collect();
        for say in said {
            let text = match say {
                Say::Centre(text) => text.to_owned(),
                Say::Found(find, count) => {
                    let (kinds, all) = self.mining.log.as_ref().map_or((0, 0), Log::kinds);
                    match find.kind {
                        Kind::Ore if count == 1 => format!(
                            "You dug out {}! New in your log ({kinds} of {all} ores).",
                            find.name
                        ),
                        Kind::Ore => format!("You dug out {} ({count} so far).", find.name),
                        Kind::Rubble => format!("The boulder crumbled: {}.", find.name),
                    }
                }
                Say::Sound(path) => {
                    self.mining_cues.push(path);
                    continue;
                }
                Say::EmptyHands => {
                    self.mining.empty_hands_until = Some(Instant::now() + EMPTY_HANDS_FOR);
                    continue;
                }
            };
            self.chat.receive(
                sjk_client::ServerEventKind::CenterPrint,
                text,
                None,
                Instant::now(),
            );
        }
    }

    /// Play the toy's sounds, the player's own, in both ears.
    pub(crate) fn play_mining_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.mining_cues.clear();
            return;
        };
        if self.mining.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, SOUNDS);
            self.mining.sounds_registered = Some(audio.generation());
        }
        let slot = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.latest_snapshot().player.client_num());
        for path in self.mining_cues.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::Mining,
                path,
                1.0,
                None,
                sjk_audio::SourceId(u32::from(slot)),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            playing: Some(true),
            health: 100,
            origin: [0.0; 3],
            yaw: 0.0,
        }
    }

    /// Flat ground under every spot.
    fn flat(spot: [f32; 3]) -> Option<([f32; 3], [f32; 3])> {
        Some(([spot[0], spot[1], FEET], [0.0, 0.0, 1.0]))
    }

    fn take_out(mining: &mut Mining, now: Instant) {
        mining.toggle();
        mining.update(frame(), now, |_| None);
        assert!(mining.on());
        mining.take_said().for_each(drop);
    }

    /// A boulder of `hits` blows just before the player (who faces +X).
    fn rock_ahead(mining: &mut Mining, now: Instant, hits: u32) {
        mining.rocks.push(Rock {
            at: [40.0, 0.0, FEET],
            normal: [0.0, 0.0, 1.0],
            spin: 0.0,
            rose: now,
            hits,
            left: hits,
            clean: 0,
            glint: None,
            next_glint: None,
        });
    }

    fn press_use(mining: &mut Mining) {
        mining.shape_command(&mut UserCommand {
            buttons: BUTTON_USE,
            ..UserCommand::default()
        });
        mining.shape_command(&mut UserCommand::default());
    }

    /// One swing from `now`, its blow landed; returns when it is over. No new boulder
    /// comes up meanwhile.
    fn swing(mining: &mut Mining, now: Instant) -> Instant {
        press_use(mining);
        mining.update(frame(), now, |_| None);
        mining.update(frame(), now + IMPACT, |_| None);
        let over = now + SWING_TIME;
        mining.update(frame(), over, |_| None);
        assert!(mining.swing.is_none());
        over
    }

    #[test]
    fn the_pickaxe_waits_until_the_player_plays() {
        let mut mining = Mining::default();
        mining.toggle();
        mining.update(Frame::default(), Instant::now(), |_| None);
        assert!(!mining.on());
        assert!(matches!(mining.take_said().next(), Some(Say::Centre(_))));
    }

    #[test]
    fn boulders_come_up_round_the_player_apart_and_in_range() {
        let mut now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        for _ in 0..200 {
            now += Duration::from_millis(100);
            mining.update(frame(), now, flat);
        }
        assert_eq!(mining.rocks.len(), ROCKS);
        for (index, rock) in mining.rocks.iter().enumerate() {
            let (distance, _) = level_way(Vec3::ZERO, Vec3::from_array(rock.at));
            assert!(distance >= ROCK_RANGE.0 - 0.01 && distance <= ROCK_RANGE.1 + 0.01);
            for other in &mining.rocks[index + 1..] {
                assert!(
                    Vec3::from_array(rock.at).distance(Vec3::from_array(other.at)) >= ROCK_APART
                );
            }
        }
        // Nowhere to stand, none comes up.
        let mut bare = Mining::default();
        take_out(&mut bare, now);
        for step in 0..50 {
            bare.update(frame(), now + Duration::from_millis(100 * step), |_| None);
        }
        assert!(bare.rocks.is_empty());
    }

    #[test]
    fn blows_break_a_boulder_for_a_find_and_another_comes_up() {
        let mut now = Instant::now();
        let mut mining = Mining {
            log_read: true,
            ..Mining::default()
        };
        take_out(&mut mining, now);
        rock_ahead(&mut mining, now, 6);
        for blow in 1..=6 {
            // No glint: the next one is pushed off each blow.
            if let Some(rock) = mining.rocks.first_mut() {
                rock.next_glint = Some(now + Duration::from_secs(60));
            }
            now = swing(&mut mining, now);
            if blow < 6 {
                assert_eq!(mining.rocks[0].left, 6 - blow);
            }
        }
        assert!(mining.rocks.is_empty());
        let said: Vec<Say> = mining.take_said().collect();
        assert!(said.iter().any(|say| matches!(say, Say::Found(_, 1))));
        assert!(mining.shown.is_some());
        let log = mining.log.as_ref().expect("a log");
        assert_eq!(FINDS.iter().map(|f| log.count(f.id)).sum::<u32>(), 1);
        // The next comes up 4 to 8 seconds later.
        for _ in 0..90 {
            now += Duration::from_millis(100);
            mining.update(frame(), now, flat);
        }
        assert!(!mining.rocks.is_empty());
    }

    #[test]
    fn a_blow_on_the_glint_is_a_clean_strike_worth_two() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        rock_ahead(&mut mining, now, 9);
        let after = swing(&mut mining, now);
        mining.rocks[0].next_glint = Some(after);
        mining.update(frame(), after, |_| None);
        assert!(mining.rocks[0].glint.is_some());
        swing(&mut mining, after);
        let rock = mining.rocks[0];
        assert_eq!((rock.left, rock.clean), (9 - 3, 1));
        assert!(rock.glint.is_none());
    }

    #[test]
    fn a_blow_reaches_only_a_boulder_close_before_the_player() {
        let now = Instant::now();
        let mut mining = Mining::default();
        rock_ahead(&mut mining, now, 6);
        assert_eq!(mining.target(Vec3::ZERO, 0.0), Some(0));
        // Behind them, out of reach, far below.
        assert_eq!(mining.target(Vec3::ZERO, std::f32::consts::PI), None);
        assert_eq!(mining.target(Vec3::new(-60.0, 0.0, 0.0), 0.0), None);
        assert_eq!(mining.target(Vec3::new(0.0, 0.0, 200.0), 0.0), None);
        // Standing on it, any way.
        assert_eq!(mining.target(Vec3::new(36.0, 0.0, 0.0), 2.0), Some(0));
        // Nothing before them: the swing says nothing in the centre (the last find
        // stays).
        let mut empty = Mining::default();
        take_out(&mut empty, now);
        empty.take_said().for_each(drop);
        swing(&mut empty, now);
        assert!(!empty.take_said().any(|say| matches!(say, Say::Centre(_))));
    }

    #[test]
    fn spam_does_not_swing_faster() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        rock_ahead(&mut mining, now, 9);
        press_use(&mut mining);
        mining.update(frame(), now, |_| None);
        // Presses during the swing are not kept.
        for step in 1..5 {
            press_use(&mut mining);
            mining.update(frame(), now + Duration::from_millis(step * 50), |_| None);
        }
        mining.update(frame(), now + IMPACT, |_| None);
        mining.update(frame(), now + SWING_TIME, |_| None);
        assert!(mining.swing.is_none());
        assert_eq!(mining.rocks[0].hits - mining.rocks[0].left, 1);
    }

    #[test]
    fn attacking_or_getting_hurt_puts_the_pickaxe_away_but_walking_does_not() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        mining.shape_command(&mut UserCommand {
            forward_move: 127,
            ..UserCommand::default()
        });
        mining.update(frame(), now, |_| None);
        assert!(mining.on());
        // The attack button swings the pickaxe, and is kept from the weapon.
        let mut attack = UserCommand {
            buttons: BUTTON_ATTACK,
            ..UserCommand::default()
        };
        mining.shape_command(&mut attack);
        assert_eq!(attack.buttons, 0);
        mining.update(frame(), now, |_| None);
        assert!(mining.on());
        assert!(mining.swing.is_some());
        // An alt attack puts it away.
        mining.shape_command(&mut UserCommand {
            buttons: BUTTON_ALT_ATTACK,
            ..UserCommand::default()
        });
        mining.update(frame(), now, |_| None);
        assert!(!mining.on());
        take_out(&mut mining, now);
        let hurt = Frame {
            health: 80,
            ..frame()
        };
        mining.update(hurt, now, |_| None);
        assert!(!mining.on());
        // Health over the maximum counting down after a spawn is no hit.
        take_out(&mut mining, now);
        let over = |health| Frame { health, ..frame() };
        mining.update(over(105), now, |_| None);
        mining.update(over(104), now, |_| None);
        mining.update(over(103), now, |_| None);
        assert!(mining.on());
        mining.update(over(90), now, |_| None);
        assert!(!mining.on());
    }

    #[test]
    fn boulders_far_behind_or_on_another_map_go() {
        let now = Instant::now();
        let mut mining = Mining::default();
        mining.set_map("mp/ffa1");
        take_out(&mut mining, now);
        rock_ahead(&mut mining, now, 6);
        let away = Frame {
            origin: [2_000.0, 0.0, 0.0],
            ..frame()
        };
        mining.update(away, now, |_| None);
        assert!(mining.rocks.is_empty());
        rock_ahead(&mut mining, now, 6);
        mining.set_map("mp/ffa1");
        assert_eq!(mining.rocks.len(), 1);
        mining.set_map("mp/ffa3");
        assert!(mining.rocks.is_empty());
    }

    #[test]
    fn the_ground_must_be_seen_flat_and_dry() {
        let eye = Vec3::new(0.0, 0.0, 36.0);
        let open = Trace {
            start_solid: false,
            fraction: 1.0,
            end: [0.0; 3],
            normal: None,
            flags: 0,
            contents: 0,
        };
        let floor = |normal: [f32; 3], contents: u32| {
            move |_: [f32; 3], end: [f32; 3], mask: u32| {
                if mask & CONTENTS_LIQUID == 0 {
                    Trace { end, ..open }
                } else {
                    Trace {
                        fraction: 0.2,
                        end: [end[0], end[1], -24.0],
                        normal: Some(normal),
                        contents,
                        ..open
                    }
                }
            }
        };
        let spot = [200.0, 0.0, 0.0];
        assert_eq!(
            ground_at(eye, spot, floor([0.0, 0.0, 1.0], CONTENTS_SOLID)),
            Some(([200.0, 0.0, -24.0], [0.0, 0.0, 1.0]))
        );
        assert_eq!(
            ground_at(eye, spot, floor([0.7, 0.0, 0.7], CONTENTS_SOLID)),
            None
        );
        assert_eq!(ground_at(eye, spot, floor([0.0, 0.0, 1.0], 4)), None);
        // Behind a wall, none.
        let walled = |_: [f32; 3], end: [f32; 3], _: u32| Trace {
            fraction: 0.5,
            end,
            ..open
        };
        assert_eq!(ground_at(eye, spot, walled), None);
    }

    #[test]
    fn choosing_a_weapon_puts_the_pickaxe_away() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        assert!(mining.on());
        mining.take_said().for_each(drop);
        mining.weapon_chosen();
        assert!(!mining.on());
        assert!(
            mining
                .take_said()
                .any(|say| matches!(say, Say::Centre("You put the pickaxe away.")))
        );
        // Away already: nothing said.
        mining.weapon_chosen();
        assert_eq!(mining.take_said().count(), 0);
    }

    #[test]
    fn the_pickaxe_is_laid_straight_in_the_plane_the_body_faces() {
        let forward = Vec3::X;
        // Rolled and tipped to the side: the handle and spike come back into the x-z
        // plane, square to each other, the spike on the side it was.
        let tipped =
            Quat::from_rotation_x(0.6) * Quat::from_rotation_z(0.4) * Quat::from_rotation_y(-0.9);
        let straight = straightened(tipped, forward);
        let handle = straight * Vec3::X;
        let spike = straight * Vec3::NEG_Z;
        assert!(handle.y.abs() < 1e-4 && spike.y.abs() < 1e-4);
        assert!(handle.dot(spike).abs() < 1e-4);
        assert!(spike.dot(tipped * Vec3::NEG_Z) > 0.0);
        // The handle's pitch in the plane is kept.
        let before = tipped * Vec3::X;
        let pitch = |v: Vec3| v.z.atan2(v.x);
        assert!((pitch(handle) - pitch(Vec3::new(before.x, 0.0, before.z))).abs() < 1e-4);
        // Already straight: unchanged.
        let level = Quat::from_rotation_y(0.7);
        assert!(straightened(level, forward).dot(level).abs() > 0.9999);
    }

    #[test]
    fn the_find_table_follows_its_chances() {
        let tally = |clean: u32| {
            let (mut rubble, mut ore) = (0, 0);
            for roll in 0..100 {
                match roll_find(roll, roll * 7, clean, 0).kind {
                    Kind::Rubble => rubble += 1,
                    Kind::Ore => ore += 1,
                }
            }
            (rubble, ore)
        };
        assert_eq!(tally(0), (40, 60));
        assert_eq!(tally(1), (30, 70));
        assert_eq!(tally(2), (20, 80));
        assert_eq!(tally(5), (20, 80));
        // Every find can come up, and ids are distinct.
        for find in &FINDS {
            let roll = if find.kind == Kind::Rubble { 0 } else { 99 };
            assert!(
                (0..1000).any(|pick| roll_find(roll, pick, 0, 0) == find),
                "{}",
                find.id
            );
            assert_eq!(FINDS.iter().filter(|f| f.id == find.id).count(), 1);
        }
        assert_eq!(Log::default().kinds(), (0, 9));
    }

    #[test]
    fn a_clean_strike_lifts_the_ore_a_tier_but_not_into_the_heart() {
        let iron = 0;
        let void = 96 + 86 + 57 + 43 + 28 + 19 + 11;
        let heart = void + 9;
        assert_eq!(roll_find(99, iron, 0, 0).id, "iron_ore");
        assert_eq!(roll_find(99, iron, 1, 0).id, "copper_ore");
        // Two clean strikes lift it no further than one.
        assert_eq!(roll_find(99, iron, 2, 0).id, "copper_ore");
        assert_eq!(roll_find(99, void, 0, 0).id, "void_crystal");
        assert_eq!(roll_find(99, void, 2, 0).id, "void_crystal");
        assert_eq!(roll_find(99, heart, 0, 0).id, HEART);
        assert_eq!(roll_find(99, heart, 1, 0).id, HEART);
    }

    #[test]
    fn the_heart_grows_likelier_the_longer_it_waits() {
        let mut log = Log::default();
        let iron = &FINDS[4];
        for _ in 0..210 {
            log.found(iron);
        }
        assert_eq!(log.heart_wait(), 210);
        assert_eq!(weight(&FINDS[12], log.heart_wait()), 3);
        log.found(&FINDS[12]);
        assert_eq!(log.heart_wait(), 0);
        // Rubble does not count towards it.
        log.found(&FINDS[0]);
        assert_eq!(log.heart_wait(), 0);
    }

    #[test]
    fn the_log_reads_back_what_it_wrote() {
        let mut log = Log::parse("iron_ore 3\nnot a line at all\nrubble 1\n");
        assert_eq!(log.count("iron_ore"), 3);
        assert_eq!(log.found(&FINDS[4]), 4);
        assert_eq!(log.found(&FINDS[9]), 1);
        assert_eq!(Log::parse(&log.text()), log);
        assert_eq!(log.kinds(), (2, 9));
    }

    #[test]
    fn each_ore_has_its_own_model() {
        let ores: Vec<_> = FINDS.iter().filter(|f| f.kind == Kind::Ore).collect();
        assert_eq!(ores.len(), ORE_MODELS.len());
        for (find, model) in ores.iter().zip(&ORE_MODELS) {
            assert_eq!(find.id, model.id);
            assert_eq!(&model.bytes[..4], b"IDP3", "{}", model.id);
        }
        for (_, bytes) in FILES {
            assert!(!bytes.is_empty());
        }
    }

    #[test]
    fn the_own_swing_is_told_to_the_pose_for_the_own_slot_only() {
        SWINGING.store((3 + 1) << 32 | 250, Ordering::Relaxed);
        assert_eq!(swinging(3), Some(250));
        assert_eq!(swinging(2), None);
        SWINGING.store(0, Ordering::Relaxed);
        assert_eq!(swinging(0), None);
        assert!(swing_pose().is_some());
    }

    #[test]
    fn the_find_falls_at_the_feet_before_the_player() {
        let now = Instant::now();
        let mut mining = Mining::default();
        let rock = Rock {
            at: [40.0, 0.0, FEET],
            normal: [0.0, 0.0, 1.0],
            spin: 0.0,
            rose: now,
            hits: 6,
            left: 0,
            clean: 0,
            glint: None,
            next_glint: None,
        };
        mining.break_rock(rock, [0.0; 3], now);
        let shown = mining.shown.expect("shown");
        assert!((shown.to[2] - (FEET + 2.0)).abs() < 1e-4);
        assert!(shown.to[0] < 40.0);
        let end = shown_instance(&shown, FLY.as_secs_f32());
        assert!(Vec3::from_array(end.position).distance(Vec3::from_array(shown.to)) < 1e-3);
    }
}
