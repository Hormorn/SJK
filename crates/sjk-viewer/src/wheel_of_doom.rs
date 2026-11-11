//! The WHEEL OF DOOM (Sol's design of 07/11/2026, placed on maps since 11/11/2026;
//! `docs/wheel-of-doom.md`): wheels SJK's creator places on maps through the hub, as
//! mystery boxes are, which any SJK player spins with `+use` for a DOOM coin (the teddy
//! bear in a mystery box brings one). The hub spends the coin, keeps the cooldown and
//! rolls the outcome (`sjk_identity::wheel_of_doom`); this module draws the wheels
//! standing on the loaded map file, asks for the spin at the one the player faces, spins
//! it to the end of its music ([`SPIN`]), slowing, onto the hub's segment under the
//! pointer, and plays what it landed on: one of the [`effects`] on the spinner's own input
//! and view, the game's `kill`, a curse, nothing, the coin back or a holocron. The other
//! SJK players on the server see and hear the same wheel spin through the hub's toy event
//! (`wheel_of_doom`, `land_<wheel>_<outcome>`). No game server knows of the wheels: they
//! are drawn by SJK clients only and have no collision.

mod draw;
mod effects;
mod mirror;

pub(crate) use effects::{Effect, Effects, Saber};

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::{Quat, Vec3};
use sjk_identity::mystery_box::MysteryBox;
use sjk_identity::wheel_of_doom as wire;
use std::time::{Duration, Instant};

/// The toy's command: since 11/11/2026 it tells the player's DOOM coins and where the
/// wheels are (before, it stood a wheel before the spinner for credits).
pub(crate) const COMMAND: &str = "toy_wheelofdoom";
/// Its line in `cmdlist`.
pub(crate) const HELP: &str = "Toy: the WHEEL OF DOOM stands on maps; spin one with Use for a DOOM coin (the teddy bear in a mystery box brings one). This tells your coins";

/// The toy's icon (key bindings; Collection > Toys draws the quick wheel atlas's copy).
pub(crate) const ICON: &str = "gfx/sjk/toy_wheelofdoom.png";
/// The stand (with the pointer) and the wheel (bundled, `scripts/wheel_of_doom_model.py`;
/// the contract Sol's model keeps is in `docs/wheel-of-doom.md`).
pub(crate) const STAND_MODEL: &str = "models/sjk/wheel_of_doom_stand.md3";
pub(crate) const WHEEL_MODEL: &str = "models/sjk/wheel_of_doom_wheel.md3";
/// Both, loaded with every map.
pub(crate) const MODELS: [&str; 2] = [STAND_MODEL, WHEEL_MODEL];
/// The wheel's radius, and how high its axle stands over the stand's origin (the floor).
const RADIUS: f32 = 40.0;
const AXLE: f32 = 64.0;
/// The segments round the wheel, and the angle each takes (20 since 11/11/2026: the
/// holocron of every tier and the coin back joined the 16).
pub(crate) const SEGMENTS: usize = 20;
const SEGMENT_DEGREES: f32 = 360.0 / SEGMENTS as f32;
/// How long the wheel spins: Sol's WHEEL OF DOOM music, 34.0 s, to whose end it lands
/// (11/11/2026; 6 s before); how many whole turns it makes before the last; and how long
/// its words say what it landed on.
const SPIN: Duration = Duration::from_millis(34_000);
const TURNS: f32 = 14.0;
const HOLD: Duration = Duration::from_secs(6);
/// How long after the stop a wheel landing on `explode` blows its spinner up.
const BLAST_DELAY: Duration = Duration::from_millis(1_500);
/// How long a spin waits for the hub before the player may ask again.
const ASK_TIMEOUT: Duration = Duration::from_secs(10);
/// The cooldown when the hub has not said it yet, seconds (a spin and a few seconds).
const DEFAULT_COOLDOWN: u64 = 40;
/// The most wheels kept on a map, and whose words are drawn (the nearest).
const MOST: usize = 8;
/// Farthest a player may stand from a wheel to spin it, across the floor and up or down,
/// and the widest angle between their facing and the wheel (radians, 60 degrees).
const REACH: f32 = 120.0;
const REACH_UP: f32 = 96.0;
const FACING: f32 = std::f32::consts::FRAC_PI_3;
/// Farthest a wheel `wheelofdoom_remove` takes without an id may stand.
const REMOVE_REACH: f32 = 256.0;

/// A peg passing the pointer, the stop, the holocrons' fanfare and the Quack effect's
/// quack (bundled, `scripts/wheel_of_doom_assets.py`).
const TICK: &str = "sound/sjk/wheel_of_doom/tick.wav";
const STOP: &str = "sound/sjk/wheel_of_doom/stop.wav";
const FANFARE: &str = "sound/sjk/wheel_of_doom/fanfare.wav";
const QUACK: &str = "sound/sjk/wheel_of_doom/quack.wav";
/// Sol's WHEEL OF DOOM music, played at the wheel for its whole spin, in the optional SJK
/// extras pack (`unofficial_sjk_extras.pk3`, the Sith cloud's `extras`); without the pack
/// the wheel spins to its ticks alone.
const MUSIC: &str = "sound/sjk/wheel_of_doom/music.mp3";
/// The laugh on a bad outcome, the mystery box's: Sol's laugh in the optional SJK extras
/// pack, else the retail gloat.
const LAUGH: &str = "sound/sjk/mystery_box/laugh.mp3";
const LAUGH_FALLBACK: &str = "sound/chars/alora/misc/gloat1.mp3";
/// The explosion where a spinner landing on `explode` stands, the mystery box's.
const BLAST_SOUND: &str = "sound/weapons/detpack/explode.mp3";
const BLAST_EFFECT: &str = "detpack/explosion";
/// The wheels' sound sources, past the other toys' (`0x5000_0000` to `0x5500_0000`):
/// one a wheel (its id), and one for the player's own quack.
const SOURCE_BASE: u32 = 0x5600_0000;

/// A bundled file's bytes, by its name in `assets/wheel_of_doom`.
macro_rules! bundled {
    ($name:literal) => {
        include_bytes!(concat!("../assets/wheel_of_doom/", $name))
    };
}

/// The bundled files at their game paths.
const FILES: [(&str, &[u8]); 10] = [
    (STAND_MODEL, bundled!("wheel_of_doom_stand.md3")),
    (WHEEL_MODEL, bundled!("wheel_of_doom_wheel.md3")),
    (
        "models/sjk/wheel_of_doom.jpg",
        bundled!("wheel_of_doom.jpg"),
    ),
    (
        "models/sjk/wheel_of_doom_glow.jpg",
        bundled!("wheel_of_doom_glow.jpg"),
    ),
    (
        "shaders/sjk_wheel_of_doom.shader",
        bundled!("wheel_of_doom.shader"),
    ),
    (
        "gfx/sjk/toy_wheelofdoom.png",
        bundled!("toy_wheelofdoom.png"),
    ),
    (TICK, bundled!("tick.wav")),
    (STOP, bundled!("stop.wav")),
    (FANFARE, bundled!("fanfare.wav")),
    (QUACK, bundled!("quack.wav")),
];

/// Mount the bundled files.
pub(crate) fn mount(vfs: &mut sjk_vfs::VirtualFileSystem) -> Result<(), sjk_vfs::VfsError> {
    vfs.mount_memory("SJK wheel of doom", FILES.iter().copied())?;
    Ok(())
}

