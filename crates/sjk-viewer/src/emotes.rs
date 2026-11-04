//! Emotes (`docs/hub-chat.md`, "Emotes in the client"): short animations an SJK
//! player plays, which other SJK players on the same game server see. They travel
//! through the SJK hub, not the game server, so stock servers and clients know
//! nothing of them.
//!
//! This is the groundwork: the catalogue's format and loader, the `sjkemote` command
//! (named apart from the `emote` some server mods have),
//! and [`ActiveEmotes`], which says which player plays which emote right now. The
//! animations, sounds and art are separate work: an emote's torso and legs
//! animations and sound are named in its catalogue file, and the player's model
//! code reads [`active`] to play them. Until then a received emote is written to the
//! console.
//!
//! The catalogue is data in the game's file system (pk3s or the folder):
//! `emotes/<id>.emote`, a JSON object:
//!
//! ```json
//! {"name": "Wave", "torso": "BOTH_...", "legs": "", "length_ms": 2500,
//!  "loop": false, "sound": "sound/emotes/wave.mp3", "clip": "", "local": false,
//!  "moving": false, "icon": "gfx/emotes/wave.png"}
//! ```
//!
//! `clip` names an animation of a rig model (`docs/rig-models.md`): a player wearing
//! a rig that has it plays it on the rig's own skeleton. `rig` names the model folder
//! the clip comes from: other rig models with the same joint names borrow it. `local` keeps the emote on
//! this client: it plays here and is never sent to the hub (an emote made for a
//! model only this player has). `moving` keeps it playing while the player moves.
//! The sound plays on the player and stops when the emote is cut short. With its
//! length (`sound_ms`) a looping emote plays it again each time it ends, and
//! `sound_loop` (its length `sound_loop_ms`) follows the first sound and repeats
//! for as long as the emote plays (music with an intro and a loop). `icon` is a
//! picture (PNG, JPG or TGA) the quick wheel shows for it (`emote_icons.rs`). The id
//! [`STOP`] ends the player's emote; `sjkemote stop all` or `sjkemote stop <player>`
//! ends other players' on this screen only, once (their next emote plays).
//! [`MUTE`] (`sjkemote mute <player>`) hides a player's emotes on this screen until
//! [`UNMUTE`] or the game closes; it holds their identity key, so it follows them
//! across slots and names. `<player>` is a slot number, part of a name, or
//! [`TARGET`], the player under the crosshair.
//!
//! `<id>` is the emote's id at the hub: 1 to 32 of `a` to `z`, `0` to `9` and `_`.
//!
//! [`SYNC`] (`sjkemote sync [player]`) joins another player's emote in step: the hub
//! relays the leader's slot with it, and each client starts the follower at the
//! leader's start as that client saw it ([`ActiveEmotes::sync`]), so the two dance in
//! phase on every screen whatever each one's delay. The follower's music starts
//! where the leader's is, so both play it in step.

use sjk_identity::Emote;
use std::time::{Duration, Instant};

/// Where the catalogue's files are.
pub(crate) const FOLDER: &str = "emotes";
/// The `emote` command.
pub(crate) const COMMAND: &str = "sjkemote";
/// The id that ends the player's emote (`sjkemote stop`), never a catalogue entry's.
pub(crate) const STOP: &str = "stop";
/// The word that joins another player's emote in step (`sjkemote sync [player]`),
/// never a catalogue entry's.
pub(crate) const SYNC: &str = "sync";
/// The id that silences the player's emote for the others while they are away
/// (`away_emote.rs`), never a catalogue entry's.
pub(crate) const QUIET: &str = "quiet";
/// The id that gives that emote its sound again, never a catalogue entry's.
pub(crate) const LOUD: &str = "loud";
/// `sjkemote mute <player>`: hide that player's emotes on this screen.
pub(crate) const MUTE: &str = "mute";
/// `sjkemote unmute <player|all>`: show them again.
pub(crate) const UNMUTE: &str = "unmute";
/// `stop all`, `unmute all`: every player.
pub(crate) const ALL: &str = "all";
/// `stop target`, `mute target`: the player under the crosshair.
pub(crate) const TARGET: &str = "target";
/// How far (game units) `sjkemote sync` alone looks for a player to join.
pub(crate) const SYNC_RANGE: f32 = 512.0;
pub(crate) const HELP: &str = "Play an emote the SJK players on this server see (sjkemote alone lists them; sjkemote sync [player] joins one in step; stop [all|player|target] ends emotes; mute/unmute <player|target> hides a player's; help_emotes explains them)";
/// How loud emote sounds play, 1 as recorded (Settings > Sound > Emotes). Apart from
/// the effects volume since 27/10/2026: times `s_volume` it stayed quiet at its top
/// for a player who lowered the game's sounds (Rollembones's report #163).
pub(crate) const VOLUME_CVAR: &str = "s_emoteVolume";
/// [`VOLUME_CVAR`]'s default: as loud as the old default 2 at the default `s_volume`
/// 0.5.
pub(crate) const DEFAULT_VOLUME: f64 = 1.0;
/// [`VOLUME_CVAR`]'s top, the Settings slider's too.
pub(crate) const MAX_VOLUME: f64 = 3.0;
/// Archived marker of the one-time move of a saved [`VOLUME_CVAR`] to its own scale
/// (`console_session.rs`).
pub(crate) const VOLUME_VERSION_CVAR: &str = "s_emoteVolumeVersion";
/// Whether emote sounds are silent while the viewed player is in a duel (a private
/// duel, or a Duel or Power Duel server: [`crate::duel_blades::in_duel`]; Settings >
/// Sound > Emotes, Groark's idea, hub report #234). The emote channel's gain goes to 0
/// ([`heard_volume`]), so the emotes already playing fall silent too, and one still
/// playing when the duel ends is heard again. Off by default (stock JKA has no emotes
/// to silence).
pub(crate) const DUEL_MUTE_CVAR: &str = "s_muteEmotesInDuel";
/// [`DUEL_MUTE_CVAR`]'s default.
pub(crate) const DEFAULT_DUEL_MUTE: bool = false;
/// Whether an attack or alt attack ends the player's own emote (Settings > Sound >
/// Emotes, [`ActiveEmotes::end_on_attack`]). Off by default: the dance goes on
/// through the attacks, as before 19/10/2026.
pub(crate) const ATTACK_STOP_CVAR: &str = "cg_emoteAttackStop";
/// [`ATTACK_STOP_CVAR`]'s default.
pub(crate) const DEFAULT_ATTACK_STOP: bool = false;
/// Whether a player typing (the chat, the console or a menu open: their talk balloon,
/// `EF_TALK`) keeps their emote whatever happens meanwhile: it ends when they move or
/// attack after closing it ([`ActiveEmotes::hold`]; Settings > Sound > Emotes).
pub(crate) const TYPING_KEEP_CVAR: &str = "cg_emoteTypingKeep";
/// [`TYPING_KEEP_CVAR`]'s default (since 23/10/2026, Rollembones' idea).
pub(crate) const DEFAULT_TYPING_KEEP: bool = true;
/// How long an emote the catalogue does not know lasts.
const UNKNOWN_LENGTH: Duration = Duration::from_secs(3);
/// How far a player may move (game units) before their emote stops.
const MOVE_LIMIT: f32 = 24.0;
/// How long a player may be missing from the snapshots before their emote ends.
const ABSENT_LIMIT: Duration = Duration::from_secs(2);
/// Slots the game numbers.
const SLOTS: usize = 64;
/// `BUTTON_ATTACK | BUTTON_ALT_ATTACK` (`codemp/qcommon/q_shared.h`): a command with
/// either ends the player's own emote ([`ActiveEmotes::end_on_attack`]).
const ATTACK_BUTTONS: u16 = 1 | 128;

