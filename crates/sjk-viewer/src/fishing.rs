//! Fishing, SJK's own toy (`toy_fishing`; Sanji's idea from #ideas, 29/10/2026, phase 1
//! of Slash's design): facing water, the player props a rod beside them, sets a bucket at
//! their feet and casts. Every 12 to 25 seconds the float bobs under (a bite) and `+use`
//! within about two seconds hooks the catch: nothing, junk or a fish, counted in the
//! player's fish log (`fishing_log.txt`). Moving, jumping, attacking, taking damage or
//! dying puts the rod away.
//!
//! An Echo (holocron) the hub drops while the player fishes does not pop up at once: it
//! bites as an Echo bite ([`Bite::echo`], its own sound, a longer window) and hooking it
//! brings its card. Missed three times, or the rod put away, the card comes as it always does.
//! Phase 1 changes nothing at the hub: the Echo is the player's from the moment the hub
//! drops it, by the usual clock and caps, and the fish are a local log, no credits, no
//! shards. So fishing earns nothing that playing does not. Since 04/11/2026 a bite
//! hooked in time is a play signal (`crate::holocrons::signals::bite`), so the hub counts
//! the time of a player standing at the water toward holocrons as it does a moving one's.
//!
//! No game server knows of it: it travels through the SJK hub as a toy in the player's
//! look ([`TOY`]), so other SJK players see the rod, the bucket and the float (where
//! their client finds water in front of that player); players on stock clients see a
//! player standing still.
//!
//! The models, picture, shader and icon are bundled ([`FILES`], made by
//! `scripts/fishing_assets.py`) and mounted below all game data. The sounds are retail
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
pub(crate) const COMMAND: &str = "toy_fishing";
/// The toy's id in a look's `toys` (`PROTOCOL.md`, "Looks").
pub(crate) const TOY: &str = "fishing";
pub(crate) const ROD: &str = "models/sjk/fishing_rod.md3";
pub(crate) const BUCKET: &str = "models/sjk/fishing_bucket.md3";
pub(crate) const FLOAT: &str = "models/sjk/fishing_float.md3";
pub(crate) const LINE: &str = "models/sjk/fishing_line.md3";
/// The pond a cast makes where there is no water.
pub(crate) const POND: &str = "models/sjk/fishing_pond.md3";
pub(crate) const ICON: &str = "gfx/sjk/toy_fishing.png";
/// The models, in the order [`GpuState::submit_fishing`] finds their meshes (each fish's
/// own in [`FISH_MODELS`]).
pub(crate) const MODELS: [&str; 5] = [ROD, BUCKET, FLOAT, LINE, POND];
/// The swing of the cast.
const CAST: &str = "sound/weapons/melee/swing1.mp3";
/// The float landing on the water.
const LAND: &str = "sound/player/footsteps/water_wade_01.mp3";
/// A fish biting: the float pulled under.
const BITE: &str = "sound/player/bodyfall_water1.mp3";
/// An Echo biting.
const ECHO_BITE: &str = "sound/player/holocron.wav";
/// The catch out of the water.
const CATCH: &str = "sound/player/watr_out.wav";
const SOUNDS: [&str; 5] = [CAST, LAND, BITE, ECHO_BITE, CATCH];
/// The fish log, in the profile's folder: `<catch id> <count>` a line.
pub(crate) const LOG_FILE: &str = "fishing_log.txt";
/// When a bite is shown as well as heard (`cg_fishingBiteCue`, for players with the
/// sound off): 0 never, 1 when the toy's sounds cannot be heard, 2 always.
pub(crate) const CUE_CVAR: &str = "cg_fishingBiteCue";
pub(crate) const CUE_DEFAULT: i64 = 1;
/// What the centre of the screen says as a fish bites, an Echo bites, or a fish's bite
/// goes unhooked, when the cue is shown.
const CUE_BITE: &str = "Bite! Press Use!";
const CUE_ECHO: &str = "Something shines under the float! Press Use!";
const CUE_MISSED: &str = "It got away.";
/// The "!" over the float: how high over the water, its size at 1080 lines and how
/// fast it pulses.
const CUE_RISE: f32 = 18.0;
const CUE_SIZE: f32 = 64.0;
const CUE_PULSE: f32 = 9.0;

/// Whether a bite is shown on screen: by `mode` ([`CUE_CVAR`]) and whether the toy's
/// sounds are `heard` now.
pub(crate) fn cue_shown(mode: i64, heard: bool) -> bool {
    match mode {
        ..=0 => false,
        1 => !heard,
        _ => true,
    }
}

/// A bundled file's bytes, by its name in `assets/fishing`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/fishing/", $name))
    };
}

/// One fish's own model (`fishing_assets.py`'s `FISH`): its catch id, its game path and
/// bundled bytes, and its length from the mouth (its origin, where the line holds it)
/// back along -X to the tip of its tail.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FishModel {
    pub(crate) id: &'static str,
    pub(crate) model: &'static str,
    bytes: &'static [u8],
    length: f32,
}

macro_rules! fish_models {
    ($(($id:literal, $length:literal)),* $(,)?) => {
        [$(FishModel {
            id: $id,
            model: concat!("models/sjk/fishing_fish_", $id, ".md3"),
            bytes: include_bytes!(concat!("../assets/fishing/fishing_fish_", $id, ".md3")),
            length: $length,
        }),*]
    };
}

/// Each fish of [`CATCHES`] its own model, in the same order.
pub(crate) const FISH_MODELS: [FishModel; 12] = fish_models![
    ("silver_darter", 9.0),
    ("mud_snapper", 11.0),
    ("swamp_gulper", 12.0),
    ("striped_bonefish", 13.0),
    ("glow_perch", 10.0),
    ("spiny_pike", 20.0),
    ("sun_carp", 14.0),
    ("moon_eel", 26.0),
    ("kyber_minnow", 6.0),
    ("ghost_ray", 17.0),
    ("temple_koi", 15.0),
    ("old_whiskers", 24.0),
];

/// The model of the fish `id`, and its place in [`FISH_MODELS`].
fn fish_model(id: &str) -> Option<(usize, &'static FishModel)> {
    FISH_MODELS
        .iter()
        .enumerate()
        .find(|(_, fish)| fish.id == id)
}

/// The bundled files at their game paths (and the fish, [`FISH_MODELS`]).
const FILES: [(&str, &[u8]); 9] = [
    (ROD, bundled!("fishing_rod.md3")),
    (BUCKET, bundled!("fishing_bucket.md3")),
    (FLOAT, bundled!("fishing_float.md3")),
    (LINE, bundled!("fishing_line.md3")),
    (POND, bundled!("fishing_pond.md3")),
    ("models/sjk/fishing.png", bundled!("fishing.png")),
    ("models/sjk/fishing_fish.png", bundled!("fishing_fish.png")),
    ("shaders/sjk_fishing.shader", bundled!("fishing.shader")),
    (ICON, bundled!("toy_fishing.png")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    let fish = FISH_MODELS.iter().map(|fish| (fish.model, fish.bytes));
    vfs.mount_memory("SJK fishing", FILES.iter().copied().chain(fish))?;
    Ok(())
}

const SLOTS: usize = crate::looks::SLOTS;
const BUTTON_ATTACK: u16 = 1;
const BUTTON_USE: u16 = 32;
const BUTTON_ALT_ATTACK: u16 = 128;
const CONTENTS_SOLID: u32 = 1;
const CONTENTS_WATER: u32 = 4;
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
const PMF_FOLLOW: u16 = 4096;
/// From the player's origin down to their feet (`DEFAULT_MINS_2`).
const FEET: f32 = -24.0;
/// The nearest and farthest water a cast reaches, ahead of the player.
const REACH_NEAR: f32 = 40.0;
const REACH_FAR: f32 = 320.0;
/// How far below the feet the water may lie (a pier, a ledge).
const DEPTH: f32 = 400.0;
/// Where there is no water, the pond goes on the ground this far ahead (the nearest
/// that has ground, at most [`POND_DROP`] below the feet).
const POND_AHEAD: [f32; 3] = [72.0, 56.0, 40.0];
const POND_DROP: f32 = 48.0;
/// The pond's water above the ground (`fishing_assets.py`'s pond).
const POND_WATER: f32 = 0.8;
/// The rod's butt beside the player: this far to their right and ahead.
const ROD_RIGHT: f32 = 16.0;
const ROD_AHEAD: f32 = 6.0;
/// How steeply the rod rises (radians).
const ROD_PITCH: f32 = 0.7;
/// The rod's length (`fishing_assets.py`'s `ROD_LENGTH`).
const ROD_LENGTH: f32 = 72.0;
/// The line model's length (`LINE_LENGTH`).
const LINE_LENGTH: f32 = 64.0;
/// The bucket beside the player: this far to their left and ahead.
const BUCKET_LEFT: f32 = 16.0;
const BUCKET_AHEAD: f32 = 4.0;
/// How long the float flies out on a cast.
const CAST_TIME: Duration = Duration::from_millis(700);
/// A catch is reeled in from the water to the tip this long, then dangles there till
/// [`SHOW_CATCH`] is up.
const REEL: Duration = Duration::from_millis(700);
const SHOW_CATCH: Duration = Duration::from_millis(2100);
/// What hangs from the tip hangs this far under it.
const HANG: f32 = 14.0;
/// The rod in the hand: the hand holds it this far up from its butt (the cork).
const GRIP: f32 = 8.0;
/// How far a bite bends the rod down (radians), and a fish being reeled in.
const BITE_BEND: f32 = 0.25;
const REEL_BEND: f32 = 0.18;
/// The torso's pose while holding the rod: a two-handed weapon's ready pose, both hands
/// before the chest (`TORSO_WEAPONREADY3`, the rifles').
const HOLD_POSE: &str = "TORSO_WEAPONREADY3";
/// The wait for a bite, in seconds (20 to 60 until 04/11/2026, 15 to 40 until 11/11/2026:
/// players said fishing brought nothing). Not below 12: with the cast and the catch shown
/// a bite comes at most every 15 seconds or so, the hub's allowance for Angler (240 an
/// hour) and its least spacing of hooked bites (10 seconds).
const BITE_WAIT: (f32, f32) = (12.0, 25.0);
/// A fish's bite holds this long (seconds): the time to press `+use` (0.8 to 1.2 until
/// 11/11/2026, too short to answer).
const BITE_WINDOW: (f32, f32) = (1.5, 2.0);
/// An Echo's bite holds longer, and comes this soon after the Echo dropped (1.5 seconds
/// until 11/11/2026).
const ECHO_WINDOW: f32 = 3.0;
const ECHO_WAIT: (f32, f32) = (5.0, 15.0);
/// A missed Echo comes back after this long, at most [`ECHO_TRIES`] times in all (60 to
/// 180 seconds and two tries until 11/11/2026).
const ECHO_AGAIN: (f32, f32) = (30.0, 90.0);
const ECHO_TRIES: u8 = 3;
/// Pulling with no bite scares the fish: the next bite waits this much longer (10 to 20
/// seconds until 11/11/2026).
const SCARED: (f32, f32) = (5.0, 10.0);
/// Another player's float is looked for again once they stand this far from where it
/// was found.
const SPOT_MOVED: f32 = 24.0;

/// What a hooked bite brings up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Junk,
    Fish,
}

