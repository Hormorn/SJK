//! The viewer's side of player identity (`docs/identity.md`): turns the
//! `cl_identity` and `cl_hubUrl` settings and the live session into the
//! [`sjk_identity`] service's inputs, and gives the scoreboard and the Identity
//! page what the service learned.
//!
//! The service runs on its own thread; this module only compares what it last
//! told it with the current settings and place twice a second, so a frame never
//! waits for the hub. With the feature off, or no hub address, it sends
//! nothing. The key file is created the first time the feature is on.

use sjk_client::{LegacyClientInfo, decode_legacy};
use sjk_identity::{HttpHub, Hub, Identity, Location, Service, Settings, Snapshot};
use sjk_protocol::GameState;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// The hub address a fresh install uses, so players set nothing. With
/// `cl_identity` off, or `cl_hubUrl` emptied, nothing is sent anywhere.
pub(crate) const DEFAULT_HUB_URL: &str = "https://sjk.dfox.app";
/// The key file, in the settings folder beside `config.cfg`.
const KEY_FILE: &str = "identity.key";
/// How often the settings and the player's place are compared with what the
/// service was told.
const SYNC_EVERY: Duration = Duration::from_millis(500);
/// Longest the exit waits for the service to withdraw the player's claim.
const SHUTDOWN_WAIT: Duration = Duration::from_secs(2);
/// `CS_PLAYERS`: the first player's configstring.
pub(crate) const CS_PLAYERS: usize = 1131;
/// Clients the legacy protocol numbers (0 to 31).
const MAX_CLIENTS: usize = 32;

/// How a player appears on the scoreboard.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Tag {
    /// The hub's operator vouches for this player's key.
    pub(crate) verified: bool,
    /// Their tick, as the hub sent it (`ui_renderer::mark_texture`).
    pub(crate) tick: sjk_identity::Tick,
    /// Their trust (`ui_renderer::mark_texture`): a small mark on the emblem.
    pub(crate) trust: Option<sjk_identity::Trust>,
    /// The medals the SJK team gave them, for the ribbon bars.
    pub(crate) medals: crate::medals::Medals,
    /// Their name emoji, after the name.
    pub(crate) emoji: NameEmoji,
    /// The client they claimed from, when it has its own icon in place of SJK's emblem
    /// (`chat::client_icons`).
    pub(crate) client: Option<&'static crate::chat::client_icons::ClientIcon>,
}

/// A name emoji (`sjk_identity::name_emoji`) held in place, so a [`Tag`] stays `Copy`:
/// at most [`sjk_identity::name_emoji::EMOJI_MAX`] characters of 4 bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NameEmoji {
    bytes: [u8; 64],
    len: u8,
}

impl Default for NameEmoji {
    fn default() -> Self {
        Self {
            bytes: [0; 64],
            len: 0,
        }
    }
}

impl NameEmoji {
    /// The emoji a hub sent, shown through the rules; none when it does not pass.
    pub(crate) fn new(sent: &str) -> Self {
        let shown = sjk_identity::name_emoji::for_display(sent);
        let mut emoji = Self::default();
        if let Some(bytes) = emoji.bytes.get_mut(..shown.len()) {
            bytes.copy_from_slice(shown.as_bytes());
            emoji.len = shown.len() as u8;
        }
        emoji
    }

    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

#[derive(Default)]
struct Runtime {
    service: Option<Service>,
    /// Why the key file could not be used; the feature stays off until restart.
    key_error: Option<String>,
    sent_settings: Option<Settings>,
    sent_location: Option<Location>,
    sent_name: Option<String>,
    /// Whether the SJK chat was last told on (`cl_sjkChat`).
    sent_chat: Option<bool>,
    /// The clan chats last told joined ([`follow_clan_chats`]), JoF's then S's.
    sent_clans: [bool; 2],
    /// The look the service was last given (`looks.rs`).
    sent_look: Option<sjk_identity::Look>,
    /// Whether the service was last told the player is actively playing
    /// (`holocrons/activity.rs`).
    sent_active: Option<bool>,
    /// Whether the game menu was open at the last frame ([`follow_game_menu`]).
    game_menu_open: bool,
    next_sync: Option<Instant>,
    /// Keys whose profile was asked for their picture's version ([`avatar_version`]),
    /// newest last, so each is asked once.
    picture_lookups: Vec<String>,
    /// Keys whose profile the player card asked for ([`hub_info`]) and when, newest
    /// last, so each is asked at most every [`CARD_LOOKUP_EVERY`].
    card_lookups: Vec<(String, Instant)>,
    /// The crash folder last handed to the service, if any was.
    sent_crash_dir: Option<Option<std::path::PathBuf>>,
    /// The model list last handed to the service ([`offer_models`]), by address, and
    /// whether it was shared.
    sent_models: Option<(usize, bool)>,
    /// The model folder the local player wears, unless every player has it (a retail
    /// one): what the player card asks about ([`set_own_model`]).
    own_model: Option<String>,
    /// Model questions the player card asked and when, newest last, so each is asked
    /// at most every [`MODEL_CHECK_EVERY`].
    model_asks: Vec<((SocketAddr, u8, String), Instant)>,
    /// The serial of the last map edits answer shown ([`map_edits_published`]).
    map_edits_seen: u64,
    /// The service's status last written to the log ([`log_status`]).
    logged_status: Option<sjk_identity::Status>,
}

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    sent_clans: [false; 2],
    service: None,
    key_error: None,
    sent_settings: None,
    sent_location: None,
    sent_name: None,
    sent_chat: None,
    sent_look: None,
    sent_active: None,
    game_menu_open: false,
    next_sync: None,
    picture_lookups: Vec::new(),
    card_lookups: Vec::new(),
    sent_crash_dir: None,
    sent_models: None,
    own_model: None,
    map_edits_seen: 0,
    model_asks: Vec::new(),
    logged_status: None,
});

fn lock() -> MutexGuard<'static, Runtime> {
    RUNTIME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn make_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    HttpHub::new(url, crate::build_info::hub_agent()).map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// A hub client for the SJK chat's long poll, whose requests may last longer.
fn make_feed_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    HttpHub::with_timeout(
        url,
        crate::build_info::hub_agent(),
        sjk_identity::feed::TIMEOUT,
    )
    .map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// A hub client for shared vitals, whose requests give up sooner: a late answer is
/// out of date anyway.
fn make_vitals_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    HttpHub::with_timeout(
        url,
        crate::build_info::hub_agent(),
        sjk_identity::vitals::TIMEOUT,
    )
    .map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// A hub client for the toy relay's sends, which give up soon: a late one is out of date.
fn make_relay_send_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    HttpHub::with_timeout(
        url,
        crate::build_info::hub_agent(),
        sjk_identity::relay::SEND_TIMEOUT,
    )
    .map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// A hub client for the toy relay's long poll.
fn make_relay_read_hub(url: &str) -> Result<Box<dyn Hub>, sjk_identity::HubError> {
    HttpHub::with_timeout(
        url,
        crate::build_info::hub_agent(),
        sjk_identity::relay::TIMEOUT,
    )
    .map(|hub| Box::new(hub) as Box<dyn Hub>)
}

/// The name the game shows for `slot`, as the server published it.
pub(crate) fn shown_name(game_state: &GameState, slot: usize) -> Option<String> {
    let bytes = game_state.config_string(CS_PLAYERS + slot)?;
    let info = LegacyClientInfo::new(bytes);
    let name = info.bytes("n").or_else(|| info.bytes("name"))?;
    (!name.is_empty()).then(|| decode_legacy(name).into_owned())
}

/// Where the player is: the server they are connected to, their slot there and
/// the name the game shows for them, or the one they wear (`worn`, the `name` setting)
/// when the server dropped characters of it (`chat::marks::claimed`), so other SJK
/// players show it whole. `None` in a local game or without a name.
pub(crate) fn location(
    server: SocketAddr,
    local: bool,
    game_state: &GameState,
    worn: &str,
) -> Option<Location> {
    let slot = usize::try_from(game_state.client_num)
        .ok()
        .filter(|slot| *slot < MAX_CLIENTS)?;
    if local {
        return None;
    }
    Some(Location {
        server,
        slot: u8::try_from(slot).ok()?,
        name: shown_name(game_state, slot)
            .map(|shown| crate::chat::marks::claimed(worn, &shown).to_owned())?,
    })
}