/// The emote channel's gain: `volume` ([`VOLUME_CVAR`], kept within 0 to
/// [`MAX_VOLUME`]), or 0 while `in_duel` and [`DUEL_MUTE_CVAR`] (`mute_in_duel`) is on.
pub(crate) fn heard_volume(volume: f64, mute_in_duel: bool, in_duel: bool) -> f32 {
    if mute_in_duel && in_duel {
        0.0
    } else {
        volume.clamp(0.0, MAX_VOLUME) as f32
    }
}

/// One emote of the catalogue.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EmoteDef {
    /// Its id at the hub, the file's name.
    pub(crate) id: String,
    /// What it is called.
    pub(crate) name: String,
    /// The torso animation (`BOTH_...` or `TORSO_...`), or empty.
    pub(crate) torso: String,
    /// The legs animation, or empty.
    pub(crate) legs: String,
    /// How long it plays.
    pub(crate) length: Duration,
    /// Whether it loops until the player moves.
    pub(crate) looping: bool,
    /// The sound it plays, or empty.
    pub(crate) sound: String,
    /// The rig model animation it plays, or empty.
    pub(crate) clip: String,
    /// Played on this client only, never sent to the hub.
    pub(crate) local: bool,
    /// Keeps playing while the player moves.
    pub(crate) moving: bool,
    /// Its picture, or empty.
    pub(crate) icon: String,
    /// How long `sound` lasts (zero: unknown, played once).
    pub(crate) sound_length: Duration,
    /// The sound that follows `sound` and repeats while the emote plays, or empty.
    pub(crate) sound_loop: String,
    /// How long `sound_loop` lasts.
    pub(crate) sound_loop_length: Duration,
    /// The rig model folder (under `models/players`) its clip comes from, or empty.
    pub(crate) rig: String,
}

/// Whether `id` may name an emote at the hub.
pub(crate) fn valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// The fields a catalogue file may hold.
const FIELDS: [&str; 14] = [
    "name",
    "torso",
    "legs",
    "length_ms",
    "loop",
    "sound",
    "clip",
    "local",
    "moving",
    "icon",
    "rig",
    "sound_ms",
    "sound_loop",
    "sound_loop_ms",
];
/// The longest animation or sound name.
const PATH_MAX: usize = 64;
/// The shortest and longest emote that is not a loop, in milliseconds.
const LENGTH_MS: std::ops::RangeInclusive<u64> = 100..=60_000;

/// Read one catalogue file: `id` from its name, `text` its contents.
pub(crate) fn parse(id: &str, text: &str) -> Result<EmoteDef, String> {
    if !valid_id(id) {
        return Err(format!(
            "{id:?} is not an emote id (1 to 32 of a to z, 0 to 9 and _)"
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("not JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "not a JSON object".to_owned())?;
    if let Some(unknown) = object.keys().find(|key| !FIELDS.contains(&key.as_str())) {
        return Err(format!("unknown field {unknown:?}"));
    }
    let text_field = |field: &str, max: usize| -> Result<String, String> {
        match object.get(field) {
            None => Ok(String::new()),
            Some(serde_json::Value::String(text))
                if text.chars().count() <= max && !text.chars().any(char::is_control) =>
            {
                Ok(text.clone())
            }
            Some(_) => Err(format!("{field} must be text of at most {max} characters")),
        }
    };
    let name = text_field("name", 32)?;
    if name.trim().is_empty() {
        return Err("name is needed".to_owned());
    }
    let flag = |field: &str| match object.get(field) {
        None => Ok(false),
        Some(serde_json::Value::Bool(value)) => Ok(*value),
        Some(_) => Err(format!("{field} must be true or false")),
    };
    let looping = flag("loop")?;
    let milliseconds = |field: &str| match object.get(field) {
        None => Ok(Duration::ZERO),
        Some(value) => value
            .as_u64()
            .filter(|ms| *ms <= 600_000)
            .map(Duration::from_millis)
            .ok_or_else(|| format!("{field} must be 0 to 600000")),
    };
    let length = match object.get("length_ms") {
        None if looping => 0,
        Some(value) => value
            .as_u64()
            .filter(|ms| LENGTH_MS.contains(ms))
            .ok_or_else(|| "length_ms must be 100 to 60000".to_owned())?,
        None => return Err("length_ms is needed unless the emote loops".to_owned()),
    };
    Ok(EmoteDef {
        id: id.to_owned(),
        name,
        torso: text_field("torso", PATH_MAX)?,
        legs: text_field("legs", PATH_MAX)?,
        length: Duration::from_millis(length),
        looping,
        sound: text_field("sound", PATH_MAX)?,
        clip: text_field("clip", PATH_MAX)?,
        local: flag("local")?,
        moving: flag("moving")?,
        icon: text_field("icon", PATH_MAX)?,
        sound_length: milliseconds("sound_ms")?,
        sound_loop: text_field("sound_loop", PATH_MAX)?,
        sound_loop_length: milliseconds("sound_loop_ms")?,
        rig: {
            let rig = text_field("rig", PATH_MAX)?;
            if rig.contains(['/', '\\']) || rig.contains("..") {
                return Err("rig must be a model folder's name".to_owned());
            }
            rig
        },
    })
}

/// The catalogue in `vfs`: every `emotes/<id>.emote` that reads, by id; the ones that
/// do not are reported to the log by name.
pub(crate) fn load(vfs: &sjk_vfs::VirtualFileSystem) -> Vec<EmoteDef> {
    let mut catalogue = Vec::new();
    for listed in vfs.list_files(FOLDER, ".emote") {
        let path = format!("{FOLDER}/{listed}");
        let id = listed.trim_end_matches(".emote");
        let read = vfs
            .read(&path)
            .map_err(|error| error.to_string())
            .and_then(|asset| asset.ok_or_else(|| "missing".to_owned()))
            .and_then(|asset| String::from_utf8(asset.bytes).map_err(|_| "not UTF-8".to_owned()))
            .and_then(|text| parse(id, &text));
        match read {
            Ok(def) => catalogue.push(def),
            Err(why) => crate::log::progress(format_args!("emotes: {path}: {why}")),
        }
    }
    catalogue.retain(|def| {
        let reserved = [
            STOP,
            SYNC,
            QUIET,
            LOUD,
            MUTE,
            UNMUTE,
            crate::emote_help::WORD,
            crate::away_emote::NONE,
        ]
        .contains(&def.id.as_str());
        if reserved {
            crate::log::progress(format_args!(
                "emotes: {FOLDER}/{}.emote: the id is reserved",
                def.id
            ));
        }
        !reserved
    });
    catalogue.sort_by(|a, b| a.id.cmp(&b.id));
    catalogue.dedup_by(|a, b| a.id == b.id);
    catalogue
}

/// One emote being played.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ActiveEmote {
    /// The emote's id.
    pub(crate) emote: String,
    pub(crate) started: Instant,
    /// When it ends, unless the player moves first; `None` while a loop plays.
    pub(crate) until: Option<Instant>,
    /// Where the player stood when it started: the first position seen, as a player
    /// out of the snapshot then has none yet.
    pub(crate) origin: Option<[f32; 3]>,
    /// Since when the player is missing from the snapshots, while they are.
    pub(crate) missing_since: Option<Instant>,
    /// Its sound, or empty.
    pub(crate) sound: String,
    /// Whether it keeps playing while the player moves.
    pub(crate) moving: bool,
    /// The sound to start next while it plays (its loop, or its sound again), when,
    /// and how long that one lasts.
    pub(crate) next_sound: Option<(Instant, String, Duration)>,
    /// The player it follows in step (`sjkemote sync`): one joining it follows them.
    pub(crate) leader: Option<u8>,
    /// Silenced while its player is away ([`ActiveEmotes::set_quiet`]): its music
    /// keeps time without playing.
    pub(crate) quiet: bool,
    /// Its player is typing ([`ActiveEmotes::hold`]): moving or attacking does not
    /// end it until they stop.
    pub(crate) held: bool,
}

/// An emote's music: its first sound, then the one that repeats after it (its loop,
/// or for a looping emote the first again), each with its length.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Music {
    first: String,
    first_length: Duration,
    then: String,
    then_length: Duration,
}

impl Music {
    fn of(def: &EmoteDef) -> Self {
        let (then, then_length) = if def.sound.is_empty() || def.sound_length.is_zero() {
            (String::new(), Duration::ZERO)
        } else if !def.sound_loop.is_empty() {
            (def.sound_loop.clone(), def.sound_loop_length)
        } else if def.looping {
            (def.sound.clone(), def.sound_length)
        } else {
            (String::new(), Duration::ZERO)
        };
        Self {
            first: def.sound.clone(),
            first_length: def.sound_length,
            then,
            then_length,
        }
    }

    /// The sound playing `elapsed` after the start, how far into it, and how long
    /// before `then` starts (again); `None` once it has all played.
    fn at(&self, elapsed: Duration) -> Option<(&str, Duration, Option<Duration>)> {
        if self.first.is_empty() {
            return None;
        }
        // A first sound of unknown length: the mixer plays nothing past its end.
        if self.first_length.is_zero() || elapsed < self.first_length {
            let next = (!self.then.is_empty() && !self.first_length.is_zero())
                .then(|| self.first_length - elapsed);
            return Some((&self.first, elapsed, next));
        }
        if self.then.is_empty() {
            return None;
        }
        let past = elapsed - self.first_length;
        if self.then_length.is_zero() {
            return Some((&self.then, past, None));
        }
        let into = Duration::from_nanos(
            u64::try_from(past.as_nanos() % self.then_length.as_nanos()).unwrap_or(0),
        );
        Some((&self.then, into, Some(self.then_length - into)))
    }
}

/// A sound to start on, or stop from, a player: what the audio does with emotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Cue {
    /// Start `sound` `offset` into it (a synced emote's music joining the leader's),
    /// at the sample `align`'s plays it (the leader's, for a follower; another of its
    /// group's, for music back from away: [`ActiveEmotes::set_quiet`]).
    Start {
        slot: u8,
        sound: String,
        offset: Duration,
        align: Option<u8>,
    },
    Stop {
        slot: u8,
    },
}

/// Which player plays which emote, by slot.
pub(crate) struct ActiveEmotes {
    slots: [Option<ActiveEmote>; SLOTS],
    /// Sounds to start and stop, taken by the audio each frame.
    cues: Vec<Cue>,
}

impl Default for ActiveEmotes {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            cues: Vec::new(),
        }
    }
}

