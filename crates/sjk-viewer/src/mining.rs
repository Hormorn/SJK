//! Mining, SJK's own toy (`toy_mining`; Slash's design, 04/11/2026, phase 1): the player
//! takes a pickaxe in hand and strikes rock with `+use`. The first blow opens a vein
//! where it lands; six to nine more break it, and it gives rubble or ore, counted in the
//! player's mining log (`mining_log.txt`). Now and then a glint flashes on the vein: a
//! blow that lands while it shines is a clean strike, worth two, and each one makes the
//! vein give better ([`roll_find`]). A broken vein leaves the rock round it spent for a
//! while ([`SPENT_FOR`]), so the player moves on to the next.
//!
//! Unlike Fishing it is no waiting game: the player may walk with the pickaxe out, and
//! the pace is theirs, a swing at a time. Attacking, taking damage or dying puts the
//! pickaxe away; walking away from a vein leaves it.
//!
//! Phase 1 changes nothing at the hub: the finds are a local log, no credits, no
//! shards, no achievements. Selling them (phase 2) is to share the Fishing junk's daily
//! cap (`fishing_sell.rs`), one cap for both, so mining earns nothing more than fishing
//! does alone.
//!
//! No game server knows of it: it travels through the SJK hub as a toy in the player's
//! look ([`TOY`]), so other SJK players see the pickaxe in their hand; players on stock
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
pub(crate) const VEIN: &str = "models/sjk/mining_vein.md3";
pub(crate) const GLINT: &str = "models/sjk/mining_glint.md3";
/// What a vein gives when it gives rubble.
pub(crate) const RUBBLE: &str = "models/sjk/mining_rubble.md3";
pub(crate) const ICON: &str = "gfx/sjk/toy_mining.png";
/// The models, in the order [`GpuState::submit_mining`] finds their meshes (each ore's
/// own in [`ORE_MODELS`]).
pub(crate) const MODELS: [&str; 4] = [PICK, VEIN, GLINT, RUBBLE];
/// A swing through the air.
const SWING: &str = "sound/weapons/melee/swing1.mp3";
/// A blow on the rock, the two taken in turn.
const HIT: [&str; 2] = [
    "sound/effects/stone_bounce.wav",
    "sound/effects/stone_bounce2.wav",
];
/// A clean strike: the pick rings.
const CLEAN: &str = "sound/weapons/flechette/ricochet1.wav";
/// The glint flashing on the vein.
const GLINT_SOUND: &str = "sound/ambience/spark1.wav";
/// The vein breaking.
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
    (VEIN, bundled!("mining_vein.md3")),
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
const SURF_SKY: u32 = 0x2000;
const SURF_NOIMPACT: u32 = 0x0008_0000;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
const PMF_FOLLOW: u16 = 4096;
/// From the player's origin down to their feet (`DEFAULT_MINS_2`).
const FEET: f32 = -24.0;
/// How far from the eye a blow reaches.
pub(crate) const REACH: f32 = 72.0;
/// A swing, from the press to the pick back at rest, and when in it the blow lands.
const SWING_TIME: Duration = Duration::from_millis(750);
const IMPACT: Duration = Duration::from_millis(320);
/// The blows a vein takes, a clean strike counting two.
const VEIN_HITS: (u32, u32) = (6, 9);
/// A blow this near a vein strikes it; farther, it opens another (the first is lost).
const VEIN_NEAR: f32 = 20.0;
/// Walking this far from the vein leaves it.
const VEIN_LEFT: f32 = 160.0;
/// The first glint comes this long after the vein opened, the next this long after the
/// last went (seconds); each shines this long.
const GLINT_FIRST: (f32, f32) = (1.0, 2.5);
const GLINT_GAP: (f32, f32) = (1.5, 4.0);
const GLINT_WINDOW: Duration = Duration::from_millis(800);
/// The rock this near a broken vein is spent this long; at most so many are kept.
const SPENT_NEAR: f32 = 80.0;
pub(crate) const SPENT_FOR: Duration = Duration::from_secs(8 * 60);
const SPENT_MAX: usize = 64;
/// What a broken vein gives flies out and lies at the player's feet this long.
const SHOW_FIND: Duration = Duration::from_millis(1800);
const FLY: Duration = Duration::from_millis(450);
/// The pickaxe's pitch (radians, up positive): at rest, raised for the blow and when it
/// lands.
const PICK_REST: f32 = 0.35;
const PICK_RAISED: f32 = 2.3;
const PICK_STRUCK: f32 = 0.95;
/// The hand holds the pickaxe this far up from its butt (`mining_assets.py`'s grip).
const GRIP: f32 = 5.0;