/// The log line for the service's `status` when it differs from the one last
/// logged (`last`): whether the hub took the registration, or why the last request
/// failed, so a player's `session.log` shows what a refusal before the registration
/// meant (`docs/identity.md`, "What happens"). Read twice a second, with [`apply`].
fn log_status(
    last: Option<&sjk_identity::Status>,
    status: &sjk_identity::Status,
) -> Option<String> {
    use sjk_identity::Status;
    if last == Some(status) {
        return None;
    }
    Some(match status {
        Status::Disabled => "identity: off (cl_identity 0)".to_owned(),
        Status::NoHub => "identity: no hub (cl_hubUrl empty)".to_owned(),
        Status::Registering => "identity: registering with the hub".to_owned(),
        Status::Online => "identity: registered with the hub".to_owned(),
        Status::Failed(why) => format!("identity: hub request failed: {why}"),
    })
}

/// Whether it is time to [`apply`] the settings and the player's place again.
/// True twice a second at most, so the caller builds them only then.
pub(crate) fn due() -> bool {
    let mut runtime = lock();
    let now = Instant::now();
    if runtime.next_sync.is_some_and(|due| now < due) {
        return false;
    }
    runtime.next_sync = Some(now + SYNC_EVERY);
    true
}

/// Bring the service in line with the settings, the in-game `name` the player wears
/// (the `name` setting, which the hub keeps in the key's name history: the player
/// chooses nothing), the player's place and whether the SJK chat is on (`chat`).
pub(crate) fn apply(
    config_directory: &Path,
    settings: Settings,
    name: String,
    location: Option<Location>,
    chat: bool,
) {
    let mut runtime = lock();
    if runtime.service.is_none() {
        if !settings.enabled || runtime.key_error.is_some() {
            return;
        }
        match Identity::load_or_create(&config_directory.join(KEY_FILE)) {
            Ok(identity) => {
                crate::log::progress(format_args!(
                    "identity: key {} ({})",
                    identity.key_id(),
                    KEY_FILE
                ));
                let service = Service::start_with_threads(
                    identity,
                    Box::new(make_hub),
                    Some(Box::new(make_feed_hub)),
                    Some(Box::new(make_vitals_hub)),
                    Some(sjk_identity::RelayHubs {
                        send: Box::new(make_relay_send_hub),
                        read: Box::new(make_relay_read_hub),
                    }),
                );
                // The hub's packs (unlockables' art) are kept beside the key.
                service.keep_assets(crate::sjk_packs::directory_in(config_directory));
                // SJK's map edits, every player's copy beside the key too.
                service.keep_map_edits(crate::map_edits::folder(config_directory));
                runtime.service = Some(service);
            }
            Err(error) => {
                crate::log::progress(format_args!("identity: {error}"));
                runtime.key_error = Some(error.to_string());
                return;
            }
        }
    }
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return;
    };
    let status = service.with_snapshot(|snapshot| snapshot.status.clone());
    if let Some(line) = log_status(runtime.logged_status.as_ref(), &status) {
        crate::log::progress(format_args!("{line}"));
    }
    runtime.logged_status = Some(status);
    if runtime.sent_name.as_ref() != Some(&name) {
        service.set_name(name.clone());
        runtime.sent_name = Some(name);
    }
    if runtime.sent_chat != Some(chat) {
        service.set_chat(chat);
        runtime.sent_chat = Some(chat);
    }
    if runtime.sent_settings.as_ref() != Some(&settings) {
        service.configure(settings.clone());
        runtime.sent_settings = Some(settings);
    }
    if runtime.sent_location != location {
        match &location {
            Some(location) => service.enter(location.clone()),
            None => service.leave(),
        }
        runtime.sent_location = location;
    }
}

/// How many packs the service has written to the cache folder (`sjk_packs.rs`
/// mounts them again when it changes; 0 before the service starts), and what its
/// last pack check did. Read twice a second.
pub(crate) fn packs() -> (u64, Option<String>) {
    lock().service.as_ref().map_or((0, None), |service| {
        service.with_snapshot(|snapshot| (snapshot.packs_revision, snapshot.assets_note.clone()))
    })
}

/// How many map edit documents the service has changed in their folder
/// ([`crate::map_edits`] reads the loaded map's again when it changes).
pub(crate) fn map_edits_revision() -> u64 {
    lock().service.as_ref().map_or(0, |service| {
        service.with_snapshot(|snapshot| snapshot.map_edits_revision)
    })
}

/// What the service's last look at the hub's map edits did (its summary, or why it fell
/// short), when it is not `seen`: read twice a second, it allocates only on a change.
pub(crate) fn map_edits_note(
    seen: Option<&Result<String, String>>,
) -> Option<Result<String, String>> {
    lock().service.as_ref().and_then(|service| {
        service.with_snapshot(|snapshot| {
            snapshot
                .map_edits_note
                .as_ref()
                .filter(|note| Some(*note) != seen)
                .cloned()
        })
    })
}

/// Have the service look at the hub's map edits now (the editor turned on).
pub(crate) fn refresh_map_edits() {
    if let Some(service) = lock().service.as_ref() {
        service.refresh_map_edits();
    }
}

/// Staff: send a map's edits to the hub; the answer comes from
/// [`map_edits_published`].
pub(crate) fn publish_map_edits(
    map: String,
    checksum: i32,
    edits: serde_json::Value,
) -> Result<(), String> {
    if !sjk_identity::map_edits::valid_map(&map) {
        return Err(format!("The hub takes no edits for a map named {map}"));
    }
    let runtime = lock();
    let service = runtime
        .service
        .as_ref()
        .ok_or("Not connected to the SJK hub: the SJK identity is off (cl_identity 1)")?;
    service.publish_map_edits(map, checksum, edits);
    Ok(())
}

/// The hub's answer to the last map edits sent, once, when it is new.
pub(crate) fn map_edits_published() -> Option<Result<(String, i32, u64), String>> {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let (serial, outcome) = runtime
        .service
        .as_ref()?
        .with_snapshot(|snapshot| snapshot.map_edits_published.clone())?;
    if runtime.map_edits_seen == serial {
        return None;
    }
    runtime.map_edits_seen = serial;
    Some(outcome)
}

/// Send a bug report through the service; false when the service has not started.
pub(crate) fn report(report: sjk_identity::BugReport) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.report(report);
    true
}

/// Have the service send the crash reports waiting in `dir`, or none (`None`).
pub(crate) fn send_crashes(dir: Option<std::path::PathBuf>) {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return;
    };
    if runtime.sent_crash_dir.as_ref() != Some(&dir) {
        service.send_crashes(dir.clone());
        runtime.sent_crash_dir = Some(dir);
    }
}

/// How many crash reports the hub took this session and what the last round did,
/// once a round did anything.
pub(crate) fn crash_outcome() -> Option<(u64, String)> {
    lock().service.as_ref().and_then(|service| {
        service.with_snapshot(|snapshot| {
            snapshot
                .crash_note
                .clone()
                .map(|note| (snapshot.crashes_sent, note))
        })
    })
}

/// Send a world note through the service: its tag for [`note_image`], or `None` when
/// the service has not started.
pub(crate) fn note(note: sjk_identity::WorldNote) -> Option<u64> {
    lock().service.as_ref().map(|service| service.note(note))
}

/// The picture of the note tagged `tag`.
pub(crate) fn note_image(tag: u64, jpeg: Vec<u8>) {
    if let Some(service) = lock().service.as_ref() {
        service.note_image(tag, jpeg);
    }
}

/// The outcome of the last world note, once the service has one.
pub(crate) fn note_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.note.clone()))
}

/// The outcome of the last bug report, once the service has one.
pub(crate) fn report_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.report.clone()))
}

/// Send a report about another player through the service; false when the service
/// has not started.
pub(crate) fn player_report(report: sjk_identity::PlayerReport) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.player_report(report);
    true
}

/// The outcome of the last player report, once the service has one.
pub(crate) fn player_report_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.with_snapshot(|snapshot| snapshot.player_report.clone()))
}