impl ActiveEmotes {
    /// The emote `slot` plays, if any.
    pub(crate) fn get(&self, slot: u8) -> Option<&ActiveEmote> {
        self.slots.get(usize::from(slot))?.as_ref()
    }

    /// `slot` starts `emote` at `now`, standing at `origin`; `def` is its catalogue
    /// entry if this client has it.
    pub(crate) fn start(
        &mut self,
        slot: u8,
        emote: &str,
        def: Option<&EmoteDef>,
        origin: Option<[f32; 3]>,
        now: Instant,
    ) {
        let Some(entry) = self.slots.get_mut(usize::from(slot)) else {
            return;
        };
        let until = match def {
            Some(def) if def.looping => None,
            Some(def) => Some(now + def.length),
            None => Some(now + UNKNOWN_LENGTH),
        };
        if entry.as_ref().is_some_and(|old| !old.sound.is_empty()) {
            self.cues.push(Cue::Stop { slot });
        }
        let music = def.map(Music::of).unwrap_or_default();
        let sound = music.first.clone();
        if !sound.is_empty() {
            self.cues.push(Cue::Start {
                slot,
                sound: sound.clone(),
                offset: Duration::ZERO,
                align: None,
            });
        }
        // After the first sound: its loop, or for a looping emote the sound again.
        let next_sound = music
            .at(Duration::ZERO)
            .and_then(|(_, _, next)| next)
            .map(|next| (now + next, music.then.clone(), music.then_length));
        *entry = Some(ActiveEmote {
            emote: emote.to_owned(),
            started: now,
            until,
            origin,
            missing_since: None,
            sound,
            moving: def.is_some_and(|def| def.moving),
            next_sound,
            leader: None,
            quiet: false,
            held: false,
        });
    }

    /// `slot` joins `leader`'s `emote` in step at `now`, standing at `origin`: it
    /// starts when the leader's started here (the player the leader follows, when the
    /// leader follows someone), and its music from where the leader's is. False,
    /// changing nothing, when the leader is not playing `emote`: the caller starts it
    /// as usual then.
    pub(crate) fn sync(
        &mut self,
        slot: u8,
        leader: u8,
        emote: &str,
        def: Option<&EmoteDef>,
        origin: Option<[f32; 3]>,
        now: Instant,
    ) -> bool {
        let Some(lead) = self.get(leader).filter(|lead| lead.emote == emote) else {
            return false;
        };
        let (started, lead_until) = (lead.started, lead.until);
        let leader = match lead.leader {
            Some(root) if self.follows(leader, root) => root,
            _ => leader,
        };
        if leader == slot || usize::from(slot) >= SLOTS {
            return false;
        }
        let until = match def {
            Some(def) if def.looping => None,
            Some(def) => Some(started + def.length),
            None => lead_until,
        };
        self.stop(slot);
        let music = def.map(Music::of).unwrap_or_default();
        let mut next_sound = None;
        if let Some((sound, offset, next)) = music.at(now.saturating_duration_since(started)) {
            self.cues.push(Cue::Start {
                slot,
                sound: sound.to_owned(),
                offset,
                align: Some(leader),
            });
            next_sound = next.map(|next| (now + next, music.then.clone(), music.then_length));
        }
        self.slots[usize::from(slot)] = Some(ActiveEmote {
            emote: emote.to_owned(),
            started,
            until,
            origin,
            missing_since: None,
            sound: music.first.clone(),
            moving: def.is_some_and(|def| def.moving),
            next_sound,
            leader: Some(leader),
            quiet: false,
            held: false,
        });
        true
    }

    /// Whether `slot` plays the emote `leader` plays, from the same start.
    fn follows(&self, slot: u8, leader: u8) -> bool {
        match (self.get(slot), self.get(leader)) {
            (Some(follower), Some(lead)) => {
                follower.emote == lead.emote && follower.started == lead.started
            }
            _ => false,
        }
    }