/// The creator's console commands, with their help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        "wheelofdoom_place",
        "SJK creator: place a WHEEL OF DOOM where you aim, facing you ('server': for this server only)",
    ),
    (
        "wheelofdoom_remove",
        "SJK creator: remove the WHEEL OF DOOM nearest you, or the one with the id given",
    ),
    (
        "wheelofdoom_list",
        "List the WHEELS OF DOOM on this map, their ids, and your DOOM coins",
    ),
];

/// Whether `name` is one of [`COMMANDS`].
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

/// What a segment does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A holocron of this tier (`mythical`, the gold jackpot, `legendary`, `rare`,
    /// `uncommon`).
    Holocron(&'static str),
    /// The DOOM coin back: spin again.
    CoinBack,
    Nothing,
    Effect(Effect),
    /// The game's `kill`, as a mystery box that blows up.
    Explode,
    /// Another bad effect, later.
    Curse,
}

impl Kind {
    /// The wheel laughs as it stops on it.
    fn bad(self) -> bool {
        matches!(self, Self::Effect(_) | Self::Explode | Self::Curse)
    }
}

/// One segment: its hub id, what it does, its name over the wheel and what it says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) id: &'static str,
    pub(crate) kind: Kind,
    pub(crate) title: &'static str,
    pub(crate) line: &'static str,
}

const fn outcome(id: &'static str, kind: Kind, title: &'static str, line: &'static str) -> Outcome {
    Outcome {
        id,
        kind,
        title,
        line,
    }
}

const fn effect(
    id: &'static str,
    effect: Effect,
    title: &'static str,
    line: &'static str,
) -> Outcome {
    outcome(id, Kind::Effect(effect), title, line)
}

/// The segments in the wheel's order, as the hub's contract of 11/11/2026 lists them
/// (segment 0, the gold Mythical jackpot, under the pointer at angle 0, the others
/// clockwise; the holocrons spread round the wheel among the bad ones).
pub(crate) const OUTCOMES: [Outcome; SEGMENTS] = [
    outcome(
        "mythical",
        Kind::Holocron("mythical"),
        "MYTHICAL HOLOCRON!",
        "The jackpot",
    ),
    effect(
        "reverse_keys",
        Effect::ReverseKeys,
        "REVERSE KEYS",
        "Forward is back for 30 s",
    ),
    outcome(
        "holo_uncommon",
        Kind::Holocron("uncommon"),
        "UNCOMMON HOLOCRON",
        "In your Collection",
    ),
    effect(
        "invert_mouse",
        Effect::InvertMouse,
        "INVERTED MOUSE",
        "The mouse turns the wrong way for 30 s",
    ),
    outcome("explode", Kind::Explode, "BOOM!", "It blows up"),
    outcome(
        "holo_rare",
        Kind::Holocron("rare"),
        "RARE HOLOCRON",
        "In your Collection",
    ),
    effect(
        "mirror",
        Effect::Mirror,
        "MIRROR",
        "The world is mirrored for 30 s",
    ),
    effect(
        "upside_down",
        Effect::UpsideDown,
        "UPSIDE DOWN",
        "The world turns over for 20 s",
    ),
    outcome(
        "doom_coin",
        Kind::CoinBack,
        "DOOM COIN BACK",
        "Spin it again",
    ),
    effect("blind", Effect::Blind, "BLIND", "Darkness for 15 s"),
    outcome(
        "holo_legendary",
        Kind::Holocron("legendary"),
        "LEGENDARY HOLOCRON!",
        "In your Collection",
    ),
    effect("drunk", Effect::Drunk, "DRUNK", "The room sways for 30 s"),
    effect(
        "tiny_fov",
        Effect::TinyFov,
        "TUNNEL VISION",
        "A narrow view for 20 s",
    ),
    outcome("nothing", Kind::Nothing, "NOTHING", "Nothing happens..."),
    effect(
        "huge_fov",
        Effect::HugeFov,
        "FISHEYE",
        "A huge view for 20 s",
    ),
    effect(
        "no_saber",
        Effect::NoSaber,
        "NO SABER",
        "Your saber stays off for 20 s",
    ),
    effect(
        "quack",
        Effect::Quack,
        "QUACK",
        "Every swing quacks for 30 s",
    ),
    effect("walk", Effect::Walk, "WALK", "No running for 30 s"),
    effect(
        "bunny",
        Effect::Bunny,
        "BUNNY",
        "You cannot stop jumping for 20 s",
    ),
    outcome(
        "curse",
        Kind::Curse,
        "CURSED...",
        "Something bad comes within a minute",
    ),
];

/// The segment `id` names, by the built-in order.
pub(crate) fn known(id: &str) -> Option<(usize, &'static Outcome)> {
    OUTCOMES
        .iter()
        .enumerate()
        .find(|(_, outcome)| outcome.id == id)
}

/// The clockwise angle (degrees, 0 to 360) the wheel stands at when `segment` is under
/// the pointer: segment `k` is painted `k` segments clockwise of the top, so the wheel
/// turns back by as much.
pub(crate) fn segment_angle(segment: usize) -> f32 {
    (360.0 - (segment % SEGMENTS) as f32 * SEGMENT_DEGREES).rem_euclid(360.0)
}

/// How far the wheel has turned clockwise (degrees) `elapsed` into a spin from angle
/// `from` landing on `segment`: [`TURNS`] whole turns and the rest of the way to the
/// segment's angle, fast at first, slowing to a stop as the music ends at [`SPIN`] (a
/// quartic ease out: the last turns crawl, so who wins stays open to the end).
pub(crate) fn wheel_angle(elapsed: Duration, from: f32, segment: usize) -> f32 {
    let rest = (segment_angle(segment) - from).rem_euclid(360.0);
    let total = TURNS * 360.0 + rest;
    let t = (elapsed.as_secs_f32() / SPIN.as_secs_f32()).min(1.0);
    from + total * (1.0 - (1.0 - t).powi(4))
}

/// How many pegs (the dividers, half a segment either side of each segment's middle)
/// have passed the pointer once the wheel turned `angle` degrees.
fn pegs_passed(angle: f32) -> u32 {
    ((angle + SEGMENT_DEGREES / 2.0) / SEGMENT_DEGREES) as u32
}

/// The wheel's own turn about its axle at `angle` degrees clockwise (as seen from the
/// front, the model's +X): a turn about -X.
fn wheel_turn(angle: f32) -> Quat {
    Quat::from_rotation_x(-angle.to_radians())
}

/// A wheel's spin under way, or just stopped.
#[derive(Clone, Debug, PartialEq)]
struct Spinning {
    /// The spinner's slot.
    slot: Option<u8>,
    /// The player's own spin.
    mine: bool,
    outcome: String,
    /// What it does, `None` for an outcome a newer hub knows and this client does not.
    kind: Option<Kind>,
    segment: usize,
    /// The angle it started from (the wheel's last stop).
    from: f32,
    started: Instant,
    /// Pegs that passed the pointer, for the ticks.
    pegs: u32,
    stopped: bool,
    blasted: bool,
    /// Its words once stopped.
    title: String,
    line: String,
}

impl Spinning {
    fn angle(&self, now: Instant) -> f32 {
        wheel_angle(
            now.saturating_duration_since(self.started),
            self.from,
            self.segment,
        )
    }

    /// Whether its words still say what it landed on at `now`.
    fn told(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started) < SPIN + HOLD
    }
}

/// One wheel standing on the loaded map.
#[derive(Clone, Debug, PartialEq)]
struct Placed {
    id: i64,
    /// Where it stands (the floor under it) and the way its face looks, degrees.
    at: Vec3,
    yaw: f32,
    /// Where it last stopped, degrees clockwise.
    rest: f32,
    spin: Option<Spinning>,
}

