//! The Toy Master's secrets (Sol's idea, 30/10/2026; `docs/toy-master.md`): toys SJK's
//! creator, the Toy Master, hides on maps through the hub, for players to find with
//! `+use`. This module knows no toy: the hub sends each one's model, sound and show with
//! the toys placed (`sjk_identity::toy_master`), so what is hidden stays a surprise.
//! It draws the toys standing on the loaded map file, counts the player's Use presses on
//! the one they face and sends them twice a second, and plays a toy's show when the hub
//! says the presses of every player there reached its number: the toy shrinks away,
//! pops back and spins to its sound. Other players' shows come through the hub's toy
//! event. The first press on a toy finds it: the hub unlocks its hidden achievement,
//! announced here. No game server knows of a toy: SJK clients alone draw it, and it has
//! no collision.

use crate::GpuState;
use crate::actor_instance::ActorInstance;
use glam::Vec3;
use sjk_identity::service::SecretDone;
use sjk_identity::toy_master::{self as wire, Placing, Show};
use sjk_protocol::UserCommand;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `BUTTON_USE`, `+use`.
const BUTTON_USE: u16 = 32;
/// `ET_PLAYER`, and the entity flags of a player not drawn: dead, `EF_NODRAW`.
const ET_PLAYER: u8 = 1;
const EF_DEAD: u32 = 1 << 1;
const EF_NODRAW: u32 = 1 << 8;
/// Farthest a player may stand from a toy to press it, across the floor.
const REACH: f32 = 80.0;
/// Farthest above or below the toy a player may stand to press it.
const REACH_UP: f32 = 96.0;
/// Widest angle between the player's facing and the toy, in radians (60 degrees).
const FACING: f32 = std::f32::consts::FRAC_PI_3;
/// Shortest time between two sendings of the presses; and most presses one carries (the
/// hub's `PRESSES_MAX`).
const SEND_EVERY: Duration = Duration::from_millis(500);
const PRESSES_MAX: u32 = 10;
/// Farthest a toy `toymaster_remove` takes without an id may stand.
const REMOVE_REACH: f32 = 256.0;
/// Played, with [`EFFECT`], as a toy shrinks away and as it pops back.
const VANISH_SOUND: &str = "sound/player/teleout.mp3";
const POP_SOUND: &str = "sound/player/telein.mp3";
const EFFECT: &str = "mp/spawn";
/// How far the show's sound carries: full up to the first, gone at the second.
const SOUND_FULL: f32 = 300.0;
const SOUND_RANGE: f32 = 1_400.0;
/// How loud the show's sound plays (twice its recording; the mixer clips the output).
const SHOW_VOLUME: f32 = 2.0;
/// The first of the toys' sound sources: past the game's entity numbers, the boombox's
/// (`0x5000_0000`), the Monitor's (`0x5100_0000`) and the mystery boxes'
/// (`0x5200_0000`).
const SOURCE_BASE: u32 = 0x5300_0000;
/// The channel a toy's show sound plays on (stopped when the show ends; kept from other
/// sounds as the boombox's is), and its other sounds'.
const SHOW_CHANNEL: sjk_audio::ChannelId = crate::audio::TOY_SHOW_CHANNEL;
const CUE_CHANNEL: sjk_audio::ChannelId = sjk_audio::ChannelId(0);

/// Toy `id`'s own sound source.
fn source(id: i64) -> sjk_audio::SourceId {
    sjk_audio::SourceId(SOURCE_BASE | (id as u32 & 0xFFFF))
}

/// The creator's console commands, with their help.
pub(crate) const COMMANDS: &[(&str, &str)] = &[
    (
        "toymaster_place",
        "SJK creator: hide a toy of the kind given where you aim, facing you ('server': for this server only; 'escort': shown only on the server you are on)",
    ),
    (
        "toymaster_escort",
        "SJK creator: show the toy nearest you (or the one with the id given) only on the server you are on (on), or to everyone (off)",
    ),
    (
        "toymaster_remove",
        "SJK creator: take away the hidden toy nearest you, or the one with the id given",
    ),
    (
        "toymaster_list",
        "SJK creator: list the toys hidden on this map and the kinds there are",
    ),
];

/// Whether `name` is one of [`COMMANDS`].
pub(crate) fn is_command(name: &str) -> bool {
    COMMANDS.iter().any(|(command, _)| *command == name)
}

mod drops;
mod float;
mod track;

use track::{Moment, Track};

/// Players' models round a toy whose show has no reach (01/11/2026) dance within this
/// distance of it, fading out to none at [`DANCE_FADE`]; a show with a reach has them
/// dance within it, fading out over [`DANCE_EDGE`] more.
const DANCE_FULL: f32 = 320.0;
const DANCE_FADE: f32 = 512.0;
const DANCE_EDGE: f32 = 48.0;
/// How much a model squishes at full speed: this much shorter at the bottom of a hop,
/// as much wider as keeps it about its size.
const SQUISH: f32 = 0.3;

/// A toy's spin as the players around it dance it this frame: where it stands, how far
/// they turn (degrees), how far its hop squishes them (0 to 1), and how near they must
/// stand: fully within `full`, not at all past `fade`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Dance {
    origin: Vec3,
    turn: f32,
    squish: f32,
    full: f32,
    fade: f32,
}

/// This frame's dances, for the actors drawn after ([`dance_actor`]).
static DANCES: std::sync::Mutex<Vec<Dance>> = std::sync::Mutex::new(Vec::new());

fn dances() -> std::sync::MutexGuard<'static, Vec<Dance>> {
    DANCES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Turn and squish an actor's drawn `transform` (feet `foot` units under its origin) as
/// the spinning toy nearest it has the players around dance: its looks alone, never
/// where the game has it nor which way it faces.
pub(crate) fn dance_actor(transform: &mut sjk_runtime::Transform, foot: f32) {
    let dances = dances();
    if !dances.is_empty() {
        dance_with(transform, foot, &dances);
    }
}

/// [`dance_actor`] among `dances`.
fn dance_with(transform: &mut sjk_runtime::Transform, foot: f32, dances: &[Dance]) {
    let at = Vec3::from_array(transform.translation);
    let Some((dance, near)) = dances
        .iter()
        .map(|dance| (dance, dance.origin.distance(at)))
        .filter(|(dance, near)| *near < dance.fade)
        .min_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return;
    };
    let strength = 1.0 - ((near - dance.full) / (dance.fade - dance.full)).clamp(0.0, 1.0);
    let turn = glam::Quat::from_rotation_z((dance.turn * strength).to_radians());
    transform.rotation = (turn * glam::Quat::from_array(transform.rotation)).to_array();
    let squish = dance.squish * strength * SQUISH;
    if squish > 0.0 {
        let [x, y, z] = transform.scale;
        let wide = 1.0 + squish * 0.5;
        transform.scale = [x * wide, y * wide, z * (1.0 - squish)];
        // The feet stay on the floor.
        transform.translation[2] -= foot * z * squish;
    }
}

/// How far a show reaches round its toy: fully, and not at all past the second.
fn reach_of(show: &Show) -> (f32, f32) {
    if show.reach > 0.0 {
        (show.reach, show.reach + DANCE_EDGE)
    } else {
        (DANCE_FULL, DANCE_FADE)
    }
}

/// What a drop is drawn as, and how much it is grown: credits a green crystal, kyber
/// shards a kyber crystal, a holocron SJK's holocron (its tier is told to its taker only).
fn drop_look(kind: wire::DropKind) -> (&'static str, f32) {
    match kind {
        wire::DropKind::Credits => ("models/map_objects/mp/crystal_green.md3", 1.3),
        wire::DropKind::Shards => ("models/sjk/holocron_kyber_rare_crystal.md3", 2.6),
        wire::DropKind::Holocron => (crate::illuminate::MODEL, 3.0),
    }
}