    /// The players emoting now, by slot.
    pub(crate) fn playing(&self) -> impl Iterator<Item = (u8, &ActiveEmote)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(slot, entry)| Some((u8::try_from(slot).ok()?, entry.as_ref()?)))
    }

    /// End `slot`'s emote now, and its sound.
    pub(crate) fn stop(&mut self, slot: u8) {
        if let Some(old) = self.slots.get_mut(usize::from(slot)).and_then(Option::take)
            && !old.sound.is_empty()
        {
            self.cues.push(Cue::Stop { slot });
        }
    }

    /// Silence `slot`'s emote (`quiet`), or give it its sound again from where its
    /// music is at `now`; `def` is its catalogue entry. False when `slot` plays nothing
    /// or the emote already is so.
    pub(crate) fn set_quiet(
        &mut self,
        slot: u8,
        quiet: bool,
        def: Option<&EmoteDef>,
        now: Instant,
    ) -> bool {
        let align = self.in_step_with(slot);
        let Some(active) = self
            .slots
            .get_mut(usize::from(slot))
            .and_then(Option::as_mut)
        else {
            return false;
        };
        if active.quiet == quiet {
            return false;
        }
        active.quiet = quiet;
        if active.sound.is_empty() {
            return true;
        }
        if quiet {
            // A sound started this frame (a quiet emote just begun) never plays.
            self.cues
                .retain(|cue| !matches!(cue, Cue::Start { slot: at, .. } if *at == slot));
            self.cues.push(Cue::Stop { slot });
            return true;
        }
        let music = def.map(Music::of).unwrap_or_default();
        if let Some((sound, offset, next)) = music.at(now.saturating_duration_since(active.started))
        {
            self.cues.push(Cue::Start {
                slot,
                sound: sound.to_owned(),
                offset,
                align,
            });
            active.next_sound =
                next.map(|next| (now + next, music.then.clone(), music.then_length));
        }
        true
    }

    /// Another player of `slot`'s synced group (its leader, else one following the
    /// same leader) whose music plays, for `slot`'s music coming back to start at the
    /// sample theirs plays. A leader back from away has no leader of its own, and a
    /// follower's leader may be away too: the clock alone would bring it back a few
    /// milliseconds off the others, two copies heard apart rather than one song (hub
    /// report #195). `None` when it is in no group or no one else in it plays.
    fn in_step_with(&self, slot: u8) -> Option<u8> {
        let active = self.get(slot)?;
        let root = active.leader.unwrap_or(slot);
        let audible = |other: u8| {
            other != slot
                && self.follows(slot, other)
                && self
                    .get(other)
                    .is_some_and(|it| !it.quiet && !it.sound.is_empty())
        };
        if audible(root) {
            return Some(root);
        }
        self.playing()
            .find(|(other, it)| it.leader == Some(root) && audible(*other))
            .map(|(other, _)| other)
    }

    /// The player's own command carried `buttons`: with `enabled`
    /// ([`ATTACK_STOP_CVAR`]), an attack or alt attack ends their emote in `slot`, as
    /// the server attacks whatever the emote shows. True when one ended, so the
    /// caller tells the others (as `sjkemote stop`).
    pub(crate) fn end_on_attack(&mut self, slot: u8, buttons: u16, enabled: bool) -> bool {
        if !enabled
            || buttons & ATTACK_BUTTONS == 0
            || self.get(slot).is_none_or(|active| active.held)
        {
            return false;
        }
        self.stop(slot);
        true
    }

    /// End the emotes of the players riding a vehicle (bit `slot` of `riding`,
    /// [`riding_slots`]): the vehicle plays the rider's animations, and an emote kept
    /// on (one that plays while moving, as `cloud_float`) fought them for the whole
    /// ride (hub report #247). True when `own`'s ended, so the caller tells the
    /// others (as `sjkemote stop`).
    pub(crate) fn end_riding(&mut self, riding: u64, own: Option<u8>) -> bool {
        let mut own_ended = false;
        for slot in 0..SLOTS {
            let Ok(slot) = u8::try_from(slot) else {
                continue;
            };
            if riding & (1 << slot) != 0 && self.get(slot).is_some() {
                self.stop(slot);
                own_ended |= own == Some(slot);
            }
        }
        own_ended
    }

    /// Which players are typing now (bit `slot` of `typing`; zero with
    /// [`TYPING_KEEP_CVAR`] off): while one is, their emote does not end when they move
    /// or attack. Once they stop, the place where they are then is the one they may
    /// not leave, so being pushed while typing does not end it either; the next move
    /// or attack does. Call it each frame before [`Self::update`].
    pub(crate) fn hold(&mut self, typing: u64) {
        for (slot, entry) in self.slots.iter_mut().enumerate() {
            let Some(active) = entry else { continue };
            let held = typing & (1 << slot) != 0;
            if active.held && !held {
                // Taken again by `update`, where the player is now.
                active.origin = None;
            }
            active.held = held;
        }
    }

    /// The sounds to start and stop since the last call.
    pub(crate) fn take_cues(&mut self) -> Vec<Cue> {
        std::mem::take(&mut self.cues)
    }

    /// End the emotes that are over at `now`, or whose player moved away (unless the
    /// emote keeps playing while moving) or left (`position` says where each slot's
    /// player is; `None` for one that left). An emote cut short stops its sound; one
    /// that ran its length lets the sound finish.
    pub(crate) fn update(&mut self, now: Instant, position: impl Fn(u8) -> Option<[f32; 3]>) {
        for (slot, entry) in self.slots.iter_mut().enumerate() {
            let Some(active) = entry else { continue };
            let Ok(slot) = u8::try_from(slot) else {
                continue;
            };
            if let Some((at, sound, length)) = &mut active.next_sound
                && now >= *at
            {
                if !active.quiet {
                    self.cues.push(Cue::Start {
                        slot,
                        sound: sound.clone(),
                        offset: Duration::ZERO,
                        align: active.leader,
                    });
                }
                if length.is_zero() {
                    active.next_sound = None;
                } else {
                    while *at <= now {
                        *at += *length;
                    }
                }
            }
            let over = active.until.is_some_and(|until| now >= until);
            let gone = match position(slot) {
                // Out of the snapshot for a moment (a crowded dance, the PVS) is not
                // gone: only a player missing for [`ABSENT_LIMIT`] (hub report #107).
                None => {
                    let since = *active.missing_since.get_or_insert(now);
                    now.saturating_duration_since(since) >= ABSENT_LIMIT
                }
                Some(at) => {
                    active.missing_since = None;
                    match active.origin {
                        _ if active.moving || active.held => false,
                        None => {
                            active.origin = Some(at);
                            false
                        }
                        Some(origin) => {
                            let d: f32 =
                                at.iter().zip(origin).map(|(a, b)| (a - b) * (a - b)).sum();
                            d > MOVE_LIMIT * MOVE_LIMIT
                        }
                    }
                }
            };
            if gone && !active.sound.is_empty() && !active.quiet {
                self.cues.push(Cue::Stop { slot });
            }
            if over || gone {
                *entry = None;
            }
        }
    }
}

/// The players typing now, for [`ActiveEmotes::hold`]: bit `slot` set for the own
/// player (`own`: their slot and whether a chat, console or menu of theirs is open)
/// and for every other player whose entity (`others`: number and `eFlags`) wears the
/// talk balloon (`EF_TALK`, which their client's `BUTTON_TALK` sets on the server, so
/// every SJK client sees the same typists without the hub). None while `enabled`
/// ([`TYPING_KEEP_CVAR`]) is off.
pub(crate) fn typing_slots(
    enabled: bool,
    own: Option<(u8, bool)>,
    others: impl IntoIterator<Item = (u16, u32)>,
) -> u64 {
    if !enabled {
        return 0;
    }
    let mut typing = match own {
        Some((slot, true)) if usize::from(slot) < SLOTS => 1u64 << slot,
        _ => 0,
    };
    for (number, flags) in others {
        if usize::from(number) < SLOTS
            && flags & sjk_game_jka::pmove_talk::EF_TALK != 0
            && own.is_none_or(|(slot, _)| u16::from(slot) != number)
        {
            typing |= 1 << number;
        }
    }
    typing
}

/// The players riding a vehicle now, for [`ActiveEmotes::end_riding`]: bit `slot` set
/// for the own player (`own`: their slot and their `playerState_t::m_iVehicleNum`)
/// and for every other player whose entity (`others`: number and
/// `entityState_t::m_iVehicleNum`) names a vehicle. Only player slots count: a
/// vehicle's own entity names its pilot there.
pub(crate) fn riding_slots(
    own: Option<(u8, u16)>,
    others: impl IntoIterator<Item = (u16, u16)>,
) -> u64 {
    let mut riding = match own {
        Some((slot, vehicle)) if usize::from(slot) < SLOTS && vehicle != 0 => 1u64 << slot,
        _ => 0,
    };
    for (number, vehicle) in others {
        if usize::from(number) < SLOTS
            && vehicle != 0
            && own.is_none_or(|(slot, _)| u16::from(slot) != number)
        {
            riding |= 1 << number;
        }
    }
    riding
}