/// One thing in the catch table: its id in the fish log, its name, its kind and its
/// weight among its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Catch {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) kind: Kind,
    pub(crate) weight: u32,
}

const fn junk(id: &'static str, name: &'static str) -> Catch {
    Catch {
        id,
        name,
        kind: Kind::Junk,
        weight: 1,
    }
}

const fn fish(id: &'static str, name: &'static str, weight: u32) -> Catch {
    Catch {
        id,
        name,
        kind: Kind::Fish,
        weight,
    }
}

/// The great catfish's id: its own achievement (Old Whiskers).
pub(crate) const GREAT_CATFISH: &str = "old_whiskers";
/// How many kinds of fish [`CATCHES`] holds.
pub(crate) const FISH_KINDS: usize = {
    let mut count = 0;
    let mut index = 0;
    while index < CATCHES.len() {
        if matches!(CATCHES[index].kind, Kind::Fish) {
            count += 1;
        }
        index += 1;
    }
    count
};

/// How many kinds of junk [`CATCHES`] holds (Beachcomber).
pub(crate) const JUNK_KINDS: usize = CATCHES.len() - FISH_KINDS;

/// Everything a bite can bring up but nothing: SJK's own names.
pub(crate) const CATCHES: [Catch; 18] = [
    junk("boot", "an old boot"),
    junk("blaster", "a rusted blaster"),
    junk("droid_head", "a droid's head"),
    junk("weed", "a tangle of weed"),
    junk("hilt", "a broken saber hilt"),
    junk("holopad", "a soggy holopad"),
    fish("silver_darter", "a Silver darter", 20),
    fish("mud_snapper", "a Mud snapper", 20),
    fish("swamp_gulper", "a Swamp gulper", 16),
    fish("striped_bonefish", "a Striped bonefish", 14),
    fish("glow_perch", "a Glow perch", 10),
    fish("spiny_pike", "a Spiny pike", 8),
    fish("sun_carp", "a Sun carp", 6),
    fish("moon_eel", "a Moon eel", 4),
    fish("kyber_minnow", "a Kyber minnow", 3),
    fish("ghost_ray", "a Ghost ray", 2),
    fish("temple_koi", "a Temple koi", 2),
    fish("old_whiskers", "Old Whiskers, the great catfish", 1),
];

/// The chance a hooked bite brings nothing, junk or a fish, in percent (nothing 15 until
/// 11/11/2026; the rest went to the fish).
const NOTHING_CHANCE: u32 = 5;
const JUNK_CHANCE: u32 = 30;
/// Bad luck protection for Old Whiskers (04/11/2026): its weight rises by one for every
/// this many fish landed since the last one ([`Log::whiskers_wait`]). At its weight of 1
/// alone (one fish in 106) a tenth of players waited over 240 fish; with this, about 37
/// on average and a tenth over 67.
const WHISKERS_PITY: u32 = 10;
/// The fish log's count of the fish landed since the last Old Whiskers, kept as a catch's
/// count is: `whiskers_wait <count>`.
const WHISKERS_WAIT: &str = "whiskers_wait";

/// The weight of `catch` among its kind, `whiskers_wait` fish after the last Old
/// Whiskers.
fn weight(catch: &Catch, whiskers_wait: u32) -> u32 {
    if catch.id == GREAT_CATFISH {
        catch.weight.saturating_add(whiskers_wait / WHISKERS_PITY)
    } else {
        catch.weight
    }
}

/// Roll a hooked bite's catch from `roll`, a uniform number below `u32::MAX`, and
/// `pick`, `whiskers_wait` fish after the last Old Whiskers.
pub(crate) fn roll_catch(roll: u32, pick: u32, whiskers_wait: u32) -> Option<&'static Catch> {
    let percent = roll % 100;
    let kind = if percent < NOTHING_CHANCE {
        return None;
    } else if percent < NOTHING_CHANCE + JUNK_CHANCE {
        Kind::Junk
    } else {
        Kind::Fish
    };
    let total: u32 = CATCHES
        .iter()
        .filter(|c| c.kind == kind)
        .map(|c| weight(c, whiskers_wait))
        .sum();
    let mut left = pick % total;
    CATCHES.iter().filter(|c| c.kind == kind).find(|c| {
        let weight = weight(c, whiskers_wait);
        if left < weight {
            true
        } else {
            left -= weight;
            false
        }
    })
}

/// A small fast random source (xorshift64*): bite times and catches need no more.
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
}

/// A bite on the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bite {
    /// It lets go then.
    until: Instant,
    /// An Echo, not a fish.
    pub(crate) echo: bool,
}

/// Where the toy is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// The float flies out; it lands then.
    Casting {
        lands: Instant,
    },
    /// The float rests on the water; a fish bites then.
    Waiting {
        bite: Instant,
    },
    Biting(Bite),
    /// A catch is reeled in and dangles from the tip until then (none: the hook came up
    /// empty); `echo`, the tier slot of an Echo fished up.
    Landed {
        until: Instant,
        catch: Option<&'static Catch>,
        echo: Option<usize>,
    },
}

/// An Echo that dropped while fishing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Echo {
    /// None waits for its card.
    None,
    /// One waits: it bites then, missed so many times.
    Hooked { bite: Instant, misses: u8 },
    /// Its card may come (caught, missed too often, or the rod put away) until no Echo
    /// waits any more.
    Free,
}

/// What the frame tells the toy of the player's own life and input.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Frame {
    /// The player plays (alive, in their own view): `None` out of a game.
    pub(crate) playing: Option<bool>,
    /// Their health.
    pub(crate) health: i32,
    /// An Echo waits for its card ([`crate::holocron_popup::HolocronPopup::unannounced`]).
    pub(crate) echo_waiting: bool,
    /// Its tier's slot, for the holocron that comes up on the line.
    pub(crate) echo_tier: Option<usize>,
}