/// What the Use prompt says at a drop, and its line.
fn drop_words(kind: wire::DropKind) -> &'static str {
    match kind {
        wire::DropKind::Credits => "Take the credits",
        wire::DropKind::Shards => "Take the kyber shards",
        wire::DropKind::Holocron => "Take the holocron",
    }
}

/// How long a show's drops wait for takers after it ends (the hub's `DROPS_KEPT`).
const DROPS_KEPT: Duration = Duration::from_secs(60);
/// How long the player waits for the hub's answer about a drop before asking again.
const TAKE_WAIT: Duration = Duration::from_secs(3);

/// One toy standing on the loaded map.
#[derive(Clone, Debug, PartialEq)]
struct Here {
    id: i64,
    origin: Vec3,
    /// Facing, in degrees.
    yaw: f32,
    model: String,
    scale: f32,
    sound: String,
    /// How its show plays.
    track: Track,
    /// Its kind, and whether it shows only where SJK's creator is (the creator's list
    /// alone).
    kind: Option<String>,
    escorted: bool,
    /// Its show under way, and the moments it played so far (0 nothing, 1 the vanish, 2
    /// the pop).
    playing: Option<(Instant, u8)>,
    /// When its last show started, kept after it for its drops, and that show's seed.
    show_at: Option<Instant>,
    seed: u32,
    /// What its last show threw out, and who floats in it.
    drops: drops::Drops,
    floats: float::Floats,
    /// It played its show on this map load: presses count no more.
    done: bool,
    /// The player's presses on it reached the hub on this map load (so it is found).
    told: bool,
}

/// A sound to play once, or the show's to stop.
struct Cue {
    path: Option<String>,
    origin: [f32; 3],
    source: sjk_audio::SourceId,
    /// The show's sound, carrying far, on [`SHOW_CHANNEL`].
    show: bool,
    /// How far into the sound it plays from, in milliseconds (a show joined late).
    from_ms: u64,
}

/// A toy the creator asked to hide, waiting for the map's checksum.
struct PendingPlace {
    kind: String,
    origin: Vec3,
    yaw: f32,
    server: String,
    escorted: bool,
}

/// The toys of the loaded map, the presses waiting to be sent and the sounds to play.
#[derive(Default)]
pub(crate) struct ToyMaster {
    /// The hub list's revision the toys were taken from, and the map, checksum, server
    /// and map load they were taken for.
    taken: Option<(u64, String, i32, String, i32)>,
    here: Vec<Here>,
    /// The kinds of toy, as the hub tells the creator.
    kinds: Vec<String>,
    /// The meshes of the toys' models once loaded, and the mesh count they were looked
    /// up at.
    meshes: Vec<(String, Option<usize>)>,
    meshes_seen: usize,
    use_pressed: bool,
    use_held: bool,
    /// Presses on a toy not sent yet, and when the last were.
    presses: Option<(i64, u32)>,
    sent: Option<Instant>,
    /// The last hub answer read.
    answer_seen: u64,
    pending_place: Option<PendingPlace>,
    cues: Vec<Cue>,
    /// Where this frame's sparkles go.
    sparkles: Vec<Vec3>,
    /// The audio generation, the list revision and the game data (its address) the
    /// sounds were handed over for: a pack that came since (the Sith cloud's) is heard.
    sounds_registered: Option<(u64, u64, usize)>,
    /// The movers the floating players and the drops meet (`ragdoll::Scene`).
    movers: crate::ragdoll::Movers,
    /// A world shot's toys, kept without a session.
    #[cfg(test)]
    shot: bool,
}

/// A hidden toy as the browser map editor shows it.
pub(crate) struct Hidden<'a> {
    pub(crate) id: i64,
    pub(crate) kind: Option<&'a str>,
    pub(crate) model: &'a str,
    pub(crate) origin: Vec3,
    /// Facing, in degrees.
    pub(crate) yaw: f32,
    pub(crate) scale: f32,
    pub(crate) escorted: bool,
}