/// The catalogue (once a game's files are mounted) and the emotes playing now.
#[derive(Default)]
pub(crate) struct State {
    pub(crate) catalogue: Option<Vec<EmoteDef>>,
    pub(crate) active: ActiveEmotes,
    /// The audio output ([`crate::GameAudio::generation`]) the catalogue's sounds were
    /// handed to, to load again on a new one (`snd_restart`, hub report #99).
    pub(crate) sounds_loaded: Option<u64>,
    /// The players whose emotes this screen hides (`sjkemote mute`): key id to the
    /// name shown when muted.
    pub(crate) muted: std::collections::BTreeMap<String, String>,
}

static STATE: std::sync::LazyLock<std::sync::Mutex<State>> =
    std::sync::LazyLock::new(std::sync::Mutex::default);

pub(crate) fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The emote the player in `slot` plays now, for the player's model code.
// The seam the stock animations and sounds plug into; rig clips use `playing`.
#[allow(dead_code)]
pub(crate) fn active(slot: u8) -> Option<ActiveEmote> {
    lock().active.get(slot).cloned()
}

/// A rig clip the player in `slot` plays at `now`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Playing {
    pub(crate) clip: String,
    /// Seconds since it started.
    pub(crate) seconds: f32,
    /// Seconds before it ends, unless the player moves first; `None` for a loop.
    pub(crate) remaining: Option<f32>,
    pub(crate) looping: bool,
}

/// The rig clip the player in `slot` plays at `now`, if their emote names one.
pub(crate) fn playing(slot: u8, now: Instant) -> Option<Playing> {
    let state = lock();
    let active = state.active.get(slot)?;
    let def = state
        .catalogue
        .as_ref()?
        .iter()
        .find(|def| def.id == active.emote)?;
    (!def.clip.is_empty()).then(|| Playing {
        clip: def.clip.clone(),
        seconds: now.saturating_duration_since(active.started).as_secs_f32(),
        remaining: active
            .until
            .map(|until| until.saturating_duration_since(now).as_secs_f32()),
        looping: def.looping,
    })
}

/// What `sjkemote`'s first argument completes to on Tab in the console: the
/// installed emotes' ids, loading the catalogue when nothing has yet, and the
/// command's own words. [`QUIET`] and [`LOUD`] are the client's, never typed.
pub(crate) fn argument_words(vfs: Option<&sjk_vfs::VirtualFileSystem>) -> Vec<String> {
    let mut state = lock();
    if state.catalogue.is_none()
        && let Some(vfs) = vfs
    {
        state.catalogue = Some(load(vfs));
    }
    words(state.catalogue.as_deref().unwrap_or_default())
}

/// [`argument_words`] of `catalogue`.
fn words(catalogue: &[EmoteDef]) -> Vec<String> {
    catalogue
        .iter()
        .map(|def| def.id.clone())
        .chain([STOP, SYNC, MUTE, UNMUTE, crate::emote_help::WORD].map(str::to_owned))
        .collect()
}

/// What `emote` alone prints: the catalogue, or how emotes come.
pub(crate) fn listing(catalogue: &[EmoteDef]) -> Vec<String> {
    if catalogue.is_empty() {
        return vec![
            "No emotes are installed yet (emotes/<id>.emote in the game's files).".to_owned(),
            "sjkemote <id> still sends any id to the SJK players here.".to_owned(),
            format!(
                "{} explains emotes and the quick wheel.",
                crate::emote_help::COMMAND
            ),
        ];
    }
    catalogue
        .iter()
        .map(|def| {
            let length = if def.looping {
                "loops".to_owned()
            } else {
                format!("{:.1} s", def.length.as_secs_f32())
            };
            format!("{} - {}, {length}", def.id, def.name)
        })
        .chain([
            format!("{COMMAND} {STOP} ends yours; {STOP} {ALL} or {STOP} <player> ends others' here, once."),
            format!("{COMMAND} {SYNC} [player] joins another player's emote in step."),
            format!("{COMMAND} {MUTE} <player>, {UNMUTE} <player|{ALL}>: hide a player's emotes here ({MUTE} alone lists)."),
            format!("{} opens a window to search them and click one to play.", crate::emote_picker::COMMAND),
            format!("{} explains emotes and the quick wheel.", crate::emote_help::COMMAND),
        ])
        .collect()
}

/// `name` without colour codes, lower case: what `sjkemote sync <player>` matches.
pub(crate) fn plain_name(name: &str) -> String {
    plain_name_shown(name).to_lowercase()
}

/// `name` without colour codes, for the console.
pub(crate) fn plain_name_shown(name: &str) -> String {
    let mut chars = name.chars().peekable();
    let mut plain = String::new();
    while let Some(c) = chars.next() {
        if c == '^' && chars.peek().is_some_and(char::is_ascii_alphanumeric) {
            chars.next();
        } else {
            plain.push(c);
        }
    }
    plain
}

