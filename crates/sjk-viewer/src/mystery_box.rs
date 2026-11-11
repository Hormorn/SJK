//! Mystery boxes (Sol's idea, 27/10/2026; `docs/mystery-boxes.md`): boxes SJK's creator
//! places on maps through the hub, which any SJK player opens for credits with `+use`.
//! The hub keeps where they stand, takes the price and rolls what is inside
//! (`sjk_identity::mystery_box`); this module draws the boxes standing on the loaded map
//! file, asks to open the one the player faces, and plays the opening: a music box tune
//! while things roll over the box, slowing to a stop on what came out, which then floats
//! there until the opener takes it with `+use` (or by itself after [`HOLD`]). Other
//! players' openings come through the hub's toy event. No game server knows of them: a
//! box is drawn by SJK clients only and has no collision.

mod hunt;
mod hunt_watch;
pub(crate) mod label;

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_identity::mystery_box::{self as wire, MysteryBox};
use sjk_identity::service::BoxDone;
use sjk_protocol::{PlayerState, UserCommand};
use std::time::{Duration, Instant};

/// The box's chest and its lid (bundled, `scripts/mystery_box_model.py`).
pub(crate) const MODEL: &str = "models/sjk/mystery_box.md3";
pub(crate) const LID_MODEL: &str = "models/sjk/mystery_box_lid.md3";
/// What the roll stops on when the box blows up (bundled, `scripts/teddy_bear_model.py`).
const TEDDY_MODEL: &str = "models/sjk/teddy_bear.md3";
/// The piles of gold coins credits come as (Sol, 11/11/2026: crystals before; bundled,
/// `scripts/box_loot_model.py`): 50, 200 and 500 credits.
const COINS_SMALL: &str = "models/sjk/coins_small.md3";
const COINS_MEDIUM: &str = "models/sjk/coins_medium.md3";
const COINS_BIG: &str = "models/sjk/coins_big.md3";
/// The Ray Gun a box may hand its opener (bundled, `scripts/box_loot_model.py`; held and
/// fired as a prop gun, `propgun.rs`).
const RAYGUN_MODEL: &str = "models/sjk/raygun.md3";
/// Heard as a Ray Gun's shield rises round the player it struck (bundled,
/// `scripts/box_loot_assets.py`).
pub(crate) const RAY_SHIELD_SOUND: &str = "sound/sjk/raygun/shield.wav";
/// The chest's height and depth (across its front), and how far the lid reaches past it.
const HEIGHT: f32 = 24.0;
const DEPTH: f32 = 26.0;
const LID_OVERHANG: f32 = 1.0;
/// How far the lid swings open, in degrees, and how long it takes to open and to close.
const LID_OPEN: f32 = 75.0;
const LID_OPENING: Duration = Duration::from_millis(450);
const LID_CLOSING: Duration = Duration::from_millis(700);
/// Played when a box opens, for the opener and everyone near: [`EXTRAS_OPEN`] when the
/// SJK extras pack has it.
const OPEN: &str = "sound/effects/crate_slide.mp3";
/// The roll's music box tune, its last chord at [`ROLL`] (bundled): [`EXTRAS_ROLL`] when
/// the SJK extras pack has it.
const ROLL_SOUND: &str = "sound/sjk/mystery_box/roll.wav";
/// The box's opening, roll and closing in the optional SJK extras pack
/// (`unofficial_sjk_extras.pk3`, the Sith cloud's `extras`; Sol's sounds of 30/10/2026,
/// the roll's last note at [`ROLL`]). Without the pack, [`OPEN`], [`ROLL_SOUND`] and no
/// sound as the lid closes.
const EXTRAS_OPEN: &str = "sound/sjk/mystery_box/extras_open.mp3";
const EXTRAS_ROLL: &str = "sound/sjk/mystery_box/extras_roll.mp3";
const EXTRAS_CLOSE: &str = "sound/sjk/mystery_box/extras_close.mp3";
/// The extras pack's sounds and what plays without the pack.
const EXTRAS: [(&str, Option<&str>); 4] = [
    (EXTRAS_OPEN, Some(OPEN)),
    (EXTRAS_ROLL, Some(ROLL_SOUND)),
    (EXTRAS_CLOSE, None),
    (LAUGH, Some(LAUGH_FALLBACK)),
];
/// The opener taking what came out (bundled).
const TAKE_SOUND: &str = "sound/sjk/mystery_box/take.wav";
/// Played as the roll stops on credits.
const WIN: &str = "sound/player/holocron.wav";
/// Played as the roll stops on the jackpot.
const JACKPOT: &str = "sound/items/respawn1.wav";
/// Played as the roll stops on nothing.
const EMPTY: &str = "sound/movers/doors/door_locked.mp3";
/// Played to every hunter, the mark too, when the box hunt picks a new mark (Sol's
/// pick, 28/10/2026).
const HUNT_MARK: &str = "sound/effects/mpalarm.wav";
/// Played, with [`BLAST_EFFECT`], where a box's opener stands when it blows up.
const BLAST_SOUND: &str = "sound/weapons/detpack/explode.mp3";
const BLAST_EFFECT: &str = "detpack/explosion";
/// Played at a box that blows up as its roll stops on the teddy bear, before the blast: a
/// ghost having fun, after Call of Duty zombies (Sol, 28/10/2026; 30/10/2026: at the
/// box, once the bear shows, rather than to every hunter at the opening). Sol's laugh of
/// 29/10/2026, in the optional SJK extras pack (`unofficial_sjk_extras.pk3`, the Sith
/// cloud's `extras`); without the pack, [`LAUGH_FALLBACK`], a retail gloat.
const LAUGH: &str = "sound/sjk/mystery_box/laugh.mp3";
const LAUGH_FALLBACK: &str = "sound/chars/alora/misc/gloat1.mp3";
/// How long the teddy bear shows, laughing, before the box blows up.
const BLAST_DELAY: Duration = Duration::from_millis(1_500);
/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;
/// Farthest a player may stand from a box to open it, across the floor.
const REACH: f32 = 80.0;
/// Farthest above or below the box a player may stand to open it.
const REACH_UP: f32 = 96.0;
/// Widest angle between the player's facing and the box, in radians (60 degrees).
const FACING: f32 = std::f32::consts::FRAC_PI_3;
/// How long the roll lasts, to the tune's last chord (`scripts/mystery_box_assets.py`)
/// and the extras pack's roll's last note (Sol, 30/10/2026; 5.5 s before).
const ROLL: Duration = Duration::from_millis(5_200);
/// How long what came out waits for its opener before it is taken by itself.
const HOLD: Duration = Duration::from_secs(10);
/// How long the roll's first thing takes to rise out of the box.
const RISE: Duration = Duration::from_millis(700);
/// How high over the box's base the roll floats (the crate is 33 units tall).
const FLOAT: f32 = 44.0;
/// How many times the roll changes what it shows; it slows to a stop on the last.
const SWAPS: f32 = 30.0;
/// How far apart the opener's client tells each other SJK player that they took what
/// came out (the toy relay, one player an event).
const TAKE_GAP: Duration = Duration::from_millis(50);

/// The toy relay's event the opener's client sends when they take what came out of box
/// `id` (Sol, 30/10/2026: the others saw it float on for the hold's 10 seconds).
fn take_event(id: i64) -> String {
    format!("take_{id}")
}

/// The box a relayed take event names.
fn parse_take(event: &str) -> Option<i64> {
    event.strip_prefix("take_")?.parse().ok()
}

/// How long an opening waits for the hub before the player may ask again.
const ASK_TIMEOUT: Duration = Duration::from_secs(10);
/// Farthest a box `mysterybox_remove` takes without an id may stand.
const REMOVE_REACH: f32 = 256.0;

/// A thing the roll shows: a model and how much it is grown to read at a box's size,
/// or a tier's holocron, every part of its form (`crate::holocrons::stage::FORMS`, by
/// slot) posed as the Holocrons tab poses it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Item {
    model: &'static str,
    scale: f32,
    form: Option<usize>,
    /// How many copies are drawn, gathered round the centre (a pile of shards).
    copies: u8,
}

const fn item(model: &'static str, scale: f32) -> Item {
    Item {
        model,
        scale,
        form: None,
        copies: 1,
    }
}

/// `copies` of `model`, grown by `scale`, gathered round the centre.
const fn pile(model: &'static str, scale: f32, copies: u8) -> Item {
    Item {
        model,
        scale,
        form: None,
        copies,
    }
}

/// Tier `slot`'s holocron (0 Uncommon to 3 Mythical), grown by `scale`.
const fn holocron(slot: usize, scale: f32) -> Item {
    Item {
        model: FORMS[slot].parts[0].model,
        scale,
        form: Some(slot),
        copies: 1,
    }
}

use crate::holocrons::stage::FORMS;

impl Item {
    /// The models it is drawn with.
    fn models(self) -> impl Iterator<Item = &'static str> {
        let parts = self.form.map_or(&[][..], |slot| FORMS[slot].parts);
        std::iter::once(self.model)
            .filter(move |_| parts.is_empty())
            .chain(parts.iter().map(|part| part.model))
    }
}

/// The four tiers' holocrons as the roll shows them (Sol, 29/10/2026: the real ones, not
/// one for all), each about the Illuminate holocron's size over the box.
const HOLOCRONS: [Item; 4] = [
    holocron(0, 2.0),
    holocron(1, 2.3),
    holocron(2, 2.0),
    holocron(3, 1.8),
];

/// The four kyber crystals as the roll shows them (Sol, 11/11/2026: a box may give one,
/// very rarely), drawn as the Holocrons tab draws them (forms 4 to 7).
const KYBERS: [Item; 4] = [
    holocron(4, 2.0),
    holocron(5, 2.0),
    holocron(6, 2.0),
    holocron(7, 2.0),
];

/// The kyber crystal a pile of shards is drawn with: the Rare kyber's, the shards' cyan.
const SHARD_MODEL: &str = "models/sjk/holocron_kyber_rare_crystal.md3";

/// What the roll stops on, by outcome: bolts (an older hub's `empty`), piles of gold coins
/// (`coins`, `double`, `jackpot`; Sol, 11/11/2026: a blue, green and red crystal
/// before), a teddy bear (it blows up; Sol, 30/10/2026, after Call of Duty zombies'
/// boxes: a thermal detonator before), SJK's holocron (a holocron of a tier not told, from
/// an older hub; every client stops on its tier's since 11/11/2026, [`HOLOCRONS`]), a pile
/// of kyber shards (shards; [`BIG_PILE`] for a big one), a little mystery box (a free
/// opening, before 11/11/2026), a saber hilt (the boxes' blade) and the Ray Gun.
const PRIZES: [Item; 10] = [
    item("models/items/metallic_bolts.md3", 1.5),
    item(COINS_SMALL, 4.0),
    item(COINS_MEDIUM, 3.2),
    item(COINS_BIG, 2.8),
    item(TEDDY_MODEL, 1.6),
    item(crate::illuminate::MODEL, 3.0),
    pile(SHARD_MODEL, 1.5, 3),
    item(MODEL, 0.45),
    item("models/weapons2/saber/saber_w.md3", 2.6),
    item(RAYGUN_MODEL, 2.2),
];