impl ToyMaster {
    /// The toys hidden on the loaded map.
    pub(crate) fn hidden(&self) -> impl Iterator<Item = Hidden<'_>> {
        self.here.iter().map(|here| Hidden {
            id: here.id,
            kind: here.kind.as_deref(),
            model: &here.model,
            origin: here.origin,
            yaw: here.yaw,
            scale: here.scale,
            escorted: here.escorted,
        })
    }

    /// The kinds of toy, as the hub tells the creator.
    pub(crate) fn kinds(&self) -> &[String] {
        &self.kinds
    }

    /// Note a `+use` press in `command` for the frame's logic.
    pub(crate) fn observe_use(&mut self, command: &UserCommand) {
        let using = command.buttons & BUTTON_USE != 0;
        if using && !self.use_held {
            self.use_pressed = true;
        }
        self.use_held = using;
    }

    /// The models to load: those of the toys here not found among the loaded ones.
    pub(crate) fn wanted_models(&self) -> impl Iterator<Item = &str> {
        self.meshes
            .iter()
            .filter(|(_, mesh)| mesh.is_none())
            .map(|(model, _)| model.as_str())
    }

    fn mesh_of(&self, model: &str) -> Option<usize> {
        self.meshes
            .iter()
            .find(|(known, _)| known == model)
            .and_then(|(_, mesh)| *mesh)
    }

    /// Leave the map: the toys go, and their sounds stop.
    fn clear(&mut self) {
        for here in self.here.drain(..) {
            if here.playing.is_some() {
                self.cues.push(Cue {
                    path: None,
                    origin: here.origin.to_array(),
                    source: source(here.id),
                    show: true,
                    from_ms: 0,
                });
            }
        }
        self.taken = None;
        self.meshes.clear();
        self.presses = None;
        self.use_pressed = false;
        dances().clear();
        crate::ragdoll::set_floating([]);
    }

    /// Hand the actors drawn this frame the spinning toys' dances and the floating
    /// players, as at `now`.
    fn publish(&self, now: Instant) {
        let mut dances = dances();
        dances.clear();
        let mut floating = Vec::new();
        for here in &self.here {
            let moment = here.moment(now);
            if let Some((turn, squish)) = moment.and_then(|moment| moment.dance) {
                let (full, fade) = reach_of(here.track.show());
                dances.push(Dance {
                    origin: here.origin,
                    turn,
                    squish,
                    full,
                    fade,
                });
            }
            let twirl = moment
                .and_then(|moment| moment.dance)
                .map_or(0.0, |(turn, _)| turn);
            floating.extend(here.floats.shown(twirl));
        }
        crate::ragdoll::set_floating(floating);
    }

    /// Take the toys standing on `map` of `checksum` on `server` from `secrets`, keeping
    /// what each was doing; `fresh_round` when the map load changed (shows may come
    /// again).
    fn take(
        &mut self,
        secrets: &wire::Secrets,
        map: &str,
        checksum: i32,
        server: &str,
        fresh_round: bool,
    ) {
        let old = std::mem::take(&mut self.here);
        self.kinds = secrets.kinds.clone();
        self.here = secrets
            .secrets
            .iter()
            .filter(|placed| placed.stands_on(map, checksum, Some(server)))
            .map(|placed| {
                let before = old.iter().find(|here| here.id == placed.id);
                Here {
                    id: placed.id,
                    origin: Vec3::from_array(placed.origin),
                    yaw: placed.yaw,
                    model: placed.model.clone(),
                    scale: placed.scale.clamp(0.05, 20.0),
                    sound: placed.sound.clone(),
                    track: before
                        .filter(|here| *here.track.show() == placed.show)
                        .map_or_else(|| Track::new(&placed.show), |here| here.track.clone()),
                    kind: placed.kind.clone(),
                    escorted: placed.escorted,
                    playing: before.and_then(|here| here.playing),
                    show_at: before.and_then(|here| here.show_at),
                    seed: before.map_or(0, |here| here.seed),
                    drops: before.map(|here| here.drops.clone()).unwrap_or_default(),
                    floats: before.map(|here| here.floats.clone()).unwrap_or_default(),
                    done: !fresh_round && before.is_some_and(|here| here.done),
                    told: !fresh_round && before.is_some_and(|here| here.told),
                }
            })
            .collect();
        let mut meshes: Vec<(String, Option<usize>)> = Vec::new();
        for here in &self.here {
            let show = here.track.show();
            let drops = show
                .steps
                .iter()
                .any(|step| step.spit)
                .then_some([
                    wire::DropKind::Credits,
                    wire::DropKind::Shards,
                    wire::DropKind::Holocron,
                ])
                .into_iter()
                .flatten()
                .map(|kind| drop_look(kind).0);
            let models = [here.model.as_str(), show.ring.as_str()]
                .into_iter()
                .chain(drops);
            for model in models {
                if !model.is_empty() && !meshes.iter().any(|(known, _)| known == model) {
                    meshes.push((model.to_owned(), None));
                }
            }
        }
        self.meshes = meshes;
        self.meshes_seen = usize::MAX;
    }

    /// Toy `id` starts its show at `now`, unless it is playing one.
    fn start(&mut self, id: i64, now: Instant) {
        if let Some(here) = self.here.iter_mut().find(|here| here.id == id)
            && here.playing.is_none()
        {
            here.playing = Some((now, 0));
            here.show_at = Some(now);
            here.seed = 0;
            here.drops = drops::Drops::default();
            here.floats = float::Floats::default();
            here.done = true;
        }
    }

    /// Toy `id`'s show, under way for `elapsed` already (the player arrived after it
    /// started, as the hub's presence tells it): joined where it is, its vanish and pop
    /// past when they are, its sound from as far in, its drops thrown as they were and
    /// the ones taken gone. Nothing when the toy played or joined a show on this map load.
    fn join(&mut self, under_way: &wire::ShowUnderWay, elapsed: Duration, now: Instant) {
        let Some(here) = self
            .here
            .iter_mut()
            .find(|here| here.id == under_way.id && here.show_at.is_none() && !here.done)
        else {
            return;
        };
        let Some(started) = now.checked_sub(elapsed) else {
            return;
        };
        let show = here.track.show();
        let back = Duration::from_millis(u64::from(show.vanish + show.gone));
        let length = Duration::from_millis(u64::from(here.track.length()));
        if elapsed >= length + DROPS_KEPT {
            return;
        }
        // Past the pop: its sounds and sparkles are over, the show's sound plays from as
        // far in as it is (`tick` plays it from the pop otherwise).
        let played = if elapsed >= back { 2 } else { 0 };
        if played == 2 && elapsed < length && !here.sound.is_empty() {
            self.cues.push(Cue {
                path: Some(here.sound.clone()),
                origin: here.origin.to_array(),
                source: source(here.id),
                show: true,
                from_ms: u64::try_from((elapsed - back).as_millis()).unwrap_or(u64::MAX),
            });
        }
        here.playing = (elapsed < length).then_some((started, played));
        here.show_at = Some(started);
        here.seed = under_way.seed;
        here.drops = drops::Drops::default();
        here.floats = float::Floats::default();
        here.done = true;
        if let Some(kinds) = under_way.kinds().filter(|kinds| !kinds.is_empty()) {
            here.drops = drops::Drops::new(here.track.show(), under_way.seed, &kinds, here.origin);
            for drop in 0..kinds.len() as u32 {
                if under_way.taken & (1 << drop) != 0
                    && let Some(drop) = here.drops.get_mut(drop)
                {
                    drop.taken = true;
                }
            }
        }
    }

    /// Toy `id`'s show throws `kinds`, from `seed` (it must be under way).
    fn throw(&mut self, id: i64, seed: u32, kinds: &[wire::DropKind]) {
        if let Some(here) = self.here.iter_mut().find(|here| here.id == id)
            && here.show_at.is_some()
            && here.drops.list.is_empty()
        {
            here.drops = drops::Drops::new(here.track.show(), seed, kinds, here.origin);
            here.seed = seed;
        }
    }

    /// Drop `drop` of toy `id`'s show was taken.
    fn taken(&mut self, id: i64, drop: u32) {
        if let Some(drop) = self
            .here
            .iter_mut()
            .find(|here| here.id == id)
            .and_then(|here| here.drops.get_mut(drop))
        {
            drop.taken = true;
            drop.asked = None;
        }
    }

    /// Move the shows on to `now`: the vanish's and the pop's sounds and sparkles, the
    /// show's sound with the pop, and its end.
    fn tick(&mut self, now: Instant) {
        for here in &mut self.here {
            let Some((started, played)) = here.playing.as_mut() else {
                continue;
            };
            let elapsed = now.saturating_duration_since(*started);
            let origin = here.origin.to_array();
            if *played == 0 {
                *played = 1;
                self.cues.push(Cue {
                    path: Some(VANISH_SOUND.to_owned()),
                    origin,
                    source: source(here.id),
                    show: false,
                    from_ms: 0,
                });
                self.sparkles.push(here.origin);
            }
            let show = here.track.show();
            let back = Duration::from_millis(u64::from(show.vanish + show.gone));
            if *played == 1 && elapsed >= back {
                *played = 2;
                self.cues.push(Cue {
                    path: Some(POP_SOUND.to_owned()),
                    origin,
                    source: source(here.id),
                    show: false,
                    from_ms: 0,
                });
                if !here.sound.is_empty() {
                    self.cues.push(Cue {
                        path: Some(here.sound.clone()),
                        origin,
                        source: source(here.id),
                        show: true,
                        from_ms: 0,
                    });
                }
                self.sparkles.push(here.origin);
            }
            if elapsed >= Duration::from_millis(u64::from(here.track.length())) {
                here.playing = None;
                self.cues.push(Cue {
                    path: None,
                    origin,
                    source: source(here.id),
                    show: true,
                    from_ms: 0,
                });
            }
        }
    }

    /// Whether the player waits for the hub's answer about a drop.
    fn asking(&self) -> bool {
        self.here
            .iter()
            .any(|here| here.drops.list.iter().any(|drop| drop.asked.is_some()))
    }

    /// The player asks for no drop any more (the hub refused).
    fn forget_asked(&mut self) {
        for here in &mut self.here {
            for drop in &mut here.drops.list {
                drop.asked = None;
            }
        }
    }

    /// Count a press on toy `id`: the presses on another toy not sent yet are dropped.
    fn press(&mut self, id: i64) {
        self.presses = match self.presses {
            Some((toy, count)) if toy == id => Some((toy, (count + 1).min(PRESSES_MAX))),
            _ => Some((id, 1)),
        };
    }

    /// The presses to send at `now`, when it is time.
    fn due(&mut self, now: Instant) -> Option<(i64, u32)> {
        if self
            .sent
            .is_some_and(|sent| now.saturating_duration_since(sent) < SEND_EVERY)
        {
            return None;
        }
        let presses = self.presses.take()?;
        self.sent = Some(now);
        Some(presses)
    }
}

/// What the Use prompt says at a toy: what it does is the secret.
const PROMPT: &str = "???";

/// Whether `here` stands to be seen at `now`: not while its show has it shrunk away.
fn shown(here: &Here, now: Instant) -> bool {
    here.moment(now).is_none_or(|moment| moment.scale > 0.01)
}

impl Here {
    /// Where its show is at `now`, while it plays.
    fn moment(&self, now: Instant) -> Option<Moment> {
        let (started, _) = self.playing?;
        self.track
            .at(now.saturating_duration_since(started).as_secs_f32() * 1_000.0)
    }

    /// Milliseconds into its last show at `now`, while its drops wait (the show and
    /// [`DROPS_KEPT`] after).
    fn show_elapsed(&self, now: Instant) -> Option<f32> {
        let since = now.saturating_duration_since(self.show_at?);
        (since < Duration::from_millis(u64::from(self.track.length())) + DROPS_KEPT)
            .then_some(since.as_secs_f32() * 1_000.0)
    }
}