/// What the toy asks the frame to say or play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Say {
    /// A line on the centre of the screen.
    Centre(&'static str),
    /// A catch, with how many the log now holds of it.
    Caught(&'static Catch, u32),
    /// A line on the centre of the screen only when the bite cue is shown
    /// ([`cue_shown`]): a bite starting, or let go.
    Cue(&'static str),
    Sound(&'static str),
    /// A console command to run (`weapmelee`: empty hands for the rod).
    Command(&'static str),
}

/// The fish log: how many of each catch the player brought up, how many of those were
/// sold to the hub (junk, [`crate::fishing_sell`]), and the last fish (in the bucket while
/// the rod is out). The counts are what was caught, whatever was sold after.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Log {
    counts: Vec<(String, u32)>,
    sold: Vec<(String, u32)>,
    last: Option<&'static Catch>,
}

/// The fish log's line naming the last fish caught: `last <id>`.
const LAST: &str = "last";
/// The fish log's lines of what was sold: `sold <id> <count>` (three words, which logs
/// before 03/11/2026 skip).
const SOLD: &str = "sold";

impl Log {
    /// Read `text` (`<id> <count>` a line, `sold <id> <count>` and `last <id>`; anything
    /// else is skipped).
    pub(crate) fn parse(text: &str) -> Self {
        let mut log = Self::default();
        for line in text.lines() {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                [SOLD, id, value] => {
                    if let Ok(count) = value.parse::<u32>() {
                        log.sold.retain(|(known, _)| known != id);
                        log.sold.push(((*id).to_owned(), count));
                    }
                }
                [LAST, value] => {
                    log.last = CATCHES
                        .iter()
                        .find(|c| c.kind == Kind::Fish && c.id == *value);
                }
                [id, value] => {
                    if let Ok(count) = value.parse::<u32>() {
                        log.counts.retain(|(known, _)| known != id);
                        log.counts.push(((*id).to_owned(), count));
                    }
                }
                _ => {}
            }
        }
        log
    }

    pub(crate) fn text(&self) -> String {
        let mut text: String = self
            .counts
            .iter()
            .map(|(id, count)| format!("{id} {count}\n"))
            .collect();
        for (id, count) in &self.sold {
            text.push_str(&format!("{SOLD} {id} {count}\n"));
        }
        if let Some(last) = self.last {
            text.push_str(&format!("{LAST} {}\n", last.id));
        }
        text
    }

    /// The fish landed since the last Old Whiskers (all of them before the first).
    pub(crate) fn whiskers_wait(&self) -> u32 {
        self.count(WHISKERS_WAIT)
    }

    /// The last fish caught.
    pub(crate) fn last(&self) -> Option<&'static Catch> {
        self.last
    }

    /// Count `catch` (the last fish, if it is one); returns how many the log now holds.
    pub(crate) fn caught(&mut self, catch: &'static Catch) -> u32 {
        if catch.kind == Kind::Fish {
            self.last = Some(catch);
            if catch.id == GREAT_CATFISH {
                self.counts.retain(|(known, _)| known != WHISKERS_WAIT);
            } else {
                self.add(WHISKERS_WAIT);
            }
        }
        self.add(catch.id)
    }

    pub(crate) fn count(&self, id: &str) -> u32 {
        self.counts
            .iter()
            .find(|(known, _)| known == id)
            .map_or(0, |(_, count)| *count)
    }

    /// Count one more `id`; returns the new count.
    pub(crate) fn add(&mut self, id: &str) -> u32 {
        if let Some((_, count)) = self.counts.iter_mut().find(|(known, _)| known == id) {
            *count = count.saturating_add(1);
            *count
        } else {
            self.counts.push((id.to_owned(), 1));
            1
        }
    }

    /// How many of `id` were sold.
    pub(crate) fn sold_count(&self, id: &str) -> u32 {
        self.sold
            .iter()
            .find(|(known, _)| known == id)
            .map_or(0, |(_, count)| *count)
    }

    /// How many of `id` the player still holds: caught, less sold.
    pub(crate) fn held(&self, id: &str) -> u32 {
        self.count(id).saturating_sub(self.sold_count(id))
    }

    /// The junk the player holds, by id (only kinds with pieces).
    pub(crate) fn junk_held(&self) -> std::collections::BTreeMap<String, u32> {
        CATCHES
            .iter()
            .filter(|c| c.kind == Kind::Junk)
            .map(|c| (c.id.to_owned(), self.held(c.id)))
            .filter(|(_, held)| *held > 0)
            .collect()
    }

    /// Count `sold` (pieces by id, as the hub took them) as sold, never more than was
    /// caught; returns how many pieces that was.
    pub(crate) fn sell(&mut self, sold: &std::collections::BTreeMap<String, u32>) -> u32 {
        let mut pieces = 0;
        for (id, count) in sold {
            let taken = (*count).min(self.held(id));
            if taken == 0 {
                continue;
            }
            pieces += taken;
            if let Some((_, known)) = self.sold.iter_mut().find(|(known, _)| known == id) {
                *known += taken;
            } else {
                self.sold.push((id.clone(), taken));
            }
        }
        pieces
    }

    /// The different fish caught, of all there are.
    pub(crate) fn species(&self) -> (usize, usize) {
        let all = CATCHES.iter().filter(|c| c.kind == Kind::Fish);
        let caught = all.clone().filter(|c| self.count(c.id) > 0).count();
        (caught, all.count())
    }
}

/// Where another player's float rests, found from where they stood.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spot {
    from: [f32; 3],
    water: Option<([f32; 3], bool)>,
}

/// The toy's state.
pub(crate) struct Fishing {
    /// The rod is out (a cast found water).
    on: bool,
    /// `toy_fishing` was asked for; the frame looks for water.
    asked: bool,
    phase: Option<Phase>,
    /// Where the float rests, the player's facing at the cast, and where they stood.
    water: [f32; 3],
    /// No water was in reach: the float rests on a pond on the ground.
    pond: bool,
    yaw: f32,
    stood: [f32; 3],
    echo: Echo,
    /// Fish bites hooked in a row, none let go since (Steady Hand); an Echo's bite neither
    /// counts nor breaks it.
    streak: u32,
    /// `+use` went down since the last frame.
    use_pressed: bool,
    use_held: bool,
    /// The player moved, jumped or attacked since the last frame.
    stirred: bool,
    health: Option<i32>,
    /// The fish log; `log_read`, it was read from the profile's folder (what was caught
    /// before that is counted in when it is).
    log: Option<Log>,
    log_read: bool,
    dice: Dice,
    said: Vec<Say>,
    /// Other players' floats, by slot.
    spots: [Option<Spot>; SLOTS],
    /// Where each fisher's line ends this frame, and how far their rod bends: the actors
    /// draw the rods in their hands and the lines from them ([`rod_in_hand`]).
    pub(crate) lines: Lines,
    /// Where each rod's tip was drawn the last frame (the actors say), for what hangs
    /// from it.
    pub(crate) tips: [Option<[f32; 3]>; SLOTS],
    sounds_registered: Option<u64>,
    /// Selling the junk to the hub ([`crate::fishing_sell`]).
    pub(crate) sale: crate::fishing_sell::Sale,
    /// The bite cue's "!" this frame ([`GpuState::append_fishing_cue`]).
    cue: sjk_ui::DrawList,
    /// Rods to draw without a session, for the world shots: where the player stands,
    /// their yaw, whether a fish hangs from the tip, and which fish ([`FISH_MODELS`]) is
    /// in the bucket and on the line.
    #[cfg(test)]
    pub(crate) shot_rods: Vec<([f32; 3], f32, bool, usize)>,
}

impl Default for Fishing {
    fn default() -> Self {
        Self {
            on: false,
            asked: false,
            phase: None,
            water: [0.0; 3],
            pond: false,
            yaw: 0.0,
            stood: [0.0; 3],
            echo: Echo::None,
            streak: 0,
            use_pressed: false,
            use_held: false,
            stirred: false,
            health: None,
            log: None,
            log_read: false,
            dice: Dice::seeded(),
            said: Vec::with_capacity(4),
            spots: [None; SLOTS],
            lines: Lines::default(),
            tips: [None; SLOTS],
            sounds_registered: None,
            sale: Default::default(),
            cue: sjk_ui::DrawList::new(2),
            #[cfg(test)]
            shot_rods: Vec::new(),
        }
    }
}

/// The level way the player faces at `yaw` (radians), and their right.
fn ahead_right(yaw: f32) -> (Vec3, Vec3) {
    let ahead = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
    (ahead, Vec3::new(ahead.y, -ahead.x, 0.0))
}

/// The water's surface ahead of a player standing at `origin` facing `yaw` (radians),
/// the nearest within reach whose column meets water before ground: `contents` says a
/// point's contents.
pub(crate) fn water_ahead(
    origin: [f32; 3],
    yaw: f32,
    contents: impl Fn([f32; 3]) -> u32,
) -> Option<[f32; 3]> {
    let (ahead, _) = ahead_right(yaw);
    let origin = Vec3::from_array(origin);
    let top = origin.z + 8.0;
    let mut distance = REACH_NEAR;
    while distance <= REACH_FAR {
        let column = origin + ahead * distance;
        let mut z = top;
        while z > origin.z - DEPTH {
            let found = contents([column.x, column.y, z]);
            if found & CONTENTS_WATER != 0 {
                // Up to the surface, a unit at a time.
                let mut surface = z;
                while contents([column.x, column.y, surface + 1.0]) & CONTENTS_WATER != 0
                    && surface < top
                {
                    surface += 1.0;
                }
                return Some([column.x, column.y, surface + 1.0]);
            }
            if found & CONTENTS_SOLID != 0 {
                break;
            }
            z -= 4.0;
        }
        distance += 8.0;
    }
    None
}

/// The ground ahead of a player standing at `origin` facing `yaw` (radians), where a
/// pond goes when there is no water: the nearest of [`POND_AHEAD`] that has ground not
/// far below the feet.
pub(crate) fn ground_ahead(
    origin: [f32; 3],
    yaw: f32,
    contents: impl Fn([f32; 3]) -> u32,
) -> Option<[f32; 3]> {
    let (ahead, _) = ahead_right(yaw);
    let origin = Vec3::from_array(origin);
    let top = origin.z + 8.0;
    POND_AHEAD.iter().find_map(|&distance| {
        let column = origin + ahead * distance;
        if contents([column.x, column.y, top]) & CONTENTS_SOLID != 0 {
            return None;
        }
        let mut z = top;
        while z > origin.z + FEET - POND_DROP {
            if contents([column.x, column.y, z]) & CONTENTS_SOLID != 0 {
                let mut surface = z;
                while contents([column.x, column.y, surface + 1.0]) & CONTENTS_SOLID != 0 {
                    surface += 1.0;
                }
                return Some([column.x, column.y, surface + 1.0]);
            }
            z -= 2.0;
        }
        None
    })
}

/// Where a cast lands ahead of a player at `origin` facing `yaw`: water if there is any
/// in reach, else a pond on the ground ahead (`true`); none on a ledge over nothing.
pub(crate) fn spot_ahead(
    origin: [f32; 3],
    yaw: f32,
    contents: impl Fn([f32; 3]) -> u32,
) -> Option<([f32; 3], bool)> {
    water_ahead(origin, yaw, &contents)
        .map(|water| (water, false))
        .or_else(|| {
            ground_ahead(origin, yaw, &contents).map(|[x, y, z]| ([x, y, z + POND_WATER], true))
        })
}

impl Fishing {
    /// `toy_fishing`: get the rod out (the frame looks for water), or put it away.
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

    /// The bite on the line now, if any: where the float rests, and whether an Echo
    /// bites.
    pub(crate) fn biting(&self) -> Option<([f32; 3], bool)> {
        match self.phase {
            Some(Phase::Biting(bite)) if self.on => Some((self.water, bite.echo)),
            _ => None,
        }
    }

    /// Whether an Echo's card waits for its bite: the pop-up holds it back.
    pub(crate) fn holds_echo(&self) -> bool {
        self.on && matches!(self.echo, Echo::Hooked { .. })
    }

    /// Note `+use`, and any move, jump or attack, in `command`.
    pub(crate) fn shape_command(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
        if command.forward_move != 0
            || command.right_move != 0
            || command.up_move != 0
            || command.buttons & (BUTTON_ATTACK | BUTTON_ALT_ATTACK) != 0
        {
            self.stirred = true;
        }
    }

