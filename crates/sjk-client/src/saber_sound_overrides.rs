//! Per-player additions to the stock saber sounds: a blade skin (an SJK unlockable,
//! drawn by the viewer) brings its own ignition, switching off, hum and swings, heard for
//! every player wearing it and only for them, on top of the stock sounds, which keep
//! playing below; or, with `s_saberSkinSounds 2`
//! ([`LegacySoundAdapter::set_saber_skin_sounds_only`]), in their place.
//!
//! The viewer registers its skins' sound sets once per gamestate
//! ([`LegacySoundAdapter::register_saber_sound_sets`]) and tells the adapter every frame
//! which client slot wears which set
//! ([`LegacySoundAdapter::set_saber_sound_overrides`]). With no set worn, every path
//! below is the compatibility one, unchanged. Added, for a client wearing a set (each
//! on a channel of its own, so it does not cut the stock sound):
//!
//! - `EV_SABER_UNHOLSTER` (the ignition the game predicts) plays the set's `on`;
//! - cgame's own ignition and switching off as the player draws the saber from another
//!   weapon or puts a lit one away ([`super::saber_switch`]) play `on` or `off`, once;
//! - `EV_GENERAL_SOUND` of a stock `saberon*`/`saberoff*` or `enemy_saber_on`/`off`
//!   sound, or of any `.sab`
//!   `soundOn`/`soundOff`, plays `on`/`off`. The game's `G_Sound` puts these on a
//!   temporary entity with no owner (`g_utils.c`), so the sound is taken as the
//!   player's whose origin is nearest the event's, within [`TOGGLE_REACH`];
//! - `EV_SABER_ATTACK` plays one of the three `swings` (as a `.sab` with
//!   `swingSound1`-`3` does, `cg_event.c`);
//! - the hum loop ([`crate::loop_sounds`]) plays `hum` once, whichever sabers are lit,
//!   and for the saber while it is thrown (over the flying saber entity's own
//!   `loopSound`).
//!
//! Animation swing cues are the viewer's (`actor_sounds.rs`).

use super::{LegacySoundAdapter, RegisteredLegacySound, intern_sound, sound_present};
use sjk_audio::SoundHandle;
use sjk_protocol::Snapshot;
use sjk_vfs::VirtualFileSystem;

/// Client slots a set can be worn in (`MAX_CLIENTS`).
pub const SABER_SOUND_CLIENTS: usize = 32;
/// How far from a player a saber on/off sound may start and still be that player's:
/// the game sounds it from where the player stood when the command ran, which the
/// rest of the server frame can move by a few units.
pub const TOGGLE_REACH: f32 = 64.0;
const ET_PLAYER: u8 = 1;

/// The game paths of one set of replacement saber sounds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaberSoundSet<'a> {
    /// Ignition.
    pub on: &'a str,
    /// Switching off.
    pub off: &'a str,
    /// The hum, looped while a blade is lit.
    pub hum: &'a str,
    /// Swings, chosen like `.sab` `swingSound1`-`3`.
    pub swings: [&'a str; 3],
}

/// A set's sounds in the event adapter's table; `None` where the set's file did not
/// load (not in the game data yet), so the stock sound plays there as for a player
/// with no set, in either `s_saberSkinSounds` mode (never silence).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct EventSet {
    on: Option<u16>,
    off: Option<u16>,
    swings: [Option<u16>; 3],
}

/// Whether a sound switches a saber on or off.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Toggle {
    On,
    Off,
}

/// Which client wears which set, and each set's event sounds.
#[derive(Clone, Debug, Default)]
pub(crate) struct SaberSoundOverrides {
    sets: Vec<EventSet>,
    clients: [Option<u16>; SABER_SOUND_CLIENTS],
    /// Whether any client wears a set, so stock events skip every check.
    active: bool,
    /// `s_saberSkinSounds 2`: a worn set's sounds replace the stock ones instead of
    /// playing over them.
    pub(crate) only: bool,
    /// The `.sab` definitions' own on/off sounds, lowercase and without extension.
    toggles: Vec<(Box<str>, Toggle)>,
    /// The server toggle each client's set last voiced: its snapshot server time and
    /// way. The toggle sounds every hilt (a single saber's removed second slot too), and
    /// the set's sound plays once over them all.
    voiced: [Option<(i32, Toggle)>; SABER_SOUND_CLIENTS],
}

/// What a worn set does with a general sound ([`SaberSoundOverrides::general`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GeneralSkin {
    /// Not a toggle of a client wearing a set with that sound: the stock sound alone.
    Stock,
    /// The set's sound, over the stock one or in its place.
    Skin(u16),
    /// Another sound of a toggle the set already voiced: the stock sound alone, or
    /// nothing with `s_saberSkinSounds 2`.
    Voiced,
}