/// Whether the player may report other players: only with the identity on, the hub
/// answering and their key verified by the SJK team.
pub(crate) fn report_gate() -> crate::ingame_menu::players::Gate {
    use crate::ingame_menu::players::Gate;
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return Gate::NoIdentity;
    };
    service.with_snapshot(|snapshot| match (&snapshot.status, &snapshot.me) {
        (sjk_identity::Status::Disabled | sjk_identity::Status::NoHub, _) => Gate::NoIdentity,
        (_, None) => Gate::Offline,
        (_, Some(me)) if me.verified => Gate::Open,
        (_, Some(_)) => Gate::NotVerified,
    })
}

/// What the hub knows of the player in `slot` whom the game shows as `shown`: their
/// key, hub name and verified flag, from a live claim under that name.
pub(crate) fn hub_mark(slot: u8, shown: &str) -> Option<crate::ingame_menu::players::HubMark> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        snapshot
            .badge(slot, shown)
            .map(|player| crate::ingame_menu::players::HubMark {
                key_id: player.key_id.clone(),
                name: player.name.clone(),
                verified: player.verified,
                tick: player.tick,
                trust: player.trust,
                emoji: sjk_identity::name_emoji::for_display(&player.emoji).to_owned(),
                medals: crate::medals::Medals::from_wire(&player.medals),
                avatar: player.avatar.clone(),
            })
    })
}

/// The service's state, or `None` when it has not started (the feature has never
/// been on, or the key file is unusable: see [`key_error`]).
pub(crate) fn snapshot() -> Option<Snapshot> {
    lock().service.as_ref().map(Service::snapshot)
}

/// [`snapshot`] as the screens of what the player holds show it: with every collectible
/// held while `debug_collectibles` (`wanted`) is on for a staff key
/// (`debug_collectibles.rs`).
pub(crate) fn collection_snapshot(wanted: bool) -> Option<Snapshot> {
    snapshot().map(|snapshot| crate::debug_collectibles::dressed(wanted, snapshot))
}

/// Read the identity service's snapshot where it is, without copying it; `None`
/// before the service started.
pub(crate) fn with_snapshot<R>(read: impl FnOnce(&Snapshot) -> R) -> Option<R> {
    lock()
        .service
        .as_ref()
        .map(|service| service.with_snapshot(read))
}

/// Whether the hub vouches for the player (verified or staff): only they start
/// encrypted whispers (`e2e_whispers.rs`).
pub(crate) fn vouched() -> bool {
    with_snapshot(|snapshot| {
        snapshot
            .me
            .as_ref()
            .is_some_and(|me| me.verified || me.staff)
    })
    .unwrap_or(false)
}

/// Whether the hub verified the player's own key (the boombox, `boombox.rs`).
pub(crate) fn verified() -> bool {
    with_snapshot(|snapshot| snapshot.me.as_ref().is_some_and(|me| me.verified)).unwrap_or(false)
}

/// The slots of the players on this server the hub verified, as bits: each one whose
/// claim names the name the game shows there, and the local player's own slot when its
/// key is verified (the boombox, `boombox.rs`).
pub(crate) fn verified_slots(game_state: &GameState) -> u64 {
    with_snapshot(|snapshot| {
        let mut slots = 0_u64;
        for player in snapshot.players.iter().filter(|player| {
            player.verified
                && usize::from(player.slot) < MAX_CLIENTS
                && shown_name(game_state, usize::from(player.slot))
                    .is_some_and(|shown| sjk_identity::names_match(&player.claimed_name, &shown))
        }) {
            slots |= 1 << player.slot;
        }
        if snapshot.me.as_ref().is_some_and(|me| me.verified)
            && let Ok(own) = usize::try_from(game_state.client_num)
            && own < MAX_CLIENTS
        {
            slots |= 1 << own;
        }
        slots
    })
    .unwrap_or(0)
}

/// Why the key file could not be used, if that is why there is no identity.
pub(crate) fn key_error() -> Option<String> {
    lock().key_error.clone()
}

/// Counts changes to the known players, so tags derived from them can be kept.
pub(crate) fn revision() -> u64 {
    lock().service.as_ref().map_or(0, |service| {
        service.with_snapshot(|snapshot| snapshot.revision)
    })
}

/// The tag for the player in `slot` whom the game shows as `shown`, if the hub
/// knows them under that name.
pub(crate) fn tag(slot: u8, shown: &str) -> Option<Tag> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        snapshot.badge(slot, shown).map(|player| Tag {
            verified: player.verified,
            tick: player.tick,
            trust: player.trust,
            medals: crate::medals::Medals::from_wire(&player.medals),
            emoji: NameEmoji::new(&player.emoji),
            client: crate::chat::client_icons::find(&player.client),
        })
    })
}

/// What the hub knows about the player in `slot` whom the game shows as `shown`.
pub(crate) fn hub_info(slot: u8, shown: &str) -> Option<crate::hud::player_card::HubInfo> {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let service = runtime.service.as_ref()?;
    let (info, wanted) = service.with_snapshot(|snapshot| {
        let player = snapshot.badge(slot, shown)?;
        // The player's own profile is the service's; another's is fetched for the card.
        let profile = if player.key_id == snapshot.key_id {
            snapshot.me.as_ref()
        } else {
            snapshot.profiles.get(&player.key_id)
        };
        let info = crate::hud::player_card::HubInfo {
            model: crate::hud::player_card::ModelSight::Unknown,
            name: player.name.clone(),
            verified: player.verified,
            tick: player.tick,
            trust: player.trust,
            medals: crate::medals::Medals::from_wire(&player.medals),
            key_id: player.key_id.clone(),
            avatar: player.avatar.clone(),
            profile: profile.map(crate::hud::player_card::ProfileFacts::of),
            vitals: crate::hud::player_card::Vitals::Private,
            client: crate::chat::client_icons::find(&player.client),
        };
        let wanted = (player.key_id != snapshot.key_id).then(|| player.key_id.clone());
        Some((info, wanted))
    })?;
    let own = wanted.is_none();
    let info = crate::hud::player_card::HubInfo {
        vitals: card_vitals(service, slot, &info.key_id),
        model: if own {
            crate::hud::player_card::ModelSight::Unknown
        } else {
            model_sight(
                service,
                &runtime.own_model,
                &mut runtime.model_asks,
                slot,
                shown,
            )
        },
        ..info
    };
    if let Some(key_id) = wanted {
        let now = Instant::now();
        let due = runtime
            .card_lookups
            .iter()
            .find(|(key, _)| *key == key_id)
            .is_none_or(|(_, asked)| now.duration_since(*asked) >= CARD_LOOKUP_EVERY);
        if due {
            runtime.card_lookups.retain(|(key, _)| *key != key_id);
            if runtime.card_lookups.len() == PICTURE_LOOKUPS {
                runtime.card_lookups.remove(0);
            }
            runtime.card_lookups.push((key_id.clone(), now));
            service.look_up(key_id);
        }
    }
    Some(info)
}

/// How often the player card asks the hub again whether a player has the local
/// player's model.
const MODEL_CHECK_EVERY: Duration = Duration::from_secs(60);
/// Model questions remembered at most.
const MODEL_ASKS: usize = 64;

/// Whether the player in `slot` whom the game shows as `shown` sees the local player's
/// model (`own`), as the hub last answered, while the slot's claim is under that name;
/// asks the hub again when the answer is missing or [`MODEL_CHECK_EVERY`] old.
fn model_sight(
    service: &Service,
    own: &Option<String>,
    asks: &mut Vec<((SocketAddr, u8, String), Instant)>,
    slot: u8,
    shown: &str,
) -> crate::hud::player_card::ModelSight {
    use crate::hud::player_card::ModelSight;
    let Some(model) = own.clone() else {
        return ModelSight::Unknown;
    };
    let (server, answer) = service.with_snapshot(|snapshot| {
        let server = snapshot.server;
        let answer = server
            .and_then(|server| snapshot.model_checks.get(&(server, slot, model.clone())))
            .filter(|check| sjk_identity::names_match(&check.claimed_name, shown))
            .and_then(|check| check.has);
        (server, answer)
    });
    let Some(server) = server else {
        return ModelSight::Unknown;
    };
    let question = (server, slot, model);
    let now = Instant::now();
    let due = asks
        .iter()
        .find(|(asked, _)| *asked == question)
        .is_none_or(|(_, at)| now.duration_since(*at) >= MODEL_CHECK_EVERY);
    if due {
        asks.retain(|(asked, _)| *asked != question);
        if asks.len() == MODEL_ASKS {
            asks.remove(0);
        }
        asks.push((question.clone(), now));
        let (server, slot, model) = question;
        service.check_model(server, slot, model);
    }
    match answer {
        Some(true) => ModelSight::Sees,
        Some(false) => ModelSight::Missing,
        None => ModelSight::Unknown,
    }
}