/// The toy among `toys` the player at `from` facing `yaw` (radians) may press: within
/// [`REACH`] across the floor and [`REACH_UP`] up or down, within [`FACING`] of their
/// facing; the nearest of those.
fn pick<'a>(from: Vec3, yaw: f32, toys: impl IntoIterator<Item = &'a Here>) -> Option<i64> {
    let facing = glam::Vec2::from_angle(yaw);
    toys.into_iter()
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

impl GpuState {
    fn toy_master_print(&mut self, text: String) {
        self.chat.receive(
            sjk_client::ServerEventKind::Print,
            text,
            None,
            Instant::now(),
        );
    }

    /// Find the toys' models among the loaded ones, when more have loaded.
    fn find_toy_master_meshes(&mut self) {
        if self.toy_master.meshes_seen == self.object_meshes.len() {
            return;
        }
        self.toy_master.meshes_seen = self.object_meshes.len();
        for (model, mesh) in &mut self.toy_master.meshes {
            *mesh = self.object_meshes.iter().position(|loaded| {
                loaded.appearance.variant.is_empty() && loaded.appearance.model == *model
            });
        }
    }

    /// The frame's Toy Master logic, after the looks: the toys of the loaded map, the
    /// hub's answers, the shows (`events`, this server's toy events) and the player's
    /// presses.
    pub(crate) fn update_toy_master(&mut self, events: &[sjk_identity::ToyEvent]) {
        #[cfg(test)]
        if self.toy_master.shot {
            self.find_toy_master_meshes();
            return;
        }
        let now = Instant::now();
        self.read_toy_master_answer(now);
        let Some(session) = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
        else {
            self.toy_master.clear();
            return;
        };
        let server = session.server().to_string();
        let round = session.server_id();
        let map_name = crate::remap_blocked_maps::map_name(&self.world_load_map);
        let (revision, on_map) = crate::player_identity::with_snapshot(|snapshot| {
            (
                snapshot.secrets_revision,
                snapshot
                    .secrets
                    .as_ref()
                    .is_some_and(|secrets| secrets.on_map(map_name)),
            )
        })
        .unwrap_or_default();
        // The map's checksum is read when a toy may stand on it (`map_edits`).
        if on_map || self.toy_master.pending_place.is_some() {
            self.map_edits.want_checksum(map_name);
        }
        let Some((map, checksum)) = self.map_edits.map().map(|(map, c)| (map.to_owned(), c)) else {
            self.toy_master.clear();
            return;
        };
        self.place_pending_toy(&map, checksum);
        let taken = self.toy_master.taken.as_ref();
        let fresh_round = taken.is_none_or(|taken| taken.4 != round);
        let fresh = taken.is_some_and(|(taken, taken_map, taken_checksum, taken_server, _)| {
            *taken == revision
                && *taken_checksum == checksum
                && *taken_map == map
                && *taken_server == server
        });
        if !fresh || fresh_round {
            let secrets = crate::player_identity::with_snapshot(|snapshot| {
                snapshot.secrets.clone().unwrap_or_default()
            })
            .unwrap_or_default();
            self.toy_master
                .take(&secrets, &map, checksum, &server, fresh_round);
            self.toy_master.taken = Some((revision, map, checksum, server.clone(), round));
        }
        self.find_toy_master_meshes();
        for event in events {
            if event.toy != wire::TOY {
                continue;
            }
            match wire::Event::parse(&event.event) {
                Some(wire::Event::Show(id)) => self.toy_master.start(id, now),
                Some(wire::Event::Drops { id, seed, kinds }) => {
                    self.toy_master.throw(id, seed, &kinds);
                }
                Some(wire::Event::Taken { id, drop }) => self.toy_master.taken(id, drop),
                None => {}
            }
        }
        // Shows that started before the player came (or loaded the map): joined where
        // they are, for this map load only.
        if let Some((told, shows)) =
            crate::player_identity::with_snapshot(|snapshot| snapshot.secret_shows.clone())
                .flatten()
        {
            let since = now.saturating_duration_since(told);
            for under_way in shows
                .iter()
                .filter(|under_way| under_way.round == i64::from(round))
            {
                let elapsed = Duration::from_millis(under_way.ago) + since;
                self.toy_master.join(under_way, elapsed, now);
            }
        }
        self.toy_master.tick(now);
        self.run_toy_master_bodies(now);
        self.toy_master.publish(now);
        if self.toy_master.here.is_empty() {
            self.toy_master.use_pressed = false;
            return;
        }
        let Some(session) = self.live_session.as_ref() else {
            return;
        };
        let player = &session.latest_snapshot().player;
        let pressed = std::mem::take(&mut self.toy_master.use_pressed);
        let plays = crate::mystery_box::plays(player);
        let from = Vec3::from_array(
            self.local_prediction
                .predicted_state()
                .map_or_else(|| player.origin(), |state| state.origin),
        );
        // A drop at rest the player faces comes first: Use takes it.
        let faced_drop = plays
            .then(|| {
                self.toy_master.here.iter().find_map(|here| {
                    here.drops
                        .faced(from, self.camera_yaw)
                        .map(|drop| (here.id, drop.index, drop.kind, drop.asked))
                })
            })
            .flatten();
        if let Some((id, index, kind, asked)) = faced_drop {
            let waiting = asked.is_some_and(|asked| now.duration_since(asked) < TAKE_WAIT);
            self.want_use_prompt(crate::use_prompt::Prompt {
                action: drop_words(kind).to_owned(),
                line: None,
                ready: !waiting,
            });
            if pressed && !waiting {
                if let Some(drop) = self
                    .toy_master
                    .here
                    .iter_mut()
                    .find(|here| here.id == id)
                    .and_then(|here| here.drops.get_mut(index))
                {
                    drop.asked = Some(now);
                }
                let server = server.clone();
                crate::player_identity::with_service(|service| {
                    service.take_secret_drop(id, server, round, index);
                });
            }
        }
        let faced = (plays && faced_drop.is_none())
            .then(|| pick(from, self.camera_yaw, &self.toy_master.here))
            .flatten()
            .and_then(|id| self.toy_master.here.iter().find(|here| here.id == id))
            .filter(|here| shown(here, now));
        // A toy that played its show here, once the player found it, takes no more.
        let counts = faced.map(|here| (here.id, !(here.done && here.told)));
        if let Some((id, counts)) = counts {
            // What Use does at it, by the crosshair as at a mystery box (`use_prompt`): a
            // secret, so "???" (Sol, 01/11/2026), greyed once it takes no more.
            self.want_use_prompt(crate::use_prompt::Prompt {
                action: PROMPT.to_owned(),
                line: None,
                ready: counts,
            });
            if pressed && counts {
                self.toy_master.press(id);
            }
        }
        if let Some((id, presses)) = self.toy_master.due(now) {
            crate::player_identity::with_service(|service| {
                service.use_secret(id, server, round, presses);
            });
        }
    }

    /// Act on the hub's last answer about the toys, once.
    fn read_toy_master_answer(&mut self, now: Instant) {
        let seen = self.toy_master.answer_seen;
        let Some((serial, outcome)) = crate::player_identity::with_snapshot(|snapshot| {
            snapshot
                .secret
                .as_ref()
                .filter(|(serial, _)| *serial != seen)
                .cloned()
        })
        .flatten() else {
            return;
        };
        self.toy_master.answer_seen = serial;
        match outcome {
            Ok(SecretDone::Used {
                id,
                show,
                done,
                found,
            }) => {
                if show {
                    self.toy_master.start(id, now);
                }
                if let Some(here) = self.toy_master.here.iter_mut().find(|here| here.id == id) {
                    here.told = true;
                    here.done |= done;
                }
                if found {
                    self.announce_toy_found();
                }
            }
            Ok(SecretDone::Took {
                id,
                drop,
                kind,
                amount,
                tier,
            }) => {
                self.toy_master.taken(id, drop);
                let what = match kind.as_str() {
                    "credits" => format!("{amount} credits"),
                    "shards" => format!("{amount} kyber shards"),
                    "holocron" => format!(
                        "a{} holocron",
                        tier.map_or(String::new(), |tier| format!(" {tier}"))
                    ),
                    other => other.to_owned(),
                };
                self.toy_master_print(format!("^3You took {what}."));
            }
            Ok(SecretDone::Placed(id)) => self.toy_master_print(format!("Toy {id} hidden")),
            Ok(SecretDone::Removed(id)) => self.toy_master_print(format!("Toy {id} taken away")),
            Ok(SecretDone::Escorted(id, true)) => {
                self.toy_master_print(format!("Toy {id} now shows only on the server you are on"))
            }
            Ok(SecretDone::Escorted(id, false)) => {
                self.toy_master_print(format!("Toy {id} now shows to everyone"));
            }
            // A drop asked for and refused: the player may ask again (the hub's
            // `taken_` takes it away when someone else has it).
            Err(why) if self.toy_master.asking() => {
                self.toy_master.forget_asked();
                self.toy_master_print(why);
            }
            // Presses on a toy are cosmetic: a refusal tells the creator alone.
            Err(why) if crate::player_identity::is_creator() => {
                self.toy_master_print(format!("Toy Master: {why}"));
            }
            Err(_) => {}
        }
    }

    /// The player found a toy: its achievement, the newest of the Toy Master's they hold,
    /// is announced as any other (the hub granted its kyber crystal).
    fn announce_toy_found(&mut self) {
        let held = crate::player_identity::own_achievements().unwrap_or_default();
        let Some(kind) = held
            .iter()
            .filter(|held| held.unlocked > 0)
            .filter_map(|held| {
                crate::achievements::secret_kind(held).map(|kind| (kind, held.unlocked))
            })
            .max_by_key(|(_, unlocked)| *unlocked)
            .map(|(kind, _)| kind)
        else {
            return;
        };
        if let Some(console) = self.console.as_mut() {
            console.push_log(format!(
                "^3Achievement unlocked: ^7{} ^5({})",
                kind.name, kind.description
            ));
        }
        self.unlock_toast
            .push(crate::unlock_toast::Unlock::Achievement {
                kind,
                crystal: true,
            });
    }

    /// Send the toy the creator asked to hide once the map's checksum is known.
    fn place_pending_toy(&mut self, map: &str, checksum: i32) {
        let Some(pending) = self.toy_master.pending_place.take() else {
            return;
        };
        let placing = Placing {
            kind: pending.kind,
            map: map.to_owned(),
            checksum,
            server: pending.server,
            origin: pending.origin.to_array(),
            yaw: pending.yaw,
            escorted: pending.escorted,
        };
        if crate::player_identity::with_service(|service| service.place_secret(placing)).is_none() {
            self.toy_master_print("Toy Master: not connected to the SJK hub".to_owned());
        }
    }

    /// `toymaster_place`, `toymaster_remove` and `toymaster_list`: SJK's creator only.
    pub(crate) fn toy_master_command(
        &mut self,
        name: &str,
        args: &[String],
    ) -> Result<Vec<String>, String> {
        if !crate::player_identity::is_creator() {
            return Err("Only the Toy Master hides toys".into());
        }
        if name == "toymaster_list" {
            return Ok(self.toy_master_list());
        }
        let session = self
            .live_session
            .as_ref()
            .filter(|session| !session.is_local())
            .ok_or("Join a game server first: toys stand on a server's map")?;
        let server = session.server().to_string();
        match name {
            "toymaster_place" => {
                const USAGE: &str = "toymaster_place <kind> [server] [escort]";
                let kind = args
                    .first()
                    .ok_or(format!("{USAGE} (toymaster_list names the kinds)"))?
                    .to_owned();
                let (mut one_server, mut escorted) = (false, false);
                for word in &args[1..] {
                    match word.as_str() {
                        "server" => one_server = true,
                        "escort" => escorted = true,
                        other => return Err(format!("{USAGE}, not {other}")),
                    }
                }
                let (eye, forward) = self.world_notes.view().ok_or("No view to aim from")?;
                let hit = self
                    .decal_surfaces
                    .ray_hit(eye, forward, 4096.0)
                    .ok_or("Aim at the floor where the toy should stand")?;
                let yaw = forward.y.atan2(forward.x).to_degrees() + 180.0;
                Ok(vec![self.toy_master_place_at(
                    kind,
                    eye + forward * hit.distance,
                    yaw,
                    one_server.then_some(server),
                    escorted,
                )])
            }
            "toymaster_escort" => {
                const USAGE: &str = "toymaster_escort [id] on|off";
                let (id, word) = match args {
                    [word] => (None, word),
                    [id, word] => (
                        Some(id.parse().map_err(|_| format!("Not a toy id: {id}"))?),
                        word,
                    ),
                    _ => return Err(USAGE.into()),
                };
                let escorted = match word.as_str() {
                    "on" => true,
                    "off" => false,
                    other => return Err(format!("{USAGE}, not {other}")),
                };
                let id = match id {
                    Some(id) => id,
                    None => {
                        let from = Vec3::from_array(session.latest_snapshot().player.origin());
                        self.toy_master
                            .here
                            .iter()
                            .map(|here| (here.id, here.origin.distance(from)))
                            .filter(|(_, distance)| *distance <= REMOVE_REACH)
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(id, _)| id)
                            .ok_or("No hidden toy near you: give its id (toymaster_list)")?
                    }
                };
                crate::player_identity::with_service(|service| {
                    service.escort_secret(id, escorted);
                })
                .ok_or("Not connected to the SJK hub")?;
                Ok(vec![format!("Setting toy {id}...")])
            }
            "toymaster_remove" => {
                let id = match args.first() {
                    Some(id) => id.parse().map_err(|_| format!("Not a toy id: {id}"))?,
                    None => {
                        let from = Vec3::from_array(session.latest_snapshot().player.origin());
                        self.toy_master
                            .here
                            .iter()
                            .map(|here| (here.id, here.origin.distance(from)))
                            .filter(|(_, distance)| *distance <= REMOVE_REACH)
                            .min_by(|a, b| a.1.total_cmp(&b.1))
                            .map(|(id, _)| id)
                            .ok_or("No hidden toy near you: give its id (toymaster_list)")?
                    }
                };
                crate::player_identity::with_service(|service| service.remove_secret(id))
                    .ok_or("Not connected to the SJK hub")?;
                Ok(vec![format!("Taking toy {id} away...")])
            }
            _ => Err(format!("Unknown command {name}")),
        }
    }

    /// Ask the hub to hide a toy of `kind` at `origin` facing `yaw` degrees, on every
    /// server running this map file or on `server` only, once the map's checksum is
    /// known (`toymaster_place` aims; the browser map editor gives the spot). The
    /// caller has checked the creator and the server.
    pub(crate) fn toy_master_place_at(
        &mut self,
        kind: String,
        origin: Vec3,
        yaw: f32,
        server: Option<String>,
        escorted: bool,
    ) -> String {
        let one_server = server.is_some();
        let line = format!(
            "Hiding a {kind} on {}{}{}...",
            crate::remap_blocked_maps::map_name(&self.world_load_map),
            if one_server { " for this server" } else { "" },
            if escorted {
                ", shown only where you are"
            } else {
                ""
            }
        );
        self.toy_master.pending_place = Some(PendingPlace {
            kind,
            origin,
            yaw: yaw.rem_euclid(360.0),
            server: server.unwrap_or_default(),
            escorted,
        });
        let map = crate::remap_blocked_maps::map_name(&self.world_load_map).to_owned();
        self.map_edits.want_checksum(&map);
        line
    }

    fn toy_master_list(&self) -> Vec<String> {
        let toys = &self.toy_master;
        let mut lines = vec![if toys.kinds.is_empty() {
            "Toy kinds: none read from the hub yet".to_owned()
        } else {
            format!("Toy kinds: {}", toys.kinds.join(", "))
        }];
        if toys.here.is_empty() {
            lines.push("No toy hidden on this map".to_owned());
            return lines;
        }
        let from = self
            .live_session
            .as_ref()
            .map(|session| Vec3::from_array(session.latest_snapshot().player.origin()));
        lines.push(format!("{} toys hidden here:", toys.here.len()));
        for here in &toys.here {
            let away = from.map_or(String::new(), |from| {
                format!(", {:.0} units away", here.origin.distance(from))
            });
            let done = if here.done { ", played its show" } else { "" };
            let escorted = if here.escorted {
                ", shown only where you are"
            } else {
                ""
            };
            lines.push(format!(
                "  {}: {} at {:.0} {:.0} {:.0}{away}{done}{escorted}",
                here.id,
                here.kind.as_deref().unwrap_or("?"),
                here.origin.x,
                here.origin.y,
                here.origin.z
            ));
        }
        lines
    }

    /// Add this frame's toys to the scene, each as its show poses it (none while gone),
    /// with the ring under it while its show draws one, and its show's drops.
    pub(crate) fn append_toy_master_instances(&mut self) {
        let now = Instant::now();
        for here in &self.toy_master.here {
            let moment = here.moment(now).unwrap_or(Moment {
                scale: 1.0,
                ..Moment::default()
            });
            if let Some(mesh) = self.toy_master.mesh_of(&here.model)
                && moment.scale > 0.01
            {
                let scale = here.scale * moment.scale;
                self.object_groups[mesh].push(ActorInstance::new(
                    (here.origin + moment.offset).to_array(),
                    sjk_client::legacy_angles_to_quaternion([
                        moment.pitch,
                        here.yaw + moment.turn,
                        moment.roll,
                    ]),
                    [scale; 3],
                ));
            }
            let show = here.track.show();
            if let Some((grown, turn)) = moment.ring
                && show.ring_radius > 0.0
                && let Some(mesh) = self.toy_master.mesh_of(&show.ring)
            {
                let (full, _) = reach_of(show);
                let scale = full / show.ring_radius * grown;
                self.object_groups[mesh].push(ActorInstance::new(
                    here.origin.to_array(),
                    sjk_client::legacy_angles_to_quaternion([0.0, here.yaw + turn, 0.0]),
                    [scale; 3],
                ));
            }
            let Some(elapsed) = here.show_elapsed(now) else {
                continue;
            };
            for (drop, at, yaw) in here.drops.shown(elapsed) {
                let (model, scale) = drop_look(drop.kind);
                if let Some(mesh) = self.toy_master.mesh_of(model) {
                    self.object_groups[mesh].push(ActorInstance::new(
                        at.to_array(),
                        sjk_client::legacy_angles_to_quaternion([0.0, yaw, 0.0]),
                        [scale; 3],
                    ));
                }
            }
        }
    }

    /// Run the shows' drops and floating players on to `now` against the map; the
    /// players within a show's reach float off as its `float` step begins.
    fn run_toy_master_bodies(&mut self, now: Instant) {
        let busy = self.toy_master.here.iter().any(|here| {
            here.show_elapsed(now).is_some()
                && (!here.drops.list.is_empty() || here.playing.is_some())
        });
        if !busy {
            return;
        }
        // Who stands round each floating show as it floats off.
        let mut players: Vec<(u8, Vec3, f32, u16)> = Vec::new();
        if let Some(session) = self.live_session.as_ref() {
            let snapshot = session.latest_snapshot();
            let time = i64::from(snapshot.server_time);
            for entity in &snapshot.entities {
                if entity.entity_type() == ET_PLAYER
                    && entity.e_flags() & (EF_DEAD | EF_NODRAW) == 0
                    && let Ok(slot) = u8::try_from(entity.number())
                    && let Some(at) = self
                        .live_world
                        .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))
                {
                    players.push((
                        slot,
                        Vec3::from_array(at.sample(time).translation),
                        entity.angles()[1],
                        crate::ragdoll::scale_percent(entity.model_scale_percent()),
                    ));
                }
            }
            if let Ok(own) = u8::try_from(session.game_state().client_num)
                && snapshot.player.health() > 0
            {
                let origin = self
                    .local_prediction
                    .predicted_state()
                    .map_or_else(|| snapshot.player.origin(), |state| state.origin);
                players.push((
                    own,
                    Vec3::from_array(origin),
                    snapshot.player.view_angles()[1],
                    crate::ragdoll::scale_percent(snapshot.player.model_scale_percent()),
                ));
            }
        }
        let ToyMaster { here, movers, .. } = &mut self.toy_master;
        let mut world = crate::ragdoll::Scene::new(
            &self.bsp,
            &mut self.trace_scratch,
            self.live_session.as_ref(),
            movers,
        );
        for here in here.iter_mut() {
            let Some(elapsed) = here.show_elapsed(now) else {
                continue;
            };
            here.drops.run(elapsed, &mut world);
            let moment = here.moment(now);
            let floating = moment.is_some_and(|moment| moment.floating);
            if floating && here.floats.is_empty() {
                let (full, _) = reach_of(here.track.show());
                let near: Vec<_> = players
                    .iter()
                    .copied()
                    .filter(|(_, at, _, _)| at.distance(here.origin) <= full)
                    .collect();
                here.floats
                    .begin(&near, elapsed, here.seed ^ here.id as u32);
            }
            here.floats.run(elapsed, floating, &mut world);
        }
    }

    /// Play the toys' sounds and sparkles; their sounds are handed to the audio when the
    /// toys or the audio change.
    pub(crate) fn play_toy_master_sounds(&mut self, audio: &mut Option<crate::GameAudio>) {
        let Some(audio) = audio.as_mut() else {
            self.toy_master.cues.clear();
            self.toy_master.sparkles.clear();
            return;
        };
        let revision = self
            .toy_master
            .taken
            .as_ref()
            .map_or(0, |(revision, ..)| *revision);
        let data = self.vfs.as_ref().map_or(0, |vfs| Arc::as_ptr(vfs) as usize);
        if self.toy_master.sounds_registered != Some((audio.generation(), revision, data))
            && !self.toy_master.here.is_empty()
            && let Some(vfs) = self.vfs.as_deref()
        {
            let shows = self
                .toy_master
                .here
                .iter()
                .map(|here| here.sound.as_str())
                .filter(|sound| !sound.is_empty());
            audio.register_effect_paths(vfs, [VANISH_SOUND, POP_SOUND].into_iter().chain(shows));
            self.toy_master.sounds_registered = Some((audio.generation(), revision, data));
        }
        if let Some(vfs) = self.vfs.clone() {
            for at in self.toy_master.sparkles.drain(..) {
                crate::effect_runtime::spawn_effect(
                    &mut self.particles,
                    &mut self.effect_aux,
                    &mut self.effects,
                    &vfs,
                    EFFECT,
                    at + Vec3::Z * 8.0,
                    Instant::now(),
                    0,
                    0,
                    &mut None,
                    crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]),
                );
            }
        }
        for cue in self.toy_master.cues.drain(..) {
            let Some(path) = cue.path else {
                audio.stop_toy(cue.source, SHOW_CHANNEL);
                continue;
            };
            if cue.show {
                // Not every player has the show's sound: without it the toy spins silent.
                if !audio.has_sound(&path) {
                    crate::log::progress(format_args!(
                        "toy master: the show's sound {path} is not in the game data (an \
                         optional pack such as the Sith cloud's has it): the toy spins silent"
                    ));
                    continue;
                }
                audio.play_toy_reaching(
                    crate::toy_sounds::Toy::Master,
                    &path,
                    SHOW_VOLUME,
                    cue.origin,
                    cue.source,
                    SHOW_CHANNEL,
                    sjk_audio::Attenuation::Linear {
                        full_volume_distance: SOUND_FULL,
                        falloff_per_unit: 1.0 / (SOUND_RANGE - SOUND_FULL),
                    },
                    cue.from_ms,
                );
            } else {
                audio.play_toy(
                    crate::toy_sounds::Toy::Master,
                    &path,
                    1.0,
                    Some(cue.origin),
                    cue.source,
                    CUE_CHANNEL,
                );
            }
        }
    }
}