/// A big pile of shards: more than [`BIG_PILE_FROM`] (the boxes' blade for a player who
/// holds it pays 60).
const BIG_PILE: Item = pile(SHARD_MODEL, 1.7, 7);
const BIG_PILE_FROM: u64 = 15;

/// What rolls over an opened box before it stops: only what a box can give (Sol,
/// 29/10/2026: the game's weapons rolled before, which no box gave), the three piles of
/// coins, the four tiers' holocrons, two kyber crystals, both piles of shards, the hilt
/// and the Ray Gun (11/11/2026: the bolts, the little box and SJK's placeholder
/// holocron rolled before); not the teddy bear, which only comes at the end.
const REEL: [Item; 13] = [
    PRIZES[1],
    PRIZES[2],
    PRIZES[3],
    PRIZES[6],
    PRIZES[8],
    PRIZES[9],
    BIG_PILE,
    HOLOCRONS[0],
    HOLOCRONS[1],
    HOLOCRONS[2],
    HOLOCRONS[3],
    KYBERS[1],
    KYBERS[3],
];

/// The prize the roll stops on for `outcome` (a holocron or kyber crystal of `tier`, when
/// told: the opener's answer, or since 11/11/2026 the hub's event); a newer hub's outcome
/// by whether it pays.
fn prize(outcome: &str, pays: bool, tier: Option<&str>) -> Item {
    let form = tier.and_then(crate::holocrons::stage::form_index);
    match outcome {
        "empty" => PRIZES[0],
        wire::COINS | "refund" => PRIZES[1],
        "double" => PRIZES[2],
        "jackpot" => PRIZES[3],
        wire::EXPLODE => PRIZES[4],
        wire::SHARD_PILE => PRIZES[6],
        wire::AGAIN => PRIZES[7],
        wire::BLADE => PRIZES[8],
        wire::RAYGUN => PRIZES[9],
        wire::HOLOCRON => form
            .filter(|slot| *slot < HOLOCRONS.len())
            .map_or(PRIZES[5], |slot| HOLOCRONS[slot]),
        wire::KYBER => form
            .and_then(|slot| slot.checked_sub(HOLOCRONS.len()))
            .filter(|slot| *slot < KYBERS.len())
            .map_or(KYBERS[1], |slot| KYBERS[slot]),
        _ if pays => PRIZES[2],
        _ => PRIZES[0],
    }
}

/// Every model the boxes draw: the box, its lid, then [`REEL`] (which holds the prizes).
/// Each once: a holocron's motes share a model, and the prizes roll too.
fn models() -> impl Iterator<Item = &'static str> {
    static MODELS: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    MODELS
        .get_or_init(|| {
            let mut models: Vec<&'static str> = Vec::new();
            let every = [MODEL, LID_MODEL].into_iter().chain(
                PRIZES
                    .iter()
                    .chain(&REEL)
                    .chain(&KYBERS)
                    .flat_map(|item| item.models()),
            );
            for model in every {
                if !models.contains(&model) {
                    models.push(model);
                }
            }
            models
        })
        .iter()
        .copied()
}

/// The creator's console commands, with their help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        "mysterybox_place",
        "SJK creator: place a mystery box where you aim, facing you ('server': for this server only)",
    ),
    (
        "mysterybox_remove",
        "SJK creator: remove the mystery box nearest you, or the one with the id given",
    ),
    (
        "mysterybox_list",
        "List the mystery boxes on this map, their ids and the price",
    ),
];

/// Whether `name` is one of [`COMMANDS`].
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

/// A bundled file's bytes, by its name in `assets/mystery_box`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/mystery_box/", $name))
    };
}

/// The bundled files at their game paths (made by `scripts/mystery_box_assets.py` and
/// `scripts/mystery_box_model.py`; the shader by hand).
const FILES: [(&str, &[u8]); 22] = [
    (ROLL_SOUND, bundled!("roll.wav")),
    (TAKE_SOUND, bundled!("take.wav")),
    (MODEL, bundled!("mystery_box.md3")),
    (LID_MODEL, bundled!("mystery_box_lid.md3")),
    (TEDDY_MODEL, bundled!("teddy_bear.md3")),
    ("models/sjk/teddy_bear.jpg", bundled!("teddy_bear.jpg")),
    ("models/sjk/mystery_box.jpg", bundled!("mystery_box.jpg")),
    (
        "models/sjk/mystery_box_glow.jpg",
        bundled!("mystery_box_glow.jpg"),
    ),
    (
        "models/sjk/mystery_box_halo.jpg",
        bundled!("mystery_box_halo.jpg"),
    ),
    (
        "shaders/sjk_mystery_box.shader",
        bundled!("mystery_box.shader"),
    ),
    ("models/sjk/box_loot.jpg", bundled!("box_loot.jpg")),
    (
        "models/sjk/box_loot_glow.jpg",
        bundled!("box_loot_glow.jpg"),
    ),
    (COINS_SMALL, bundled!("coins_small.md3")),
    (COINS_MEDIUM, bundled!("coins_medium.md3")),
    (COINS_BIG, bundled!("coins_big.md3")),
    (RAYGUN_MODEL, bundled!("raygun.md3")),
    ("models/sjk/raygun_bolt.md3", bundled!("raygun_bolt.md3")),
    ("sound/sjk/raygun/fire.wav", bundled!("raygun_fire.wav")),
    ("sound/sjk/raygun/impact.wav", bundled!("raygun_impact.wav")),
    ("sound/sjk/raygun/raise.wav", bundled!("raygun_raise.wav")),
    (RAY_SHIELD_SOUND, bundled!("raygun_shield.wav")),
    (crate::propgun::RAYGUN_FILE, bundled!("raygun.gun")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK mystery box", FILES.iter().copied())?;
    Ok(())
}

/// What the opener gets, told when they take it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Prize {
    price: u64,
    won: u64,
    balance: u64,
    /// A box that blew up: the hunter who got the price.
    gift: Option<wire::Gift>,
    /// The tier of the holocron it gave.
    holocron: Option<String>,
    /// The kyber shards it gave.
    shards: Option<u64>,
    /// The saber shader it gave.
    blade: Option<String>,
    /// The tier of the kyber crystal it gave (`kyber_rare`).
    kyber: Option<String>,
    /// The seconds of the Ray Gun it gave.
    raygun: Option<u64>,
    /// The DOOM coins held after (a box that blew up gave one).
    doom_coins: Option<u64>,
}

/// A box's opening under way: the roll, then what came out waiting to be taken.
#[derive(Clone, Debug, PartialEq)]
struct Opening {
    started: Instant,
    outcome: String,
    /// What the roll stops on.
    prize: Item,
    /// Varies what the roll shows from one opening to the next.
    seed: u32,
    /// The player's own opening: what they get.
    mine: Option<Prize>,
    /// The slot of the player who opened it, blown up if it blows up.
    opener: Option<u8>,
    /// The roll has stopped (its sound played).
    revealed: bool,
    /// A box that blew up has blown up, [`BLAST_DELAY`] after the roll stopped.
    blasted: bool,
}

impl Opening {
    /// What the roll shows `elapsed` into it: changing fast, then slower, the prize from
    /// the last change on.
    fn showing(&self, elapsed: Duration) -> Item {
        let t = (elapsed.as_secs_f32() / ROLL.as_secs_f32()).min(1.0);
        let swap = (SWAPS * (1.0 - (1.0 - t) * (1.0 - t))) as u32;
        if swap + 1 >= SWAPS as u32 {
            return self.prize;
        }
        let mixed = (self.seed ^ swap.wrapping_mul(0x9E37_79B9)).wrapping_mul(0x85EB_CA6B);
        REEL[(mixed >> 16) as usize % REEL.len()]
    }
}

/// One box standing on the loaded map.
#[derive(Clone, Debug, PartialEq)]
struct Here {
    id: i64,
    origin: Vec3,
    /// Facing, in degrees.
    yaw: f32,
    opening: Option<Opening>,
    lid: Lid,
}

/// A box's lid: open or not, and since when (it swings between the two).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Lid {
    open: bool,
    since: Option<Instant>,
}

impl Lid {
    fn set(&mut self, open: bool, now: Instant) {
        if self.open != open {
            *self = Self {
                open,
                since: Some(now),
            };
        }
    }

    /// How far open it is at `now`, in degrees.
    fn angle(&self, now: Instant) -> f32 {
        let Some(since) = self.since else {
            return 0.0;
        };
        let elapsed = now.saturating_duration_since(since).as_secs_f32();
        let ease = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        if self.open {
            LID_OPEN * ease(elapsed / LID_OPENING.as_secs_f32())
        } else {
            LID_OPEN * (1.0 - ease(elapsed / LID_CLOSING.as_secs_f32()))
        }
    }
}

/// A sound to play once.
struct Cue {
    path: &'static str,
    /// Where it plays; `None` for the player alone, wherever they are.
    origin: Option<[f32; 3]>,
    /// The box it comes from's own sound source ([`source`]), or the hunt's.
    source: sjk_audio::SourceId,
}

/// The first of the boxes' sound sources: past the game's entity numbers, which the
/// audio moves each frame to where their entity is (box sounds on source 0 followed the
/// player on slot 0 around the map, bug report #185), and past the boombox's and the
/// Monitor's.
const SOURCE_BASE: u32 = 0x5200_0000;

/// Box `id`'s own sound source.
fn source(id: i64) -> sjk_audio::SourceId {
    sjk_audio::SourceId(SOURCE_BASE | (id as u32 & 0xFFFF))
}

/// The box hunt's sound source, past every box's.
const HUNT_SOURCE: sjk_audio::SourceId = sjk_audio::SourceId(SOURCE_BASE | 0x1_0000);

/// The player took what came out of their box (`by_itself`: after [`HOLD`]).
#[derive(Debug, PartialEq)]
struct Taken {
    outcome: String,
    prize: Prize,
    by_itself: bool,
}

/// What the player is told of their own opening.
#[derive(Debug, PartialEq)]
enum Moment {
    /// The roll stopped on what came out of box `id`.
    Stopped {
        id: i64,
        outcome: String,
        prize: Prize,
    },
    Taken(Taken),
    /// A box that blew up at the roll's end, for everyone who sees it: the opener's slot
    /// and the box's place.
    Blast {
        opener: Option<u8>,
        at: Vec3,
    },
}

/// A number to vary a roll by.
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos())
}