/// Hand the service the player models the game has (`installed_models.rs`), or an
/// empty list, which makes the hub forget the last one, when the player does not
/// share it (`cl_shareModels` off). Each list goes once.
pub(crate) fn offer_models(models: &std::sync::Arc<crate::installed_models::Models>, share: bool) {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return;
    };
    let offer = (std::sync::Arc::as_ptr(models) as usize, share);
    if runtime.sent_models != Some(offer) {
        service.set_models(if share {
            models.all.clone()
        } else {
            Vec::new()
        });
        runtime.sent_models = Some(offer);
    }
}

/// The model folder the local player wears, or `None` when every player has it (a
/// retail model) or there is none: the player card's model line asks about it.
pub(crate) fn set_own_model(model: Option<String>) {
    lock().own_model = model;
}

/// The model folder the local player wears ([`set_own_model`]), for `sharemodel`.
pub(crate) fn own_model_folder() -> Option<String> {
    lock().own_model.clone()
}

/// Ask the hub for a model sync upload ticket; false when the service has not started.
pub(crate) fn ask_model_ticket(model: String, md5: String) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.ask_model_ticket(model, md5);
    true
}

/// Ask the hub for a staff rollback ticket; false when the service has not started.
pub(crate) fn ask_model_rollback_ticket(model: String, version: u32) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.ask_model_rollback_ticket(model, version);
    true
}

/// The last model sync ticket asked for, once the service has one.
pub(crate) fn model_ticket() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(|service| service.model_ticket())
}

/// How often the player card asks the hub again for a player's profile.
const CARD_LOOKUP_EVERY: Duration = Duration::from_secs(600);

/// Counts profiles fetched for other players, so the player card reads them again.
pub(crate) fn profiles_revision() -> u64 {
    lock().service.as_ref().map_or(0, |service| {
        service.with_snapshot(|snapshot| snapshot.profiles_revision)
    })
}

/// This PC's key id (the pop-up's seen list is kept per key, whatever person the key
/// belongs to) and the medals the player's profile lists, once the hub has answered; `None` before that or with the feature off. Read twice a second at most,
/// for the new medal pop-up.
pub(crate) fn own_medals() -> Option<(String, Vec<sjk_identity::Medal>)> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        let me = snapshot.me.as_ref()?;
        if matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        ) {
            return None;
        }
        Some((snapshot.local_key_id.clone(), me.medals.clone()))
    })
}

/// The slots of the players the hub's operator vouches for on this server, as bits:
/// each one whose claim names the name the game shows there, and the local player's
/// own slot when its key is verified. For the nameplates' verified badge.
pub(crate) fn slot_marks(game_state: &GameState) -> SlotMarks {
    let mut marks = [None; MAX_CLIENTS];
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return marks;
    };
    service.with_snapshot(|snapshot| {
        for player in snapshot
            .players
            .iter()
            .filter(|player| usize::from(player.slot) < MAX_CLIENTS)
            .filter(|player| {
                shown_name(game_state, usize::from(player.slot))
                    .is_some_and(|shown| sjk_identity::names_match(&player.claimed_name, &shown))
            })
        {
            marks[usize::from(player.slot)] =
                crate::ui_renderer::mark_texture(player.verified, player.tick, player.trust);
        }
        if let Some(me) = snapshot.me.as_ref()
            && let Ok(own) = usize::try_from(game_state.client_num)
            && own < MAX_CLIENTS
        {
            marks[own] = crate::ui_renderer::mark_texture(me.verified, me.tick, me.trust);
        }
    });
    marks
}

/// Each slot's mark after its nameplate's name ([`slot_marks`]).
pub(crate) type SlotMarks = [Option<sjk_ui::TextureId>; MAX_CLIENTS];

/// The icon of each slot's client, small after its nameplate's mark
/// (`chat::client_icons::plate_texture`): the client the hub's claim for the slot names
/// (presence `client`), none for SJK; for a slot no claim names and not a bot's, the
/// client players who read SJK chat without claiming are taken to be on, where one
/// such reader seems to sit (`scoreboard::chat_readers`, the scoreboard's rule: a name
/// proves nothing, so the icon comes alone, without a tick). The player's own slot is
/// SJK's.
pub(crate) fn slot_clients(game_state: &GameState) -> SlotMarks {
    let mut clients = [None; MAX_CLIENTS];
    let own = usize::try_from(game_state.client_num).ok();
    let mut rows: Vec<(u8, String, bool)> = (0..MAX_CLIENTS)
        .filter(|&slot| {
            game_state
                .config_string(CS_PLAYERS + slot)
                .is_some_and(|bytes| LegacyClientInfo::new(bytes).bytes("skill").is_none())
        })
        .filter_map(|slot| {
            let name = shown_name(game_state, slot)?;
            Some((u8::try_from(slot).ok()?, name, Some(slot) == own))
        })
        .collect();
    with_snapshot(|snapshot| {
        for (slot, name, claimed) in &mut rows {
            if let Some(player) = snapshot.badge(*slot, name) {
                *claimed = true;
                clients[usize::from(*slot)] =
                    crate::chat::client_icons::plate_texture(&player.client);
            }
        }
    });
    if let Some(icon) = crate::chat::client_icons::unclaimed() {
        let shown: Vec<(u8, &str, bool)> = rows
            .iter()
            .map(|(slot, name, claimed)| (*slot, name.as_str(), *claimed))
            .collect();
        let slots = crate::scoreboard::chat_readers::slots(&shown);
        for (slot, client) in clients.iter_mut().enumerate() {
            if slot < 64 && slots & 1 << slot != 0 {
                *client = Some(icon.texture);
            }
        }
    }
    clients
}

/// The pictures of the SJK players on this server, by slot, for their nameplates: each
/// one whose claim names the name the game shows there, and the local player's own slot
/// from their profile. A slot the hub knows nobody in is `None`: its nameplate shows the
/// model's head icon.
pub(crate) fn slot_pictures(game_state: &GameState) -> crate::hud::nameplate::SlotPictures {
    use crate::hud::nameplate::SlotPicture;
    let mut pictures = [None; MAX_CLIENTS];
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return pictures;
    };
    service.with_snapshot(|snapshot| {
        for player in snapshot
            .players
            .iter()
            .filter(|player| usize::from(player.slot) < MAX_CLIENTS)
            .filter(|player| {
                shown_name(game_state, usize::from(player.slot))
                    .is_some_and(|shown| sjk_identity::names_match(&player.claimed_name, &shown))
            })
        {
            pictures[usize::from(player.slot)] =
                SlotPicture::new(&player.key_id, &player.avatar, &player.name);
        }
        if let Some(me) = snapshot.me.as_ref()
            && let Ok(own) = usize::try_from(game_state.client_num)
            && own < MAX_CLIENTS
        {
            pictures[own] = SlotPicture::new(&snapshot.key_id, &me.avatar, &me.name);
        }
    });
    pictures
}

/// How long the hub's list of sharers stays good without a new answer: a little over
/// the life of its entries.
const SHARED_FRESH: Duration = Duration::from_secs(6);

/// Tell the hub what the player shares on their nameplate (`PROTOCOL.md`, "Vitals").
pub(crate) fn share_vitals(sharing: sjk_identity::Sharing) {
    if let Some(service) = lock().service.as_ref() {
        service.set_sharing(sharing);
    }
}