impl Placed {
    fn angle(&self, now: Instant) -> f32 {
        self.spin.as_ref().map_or(self.rest, |spin| spin.angle(now))
    }

    /// A spin of it runs (its music plays): nobody else may spin it.
    fn busy(&self, now: Instant) -> bool {
        self.spin
            .as_ref()
            .is_some_and(|spin| now.saturating_duration_since(spin.started) < SPIN)
    }
}

/// A sound to play once, at a wheel (`None`: in the player's ears).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cue {
    path: &'static str,
    origin: Option<[f32; 3]>,
    source: sjk_audio::SourceId,
}

/// Wheel `id`'s own sound source (`None`: the player's own, for the quack).
fn source(id: Option<i64>) -> sjk_audio::SourceId {
    sjk_audio::SourceId(SOURCE_BASE | id.map_or(0xFFFF, |id| id as u32 & 0x7FFF))
}

/// What the player is told as a wheel stops or blows up.
#[derive(Clone, Debug, PartialEq)]
enum Moment {
    /// The player's own wheel stopped on `kind` (`None`: unknown), `title` and `line`.
    Stopped {
        kind: Option<Kind>,
        title: String,
        line: String,
    },
    /// A wheel on `explode` blew its spinner on `slot` up.
    Blast { slot: Option<u8>, at: Vec3 },
}

/// A number to vary a curse by.
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos())
}

/// What the player knows before asking for a spin.
#[derive(Clone, Copy, Debug, Default)]
struct Check {
    /// The hub has the wheel off (its `open` false): out of order, whatever else.
    closed: bool,
    /// The Ragdoll could strike them: alive, playing, not in a duel or with the flag.
    strikable: bool,
    /// Seconds left of the cooldown, known to the client.
    cooldown: Option<u64>,
    /// A spin was asked and the hub has not answered.
    asked: bool,
    /// Someone's spin of this wheel runs.
    busy: bool,
    /// The player's DOOM coins, when the hub said.
    coins: Option<u64>,
}

/// Why a spin is refused before the hub is asked, in the player's words; `None` when it
/// may go.
fn refusal(check: Check) -> Option<String> {
    if check.closed {
        return Some(wire::OUT_OF_ORDER.to_owned());
    }
    if check.asked {
        return Some("The WHEEL OF DOOM is asking the hub...".to_owned());
    }
    if check.busy {
        return Some("Someone is spinning this WHEEL OF DOOM".to_owned());
    }
    if !check.strikable {
        return Some(
            "Not now: the WHEEL OF DOOM spins out of duels, alive, without the flag".to_owned(),
        );
    }
    if let Some(left) = check.cooldown {
        return Some(format!("Your next spin in {left} s"));
    }
    (check.coins == Some(0)).then(|| wire::NO_COIN.to_owned())
}

/// What the Use prompt says at a wheel: spin it for a coin, or why Use does nothing now.
fn wheel_prompt(check: Check, now_busy_mine: bool) -> Option<crate::use_prompt::Prompt> {
    use crate::use_prompt::Prompt;
    let action = "Spin the WHEEL OF DOOM";
    if now_busy_mine {
        return None;
    }
    let (line, ready) = if check.closed {
        ("Out of order for now".to_owned(), false)
    } else if check.asked {
        ("Spinning...".to_owned(), false)
    } else if check.busy {
        ("Someone is spinning it".to_owned(), false)
    } else if let Some(left) = check.cooldown {
        (format!("Your turn again in {left} s"), false)
    } else {
        match check.coins {
            Some(0) => ("No DOOM coin: the teddy bear brings one".to_owned(), false),
            Some(1) => ("1 DOOM coin (your last)".to_owned(), true),
            Some(coins) => (format!("1 DOOM coin (you hold {coins})"), true),
            None => ("1 DOOM coin".to_owned(), true),
        }
    };
    Some(Prompt {
        action: action.to_owned(),
        line: Some(line),
        ready,
    })
}