/// A box the creator asked to place, waiting for the map's checksum.
struct PendingPlace {
    origin: Vec3,
    yaw: f32,
    /// `ip:port` for this server only, empty for every server.
    server: String,
}

/// Where the lid of a box at `origin` facing `yaw` degrees hangs: its hinge, the chest's
/// back top edge.
pub(crate) fn lid_origin(origin: Vec3, yaw: f32) -> Vec3 {
    let turn = Quat::from_rotation_z(yaw.to_radians());
    origin + turn * Vec3::new(-(DEPTH / 2.0 + LID_OVERHANG), 0.0, HEIGHT)
}

/// The boxes of the loaded map, the openings under way and the sounds to play.
#[derive(Default)]
pub(crate) struct MysteryBoxes {
    /// The hub list's revision the boxes were taken from, and the map, checksum and
    /// server they were taken for.
    taken: Option<(u64, String, i32, Option<String>)>,
    here: Vec<Here>,
    /// What opening one costs, as the hub said.
    price: u64,
    /// The seconds between two of the player's openings, as the hub said.
    cooldown: u64,
    /// When the player's own last opening started, for the cooldown.
    last_mine: Option<Instant>,
    /// The rigid meshes of [`models`] once loaded, in its order, and the mesh count
    /// they were looked up at.
    meshes: Vec<Option<usize>>,
    meshes_seen: usize,
    use_pressed: bool,
    use_held: bool,
    /// The box asked to open and when, until the hub answers.
    asked: Option<(i64, Instant)>,
    /// The last hub answer read.
    answer_seen: u64,
    pending_place: Option<PendingPlace>,
    cues: Vec<Cue>,
    /// Where boxes that blew up flung their openers this frame, for their explosions.
    blasts: Vec<Vec3>,
    sounds_registered: Option<u64>,
    /// What an outcome pays, by the hub's list (for other players' openings).
    outcomes: Vec<(String, u64)>,
    /// Free openings the player won in the box hunt and has not spent.
    free: u64,
    hunt: hunt::Hunt,
    /// Boxes whose prize the player took, and the slots of the SJK players still to tell
    /// (a bit a slot), and when the next may be told.
    takes: Vec<(i64, u64)>,
    tell_at: Option<Instant>,
    labels: label::Labels,
    /// A world shot's boxes, kept without a session.
    #[cfg(test)]
    shot: bool,
}

impl MysteryBoxes {
    /// The boxes standing on the loaded map: each one's id, where it stands and the way
    /// it faces, in degrees.
    pub(crate) fn standing(&self) -> Vec<(i64, Vec3, f32)> {
        self.here
            .iter()
            .map(|here| (here.id, here.origin, here.yaw))
            .collect()
    }