    pub(crate) fn take_said(&mut self) -> std::vec::Drain<'_, Say> {
        self.said.drain(..)
    }

    fn put_away(&mut self) {
        self.on = false;
        self.asked = false;
        self.phase = None;
        if matches!(self.echo, Echo::Hooked { .. }) {
            self.echo = Echo::Free;
        }
    }

    /// Cast at `water` (a pond's), standing at `stood` facing `yaw` (radians).
    fn cast(&mut self, (water, pond): ([f32; 3], bool), stood: [f32; 3], yaw: f32, now: Instant) {
        self.on = true;
        self.water = water;
        self.pond = pond;
        self.stood = stood;
        self.yaw = yaw;
        self.phase = Some(Phase::Casting {
            lands: now + CAST_TIME,
        });
        self.said.push(Say::Sound(CAST));
        self.said.push(Say::Command("weapmelee"));
    }

    fn wait(&mut self, now: Instant, extra: Duration) {
        let bite = now + self.dice.seconds(BITE_WAIT) + extra;
        self.phase = Some(Phase::Waiting { bite });
    }

    /// Move the toy on to `now`. `water` finds water ahead when the rod is asked for.
    pub(crate) fn update(
        &mut self,
        frame: Frame,
        now: Instant,
        water: impl FnOnce() -> Option<(([f32; 3], bool), [f32; 3], f32)>,
    ) {
        let used = std::mem::take(&mut self.use_pressed);
        let stirred = std::mem::take(&mut self.stirred);
        // The Echo's card: free once none waits.
        if !frame.echo_waiting {
            self.echo = Echo::None;
        }
        let health = self.health.replace(frame.health);
        if std::mem::take(&mut self.asked) && !self.on {
            match frame.playing {
                Some(true) => match water() {
                    Some((spot, stood, yaw)) => self.cast(spot, stood, yaw, now),
                    None => self.said.push(Say::Centre(
                        "Nowhere to cast: face the water or the ground ahead.",
                    )),
                },
                _ => self.said.push(Say::Centre("Fishing waits until you play.")),
            }
            return;
        }
        if !self.on {
            return;
        }
        let hurt = health.is_some_and(|before| frame.health < before);
        if frame.playing != Some(true) || stirred || hurt {
            self.put_away();
            if frame.playing == Some(true) {
                self.said.push(Say::Centre("You put the rod away."));
            }
            return;
        }
        if frame.echo_waiting && self.echo == Echo::None {
            self.echo = Echo::Hooked {
                bite: now + self.dice.seconds(ECHO_WAIT),
                misses: 0,
            };
        }
        let Some(phase) = self.phase else {
            return;
        };
        match phase {
            Phase::Casting { lands } => {
                if now >= lands {
                    self.said.push(Say::Sound(LAND));
                    self.wait(now, Duration::ZERO);
                }
            }
            Phase::Waiting { bite } => {
                if let Echo::Hooked { bite: echo_at, .. } = self.echo
                    && now >= echo_at
                {
                    self.phase = Some(Phase::Biting(Bite {
                        until: now + Duration::from_secs_f32(ECHO_WINDOW),
                        echo: true,
                    }));
                    self.said.push(Say::Sound(ECHO_BITE));
                    self.said.push(Say::Cue(CUE_ECHO));
                } else if used {
                    self.said
                        .push(Say::Centre("Too soon: you scared the fish off."));
                    let scared = self.dice.seconds(SCARED);
                    self.wait(now, scared);
                } else if now >= bite {
                    let window = self.dice.seconds(BITE_WINDOW);
                    self.phase = Some(Phase::Biting(Bite {
                        until: now + window,
                        echo: false,
                    }));
                    self.said.push(Say::Sound(BITE));
                    self.said.push(Say::Cue(CUE_BITE));
                }
            }
            Phase::Biting(bite) => {
                if used {
                    // A bite answered in time is play: the hub counts the time of a
                    // player standing at the water by these.
                    crate::holocrons::signals::bite();
                    self.said.push(Say::Sound(CATCH));
                    if bite.echo {
                        self.echo = Echo::Free;
                        self.said.push(Say::Centre("You fished up an Echo!"));
                        self.phase = Some(Phase::Landed {
                            until: now + SHOW_CATCH,
                            catch: None,
                            echo: Some(frame.echo_tier.unwrap_or(0)),
                        });
                    } else {
                        self.streak = self.streak.saturating_add(1);
                        let (roll, pick) = (self.dice.next(), self.dice.next());
                        let whiskers_wait = self.log.as_ref().map_or(0, Log::whiskers_wait);
                        let catch = roll_catch(roll, pick, whiskers_wait);
                        match catch {
                            Some(catch) => {
                                let count = self.log.get_or_insert_with(Log::default).caught(catch);
                                self.said.push(Say::Caught(catch, count));
                            }
                            None => self.said.push(Say::Centre("The hook came up empty.")),
                        }
                        self.phase = Some(Phase::Landed {
                            until: now + SHOW_CATCH,
                            catch,
                            echo: None,
                        });
                    }
                } else if now >= bite.until {
                    if !bite.echo {
                        self.streak = 0;
                    }
                    if bite.echo
                        && let Echo::Hooked { misses, .. } = self.echo
                    {
                        let misses = misses + 1;
                        if misses >= ECHO_TRIES {
                            self.echo = Echo::Free;
                            self.said.push(Say::Centre(
                                "The Echo slipped off the hook. It waits for you all the same.",
                            ));
                        } else {
                            self.echo = Echo::Hooked {
                                bite: now + self.dice.seconds(ECHO_AGAIN),
                                misses,
                            };
                            self.said.push(Say::Centre(
                                "Something shone under the float, and swam off.",
                            ));
                        }
                    } else if !bite.echo {
                        self.said.push(Say::Cue(CUE_MISSED));
                    }
                    self.wait(now, Duration::ZERO);
                }
            }
            Phase::Landed { until, .. } => {
                if now >= until {
                    self.wait(now, Duration::ZERO);
                }
            }
        }
    }

    /// The fish log, read from `directory` first if it was not yet.
    pub(crate) fn log_in(&mut self, directory: &Path) -> &mut Log {
        self.read_log(directory);
        self.log.get_or_insert_with(Log::default)
    }

    /// Read the fish log from `directory` once, counting in what was caught before.
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
            log.last = pending.last.or(log.last);
        }
        self.log = Some(log);
    }

    /// The last fish caught, for the bucket.
    fn last_fish(&self) -> Option<&'static Catch> {
        self.log.as_ref().and_then(Log::last)
    }

    pub(crate) fn save_log(&self, directory: &Path) {
        if let Some(log) = &self.log
            && let Err(error) = std::fs::write(directory.join(LOG_FILE), log.text())
        {
            crate::log::progress(format_args!("fishing: cannot write {LOG_FILE}: {error}"));
        }
    }

    /// How deep the float sits at `now` (units below the surface): a slow bob, pulled
    /// under by a bite.
    fn float_dip(&self, now: Instant, since: Instant) -> f32 {
        let t = now.saturating_duration_since(since).as_secs_f32();
        match self.phase {
            Some(Phase::Biting(bite)) => {
                let shake = (t * 40.0).sin() * 0.6;
                if bite.echo { 2.4 + shake } else { 1.8 + shake }
            }
            _ => (t * 2.1).sin() * 0.25,
        }
    }
}

/// Each fisher's line this frame, by slot: where it ends, and how far the rod bends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Lines {
    pub(crate) ends: [Option<[f32; 3]>; SLOTS],
    pub(crate) bend: [f32; SLOTS],
}

impl Default for Lines {
    fn default() -> Self {
        Self {
            ends: [None; SLOTS],
            bend: [0.0; SLOTS],
        }
    }
}

/// Who holds a rod this frame (bits by slot), for the pose ([`holding_rod`]).
static HOLDING: AtomicU64 = AtomicU64::new(0);

/// Whether the player in `slot` holds a rod this frame: their torso takes the rod's
/// pose (`actor_pose_steps.rs`).
pub(crate) fn holding_rod(slot: usize) -> bool {
    slot < SLOTS && HOLDING.load(Ordering::Relaxed) & (1 << slot) != 0
}

/// The protocol animation of the pose holding the rod.
pub(crate) fn hold_pose() -> Option<(usize, &'static str)> {
    static POSE: std::sync::OnceLock<Option<usize>> = std::sync::OnceLock::new();
    POSE.get_or_init(|| sjk_game_jka::legacy_animation_index(HOLD_POSE))
        .map(|index| (index, HOLD_POSE))
}

/// The rod in the right hand of a body posed with `matrices`, placed at `transform`:
/// held at its cork, pointing ahead where the body faces and up, bent down by `bend`
/// (radians). The instance and the tip, where the line starts.
pub(crate) fn rod_in_hand(
    preview: &crate::PlayerPreview,
    matrices: &[[[f32; 4]; 3]],
    transform: sjk_runtime::Transform,
    bend: f32,
) -> Option<(ActorInstance, Vec3)> {
    let hand = crate::saber::attachment_from_matrices(preview, matrices, crate::saber::HAND_BOLT)?;
    let rotation = crate::weapon_view::actor_world_rotation(transform.rotation);
    let (grip, _) =
        crate::saber::world_attachment(Vec3::from_array(transform.translation), rotation, hand);
    let forward = Quat::from_array(transform.rotation) * Vec3::X;
    let yaw = forward.y.atan2(forward.x);
    let tilt = Quat::from_rotation_z(yaw) * Quat::from_rotation_y(-(ROD_PITCH - bend));
    let butt = grip - tilt * Vec3::X * GRIP;
    let tip = butt + tilt * Vec3::X * ROD_LENGTH;
    Some((
        ActorInstance::new(butt.to_array(), tilt.to_array(), [1.0; 3]),
        tip,
    ))
}