/// What a broken vein gives.
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

/// The rarest ore's id: bad luck protection raises its weight ([`HEART_PITY`]).
pub(crate) const HEART: &str = "heart_of_the_mountain";
/// Everything a vein can give: SJK's own names.
pub(crate) const FINDS: [Find; 13] = [
    rubble("rubble", "a handful of rubble"),
    rubble("slag", "a lump of slag"),
    rubble("drill_bit", "a worn drill bit"),
    rubble("fossil", "a fossil shell"),
    ore("iron_ore", "a chunk of iron ore", 20),
    ore("copper_ore", "a chunk of copper ore", 18),
    ore("silver_nugget", "a silver nugget", 12),
    ore("glowstone", "a glowstone", 9),
    ore("sunstone", "a sunstone", 6),
    ore("moonstone", "a moonstone", 4),
    ore("star_quartz", "a star quartz", 3),
    ore("void_crystal", "a void crystal", 2),
    ore(HEART, "the Heart of the Mountain", 1),
];

/// The chance a vein gives rubble, in percent, less this much for each clean strike, but
/// never under the floor.
const RUBBLE_CHANCE: u32 = 40;
const CLEAN_RUBBLE_CUT: u32 = 10;
const RUBBLE_FLOOR: u32 = 10;
/// Each clean strike (at most this many) rolls the ore once more, the rarest kept.
const CLEAN_ROLLS: u32 = 2;
/// Bad luck protection for the Heart of the Mountain, as Fishing's for Old Whiskers: its
/// weight rises by one for every this many ores dug since the last one.
const HEART_PITY: u32 = 10;
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

/// Roll a broken vein's find from `roll` and `picks`, uniform numbers, after `clean`
/// clean strikes, `heart_wait` ores after the last Heart: rubble less often for each
/// clean strike, and the ore rolled once more for each (at most [`CLEAN_ROLLS`]), the
/// rarest of the rolls kept.
pub(crate) fn roll_find(roll: u32, picks: [u32; 3], clean: u32, heart_wait: u32) -> &'static Find {
    let rubble = RUBBLE_CHANCE
        .saturating_sub(CLEAN_RUBBLE_CUT.saturating_mul(clean))
        .max(RUBBLE_FLOOR);
    if roll % 100 < rubble {
        return pick_of(Kind::Rubble, picks[0], 0);
    }
    let rolls = 1 + clean.min(CLEAN_ROLLS) as usize;
    picks[..rolls]
        .iter()
        .map(|&pick| pick_of(Kind::Ore, pick, heart_wait))
        .min_by_key(|find| find.weight)
        .unwrap_or(&FINDS[0])
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

    /// A number in `range`, seconds.
    fn seconds(&mut self, (low, high): (f32, f32)) -> Duration {
        let unit = self.next() as f32 / u32::MAX as f32;
        Duration::from_secs_f32(low + (high - low) * unit)
    }

    /// A whole number from `low` to `high`, both in.
    fn between(&mut self, (low, high): (u32, u32)) -> u32 {
        low + self.next() % (high - low + 1)
    }
}

/// The vein being broken.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Vein {
    /// Where it sits on the rock, and the way the rock faces there.
    at: [f32; 3],
    normal: [f32; 3],
    /// The blows it takes in all, and still.
    hits: u32,
    left: u32,
    /// The clean strikes it took.
    clean: u32,
    /// A glint shines on it until then; or the next comes then.
    glint: Option<Instant>,
    next_glint: Instant,
}

/// A swing under way: it started then, and its blow has landed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Swing {
    started: Instant,
    landed: bool,
}

/// A broken vein's find, flying out of the rock and lying at the player's feet until
/// then.
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
    /// Where they stand (their origin).
    pub(crate) origin: [f32; 3],
}

/// What a blow found: where it landed on the rock and the way the rock faces there; or
/// it struck what no pick digs (the sky, a forcefield) or nothing in reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Blow {
    Rock { at: [f32; 3], normal: [f32; 3] },
    Undiggable,
    Air,
}