/// The wheel among `wheels` the player at `from` facing `yaw` (radians) may spin: within
/// [`REACH`] across the floor and [`REACH_UP`] up or down, within [`FACING`] of their
/// facing; the nearest of those.
fn pick_wheel<'a>(
    from: Vec3,
    yaw: f32,
    wheels: impl IntoIterator<Item = &'a Placed>,
) -> Option<i64> {
    let facing = glam::Vec2::from_angle(yaw);
    wheels
        .into_iter()
        .filter_map(|placed| {
            let way = (placed.at - from).truncate();
            let across = way.length();
            let up = (placed.at.z - from.z).abs();
            let faced = across < 1.0 || facing.angle_to(way).abs() <= FACING;
            (across <= REACH && up <= REACH_UP && faced).then_some((placed.id, across))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// A wheel the creator asked to place, waiting for the map's checksum.
struct PendingPlace {
    origin: Vec3,
    yaw: f32,
    /// `ip:port` for this server only, empty for every server.
    server: String,
}

/// The toy's state: the wheels of the map, the spinner's effects, the spin asked and the
/// sounds.
pub(crate) struct WheelOfDoom {
    /// The hub list's revision the wheels were taken from, and the map, checksum and
    /// server they were taken for.
    taken: Option<(u64, String, i32, String)>,
    placed: Vec<Placed>,
    /// The spinner's own effect and curse.
    pub(crate) effects: Effects,
    /// The wheel asked to spin, and when.
    asked: Option<(i64, Instant)>,
    /// When the player may spin again, by the client's count or the hub's refusal.
    cooldown_until: Option<Instant>,
    answer_seen: u64,
    /// When the wheels were last asked of the hub.
    read_at: Option<Instant>,
    /// The server and map the effects began on: another ends them.
    place: Option<(std::net::SocketAddr, String)>,
    pending_place: Option<PendingPlace>,
    use_pressed: bool,
    use_held: bool,
    cues: Vec<Cue>,
    blasts: Vec<Vec3>,
    sounds_registered: Option<u64>,
    meshes: [Option<usize>; 2],
    meshes_seen: usize,
    pub(crate) draw: draw::Draw,
    mirror: mirror::Mirror,
    /// A world shot's wheels, kept without a session.
    #[cfg(test)]
    shot: bool,
    /// How far into their spin a world shot's wheels stay, frame after frame.
    #[cfg(test)]
    shot_since: Duration,
}

impl Default for WheelOfDoom {
    fn default() -> Self {
        Self {
            taken: None,
            placed: Vec::with_capacity(MOST),
            effects: Effects::default(),
            asked: None,
            cooldown_until: None,
            answer_seen: 0,
            read_at: None,
            place: None,
            pending_place: None,
            use_pressed: false,
            use_held: false,
            cues: Vec::with_capacity(16),
            blasts: Vec::with_capacity(2),
            sounds_registered: None,
            meshes: [None; 2],
            meshes_seen: 0,
            draw: draw::Draw::default(),
            mirror: mirror::Mirror::default(),
            #[cfg(test)]
            shot: false,
            #[cfg(test)]
            shot_since: Duration::ZERO,
        }
    }
}

/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;

impl WheelOfDoom {
    /// The player's own wheel spins, or an effect or a curse runs: the Toys tab's switch
    /// shows on.
    pub(crate) fn running(&self) -> bool {
        let now = Instant::now();
        self.placed.iter().any(|placed| {
            placed
                .spin
                .as_ref()
                .is_some_and(|spin| spin.mine && spin.told(now))
        }) || self.effects.any()
    }

    /// Note a `+use` press in `command` for the frame's logic.
    pub(crate) fn observe_use(&mut self, command: &sjk_protocol::UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// The whole seconds left before the player may spin again.
    fn cooldown_left(&self, now: Instant) -> Option<u64> {
        let wait = self.cooldown_until?.checked_duration_since(now)?;
        (!wait.is_zero()).then(|| wait.as_secs_f32().ceil() as u64)
    }

    /// Take the wheels standing on `map` of `checksum` on `server` from `wheels`, keeping
    /// the spins and rests of those already here.
    fn take(&mut self, wheels: &[MysteryBox], map: &str, checksum: i32, server: &str) {
        let mut old = std::mem::take(&mut self.placed);
        self.placed = wheels
            .iter()
            .filter(|placed| placed.stands_on(map, checksum, Some(server)))
            .take(MOST)
            .map(|placed| {
                let kept = old.iter_mut().find(|here| here.id == placed.id);
                let (rest, spin) = kept.map_or((0.0, None), |here| (here.rest, here.spin.take()));
                Placed {
                    id: placed.id,
                    at: Vec3::from_array(placed.origin),
                    yaw: placed.yaw,
                    rest,
                    spin,
                }
            })
            .collect();
    }

    /// Wheel `id` starts spinning at `now` for the player on `slot` (`mine`: the player's
    /// own), landing on `segment` for `outcome`. False when no such wheel stands here.
    fn start(
        &mut self,
        id: i64,
        slot: Option<u8>,
        mine: bool,
        outcome: &str,
        segment: usize,
        now: Instant,
    ) -> bool {
        let Some(placed) = self.placed.iter_mut().find(|placed| placed.id == id) else {
            return false;
        };
        let found = known(outcome).map(|(_, known)| known);
        let (title, line) = match found {
            Some(known) => (known.title.to_owned(), known.line.to_owned()),
            None => (
                outcome.replace('_', " ").to_uppercase(),
                "Nothing happens...".to_owned(),
            ),
        };
        let from = placed.angle(now).rem_euclid(360.0);
        placed.rest = from;
        placed.spin = Some(Spinning {
            slot,
            mine,
            outcome: outcome.to_owned(),
            kind: found.map(|known| known.kind),
            segment: segment % SEGMENTS,
            from,
            started: now,
            pegs: pegs_passed(from),
            stopped: false,
            blasted: false,
            title,
            line,
        });
        let origin = Some((placed.at + Vec3::Z * AXLE).to_array());
        self.cues.push(Cue {
            path: MUSIC,
            origin,
            source: source(Some(id)),
        });
        true
    }

    /// Move the wheels on to `now`: a tick for each peg passing, the stop and what it
    /// says, a blast after an `explode`, and spins done with once their words go.
    fn tick(&mut self, now: Instant) -> Vec<Moment> {
        let mut moments = Vec::new();
        for placed in &mut self.placed {
            let Some(spin) = placed.spin.as_mut() else {
                continue;
            };
            let elapsed = now.saturating_duration_since(spin.started);
            let origin = Some((placed.at + Vec3::Z * AXLE).to_array());
            let source = source(Some(placed.id));
            if !spin.stopped {
                let pegs = pegs_passed(spin.angle(now));
                if pegs > spin.pegs {
                    spin.pegs = pegs;
                    self.cues.push(Cue {
                        path: TICK,
                        origin,
                        source,
                    });
                }
            }
            if !spin.stopped && elapsed >= SPIN {
                spin.stopped = true;
                self.cues.push(Cue {
                    path: STOP,
                    origin,
                    source,
                });
                let follow = match spin.kind {
                    Some(Kind::Holocron(_) | Kind::CoinBack) => Some(FANFARE),
                    Some(kind) if kind.bad() => Some(LAUGH),
                    _ => None,
                };
                if let Some(path) = follow {
                    self.cues.push(Cue {
                        path,
                        origin,
                        source,
                    });
                }
                if spin.mine {
                    moments.push(Moment::Stopped {
                        kind: spin.kind,
                        title: spin.title.clone(),
                        line: spin.line.clone(),
                    });
                }
            }
            if spin.kind == Some(Kind::Explode) && !spin.blasted && elapsed >= SPIN + BLAST_DELAY {
                spin.blasted = true;
                moments.push(Moment::Blast {
                    slot: spin.slot,
                    at: placed.at,
                });
                self.cues.push(Cue {
                    path: BLAST_SOUND,
                    origin,
                    source,
                });
            }
            if !spin.told(now) && (spin.blasted || spin.kind != Some(Kind::Explode)) {
                placed.rest = spin.angle(now).rem_euclid(360.0);
                placed.spin = None;
            }
        }
        moments
    }

    /// Everything of this server and map goes: the wheels' spins and the effects.
    fn clear(&mut self) {
        for placed in &mut self.placed {
            placed.spin = None;
        }
        self.effects.clear();
        self.asked = None;
        self.use_pressed = false;
    }

    /// The player's own view from `eye` at `target` with `up`, turned at `now` by the
    /// effect running (Upside down's roll, Drunk's sway): the new target and up. The aim
    /// is not turned, only what is seen.
    pub(crate) fn turn_view(
        &self,
        eye: Vec3,
        target: Vec3,
        up: Vec3,
        now: Instant,
    ) -> (Vec3, Vec3) {
        let Some([roll, pitch, yaw]) = self.effects.view_turn(now) else {
            return (target, up);
        };
        let forward = (target - eye).normalize_or_zero();
        let right = forward.cross(up).normalize_or_zero();
        if forward == Vec3::ZERO || right == Vec3::ZERO {
            return (target, up);
        }
        let sway = Quat::from_axis_angle(up.normalize_or_zero(), yaw.to_radians())
            * Quat::from_axis_angle(right, pitch.to_radians());
        let forward = sway * forward;
        let up = Quat::from_axis_angle(forward, roll.to_radians()) * (sway * up);
        (eye + forward * target.distance(eye), up)
    }

    /// The models to find among the loaded ones.
    fn mesh_of(&self, model: &str) -> Option<usize> {
        let index = MODELS.iter().position(|known| *known == model)?;
        self.meshes[index]
    }
}

impl GpuState {
    fn wheel_of_doom_centre(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::CenterPrint,
            text,
            None,
            Instant::now(),
        );
    }

    fn wheel_of_doom_print(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::Print,
            text,
            None,
            Instant::now(),
        );
    }

    /// The player's DOOM coins, when the hub said.
    fn doom_coins(&self) -> Option<u64> {
        crate::player_identity::with_snapshot(|snapshot| {
            snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.coins)
        })
        .flatten()
    }

    /// `toy_wheelofdoom`: the player's DOOM coins and where to spend them, in the centre
    /// of the screen (since 11/11/2026 the wheels stand on maps).
    pub(crate) fn wheel_of_doom_command(&mut self) {
        let now = Instant::now();
        self.read_wheel_of_doom(now);
        let coins = match self.doom_coins() {
            Some(1) => "You hold 1 DOOM coin".to_owned(),
            Some(coins) => format!("You hold {coins} DOOM coins"),
            None => "Asking the hub for your DOOM coins".to_owned(),
        };
        let here = match self.wheel_of_doom.placed.len() {
            0 => "No WHEEL OF DOOM on this map".to_owned(),
            1 => "A WHEEL OF DOOM stands on this map".to_owned(),
            count => format!("{count} WHEELS OF DOOM stand on this map"),
        };
        self.wheel_of_doom_centre(format!(
            "{coins}. {here}: press Use at one to spin it. The teddy bear in a mystery box brings a coin"
        ));
    }

    /// Ask the hub for the wheels and the coins, at most every 5 seconds.
    fn read_wheel_of_doom(&mut self, now: Instant) {
        if self
            .wheel_of_doom
            .read_at
            .is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(5))
        {
            return;
        }
        self.wheel_of_doom.read_at = Some(now);
        let _ = crate::player_identity::with_service(|service| service.read_wheel_of_doom());
    }

    /// Find the wheel's models among the loaded ones, when more have loaded.
    fn find_wheel_of_doom_meshes(&mut self) {
        if self.wheel_of_doom.meshes_seen == self.object_meshes.len() {
            return;
        }
        self.wheel_of_doom.meshes_seen = self.object_meshes.len();
        for (slot, model) in MODELS.iter().enumerate() {
            self.wheel_of_doom.meshes[slot] = self.object_meshes.iter().position(|mesh| {
                mesh.appearance.variant.is_empty() && mesh.appearance.model == *model
            });
        }
    }

    /// The frame's WHEEL OF DOOM logic, after the looks: the wheels of the loaded map, the
    /// hub's answer, other players' spins (`events`, this server's toy events), a `+use`
    /// press at a wheel, and the effects, which end on a new map or server, a disconnect
    /// or a death.
    pub(crate) fn update_wheel_of_doom(&mut self, events: &[sjk_identity::ToyEvent]) {
        #[cfg(test)]
        if self.wheel_of_doom.shot {
            self.find_wheel_of_doom_meshes();
            let started = Instant::now() - self.wheel_of_doom.shot_since;
            for placed in &mut self.wheel_of_doom.placed {
                if let Some(spin) = placed.spin.as_mut() {
                    spin.started = started;
                }
            }
            return;
        }
        let now = Instant::now();
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.wheel_of_doom.clear();
            self.wheel_of_doom.placed.clear();
            self.wheel_of_doom.taken = None;
            self.wheel_of_doom.place = None;
            return;
        };
        let server = session.server();
        let server_text = server.to_string();
        let own = u8::try_from(session.game_state().client_num).ok();
        let alive = session.latest_snapshot().player.health() > 0;
        let map_name = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        let moved = self
            .wheel_of_doom
            .place
            .as_ref()
            .is_none_or(|(at_server, at_map)| *at_server != server || *at_map != map_name);
        if moved {
            self.wheel_of_doom.clear();
            self.wheel_of_doom.place = Some((server, map_name.clone()));
        }
        if !alive {
            self.wheel_of_doom.effects.clear();
        }
        // The wheels of this map file, read with the map's checksum (`map_edits`).
        let (revision, on_map) = crate::player_identity::with_snapshot(|snapshot| {
            (
                snapshot.wheel_of_doom_revision,
                snapshot
                    .wheel_of_doom
                    .as_ref()
                    .is_some_and(|wheel| wheel.on_map(&map_name)),
            )
        })
        .unwrap_or_default();
        if on_map || self.wheel_of_doom.pending_place.is_some() {
            self.map_edits.want_checksum(&map_name);
        }
        if let Some((map, checksum)) = self.map_edits.map().map(|(map, c)| (map.to_owned(), c)) {
            self.place_pending_wheel_of_doom(&map, checksum);
            let fresh = self.wheel_of_doom.taken.as_ref().is_some_and(
                |(taken, taken_map, taken_checksum, taken_server)| {
                    *taken == revision
                        && *taken_checksum == checksum
                        && *taken_map == map
                        && *taken_server == server_text
                },
            );
            if !fresh {
                let wheels = crate::player_identity::with_snapshot(|snapshot| {
                    snapshot
                        .wheel_of_doom
                        .as_ref()
                        .map(|wheel| wheel.wheels.clone())
                        .unwrap_or_default()
                })
                .unwrap_or_default();
                self.wheel_of_doom
                    .take(&wheels, &map, checksum, &server_text);
                self.wheel_of_doom.taken = Some((revision, map, checksum, server_text.clone()));
            }
        } else {
            self.wheel_of_doom.placed.clear();
        }
        self.find_wheel_of_doom_meshes();
        self.read_wheel_of_doom_answer(now, own);
        // Other players' spins: the hub made the event, so it holds the outcome.
        for event in events {
            if event.toy != wire::TOY || Some(event.slot) == own {
                continue;
            }
            let Some((wheel, outcome)) = wire::parse_event(&event.event) else {
                continue;
            };
            let segment = self.wheel_of_doom_segment(outcome);
            self.wheel_of_doom
                .start(wheel, Some(event.slot), false, outcome, segment, now);
        }
        for moment in self.wheel_of_doom.tick(now) {
            self.wheel_of_doom_moment(moment, own, now);
        }
        if let Some(effect) = self.wheel_of_doom.effects.tick(now) {
            self.wheel_of_doom_centre(format!("The curse strikes: {}!", effect.hud()));
        }
        self.release_wheel_of_doom_mirror();
        self.wheel_of_doom_use(server_text, now);
    }

    /// The wheel the player faces: the Use prompt, and a `+use` press asks the hub to
    /// spin it.
    fn wheel_of_doom_use(&mut self, server: String, now: Instant) {
        let pressed = std::mem::take(&mut self.wheel_of_doom.use_pressed);
        if self.wheel_of_doom.placed.is_empty() {
            return;
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let player = &session.latest_snapshot().player;
        if !crate::mystery_box::plays(player) {
            return;
        }
        let from = self
            .local_prediction
            .predicted_state()
            .map_or_else(|| player.origin(), |state| state.origin);
        let Some(id) = pick_wheel(
            Vec3::from_array(from),
            self.camera_yaw,
            &self.wheel_of_doom.placed,
        ) else {
            return;
        };
        let closed = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom
                .as_ref()
                .is_some_and(|wheel| !wheel.open)
        })
        .unwrap_or(false);
        let toy = &self.wheel_of_doom;
        let placed = toy.placed.iter().find(|placed| placed.id == id);
        let mine_running = placed
            .and_then(|placed| placed.spin.as_ref())
            .is_some_and(|spin| spin.mine && !spin.stopped);
        let check = Check {
            closed,
            strikable: self.ragdoll.strikable(),
            cooldown: toy.cooldown_left(now),
            asked: toy
                .asked
                .is_some_and(|(_, asked)| now.saturating_duration_since(asked) < ASK_TIMEOUT),
            busy: placed.is_some_and(|placed| placed.busy(now)),
            coins: self.doom_coins(),
        };
        if let Some(prompt) = wheel_prompt(check, mine_running) {
            self.want_use_prompt(prompt);
        }
        if !pressed || mine_running {
            return;
        }
        if closed || self.doom_coins().is_none() {
            self.read_wheel_of_doom(now);
        }
        if let Some(why) = refusal(check) {
            self.wheel_of_doom_centre(why);
            return;
        }
        if crate::player_identity::with_service(|service| service.spin_wheel_of_doom(id, server))
            .is_some()
        {
            self.wheel_of_doom.asked = Some((id, now));
        } else {
            self.wheel_of_doom_centre("Not connected to the SJK hub".to_owned());
        }
    }

    /// The segment a spin of `outcome` lands on, for another player's: by the hub's list,
    /// else the built-in order, else `nothing`'s.
    fn wheel_of_doom_segment(&self, outcome: &str) -> usize {
        crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom
                .as_ref()
                .and_then(|wheel| wheel.segment_of(outcome))
        })
        .flatten()
        .or_else(|| known(outcome).map(|(segment, _)| segment))
        .or_else(|| known("nothing").map(|(segment, _)| segment))
        .unwrap_or(0)
    }

    /// Act on the hub's last answer to a spin, once.
    fn read_wheel_of_doom_answer(&mut self, now: Instant, own: Option<u8>) {
        let seen = self.wheel_of_doom.answer_seen;
        let Some((serial, outcome, cooldown)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .wheel_of_doom_spin
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .map(|(serial, outcome)| {
                    (
                        *serial,
                        outcome.clone(),
                        snapshot.wheel_of_doom.as_ref().map(|wheel| wheel.cooldown),
                    )
                })
        })
        .flatten() else {
            return;
        };
        self.wheel_of_doom.answer_seen = serial;
        let asked = self.wheel_of_doom.asked.take();
        match outcome {
            Ok(spin) => {
                let cooldown = cooldown
                    .filter(|seconds| *seconds > 0)
                    .unwrap_or(DEFAULT_COOLDOWN);
                self.wheel_of_doom.cooldown_until = Some(now + Duration::from_secs(cooldown));
                let wheel = if spin.wheel != 0 {
                    spin.wheel
                } else {
                    asked.map_or(0, |(id, _)| id)
                };
                self.wheel_of_doom.start(
                    wheel,
                    own,
                    true,
                    &spin.outcome,
                    spin.segment as usize,
                    now,
                );
            }
            Err(refused) => {
                if let Some(left) = refused.left {
                    self.wheel_of_doom.cooldown_until = Some(now + Duration::from_secs(left));
                }
                if asked.is_some() {
                    self.wheel_of_doom_centre(refused.words);
                }
            }
        }
    }

    /// The player's own wheel stopped, or a wheel blew its spinner up.
    fn wheel_of_doom_moment(&mut self, moment: Moment, own: Option<u8>, now: Instant) {
        match moment {
            Moment::Stopped { kind, title, line } => {
                let text = match kind {
                    Some(Kind::Effect(effect)) => {
                        self.wheel_of_doom.effects.start(effect, now);
                        format!("{title} {line}")
                    }
                    Some(Kind::Curse) => {
                        self.wheel_of_doom.effects.curse(now, seed());
                        format!("{title} {line}")
                    }
                    Some(Kind::Holocron(_)) => format!("{title} In your Collection"),
                    Some(Kind::CoinBack) => format!("{title}! {line}"),
                    Some(Kind::Explode) => title,
                    Some(Kind::Nothing) => line,
                    None => format!("{title}: {line}"),
                };
                self.wheel_of_doom_centre(text);
            }
            Moment::Blast { slot, at } => {
                let mine = slot.is_some() && slot == own;
                let stands = if mine {
                    self.local_prediction
                        .predicted_state()
                        .map(|state| Vec3::from_array(state.origin))
                } else {
                    slot.and_then(|slot| {
                        let time = self.presentation_clock.sample(now);
                        let presented = self
                            .live_world
                            .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))?;
                        Some(Vec3::from_array(presented.sample(time).translation))
                    })
                };
                self.wheel_of_doom.blasts.push(stands.unwrap_or(at));
                // The game's `kill`, as the mystery box's: never in a duel, with the flag
                // or wherever the Ragdoll could not strike.
                if mine
                    && self.ragdoll.strikable()
                    && let Some(session) = self.live_session.as_mut()
                    && let Err(error) = session.send_reliable_command(b"kill")
                {
                    crate::log::progress(format_args!("wheel of doom: kill not sent: {error}"));
                }
            }
        }
    }

    /// Send the wheel the creator asked to place once the map's checksum is known.
    fn place_pending_wheel_of_doom(&mut self, map: &str, checksum: i32) {
        let Some(pending) = self.wheel_of_doom.pending_place.take() else {
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
        if crate::player_identity::with_service(|service| service.place_wheel_of_doom(placed))
            .is_none()
        {
            self.wheel_of_doom_print("WHEEL OF DOOM: not connected to the SJK hub".to_owned());
        }
    }

    /// `wheelofdoom_place`, `wheelofdoom_remove` and `wheelofdoom_list`.
    pub(crate) fn wheel_of_doom_creator_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if name == "wheelofdoom_list" {
            return Ok(self.wheel_of_doom_list());
        }
        if !crate::player_identity::is_creator() {
            return Err("Only SJK's creator places and removes wheels of doom".into());
        }
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .ok_or("Join a game server first: wheels stand on a server's map")?;
        let server = session.server().to_string();
        match name {
            "wheelofdoom_place" => {
                let one_server = match args.first().map(String::as_str) {
                    None => false,
                    Some("server") => true,
                    Some(other) => {
                        return Err(format!("wheelofdoom_place [server], not {other}"));
                    }
                };
                let (eye, forward) = self.world_notes.view().ok_or("No view to aim from")?;
                let hit = self
                    .decal_surfaces
                    .ray_hit(eye, forward, 4096.0)
                    .ok_or("Aim at the floor where the wheel should stand")?;
                let yaw = (forward.y.atan2(forward.x).to_degrees() + 180.0).rem_euclid(360.0);
                self.wheel_of_doom.pending_place = Some(PendingPlace {
                    origin: eye + forward * hit.distance,
                    yaw,
                    server: if one_server { server } else { String::new() },
                });
                let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
                self.map_edits.want_checksum(&map);
                Ok(vec![format!(
                    "Placing a WHEEL OF DOOM on {map}{}...",
                    if one_server { " for this server" } else { "" }
                )])
            }
            "wheelofdoom_remove" => {
                let id = match args.first() {
                    Some(id) => id.parse().map_err(|_| format!("Not a wheel id: {id}"))?,
                    None => {
                        let from = Vec3::from_array(session.latest_snapshot().player.origin());
                        self.wheel_of_doom
                            .placed
                            .iter()
                            .map(|placed| (placed.id, placed.at.distance(from)))
                            .filter(|(_, distance)| *distance <= REMOVE_REACH)
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(id, _)| id)
                            .ok_or("No WHEEL OF DOOM near you: give its id (wheelofdoom_list)")?
                    }
                };
                crate::player_identity::with_service(|service| service.remove_wheel_of_doom(id))
                    .ok_or("Not connected to the SJK hub")?;
                Ok(vec![format!("Removing WHEEL OF DOOM {id}...")])
            }
            _ => Err(format!("Unknown command {name}")),
        }
    }

    fn wheel_of_doom_list(&self) -> Vec<String> {
        let coins = self
            .doom_coins()
            .map_or_else(|| "unknown".to_owned(), |coins| coins.to_string());
        let placed = &self.wheel_of_doom.placed;
        if placed.is_empty() {
            return vec![format!(
                "No WHEEL OF DOOM on this map. Your DOOM coins: {coins}"
            )];
        }
        let from = self
            .live_session
            .as_ref()
            .map(|session| Vec3::from_array(session.latest_snapshot().player.origin()));
        let mut lines = vec![format!(
            "{} WHEELS OF DOOM here, a DOOM coin a spin (yours: {coins}):",
            placed.len()
        )];
        for wheel in placed {
            let away = from.map_or(String::new(), |from| {
                format!(", {:.0} units away", wheel.at.distance(from))
            });
            lines.push(format!(
                "  {}: at {:.0} {:.0} {:.0}{away}",
                wheel.id, wheel.at.x, wheel.at.y, wheel.at.z
            ));
        }
        lines
    }

    /// Add this frame's wheels to the scene: each stand, and its wheel turned on its axle.
    pub(crate) fn append_wheel_of_doom_instances(&mut self) {
        let now = Instant::now();
        let toy = &self.wheel_of_doom;
        let (Some(stand), Some(spun)) = (toy.mesh_of(STAND_MODEL), toy.mesh_of(WHEEL_MODEL)) else {
            return;
        };
        for placed in &toy.placed {
            let facing = Quat::from_array(sjk_client::legacy_angles_to_quaternion([
                0.0, placed.yaw, 0.0,
            ]));
            self.object_groups[stand].push(ActorInstance::new(
                placed.at.to_array(),
                facing.to_array(),
                [1.0; 3],
            ));
            self.object_groups[spun].push(ActorInstance::new(
                (placed.at + Vec3::Z * AXLE).to_array(),
                (facing * wheel_turn(placed.angle(now))).to_array(),
                [1.0; 3],
            ));
        }
    }

    /// Play the wheels' sounds and the quacks; the sounds are handed to the audio the
    /// first time there is one.
    pub(crate) fn play_wheel_of_doom_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let quack = self.wheel_of_doom.effects.take_quack();
        let Some(audio) = audio.as_mut() else {
            self.wheel_of_doom.cues.clear();
            self.wheel_of_doom.blasts.clear();
            return;
        };
        if let Some(vfs) = self.vfs.clone() {
            for stands in self.wheel_of_doom.blasts.drain(..) {
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
        if self.wheel_of_doom.cues.is_empty() && !quack {
            return;
        }
        if self.wheel_of_doom.sounds_registered != Some(audio.generation())
            && let Some(vfs) = self.vfs.as_deref()
        {
            audio.register_effect_paths(
                vfs,
                [
                    TICK,
                    STOP,
                    FANFARE,
                    QUACK,
                    MUSIC,
                    LAUGH,
                    LAUGH_FALLBACK,
                    BLAST_SOUND,
                ],
            );
            self.wheel_of_doom.sounds_registered = Some(audio.generation());
        }
        if quack {
            audio.play_toy(
                crate::toy_sounds::Toy::WheelOfDoom,
                QUACK,
                1.0,
                None,
                source(None),
                sjk_audio::ChannelId(1),
            );
        }
        for cue in self.wheel_of_doom.cues.drain(..) {
            let path = match cue.path {
                LAUGH if !audio.has_sound(LAUGH) => LAUGH_FALLBACK,
                // Without the extras pack the wheel spins to its ticks alone.
                MUSIC if !audio.has_sound(MUSIC) => continue,
                path => path,
            };
            audio.play_toy(
                crate::toy_sounds::Toy::WheelOfDoom,
                path,
                1.0,
                cue.origin,
                cue.source,
                // `CHAN_AUTO`: the music, the ticks, the stop and the laugh overlap.
                sjk_audio::ChannelId(0),
            );
        }
    }

    /// The saber as the player state says, for No saber.
    pub(crate) fn wheel_of_doom_saber(player: &sjk_protocol::PlayerState) -> Saber {
        Saber {
            weapon: player.weapon(),
            holstered: player.saber_holstered(),
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: a wheel at each of `wheels` (origin and facing yaw), spun
    /// `since` ago onto segment `n` for the `n`th.
    pub(crate) fn shot_wheels_of_doom(&mut self, wheels: &[([f32; 3], f32)], since: Duration) {
        let started = Instant::now() - since;
        self.wheel_of_doom.shot = true;
        self.wheel_of_doom.shot_since = since;
        self.wheel_of_doom.placed = wheels
            .iter()
            .enumerate()
            .map(|(index, (origin, yaw))| Placed {
                id: index as i64 + 1,
                at: Vec3::from_array(*origin),
                yaw: *yaw,
                rest: 0.0,
                spin: None,
            })
            .collect();
        for index in 0..wheels.len() {
            let outcome = OUTCOMES[index % SEGMENTS].id;
            self.wheel_of_doom.start(
                index as i64 + 1,
                Some(index as u8),
                false,
                outcome,
                index,
                started,
            );
            if let Some(spin) = self.wheel_of_doom.placed[index].spin.as_mut() {
                spin.stopped = since >= SPIN;
            }
        }
        self.wheel_of_doom.cues.clear();
    }

    /// For the world shots: the wheel's meshes found, and the wheels kept.
    pub(crate) fn shot_wheel_of_doom_state(&self) -> ([Option<usize>; 2], usize) {
        (self.wheel_of_doom.meshes, self.wheel_of_doom.placed.len())
    }
}

/// How long a spin takes, for the world shots.
#[cfg(test)]
pub(crate) const SHOT_SPIN: Duration = SPIN;

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: i64, at: Vec3) -> Placed {
        Placed {
            id,
            at,
            yaw: 0.0,
            rest: 0.0,
            spin: None,
        }
    }

    fn toy_with(ids: &[i64]) -> WheelOfDoom {
        WheelOfDoom {
            placed: ids
                .iter()
                .map(|id| placed(*id, Vec3::X * *id as f32))
                .collect(),
            ..WheelOfDoom::default()
        }
    }

    #[test]
    fn the_outcomes_are_the_hubs_twenty_in_order() {
        let ids: Vec<_> = OUTCOMES.iter().map(|outcome| outcome.id).collect();
        assert_eq!(
            ids,
            [
                "mythical",
                "reverse_keys",
                "holo_uncommon",
                "invert_mouse",
                "explode",
                "holo_rare",
                "mirror",
                "upside_down",
                "doom_coin",
                "blind",
                "holo_legendary",
                "drunk",
                "tiny_fov",
                "nothing",
                "huge_fov",
                "no_saber",
                "quack",
                "walk",
                "bunny",
                "curse"
            ]
        );
        // Every effect has its segment, and a holocron of every tier stands on it.
        for effect in Effect::ALL {
            assert!(
                OUTCOMES
                    .iter()
                    .any(|outcome| outcome.kind == Kind::Effect(effect))
            );
        }
        for tier in ["uncommon", "rare", "legendary", "mythical"] {
            assert!(
                OUTCOMES
                    .iter()
                    .any(|outcome| outcome.kind == Kind::Holocron(tier)),
                "{tier}"
            );
            assert!(crate::holocrons::by_id(tier).is_some(), "{tier}");
        }
    }

    #[test]
    fn each_outcome_stops_its_segment_under_the_pointer_from_any_rest() {
        assert_eq!(segment_angle(0), 0.0);
        assert_eq!(segment_angle(1), 342.0);
        assert_eq!(segment_angle(5), 270.0);
        for from in [0.0, 37.0, 300.0] {
            for segment in 0..SEGMENTS {
                // Segment k's middle is painted k segments clockwise of the top (y right,
                // z up, seen from +X): turned by the wheel's stop, it stands at the top.
                let painted = (segment as f32 * SEGMENT_DEGREES).to_radians();
                let middle = Vec3::new(0.0, painted.sin(), painted.cos());
                let stopped = wheel_turn(wheel_angle(SPIN, from, segment)) * middle;
                assert!(
                    stopped.abs_diff_eq(Vec3::Z, 2e-3),
                    "{from} {segment}: {stopped}"
                );
            }
        }
    }

    #[test]
    fn the_spin_lasts_the_music_and_slows_to_its_stop() {
        assert_eq!(SPIN, Duration::from_millis(34_000));
        let segment = 7;
        let step = Duration::from_millis(100);
        let mut last = 0.0;
        let mut last_speed = f32::MAX;
        let mut at = Duration::ZERO;
        while at < SPIN {
            at += step;
            let angle = wheel_angle(at, 0.0, segment);
            let speed = angle - last;
            assert!(speed >= 0.0 && speed <= last_speed + 1e-3, "{at:?}");
            last = angle;
            last_speed = speed;
        }
        assert_eq!(
            wheel_angle(SPIN, 0.0, segment),
            TURNS * 360.0 + segment_angle(segment)
        );
        assert_eq!(
            wheel_angle(SPIN * 2, 0.0, segment),
            wheel_angle(SPIN, 0.0, segment)
        );
        // Still turning ten seconds before the end.
        let late = wheel_angle(SPIN - Duration::from_secs(10), 0.0, segment);
        assert!(wheel_angle(SPIN, 0.0, segment) - late > SEGMENT_DEGREES);
    }

    #[test]
    fn a_spin_plays_its_music_ticks_stops_says_its_outcome_then_rests() {
        let mut toy = toy_with(&[5]);
        let start = Instant::now();
        assert!(toy.start(5, Some(3), true, "mirror", 6, start));
        assert!(
            !toy.start(9, Some(3), true, "mirror", 6, start),
            "no wheel 9"
        );
        assert_eq!(toy.cues[0].path, MUSIC);
        assert!(toy.tick(start + Duration::from_millis(500)).is_empty());
        assert!(toy.cues.iter().any(|cue| cue.path == TICK));
        assert!(toy.placed[0].busy(start + Duration::from_secs(20)));
        toy.cues.clear();
        let moments = toy.tick(start + SPIN);
        assert_eq!(
            moments,
            [Moment::Stopped {
                kind: Some(Kind::Effect(Effect::Mirror)),
                title: "MIRROR".to_owned(),
                line: "The world is mirrored for 30 s".to_owned()
            }]
        );
        let paths: Vec<_> = toy.cues.iter().map(|cue| cue.path).collect();
        assert!(paths.ends_with(&[STOP, LAUGH]), "{paths:?}");
        assert!(!toy.placed[0].busy(start + SPIN));
        assert!(toy.tick(start + SPIN + HOLD).is_empty());
        let wheel = &toy.placed[0];
        assert!(wheel.spin.is_none(), "its words gone");
        assert!(
            (wheel.rest - segment_angle(6)).abs() < 0.01,
            "it rests where it landed"
        );
    }

    #[test]
    fn holocrons_fanfare_and_explode_blows_up_later() {
        let mut toy = toy_with(&[1, 2]);
        let start = Instant::now();
        toy.start(1, Some(1), true, "holo_rare", 5, start);
        toy.start(2, Some(2), false, "explode", 4, start);
        let moments = toy.tick(start + SPIN);
        assert_eq!(moments.len(), 1, "only the player's own is told");
        assert!(toy.cues.iter().any(|cue| cue.path == FANFARE));
        let blast = toy.tick(start + SPIN + BLAST_DELAY);
        assert_eq!(
            blast,
            [Moment::Blast {
                slot: Some(2),
                at: Vec3::X * 2.0
            }]
        );
        assert!(toy.tick(start + SPIN + BLAST_DELAY * 2).is_empty(), "once");
    }

    #[test]
    fn an_unknown_outcome_is_shown_by_name_and_does_nothing() {
        let mut toy = toy_with(&[1]);
        let start = Instant::now();
        toy.start(1, Some(1), true, "big_party", 9, start);
        let moments = toy.tick(start + SPIN);
        assert_eq!(
            moments,
            [Moment::Stopped {
                kind: None,
                title: "BIG PARTY".to_owned(),
                line: "Nothing happens...".to_owned()
            }]
        );
        assert!(!toy.cues.iter().any(|cue| cue.path == LAUGH));
        assert!(!toy.effects.any());
    }

    #[test]
    fn a_spin_is_refused_while_busy_in_a_duel_in_the_cooldown_or_without_a_coin() {
        let ready = Check {
            strikable: true,
            coins: Some(2),
            ..Check::default()
        };
        assert_eq!(refusal(ready), None);
        assert!(
            refusal(Check {
                coins: None,
                ..ready
            })
            .is_none(),
            "unknown: the hub says"
        );
        let duel = Check {
            strikable: false,
            ..ready
        };
        assert!(refusal(duel).is_some_and(|why| why.contains("duels")));
        let cooling = Check {
            cooldown: Some(4),
            ..ready
        };
        assert_eq!(refusal(cooling).as_deref(), Some("Your next spin in 4 s"));
        let busy = Check {
            busy: true,
            ..ready
        };
        assert!(refusal(busy).is_some_and(|why| why.contains("Someone")));
        let poor = Check {
            coins: Some(0),
            ..ready
        };
        assert_eq!(refusal(poor).as_deref(), Some(wire::NO_COIN));
        let closed = Check {
            closed: true,
            ..ready
        };
        assert_eq!(refusal(closed).as_deref(), Some(wire::OUT_OF_ORDER));
        // The prompt says the same, a coin a spin.
        let prompt = wheel_prompt(ready, false).unwrap();
        assert!(prompt.ready && prompt.line.as_deref() == Some("1 DOOM coin (you hold 2)"));
        assert!(!wheel_prompt(poor, false).unwrap().ready);
        assert!(
            wheel_prompt(ready, true).is_none(),
            "the player's own spin runs"
        );
        let mut toy = WheelOfDoom::default();
        let now = Instant::now();
        toy.cooldown_until = Some(now + Duration::from_millis(3_200));
        assert_eq!(toy.cooldown_left(now), Some(4));
        assert_eq!(toy.cooldown_left(now + Duration::from_secs(4)), None);
    }

    #[test]
    fn the_wheel_faced_within_reach_is_the_one_spun() {
        let wheels = [
            placed(1, Vec3::new(100.0, 0.0, 0.0)),
            placed(2, Vec3::new(60.0, 10.0, 0.0)),
        ];
        assert_eq!(pick_wheel(Vec3::new(0.0, 0.0, 24.0), 0.0, &wheels), Some(2));
        assert_eq!(
            pick_wheel(Vec3::new(0.0, 0.0, 24.0), std::f32::consts::PI, &wheels),
            None
        );
        assert_eq!(pick_wheel(Vec3::new(-100.0, 0.0, 24.0), 0.0, &wheels), None);
    }

    #[test]
    fn wheels_are_taken_for_their_map_file_and_server_keeping_their_spins() {
        let wheel = |id, map: &str, checksum, server: &str| MysteryBox {
            id,
            map: map.to_owned(),
            checksum,
            server: server.to_owned(),
            origin: [1.0, 2.0, 3.0],
            yaw: 90.0,
        };
        let wheels = [
            wheel(1, "mp/ffa3", 5, ""),
            wheel(2, "mp/ffa3", 6, ""),
            wheel(3, "mp/ffa3", 5, "1.2.3.4:29070"),
        ];
        let mut toy = WheelOfDoom::default();
        toy.take(&wheels, "mp/ffa3", 5, "5.6.7.8:29070");
        assert_eq!(toy.placed.iter().map(|p| p.id).collect::<Vec<_>>(), [1]);
        let now = Instant::now();
        toy.start(1, None, false, "walk", 17, now);
        toy.take(&wheels, "mp/ffa3", 5, "1.2.3.4:29070");
        assert_eq!(toy.placed.iter().map(|p| p.id).collect::<Vec<_>>(), [1, 3]);
        assert!(toy.placed[0].spin.is_some(), "the spin goes on");
    }

    #[test]
    fn a_new_map_clears_the_spins_and_the_effects() {
        let mut toy = toy_with(&[1]);
        let now = Instant::now();
        toy.start(1, Some(1), true, "walk", 17, now);
        toy.effects.start(Effect::Walk, now);
        assert!(toy.running());
        toy.clear();
        assert!(toy.placed[0].spin.is_none() && !toy.effects.any());
    }
}