/// Where the own rod is held in first person, in the camera's frame (ahead, left, up)
/// from the eye: low on the right, as a weapon is.
const VIEW_GRIP: Vec3 = Vec3::new(10.0, -8.0, -12.0);
/// How far the own rod in first person points up from the view, and in towards its middle.
const VIEW_PITCH: f32 = 0.5;
const VIEW_IN: f32 = 0.18;

/// The own rod in first person, where the body's hand is not drawn: held from the eye at
/// `camera` looking `yaw` and `pitch` (radians, up positive), bent by `bend`. The
/// instance and its tip.
pub(crate) fn rod_in_view(camera: Vec3, yaw: f32, pitch: f32, bend: f32) -> (ActorInstance, Vec3) {
    let view = Quat::from_rotation_z(yaw) * Quat::from_rotation_y(-pitch);
    let tilt = view * Quat::from_rotation_z(VIEW_IN) * Quat::from_rotation_y(-(VIEW_PITCH - bend));
    let butt = camera + view * VIEW_GRIP - tilt * Vec3::X * GRIP;
    let tip = butt + tilt * Vec3::X * ROD_LENGTH;
    (
        ActorInstance::new(butt.to_array(), tilt.to_array(), [1.0; 3]),
        tip,
    )
}

/// The rod beside a player standing at `origin` facing `yaw`: the instance and its tip
/// (the world shots, and where the tip is guessed before an actor said).
fn rod(origin: [f32; 3], yaw: f32) -> (ActorInstance, Vec3) {
    let (ahead, right) = ahead_right(yaw);
    let butt = Vec3::from_array(origin) + right * ROD_RIGHT + ahead * ROD_AHEAD + Vec3::Z * FEET;
    let rotation = Quat::from_rotation_z(yaw) * Quat::from_rotation_y(-ROD_PITCH);
    let tip = butt + rotation * Vec3::X * ROD_LENGTH;
    (
        ActorInstance::new(butt.to_array(), rotation.to_array(), [1.0; 3]),
        tip,
    )
}

/// The bucket at the feet of a player standing at `origin` facing `yaw`.
fn bucket(origin: [f32; 3], yaw: f32) -> (ActorInstance, Vec3) {
    let (ahead, right) = ahead_right(yaw);
    let at = Vec3::from_array(origin) - right * BUCKET_LEFT + ahead * BUCKET_AHEAD + Vec3::Z * FEET;
    (
        ActorInstance::new(
            at.to_array(),
            Quat::from_rotation_z(yaw).to_array(),
            [1.0; 3],
        ),
        at,
    )
}

/// The line from `from` to `to`.
pub(crate) fn line(from: Vec3, to: Vec3) -> Option<ActorInstance> {
    let way = to - from;
    let length = way.length();
    (length > 0.5).then(|| {
        let rotation = Quat::from_rotation_arc(Vec3::X, way / length);
        ActorInstance::new(
            from.to_array(),
            rotation.to_array(),
            [length / LINE_LENGTH, 1.0, 1.0],
        )
    })
}

/// The pond whose water is at `water`.
fn pond(water: Vec3) -> ActorInstance {
    let ground = water - Vec3::Z * POND_WATER;
    ActorInstance::new(ground.to_array(), Quat::IDENTITY.to_array(), [1.0; 3])
}

fn float(at: Vec3) -> ActorInstance {
    ActorInstance::new(at.to_array(), Quat::IDENTITY.to_array(), [1.0; 3])
}

/// The holocron of tier `slot` hanging at `at` (its top), turning.
fn hanging_holocron(at: Vec3, turn: f32) -> ActorInstance {
    ActorInstance::new(
        (at - Vec3::Z * 4.5).to_array(),
        Quat::from_rotation_z(turn).to_array(),
        [1.0; 3],
    )
}

/// A fish hanging by its mouth at `at` (the line's end), its tail down, swinging.
fn hanging_fish(at: Vec3, swing: f32) -> ActorInstance {
    let rotation =
        Quat::from_rotation_x(swing) * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    ActorInstance::new(at.to_array(), rotation.to_array(), [1.0; 3])
}

/// The bucket's water above its bottom, and its rim (`fishing_assets.py`'s bucket).
const BUCKET_WATER: f32 = 8.0;
const BUCKET_RIM: f32 = 11.0;
/// A fish this long or shorter floats on its side on the bucket's water; a longer one
/// stands in it head down, its tail out over the rim this far, leaning this much
/// (radians), shrunk to at most this long.
const BUCKET_FLOATS: f32 = 11.0;
const BUCKET_TAIL_OUT: f32 = 5.0;
const BUCKET_LEAN: f32 = 0.35;
const BUCKET_LONGEST: f32 = 18.0;

/// The last fish caught, `fish`, in the bucket at `at` (its bottom) facing `yaw`.
fn bucket_fish(at: Vec3, yaw: f32, fish: &FishModel) -> ActorInstance {
    let (mouth, rotation, scale) = in_bucket(at, yaw, fish);
    ActorInstance::new(mouth.to_array(), rotation.to_array(), [scale; 3])
}

/// Where `fish` goes in the bucket at `at`: its mouth, its turn and its scale.
fn in_bucket(at: Vec3, yaw: f32, fish: &FishModel) -> (Vec3, Quat, f32) {
    let turn = Quat::from_rotation_z(yaw + 0.6);
    if fish.length <= BUCKET_FLOATS {
        let rotation = turn * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
        let mouth = at + turn * Vec3::X * (fish.length / 2.0) + Vec3::Z * (BUCKET_WATER + 0.3);
        return (mouth, rotation, 1.0);
    }
    let scale = (BUCKET_LONGEST / fish.length).min(1.0);
    let rotation = turn * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 - BUCKET_LEAN);
    let height = fish.length * scale * BUCKET_LEAN.cos();
    let mouth_z = (BUCKET_RIM + BUCKET_TAIL_OUT - height).clamp(1.0, BUCKET_WATER - 2.0);
    (
        at + turn * Vec3::X * 2.5 + Vec3::Z * mouth_z,
        rotation,
        scale,
    )
}

fn drawn(entity: &sjk_protocol::EntityState) -> bool {
    entity.entity_type() == ET_PLAYER && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
}

impl GpuState {
    /// `toy_fishing`: get the rod out, or put it away.
    pub(crate) fn toggle_fishing(&mut self) {
        self.fishing.toggle();
    }

    /// The toy's frame: the own rod's logic, then every rod drawn (the own and other SJK
    /// players' whose look has it), and what it says.
    pub(crate) fn submit_fishing(&mut self, presentation_time: i64, now: Instant) {
        self.fishing_frame(presentation_time, now);
        self.say_fishing();
    }