impl SaberSoundOverrides {
    fn register(
        &mut self,
        sets: &[SaberSoundSet<'_>],
        sounds: &mut Vec<RegisteredLegacySound>,
        vfs: &VirtualFileSystem,
        register: &mut impl FnMut(&str, &[u8]) -> Option<SoundHandle>,
    ) {
        let mut intern = |path: &str| {
            if !sound_present(vfs, path) {
                return None;
            }
            let index = intern_sound(sounds, vfs, path, register);
            sounds[usize::from(index)].handle.map(|_| index)
        };
        self.sets = sets
            .iter()
            .map(|set| EventSet {
                on: intern(set.on),
                off: intern(set.off),
                swings: set.swings.map(&mut intern),
            })
            .collect();
        self.toggles.clear();
        let definitions = crate::shared_saber_definitions(vfs).unwrap_or_default();
        for definition in definitions.values() {
            for (path, toggle) in [
                (definition.sound_on.as_deref(), Toggle::On),
                (definition.sound_off.as_deref(), Toggle::Off),
            ] {
                let Some(stem) = path.map(stem) else {
                    continue;
                };
                if !stem.is_empty()
                    && !self
                        .toggles
                        .iter()
                        .any(|(known, _)| known.eq_ignore_ascii_case(stem))
                {
                    self.toggles
                        .push((stem.to_ascii_lowercase().into(), toggle));
                }
            }
        }
        let clients = self.clients;
        self.set_clients(&clients);
    }

    fn set_clients(&mut self, clients: &[Option<u16>; SABER_SOUND_CLIENTS]) {
        self.clients = clients.map(|set| set.filter(|&set| usize::from(set) < self.sets.len()));
        self.active = self.clients.iter().any(Option::is_some);
    }

    /// The set entity `source` wears: a client's body (`source` < 32) with one.
    fn for_source(&self, source: u16) -> Option<EventSet> {
        if !self.active {
            return None;
        }
        let set = (*self.clients.get(usize::from(source))?)?;
        self.sets.get(usize::from(set)).copied()
    }

    /// The set's swing for `variant`, as `ci->saber[0].swingSound[Q_irand(0, 2)]`.
    pub(crate) fn swing(&self, source: u16, variant: usize) -> Option<u16> {
        self.for_source(source)?.swings[variant % 3]
    }

    /// The set's ignition.
    pub(crate) fn on(&self, source: u16) -> Option<u16> {
        self.for_source(source)?.on
    }

    /// The set's switching off.
    pub(crate) fn off(&self, source: u16) -> Option<u16> {
        self.for_source(source)?.off
    }

    /// What a general sound `sound` started at `origin` gets: the set's on or off when
    /// `sound` switches a saber and the nearest player wears a set, once per toggle.
    pub(crate) fn general(
        &mut self,
        sound: Option<u16>,
        sounds: &[RegisteredLegacySound],
        origin: [f32; 3],
        snapshot: &Snapshot,
    ) -> GeneralSkin {
        if !self.active {
            return GeneralSkin::Stock;
        }
        let Some(path) = sound.and_then(|index| sounds.get(usize::from(index))) else {
            return GeneralSkin::Stock;
        };
        let Some(toggle) = self.toggle(&path.path) else {
            return GeneralSkin::Stock;
        };
        let Some(client) = nearest_player(snapshot, origin) else {
            return GeneralSkin::Stock;
        };
        let Some(set) = self.for_source(client) else {
            return GeneralSkin::Stock;
        };
        let skin = match toggle {
            Toggle::On => set.on,
            Toggle::Off => set.off,
        };
        let Some(skin) = skin else {
            return GeneralSkin::Stock;
        };
        let voiced = &mut self.voiced[usize::from(client)];
        let key = (snapshot.server_time, toggle);
        if *voiced == Some(key) {
            return GeneralSkin::Voiced;
        }
        *voiced = Some(key);
        GeneralSkin::Skin(skin)
    }

    fn toggle(&self, path: &str) -> Option<Toggle> {
        let stem = stem(path);
        let file = stem.rsplit('/').next().unwrap_or(stem);
        let stock = stem.len() > file.len()
            && stem[..stem.len() - file.len()].eq_ignore_ascii_case("sound/weapons/saber/");
        let starts = |prefix: &str| {
            file.get(..prefix.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
        };
        if stock && starts("saberon") {
            return Some(Toggle::On);
        }
        if stock && starts("saberoff") {
            return Some(Toggle::Off);
        }
        // `WP_SaberSetDefaults`' sounds, for a hilt that authors none.
        if stock && file.eq_ignore_ascii_case("enemy_saber_on") {
            return Some(Toggle::On);
        }
        if stock && file.eq_ignore_ascii_case("enemy_saber_off") {
            return Some(Toggle::Off);
        }
        self.toggles
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(stem))
            .map(|(_, toggle)| *toggle)
    }
}

/// `path` without its extension.
fn stem(path: &str) -> &str {
    match path.rsplit_once('.') {
        Some((stem, extension)) if !extension.contains('/') => stem,
        _ => path,
    }
}

/// The client whose body is nearest `origin`, within [`TOGGLE_REACH`]: the local
/// player from its own state, others from their entities.
fn nearest_player(snapshot: &Snapshot, origin: [f32; 3]) -> Option<u16> {
    let distance = |at: [f32; 3]| {
        (0..3)
            .map(|axis| (at[axis] - origin[axis]).powi(2))
            .sum::<f32>()
    };
    let local = (
        snapshot.player.client_num(),
        distance(snapshot.player.origin()),
    );
    snapshot
        .entities
        .iter()
        .filter(|state| {
            state.entity_type() == ET_PLAYER && usize::from(state.number()) < SABER_SOUND_CLIENTS
        })
        .map(|state| (state.number(), distance(state.trajectory_base())))
        .chain(std::iter::once(local))
        .filter(|(_, squared)| *squared <= TOGGLE_REACH * TOGGLE_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(client, _)| client)
}

impl LegacySoundAdapter {
    /// Whether a general sound `sound` at `origin` is a saber switching on or off whose
    /// player has spent their toggle sounds ([`super::saber_toggle_limit`]).
    pub(super) fn saber_toggle_muted(
        &mut self,
        sound: Option<u16>,
        origin: [f32; 3],
        snapshot: &Snapshot,
    ) -> bool {
        let Some(registered) = sound.and_then(|index| self.sounds.get(usize::from(index))) else {
            return false;
        };
        if self.saber_overrides.toggle(&registered.path).is_none() {
            return false;
        }
        nearest_player(snapshot, origin)
            .is_some_and(|client| !self.saber_toggle_limit.allow(client, snapshot.server_time))
    }