    /// Note a `+use` press in `command` for the frame's logic.
    pub(crate) fn observe_use(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// The models to load: those of [`models`] not loaded, while a box stands here.
    pub(crate) fn wanted_models(&self) -> impl Iterator<Item = &'static str> + '_ {
        models()
            .enumerate()
            .filter(|_| !self.here.is_empty())
            .filter(|(index, _)| self.meshes.get(*index).copied().flatten().is_none())
            .map(|(_, model)| model)
    }

    /// The words over the boxes this frame (`label`).
    pub(crate) fn labels(&self) -> &sjk_ui::DrawList {
        &self.labels.list
    }

    /// What `outcome` pays, by the hub's list.
    fn pays(&self, outcome: &str) -> Option<u64> {
        self.outcomes
            .iter()
            .find(|(id, _)| id == outcome)
            .map(|(_, credits)| *credits)
    }

    fn mesh_of(&self, model: &str) -> Option<usize> {
        let index = models().position(|known| known == model)?;
        self.meshes.get(index).copied().flatten()
    }

    fn clear(&mut self) {
        self.taken = None;
        self.here.clear();
        self.asked = None;
        self.use_pressed = false;
    }

    /// Take the boxes standing on `map` of `checksum` on `server` from `boxes`.
    fn take(&mut self, boxes: &wire::Boxes, map: &str, checksum: i32, server: Option<&str>) {
        let mut old = std::mem::take(&mut self.here);
        self.price = boxes.price;
        self.cooldown = boxes.cooldown;
        self.outcomes = boxes
            .outcomes
            .iter()
            .map(|outcome| (outcome.id.clone(), outcome.credits))
            .collect();
        self.here = boxes
            .boxes
            .iter()
            .filter(|placed| placed.stands_on(map, checksum, server))
            .map(|placed| Here {
                id: placed.id,
                origin: Vec3::from_array(placed.origin),
                yaw: placed.yaw,
                opening: old
                    .iter_mut()
                    .find(|here| here.id == placed.id)
                    .and_then(|here| here.opening.take()),
                lid: old
                    .iter()
                    .find(|here| here.id == placed.id)
                    .map(|here| here.lid)
                    .unwrap_or_default(),
            })
            .collect();
    }

    /// Box `id` opened at `now` on `outcome` by the player on slot `opener`: the roll
    /// starts. `mine` is what the player gets from their own opening; `told` the tier of
    /// a holocron or crystal the hub's event names (another player's); `seed` varies what
    /// the roll shows.
    #[allow(clippy::too_many_arguments)]
    fn opened(
        &mut self,
        id: i64,
        outcome: &str,
        mine: Option<Prize>,
        opener: Option<u8>,
        told: Option<&str>,
        seed: u32,
        now: Instant,
    ) {
        let Some(here) = self.here.iter_mut().find(|here| here.id == id) else {
            return;
        };
        let pays = outcome != "empty" && mine.as_ref().is_none_or(|prize| prize.won > 0);
        let tier = mine
            .as_ref()
            .and_then(|prize| prize.holocron.as_deref().or(prize.kyber.as_deref()))
            .or(told);
        let mut stops = prize(outcome, pays, tier);
        // The opener's pile of shards as big as it is (the boxes' blade, held already,
        // pays a pile too).
        if let Some(shards) = mine.as_ref().and_then(|prize| prize.shards) {
            stops = if shards > BIG_PILE_FROM {
                BIG_PILE
            } else {
                PRIZES[6]
            };
        }
        here.opening = Some(Opening {
            started: now,
            outcome: outcome.to_owned(),
            prize: stops,
            seed,
            mine,
            opener,
            revealed: false,
            blasted: false,
        });
        here.lid.set(true, now);
        let (origin, id) = (here.origin.to_array(), here.id);
        for path in [EXTRAS_OPEN, EXTRAS_ROLL] {
            self.cues.push(Cue {
                path,
                origin: Some(origin),
                source: source(id),
            });
        }
    }

    /// Move the openings on to `now`: a roll that stops plays what it stopped on, and
    /// what waited [`HOLD`] is taken by itself; what the player is told of theirs.
    fn tick(&mut self, now: Instant) -> Vec<Moment> {
        let mut moments = Vec::new();
        for here in &mut self.here {
            let Some(opening) = here.opening.as_mut() else {
                continue;
            };
            let elapsed = now.saturating_duration_since(opening.started);
            let explodes = opening.outcome == wire::EXPLODE;
            if !opening.revealed && elapsed >= ROLL {
                opening.revealed = true;
                // A box that blows up stops on the teddy bear and laughs; it blows up
                // [`BLAST_DELAY`] later.
                let path = match opening.outcome.as_str() {
                    "jackpot" | wire::BLADE | wire::KYBER => JACKPOT,
                    wire::EXPLODE => LAUGH,
                    _ if opening.prize == PRIZES[0] => EMPTY,
                    _ => WIN,
                };
                self.cues.push(Cue {
                    path,
                    origin: Some(here.origin.to_array()),
                    source: source(here.id),
                });
                if let Some(prize) = opening.mine.as_ref().filter(|_| !explodes) {
                    moments.push(Moment::Stopped {
                        id: here.id,
                        outcome: opening.outcome.clone(),
                        prize: prize.clone(),
                    });
                }
            }
            if explodes && !opening.blasted && elapsed >= ROLL + BLAST_DELAY {
                opening.blasted = true;
                moments.push(Moment::Blast {
                    opener: opening.opener,
                    at: here.origin,
                });
                self.cues.push(Cue {
                    path: BLAST_SOUND,
                    origin: Some(here.origin.to_array()),
                    source: source(here.id),
                });
                if let Some(prize) = &opening.mine {
                    moments.push(Moment::Stopped {
                        id: here.id,
                        outcome: opening.outcome.clone(),
                        prize: prize.clone(),
                    });
                }
            }
            if elapsed >= ROLL + HOLD {
                here.lid.set(false, now);
                self.cues.push(Cue {
                    path: EXTRAS_CLOSE,
                    origin: Some(here.origin.to_array()),
                    source: source(here.id),
                });
            }
            if elapsed >= ROLL + HOLD
                && let Some(opening) = here.opening.take()
                && let Some(prize) = opening.mine
            {
                moments.push(Moment::Taken(Taken {
                    outcome: opening.outcome,
                    prize,
                    by_itself: true,
                }));
            }
        }
        moments
    }

    /// The player on slot `slot` took what came out of their box `id` (the toy relay): it
    /// goes, the lid closes, as for the opener. Only from the player who opened it.
    fn taken_by(&mut self, id: i64, slot: u8, now: Instant) {
        let Some(here) = self.here.iter_mut().find(|here| here.id == id) else {
            return;
        };
        let theirs = here
            .opening
            .as_ref()
            .is_some_and(|opening| opening.mine.is_none() && opening.opener == Some(slot));
        if !theirs {
            return;
        }
        here.opening = None;
        here.lid.set(false, now);
        for path in [TAKE_SOUND, EXTRAS_CLOSE] {
            self.cues.push(Cue {
                path,
                origin: Some(here.origin.to_array()),
                source: source(here.id),
            });
        }
    }

    /// The next take to relay, at `now`, to one of `others` (bits a slot): the box and
    /// the slot; none while the gap since the last one runs or none is left.
    fn next_take(&mut self, others: u64, now: Instant) -> Option<(i64, u8)> {
        if self.tell_at.is_some_and(|at| now < at) {
            return None;
        }
        while let Some((id, slots)) = self.takes.first_mut() {
            *slots &= others;
            if *slots == 0 {
                self.takes.remove(0);
                continue;
            }
            let slot = slots.trailing_zeros() as u8;
            *slots &= !(1 << slot);
            let id = *id;
            self.tell_at = Some(now + TAKE_GAP);
            return Some((id, slot));
        }
        None
    }

    /// The player takes what came out of box `id`, when it is theirs and waiting.
    fn take_prize(&mut self, id: i64) -> Option<Taken> {
        let here = self.here.iter_mut().find(|here| here.id == id)?;
        let waiting = here
            .opening
            .as_ref()
            .is_some_and(|opening| opening.revealed && opening.mine.is_some());
        if !waiting {
            return None;
        }
        let opening = here.opening.take()?;
        here.lid.set(false, Instant::now());
        for path in [TAKE_SOUND, EXTRAS_CLOSE] {
            self.cues.push(Cue {
                path,
                origin: Some(here.origin.to_array()),
                source: source(here.id),
            });
        }
        Some(Taken {
            outcome: opening.outcome,
            prize: opening.mine?,
            by_itself: false,
        })
    }
}

/// The box among `boxes` the player at `from` facing `yaw` (radians) may open: within
/// [`REACH`] across the floor and [`REACH_UP`] up or down, within [`FACING`] of their
/// facing; the nearest of those.
fn pick_box<'a>(from: Vec3, yaw: f32, boxes: impl IntoIterator<Item = &'a Here>) -> Option<i64> {
    let facing = glam::Vec2::from_angle(yaw);
    boxes
        .into_iter()
        .filter_map(|here| {
            let way = (here.origin - from).truncate();
            let across = way.length();
            let up = (here.origin.z - from.z).abs();
            let faced = across < 1.0 || facing.angle_to(way).abs() <= FACING;
            (across <= REACH && up <= REACH_UP && faced).then_some((here.id, across))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// What opening a box costs the player: the price, their free openings and their
/// credits (when known).
struct Wallet {
    price: u64,
    free: u64,
    credits: Option<u64>,
}

/// What the Use prompt says at box `here`: open it (for its price, or free), take what
/// came out of the player's own, or why Use does nothing now (`cooldown`: the seconds
/// the player still waits; `asked`: the hub has not answered yet; another player's
/// roll; too few credits). None while the player's own roll runs, and for a box that
/// blows up.
fn box_prompt(
    here: &Here,
    wallet: &Wallet,
    cooldown: Option<u64>,
    asked: bool,
    now: Instant,
) -> Option<crate::use_prompt::Prompt> {
    use crate::use_prompt::Prompt;
    let prompt = |action: &str, line: Option<String>, ready: bool| Prompt {
        action: action.to_owned(),
        line,
        ready,
    };
    if let Some(opening) = &here.opening {
        let rolled = now.saturating_duration_since(opening.started) >= ROLL;
        return match (&opening.mine, rolled) {
            _ if opening.outcome == wire::EXPLODE => None,
            (Some(_), true) => Some(prompt("Take it", None, true)),
            (Some(_), false) => None,
            (None, _) => Some(prompt("Someone is opening it", None, false)),
        };
    }
    let open = "Open mystery box";
    Some(if asked {
        prompt("Opening...", None, false)
    } else if let Some(left) = cooldown {
        prompt(open, Some(format!("Your turn again in {left} s")), false)
    } else if wallet.free > 0 {
        prompt(open, Some("Free roll".to_owned()), true)
    } else if wallet.credits.is_some_and(|credits| credits < wallet.price) {
        prompt(
            open,
            Some(format!("{} credits: not enough", wallet.price)),
            false,
        )
    } else {
        prompt(open, Some(format!("{} credits", wallet.price)), true)
    })
}

/// The whole seconds left before the player may open another box, their last opening
/// at `last` and `cooldown` seconds between two; none once they have passed.
fn cooldown_left(last: Option<Instant>, cooldown: u64, now: Instant) -> Option<u64> {
    let since = now.saturating_duration_since(last?);
    let wait = Duration::from_secs(cooldown).checked_sub(since)?;
    (!wait.is_zero()).then(|| wait.as_secs_f32().ceil() as u64)
}

/// How an opening reads for the opener: [`result_text`], or for a box that blew up or
/// gave a holocron, what became of it.
fn prize_text(outcome: &str, prize: &Prize) -> String {
    // The teddy bear brings a DOOM coin (11/11/2026), the WHEEL OF DOOM's only price.
    let coin = if prize.doom_coins.is_some() {
        ". ^1+1 DOOM coin^7: spin a WHEEL OF DOOM"
    } else {
        ""
    };
    match (outcome, &prize.gift, &prize.holocron) {
        (wire::EXPLODE, Some(gift), _) if gift.credits > 0 => format!(
            "BOOM! The box blew up: your {} credits went to {}^7{coin}",
            gift.credits, gift.name
        ),
        (wire::EXPLODE, None, _) if prize.price > 0 => {
            format!("BOOM! The box blew up: -{} credits{coin}", prize.price)
        }
        (wire::EXPLODE, ..) => format!("BOOM! The box blew up: your free roll is spent{coin}"),
        (wire::HOLOCRON, _, Some(tier)) => {
            let name = crate::holocrons::name(tier).unwrap_or("A holocron");
            format!("{name}! In your Collection")
        }
        (wire::KYBER, ..) => {
            let name = prize
                .kyber
                .as_deref()
                .and_then(crate::kyber::from_id)
                .map_or("A kyber crystal", |tier| tier.name);
            format!("{name}! In your Collection")
        }
        (wire::RAYGUN, ..) => format!(
            "THE RAY GUN! {} s in your hand: attack fires it",
            prize.raygun.unwrap_or(30)
        ),
        (wire::BLADE, ..) if prize.blade.is_some() => {
            "The Mystery blade! In your Collection".to_owned()
        }
        (wire::BLADE, ..) if prize.shards.is_some() => format!(
            "The Mystery blade, again: +{} kyber shards",
            prize.shards.unwrap_or(0)
        ),
        (wire::SHARD_PILE, ..) if prize.shards.is_some() => {
            format!("+{} kyber shards", prize.shards.unwrap_or(0))
        }
        (wire::AGAIN, ..) => "A box in the box: your next roll is free".to_owned(),
        _ => result_text(outcome, prize.price, prize.won),
    }
}

/// How an opening reads for the opener.
fn result_text(outcome: &str, price: u64, won: u64) -> String {
    // A free roll won in the box hunt: what it paid alone.
    if price == 0 {
        return match outcome {
            "jackpot" => format!("JACKPOT! +{won} credits"),
            "double" => format!("A pile of gold! +{won} credits"),
            _ if won > 0 => format!("+{won} credits"),
            _ => "The box was empty: your free roll is spent".to_owned(),
        };
    }
    match outcome {
        "empty" => format!("The box was empty: -{price} credits"),
        "refund" => format!("Your {price} credits back"),
        wire::COINS => format!("Gold coins! +{won} credits"),
        "double" => format!("A pile of gold! +{won} credits"),
        "jackpot" => format!("JACKPOT! +{won} credits"),
        _ if won > 0 => format!("+{won} credits"),
        _ => format!("Nothing inside: -{price} credits"),
    }
}

/// Where the thing an opening shows floats `elapsed` into it, over a box facing `yaw`
/// (degrees): its height over the box's base (rising out of it, then bobbing) and its
/// yaw (side on to the box's front, turning slowly once the roll stopped).
fn float(elapsed: Duration, yaw: f32) -> (f32, f32) {
    let seconds = elapsed.as_secs_f32();
    let rise = (seconds / RISE.as_secs_f32()).min(1.0);
    let rise = 1.0 - (1.0 - rise) * (1.0 - rise);
    let bob = (seconds * 2.4).sin() * 1.5;
    let height = 12.0 + (FLOAT - 12.0) * rise + bob;
    let held = seconds - ROLL.as_secs_f32();
    let turn = if held > 0.0 { held * 60.0 } else { 0.0 };
    (height, yaw + 90.0 + turn)
}

/// Where each of `copies` stands round the centre (units, before the item's scale) and
/// how it leans: one alone stands upright at the centre; more stand in a ring, leaning
/// out, every other one lower and smaller-looking.
fn pile_spots(copies: u8) -> impl Iterator<Item = (Vec3, Quat)> {
    let count = copies.max(1);
    (0..count).map(move |index| {
        if count == 1 {
            return (Vec3::ZERO, Quat::IDENTITY);
        }
        let around = index as f32 / count as f32 * std::f32::consts::TAU + 0.4;
        let (sin, cos) = around.sin_cos();
        let low = if index % 2 == 1 { -2.0 } else { 1.0 };
        let reach = if count > 4 { 6.0 } else { 4.0 };
        let at = Vec3::new(cos * reach, sin * reach, low);
        let lean =
            Quat::from_axis_angle(Vec3::new(-sin, cos, 0.0), 0.35) * Quat::from_rotation_z(around);
        (at, lean)
    })
}

/// `pmtype_t`: the player plays (not dead, spectating or in intermission).
pub(crate) fn plays(player: &PlayerState) -> bool {
    /// `PMF_FOLLOW`: the player state is that of the player being followed.
    const PMF_FOLLOW: u16 = 4096;
    player.health() > 0
        && !matches!(player.movement_type(), 4 | 5 | 7 | 8)
        && player.movement_flags() & PMF_FOLLOW == 0
}

impl GpuState {
    fn mystery_box_print(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::Print,
            text,
            None,
            Instant::now(),
        );
    }

    /// A box at `at` blew up on the player on slot `opener`: an explosion where they
    /// stand, and when they are this client's player, it kills them (Sol, 30/10/2026: the
    /// game's `kill`, which the server carries out, so everyone sees them die; a ragdoll
    /// thrown up and away from the box before). Not where the Ragdoll could not strike
    /// them either (dead, in a duel, with the flag).
    fn mystery_box_blast(&mut self, opener: Option<u8>, at: Vec3) {
        let own = self
            .live_session
            .as_ref()
            .and_then(|session| u8::try_from(session.game_state().client_num).ok());
        let mine = opener.is_some() && opener == own;
        let stands = if mine {
            self.local_prediction
                .predicted_state()
                .map(|state| Vec3::from_array(state.origin))
        } else {
            opener.and_then(|slot| {
                let time = self.presentation_clock.sample(Instant::now());
                let presented = self
                    .live_world
                    .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))?;
                Some(Vec3::from_array(presented.sample(time).translation))
            })
        };
        let stands = stands.unwrap_or(at);
        self.mystery_boxes.blasts.push(stands);
        if mine
            && self.ragdoll.strikable()
            && let Some(session) = self.live_session.as_mut()
            && let Err(error) = session.send_reliable_command(b"kill")
        {
            crate::log::progress(format_args!("mystery box: kill not sent: {error}"));
        }
    }

    fn mystery_box_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    /// The frame's mystery box logic, after the looks: the boxes of the loaded map,
    /// the hub's answers, a `+use` press, the hint and other players' openings
    /// (`events`, this server's toy events).
    /// Find the boxes' models among the loaded ones, when more have loaded.
    fn find_mystery_box_meshes(&mut self) {
        if self.mystery_boxes.meshes_seen != self.object_meshes.len() {
            self.mystery_boxes.meshes_seen = self.object_meshes.len();
            self.mystery_boxes.meshes = models()
                .map(|model| {
                    self.object_meshes.iter().position(|mesh| {
                        mesh.appearance.variant.is_empty() && mesh.appearance.model == model
                    })
                })
                .collect();
        }
    }

    pub(crate) fn update_mystery_boxes(
        &mut self,
        events: &[sjk_identity::ToyEvent],
        relay_events: &[sjk_identity::ToyEvent],
    ) {
        #[cfg(test)]
        if self.mystery_boxes.shot {
            self.find_mystery_box_meshes();
            return;
        }
        let now = Instant::now();
        self.read_mystery_box_answer(now);
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.mystery_boxes.clear();
            return;
        };
        let server = session.server().to_string();
        if let Ok(own) = u8::try_from(session.game_state().client_num) {
            self.update_box_hunt(events, &server, own, now);
        }
        // The map's checksum is read when a box may stand on it (`map_edits`).
        let map_name = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let (revision, on_map) = crate::player_identity::with_snapshot(|snapshot| {
            (
                snapshot.mystery_boxes_revision,
                snapshot
                    .mystery_boxes
                    .as_ref()
                    .is_some_and(|boxes| boxes.on_map(map_name)),
            )
        })
        .unwrap_or_default();
        if on_map || self.mystery_boxes.pending_place.is_some() {
            self.map_edits.want_checksum(map_name);
        }
        let Some((map, checksum)) = self.map_edits.map().map(|(map, c)| (map.to_owned(), c)) else {
            self.mystery_boxes.here.clear();
            return;
        };
        self.place_pending_mystery_box(&map, checksum);
        let fresh = self.mystery_boxes.taken.as_ref().is_some_and(
            |(taken, taken_map, taken_checksum, taken_server)| {
                *taken == revision
                    && *taken_checksum == checksum
                    && *taken_map == map
                    && taken_server.as_deref() == Some(server.as_str())
            },
        );
        if !fresh {
            let boxes = crate::player_identity::with_snapshot(|snapshot| {
                snapshot.mystery_boxes.clone().unwrap_or_default()
            })
            .unwrap_or_default();
            self.mystery_boxes
                .take(&boxes, &map, checksum, Some(&server));
            self.mystery_boxes.taken = Some((revision, map, checksum, Some(server.clone())));
        }
        self.find_mystery_box_meshes();
        for moment in self.mystery_boxes.tick(now) {
            self.mystery_box_moment(moment);
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let own = u8::try_from(session.game_state().client_num).ok();
        // Other players' openings: the hub made the event, so it holds what came out.
        for event in events {
            if event.toy != wire::TOY || Some(event.slot) == own {
                continue;
            }
            if let Some(told) = wire::parse_event(&event.event) {
                self.mystery_boxes.opened(
                    told.id,
                    told.outcome,
                    None,
                    Some(event.slot),
                    told.tier.as_deref(),
                    seed(),
                    now,
                );
            }
        }
        // Their takes: the opener's client says so through the toy relay.
        for event in relay_events {
            if event.toy != wire::TOY || Some(event.slot) == own {
                continue;
            }
            if let Some(id) = parse_take(&event.event) {
                self.mystery_boxes.taken_by(id, event.slot, now);
            }
        }
        // The player's own takes, told to every other SJK player here, one at a time.
        if let Some(own) = own {
            let others = self.looks.sjk_slots() & !(1_u64 << own);
            if let Some((id, slot)) = self.mystery_boxes.next_take(others, now) {
                crate::player_identity::relay_toy_event(wire::TOY, take_event(id), slot, false);
            }
        }
        if self.mystery_boxes.here.is_empty() {
            self.mystery_boxes.use_pressed = false;
            return;
        }
        let player = &session.latest_snapshot().player;
        if !plays(player) {
            self.mystery_boxes.use_pressed = false;
            return;
        }
        let from = self
            .local_prediction
            .predicted_state()
            .map_or_else(|| player.origin(), |state| state.origin);
        let at = pick_box(
            Vec3::from_array(from),
            self.camera_yaw,
            &self.mystery_boxes.here,
        );
        let pressed = std::mem::take(&mut self.mystery_boxes.use_pressed);
        let credits = crate::player_identity::with_snapshot(|snapshot| {
            snapshot.me.as_ref().map(|me| me.credits)
        })
        .flatten();
        // What Use does at the box faced, by the crosshair (`use_prompt`).
        let boxes = &self.mystery_boxes;
        let prompt = at
            .and_then(|id| boxes.here.iter().find(|here| here.id == id))
            .and_then(|here| {
                box_prompt(
                    here,
                    &Wallet {
                        price: boxes.price,
                        free: boxes.free,
                        credits,
                    },
                    cooldown_left(boxes.last_mine, boxes.cooldown, now),
                    boxes
                        .asked
                        .is_some_and(|(_, asked)| now.duration_since(asked) < ASK_TIMEOUT),
                    now,
                )
            });
        if let Some(prompt) = prompt {
            self.want_use_prompt(prompt);
        }
        let (Some(id), true) = (at, pressed) else {
            return;
        };
        if let Some(taken) = self.mystery_boxes.take_prize(id) {
            self.mystery_box_moment(Moment::Taken(taken));
            // Told to the other SJK players here, whose box would float on till the hold
            // ends.
            self.mystery_boxes.takes.push((id, u64::MAX));
            return;
        }
        let busy = self
            .mystery_boxes
            .here
            .iter()
            .find(|here| here.id == id)
            .and_then(|here| here.opening.as_ref());
        // The prompt says why Use does nothing now: the box is busy, the player waits out
        // the cooldown or lacks the credits.
        if busy.is_some() {
            return;
        }
        if self
            .mystery_boxes
            .asked
            .is_some_and(|(_, asked)| now.duration_since(asked) < ASK_TIMEOUT)
        {
            return;
        }
        // One opening a player every `cooldown` seconds, so another may take a turn.
        if cooldown_left(
            self.mystery_boxes.last_mine,
            self.mystery_boxes.cooldown,
            now,
        )
        .is_some()
        {
            return;
        }
        if self.mystery_boxes.free == 0
            && credits.is_some_and(|credits| credits < self.mystery_boxes.price)
        {
            return;
        }
        if crate::player_identity::with_service(|service| service.open_mystery_box(id, server))
            .is_some()
        {
            self.mystery_boxes.asked = Some((id, now));
        }
    }

    /// Act on the hub's last answer to a mystery box request, once.
    fn read_mystery_box_answer(&mut self, now: Instant) {
        let seen = self.mystery_boxes.answer_seen;
        let Some((serial, outcome)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .mystery_box
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .cloned()
        })
        .flatten() else {
            return;
        };
        self.mystery_boxes.answer_seen = serial;
        let asked = self.mystery_boxes.asked.take();
        match outcome {
            Ok(BoxDone::Opened(opened)) => {
                // The hub says the free openings left (a box may give one); an older hub
                // does not, and a free opening was spent.
                match opened.free {
                    Some(free) => self.mystery_boxes.free = free,
                    None if opened.price == 0 => {
                        self.mystery_boxes.free = self.mystery_boxes.free.saturating_sub(1);
                    }
                    None => {}
                }
                let mine = Prize {
                    price: opened.price,
                    won: opened.won,
                    balance: opened.balance,
                    gift: opened.gift.clone(),
                    holocron: opened
                        .holocron
                        .as_ref()
                        .map(|holocron| holocron.tier.clone()),
                    shards: opened.shards,
                    blade: opened.blade.clone(),
                    kyber: opened.kyber.as_ref().map(|kyber| kyber.tier.clone()),
                    raygun: opened.raygun,
                    doom_coins: opened.doom_coins,
                };
                let own = self
                    .live_session
                    .as_ref()
                    .and_then(|session| u8::try_from(session.game_state().client_num).ok());
                self.mystery_boxes.last_mine = Some(now);
                self.mystery_boxes.opened(
                    opened.box_id,
                    &opened.outcome,
                    Some(mine),
                    own,
                    None,
                    seed(),
                    now,
                );
            }
            Ok(BoxDone::Placed(id)) => {
                self.mystery_box_print(format!("Mystery box {id} placed"));
            }
            Ok(BoxDone::Removed(id)) => {
                self.mystery_box_print(format!("Mystery box {id} removed"));
            }
            Ok(BoxDone::WheelPlaced(id)) => {
                self.mystery_box_print(format!("WHEEL OF DOOM {id} placed"));
            }
            Ok(BoxDone::WheelRemoved(id)) => {
                self.mystery_box_print(format!("WHEEL OF DOOM {id} removed"));
            }
            Err(why) if asked.is_some() => self.mystery_box_centre(why),
            Err(why) => self.mystery_box_print(format!("Mystery box: {why}")),
        }
    }

    /// Tell the player their roll stopped, or that they took what came out.
    fn mystery_box_moment(&mut self, moment: Moment) {
        match moment {
            Moment::Blast { opener, at } => self.mystery_box_blast(opener, at),
            Moment::Stopped { id, outcome, prize } => {
                // The Use prompt says to take it; the Ray Gun jumps into the hand at once.
                self.mystery_box_centre(prize_text(&outcome, &prize));
                if outcome == wire::RAYGUN
                    && let Some(taken) = self.mystery_boxes.take_prize(id)
                {
                    self.mystery_boxes.takes.push((id, u64::MAX));
                    self.mystery_box_moment(Moment::Taken(taken));
                }
            }
            Moment::Taken(taken) => {
                if taken.outcome == wire::RAYGUN
                    && let Some(seconds) = taken.prize.raygun
                {
                    self.raygun_start(seconds);
                }
                let text = prize_text(&taken.outcome, &taken.prize);
                let how = if taken.by_itself {
                    " (taken for you)"
                } else {
                    ""
                };
                self.mystery_box_print(format!(
                    "Mystery box: {text}{how} ({} credits left)",
                    taken.prize.balance
                ));
            }
        }
    }

    /// Send the box the creator asked to place once the map's checksum is known.
    fn place_pending_mystery_box(&mut self, map: &str, checksum: i32) {
        let Some(pending) = self.mystery_boxes.pending_place.take() else {
            return;
        };
        let placed = MysteryBox {
            id: 0,
            map: map.to_owned(),
            checksum,
            server: pending.server,
            origin: pending.origin.to_array(),
            yaw: pending.yaw,
        };
        if crate::player_identity::with_service(|service| service.place_mystery_box(placed))
            .is_none()
        {
            self.mystery_box_print("Mystery box: not connected to the SJK hub".to_owned());
        }
    }

    /// `mysterybox_place`, `mysterybox_remove` and `mysterybox_list`.
    pub(crate) fn mystery_box_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if name == "mysterybox_list" {
            return Ok(self.mystery_box_list());
        }
        if !crate::player_identity::is_creator() {
            return Err("Only SJK's creator places and removes mystery boxes".into());
        }
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .ok_or("Join a game server first: boxes stand on a server's map")?;
        let server = session.server().to_string();
        match name {
            "mysterybox_place" => {
                let one_server = match args.first().map(String::as_str) {
                    None => false,
                    Some("server") => true,
                    Some(other) => return Err(format!("mysterybox_place [server], not {other}")),
                };
                let (eye, forward) = self.world_notes.view().ok_or("No view to aim from")?;
                let hit = self
                    .decal_surfaces
                    .ray_hit(eye, forward, 4096.0)
                    .ok_or("Aim at the floor where the box should stand")?;
                let yaw = forward.y.atan2(forward.x).to_degrees() + 180.0;
                Ok(vec![self.mystery_box_place_at(
                    eye + forward * hit.distance,
                    yaw,
                    one_server.then_some(server),
                )])
            }
            "mysterybox_remove" => {
                let id = match args.first() {
                    Some(id) => id.parse().map_err(|_| format!("Not a box id: {id}"))?,
                    None => {
                        let from = Vec3::from_array(session.latest_snapshot().player.origin());
                        self.mystery_boxes
                            .here
                            .iter()
                            .map(|here| (here.id, here.origin.distance(from)))
                            .filter(|(_, distance)| *distance <= REMOVE_REACH)
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(id, _)| id)
                            .ok_or("No mystery box near you: give its id (mysterybox_list)")?
                    }
                };
                crate::player_identity::with_service(|service| service.remove_mystery_box(id))
                    .ok_or("Not connected to the SJK hub")?;
                Ok(vec![format!("Removing mystery box {id}...")])
            }
            _ => Err(format!("Unknown command {name}")),
        }
    }

    /// Ask the hub to place a box at `origin` facing `yaw` degrees, on every server
    /// running this map file or on `server` only, once the map's checksum is known
    /// (`mysterybox_place` aims; the browser map editor gives the spot). The caller has
    /// checked the creator and the server.
    pub(crate) fn mystery_box_place_at(
        &mut self,
        origin: Vec3,
        yaw: f32,
        server: Option<String>,
    ) -> String {
        let one_server = server.is_some();
        self.mystery_boxes.pending_place = Some(PendingPlace {
            origin,
            yaw: yaw.rem_euclid(360.0),
            server: server.unwrap_or_default(),
        });
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        self.map_edits.want_checksum(&map);
        format!(
            "Placing a mystery box on {map}{}...",
            if one_server { " for this server" } else { "" }
        )
    }

    fn mystery_box_list(&self) -> Vec<String> {
        let here = &self.mystery_boxes.here;
        if here.is_empty() {
            return vec!["No mystery box on this map".to_owned()];
        }
        let from = self
            .live_session
            .as_ref()
            .map(|session| Vec3::from_array(session.latest_snapshot().player.origin()));
        let mut lines = vec![format!(
            "{} mystery boxes here, {} credits to open:",
            here.len(),
            self.mystery_boxes.price
        )];
        for placed in here {
            let away = from.map_or(String::new(), |from| {
                format!(", {:.0} units away", placed.origin.distance(from))
            });
            lines.push(format!(
                "  {}: at {:.0} {:.0} {:.0}{away}",
                placed.id, placed.origin.x, placed.origin.y, placed.origin.z
            ));
        }
        lines
    }

    /// Add this frame's boxes to the scene: each standing, an opened one shaking while it
    /// rolls, and what its opening shows floating over it.
    pub(crate) fn append_mystery_box_instances(&mut self) {
        let now = Instant::now();
        let boxes = &self.mystery_boxes;
        let box_mesh = boxes.mesh_of(MODEL);
        let lid_mesh = boxes.mesh_of(LID_MODEL);
        for here in &boxes.here {
            let elapsed = here
                .opening
                .as_ref()
                .map(|opening| now.saturating_duration_since(opening.started));
            if let Some(mesh) = box_mesh {
                // Rattling while the roll runs, less and less as it slows.
                let shake = elapsed
                    .filter(|elapsed| *elapsed < ROLL)
                    .map_or(0.0, |elapsed| {
                        let t = elapsed.as_secs_f32();
                        (t * 47.0).sin() * 2.5 * (1.0 - t / ROLL.as_secs_f32())
                    });
                self.object_groups[mesh].push(ActorInstance::new(
                    here.origin.to_array(),
                    sjk_client::legacy_angles_to_quaternion([0.0, here.yaw + shake, 0.0]),
                    [1.0; 3],
                ));
                // The lid on its hinge, the chest's back top edge, pitched open.
                if let Some(lid) = lid_mesh {
                    self.object_groups[lid].push(ActorInstance::new(
                        lid_origin(here.origin, here.yaw + shake).to_array(),
                        sjk_client::legacy_angles_to_quaternion([
                            -here.lid.angle(now),
                            here.yaw + shake,
                            0.0,
                        ]),
                        [1.0; 3],
                    ));
                }
            }
            let (Some(opening), Some(elapsed)) = (&here.opening, elapsed) else {
                continue;
            };
            let item = opening.showing(elapsed);
            let (height, yaw) = float(elapsed, here.yaw);
            let rotation = sjk_client::legacy_angles_to_quaternion([0.0, yaw, 0.0]);
            // A holocron: each part of its form about its own centre, moving as it does.
            if let Some(slot) = item.form {
                let turn = Quat::from_array(rotation);
                let centre = here.origin + Vec3::Z * height;
                for part in FORMS[slot].parts {
                    let Some(mesh) = boxes.mesh_of(part.model) else {
                        continue;
                    };
                    let (at, turned) = part.pose(elapsed.as_secs_f32());
                    self.object_groups[mesh].push(ActorInstance::new(
                        (centre + turn * (at * item.scale)).to_array(),
                        (turn * turned).to_array(),
                        [item.scale; 3],
                    ));
                }
                continue;
            }
            let Some(mesh) = boxes.mesh_of(item.model) else {
                continue;
            };
            // The model's own centre over the box, whatever its origin; a pile's copies
            // round it, each leaning its own way.
            let centre = Vec3::from_array(self.object_meshes[mesh].center) * item.scale;
            let turn = Quat::from_array(rotation);
            for (at, lean) in pile_spots(item.copies) {
                let turned = turn * lean;
                let origin =
                    here.origin + Vec3::Z * height + turn * (at * item.scale) - turned * centre;
                self.object_groups[mesh].push(ActorInstance::new(
                    origin.to_array(),
                    turned.to_array(),
                    [item.scale; 3],
                ));
            }
        }
    }

    /// Play the boxes' one-shot sounds; the sounds are handed to the audio the first
    /// time there is one.
    pub(crate) fn play_mystery_box_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.mystery_boxes.cues.clear();
            return;
        };
        if self.mystery_boxes.cues.is_empty() {
            return;
        }
        // The explosions of boxes that blew up; their sound is the box's own cue, so
        // "Mystery box sounds" silences it.
        if let Some(vfs) = self.vfs.clone() {
            for stands in self.mystery_boxes.blasts.drain(..) {
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    BLAST_EFFECT,
                    stands + Vec3::Z * 8.0,
                    Instant::now(),
                    seed(),
                    0,
                    &mut None,
                    crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]),
                );
            }
        }
        if self.mystery_boxes.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(
                vfs,
                [
                    OPEN,
                    ROLL_SOUND,
                    EXTRAS_OPEN,
                    EXTRAS_ROLL,
                    EXTRAS_CLOSE,
                    TAKE_SOUND,
                    WIN,
                    JACKPOT,
                    EMPTY,
                    HUNT_MARK,
                    BLAST_SOUND,
                    LAUGH,
                    LAUGH_FALLBACK,
                ],
            );
            self.mystery_boxes.sounds_registered = Some(audio.generation());
        }
        for cue in self.mystery_boxes.cues.drain(..) {
            // The extras pack's sounds: without the pack, what stands in for them.
            let path = match EXTRAS.iter().find(|(extra, _)| *extra == cue.path) {
                Some((extra, fallback)) if !audio.has_sound(extra) => match fallback {
                    Some(fallback) => fallback,
                    None => continue,
                },
                _ => cue.path,
            };
            audio.play_toy(
                crate::toy_sounds::Toy::MysteryBox,
                path,
                1.0,
                cue.origin,
                cue.source,
                // `CHAN_AUTO`: the crate's slide, the tune and what came out overlap
                // rather than cut each other off.
                sjk_audio::ChannelId(0),
            );
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: a box at each of `boxes` (origin and yaw), opened `since` ago
    /// on a holocron of tier `n % 4` for the `n`th: the roll shows its tier's holocron once
    /// `since` passes the roll.
    pub(crate) fn shot_mystery_boxes(&mut self, boxes: &[([f32; 3], f32)], since: Duration) {
        self.shot_mystery_box_prizes(boxes, since, &HOLOCRONS);
    }

    /// [`Self::shot_mystery_boxes`], stopped on the prizes of 29/10/2026: a pile of
    /// shards, a big one, a free opening and the boxes' blade.
    pub(crate) fn shot_mystery_box_loot(&mut self, boxes: &[([f32; 3], f32)], since: Duration) {
        self.shot_mystery_box_prizes(boxes, since, &[PRIZES[6], BIG_PILE, PRIZES[7], PRIZES[8]]);
    }

    /// [`Self::shot_mystery_boxes`], stopped on the loot of 11/11/2026: the three piles of
    /// gold coins and the Ray Gun, or with `kybers` the four kyber crystals.
    pub(crate) fn shot_mystery_box_rework(
        &mut self,
        boxes: &[([f32; 3], f32)],
        since: Duration,
        kybers: bool,
    ) {
        let loot = if kybers {
            KYBERS
        } else {
            [PRIZES[1], PRIZES[2], PRIZES[3], PRIZES[9]]
        };
        self.shot_mystery_box_prizes(boxes, since, &loot);
    }

    /// For the world shots: the first of `boxes` closed, the others stopped on the teddy
    /// bear of a box that blows up (30/10/2026).
    pub(crate) fn shot_mystery_box_teddy(&mut self, boxes: &[([f32; 3], f32)], since: Duration) {
        self.shot_mystery_box_prizes(boxes, since, &[PRIZES[4]]);
        if let Some(first) = self.mystery_boxes.here.first_mut() {
            first.opening = None;
            first.lid = Lid::default();
        }
    }

    fn shot_mystery_box_prizes(
        &mut self,
        boxes: &[([f32; 3], f32)],
        since: Duration,
        prizes: &[Item],
    ) {
        let started = Instant::now() - since;
        self.mystery_boxes.shot = true;
        self.mystery_boxes.here = boxes
            .iter()
            .enumerate()
            .map(|(index, (origin, yaw))| Here {
                id: index as i64 + 1,
                origin: Vec3::from_array(*origin),
                yaw: *yaw,
                opening: Some(Opening {
                    started,
                    outcome: wire::HOLOCRON.to_owned(),
                    prize: prizes[index % prizes.len()],
                    seed: index as u32,
                    mine: None,
                    opener: None,
                    revealed: true,
                    blasted: false,
                }),
                lid: Lid {
                    open: true,
                    since: Some(started),
                },
            })
            .collect();
    }
}