    fn fishing_frame(&mut self, presentation_time: i64, now: Instant) {
        self.fishing_sale_frame(now);
        let mesh_of = |model: &str| {
            self.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == model)
        };
        let meshes = MODELS.map(mesh_of);
        let fish_meshes = FISH_MODELS.map(|fish| mesh_of(fish.model));
        // The mesh and model of the fish `id`.
        let fish_of = |id: &str| {
            fish_model(id).and_then(|(index, fish)| fish_meshes[index].map(|mesh| (mesh, fish)))
        };
        #[cfg(test)]
        if let [
            Some(rod_mesh),
            Some(bucket_mesh),
            Some(float_mesh),
            Some(line_mesh),
            Some(pond_mesh),
        ] = meshes
        {
            for &(stood, yaw, hanging, fish) in &self.fishing.shot_rods {
                let bsp = &self.bsp;
                let (rod_instance, tip) = rod(stood, yaw);
                let (bucket_instance, bucket_at) = bucket(stood, yaw);
                self.object_groups[rod_mesh].push(rod_instance);
                self.object_groups[bucket_mesh].push(bucket_instance);
                let fish = FISH_MODELS[fish % FISH_MODELS.len()];
                let Some((fish_mesh, fish)) = fish_of(fish.id) else {
                    continue;
                };
                self.object_groups[fish_mesh].push(bucket_fish(bucket_at, yaw, fish));
                let Some((water, in_pond)) = spot_ahead(stood, yaw, |point| {
                    bsp.point_contents(point, CONTENTS_SOLID | CONTENTS_WATER)
                }) else {
                    continue;
                };
                let water = Vec3::from_array(water);
                if in_pond {
                    self.object_groups[pond_mesh].push(pond(water));
                }
                let at = if hanging {
                    let hang = tip - Vec3::Z * 14.0;
                    self.object_groups[fish_mesh].push(hanging_fish(hang, 0.3));
                    hang
                } else {
                    water
                };
                self.object_groups[float_mesh].push(float(at));
                if let Some(instance) = line(tip, at + Vec3::Z * 3.0) {
                    self.object_groups[line_mesh].push(instance);
                }
            }
        }
        let Some(session) = self.live_session.as_ref() else {
            self.fishing.update(Frame::default(), now, || None);
            self.fishing.spots = [None; SLOTS];
            self.fishing.lines = Lines::default();
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
        let frame = Frame {
            playing: Some(alive && !spectating),
            health: player.health(),
            echo_waiting: self.holocron_popup.unannounced(),
            echo_tier: self.holocron_popup.unannounced_tier(),
        };
        // The achievements count catches on servers, as the rest: never a demo, a game this
        // client hosts or a server on this machine (`achievements::tracker`).
        let counts = !session.is_local() && !session.server().ip().is_loopback();
        let yaw = self.camera_yaw;
        let bsp = &self.bsp;
        let was_landed = matches!(self.fishing.phase, Some(Phase::Landed { .. }));
        self.fishing.update(frame, now, || {
            spot_ahead(origin, yaw, |point| {
                bsp.point_contents(point, CONTENTS_SOLID | CONTENTS_WATER)
            })
            .map(|spot| (spot, origin, yaw))
        });
        if !was_landed && let Some(Phase::Landed { catch, echo, .. }) = self.fishing.phase {
            if counts {
                let streak = self.fishing.streak;
                crate::achievements::update(|record| record.fished(catch, echo.is_some(), streak));
            }
            if let Some(directory) = self
                .console
                .as_ref()
                .map(|c| c.config_directory().to_owned())
            {
                self.fishing.read_log(&directory);
                self.fishing.save_log(&directory);
            }
        }
        // The rod out: the log is read for the last fish, in the bucket.
        if self.fishing.on
            && !self.fishing.log_read
            && let Some(directory) = self
                .console
                .as_ref()
                .map(|c| c.config_directory().to_owned())
        {
            self.fishing.read_log(&directory);
        }
        // What the actors drew the last frame, and what they draw this one.
        let tips = std::mem::replace(&mut self.fishing.tips, [None; SLOTS]);
        self.fishing.lines = Lines::default();
        let mut holding = 0_u64;
        let [
            _,
            Some(bucket_mesh),
            Some(float_mesh),
            Some(line_mesh),
            Some(pond_mesh),
        ] = meshes
        else {
            HOLDING.store(0, Ordering::Relaxed);
            return;
        };
        let holocron_mesh = |tier: usize| {
            let request = crate::holocrons::stage::Request { tier, owned: true };
            let body = request.model_of(&request.parts()[0]);
            self.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == body)
        };
        let seconds = presentation_time as f32 / 1000.0;
        // The own rod.
        if self.fishing.on
            && let Some(own) = own
        {
            holding |= 1 << own;
            let fishing = &self.fishing;
            let tip = tips[own]
                .map(Vec3::from_array)
                .unwrap_or_else(|| rod(fishing.stood, fishing.yaw).1);
            let (bucket_instance, bucket_at) = bucket(fishing.stood, fishing.yaw);
            self.object_groups[bucket_mesh].push(bucket_instance);
            if let Some((mesh, fish)) = fishing.last_fish().and_then(|last| fish_of(last.id)) {
                self.object_groups[mesh].push(bucket_fish(bucket_at, fishing.yaw, fish));
            }
            let water = Vec3::from_array(fishing.water);
            if fishing.pond {
                self.object_groups[pond_mesh].push(pond(water));
            }
            let (end, bend) = match fishing.phase {
                Some(Phase::Casting { lands }) => {
                    let left = lands.saturating_duration_since(now).as_secs_f32()
                        / CAST_TIME.as_secs_f32();
                    let along = 1.0 - left;
                    let at = tip.lerp(water, along) + Vec3::Z * (along * (1.0 - along) * 60.0);
                    self.object_groups[float_mesh].push(float(at));
                    (at + Vec3::Z * 3.0, 0.0)
                }
                Some(Phase::Landed { until, catch, echo }) => {
                    let elapsed = SHOW_CATCH.saturating_sub(until.saturating_duration_since(now));
                    let reeled = (elapsed.as_secs_f32() / REEL.as_secs_f32()).min(1.0);
                    let eased = reeled * reeled * (3.0 - 2.0 * reeled);
                    let hang = tip - Vec3::Z * HANG;
                    let at = water.lerp(hang, eased);
                    let swing = (elapsed.as_secs_f32() * 6.0).sin() * 0.5 * eased;
                    match (catch, echo) {
                        (_, Some(tier)) => {
                            if let Some(mesh) = holocron_mesh(tier) {
                                self.object_groups[mesh]
                                    .push(hanging_holocron(at, elapsed.as_secs_f32() * 1.5));
                            }
                        }
                        (Some(catch), None) if catch.kind == Kind::Fish => {
                            if let Some((mesh, _)) = fish_of(catch.id) {
                                self.object_groups[mesh].push(hanging_fish(at, swing));
                            }
                        }
                        _ => self.object_groups[float_mesh].push(float(at)),
                    }
                    (at, if reeled < 1.0 { REEL_BEND } else { 0.0 })
                }
                Some(Phase::Biting(_)) => {
                    let at = water - Vec3::Z * fishing.float_dip(now, fishing_since(now));
                    self.object_groups[float_mesh].push(float(at));
                    (
                        at + Vec3::Z * 3.0,
                        BITE_BEND + (seconds * 30.0).sin() * 0.05,
                    )
                }
                _ => {
                    let at = water - Vec3::Z * fishing.float_dip(now, fishing_since(now));
                    self.object_groups[float_mesh].push(float(at));
                    (at + Vec3::Z * 3.0, 0.0)
                }
            };
            self.fishing.lines.ends[own] = Some(end.to_array());
            self.fishing.lines.bend[own] = bend;
            // No hand drew a rod the last frame (first person, a model without the
            // bolt): the line runs from where the tip would be.
            if tips[own].is_none()
                && let Some(instance) = line(tip, end)
            {
                self.object_groups[line_mesh].push(instance);
            }
        }
        // Other players' rods: their float where this client finds water ahead of them.
        let followed = slots.followed().filter(|&slot| slot < SLOTS);
        for entity in &snapshot.entities {
            let slot = usize::from(entity.number());
            if slot >= SLOTS
                || Some(slot) == own
                || Some(slot) == followed
                || !drawn(entity)
                || !self.looks.toy(slot, TOY)
            {
                if slot < SLOTS {
                    self.fishing.spots[slot] = None;
                }
                continue;
            }
            let Some(presented) = self
                .live_world
                .entity(sjk_runtime::EntityId::new(slot as u64 + 1))
            else {
                continue;
            };
            holding |= 1 << slot;
            let origin = presented.sample(presentation_time).translation;
            let yaw = presented
                .sample_pose(presentation_time)
                .map_or(0.0, |pose| pose.view_angles_degrees[1])
                .to_radians();
            let stale = self.fishing.spots[slot].is_none_or(|spot| {
                Vec3::from_array(spot.from).distance(Vec3::from_array(origin)) > SPOT_MOVED
            });
            if stale {
                let water = spot_ahead(origin, yaw, |point| {
                    bsp.point_contents(point, CONTENTS_SOLID | CONTENTS_WATER)
                });
                self.fishing.spots[slot] = Some(Spot {
                    from: origin,
                    water,
                });
            }
            self.object_groups[bucket_mesh].push(bucket(origin, yaw).0);
            if let Some((water, in_pond)) = self.fishing.spots[slot].and_then(|spot| spot.water) {
                if in_pond {
                    self.object_groups[pond_mesh].push(pond(Vec3::from_array(water)));
                }
                let t = seconds + slot as f32;
                let at = Vec3::from_array(water) - Vec3::Z * (t * 2.1).sin() * 0.25;
                self.object_groups[float_mesh].push(float(at));
                self.fishing.lines.ends[slot] = Some((at + Vec3::Z * 3.0).to_array());
                if tips[slot].is_none()
                    && let Some(instance) = line(rod(origin, yaw).1, at + Vec3::Z * 3.0)
                {
                    self.object_groups[line_mesh].push(instance);
                }
            }
        }
        if own.is_none() {
            self.fishing.spots = [None; SLOTS];
        }
        HOLDING.store(holding, Ordering::Relaxed);
    }

    /// Say on the centre of the screen what the toy has to say; its sounds wait for
    /// [`Self::play_fishing_sounds`].
    fn say_fishing(&mut self) {
        let said: Vec<Say> = self.fishing.take_said().collect();
        let cue = said.iter().any(|say| matches!(say, Say::Cue(_))) && self.fishing_cue_shown();
        for say in said {
            let text = match say {
                Say::Centre(text) => text.to_owned(),
                Say::Cue(text) if cue => text.to_owned(),
                Say::Cue(_) => continue,
                Say::Caught(catch, count) => {
                    let (species, all) = self.fishing.log.as_ref().map_or((0, 0), Log::species);
                    match catch.kind {
                        Kind::Fish if count == 1 => {
                            format!(
                                "You caught {}! New in your log ({species} of {all} fish).",
                                catch.name
                            )
                        }
                        Kind::Fish => format!("You caught {} ({count} so far).", catch.name),
                        _ => format!("You fished up {}.", catch.name),
                    }
                }
                Say::Sound(path) => {
                    self.fishing_cues.push(path);
                    continue;
                }
                Say::Command(command) => {
                    if let Some(console) = self.console.as_mut()
                        && let Err(error) = console.queue_command(command)
                    {
                        crate::log::progress(format_args!("fishing: {command}: {error}"));
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

    /// Whether this frame shows bites on screen ([`CUE_CVAR`]): the toy's sounds count
    /// as heard while the Fishing sounds are on, the toys' volume and the effects volume
    /// are above 0 and the game is not muted while away.
    pub(crate) fn fishing_cue_shown(&self) -> bool {
        let console = self.console.as_ref();
        let mode = console
            .and_then(|console| console.integer_cvar(CUE_CVAR))
            .unwrap_or(CUE_DEFAULT);
        let effects = console
            .and_then(|console| console.float_cvar("s_volume"))
            .unwrap_or(0.5);
        let heard = effects > 0.0
            && !console.is_some_and(crate::console::ViewerConsole::window_muted)
            && crate::toy_sounds::Options::read(console)
                .gain(crate::toy_sounds::Toy::Fishing, 1.0)
                .is_some();
        cue_shown(mode, heard)
    }

    /// The "!" over the float while something bites, when the cue is shown
    /// ([`Self::fishing_cue_shown`]); gold for an Echo. Nothing while `shown` is false
    /// (menus, console, scoreboard, intermission) or the float is out of view.
    pub(crate) fn append_fishing_cue(&mut self, viewport: [f32; 2], shown: bool) {
        self.fishing.cue.clear();
        let Some((water, echo)) = self.fishing.biting() else {
            return;
        };
        let Some(camera) = self.frame_view.filter(|_| shown) else {
            return;
        };
        if !self.fishing_cue_shown() {
            return;
        }
        let point = Vec3::from_array(water) + Vec3::Z * CUE_RISE;
        let Some(at) = camera.project(point) else {
            return;
        };
        let unit = crate::ui_scale::height_scale(viewport[1]);
        let seconds = fishing_since(Instant::now()).elapsed().as_secs_f32();
        let size = CUE_SIZE * unit * (1.0 + 0.12 * (seconds * CUE_PULSE).sin());
        let colour = if echo {
            sjk_ui::Color::new(1.0, 0.82, 0.3, 1.0)
        } else {
            sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0)
        };
        let width = size * 2.0;
        for (offset, color) in [
            (2.0 * unit, sjk_ui::Color::new(0.0, 0.0, 0.0, 0.8)),
            (0.0, colour),
        ] {
            let _ = self.fishing.cue.push(sjk_ui::DrawCommand::Text {
                rect: sjk_ui::Rect::new(
                    at[0] - width * 0.5 + offset,
                    at[1] - size * 1.3 + offset,
                    width,
                    size * 1.3,
                ),
                text: sjk_ui::TextId(0),
                size,
                color,
                align: sjk_ui::TextAlign::Center,
                overflow: sjk_ui::TextOverflow::Clip,
                weight: sjk_ui::FontWeight::Semibold,
                letter_spacing: 0.0,
            });
        }
        let cue = &self.fishing.cue;
        let (vertices, font) = self.game_fonts.target(
            crate::game_font::RetailFont::Headline,
            &mut self.text_vertices,
            &self.ui_font,
        );
        crate::ui_renderer::append_text_commands_where(
            cue,
            |_| "!",
            |_, _| true,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            crate::text::CodePalette::Legible,
        );
    }

    /// Play the toy's sounds, the player's own, in both ears.
    pub(crate) fn play_fishing_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.fishing_cues.clear();
            return;
        };
        if self.fishing.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(vfs, SOUNDS);
            self.fishing.sounds_registered = Some(audio.generation());
        }
        let slot = self
            .live_session
            .as_ref()
            .map_or(0, |session| session.latest_snapshot().player.client_num());
        for path in self.fishing_cues.drain(..) {
            audio.play_toy(
                crate::toy_sounds::Toy::Fishing,
                path,
                1.0,
                None,
                sjk_audio::SourceId(u32::from(slot)),
                crate::audio::TOY_CHANNEL,
            );
        }
    }
}