    /// Register replacement saber sound sets; a set is then worn by its index here.
    /// Call once after [`Self::new`], with the same `register`, before snapshots are
    /// observed (registration reads the files and the `.sab` definitions).
    pub fn register_saber_sound_sets(
        &mut self,
        sets: &[SaberSoundSet<'_>],
        vfs: &VirtualFileSystem,
        mut register: impl FnMut(&str, &[u8]) -> Option<SoundHandle>,
    ) {
        self.saber_overrides
            .register(sets, &mut self.sounds, vfs, &mut register);
        let hums: Vec<&str> = sets.iter().map(|set| set.hum).collect();
        self.loops.register_saber_hums(&hums, vfs, &mut register);
        // The clients' hums point into the table just rebuilt: an unchanged client list
        // would not refresh them ([`Self::set_saber_sound_overrides`]).
        self.loops
            .set_saber_hum_overrides(&self.saber_overrides.clients);
    }

    /// Which registered set each client slot wears (`None`: the stock sounds). Cheap
    /// (two 32-entry copies); the viewer calls it every frame.
    pub fn set_saber_sound_overrides(&mut self, clients: &[Option<u16>; SABER_SOUND_CLIENTS]) {
        if *clients == self.saber_overrides.clients {
            return;
        }
        self.saber_overrides.set_clients(clients);
        self.loops
            .set_saber_hum_overrides(&self.saber_overrides.clients);
    }

    /// `s_saberSkinSounds 2`: whether a worn blade skin's sounds replace the stock
    /// saber sounds (ignition, switching off, swings and hum) instead of playing over
    /// them. Cheap; the viewer calls it every frame.
    pub fn set_saber_skin_sounds_only(&mut self, only: bool) {
        self.saber_overrides.only = only;
        self.loops.skin_hum_only = only;
    }

    /// Whether a worn blade skin's sounds replace the stock ones
    /// ([`Self::set_saber_skin_sounds_only`]).
    pub fn saber_skin_sounds_only(&self) -> bool {
        self.saber_overrides.only
    }

    /// The set each client slot wears, after registration's checks.
    pub fn saber_sound_overrides(&self) -> &[Option<u16>; SABER_SOUND_CLIENTS] {
        &self.saber_overrides.clients
    }
}

#[cfg(test)]
#[path = "saber_sound_overrides_tests.rs"]
mod tests;