/// How long the roll takes, for the world shots.
#[cfg(test)]
pub(crate) const SHOT_ROLL: Duration = ROLL;

#[cfg(test)]
mod tests {
    use super::*;

    fn here(id: i64, origin: [f32; 3]) -> Here {
        Here {
            id,
            origin: Vec3::from_array(origin),
            yaw: 0.0,
            opening: None,
            lid: Lid::default(),
        }
    }

    fn prize_of(won: u64) -> Prize {
        Prize {
            price: 100,
            won,
            balance: 900,
            ..Prize::default()
        }
    }

    #[test]
    fn the_box_faced_within_reach_is_the_one_opened() {
        let boxes = [here(1, [60.0, 0.0, 0.0]), here(2, [40.0, 10.0, 0.0])];
        // Facing +x: the nearer of the two.
        assert_eq!(pick_box(Vec3::new(0.0, 0.0, 24.0), 0.0, &boxes), Some(2));
        // Facing away: neither.
        assert_eq!(
            pick_box(Vec3::new(0.0, 0.0, 24.0), std::f32::consts::PI, &boxes),
            None
        );
        // Too far across, or too far up.
        assert_eq!(
            pick_box(Vec3::new(-50.0, 0.0, 24.0), 0.0, &boxes[..1]),
            None
        );
        assert_eq!(pick_box(Vec3::new(0.0, 0.0, 200.0), 0.0, &boxes), None);
        // Standing on it: any facing.
        assert_eq!(
            pick_box(Vec3::new(60.0, 0.0, 40.0), 2.0, &boxes[..1]),
            Some(1)
        );
    }