/// The float's bob runs on a clock of its own: any fixed start serves.
fn fishing_since(now: Instant) -> Instant {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *START.get_or_init(|| now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What was sold is kept apart from what was caught: the counts stay, held is the
    /// difference, a sale never takes more than is held, and the log reads back.
    #[test]
    fn sold_junk_is_kept_beside_the_catches() {
        let mut log = Log::parse("boot 4\nhilt 2\nsilver_darter 3\nlast silver_darter\n");
        assert_eq!(log.held("boot"), 4);
        let sold: std::collections::BTreeMap<String, u32> =
            [("boot".to_owned(), 3), ("hilt".to_owned(), 5)].into();
        assert_eq!(log.sell(&sold), 5, "three boots and the two hilts held");
        assert_eq!((log.count("boot"), log.held("boot")), (4, 1));
        assert_eq!((log.count("hilt"), log.held("hilt")), (2, 0));
        assert_eq!(
            log.junk_held(),
            [("boot".to_owned(), 1)].into_iter().collect()
        );
        let text = log.text();
        assert!(text.contains("sold boot 3\n") && text.contains("sold hilt 2\n"));
        let again = Log::parse(&text);
        assert_eq!(again, log);
        assert_eq!(again.species(), (1, FISH_KINDS));
        // A log written before selling reads as nothing sold.
        assert_eq!(Log::parse("boot 4\n").held("boot"), 4);
    }

    /// The bite cue: never at 0, only while the sound cannot be heard at 1 (the
    /// default), always at 2.
    #[test]
    fn the_bite_cue_follows_its_setting_and_the_sound() {
        assert_eq!(CUE_DEFAULT, 1);
        for heard in [false, true] {
            assert!(!cue_shown(0, heard));
            assert!(!cue_shown(-1, heard));
            assert!(cue_shown(2, heard));
            assert!(cue_shown(3, heard));
        }
        assert!(cue_shown(1, false));
        assert!(!cue_shown(1, true));
    }

    /// A bite says its cue as it starts and as it goes unhooked; an Echo's bite its own.
    #[test]
    fn a_bite_starts_and_ends_with_its_cue() {
        let mut fishing = Fishing::default();
        let now = Instant::now();
        cast(&mut fishing, now);
        let _ = fishing.take_said().count();
        let Some(Phase::Waiting { bite }) = fishing.phase else {
            panic!("waiting");
        };
        fishing.update(frame(false), bite, || None);
        assert_eq!(fishing.biting(), Some(([100.0, 0.0, -40.0], false)));
        let said: Vec<Say> = fishing.take_said().collect();
        assert!(said.contains(&Say::Cue(CUE_BITE)), "{said:?}");
        fishing.update(frame(false), bite + Duration::from_secs(5), || None);
        assert_eq!(fishing.biting(), None);
        let said: Vec<Say> = fishing.take_said().collect();
        assert!(said.contains(&Say::Cue(CUE_MISSED)), "{said:?}");
        // An Echo waiting bites with its own cue.
        fishing.update(frame(true), bite + Duration::from_secs(6), || None);
        let Echo::Hooked { bite: echo_at, .. } = fishing.echo else {
            panic!("an Echo waits");
        };
        fishing.update(frame(true), echo_at, || None);
        assert_eq!(fishing.biting().map(|(_, echo)| echo), Some(true));
        let said: Vec<Say> = fishing.take_said().collect();
        assert!(said.contains(&Say::Cue(CUE_ECHO)), "{said:?}");
    }

    fn frame(echo_waiting: bool) -> Frame {
        Frame {
            playing: Some(true),
            health: 100,
            echo_waiting,
            echo_tier: echo_waiting.then_some(1),
        }
    }

    fn cast(fishing: &mut Fishing, now: Instant) {
        fishing.toggle();
        fishing.update(frame(false), now, || {
            Some((([100.0, 0.0, -40.0], false), [0.0; 3], 0.0))
        });
        assert!(fishing.on());
        fishing.update(frame(false), now + CAST_TIME, || None);
        assert!(matches!(fishing.phase, Some(Phase::Waiting { .. })));
    }

    fn press_use(fishing: &mut Fishing) {
        let command = UserCommand {
            buttons: BUTTON_USE,
            ..UserCommand::default()
        };
        fishing.shape_command(&command);
        fishing.shape_command(&UserCommand::default());
    }

    #[test]
    fn a_cast_needs_somewhere_to_land() {
        let mut fishing = Fishing::default();
        fishing.toggle();
        fishing.update(frame(false), Instant::now(), || None);
        assert!(!fishing.on());
        assert!(matches!(fishing.take_said().next(), Some(Say::Centre(_))));
    }

    #[test]
    fn a_bite_hooked_in_time_lands_and_a_late_one_waits_again() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        let Some(Phase::Waiting { bite }) = fishing.phase else {
            unreachable!()
        };
        let wait = bite - now;
        // The cast, then BITE_WAIT.
        let (low, high) = BITE_WAIT;
        assert!(wait >= CAST_TIME + Duration::from_secs_f32(low));
        assert!(wait <= CAST_TIME + Duration::from_secs_f32(high) + Duration::from_millis(1));
        fishing.update(frame(false), bite, || None);
        assert!(matches!(
            fishing.phase,
            Some(Phase::Biting(Bite { echo: false, .. }))
        ));
        press_use(&mut fishing);
        fishing.update(frame(false), bite + Duration::from_millis(300), || None);
        assert!(matches!(fishing.phase, Some(Phase::Landed { .. })));
        // Another bite let go waits for the next.
        fishing.update(frame(false), bite + Duration::from_secs(3), || None);
        let Some(Phase::Waiting { bite }) = fishing.phase else {
            unreachable!()
        };
        fishing.update(frame(false), bite, || None);
        // Past the longest window (BITE_WINDOW's 2 s), the fish lets go.
        fishing.update(frame(false), bite + Duration::from_secs(3), || None);
        assert!(matches!(fishing.phase, Some(Phase::Waiting { .. })));
    }

    #[test]
    fn pulling_without_a_bite_scares_the_fish() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        press_use(&mut fishing);
        let later = now + CAST_TIME + Duration::from_secs(1);
        fishing.update(frame(false), later, || None);
        let Some(Phase::Waiting { bite }) = fishing.phase else {
            unreachable!()
        };
        assert!(bite - later >= Duration::from_secs(16));
    }

    #[test]
    fn moving_or_getting_hurt_puts_the_rod_away() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        fishing.shape_command(&UserCommand {
            forward_move: 127,
            ..UserCommand::default()
        });
        fishing.update(frame(false), now + Duration::from_secs(2), || None);
        assert!(!fishing.on());
        cast(&mut fishing, now);
        let hurt = Frame {
            health: 80,
            ..frame(false)
        };
        fishing.update(hurt, now + Duration::from_secs(2), || None);
        assert!(!fishing.on());
    }

    #[test]
    fn an_echo_waits_for_its_bite_and_goes_free_when_hooked() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        let start = now + CAST_TIME;
        fishing.update(frame(true), start, || None);
        assert!(fishing.holds_echo());
        let at = start + Duration::from_secs(16);
        fishing.update(frame(true), at, || None);
        assert!(matches!(
            fishing.phase,
            Some(Phase::Biting(Bite { echo: true, .. }))
        ));
        press_use(&mut fishing);
        fishing.update(frame(true), at + Duration::from_millis(500), || None);
        assert!(!fishing.holds_echo());
        // Its card shown, a new Echo is held again.
        fishing.update(frame(false), at + Duration::from_secs(1), || None);
        fishing.update(frame(true), at + Duration::from_secs(2), || None);
        assert!(fishing.holds_echo());
    }

    #[test]
    fn an_echo_missed_every_try_or_the_rod_put_away_lets_its_card_come() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        let mut at = now + CAST_TIME;
        fishing.update(frame(true), at, || None);
        for _ in 0..ECHO_TRIES {
            assert!(fishing.holds_echo());
            at += Duration::from_secs(200);
            fishing.update(frame(true), at, || None);
            assert!(matches!(
                fishing.phase,
                Some(Phase::Biting(Bite { echo: true, .. }))
            ));
            at += Duration::from_secs(4);
            fishing.update(frame(true), at, || None);
        }
        assert!(!fishing.holds_echo());
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        fishing.update(frame(true), now + CAST_TIME, || None);
        assert!(fishing.holds_echo());
        fishing.toggle();
        assert!(!fishing.holds_echo());
    }

    #[test]
    fn the_catch_table_follows_its_chances() {
        let (mut nothing, mut junk, mut fish) = (0, 0, 0);
        for roll in 0..100 {
            match roll_catch(roll, roll * 7, 0) {
                None => nothing += 1,
                Some(c) if c.kind == Kind::Junk => junk += 1,
                Some(_) => fish += 1,
            }
        }
        assert_eq!((nothing, junk, fish), (5, 30, 65));
        // Every catch can come up, and ids are distinct.
        for catch in &CATCHES {
            let kind_roll = if catch.kind == Kind::Junk { 30 } else { 90 };
            assert!(
                (0..1000).any(|pick| roll_catch(kind_roll, pick, 0) == Some(catch)),
                "{}",
                catch.id
            );
            assert_eq!(CATCHES.iter().filter(|c| c.id == catch.id).count(), 1);
        }
    }

    /// Old Whiskers' weight rises by one for every ten fish since the last, the others'
    /// stay as they are.
    #[test]
    fn old_whiskers_comes_sooner_the_longer_it_is_missed() {
        let whiskers = |wait: u32, picks: u32| {
            (0..picks)
                .filter(|&pick| roll_catch(90, pick, wait).map(|c| c.id) == Some(GREAT_CATFISH))
                .count()
        };
        assert_eq!(whiskers(0, 106), 1);
        assert_eq!(whiskers(9, 106), 1);
        assert_eq!(whiskers(10, 107), 2);
        assert_eq!(whiskers(1000, 206), 101);
        // The fish before it in the table keep their own share.
        let darters = (0..206)
            .filter(|&pick| roll_catch(90, pick, 1000).map(|c| c.id) == Some("silver_darter"))
            .count();
        assert_eq!(darters, 20);
    }

    /// The log counts the fish since the last Old Whiskers, junk aside, and keeps the count.
    #[test]
    fn the_log_counts_the_wait_for_old_whiskers() {
        let catch = |id: &str| CATCHES.iter().find(|c| c.id == id).expect("a catch");
        let mut log = Log::default();
        log.caught(catch("silver_darter"));
        log.caught(catch("moon_eel"));
        log.caught(catch("boot"));
        assert_eq!(log.whiskers_wait(), 2);
        assert_eq!(Log::parse(&log.text()), log);
        log.caught(catch(GREAT_CATFISH));
        assert_eq!(log.whiskers_wait(), 0);
        log.caught(catch("sun_carp"));
        assert_eq!(Log::parse(&log.text()).whiskers_wait(), 1);
        // A log from before it, or a malformed line, starts from none.
        assert_eq!(Log::parse("silver_darter 40\n").whiskers_wait(), 0);
        assert_eq!(Log::parse("whiskers_wait lots\n").whiskers_wait(), 0);
        // It is no catch: the kinds caught leave it out.
        assert_eq!(Log::parse("whiskers_wait 30\n").species(), (0, 12));
    }

    /// Each fish bite hooked adds to the streak; one let go ends it; an Echo's does neither.
    #[test]
    fn the_streak_counts_bites_hooked_in_a_row() {
        let now = Instant::now();
        let mut fishing = Fishing::default();
        cast(&mut fishing, now);
        let hook = |fishing: &mut Fishing, take: bool| {
            let Some(Phase::Waiting { bite }) = fishing.phase else {
                unreachable!()
            };
            fishing.update(frame(false), bite, || None);
            if take {
                press_use(fishing);
            }
            fishing.update(frame(false), bite + Duration::from_secs(2), || None);
            if take {
                fishing.update(frame(false), bite + Duration::from_secs(5), || None);
            }
        };
        hook(&mut fishing, true);
        hook(&mut fishing, true);
        assert_eq!(fishing.streak, 2);
        hook(&mut fishing, false);
        assert_eq!(fishing.streak, 0);
        hook(&mut fishing, true);
        assert_eq!(fishing.streak, 1);
    }

    #[test]
    fn the_log_reads_back_what_it_wrote() {
        let mut log = Log::parse("silver_darter 3\nnot a line at all\nboot 1\n");
        assert_eq!(log.count("silver_darter"), 3);
        assert_eq!(log.add("silver_darter"), 4);
        assert_eq!(log.add("moon_eel"), 1);
        assert_eq!(Log::parse(&log.text()), log);
        assert_eq!(log.species(), (2, 12));
    }

    #[test]
    fn the_log_keeps_the_last_fish_for_the_bucket() {
        let catch = |id: &str| CATCHES.iter().find(|c| c.id == id).expect("a catch");
        let mut log = Log::parse(
            "silver_darter 3
last moon_eel
",
        );
        assert_eq!(log.last(), Some(catch("moon_eel")));
        assert_eq!(log.caught(catch("sun_carp")), 1);
        // Junk never goes in the bucket.
        log.caught(catch("boot"));
        assert_eq!(log.last(), Some(catch("sun_carp")));
        assert_eq!(Log::parse(&log.text()), log);
        // A last line naming no fish is skipped.
        assert_eq!(
            Log::parse(
                "last boot
last nothing
"
            )
            .last(),
            None
        );
    }

    #[test]
    fn each_fish_has_its_own_model_as_long_as_the_table_says() {
        let fish: Vec<_> = CATCHES.iter().filter(|c| c.kind == Kind::Fish).collect();
        assert_eq!(fish.len(), FISH_MODELS.len());
        for (catch, model) in fish.iter().zip(&FISH_MODELS) {
            assert_eq!(catch.id, model.id);
            // The MD3's one frame: its bounds' lowest X is the tail's tip (the mouth at 0).
            let bytes = model.bytes;
            assert_eq!(&bytes[..4], b"IDP3", "{}", model.id);
            let ofs_frames = u32::from_le_bytes(bytes[92..96].try_into().unwrap()) as usize;
            let min_x = f32::from_le_bytes(bytes[ofs_frames..ofs_frames + 4].try_into().unwrap());
            assert!((min_x + model.length).abs() < 0.1, "{}: {min_x}", model.id);
        }
    }

    #[test]
    fn the_last_fish_sits_in_the_bucket_with_its_tail_out() {
        for fish in &FISH_MODELS {
            let (mouth, rotation, scale) = in_bucket(Vec3::ZERO, 0.4, fish);
            let tail = mouth + rotation * Vec3::NEG_X * fish.length * scale;
            for end in [mouth, tail] {
                // Within the bucket's rim (7.5 across from its middle), never under its
                // bottom.
                assert!(end.truncate().length() < 7.5, "{}: {end}", fish.id);
                assert!(end.z >= 0.9, "{}: {end}", fish.id);
            }
            if fish.length > BUCKET_FLOATS {
                assert!(tail.z > BUCKET_RIM, "{}: the tail at {tail}", fish.id);
                assert!(mouth.z < BUCKET_WATER, "{}: the mouth at {mouth}", fish.id);
            } else {
                assert!((tail.z - mouth.z).abs() < 0.01, "{} floats flat", fish.id);
                assert!(tail.z > BUCKET_WATER, "{} floats", fish.id);
            }
        }
    }

    #[test]
    fn water_is_found_ahead_before_ground() {
        // Water below z 0 from x 100 on; ground under the player before it.
        let contents = |p: [f32; 3]| {
            if p[0] >= 100.0 && p[2] < 0.0 {
                CONTENTS_WATER
            } else if p[0] < 100.0 && p[2] < -24.0 {
                CONTENTS_SOLID
            } else {
                0
            }
        };
        let water = water_ahead([0.0, 0.0, 0.0], 0.0, contents).unwrap();
        assert!(water[0] >= 100.0 && water[0] < 110.0);
        assert!((water[2] - 0.0).abs() <= 1.0);
        // Facing away from it, none: a pond on the ground instead.
        assert_eq!(
            water_ahead([0.0, 0.0, 0.0], std::f32::consts::PI, contents),
            None
        );
        let (pond, in_pond) = spot_ahead([0.0, 0.0, 0.0], std::f32::consts::PI, contents).unwrap();
        assert!(in_pond);
        assert!((pond[0] + 72.0).abs() < 0.01);
        assert!((pond[2] - (-24.0 + POND_WATER)).abs() <= 1.0);
        assert_eq!(
            spot_ahead([0.0, 0.0, 0.0], 0.0, contents).map(|(_, p)| p),
            Some(false)
        );
        // Over nothing (a ledge), no cast.
        assert_eq!(spot_ahead([0.0, 0.0, 0.0], 0.0, |_| 0), None);
    }

    #[test]
    fn the_line_runs_from_the_tip_to_the_float() {
        let (_, tip) = rod([0.0; 3], 0.0);
        let to = Vec3::new(150.0, 0.0, -30.0);
        let instance = line(tip, to).unwrap();
        let rotation = Quat::from_array(instance.rotation);
        let end = Vec3::from_array(instance.position)
            + rotation * (Vec3::X * LINE_LENGTH * instance.scale[0]);
        assert!(end.distance(to) < 0.01);
    }
}