/// The other players' shared vitals by slot, each only while their claimed name
/// matches the name the game shows there; the player's own slot is left out.
pub(crate) fn shared_vitals(
    game_state: &GameState,
) -> [Option<crate::hud::nameplate::SharedSlot>; MAX_CLIENTS] {
    let mut slots = [None; MAX_CLIENTS];
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return slots;
    };
    let own = usize::try_from(game_state.client_num).ok();
    service.with_vitals(|table| {
        if table
            .read_at
            .is_none_or(|read| read.elapsed() > SHARED_FRESH)
        {
            return;
        }
        for player in &table.players {
            let slot = usize::from(player.slot);
            if slot >= MAX_CLIENTS || Some(slot) == own || !player.share {
                continue;
            }
            let shown = shown_name(game_state, slot);
            if shown.is_some_and(|shown| sjk_identity::names_match(&player.claimed_name, &shown)) {
                slots[slot] = Some(crate::hud::nameplate::SharedSlot {
                    estimates: player.estimates,
                    vitals: player.vitals,
                });
            }
        }
    });
    slots
}

/// Whether the hub has the player's own sharing: its last answer, still fresh, lists
/// them in their slot as sharing. Others' shared and estimated bars show only then.
pub(crate) fn sharing_live(game_state: &GameState) -> bool {
    let runtime = lock();
    let (Some(service), Ok(own)) = (
        runtime.service.as_ref(),
        u8::try_from(game_state.client_num),
    ) else {
        return false;
    };
    service.with_vitals(|table| {
        table
            .read_at
            .is_some_and(|read| read.elapsed() <= SHARED_FRESH)
            && table
                .players
                .iter()
                .any(|player| player.slot == own && player.share)
    })
}

/// Two bits a slot of what each player shares (shares, sees everyone's bars), so the
/// player card is rebuilt when they change.
pub(crate) fn vitals_flags(game_state: &GameState) -> u64 {
    shared_vitals(game_state)
        .iter()
        .enumerate()
        .fold(0, |bits, (slot, shared)| match shared {
            Some(shared) => bits | (1 | u64::from(shared.estimates) << 1) << (slot * 2),
            None => bits,
        })
}

/// What the player of `key_id` in `slot` shares, as the player card says it.
fn card_vitals(service: &Service, slot: u8, key_id: &str) -> crate::hud::player_card::Vitals {
    use crate::hud::player_card::Vitals;
    service.with_vitals(|table| {
        let fresh = table
            .read_at
            .is_some_and(|read| read.elapsed() <= SHARED_FRESH);
        match table
            .players
            .iter()
            .find(|player| fresh && player.share && player.slot == slot && player.key_id == key_id)
        {
            Some(player) if player.estimates => Vitals::SeesAll,
            Some(_) => Vitals::Shared,
            None => Vitals::Private,
        }
    })
}

/// Ask the hub to change the player's bio. The result shows in
/// [`Snapshot::notice`].
pub(crate) fn set_bio(bio: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_bio(bio))
        .is_some()
}

/// Ask the hub to change the player's name emoji (empty takes it away); false when
/// the service has not started. The result shows in [`Snapshot::notice`].
pub(crate) fn set_emoji(emoji: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_emoji(emoji))
        .is_some()
}

/// Ask the hub to make `name` the player's custom SJK name; false when the service has
/// not started. The result shows in [`Snapshot::notice`].
pub(crate) fn set_custom_name(name: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_custom_name(name))
        .is_some()
}

/// Ask the hub to make `png` the player's picture; false when the service has not
/// started (the identity is off). The outcome shows in [`Snapshot::avatar`].
pub(crate) fn set_avatar(png: Vec<u8>) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_avatar(png))
        .is_some()
}

/// Ask the hub to take the player's picture down; false when the service has not
/// started.
pub(crate) fn remove_avatar() -> bool {
    lock()
        .service
        .as_ref()
        .map(Service::remove_avatar)
        .is_some()
}

/// Give the service the counts the client keeps for its achievements (`achievements.rs`),
/// to send to the hub; false when the service has not started (the identity is off).
pub(crate) fn set_achievement_counts(counts: &std::collections::BTreeMap<String, u64>) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.set_achievement_counts(counts.clone()))
        .is_some()
}

/// The achievements the hub holds for the player, once their profile came; `None`
/// before that or with the feature off.
pub(crate) fn own_achievements() -> Option<Vec<sjk_identity::Achievement>> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        if matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        ) {
            return None;
        }
        snapshot.me.as_ref().map(|me| me.achievements.clone())
    })
}

/// Whether the player's achievements go to an SJK hub: the identity is on with a hub
/// address (whether it answers right now or not). The hub then has the last word on
/// the achievements the client counts, and grants their kyber crystals
/// (`achievements.rs`).
pub(crate) fn hub_on() -> bool {
    let runtime = lock();
    runtime.service.is_some()
        && runtime
            .sent_settings
            .as_ref()
            .is_some_and(|settings| settings.enabled && !settings.hub_url.trim().is_empty())
}

/// Whether the hub says the player's own key is staff.
pub(crate) fn is_staff() -> bool {
    lock().service.as_ref().is_some_and(|service| {
        service.with_snapshot(|snapshot| snapshot.me.as_ref().is_some_and(|me| me.staff))
    })
}

/// Whether the player's own key is SJK's creator, as the hub's profile says.
pub(crate) fn is_creator() -> bool {
    lock().service.as_ref().is_some_and(|service| {
        service.with_snapshot(|snapshot| snapshot.me.as_ref().is_some_and(|me| me.creator))
    })
}

/// Whether the hub vouches for the player's own key (verified or staff): such a
/// player may ban unverified players from the SJK chat ([`crate::chat_ban`]).
pub(crate) fn can_ban() -> bool {
    lock().service.as_ref().is_some_and(|service| {
        service.with_snapshot(|snapshot| {
            snapshot
                .me
                .as_ref()
                .is_some_and(|me| me.verified || me.staff)
        })
    })
}

/// Request the desired reaction state; false when the service has not started.
/// The result arrives in the chat snapshot's separate reaction outcome.
pub(crate) fn react(message: u64, emoji: String, active: bool) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.react(message, emoji, active))
        .is_some()
}

/// Request a chat ban for another key.
pub(crate) fn chat_ban(key_id: String) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.chat_ban(key_id))
        .is_some()
}

/// The player's own id at the hub, once the service started: this PC's key id, or the
/// id of the person it is linked to once the hub said so (`Snapshot::key_id`).
pub(crate) fn own_key_id() -> Option<String> {
    lock()
        .service
        .as_ref()
        .map(|service| service.with_snapshot(|snapshot| snapshot.key_id.clone()))
}

/// Whether `key_id` is another player's, not this player's own. Key ids are never
/// drawn in the game, the player's own or anyone else's (Sol's rule, 26/10/2026), so a
/// stream never shows one; this only tells whose a key is.
pub(crate) fn is_others_key(key_id: &str) -> bool {
    others_key(key_id, own_key_id().as_deref())
}

/// Whether `key_id` is not `own`.
fn others_key(key_id: &str, own: Option<&str>) -> bool {
    own != Some(key_id)
}

/// This PC's key id (for the pop-up's seen list, kept per key) and what the player's
/// profile holds of holocrons (their counts and
/// their recent list, newest first), read in place; `None` before the hub answered or
/// with the feature off. Keep `read` short: the service waits.
pub(crate) fn with_own_holocrons<R>(
    read: impl FnOnce(&str, &sjk_identity::HolocronCounts, &[sjk_identity::Holocron]) -> R,
) -> Option<R> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        let me = snapshot.me.as_ref()?;
        if matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        ) {
            return None;
        }
        Some(read(
            &snapshot.local_key_id,
            &me.holocron_counts,
            &me.holocrons,
        ))
    })
}

/// Ask the hub to open the player's holocrons `ids` (at most
/// [`sjk_identity::OPEN_MAX`] are taken); false when the service has not started. What
/// they brought arrives in [`opening_outcome`].
pub(crate) fn open_holocrons(ids: Vec<u64>) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.open_holocrons(ids))
        .is_some()
}

/// Ask the hub to reveal the player's sealed cards; false when the service has not
/// started. What they became arrives in [`reveal_outcome`].
pub(crate) fn reveal_cards() -> bool {
    lock()
        .service
        .as_ref()
        .map(sjk_identity::Service::reveal_cards)
        .is_some()
}