/// What the toy asks the frame to say or play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Say {
    /// A line on the centre of the screen.
    Centre(&'static str),
    /// A find, with how many the log now holds of it.
    Found(&'static Find, u32),
    Sound(&'static str),
    /// A console command to run (`weapmelee`: empty hands for the pickaxe).
    Command(&'static str),
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

/// Each holder's pickaxe this frame, by slot: its pitch (radians, up positive).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Picks {
    pub(crate) pitch: [Option<f32>; SLOTS],
}

impl Default for Picks {
    fn default() -> Self {
        Self {
            pitch: [None; SLOTS],
        }
    }
}

/// The toy's state.
pub(crate) struct Mining {
    /// The pickaxe is in hand.
    on: bool,
    /// `toy_mining` was asked for.
    asked: bool,
    swing: Option<Swing>,
    vein: Option<Vein>,
    /// Where veins broke, and until when the rock there is spent.
    spent: Vec<([f32; 3], Instant)>,
    /// The map the spent rock is on.
    map: String,
    shown: Option<Shown>,
    /// The blows struck, for the two hit sounds in turn.
    blows: u32,
    /// `+use` went down since the last frame.
    use_pressed: bool,
    use_held: bool,
    /// The player attacked since the last frame.
    attacked: bool,
    health: Option<i32>,
    /// The mining log; `log_read`, it was read from the profile's folder (what was dug
    /// before that is counted in when it is).
    log: Option<Log>,
    log_read: bool,
    dice: Dice,
    said: Vec<Say>,
    /// Each holder's pickaxe, for the actors to draw ([`pick_in_hand`]).
    pub(crate) picks: Picks,
    sounds_registered: Option<u64>,
}

impl Default for Mining {
    fn default() -> Self {
        Self {
            on: false,
            asked: false,
            swing: None,
            vein: None,
            spent: Vec::new(),
            map: String::new(),
            shown: None,
            blows: 0,
            use_pressed: false,
            use_held: false,
            attacked: false,
            health: None,
            log: None,
            log_read: false,
            dice: Dice::seeded(),
            said: Vec::with_capacity(4),
            picks: Picks::default(),
            sounds_registered: None,
        }
    }
}

/// The pickaxe's pitch `since` a swing started at `now`: raised, brought down to the
/// blow at [`IMPACT`], back to rest by [`SWING_TIME`].
fn swing_pitch(since: Duration) -> f32 {
    let t = since.as_secs_f32();
    let raise = IMPACT.as_secs_f32() * 0.55;
    let impact = IMPACT.as_secs_f32();
    let end = SWING_TIME.as_secs_f32();
    let ease = |x: f32| x * x * (3.0 - 2.0 * x);
    if t < raise {
        PICK_REST + (PICK_RAISED - PICK_REST) * ease(t / raise)
    } else if t < impact {
        let x = (t - raise) / (impact - raise);
        PICK_RAISED + (PICK_STRUCK - PICK_RAISED) * x * x
    } else if t < end {
        PICK_STRUCK + (PICK_REST - PICK_STRUCK) * ease((t - impact) / (end - impact))
    } else {
        PICK_REST
    }
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

    /// Note `+use`, and any attack, in `command`.
    pub(crate) fn shape_command(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
        if command.buttons & (BUTTON_ATTACK | BUTTON_ALT_ATTACK) != 0 {
            self.attacked = true;
        }
    }

    pub(crate) fn take_said(&mut self) -> std::vec::Drain<'_, Say> {
        self.said.drain(..)
    }

    pub(crate) fn put_away(&mut self) {
        self.on = false;
        self.asked = false;
        self.swing = None;
        self.vein = None;
    }

    /// The map being played is `map`: rock spent on another is not spent here.
    pub(crate) fn set_map(&mut self, map: &str) {
        if self.map != map {
            map.clone_into(&mut self.map);
            self.spent.clear();
            self.vein = None;
            self.shown = None;
        }
    }

    /// The pickaxe's pitch now.
    fn pitch(&self, now: Instant) -> f32 {
        self.swing.map_or(PICK_REST, |swing| {
            swing_pitch(now.saturating_duration_since(swing.started))
        })
    }

    /// Move the toy on to `now`. `blow` finds what a blow landing now strikes.
    pub(crate) fn update(&mut self, frame: Frame, now: Instant, blow: impl FnOnce() -> Blow) {
        let used = std::mem::take(&mut self.use_pressed);
        let attacked = std::mem::take(&mut self.attacked);
        let health = self.health.replace(frame.health);
        self.spent.retain(|(_, until)| now < *until);
        if self
            .shown
            .is_some_and(|shown| now >= shown.since + SHOW_FIND)
        {
            self.shown = None;
        }
        if std::mem::take(&mut self.asked) && !self.on {
            if frame.playing == Some(true) {
                self.on = true;
                self.said.push(Say::Command("weapmelee"));
                self.said.push(Say::Centre(
                    "Pickaxe out: strike rock with Use. Strike as it glints!",
                ));
            } else {
                self.said.push(Say::Centre("Mining waits until you play."));
            }
            return;
        }
        if !self.on {
            return;
        }
        let hurt = health.is_some_and(|before| frame.health < before);
        if frame.playing != Some(true) || attacked || hurt {
            self.put_away();
            if frame.playing == Some(true) {
                self.said.push(Say::Centre("You put the pickaxe away."));
            }
            return;
        }
        let origin = Vec3::from_array(frame.origin);
        if self
            .vein
            .is_some_and(|vein| Vec3::from_array(vein.at).distance(origin) > VEIN_LEFT)
        {
            self.vein = None;
        }
        // The glint: it shines a while, then the next waits.
        if let Some(vein) = self.vein.as_mut() {
            match vein.glint {
                Some(until) if now >= until => {
                    vein.glint = None;
                    vein.next_glint = now + self.dice.seconds(GLINT_GAP);
                }
                None if now >= vein.next_glint => {
                    vein.glint = Some(now + GLINT_WINDOW);
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
        if let Some(swing) = self.swing.as_mut() {
            if !swing.landed && now >= swing.started + IMPACT {
                swing.landed = true;
                self.strike(blow(), frame.origin, now);
            }
            if self
                .swing
                .is_some_and(|swing| now >= swing.started + SWING_TIME)
            {
                self.swing = None;
            }
        }
    }

    /// A blow lands on what `blow` found, the player standing at `origin`.
    fn strike(&mut self, blow: Blow, origin: [f32; 3], now: Instant) {
        let (at, normal) = match blow {
            Blow::Rock { at, normal } => (at, normal),
            Blow::Undiggable => {
                self.said.push(Say::Sound(CLEAN));
                self.said.push(Say::Centre("Your pick bounces off."));
                return;
            }
            Blow::Air => {
                self.said
                    .push(Say::Centre("Nothing to strike: get closer to the rock."));
                return;
            }
        };
        self.blows = self.blows.wrapping_add(1);
        let hit = HIT[self.blows as usize % HIT.len()];
        let point = Vec3::from_array(at);
        if self
            .spent
            .iter()
            .any(|(spent, _)| Vec3::from_array(*spent).distance(point) < SPENT_NEAR)
        {
            self.said.push(Say::Sound(hit));
            self.said
                .push(Say::Centre("This rock is spent. Try another spot."));
            return;
        }
        let near = self
            .vein
            .is_some_and(|vein| Vec3::from_array(vein.at).distance(point) < VEIN_NEAR);
        if !near {
            let hits = self.dice.between(VEIN_HITS);
            self.vein = Some(Vein {
                at,
                normal,
                hits,
                left: hits,
                clean: 0,
                glint: None,
                next_glint: now + self.dice.seconds(GLINT_FIRST),
            });
        }
        let Some(vein) = self.vein.as_mut() else {
            return;
        };
        if vein.glint.is_some_and(|until| now < until) {
            vein.clean += 1;
            vein.left = vein.left.saturating_sub(2);
            vein.glint = None;
            vein.next_glint = now + self.dice.seconds(GLINT_GAP);
            self.said.push(Say::Sound(CLEAN));
            self.said.push(Say::Centre("Clean strike!"));
        } else {
            vein.left = vein.left.saturating_sub(1);
            self.said.push(Say::Sound(hit));
        }
        if vein.left == 0 {
            let vein = *vein;
            self.vein = None;
            self.break_vein(vein, origin, now);
        }
    }

    fn break_vein(&mut self, vein: Vein, origin: [f32; 3], now: Instant) {
        let heart_wait = self.log.as_ref().map_or(0, Log::heart_wait);
        let roll = self.dice.next();
        let picks = [self.dice.next(), self.dice.next(), self.dice.next()];
        let find = roll_find(roll, picks, vein.clean, heart_wait);
        let count = self.log.get_or_insert_with(Log::default).found(find);
        self.said.push(Say::Sound(BREAK));
        if find.id == HEART {
            self.said.push(Say::Sound(RARE));
        }
        self.said.push(Say::Found(find, count));
        if self.spent.len() >= SPENT_MAX {
            self.spent.remove(0);
        }
        self.spent.push((vein.at, now + SPENT_FOR));
        // It falls out of the rock to the ground before the player.
        let from = Vec3::from_array(vein.at) + Vec3::from_array(vein.normal) * 4.0;
        let feet = Vec3::from_array(origin) + Vec3::Z * FEET;
        let toward = (feet - from).truncate().normalize_or_zero();
        let to = (from + (toward * 14.0).extend(0.0))
            .truncate()
            .extend(feet.z + 2.0);
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

/// What a blow from `eye` along `forward` strikes within [`REACH`]: `trace` sweeps a point
/// and says the fraction, the end, the hit plane's normal and the surface flags.
pub(crate) fn blow_from(
    eye: Vec3,
    forward: Vec3,
    trace: impl FnOnce([f32; 3], [f32; 3]) -> (f32, [f32; 3], Option<[f32; 3]>, u32),
) -> Blow {
    let end = eye + forward * REACH;
    let (fraction, at, normal, flags) = trace(eye.to_array(), end.to_array());
    match normal {
        Some(_) if fraction >= 1.0 => Blow::Air,
        None => Blow::Air,
        Some(_) if flags & (SURF_SKY | SURF_NOIMPACT) != 0 => Blow::Undiggable,
        Some(normal) => Blow::Rock { at, normal },
    }
}

/// Who holds a pickaxe this frame (bits by slot), for the pose ([`holding_pick`]).
static HOLDING: AtomicU64 = AtomicU64::new(0);

/// Whether the player in `slot` holds a pickaxe this frame: their torso takes the rod's
/// pose, both hands before the chest (`fishing::hold_pose`, `actor_pose_steps.rs`).
pub(crate) fn holding_pick(slot: usize) -> bool {
    slot < SLOTS && HOLDING.load(Ordering::Relaxed) & (1 << slot) != 0
}

/// The pickaxe in the right hand of a body posed with `matrices`, placed at `transform`:
/// held near its butt, its handle pointing ahead where the body faces, raised by `pitch`
/// (radians).
pub(crate) fn pick_in_hand(
    preview: &crate::PlayerPreview,
    matrices: &[[[f32; 4]; 3]],
    transform: sjk_runtime::Transform,
    pitch: f32,
) -> Option<ActorInstance> {
    let hand = crate::saber::attachment_from_matrices(preview, matrices, crate::saber::HAND_BOLT)?;
    let rotation = crate::weapon_view::actor_world_rotation(transform.rotation);
    let (grip, _) =
        crate::saber::world_attachment(Vec3::from_array(transform.translation), rotation, hand);
    let forward = Quat::from_array(transform.rotation) * Vec3::X;
    let yaw = forward.y.atan2(forward.x);
    let tilt = Quat::from_rotation_z(yaw) * Quat::from_rotation_y(-pitch);
    let butt = grip - tilt * Vec3::X * GRIP;
    Some(ActorInstance::new(
        butt.to_array(),
        tilt.to_array(),
        [1.0; 3],
    ))
}

/// Where the own pickaxe is held in first person, in the camera's frame (ahead, left, up)
/// from the eye: low on the right, as a weapon is.
const VIEW_GRIP: Vec3 = Vec3::new(8.0, -9.0, -14.0);
/// How far in towards the middle of the view the own pickaxe points.
const VIEW_IN: f32 = 0.22;
/// Its pitch in first person is the hand's less this: the view looks down on it.
const VIEW_DROP: f32 = 0.25;

/// The own pickaxe in first person, where the body's hand is not drawn: held from the
/// eye at `camera` looking `yaw` and `pitch` (radians, up positive), the pick raised by
/// `pick` (radians).
pub(crate) fn pick_in_view(camera: Vec3, yaw: f32, pitch: f32, pick: f32) -> ActorInstance {
    let view = Quat::from_rotation_z(yaw) * Quat::from_rotation_y(-pitch);
    let tilt = view * Quat::from_rotation_z(VIEW_IN) * Quat::from_rotation_y(-(pick - VIEW_DROP));
    let butt = camera + view * VIEW_GRIP - tilt * Vec3::X * GRIP;
    ActorInstance::new(butt.to_array(), tilt.to_array(), [1.0; 3])
}

/// The vein at `vein`, shrunk as it is broken.
fn vein_instance(vein: &Vein) -> ActorInstance {
    let normal = Vec3::from_array(vein.normal).normalize_or(Vec3::X);
    let rotation = Quat::from_rotation_arc(Vec3::X, normal);
    let scale = 0.55 + 0.45 * vein.left as f32 / vein.hits.max(1) as f32;
    ActorInstance::new(vein.at, rotation.to_array(), [scale; 3])
}

/// The glint on `vein`, turning and swelling `t` seconds into its shine.
fn glint_instance(vein: &Vein, t: f32) -> ActorInstance {
    let normal = Vec3::from_array(vein.normal).normalize_or(Vec3::X);
    let at = Vec3::from_array(vein.at) + normal * 5.0;
    let window = GLINT_WINDOW.as_secs_f32();
    let swell = (t / window * std::f32::consts::PI).sin().max(0.0);
    let scale = 0.4 + 1.2 * swell;
    ActorInstance::new(
        at.to_array(),
        Quat::from_rotation_z(t * 3.0).to_array(),
        [scale; 3],
    )
}

/// A broken vein's find `t` seconds after it broke: flying out in an arc, then lying
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

    /// The toy's frame: the own pickaxe's logic, then the vein and every pickaxe (the
    /// own and other SJK players' whose look has it), and what it says.
    pub(crate) fn submit_mining(&mut self, now: Instant) {
        self.mining_frame(now);
        self.say_mining();
    }

    fn mining_frame(&mut self, now: Instant) {
        self.mining.picks = Picks::default();
        let Some(session) = self.live_session.as_ref() else {
            self.mining.update(Frame::default(), now, || Blow::Air);
            HOLDING.store(0, Ordering::Relaxed);
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
        };
        let eye = Vec3::from_array(origin) + Vec3::Z * player.view_height() as f32;
        let (yaw, pitch) = (self.camera_yaw, self.camera_pitch);
        let forward = Vec3::new(
            yaw.cos() * pitch.cos(),
            yaw.sin() * pitch.cos(),
            pitch.sin(),
        );
        let bsp = &self.bsp;
        self.mining.update(frame, now, || {
            blow_from(eye, forward, |start, end| {
                let trace = bsp.trace_box(start, end, sjk_bsp::Aabb::POINT, CONTENTS_SOLID);
                if trace.start_solid {
                    return (1.0, end, None, 0);
                }
                (
                    trace.fraction,
                    trace.end_position,
                    trace.plane.map(|plane| plane.normal),
                    trace.surface_flags,
                )
            })
        });
        let broke = self
            .mining
            .said
            .iter()
            .any(|say| matches!(say, Say::Found(..)));
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
        let [_, vein_mesh, glint_mesh, rubble_mesh] = MODELS.map(mesh_of);
        let ore_meshes = ORE_MODELS.map(|ore| mesh_of(ore.model));
        let mut holding = 0_u64;
        if self.mining.on
            && let Some(own) = own
        {
            holding |= 1 << own;
            self.mining.picks.pitch[own] = Some(self.mining.pitch(now));
        }
        if let (Some(mesh), Some(vein)) = (vein_mesh, self.mining.vein.as_ref()) {
            self.object_groups[mesh].push(vein_instance(vein));
            if let (Some(glint), Some(until)) = (glint_mesh, vein.glint) {
                let t = GLINT_WINDOW
                    .saturating_sub(until.saturating_duration_since(now))
                    .as_secs_f32();
                self.object_groups[glint].push(glint_instance(vein, t));
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
        // Other players' pickaxes, at rest: their swings are not sent.
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
            self.mining.picks.pitch[slot] = Some(PICK_REST);
        }
        HOLDING.store(holding, Ordering::Relaxed);
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
                        Kind::Rubble => format!("The vein crumbled: {}.", find.name),
                    }
                }
                Say::Sound(path) => {
                    self.mining_cues.push(path);
                    continue;
                }
                Say::Command(command) => {
                    if let Some(console) = self.console.as_mut()
                        && let Err(error) = console.queue_command(command)
                    {
                        crate::log::progress(format_args!("mining: {command}: {error}"));
                    }
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
        }
    }

    const ROCK: Blow = Blow::Rock {
        at: [40.0, 0.0, 20.0],
        normal: [-1.0, 0.0, 0.0],
    };

    fn take_out(mining: &mut Mining, now: Instant) {
        mining.toggle();
        mining.update(frame(), now, || Blow::Air);
        assert!(mining.on());
        mining.take_said().for_each(drop);
    }

    fn press_use(mining: &mut Mining) {
        mining.shape_command(&UserCommand {
            buttons: BUTTON_USE,
            ..UserCommand::default()
        });
        mining.shape_command(&UserCommand::default());
    }

    /// One swing at `blow` from `now`, its blow landed; returns when it is over.
    fn swing(mining: &mut Mining, now: Instant, blow: Blow) -> Instant {
        press_use(mining);
        mining.update(frame(), now, || unreachable!("no blow yet"));
        mining.update(frame(), now + IMPACT, || blow);
        let over = now + SWING_TIME;
        mining.update(frame(), over, || unreachable!("one blow a swing"));
        assert!(mining.swing.is_none());
        over
    }

    #[test]
    fn the_pickaxe_waits_until_the_player_plays() {
        let mut mining = Mining::default();
        mining.toggle();
        mining.update(Frame::default(), Instant::now(), || Blow::Air);
        assert!(!mining.on());
        assert!(matches!(mining.take_said().next(), Some(Say::Centre(_))));
    }

    #[test]
    fn a_blow_opens_a_vein_and_enough_break_it() {
        let mut now = Instant::now();
        let mut mining = Mining {
            log_read: true,
            ..Mining::default()
        };
        take_out(&mut mining, now);
        now = swing(&mut mining, now, ROCK);
        let vein = mining.vein.expect("a vein");
        assert_eq!(vein.left, vein.hits - 1);
        assert!((VEIN_HITS.0..=VEIN_HITS.1).contains(&vein.hits));
        // No glint till it breaks: the next one is pushed off each blow.
        for _ in 1..vein.hits {
            if let Some(vein) = mining.vein.as_mut() {
                vein.next_glint = now + Duration::from_secs(60);
            }
            now = swing(&mut mining, now, ROCK);
        }
        assert!(mining.vein.is_none());
        let said: Vec<Say> = mining.take_said().collect();
        assert!(said.iter().any(|say| matches!(say, Say::Found(_, 1))));
        assert!(mining.shown.is_some());
        let log = mining.log.as_ref().expect("a log");
        assert_eq!(FINDS.iter().map(|f| log.count(f.id)).sum::<u32>(), 1);
        // The rock round it is spent.
        swing(&mut mining, now, ROCK);
        assert!(mining.vein.is_none());
        assert!(
            mining
                .take_said()
                .any(|say| say == Say::Centre("This rock is spent. Try another spot."))
        );
    }

    #[test]
    fn a_blow_on_the_glint_is_a_clean_strike_worth_two() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        let after = swing(&mut mining, now, ROCK);
        let left = mining.vein.expect("a vein").left;
        if let Some(vein) = mining.vein.as_mut() {
            vein.next_glint = after;
        }
        mining.update(frame(), after, || Blow::Air);
        assert!(mining.vein.expect("a vein").glint.is_some());
        swing(&mut mining, after, ROCK);
        let vein = mining.vein.expect("a vein");
        assert_eq!((vein.left, vein.clean), (left - 2, 1));
        assert!(vein.glint.is_none());
    }

    #[test]
    fn spam_does_not_swing_faster() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        press_use(&mut mining);
        mining.update(frame(), now, || Blow::Air);
        // Presses during the swing are not kept.
        for step in 1..5 {
            press_use(&mut mining);
            mining.update(frame(), now + Duration::from_millis(step * 50), || ROCK);
        }
        mining.update(frame(), now + IMPACT, || ROCK);
        mining.update(frame(), now + SWING_TIME, || Blow::Air);
        assert!(mining.swing.is_none());
        assert_eq!(mining.vein.map(|v| v.hits - v.left), Some(1));
    }

    #[test]
    fn a_blow_elsewhere_opens_another_vein_and_walking_off_leaves_it() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        let after = swing(&mut mining, now, ROCK);
        let other = Blow::Rock {
            at: [40.0, 60.0, 20.0],
            normal: [-1.0, 0.0, 0.0],
        };
        swing(&mut mining, after, other);
        let vein = mining.vein.expect("a vein");
        assert_eq!((vein.at, vein.left), ([40.0, 60.0, 20.0], vein.hits - 1));
        let away = Frame {
            origin: [400.0, 0.0, 0.0],
            ..frame()
        };
        mining.update(away, after + SWING_TIME, || Blow::Air);
        assert!(mining.on());
        assert!(mining.vein.is_none());
    }

    #[test]
    fn attacking_or_getting_hurt_puts_the_pickaxe_away_but_walking_does_not() {
        let now = Instant::now();
        let mut mining = Mining::default();
        take_out(&mut mining, now);
        mining.shape_command(&UserCommand {
            forward_move: 127,
            ..UserCommand::default()
        });
        mining.update(frame(), now, || Blow::Air);
        assert!(mining.on());
        mining.shape_command(&UserCommand {
            buttons: BUTTON_ATTACK,
            ..UserCommand::default()
        });
        mining.update(frame(), now, || Blow::Air);
        assert!(!mining.on());
        take_out(&mut mining, now);
        let hurt = Frame {
            health: 80,
            ..frame()
        };
        mining.update(hurt, now, || Blow::Air);
        assert!(!mining.on());
    }

    #[test]
    fn spent_rock_recovers_and_another_map_has_none() {
        let now = Instant::now();
        let mut mining = Mining::default();
        mining.set_map("mp/ffa1");
        mining.spent.push(([40.0, 0.0, 20.0], now + SPENT_FOR));
        take_out(&mut mining, now);
        swing(&mut mining, now, ROCK);
        assert!(mining.vein.is_none());
        let later = now + SPENT_FOR;
        swing(&mut mining, later, ROCK);
        assert!(mining.vein.is_some());
        mining.spent.push(([40.0, 0.0, 20.0], later + SPENT_FOR));
        mining.set_map("mp/ffa3");
        assert!(mining.spent.is_empty() && mining.vein.is_none());
    }

    #[test]
    fn the_find_table_follows_its_chances() {
        let tally = |clean: u32| {
            let (mut rubble, mut ore) = (0, 0);
            for roll in 0..100 {
                match roll_find(roll, [roll * 7, roll * 13, roll * 31], clean, 0).kind {
                    Kind::Rubble => rubble += 1,
                    Kind::Ore => ore += 1,
                }
            }
            (rubble, ore)
        };
        assert_eq!(tally(0), (40, 60));
        assert_eq!(tally(1), (30, 70));
        assert_eq!(tally(5), (10, 90));
        // Every find can come up, and ids are distinct.
        for find in &FINDS {
            let roll = if find.kind == Kind::Rubble { 0 } else { 99 };
            assert!(
                (0..1000).any(|pick| roll_find(roll, [pick; 3], 0, 0) == find),
                "{}",
                find.id
            );
            assert_eq!(FINDS.iter().filter(|f| f.id == find.id).count(), 1);
        }
        assert_eq!(Log::default().kinds(), (0, 9));
    }

    #[test]
    fn clean_strikes_keep_the_rarest_of_more_rolls() {
        // Iron from the first pick, a glowstone from the second, the Heart from the third.
        let iron = 0;
        let glowstone = 20 + 18 + 12;
        let heart = 74;
        assert_eq!(roll_find(99, [iron, glowstone, heart], 0, 0).id, "iron_ore");
        assert_eq!(
            roll_find(99, [iron, glowstone, heart], 1, 0).id,
            "glowstone"
        );
        assert_eq!(roll_find(99, [iron, glowstone, heart], 3, 0).id, HEART);
    }

    #[test]
    fn the_heart_grows_likelier_the_longer_it_waits() {
        let mut log = Log::default();
        let iron = &FINDS[4];
        for _ in 0..25 {
            log.found(iron);
        }
        assert_eq!(log.heart_wait(), 25);
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
    fn a_blow_strikes_rock_in_reach_but_not_the_sky() {
        let eye = Vec3::new(0.0, 0.0, 36.0);
        let rock = blow_from(eye, Vec3::X, |_, _| {
            (0.5, [36.0, 0.0, 36.0], Some([-1.0, 0.0, 0.0]), 0)
        });
        assert_eq!(
            rock,
            Blow::Rock {
                at: [36.0, 0.0, 36.0],
                normal: [-1.0, 0.0, 0.0]
            }
        );
        let sky = blow_from(eye, Vec3::X, |_, _| {
            (0.5, [36.0, 0.0, 36.0], Some([-1.0, 0.0, 0.0]), SURF_SKY)
        });
        assert_eq!(sky, Blow::Undiggable);
        let air = blow_from(eye, Vec3::X, |_, end| (1.0, end, None, 0));
        assert_eq!(air, Blow::Air);
    }

    #[test]
    fn the_swing_raises_the_pick_then_brings_it_down_to_the_blow() {
        assert!((swing_pitch(Duration::ZERO) - PICK_REST).abs() < 1e-4);
        let raised = swing_pitch(Duration::from_secs_f32(IMPACT.as_secs_f32() * 0.55));
        assert!((raised - PICK_RAISED).abs() < 1e-3);
        assert!((swing_pitch(IMPACT) - PICK_STRUCK).abs() < 1e-3);
        assert!((swing_pitch(SWING_TIME) - PICK_REST).abs() < 1e-4);
    }

    #[test]
    fn the_find_falls_at_the_feet_before_the_player() {
        let now = Instant::now();
        let mut mining = Mining::default();
        let vein = Vein {
            at: [40.0, 0.0, 20.0],
            normal: [-1.0, 0.0, 0.0],
            hits: 6,
            left: 0,
            clean: 0,
            glint: None,
            next_glint: now,
        };
        mining.break_vein(vein, [0.0; 3], now);
        let shown = mining.shown.expect("shown");
        assert!((shown.to[2] - (FEET + 2.0)).abs() < 1e-4);
        assert!(shown.to[0] < shown.from[0]);
        let end = shown_instance(&shown, FLY.as_secs_f32());
        assert!(Vec3::from_array(end.position).distance(Vec3::from_array(shown.to)) < 1e-3);
    }
}