#[cfg(test)]
impl GpuState {
    /// For the world shots: toys of `model` at each of `toys` (origin and yaw), each
    /// `since` into its show when given (`show` playing), else standing still.
    pub(crate) fn shot_toys(
        &mut self,
        model: &str,
        show: Show,
        toys: &[([f32; 3], f32, Option<Duration>)],
    ) {
        let now = Instant::now();
        self.toy_master.shot = true;
        self.toy_master.here = toys
            .iter()
            .enumerate()
            .map(|(index, (origin, yaw, since))| Here {
                id: index as i64 + 1,
                origin: Vec3::from_array(*origin),
                yaw: *yaw,
                model: model.to_owned(),
                scale: 1.0,
                sound: String::new(),
                track: Track::new(&show),
                kind: None,
                escorted: false,
                playing: since.map(|since| (now - since, 2)),
                show_at: since.map(|since| now - since),
                seed: 0,
                drops: drops::Drops::default(),
                floats: float::Floats::default(),
                done: false,
                told: false,
            })
            .collect();
        self.toy_master.meshes = vec![(model.to_owned(), None)];
        self.toy_master.meshes_seen = usize::MAX;
    }

    /// For the world shots: the shot's toys of `model` playing `show` (its ring and
    /// drops' models wanted too), each stood on the floor under where it is put, the last
    /// one's show throwing `kinds` from `seed`, run against the map to where it is.
    /// Answers where the last one stands.
    pub(crate) fn shot_show(
        &mut self,
        model: &str,
        show: Show,
        toys: &[([f32; 3], f32, Option<Duration>)],
        seed: u32,
        kinds: &[wire::DropKind],
    ) -> Vec3 {
        self.shot_toys(model, show.clone(), toys);
        let secrets = wire::Secrets {
            secrets: vec![wire::Secret {
                id: 1,
                map: String::new(),
                checksum: 0,
                server: String::new(),
                origin: [0.0; 3],
                yaw: 0.0,
                model: model.to_owned(),
                scale: 1.0,
                sound: String::new(),
                show,
                kind: None,
                escorted: false,
            }],
            kinds: Vec::new(),
        };
        // The models a show wants, as `take` finds them.
        let mut wanted = ToyMaster::default();
        wanted.take(&secrets, "", 0, "", true);
        self.toy_master.meshes = wanted.meshes;
        let now = Instant::now();
        let mut movers = crate::ragdoll::Movers::default();
        let mut world =
            crate::ragdoll::Scene::new(&self.bsp, &mut self.trace_scratch, None, &mut movers);
        // Each toy stands on the floor under where the shot puts it.
        for here in &mut self.toy_master.here {
            use crate::ragdoll::World as _;
            if let Some(hit) = world.trace(here.origin, here.origin - Vec3::Z * 4_096.0, 0.0) {
                here.origin = hit.stop;
            }
        }
        let last = self.toy_master.here.len() - 1;
        let here = &mut self.toy_master.here[last];
        here.drops = drops::Drops::new(here.track.show(), seed, kinds, here.origin);
        let elapsed = here.show_elapsed(now).unwrap_or_default();
        here.drops.run(elapsed, &mut world);
        println!(
            "toy at {:?}; drops at rest {} of {}",
            here.origin,
            here.drops.list.iter().filter(|drop| drop.resting).count(),
            here.drops.list.len()
        );
        here.origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOW: Show = Show {
        vanish: 200,
        gone: 1_000,
        pop: 400,
        spin: 8_000,
        turns: 2.0,
        steps: Vec::new(),
        length: 0,
        reach: 0.0,
        ring: String::new(),
        ring_radius: 0.0,
        voice: Vec::new(),
    };

    /// A model near a spinning toy turns and squishes with its feet on the floor; one far
    /// off is left as it was, one between fades.
    #[test]
    fn a_dance_turns_and_squishes_the_model_feet_down() {
        let dances = [Dance {
            origin: Vec3::ZERO,
            turn: 90.0,
            squish: 1.0,
            full: DANCE_FULL,
            fade: DANCE_FADE,
        }];
        let start = sjk_runtime::Transform {
            translation: [100.0, 0.0, 24.0],
            rotation: glam::Quat::IDENTITY.to_array(),
            scale: [1.0; 3],
        };
        let mut near = start;
        dance_with(&mut near, 24.0, &dances);
        let turned = glam::Quat::from_array(near.rotation) * Vec3::X;
        assert!((turned - Vec3::Y).length() < 1e-4, "{turned}");
        assert!((near.scale[2] - (1.0 - SQUISH)).abs() < 1e-5);
        assert!(near.scale[0] > 1.0 && near.scale[0] == near.scale[1]);
        // The feet, 24 under the origin times the height, stay at 0.
        assert!((near.translation[2] - 24.0 * near.scale[2]).abs() < 1e-4);
        let mut far = start;
        far.translation[0] = DANCE_FADE + 1.0;
        dance_with(&mut far, 24.0, &dances);
        assert_eq!(far, start_at(&start, DANCE_FADE + 1.0));
        let mut between = start;
        between.translation[0] = (DANCE_FULL + DANCE_FADE) / 2.0;
        dance_with(&mut between, 24.0, &dances);
        let squished = 1.0 - between.scale[2];
        assert!(squished > 0.0 && squished < SQUISH, "{squished}");
    }

    fn start_at(start: &sjk_runtime::Transform, x: f32) -> sjk_runtime::Transform {
        let mut moved = *start;
        moved.translation[0] = x;
        moved
    }

    /// A show with a reach has the players dance within it; one without, as before.
    #[test]
    fn a_show_reaches_as_far_as_it_says() {
        assert_eq!(reach_of(&SHOW), (DANCE_FULL, DANCE_FADE));
        let show = Show {
            reach: 400.0,
            ..SHOW
        };
        assert_eq!(reach_of(&show), (400.0, 400.0 + DANCE_EDGE));
    }

    /// A toy shrunk away in its show is not shown, so no prompt stands over it.
    #[test]
    fn a_toy_gone_in_its_show_is_not_shown() {
        let mut toys = ToyMaster::default();
        toys.take(
            &wire::Secrets {
                secrets: vec![toy(1, [0.0; 3])],
                ..Default::default()
            },
            "mp/ffa3",
            -5,
            "",
            true,
        );
        let now = Instant::now();
        assert!(shown(&toys.here[0], now));
        toys.start(1, now);
        assert!(shown(&toys.here[0], now));
        assert!(!shown(&toys.here[0], now + Duration::from_millis(700)));
        assert!(shown(&toys.here[0], now + Duration::from_millis(2_000)));
    }

    fn toy(id: i64, origin: [f32; 3]) -> wire::Secret {
        wire::Secret {
            id,
            map: "mp/ffa3".to_owned(),
            checksum: -5,
            server: String::new(),
            origin,
            yaw: 0.0,
            model: "models/a.md3".to_owned(),
            scale: 1.0,
            sound: "sound/a.mp3".to_owned(),
            show: SHOW,
            kind: None,
            escorted: false,
        }
    }

    #[test]
    fn the_toys_of_the_map_are_taken_and_each_model_wanted_once() {
        let mut toys = ToyMaster::default();
        let mut other = toy(3, [0.0; 3]);
        other.map = "mp/ffa1".to_owned();
        let secrets = wire::Secrets {
            secrets: vec![toy(1, [0.0; 3]), toy(2, [100.0, 0.0, 0.0]), other],
            kinds: Vec::new(),
        };
        toys.take(&secrets, "mp/ffa3", -5, "1.2.3.4:29070", true);
        assert_eq!(toys.here.len(), 2);
        assert_eq!(toys.wanted_models().collect::<Vec<_>>(), ["models/a.md3"]);
        // A show played on this map load stays played when the list is read again, and
        // comes back with a new map load.
        toys.start(1, Instant::now());
        toys.take(&secrets, "mp/ffa3", -5, "1.2.3.4:29070", false);
        assert!(toys.here[0].done && toys.here[0].playing.is_some());
        toys.take(&secrets, "mp/ffa3", -5, "1.2.3.4:29070", true);
        assert!(!toys.here[0].done);
    }

    /// A show with a ring and drops wants their models too, each once.
    #[test]
    fn a_shows_ring_and_drops_are_wanted() {
        let mut toys = ToyMaster::default();
        let mut stepped = toy(1, [0.0; 3]);
        stepped.show = track::tests::stepped();
        toys.take(
            &wire::Secrets {
                secrets: vec![stepped.clone(), wire::Secret { id: 2, ..stepped }],
                kinds: Vec::new(),
            },
            "mp/ffa3",
            -5,
            "",
            true,
        );
        let wanted: Vec<_> = toys.wanted_models().collect();
        assert_eq!(wanted.len(), 5, "{wanted:?}");
        assert!(wanted.contains(&"models/r.md3") && wanted.contains(&crate::illuminate::MODEL));
    }

    /// A show's drops come with its show and go when taken; none before the show.
    #[test]
    fn drops_come_with_the_show_and_go_when_taken() {
        let mut toys = ToyMaster::default();
        let mut stepped = toy(1, [0.0; 3]);
        stepped.show = track::tests::stepped();
        toys.take(
            &wire::Secrets {
                secrets: vec![stepped],
                kinds: Vec::new(),
            },
            "mp/ffa3",
            -5,
            "",
            true,
        );
        let kinds = [wire::DropKind::Credits, wire::DropKind::Holocron];
        toys.throw(1, 5, &kinds);
        assert!(toys.here[0].drops.list.is_empty());
        let now = Instant::now();
        toys.start(1, now);
        toys.throw(1, 5, &kinds);
        assert_eq!(toys.here[0].drops.list.len(), 2);
        toys.taken(1, 1);
        assert!(toys.here[0].drops.list[1].taken && !toys.here[0].drops.list[0].taken);
        // The drops wait a minute past the show's end, then go.
        let length = Duration::from_millis(u64::from(toys.here[0].track.length()));
        assert!(toys.here[0].show_elapsed(now + length).is_some());
        assert!(
            toys.here[0]
                .show_elapsed(now + length + DROPS_KEPT)
                .is_none()
        );
    }

    #[test]
    fn presses_gather_and_go_twice_a_second() {
        let mut toys = ToyMaster::default();
        let now = Instant::now();
        assert_eq!(toys.due(now), None);
        for _ in 0..3 {
            toys.press(4);
        }
        assert_eq!(toys.due(now), Some((4, 3)));
        toys.press(4);
        assert_eq!(toys.due(now + Duration::from_millis(100)), None);
        // Presses on another toy replace them; never more than the hub takes.
        for _ in 0..30 {
            toys.press(5);
        }
        assert_eq!(toys.due(now + SEND_EVERY), Some((5, PRESSES_MAX)));
    }

    #[test]
    fn the_faced_toy_within_reach_is_pressed() {
        let mut toys = ToyMaster::default();
        let secrets = wire::Secrets {
            secrets: vec![toy(1, [60.0, 0.0, 0.0]), toy(2, [-60.0, 0.0, 0.0])],
            kinds: Vec::new(),
        };
        toys.take(&secrets, "mp/ffa3", -5, "1.2.3.4:29070", true);
        assert_eq!(pick(Vec3::ZERO, 0.0, &toys.here), Some(1));
        assert_eq!(pick(Vec3::ZERO, std::f32::consts::PI, &toys.here), Some(2));
        assert_eq!(pick(Vec3::new(0.0, 500.0, 0.0), 0.0, &toys.here), None);
    }

    #[test]
    fn a_show_plays_its_moments_once_then_stops_its_sound() {
        let mut toys = ToyMaster::default();
        let secrets = wire::Secrets {
            secrets: vec![toy(1, [0.0; 3])],
            kinds: Vec::new(),
        };
        toys.take(&secrets, "mp/ffa3", -5, "1.2.3.4:29070", true);
        let start = Instant::now();
        toys.start(1, start);
        toys.tick(start);
        toys.tick(start + Duration::from_millis(100));
        assert_eq!(toys.cues.len(), 1);
        toys.tick(start + Duration::from_millis(1_300));
        let paths: Vec<Option<&str>> = toys.cues.iter().map(|cue| cue.path.as_deref()).collect();
        assert_eq!(
            paths,
            [Some(VANISH_SOUND), Some(POP_SOUND), Some("sound/a.mp3")]
        );
        // Starting again while it plays changes nothing.
        toys.start(1, start + Duration::from_millis(1_400));
        toys.tick(start + Duration::from_millis(9_600));
        assert!(toys.here[0].playing.is_none() && toys.here[0].done);
        assert!(
            toys.cues
                .last()
                .is_some_and(|cue| cue.path.is_none() && cue.show)
        );
        assert_eq!(toys.sparkles.len(), 2);
    }

    /// A player who arrives after a show started joins it where it is: no vanish or pop
    /// once they are past, the sound from as far in, the drops thrown with the ones taken
    /// gone; a toy that played its show on this map load is not joined again.
    #[test]
    fn a_show_under_way_is_joined_where_it_is() {
        let mut toys = ToyMaster::default();
        let mut stepped = toy(1, [0.0; 3]);
        stepped.show = track::tests::stepped();
        let secrets = wire::Secrets {
            secrets: vec![stepped, toy(2, [500.0, 0.0, 0.0])],
            kinds: Vec::new(),
        };
        toys.take(&secrets, "mp/ffa3", -5, "", true);
        let under_way = wire::ShowUnderWay {
            id: 1,
            round: 7,
            ago: 40_000,
            seed: 5,
            drops: "cch".to_owned(),
            taken: 0b010,
        };
        let now = Instant::now();
        toys.join(&under_way, Duration::from_millis(40_000), now);
        let here = &toys.here[0];
        assert_eq!(here.playing.map(|(_, played)| played), Some(2));
        assert_eq!(here.show_elapsed(now).map(|ms| ms.round()), Some(40_000.0));
        assert_eq!(here.drops.list.len(), 3);
        assert!(here.drops.list[1].taken && !here.drops.list[0].taken);
        // Only the show's sound, from where it is: 40 s less the vanish and the gone.
        assert_eq!(toys.cues.len(), 1);
        assert_eq!(toys.cues[0].path.as_deref(), Some("sound/a.mp3"));
        assert_eq!(toys.cues[0].from_ms, 40_000 - 180 - 1_200);
        toys.tick(now);
        assert_eq!(toys.cues.len(), 1, "no vanish or pop once past them");
        // Told again (every presence answer): joined once only.
        toys.join(&under_way, Duration::from_millis(41_000), now);
        assert_eq!(toys.cues.len(), 1);
        // Just started: it plays its vanish and pop as anyone's.
        let early = wire::ShowUnderWay {
            id: 2,
            ago: 100,
            ..under_way.clone()
        };
        toys.join(&early, Duration::from_millis(100), now);
        assert_eq!(toys.here[1].playing.map(|(_, played)| played), Some(0));
        assert_eq!(toys.cues.len(), 1);
        // Long over, drops and all: nothing.
        let mut late = ToyMaster::default();
        late.take(&secrets, "mp/ffa3", -5, "", true);
        late.join(&under_way, Duration::from_secs(600), now);
        assert!(late.here[0].show_at.is_none() && late.cues.is_empty());
    }
}