/// Run `act` on the identity service; `None` before it started (the economy's requests,
/// `crate::economy`, and the shop's).
pub(crate) fn with_service<R>(act: impl FnOnce(&Service) -> R) -> Option<R> {
    lock().service.as_ref().map(act)
}

/// Ask the hub to sell today's offer `id` for credits; false when the service has not
/// started. What came of it arrives in the snapshot's `trade`.
pub(crate) fn buy(id: &str) -> bool {
    with_service(|service| service.buy(id)).is_some()
}

/// Ask the hub for the shop's offers of the day; false when the service has not
/// started. They arrive in the snapshot's `shop`.
pub(crate) fn read_shop() -> bool {
    with_service(Service::read_shop).is_some()
}

/// Ask the hub for a page of the player's history: the newest (`None`) or the one before
/// `before`; false when the service has not started. It arrives in the snapshot's
/// `history`.
pub(crate) fn read_history(before: Option<i64>) -> bool {
    with_service(|service| service.read_history(before)).is_some()
}

/// What became of the last reveal of sealed cards, if the service has started.
pub(crate) fn reveal_outcome() -> Option<sjk_identity::RevealOutcome> {
    with_snapshot(|snapshot| snapshot.reveal.clone()).flatten()
}

/// What became of the last opening of holocrons, if the service has started.
pub(crate) fn opening_outcome() -> Option<sjk_identity::OpenOutcome> {
    with_snapshot(|snapshot| snapshot.opening.clone()).flatten()
}

/// Ask the service for fresh holocron progress (it reads at most every 30 seconds).
pub(crate) fn refresh_holocrons() {
    if let Some(service) = lock().service.as_ref() {
        service.refresh_holocrons();
    }
}

/// Hand the service what the player did lately (`holocrons/signals.rs`), for the next
/// active claim; dropped while the service has not started.
pub(crate) fn add_play(play: sjk_identity::PlaySignals) {
    if let Some(service) = lock().service.as_ref() {
        service.add_play(play);
    }
}

/// Break one tag copy down into kyber shards (hubs whose economy says `tag_copies`).
pub(crate) fn shatter_tag(copy: sjk_identity::TagCopy) {
    if let Some(service) = lock().service.as_ref() {
        service.shatter_tag(copy);
    }
}

/// Give two tag copies of one tier and finish for a tag not held yet.
pub(crate) fn craft_tags(copies: [sjk_identity::TagCopy; 2]) {
    if let Some(service) = lock().service.as_ref() {
        service.craft_tags(copies);
    }
}

/// Spend a blank tag on GIPHY GIF `giphy` for a GIF tag (verified players).
pub(crate) fn make_gif_tag(giphy: String) {
    if let Some(service) = lock().service.as_ref() {
        service.make_gif_tag(giphy);
    }
}

/// Tell the service whether the player is actively playing, when that is not what it
/// was last told; false when the service has not started.
pub(crate) fn set_active(active: bool) -> bool {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    if newly(&mut runtime.sent_active, &active) {
        service.set_active(active);
    }
    true
}

/// Follow whether the game menu is open (each frame): as it opens, the service reads
/// what its pages show of the hub again (it keeps its own cooldown).
pub(crate) fn follow_game_menu(open: bool) {
    let mut runtime = lock();
    if open
        && !runtime.game_menu_open
        && let Some(service) = runtime.service.as_ref()
    {
        service.menu_opened();
    }
    runtime.game_menu_open = open;
}

/// The players SJK staff blocked for everyone on the server being played, until its map
/// changes (`map_blocks.rs`), as the hub's presence last gave them.
pub(crate) fn map_blocks() -> Vec<sjk_identity::MapBlock> {
    lock().service.as_ref().map_or_else(Vec::new, |service| {
        service.with_snapshot(|snapshot| snapshot.map_blocks.clone())
    })
}

/// The maps on which SJK's creator blocked server remaps for every SJK client
/// (`remap_blocked_maps.rs`), with their revision, when it is not `seen`: read twice a
/// second, it allocates only on a change. `None` also before any hub answered.
pub(crate) fn remap_blocks(seen: u64) -> Option<(u64, Vec<String>)> {
    lock().service.as_ref()?.with_snapshot(|snapshot| {
        (snapshot.remap_blocks_revision != seen)
            .then(|| snapshot.remap_blocks.clone())
            .flatten()
            .map(|maps| (snapshot.remap_blocks_revision, maps))
    })
}

/// The game server the hub sees this player on (their claim's), if any.
pub(crate) fn claimed_server() -> Option<std::net::SocketAddr> {
    lock()
        .service
        .as_ref()?
        .with_snapshot(|snapshot| snapshot.server)
}

/// Send a staff request through the service; false when the service has not started.
pub(crate) fn staff(request: sjk_identity::StaffRequest) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.staff(request))
        .is_some()
}

/// Send a staff query about logs or crash reports; false when the service has not
/// started.
pub(crate) fn staff_query(query: sjk_identity::StaffQuery) -> bool {
    lock()
        .service
        .as_ref()
        .map(|service| service.staff_query(query))
        .is_some()
}

/// The logs or crash report a staff query opened, once.
pub(crate) fn take_opened() -> Option<sjk_identity::OpenedText> {
    lock()
        .service
        .as_ref()
        .and_then(sjk_identity::Service::take_opened)
}

/// Answer the SJK team's log request `id`: send `upload`, or decline with `None`.
pub(crate) fn answer_logs(id: i64, upload: Option<sjk_identity::LogUpload>) {
    if let Some(service) = lock().service.as_ref() {
        service.answer_logs(id, upload);
    }
}

/// What staff requests brought back, once the service started.
pub(crate) fn staff_state() -> Option<sjk_identity::StaffState> {
    lock()
        .service
        .as_ref()
        .map(sjk_identity::Service::staff_state)
}

/// Send an SJK chat message; false when the service has not started. What became of
/// it arrives in the chat's `outcome`. In a match, the game server's chat gets it too
/// (`chat/server_copy.rs`).
pub(crate) fn chat(text: String) -> bool {
    send_chat(text, true)
}

/// [`chat`], but never said in the game server's chat: the SJK chat page's Send only in
/// SJK, and staff's Shift+Enter. [`chat`] keeps a message with a GIF out of it too (`docs/mods.md`, "SJK chat on JoF servers").
pub(crate) fn chat_sjk_only(text: String) -> bool {
    send_chat(text, false)
}

/// Whether an SJK chat message may be said in the game server's chat too where a mod
/// says SJK chat there: not one with a GIF, whose link would only spam the server's
/// other players (Sol, 22/10/2026).
fn may_go_to_server(text: &str) -> bool {
    crate::chat_gifs::link::find(text).is_none()
}

fn send_chat(text: String, server_too: bool) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    if server_too && may_go_to_server(&text) {
        let outcome = service.with_chat(|chat| chat.outcome.as_ref().map_or(0, |o| o.serial));
        crate::chat::server_copy::note_said(&text, outcome);
    }
    service.chat(text);
    true
}

/// Send a message to a clan chat (`chat/tabs.rs`); false when the service has not
/// started. What became of it arrives in the chat's `outcome`. It never goes to the game
/// server.
pub(crate) fn chat_clan(text: String, channel: sjk_identity::HubChannel) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.clan_chat(text, channel);
    true
}

/// Which clan chats the player reads, JoF's then S's: the ones the hub lists for them
/// that `choice` (their join settings, `chat::tabs::join_choice`) keeps, telling the
/// service when that changes. Called every frame; it allocates only on a change.
pub(crate) fn follow_clan_chats(
    choice: impl Fn(sjk_identity::HubChannel) -> Option<bool>,
) -> [bool; 2] {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return [false; 2];
    };
    let joined = service.with_chat(|chat| {
        sjk_identity::channels::CLANS.map(|clan| {
            chat.channels
                .iter()
                .any(|listed| listed.id == clan && choice(clan).unwrap_or(listed.home))
        })
    });
    if joined != runtime.sent_clans {
        runtime.sent_clans = joined;
        let wanted = sjk_identity::channels::CLANS
            .into_iter()
            .zip(joined)
            .filter_map(|(clan, on)| on.then_some(clan))
            .collect();
        service.set_clan_chats(wanted);
    }
    joined
}