/// Whether to believe `emote`: the game shows `shown` in its slot, and the claim the
/// hub took the emote's slot from was made under that name (the badges' rule).
pub(crate) fn trusted(emote: &Emote, shown: Option<&str>) -> bool {
    shown.is_some_and(|shown| sjk_identity::names_match(&emote.claimed_name, shown))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hub report #234 (Groark): `s_muteEmotesInDuel` silences the emote channel in a
    /// duel only, and only when on; otherwise the emote volume, kept within its range.
    #[test]
    fn emotes_are_silent_in_a_duel_only_with_the_setting() {
        assert_eq!(heard_volume(2.0, false, false), 2.0);
        assert_eq!(heard_volume(2.0, false, true), 2.0);
        assert_eq!(heard_volume(2.0, true, false), 2.0);
        assert_eq!(heard_volume(2.0, true, true), 0.0);
        assert_eq!(heard_volume(9.0, true, false), MAX_VOLUME as f32);
        assert_eq!(heard_volume(-1.0, false, false), 0.0);
        const { assert!(!DEFAULT_DUEL_MUTE) };
    }

    const WAVE: &str = r#"{"name": "Wave", "torso": "BOTH_ENGAGETAUNT", "legs": "",
        "length_ms": 2500, "loop": false, "sound": "sound/emotes/wave.mp3"}"#;

    fn wave() -> EmoteDef {
        parse("wave", WAVE).unwrap()
    }

    #[test]
    fn tab_completes_the_installed_ids_and_the_commands_words() {
        assert_eq!(
            words(&[wave()]),
            ["wave", STOP, SYNC, MUTE, UNMUTE, crate::emote_help::WORD]
        );
        assert!(!words(&[]).iter().any(|word| word == QUIET || word == LOUD));
    }

    #[test]
    fn a_rig_clip_emote_reads_its_clip_and_stays_local() {
        let def = parse(
            "kane_dance",
            r#"{"name": "Dance", "clip": "kane_dance", "length_ms": 3400, "local": true}"#,
        )
        .unwrap();
        assert_eq!((def.clip.as_str(), def.local), ("kane_dance", true));
        assert!(!wave().local && wave().clip.is_empty());
        assert!(parse("x", r#"{"name": "X", "length_ms": 500, "local": 1}"#).is_err());
    }

    #[test]
    fn an_emote_file_parses_and_bad_ones_are_named() {
        let def = wave();
        assert_eq!(def.id, "wave");
        assert_eq!(def.name, "Wave");
        assert_eq!(def.torso, "BOTH_ENGAGETAUNT");
        assert_eq!(def.length, Duration::from_millis(2500));
        assert!(!def.looping);
        assert_eq!(def.sound, "sound/emotes/wave.mp3");
        // Only the name is required; a loop needs no length.
        let sit = parse(
            "sit",
            r#"{"name": "Sit", "legs": "BOTH_SIT1", "loop": true}"#,
        )
        .unwrap();
        assert!(sit.looping && sit.torso.is_empty());
        for (id, text, why) in [
            ("Wave", WAVE, "id"),
            ("wave", "not json", "JSON"),
            ("wave", r#"{"torso": "X"}"#, "name"),
            ("wave", r#"{"name": "Wave"}"#, "length_ms"),
            ("wave", r#"{"name": "Wave", "length_ms": 50}"#, "length_ms"),
            (
                "wave",
                r#"{"name": "Wave", "length_ms": 1000, "extra": 1}"#,
                "extra",
            ),
        ] {
            let error = parse(id, text).unwrap_err();
            assert!(error.contains(why), "{id} {text}: {error}");
        }
    }

    #[test]
    fn the_listing_names_each_emote_or_says_none_is_installed() {
        assert!(listing(&[])[0].starts_with("No emotes are installed"));
        let sit = parse("sit", r#"{"name": "Sit", "loop": true}"#).unwrap();
        assert_eq!(
            listing(&[sit, wave()]),
            [
                "sit - Sit, loops",
                "wave - Wave, 2.5 s",
                "sjkemote stop ends yours; stop all or stop <player> ends others' here, once.",
                "sjkemote sync [player] joins another player's emote in step.",
                "sjkemote mute <player>, unmute <player|all>: hide a player's emotes here (mute alone lists).",
                "emotes opens a window to search them and click one to play.",
                "help_emotes explains emotes and the quick wheel."
            ]
        );
    }

    #[test]
    fn emote_ids_are_the_hubs() {
        assert!(valid_id("wave"));
        assert!(valid_id("sit_2"));
        assert!(!valid_id(""));
        assert!(!valid_id("Wave"));
        assert!(!valid_id(&"a".repeat(33)));
    }

    #[test]
    fn an_attack_or_alt_attack_ends_the_own_emote_and_its_sound() {
        let t0 = Instant::now();
        let dance = parse(
            "dance",
            r#"{"name": "Dance", "loop": true, "moving": true,
                "sound": "sound/emotes/dance.wav"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        // Nobody dancing: nothing to tell. Walking (16) alone leaves it.
        assert!(!active.end_on_attack(2, 1, true));
        active.start(2, "dance", Some(&dance), Some([0.0; 3]), t0);
        active.take_cues();
        assert!(!active.end_on_attack(2, 16, true));
        assert!(active.get(2).is_some());
        assert!(active.end_on_attack(2, 1 | 16, true));
        assert!(active.get(2).is_none());
        assert_eq!(active.take_cues(), vec![Cue::Stop { slot: 2 }]);
        active.start(2, "dance", Some(&dance), Some([0.0; 3]), t0);
        assert!(active.end_on_attack(2, 128, true), "alt attack");
        assert!(active.get(2).is_none());
    }

    /// Rollembones' idea (23/10/2026): a player typing stays in their emote until they
    /// move or attack after closing the chat, whatever happens while they type.
    #[test]
    fn typing_keeps_the_emote_until_the_player_moves_after() {
        let t0 = Instant::now();
        let sit = parse(
            "sit",
            r#"{"name": "Sit", "loop": true, "sound": "sound/emotes/sit.wav"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        active.start(2, "sit", Some(&sit), Some([0.0; 3]), t0);
        active.start(3, "sit", Some(&sit), Some([0.0; 3]), t0);
        active.take_cues();
        // Slot 2 types; both are pushed 40 units away.
        active.hold(1 << 2);
        let pushed = [40.0, 0.0, 0.0];
        active.update(t0 + Duration::from_millis(100), |_| Some(pushed));
        assert!(active.get(2).is_some(), "typing: pushed, still sitting");
        assert!(active.get(3).is_none(), "not typing: moved, ended");
        assert!(
            !active.end_on_attack(2, 1, true),
            "a click while typing is not an attack"
        );
        assert!(active.get(2).is_some());
        // The chat closes: where they are now is the new place to stay.
        active.hold(0);
        active.update(t0 + Duration::from_millis(200), |_| Some(pushed));
        assert!(
            active.get(2).is_some(),
            "closing the chat alone ends nothing"
        );
        active.update(t0 + Duration::from_millis(300), |_| Some([52.0, 0.0, 0.0]));
        assert!(active.get(2).is_some(), "12 units: still there");
        active.update(t0 + Duration::from_millis(400), |_| Some([70.0, 0.0, 0.0]));
        assert!(active.get(2).is_none(), "walking off after typing ends it");
        // And an attack after typing does too.
        active.start(2, "sit", Some(&sit), Some([0.0; 3]), t0);
        active.hold(1 << 2);
        active.hold(0);
        assert!(active.end_on_attack(2, 1, true));
    }

    #[test]
    fn with_the_setting_off_typing_holds_nothing() {
        let t0 = Instant::now();
        let sit = parse("sit", r#"{"name": "Sit", "loop": true}"#).unwrap();
        let mut active = ActiveEmotes::default();
        active.start(2, "sit", Some(&sit), Some([0.0; 3]), t0);
        // The frame hook passes no typists while `cg_emoteTypingKeep` is off.
        active.hold(typing_slots(false, Some((2, true)), std::iter::empty()));
        active.update(t0 + Duration::from_millis(100), |_| Some([40.0, 0.0, 0.0]));
        assert!(active.get(2).is_none());
    }

    #[test]
    fn typists_are_the_talk_balloons_and_the_own_open_chat() {
        const EF_TALK: u32 = sjk_game_jka::pmove_talk::EF_TALK;
        let others = [(4u16, EF_TALK), (5, 0), (70, EF_TALK), (6, EF_TALK | 2)];
        assert_eq!(
            typing_slots(true, Some((1, true)), others.iter().copied()),
            1 << 1 | 1 << 4 | 1 << 6,
            "a number past the slots is no player"
        );
        assert_eq!(typing_slots(true, Some((1, false)), std::iter::empty()), 0);
        assert_eq!(
            typing_slots(false, Some((1, true)), others.iter().copied()),
            0
        );
    }

    /// Hub report #247: a `moving` emote (as `cloud_float`) stayed on through a ride,
    /// fighting the vehicle's rider animations.
    #[test]
    fn mounting_a_vehicle_ends_even_a_moving_emote() {
        let t0 = Instant::now();
        let float = parse(
            "cloud_float",
            r#"{"name": "Cloud", "loop": true, "moving": true, "sound": "sound/emotes/cloud.mp3"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        active.start(1, "cloud_float", Some(&float), Some([0.0; 3]), t0);
        active.start(3, "cloud_float", Some(&float), Some([0.0; 3]), t0);
        active.take_cues();
        // Nobody rides: both keep floating.
        assert!(!active.end_riding(0, Some(1)));
        assert!(active.get(1).is_some() && active.get(3).is_some());
        // Another player mounts: theirs ends here, the own player has nothing to send.
        assert!(!active.end_riding(1 << 3, Some(1)));
        assert!(active.get(3).is_none() && active.get(1).is_some());
        assert!(matches!(active.take_cues()[..], [Cue::Stop { slot: 3 }]));
        // The own player mounts: theirs ends and the others are told.
        assert!(active.end_riding(1 << 1, Some(1)));
        assert!(active.get(1).is_none());
        assert!(!active.end_riding(1 << 1, Some(1)), "told once");
    }

    #[test]
    fn riders_are_the_players_whose_vehicle_number_is_set() {
        // Slot 70 is the vehicle's own entity, naming its pilot: no player.
        let others = [(4u16, 70u16), (5, 0), (70, 4), (2, 0)];
        assert_eq!(
            riding_slots(Some((1, 71)), others.iter().copied()),
            1 << 1 | 1 << 4
        );
        // The own player's entity in the list does not override their playerState.
        assert_eq!(riding_slots(Some((5, 0)), [(5u16, 70u16)]), 0);
        assert_eq!(riding_slots(None, std::iter::empty()), 0);
    }

    /// Hub report #107: a dancer out of the snapshot for a moment keeps dancing and
    /// their music; only one missing for two seconds is gone.
    #[test]
    fn a_dancer_missing_for_a_moment_keeps_dancing() {
        let t0 = Instant::now();
        let dance = parse(
            "dance",
            r#"{"name": "Dance", "loop": true, "sound": "sound/emotes/dance.wav"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        active.start(2, "dance", Some(&dance), Some([0.0; 3]), t0);
        active.take_cues();
        active.update(t0 + Duration::from_millis(100), |_| None);
        active.update(t0 + Duration::from_millis(200), |_| Some([0.0; 3]));
        assert!(active.get(2).is_some(), "a frame missing");
        assert!(active.take_cues().is_empty());
        active.update(t0 + Duration::from_secs(1), |_| None);
        active.update(t0 + Duration::from_secs(3), |_| None);
        assert!(active.get(2).is_none(), "missing for two seconds");
        assert_eq!(active.take_cues(), vec![Cue::Stop { slot: 2 }]);
    }

    /// Hub report #107: an emote started while its player was not in the snapshot
    /// stands where they are first seen, not at the map's centre.
    #[test]
    fn an_emote_started_unseen_stands_where_its_player_is_first_seen() {
        let t0 = Instant::now();
        let dance = parse("dance", r#"{"name": "Dance", "loop": true}"#).unwrap();
        let mut active = ActiveEmotes::default();
        active.start(2, "dance", Some(&dance), None, t0);
        let far = [900.0, 400.0, 64.0];
        active.update(t0 + Duration::from_millis(100), |_| Some(far));
        active.update(t0 + Duration::from_millis(200), |_| Some(far));
        assert_eq!(active.get(2).and_then(|dance| dance.origin), Some(far));
    }

    #[test]
    fn by_default_an_attack_leaves_the_own_emote_dancing() {
        let t0 = Instant::now();
        let dance = parse("dance", r#"{"name": "Dance", "loop": true}"#).unwrap();
        let mut active = ActiveEmotes::default();
        active.start(2, "dance", Some(&dance), Some([0.0; 3]), t0);
        active.take_cues();
        assert!(!active.end_on_attack(2, 1, DEFAULT_ATTACK_STOP));
        assert!(!active.end_on_attack(2, 128, DEFAULT_ATTACK_STOP));
        assert!(active.get(2).is_some());
        assert!(active.take_cues().is_empty());
    }

    #[test]
    fn known_emotes_last_their_length_and_unknown_ones_three_seconds() {
        let t0 = Instant::now();
        let mut active = ActiveEmotes::default();
        active.start(3, "wave", Some(&wave()), Some([0.0; 3]), t0);
        active.start(4, "dance", None, Some([0.0; 3]), t0);
        let here = |_| Some([0.0; 3]);
        active.update(t0 + Duration::from_millis(2_400), here);
        assert_eq!(active.get(3).map(|e| e.emote.as_str()), Some("wave"));
        active.update(t0 + Duration::from_millis(2_600), here);
        assert!(active.get(3).is_none());
        assert!(active.get(4).is_some());
        active.update(t0 + Duration::from_secs(3), here);
        assert!(active.get(4).is_none());
    }

    #[test]
    fn a_moving_emote_survives_walking_and_sounds_start_and_stop() {
        let t0 = Instant::now();
        let dance = parse(
            "dance",
            r#"{"name": "Dance", "length_ms": 6800, "moving": true,
                "sound": "sound/emotes/dance.wav"}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        active.start(5, "dance", Some(&dance), Some([0.0; 3]), t0);
        assert_eq!(
            active.take_cues(),
            vec![Cue::Start {
                slot: 5,
                sound: "sound/emotes/dance.wav".into(),
                offset: Duration::ZERO,
                align: None
            }]
        );
        active.update(t0 + Duration::from_secs(1), |_| Some([500.0, 0.0, 0.0]));
        assert!(active.get(5).is_some(), "it dances on while running");
        // Running its length lets the sound finish.
        active.update(t0 + Duration::from_secs(7), |_| Some([500.0, 0.0, 0.0]));
        assert!(active.get(5).is_none() && active.take_cues().is_empty());
        // Cut short (stopped, or replaced), the sound stops.
        active.start(5, "dance", Some(&dance), Some([0.0; 3]), t0);
        active.start(5, "wave", Some(&wave()), Some([0.0; 3]), t0);
        active.stop(5);
        assert_eq!(
            active.take_cues(),
            vec![
                Cue::Start {
                    slot: 5,
                    sound: "sound/emotes/dance.wav".into(),
                    offset: Duration::ZERO,
                    align: None
                },
                Cue::Stop { slot: 5 },
                Cue::Start {
                    slot: 5,
                    sound: "sound/emotes/wave.mp3".into(),
                    offset: Duration::ZERO,
                    align: None
                },
                Cue::Stop { slot: 5 },
            ]
        );
        assert!(active.get(5).is_none());
    }

    #[test]
    fn moving_or_leaving_ends_an_emote_and_a_loop_lasts_until_then() {
        let t0 = Instant::now();
        let sit = parse("sit", r#"{"name": "Sit", "loop": true}"#).unwrap();
        let mut active = ActiveEmotes::default();
        active.start(1, "sit", Some(&sit), Some([100.0, 0.0, 0.0]), t0);
        active.start(2, "wave", Some(&wave()), Some([0.0; 3]), t0);
        active.start(70, "wave", None, Some([0.0; 3]), t0);
        assert!(active.get(70).is_none(), "no such slot");
        let later = t0 + Duration::from_secs(600);
        active.update(later, |slot| (slot == 1).then_some([110.0, 0.0, 0.0]));
        assert!(active.get(1).is_some(), "a loop, moved a little");
        assert!(active.get(2).is_none(), "left the server");
        active.update(later, |_| Some([130.0, 0.0, 0.0]));
        assert!(active.get(1).is_none(), "walked off");
    }

    #[test]
    fn an_emote_needs_the_slot_name_to_match_the_claim() {
        let emote = Emote {
            id: 1,
            at: 0,
            slot: 3,
            claimed_name: "^2Sol".to_owned(),
            key_id: "0123456789abcdef".to_owned(),
            emote: "wave".to_owned(),
            sync: None,
            quiet: false,
        };
        assert!(trusted(&emote, Some("Sol")));
        assert!(
            !trusted(&emote, Some("Fox")),
            "someone else in the slot now"
        );
        assert!(!trusted(&emote, None), "nobody in the slot");
    }

    #[test]
    fn music_with_an_intro_then_loops_and_a_looping_emote_repeats_its_sound() {
        let def = parse(
            "heavy",
            r#"{"name": "Heavy", "length_ms": 9000, "sound": "a.wav", "sound_ms": 2000,
                "sound_loop": "b.wav", "sound_loop_ms": 3000}"#,
        )
        .unwrap();
        let mut active = ActiveEmotes::default();
        let start = Instant::now();
        active.start(1, "heavy", Some(&def), Some([0.0; 3]), start);
        let started = |active: &mut ActiveEmotes| {
            active
                .take_cues()
                .into_iter()
                .filter_map(|cue| match cue {
                    Cue::Start { sound, .. } => Some(sound),
                    Cue::Stop { .. } => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(started(&mut active), ["a.wav"]);
        let here = |_: u8| Some([0.0; 3]);
        active.update(start + Duration::from_millis(1900), here);
        assert!(started(&mut active).is_empty());
        active.update(start + Duration::from_millis(2000), here);
        assert_eq!(started(&mut active), ["b.wav"]);
        active.update(start + Duration::from_millis(5000), here);
        assert_eq!(started(&mut active), ["b.wav"]);
        // A looping emote with one sound plays it again when it ends.
        let looped = parse(
            "boom",
            r#"{"name": "Boom", "loop": true, "sound": "c.wav", "sound_ms": 4000}"#,
        )
        .unwrap();
        active.start(2, "boom", Some(&looped), Some([0.0; 3]), start);
        let _ = active.take_cues();
        active.update(start + Duration::from_millis(4000), here);
        assert!(started(&mut active).contains(&"c.wav".to_owned()));
    }

    fn heavy() -> EmoteDef {
        parse(
            "heavy",
            r#"{"name": "Heavy", "length_ms": 9000, "sound": "a.wav", "sound_ms": 2000,
                "sound_loop": "b.wav", "sound_loop_ms": 3000}"#,
        )
        .unwrap()
    }

    #[test]
    fn a_quiet_emote_keeps_time_silently_and_its_sound_comes_back_in_place() {
        let t0 = Instant::now();
        let here = |_: u8| Some([0.0; 3]);
        let mut active = ActiveEmotes::default();
        active.start(1, "heavy", Some(&heavy()), Some([0.0; 3]), t0);
        // Quiet from its start (an away emote): its sound never starts.
        assert!(active.set_quiet(1, true, Some(&heavy()), t0));
        assert_eq!(active.take_cues(), [Cue::Stop { slot: 1 }]);
        assert!(
            !active.set_quiet(1, true, Some(&heavy()), t0),
            "already quiet"
        );
        active.update(t0 + Duration::from_millis(2500), here);
        assert!(active.take_cues().is_empty(), "the loop stays silent");
        // Loud again 3.5 s in: the loop, 1.5 s into it, then again 1.5 s later.
        let back = t0 + Duration::from_millis(3500);
        assert!(active.set_quiet(1, false, Some(&heavy()), back));
        assert_eq!(
            active.take_cues(),
            [Cue::Start {
                slot: 1,
                sound: "b.wav".to_owned(),
                offset: Duration::from_millis(1500),
                align: None,
            }]
        );
        active.update(back + Duration::from_millis(1500), here);
        assert_eq!(active.take_cues().len(), 1);
        assert!(
            !active.set_quiet(2, true, None, back),
            "nobody emotes there"
        );
    }

    #[test]
    fn a_synced_emote_starts_with_its_leader_and_plays_the_music_in_step() {
        let t0 = Instant::now();
        let here = |_: u8| Some([0.0; 3]);
        let mut active = ActiveEmotes::default();
        active.start(1, "heavy", Some(&heavy()), Some([0.0; 3]), t0);
        let _ = active.take_cues();
        let late = t0 + Duration::from_millis(1500);
        assert!(
            !active.sync(2, 1, "wave", Some(&wave()), Some([0.0; 3]), late),
            "not that emote"
        );
        assert!(
            !active.sync(2, 9, "heavy", Some(&heavy()), Some([0.0; 3]), late),
            "nobody there"
        );
        // Joining 1.5 s late (the hub's delay) still lines up with the leader, and
        // its music starts where the leader's is.
        active.update(late, here);
        assert!(active.sync(2, 1, "heavy", Some(&heavy()), Some([0.0; 3]), late));
        let follower = active.get(2).unwrap();
        assert_eq!((follower.started, follower.leader), (t0, Some(1)));
        assert_eq!(follower.until, Some(t0 + Duration::from_millis(9000)));
        assert_eq!(
            active.take_cues(),
            [Cue::Start {
                slot: 2,
                sound: "a.wav".into(),
                offset: Duration::from_millis(1500),
                align: Some(1)
            }]
        );
        // The loop starts on both at the same moment.
        active.update(t0 + Duration::from_millis(2000), here);
        let b = |slot, align| Cue::Start {
            slot,
            sound: "b.wav".into(),
            offset: Duration::ZERO,
            align,
        };
        assert_eq!(active.take_cues(), [b(1, None), b(2, Some(1))]);
        // A third joining the follower follows the first leader.
        assert!(active.sync(3, 2, "heavy", Some(&heavy()), Some([0.0; 3]), late));
        assert_eq!(active.get(3).unwrap().leader, Some(1));
    }

    #[test]
    fn music_back_from_away_lines_up_with_the_group_still_playing() {
        let t0 = Instant::now();
        let mut active = ActiveEmotes::default();
        active.start(1, "heavy", Some(&heavy()), Some([0.0; 3]), t0);
        assert!(active.sync(2, 1, "heavy", Some(&heavy()), Some([0.0; 3]), t0));
        assert!(active.sync(3, 1, "heavy", Some(&heavy()), Some([0.0; 3]), t0));
        let back = t0 + Duration::from_millis(1000);
        let start = |slot, align| Cue::Start {
            slot,
            sound: "a.wav".into(),
            offset: Duration::from_millis(1000),
            align,
        };
        // The leader alt-tabs and comes back (hub report #195): its music starts at
        // the sample a follower's plays, not from the clock alone.
        assert!(active.set_quiet(1, true, Some(&heavy()), t0));
        let _ = active.take_cues();
        assert!(active.set_quiet(1, false, Some(&heavy()), back));
        assert_eq!(active.take_cues(), [start(1, Some(2))]);
        // A follower back while its leader is away lines up with the other follower.
        assert!(active.set_quiet(1, true, Some(&heavy()), back));
        assert!(active.set_quiet(2, true, Some(&heavy()), back));
        let _ = active.take_cues();
        assert!(active.set_quiet(2, false, Some(&heavy()), back));
        assert_eq!(active.take_cues(), [start(2, Some(3))]);
        // A follower back with its leader playing lines up with the leader, as before.
        assert!(active.set_quiet(1, false, Some(&heavy()), back));
        assert!(active.set_quiet(3, true, Some(&heavy()), back));
        let _ = active.take_cues();
        assert!(active.set_quiet(3, false, Some(&heavy()), back));
        assert_eq!(active.take_cues(), [start(3, Some(1))]);
    }

    #[test]
    fn a_follower_keeps_its_music_when_the_leader_stops() {
        let t0 = Instant::now();
        let here = |_: u8| Some([0.0; 3]);
        let mut active = ActiveEmotes::default();
        active.start(1, "heavy", Some(&heavy()), Some([0.0; 3]), t0);
        assert!(active.sync(2, 1, "heavy", Some(&heavy()), Some([0.0; 3]), t0));
        let _ = active.take_cues();
        active.stop(1);
        active.update(t0 + Duration::from_millis(1000), here);
        assert_eq!(active.take_cues(), [Cue::Stop { slot: 1 }]);
        active.stop(2);
        assert_eq!(active.take_cues(), [Cue::Stop { slot: 2 }]);
    }

    #[test]
    fn music_knows_what_plays_when() {
        let music = Music::of(&heavy());
        let at = |ms| music.at(Duration::from_millis(ms));
        assert_eq!(
            at(500),
            Some((
                "a.wav",
                Duration::from_millis(500),
                Some(Duration::from_millis(1500))
            ))
        );
        assert_eq!(
            at(8000),
            Some(("b.wav", Duration::ZERO, Some(Duration::from_millis(3000))))
        );
        let once = Music::of(&wave());
        assert_eq!(
            once.at(Duration::from_secs(9)).map(|(s, ..)| s),
            Some("sound/emotes/wave.mp3")
        );
        let timed = Music::of(
            &parse(
                "t",
                r#"{"name": "T", "length_ms": 3000, "sound": "t.wav", "sound_ms": 1000}"#,
            )
            .unwrap(),
        );
        assert_eq!(timed.at(Duration::from_secs(2)), None, "over");
    }
}