    #[test]
    fn boxes_are_taken_for_their_map_file_and_server() {
        let mut state = MysteryBoxes::default();
        let placed = |id, map: &str, checksum, server: &str| MysteryBox {
            id,
            map: map.to_owned(),
            checksum,
            server: server.to_owned(),
            origin: [1.0, 2.0, 3.0],
            yaw: 90.0,
        };
        let boxes = wire::Boxes {
            boxes: vec![
                placed(1, "mp/ffa3", 5, ""),
                placed(2, "mp/ffa3", 6, ""),
                placed(3, "mp/ffa3", 5, "1.2.3.4:29070"),
                placed(4, "mp/ffa2", 5, ""),
            ],
            price: 100,
            ..Default::default()
        };
        assert_eq!(state.wanted_models().count(), 0);
        state.take(&boxes, "mp/ffa3", 5, Some("5.6.7.8:29070"));
        let ids: Vec<i64> = state.here.iter().map(|here| here.id).collect();
        assert_eq!(ids, [1]);
        state.take(&boxes, "mp/ffa3", 5, Some("1.2.3.4:29070"));
        let ids: Vec<i64> = state.here.iter().map(|here| here.id).collect();
        assert_eq!(ids, [1, 3]);
        assert_eq!(state.price, 100);
        // The box, its lid and everything the roll shows, each once.
        let wanted: Vec<&str> = state.wanted_models().collect();
        assert_eq!(wanted[..2], [MODEL, LID_MODEL]);
        assert_eq!(wanted.len(), models().count());
        let mut unique = wanted.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), wanted.len(), "each once");
        state.meshes = vec![Some(0); wanted.len()];
        assert_eq!(state.wanted_models().count(), 0);
    }

    #[test]
    fn the_roll_slows_to_a_stop_on_what_came_out() {
        let opening = Opening {
            started: Instant::now(),
            outcome: "jackpot".to_owned(),
            prize: prize("jackpot", true, None),
            seed: 12345,
            mine: None,
            opener: None,
            revealed: false,
            blasted: false,
        };
        // Changing often early on, the prize over the last second and after.
        let early: std::collections::HashSet<&str> = (0..20)
            .map(|step| opening.showing(Duration::from_millis(step * 50)).model)
            .collect();
        assert!(early.len() > 3, "{early:?}");
        for ms in [ROLL.as_millis() as u64 - 700, 5_500, 12_000] {
            assert_eq!(opening.showing(Duration::from_millis(ms)), PRIZES[3]);
        }
        assert_eq!(prize("empty", false, None), PRIZES[0]);
        assert_eq!(prize("newer_thing", true, None), PRIZES[2]);
        // The prizes of 29/10/2026; the roll shows nothing a box cannot give.
        assert_eq!(prize(wire::SHARD_PILE, false, None), PRIZES[6]);
        assert_eq!(prize(wire::AGAIN, false, None), PRIZES[7]);
        assert_eq!(prize(wire::BLADE, false, None), PRIZES[8]);
        for item in REEL {
            assert!(
                PRIZES.contains(&item)
                    || HOLOCRONS.contains(&item)
                    || KYBERS.contains(&item)
                    || item == BIG_PILE,
                "{}",
                item.model
            );
        }
        // 11/11/2026: coins for credits, the Ray Gun, the crystals, and never the
        // placeholder holocron, the bolts or the little box rolling by.
        assert_eq!(prize(wire::COINS, true, None).model, COINS_SMALL);
        assert_eq!(prize("double", true, None).model, COINS_MEDIUM);
        assert_eq!(prize("jackpot", true, None).model, COINS_BIG);
        assert_eq!(prize(wire::RAYGUN, false, None), PRIZES[9]);
        assert_eq!(prize(wire::KYBER, false, Some("kyber_mythical")), KYBERS[3]);
        assert_eq!(prize(wire::KYBER, false, Some("rare")), KYBERS[1]);
        assert_eq!(prize(wire::KYBER, false, None), KYBERS[1]);
        for gone in [PRIZES[0], PRIZES[5], PRIZES[7]] {
            assert!(!REEL.contains(&gone), "{}", gone.model);
        }
        assert_eq!(pile_spots(1).count(), 1);
        assert_eq!(pile_spots(BIG_PILE.copies).count(), 7);
        // A holocron stops on its tier's own, the plain one when the tier is not told.
        assert_eq!(
            prize(wire::HOLOCRON, false, Some("legendary")),
            HOLOCRONS[2]
        );
        assert_eq!(prize(wire::HOLOCRON, false, None), PRIZES[5]);
        assert_eq!(prize(wire::HOLOCRON, false, Some("later_tier")), PRIZES[5]);
        assert_eq!(prize(wire::HOLOCRON, false, Some("kyber_rare")), PRIZES[5]);
        // Every part of every tier's holocron is among the models the boxes load.
        for item in HOLOCRONS {
            assert_eq!(FORMS[item.form.unwrap()].parts[0].model, item.model);
            for model in item.models() {
                assert!(models().any(|known| known == model), "{model}");
            }
        }
        assert!(models().any(|known| known == crate::illuminate::MODEL));
        // Rising out of the box, then floating over it.
        assert!(float(Duration::ZERO, 0.0).0 < 33.0);
        assert!((float(Duration::from_secs(3), 0.0).0 - FLOAT).abs() <= 1.5);
    }

    #[test]
    fn an_opening_rolls_stops_and_waits_to_be_taken() {
        let mut state = MysteryBoxes {
            here: vec![here(7, [0.0; 3])],
            ..Default::default()
        };
        let now = Instant::now();
        state.opened(7, "double", Some(prize_of(200)), Some(3), None, 1, now);
        // Each at the box, on the box's own source: never an entity's, which the audio
        // moves to where that entity is (#185).
        assert!(
            state
                .cues
                .iter()
                .all(|cue| cue.origin == Some([0.0; 3]) && cue.source == source(7))
        );
        assert_ne!(source(7), sjk_audio::SourceId(0));
        assert!(source(7).0 >= SOURCE_BASE && source(7) != source(8));
        // The hunt's source is no box's.
        assert!((0..=0xFFFF).all(|id| source(id) != HUNT_SOURCE));
        let paths: Vec<&str> = state.cues.drain(..).map(|cue| cue.path).collect();
        // The extras pack's opening and roll, or the bundled ones without it.
        assert_eq!(paths, [EXTRAS_OPEN, EXTRAS_ROLL]);
        assert!(EXTRAS.contains(&(EXTRAS_OPEN, Some(OPEN))));
        assert!(EXTRAS.contains(&(EXTRAS_ROLL, Some(ROLL_SOUND))));
        // The lid swings open, and stays open while it rolls.
        assert_eq!(state.here[0].lid.angle(now), 0.0);
        assert_eq!(state.here[0].lid.angle(now + LID_OPENING), LID_OPEN);
        // Not taken while it rolls; the box is busy.
        assert!(state.tick(now + Duration::from_secs(2)).is_empty());
        assert_eq!(state.take_prize(7), None);
        // The roll stops: its sound, and the player is told.
        let moments = state.tick(now + ROLL);
        assert_eq!(
            moments,
            [Moment::Stopped {
                id: 7,
                outcome: "double".to_owned(),
                prize: prize_of(200),
            }]
        );
        assert_eq!(state.cues.drain(..).next().map(|cue| cue.path), Some(WIN));
        // Taken with Use.
        let taken = state.take_prize(7).expect("taken");
        assert!(!taken.by_itself);
        assert_eq!(taken.prize.won, 200);
        let paths: Vec<&str> = state.cues.drain(..).map(|cue| cue.path).collect();
        assert_eq!(paths, [TAKE_SOUND, EXTRAS_CLOSE]);
        assert!(state.here[0].opening.is_none());
        assert!(!state.here[0].lid.open);
        assert!(state.tick(now + ROLL + HOLD).is_empty());
    }

    #[test]
    fn what_is_left_is_taken_by_itself_after_the_hold() {
        let mut state = MysteryBoxes {
            here: vec![here(7, [0.0; 3]), here(8, [99.0; 3])],
            ..Default::default()
        };
        let now = Instant::now();
        state.opened(7, "empty", Some(prize_of(0)), Some(3), None, 1, now);
        // Another player's opening: rolls and goes, nothing taken for the player.
        state.opened(8, "jackpot", None, Some(5), None, 2, now);
        let moments = state.tick(now + ROLL);
        assert_eq!(moments.len(), 1);
        let stops: Vec<&str> = state.cues[4..].iter().map(|cue| cue.path).collect();
        assert_eq!(stops, [EMPTY, JACKPOT]);
        assert_eq!(state.take_prize(8), None);
        assert!(state.tick(now + ROLL + HOLD / 2).is_empty());
        let moments = state.tick(now + ROLL + HOLD);
        let [Moment::Taken(taken)] = moments.as_slice() else {
            panic!("{moments:?}");
        };
        assert!(taken.by_itself);
        assert_eq!(taken.outcome, "empty");
        assert!(state.here.iter().all(|here| here.opening.is_none()));
    }

    #[test]
    fn results_read_as_the_player_is_told() {
        assert_eq!(
            result_text("empty", 100, 0),
            "The box was empty: -100 credits"
        );
        assert_eq!(result_text("jackpot", 100, 500), "JACKPOT! +500 credits");
        assert_eq!(result_text("refund", 0, 100), "+100 credits");
        assert_eq!(
            result_text("empty", 0, 0),
            "The box was empty: your free roll is spent"
        );
        assert_eq!(result_text("newer_thing", 100, 300), "+300 credits");
        assert_eq!(
            result_text("newer_thing", 100, 0),
            "Nothing inside: -100 credits"
        );
    }

    #[test]
    fn shards_a_free_roll_and_the_blade_say_what_they_gave() {
        let got = |shards: Option<u64>, blade: Option<&str>| Prize {
            price: 100,
            shards,
            blade: blade.map(str::to_owned),
            ..Prize::default()
        };
        assert_eq!(
            prize_text(wire::SHARD_PILE, &got(Some(18), None)),
            "+18 kyber shards"
        );
        assert_eq!(
            prize_text(wire::AGAIN, &got(None, None)),
            "A box in the box: your next roll is free"
        );
        assert_eq!(
            prize_text(wire::BLADE, &got(None, Some(wire::BLADE_ID))),
            "The Mystery blade! In your Collection"
        );
        assert_eq!(
            prize_text(wire::BLADE, &got(Some(60), None)),
            "The Mystery blade, again: +60 kyber shards"
        );
    }

    #[test]
    fn a_box_that_blew_up_or_gave_a_holocron_says_what_became_of_it() {
        let paid = |gift: Option<wire::Gift>| Prize {
            price: 100,
            gift,
            ..Prize::default()
        };
        let gift = wire::Gift {
            slot: 4,
            name: "^1Jan".to_owned(),
            credits: 100,
        };
        assert_eq!(
            prize_text(wire::EXPLODE, &paid(Some(gift))),
            "BOOM! The box blew up: your 100 credits went to ^1Jan^7"
        );
        assert_eq!(
            prize_text(wire::EXPLODE, &paid(None)),
            "BOOM! The box blew up: -100 credits"
        );
        assert_eq!(
            prize_text(wire::EXPLODE, &Prize::default()),
            "BOOM! The box blew up: your free roll is spent"
        );
        let found = Prize {
            holocron: Some("legendary".to_owned()),
            ..paid(None)
        };
        assert_eq!(
            prize_text(wire::HOLOCRON, &found),
            "Legendary Holocron! In your Collection"
        );
        // The others read as before.
        assert_eq!(
            prize_text(
                "double",
                &Prize {
                    won: 200,
                    ..paid(None)
                }
            ),
            "A pile of gold! +200 credits"
        );
        assert_eq!(
            prize_text(
                wire::COINS,
                &Prize {
                    won: 50,
                    ..paid(None)
                }
            ),
            "Gold coins! +50 credits"
        );
        let crystal = Prize {
            kyber: Some("kyber_rare".to_owned()),
            ..Prize::default()
        };
        assert_eq!(
            prize_text(wire::KYBER, &crystal),
            "Rare Kyber Crystal! In your Collection"
        );
        let gun = Prize {
            raygun: Some(30),
            ..Prize::default()
        };
        assert!(prize_text(wire::RAYGUN, &gun).starts_with("THE RAY GUN! 30 s"));
        // The teddy bear's DOOM coin.
        let coined = Prize {
            doom_coins: Some(1),
            ..paid(None)
        };
        assert!(prize_text(wire::EXPLODE, &coined).contains("+1 DOOM coin"));
        // What the roll stops on, and the models it loads.
        assert_eq!(prize(wire::EXPLODE, false, None), PRIZES[4]);
        assert_eq!(prize(wire::HOLOCRON, false, None), PRIZES[5]);
        assert!(models().any(|model| model == PRIZES[4].model));
        assert!(models().any(|model| model == PRIZES[5].model));
    }

    #[test]
    fn a_box_that_blows_up_shows_the_teddy_bear_laughs_then_blows_up() {
        let mut state = MysteryBoxes {
            here: vec![here(7, [0.0; 3])],
            ..Default::default()
        };
        let now = Instant::now();
        let paid = Prize {
            price: 100,
            balance: 400,
            ..Prize::default()
        };
        state.opened(7, wire::EXPLODE, Some(paid.clone()), Some(3), None, 1, now);
        state.cues.clear();
        // The bear only comes at the end: the roll never shows it before.
        assert!(!REEL.contains(&PRIZES[4]));
        assert_eq!(PRIZES[4].model, TEDDY_MODEL);
        let opening = state.here[0].opening.clone().unwrap();
        assert_eq!(opening.showing(ROLL), PRIZES[4]);
        // The roll stops on the bear: the laugh, and nothing blows up yet.
        assert!(state.tick(now + ROLL).is_empty());
        let paths: Vec<&str> = state.cues.drain(..).map(|cue| cue.path).collect();
        assert_eq!(paths, [LAUGH]);
        assert!(state.tick(now + ROLL + BLAST_DELAY / 2).is_empty());
        // Then it blows up, once, and the opener is told.
        let moments = state.tick(now + ROLL + BLAST_DELAY);
        assert_eq!(
            moments,
            [
                Moment::Blast {
                    opener: Some(3),
                    at: Vec3::ZERO,
                },
                Moment::Stopped {
                    id: 7,
                    outcome: wire::EXPLODE.to_owned(),
                    prize: paid,
                },
            ]
        );
        assert_eq!(
            state.cues.drain(..).map(|cue| cue.path).collect::<Vec<_>>(),
            [BLAST_SOUND]
        );
        assert!(state.tick(now + ROLL + BLAST_DELAY * 2).is_empty());
        // The laugh is the extras pack's, the retail gloat without it.
        assert!(EXTRAS.contains(&(LAUGH, Some(LAUGH_FALLBACK))));
        // The lid closing sounds only with the extras pack.
        assert!(EXTRAS.contains(&(EXTRAS_CLOSE, None)));
        state.tick(now + ROLL + HOLD);
        assert!(state.cues.iter().any(|cue| cue.path == EXTRAS_CLOSE));
    }

    #[test]
    fn the_use_prompt_says_what_use_does_at_the_box() {
        let now = Instant::now();
        let wallet = Wallet {
            price: 100,
            free: 0,
            credits: Some(500),
        };
        let at = |here: &Here, wallet: &Wallet, cooldown, asked| {
            box_prompt(here, wallet, cooldown, asked, now)
                .map(|prompt| (prompt.action, prompt.line, prompt.ready))
        };
        let idle = here(1, [0.0; 3]);
        let open = "Open mystery box".to_owned();
        assert_eq!(
            at(&idle, &wallet, None, false),
            Some((open.clone(), Some("100 credits".to_owned()), true))
        );
        let free = Wallet { free: 1, ..wallet };
        assert_eq!(
            at(&idle, &free, None, false),
            Some((open.clone(), Some("Free roll".to_owned()), true))
        );
        let poor = Wallet {
            credits: Some(40),
            ..wallet
        };
        assert_eq!(
            at(&idle, &poor, None, false),
            Some((
                open.clone(),
                Some("100 credits: not enough".to_owned()),
                false
            ))
        );
        assert_eq!(
            at(&idle, &wallet, Some(9), false),
            Some((open, Some("Your turn again in 9 s".to_owned()), false))
        );
        assert_eq!(
            at(&idle, &wallet, None, true),
            Some(("Opening...".to_owned(), None, false))
        );
        // Another player's roll; the player's own, rolling, then waiting to be taken.
        let mut state = MysteryBoxes {
            here: vec![here(1, [0.0; 3]), here(2, [0.0; 3]), here(3, [0.0; 3])],
            ..Default::default()
        };
        let started = now - ROLL;
        state.opened(1, "double", None, Some(5), None, 1, now);
        state.opened(2, "double", Some(prize_of(200)), Some(3), None, 1, now);
        state.opened(3, "double", Some(prize_of(200)), Some(3), None, 1, started);
        assert_eq!(
            at(&state.here[0], &wallet, None, false),
            Some(("Someone is opening it".to_owned(), None, false))
        );
        assert_eq!(at(&state.here[1], &wallet, None, false), None);
        assert_eq!(
            at(&state.here[2], &wallet, None, false),
            Some(("Take it".to_owned(), None, true))
        );
        // A box that blows up says nothing: the player is about to die.
        state.opened(
            1,
            wire::EXPLODE,
            Some(prize_of(0)),
            Some(3),
            None,
            1,
            started,
        );
        assert_eq!(at(&state.here[0], &wallet, None, false), None);
    }

    #[test]
    fn a_take_is_told_to_each_other_player_and_closes_their_box() {
        let now = Instant::now();
        // The opener's side: one other player at a time, the gap between.
        let mut opener = MysteryBoxes::default();
        opener.takes.push((7, u64::MAX));
        let others = 0b1010;
        assert_eq!(opener.next_take(others, now), Some((7, 1)));
        assert_eq!(opener.next_take(others, now), None);
        assert_eq!(opener.next_take(others, now + TAKE_GAP), Some((7, 3)));
        assert_eq!(opener.next_take(others, now + TAKE_GAP * 2), None);
        assert!(opener.takes.is_empty());
        assert_eq!(parse_take(&take_event(7)), Some(7));
        assert_eq!(parse_take("open_7_double"), None);
        // A watcher's side: only the opener's take closes the box.
        let mut watcher = MysteryBoxes {
            here: vec![here(7, [0.0; 3])],
            ..Default::default()
        };
        watcher.opened(7, "double", None, Some(5), None, 1, now);
        watcher.cues.clear();
        watcher.taken_by(7, 6, now + ROLL);
        assert!(watcher.here[0].opening.is_some(), "not the opener");
        watcher.taken_by(7, 5, now + ROLL);
        assert!(watcher.here[0].opening.is_none());
        assert!(!watcher.here[0].lid.open);
        let paths: Vec<&str> = watcher.cues.drain(..).map(|cue| cue.path).collect();
        assert_eq!(paths, [TAKE_SOUND, EXTRAS_CLOSE]);
    }

    #[test]
    fn the_player_waits_out_the_cooldown_between_two_openings() {
        let now = Instant::now();
        assert_eq!(cooldown_left(None, 15, now), None);
        assert_eq!(cooldown_left(Some(now), 15, now), Some(15));
        assert_eq!(
            cooldown_left(Some(now), 15, now + Duration::from_millis(5_500)),
            Some(10)
        );
        assert_eq!(
            cooldown_left(Some(now), 15, now + Duration::from_secs(15)),
            None
        );
        assert_eq!(cooldown_left(Some(now), 0, now), None);
    }
}