/// Play an emote for the player's slot; false when the service has not started.
pub(crate) fn emote(id: String, sync: Option<u8>) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.emote(id, sync);
    note_emote_post();
    true
}

/// Play an emote as [`emote`] does, without its sound for the others (the player is
/// away, `away_emote.rs`); false when the service has not started.
pub(crate) fn emote_quiet(id: String) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.emote_quiet(id);
    note_emote_post();
    true
}

/// The hub takes one emote every 2 seconds from a key (`PROTOCOL.md`, "Emotes"), and
/// a little more for the clock's rounding.
const EMOTE_GAP: Duration = Duration::from_millis(2_100);
static LAST_EMOTE_POST: Mutex<Option<Instant>> = Mutex::new(None);

fn note_emote_post() {
    *LAST_EMOTE_POST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Instant::now());
}

/// Whether the hub would take an emote from the player at `now`, as far as this
/// client's own posts go.
pub(crate) fn emote_ready(now: Instant) -> bool {
    LAST_EMOTE_POST
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_none_or(|last| now.saturating_duration_since(last) >= EMOTE_GAP)
}

/// Ping `pos` (whole game units) for `scope` from the player's slot
/// (`world_ping.rs`); false when the service has not started. What became of it
/// arrives in [`ping_outcome`].
pub(crate) fn ping(pos: [i32; 3], scope: &str) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.ping(pos, scope.to_owned());
    true
}

/// What became of the player's last ping; `None` before one, or when the service has
/// not started.
pub(crate) fn ping_outcome() -> Option<sjk_identity::ReportOutcome> {
    lock()
        .service
        .as_ref()
        .and_then(sjk_identity::Service::ping_outcome)
}

/// The pings the feed received for `server` since the last call (none for another).
pub(crate) fn take_pings(server: Option<SocketAddr>) -> Vec<sjk_identity::Ping> {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_pings(server))
        .unwrap_or_default()
}

/// Tell the SJK players on the server that the player's toy `toy` did `event` to the
/// player in slot `target` (`PROTOCOL.md`, "Toy events"); false when the service has
/// not started.
pub(crate) fn toy_event(toy: &str, event: &str, target: u8) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.toy_event(toy.to_owned(), event.to_owned(), target);
    true
}

/// Relay `event` of the player's toy `toy` to the player in slot `target` at once
/// (`PROTOCOL.md`, "Toy relay"); `replace` lets a later one take its place while it
/// waits. False when the service has not started.
pub(crate) fn relay_toy_event(toy: &str, event: String, target: u8, replace: bool) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.relay_toy_event(toy.to_owned(), event, target, replace);
    true
}

/// Whether the toy relay reads the events meant for the player.
pub(crate) fn set_relay_listen(listen: bool) {
    if let Some(service) = lock().service.as_ref() {
        service.set_relay_listen(listen);
    }
}

/// Whether the player takes encrypted whispers (`e2e_whispers.rs`).
pub(crate) fn set_whisper_listen(listen: bool) {
    if let Some(service) = lock().service.as_ref() {
        service.set_whisper_listen(listen);
    }
}

/// Seal and send an encrypted whisper; false when the service has not started.
pub(crate) fn send_whisper(whisper: sjk_identity::whisper::Outgoing) -> bool {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    service.send_whisper(whisper);
    true
}

/// What became of the encrypted whispers sent since the last call.
pub(crate) fn take_whispers_sent() -> Vec<sjk_identity::whisper::Sent> {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_whispers_sent())
        .unwrap_or_default()
}

/// The encrypted whispers read for `server` since the last call.
pub(crate) fn take_whispers(server: Option<SocketAddr>) -> Vec<sjk_identity::whisper::Received> {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_whispers(server))
        .unwrap_or_default()
}

/// The relayed toy events read for `server` since the last call.
pub(crate) fn take_relay_events(server: Option<SocketAddr>) -> Vec<sjk_identity::ToyEvent> {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_relay_events(server))
        .unwrap_or_default()
}

/// The toy events received for `server`, the game server the player is on, since the
/// last call; none when the service has not started.
pub(crate) fn take_toy_events(server: Option<SocketAddr>) -> Vec<sjk_identity::ToyEvent> {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_toy_events(server))
        .unwrap_or_default()
}

/// Read the SJK chat; `None` when the service has not started. Keep `read` short.
pub(crate) fn with_chat<R>(read: impl FnOnce(&sjk_identity::ChatState) -> R) -> Option<R> {
    lock()
        .service
        .as_ref()
        .map(|service| service.with_chat(read))
}

/// The emotes received since the last call.
pub(crate) fn take_emotes() -> Vec<sjk_identity::Emote> {
    lock()
        .service
        .as_ref()
        .map(Service::take_emotes)
        .unwrap_or_default()
}

/// The looks the feed received since the last call for `server`, the game server the
/// player is on, under the feed's current reading (`Service::take_looks`).
pub(crate) fn take_looks(server: Option<SocketAddr>) -> sjk_identity::ReceivedLooks {
    lock()
        .service
        .as_ref()
        .map(|service| service.take_looks(server))
        .unwrap_or_default()
}

/// Read the known players on the server (their looks); `None` when the service has
/// not started. Keep `read` short.
pub(crate) fn with_roster<R>(read: impl FnOnce(&[sjk_identity::Presence]) -> R) -> Option<R> {
    lock()
        .service
        .as_ref()
        .map(|service| service.with_snapshot(|snapshot| read(&snapshot.players)))
}

/// Whether the player's own hub profile lists unlock `id`: false with the identity
/// off, no hub or before the hub answered.
pub(crate) fn owns_unlock(id: &str) -> bool {
    lock().service.as_ref().is_some_and(|service| {
        service.with_snapshot(|snapshot| {
            !matches!(
                snapshot.status,
                sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
            ) && snapshot
                .me
                .as_ref()
                .is_some_and(|me| me.unlocks.iter().any(|unlock| unlock.id == id))
        })
    })
}

/// What the player's own hub profile holds of tag or card `id`, by finish
/// ([`crate::tags::Held::of`]), by the same rule as [`owns_unlock`]: only the free tags
/// with the identity off, no hub or before the hub answered.
pub(crate) fn tag_held(id: &str) -> crate::tags::Held {
    with_snapshot(|snapshot| {
        let known = !matches!(
            snapshot.status,
            sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
        );
        crate::tags::Held::of(id, snapshot.me.as_ref().filter(|_| known))
    })
    .unwrap_or_else(|| crate::tags::Held::of(id, None))
}

/// Whether the player's own hub profile lists a revealed copy of `gate`'s card in its
/// finish (`blade_extras.rs`): false with the identity off, no hub or before the hub
/// answered.
pub(crate) fn holds_card(gate: &crate::blade_extras::Gate) -> bool {
    lock().service.as_ref().is_some_and(|service| {
        service.with_snapshot(|snapshot| {
            !matches!(
                snapshot.status,
                sjk_identity::Status::Disabled | sjk_identity::Status::NoHub
            ) && snapshot
                .me
                .as_ref()
                .is_some_and(|me| crate::blade_extras::holds(&me.cards, gate))
        })
    })
}

/// Give the service the look the player wears, when it is not the one it was last
/// given; false when the service has not started.
pub(crate) fn set_look(look: &sjk_identity::Look) -> bool {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let Some(service) = runtime.service.as_ref() else {
        return false;
    };
    if newly(&mut runtime.sent_look, look) {
        service.set_look(look.clone());
    }
    true
}

/// Whether `value` is not the one last `sent`, which then becomes it.
fn newly<T: Clone + PartialEq>(sent: &mut Option<T>, value: &T) -> bool {
    if sent.as_ref() == Some(value) {
        return false;
    }
    *sent = Some(value.clone());
    true
}

/// Read the hub's claims on the server being played (none when the service has not
/// started), as the mute list matches them to slots (`chat_mutes.rs`). Keep `read`
/// short: the service waits.
pub(crate) fn with_claims<R>(read: impl FnOnce(&[crate::chat_mutes::Claim<'_>]) -> R) -> R {
    let runtime = lock();
    let Some(service) = runtime.service.as_ref() else {
        return read(&[]);
    };
    service.with_snapshot(|snapshot| {
        let claims: Vec<_> = snapshot
            .players
            .iter()
            .map(|player| crate::chat_mutes::Claim {
                slot: player.slot,
                claimed_name: &player.claimed_name,
                key_id: &player.key_id,
            })
            .collect();
        read(&claims)
    })
}

/// Profiles [`avatar_version`] asks for at most, the oldest forgotten first.
const PICTURE_LOOKUPS: usize = 64;

/// The version of the picture of the player with key `key_id` (empty for none), for a
/// card about them (`sender_card.rs`): from the hub's players on this server, else a
/// profile fetched for them. `None` while neither is known: the first such call asks the
/// hub for their profile, once. Locks the identity: not for a caller holding it.
pub(crate) fn avatar_version(key_id: &str) -> Option<String> {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let service = runtime.service.as_ref()?;
    let known = service.with_snapshot(|snapshot| {
        snapshot
            .players
            .iter()
            .find(|player| player.key_id == key_id)
            .map(|player| player.avatar.clone())
            .or_else(|| {
                snapshot
                    .profiles
                    .get(key_id)
                    .map(|profile| profile.avatar.clone())
            })
    });
    if known.is_none() && !runtime.picture_lookups.iter().any(|key| key == key_id) {
        if runtime.picture_lookups.len() == PICTURE_LOOKUPS {
            runtime.picture_lookups.remove(0);
        }
        runtime.picture_lookups.push(key_id.to_owned());
        service.look_up(key_id.to_owned());
    }
    known
}

/// Avatar from presence first, with a static banner from the cached profile.
/// Unknown profiles are requested once without delaying a known presence avatar.
pub(crate) fn card_appearance(key_id: &str) -> Option<(String, &'static str)> {
    let mut runtime = lock();
    let runtime = &mut *runtime;
    let service = runtime.service.as_ref()?;
    let (known, has_profile) = service.with_snapshot(|snapshot| {
        let profile = snapshot
            .me
            .as_ref()
            .filter(|profile| profile.key_id == key_id)
            .or_else(|| snapshot.profiles.get(key_id));
        let presence = snapshot
            .players
            .iter()
            .find(|player| player.key_id == key_id);
        (
            appearance(presence.map(|player| player.avatar.as_str()), profile),
            profile.is_some(),
        )
    });
    if !has_profile && !runtime.picture_lookups.iter().any(|key| key == key_id) {
        if runtime.picture_lookups.len() == PICTURE_LOOKUPS {
            runtime.picture_lookups.remove(0);
        }
        runtime.picture_lookups.push(key_id.to_owned());
        service.look_up(key_id.to_owned());
    }
    known
}

fn appearance(
    presence_avatar: Option<&str>,
    profile: Option<&sjk_identity::Profile>,
) -> Option<(String, &'static str)> {
    presence_avatar
        .or_else(|| profile.map(|profile| profile.avatar.as_str()))
        .map(|avatar| {
            (
                avatar.to_owned(),
                crate::profile_card::banner::equipped(profile),
            )
        })
}

/// Ask the hub for another player's profile (their bio).
pub(crate) fn look_up(key_id: &str) {
    if let Some(service) = lock().service.as_ref() {
        service.look_up(key_id.to_owned());
    }
}

/// Withdraw the player's claim on the way out.
pub(crate) fn shutdown() {
    // Not under the lock: the wait can last as long as the hub takes to answer.
    let service = lock().service.take();
    if let Some(service) = service {
        service.shutdown(SHUTDOWN_WAIT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_presence_avatar_is_available_without_a_profile_and_overrides_stale_cache() {
        assert_eq!(
            appearance(Some("current"), None),
            Some(("current".into(), ""))
        );
        assert_eq!(appearance(Some(""), None), Some(("".into(), "")));
        assert_eq!(appearance(None, None), None);
        let profile: sjk_identity::Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Player","bio":"","verified":false,"created":5,"avatar":"cached"}"#,
        )
        .unwrap();
        assert_eq!(
            appearance(Some("current"), Some(&profile)),
            Some(("current".into(), ""))
        );
        assert_eq!(
            appearance(None, Some(&profile)),
            Some(("cached".into(), ""))
        );
    }

    #[test]
    fn the_log_tells_each_change_of_the_hub_status_once() {
        use sjk_identity::Status;
        let registering = log_status(None, &Status::Registering);
        assert_eq!(
            registering.as_deref(),
            Some("identity: registering with the hub")
        );
        assert_eq!(
            log_status(Some(&Status::Registering), &Status::Registering),
            None
        );
        let refused = Status::Failed("the hub refused it (rate_limited): slow down".to_owned());
        assert_eq!(
            log_status(Some(&Status::Registering), &refused).as_deref(),
            Some("identity: hub request failed: the hub refused it (rate_limited): slow down")
        );
        assert_eq!(log_status(Some(&refused), &refused), None);
        assert_eq!(
            log_status(Some(&refused), &Status::Online).as_deref(),
            Some("identity: registered with the hub")
        );
    }

    #[test]
    fn a_message_with_a_gif_is_never_said_on_the_server() {
        assert!(may_go_to_server("gg wp"));
        assert!(!may_go_to_server(
            "hello https://giphy.com/gifs/hello-wave-ShotGifWide01"
        ));
        assert!(!may_go_to_server(
            "https://i.giphy.com/ShotGifSquare02.gif nice"
        ));
    }

    #[test]
    fn a_key_is_the_players_own_only_when_it_matches() {
        let own = Some("0123456789abcdef");
        assert!(!others_key("0123456789abcdef", own));
        assert!(others_key("fedcba9876543210", own));
        // Before the identity started every key is another's.
        assert!(others_key("0123456789abcdef", None));
    }

    #[test]
    fn the_tag_is_none_before_the_service_starts() {
        assert_eq!(tag(3, "Sol"), None);
        assert_eq!(revision(), 0);
        assert!(snapshot().is_none());
        assert_eq!(slot_marks(&GameState::empty_local(0)), [None; MAX_CLIENTS]);
        assert_eq!(
            slot_clients(&GameState::empty_local(0)),
            [None; MAX_CLIENTS]
        );
        assert_eq!(report_gate(), crate::ingame_menu::players::Gate::NoIdentity);
        assert_eq!(hub_mark(3, "Sol"), None);
        assert!(player_report_outcome().is_none());
        assert!(own_medals().is_none());
        assert!(with_own_holocrons(|_, _, _| ()).is_none());
        assert!(!set_active(true));
        assert!(own_achievements().is_none());
        assert!(!is_staff());
        assert!(staff_state().is_none());
        assert!(own_key_id().is_none());
        assert!(!set_achievement_counts(&std::collections::BTreeMap::new()));
        assert!(!chat("hi".to_owned()));
        assert!(!set_avatar(vec![1, 2, 3]));
        assert!(!remove_avatar());
        assert!(!emote("wave".to_owned(), None));
        assert!(with_chat(|chat| chat.revision).is_none());
        assert!(take_emotes().is_empty());
        assert_eq!(with_claims(|claims| claims.len()), 0);
        assert_eq!(avatar_version("0123456789abcdef"), None);
        assert_eq!(take_looks(None), sjk_identity::ReceivedLooks::default());
        assert!(with_roster(|players| players.len()).is_none());
        assert!(!owns_unlock("saber_sun"));
        assert!(!set_look(&sjk_identity::Look::default()));
    }

    #[test]
    fn the_look_is_given_only_when_it_changes() {
        let lit = sjk_identity::Look {
            saber: String::new(),
            illuminate: true,
            saber_off: Vec::new(),
            toys: Vec::new(),
            saber2: None,
            saber2_off: Vec::new(),
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            saber_sounds: String::new(),
            tag: None,
            finisher: String::new(),
        };
        let mut sent = None;
        assert!(newly(&mut sent, &lit));
        assert!(!newly(&mut sent, &lit));
        assert!(newly(&mut sent, &sjk_identity::Look::default()));
        assert!(newly(&mut sent, &lit));
    }
}
