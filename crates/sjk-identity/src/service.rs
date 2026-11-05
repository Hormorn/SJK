//! The background service that keeps the player registered with the hub, repeats
//! their game-server claim while they play, tells the hub the look they wear there,
//! and reads who else on that server is known. Nothing here blocks a frame: the viewer sends [`Command`]s through
//! [`Service`] and reads a [`Snapshot`].
//!
//! Once registered, and only when told where ([`Service::keep_assets`]), it also keeps
//! the hub's asset packs ([`crate::assets`]) in a cache folder.
//!
//! The service is inert while it is disabled or has no hub address: it makes no
//! request of any kind.

use crate::feed::{
    ChatState, FeedShared, FeedWorker, QueuedLook, QueuedPing, QueuedToyEvent, ReceivedLooks,
};
use crate::hub::{Hub, HubError};
use crate::keys::Identity;
use crate::logs::{LogRequest, LogUpload};
use crate::news::{AnnounceRequest, Announcement, Rewards};
use crate::relay::RelayHubs;
use crate::report::{BugReport, PlayerReport, WorldNote};
use crate::staff::{OpenedText, StaffQuery, StaffRequest, StaffState};
use crate::vitals::{Sharing, VitalsTable, VitalsWorker};
use crate::wire::{
    Achievement, Emote, HolocronState, Look, Ping, Presence, Profile, ToyEvent, Unlock, names_match,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How often a claim is repeated; a claim lives 90 seconds at the hub.
const CLAIM_EVERY: Duration = Duration::from_secs(45);
/// How often the roster of known players is read (15 seconds before 27/10/2026).
const POLL_EVERY: Duration = Duration::from_secs(5);
/// How often the player's own profile is read again while registered, so a medal
/// the SJK team gives during a session shows without a restart.
const PROFILE_EVERY: Duration = Duration::from_secs(600);
/// Shortest time between two readings of the own profile brought forward because its
/// unlocks look out of date (a skin refused, or taken back at the hub).
const PROFILE_SOON: Duration = Duration::from_secs(30);
/// How often the progress toward the next holocron is read while registered.
const HOLOCRONS_EVERY: Duration = Duration::from_secs(300);
/// How often the player's trades are read while one runs (the feed tells of changes at
/// once; this keeps the list true without it).
const TRADES_LIVE: Duration = Duration::from_secs(15);
/// How often they are read otherwise, for whether the hub lets players trade.
const TRADES_EVERY: Duration = Duration::from_secs(600);
/// How often they are read when the hub says trading is off for this player: not every
/// [`TRADES_EVERY`] (Sol's review, 24/10/2026), yet soon enough that trading switched on
/// at the hub (`--trading staff`, then everyone) reaches running clients without a
/// restart (Sol's review, 25/10/2026), as the announcements and rewards are read.
/// Registering again or the feed's `trades` event reads the list at once.
const TRADES_OFF: Duration = Duration::from_secs(1_800);

/// This PC's clock, unix seconds, standing in for the hub's when it does not say.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}
/// Shortest time between two readings of it brought forward (a drop of the player's
/// own key, or the viewer asking), and the profile's early reading for a drop.
const HOLOCRONS_SOON: Duration = Duration::from_secs(30);
/// How long a hub without the reading (from before holocrons) is left alone.
const HOLOCRONS_AGAIN: Duration = Duration::from_secs(3_600);
/// Shortest time between two sendings of the achievement counts.
const ACHIEVEMENTS_EVERY: Duration = Duration::from_secs(60);
/// How long counts the hub held back (over an hourly allowance) wait before they are
/// sent again unchanged (changed ones go at [`ACHIEVEMENTS_EVERY`]), and how long a
/// refusal waits.
const ACHIEVEMENTS_AGAIN: Duration = Duration::from_secs(3_600);
/// How long a refused model list waits before it is sent again unchanged.
const MODELS_AGAIN: Duration = Duration::from_secs(3_600);
/// How long looks go without `saber2` after a hub refused it as unknown (`bad_body`),
/// before trying again: the hub may have been updated meanwhile.
const SABER2_RETRY: Duration = Duration::from_secs(600);
/// Model answers kept at most; past it the oldest are forgotten.
const MODEL_CHECKS_MAX: usize = 256;
/// First wait after a failure, doubled up to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(120);
/// Longest the worker sleeps without looking at its commands.
const IDLE_MAX: Duration = Duration::from_secs(60);
/// Shortest time between two looks sent: the hub takes one a second.
const LOOK_EVERY: Duration = Duration::from_secs(1);
/// How long a look waits after the hub said too many (429: `look_quota`, or the
/// address's `rate_limited`), refused the signature (401) or failed.
const LOOK_AGAIN: Duration = Duration::from_secs(10);
/// How long a look waits after the hub said the key's looks came too fast (429
/// `look_quota`): the hub counts whole seconds, so a second more is enough.
const LOOK_QUOTA_AGAIN: Duration = Duration::from_secs(1);
/// The hub's answer to a map edits publication: the map, the checksum and the new
/// revision, or why it was refused.
pub type MapEditsPublished = Result<(String, i32, u64), String>;

/// How often the hub's list of map edits is read while registered.
const MAP_EDITS_EVERY: Duration = Duration::from_secs(600);
/// How often the hub's mystery boxes are read while registered.
const MYSTERY_BOXES_EVERY: Duration = Duration::from_secs(600);
/// How often the Toy Master's toys are read while registered.
const SECRETS_EVERY: Duration = Duration::from_secs(600);

/// What a request about the Toy Master's toys did ([`Snapshot::secret`]).
#[derive(Clone, Debug, PartialEq)]
pub enum SecretDone {
    /// The player's presses on toy `id` were counted: whether they started its show,
    /// whether it played one on this map load already, and whether they found it.
    Used {
        /// The toy.
        id: i64,
        /// The presses started its show.
        show: bool,
        /// It played its show on this map load (now or before).
        done: bool,
        /// The player found it with these presses (its achievement unlocked).
        found: bool,
    },
    /// The player took drop `drop` of toy `id`'s show: what it gave (`credits`,
    /// `shards` or `holocron`), how many, and a holocron's tier.
    Took {
        /// The toy.
        id: i64,
        /// The drop.
        drop: u32,
        /// What it gave.
        kind: String,
        /// How many credits or shards.
        amount: i64,
        /// A holocron's tier.
        tier: Option<String>,
    },
    /// SJK's creator hid a toy: its id.
    Placed(i64),
    /// SJK's creator took toy `id` away.
    Removed(i64),
    /// SJK's creator set toy `id` to show only where they are (`true`) or everywhere.
    Escorted(i64, bool),
}

/// What a mystery box request did ([`Snapshot::mystery_box`]).
#[derive(Clone, Debug, PartialEq)]
pub enum BoxDone {
    /// The box was opened: what came out.
    Opened(crate::mystery_box::Opened),
    /// SJK's creator placed a box: its id.
    Placed(i64),
    /// SJK's creator removed box `id`.
    Removed(i64),
}
/// How often the hub's list of asset packs is read while registered.
const ASSETS_EVERY: Duration = Duration::from_secs(6 * 3_600);
/// First wait after an asset check fell short, doubled up to [`ASSETS_EVERY`].
/// How often the crash folder is looked at again while registered: a report a worker
/// thread's panic left while the game went on goes within the hour.
const CRASHES_EVERY: Duration = Duration::from_secs(3_600);
/// How long reports the hub could not take wait before the next try.
const CRASHES_AGAIN: Duration = Duration::from_secs(600);
const ASSETS_RETRY_MIN: Duration = Duration::from_secs(60);
/// How often the SJK team's requests for the player's logs are looked for.
const LOG_REQUESTS_EVERY: Duration = Duration::from_secs(120);
/// How long a hub that refused the reading (from before log requests) is left alone.
const LOG_REQUESTS_AGAIN: Duration = Duration::from_secs(3_600);
/// How often the SJK team's announcements and the player's rewards are read while
/// registered (the feed tells of new ones as they come).
const NEWS_EVERY: Duration = Duration::from_secs(1_800);
/// How long a hub that refused the reading (from before announcements) is left alone.
const NEWS_AGAIN: Duration = Duration::from_secs(3_600);
/// Shortest time between two readings of the rewards brought forward (a window that
/// shows them opening, a round the feed told of).
const REWARDS_SOON: Duration = Duration::from_secs(30);
/// Shortest time between two readings of the profile, holocron progress and trades
/// brought forward by the game menu opening ([`Service::menu_opened`]).
const MENU_SOON: Duration = Duration::from_secs(15);
/// Shortest time between two sendings of the achievement counts brought forward by it:
/// the hub takes 40 in 10 minutes from an address.
const MENU_COUNTS_SOON: Duration = Duration::from_secs(30);
/// Announcements kept for the viewer, newest first.
const ANNOUNCEMENTS_KEPT: usize = 20;

/// What the player has configured.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Settings {
    /// Whether the identity features are on.
    pub enabled: bool,
    /// The hub's address; empty means no hub, so nothing is sent.
    pub hub_url: String,
}

/// Where the player is: the game server they are connected to and how the game
/// shows them there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    /// The server's address as connected.
    pub server: SocketAddr,
    /// The player's own client number there.
    pub slot: u8,
    /// The player's name as the game shows it.
    pub name: String,
}

/// How the service is doing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    /// Turned off in the settings.
    Disabled,
    /// On, but there is no hub address.
    NoHub,
    /// Registering with the hub.
    Registering,
    /// Registered; claims and reads work.
    Online,
    /// The last hub request failed (the text says why); the service retries.
    Failed(String),
}

/// Whether the player of a slot has a model (`PROTOCOL.md`, "Models").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCheck {
    /// The name the slot's claim was made under, empty when nobody the hub knows
    /// holds it.
    pub claimed_name: String,
    /// Whether they have the model; `None` when they sent no list, or the hub did
    /// not answer.
    pub has: Option<bool>,
}

/// What the viewer may read.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The service's status.
    pub status: Status,
    /// The player's own id at the hub: this PC's key id until the hub answered the
    /// player's profile, then the id of the person that profile is (another one when
    /// this key is linked to a person's main key). Presence, the feed and the pages
    /// recognise the player by it.
    pub key_id: String,
    /// This PC's key id (the key file's), whatever person it belongs to: for what is
    /// kept per key on this PC (seen lists, counts) and the Identity page.
    pub local_key_id: String,
    /// This PC's public key as the hub writes it (unpadded base64url), for encrypted
    /// whispers' safety codes (`crate::whisper::safety_code`).
    pub public_key: String,
    /// The player's profile at the hub, once registered.
    pub me: Option<Profile>,
    /// The game server being claimed on, if any.
    pub server: Option<SocketAddr>,
    /// Known players on that server, by slot.
    pub players: Vec<Presence>,
    /// The players SJK staff blocked there for everyone until the map changes
    /// (`PROTOCOL.md`, "Map blocks"), by slot; counted in `revision` with `players`.
    pub map_blocks: Vec<crate::MapBlock>,
    /// Profiles fetched with [`Service::look_up`].
    pub profiles: HashMap<String, Profile>,
    /// What the hub answered to [`Service::check_model`], by server, slot and model.
    pub model_checks: HashMap<(SocketAddr, u8, String), ModelCheck>,
    /// Counts profiles fetched into [`Snapshot::profiles`] and answers stored in
    /// [`Snapshot::model_checks`], so a reader that derived something from one (the
    /// player card) knows when to read again.
    pub profiles_revision: u64,
    /// Counts changes to [`Snapshot::me`], so a reader that derived something from the
    /// own profile (the trade window's items) knows when to derive again.
    pub me_revision: u64,
    /// The outcome of the last profile change or lookup, for the page to show.
    pub notice: Option<String>,
    /// Counts changes to [`Snapshot::players`], so a reader that caches what it
    /// derived from them (scoreboard tags) knows when to derive again.
    pub revision: u64,
    /// The outcome of the last bug report sent with [`Service::report`].
    pub report: Option<ReportOutcome>,
    /// The outcome of the last world note sent with [`Service::note`].
    pub note: Option<ReportOutcome>,
    /// The outcome of the last player report sent with [`Service::player_report`].
    pub player_report: Option<ReportOutcome>,
    /// The outcome of the last picture change ([`Service::set_avatar`],
    /// [`Service::remove_avatar`]): sent when the hub took it.
    pub avatar: Option<ReportOutcome>,
    /// What became of the last look sent ([`Service::set_look`]).
    pub look_outcome: Option<ReportOutcome>,
    /// Counts the asset packs written into the folder given to
    /// [`Service::keep_assets`], so the viewer knows to mount them again.
    pub packs_revision: u64,
    /// What the last check of the asset packs did, for the log. Asset packs are
    /// cosmetic: a failure shows here, never in [`Snapshot::status`].
    pub assets_note: Option<String>,
    /// Counts the map edit documents written or removed in the folder given to
    /// [`Service::keep_map_edits`], so the viewer reads the loaded map's again.
    pub map_edits_revision: u64,
    /// What the last look at the hub's map edits did: its summary
    /// ([`crate::map_edits::Refresh::summary`]), or why it fell short, for the log and the
    /// editor. Map edits are cosmetic: a failure shows here, never in [`Snapshot::status`].
    pub map_edits_note: Option<Result<String, String>>,
    /// The hub's answer to the last [`Service::publish_map_edits`], by serial: the map,
    /// the checksum and the new revision, or why it was refused.
    pub map_edits_published: Option<(u64, MapEditsPublished)>,
    /// The hub's mystery boxes, once read ([`Service::open_mystery_box`]).
    pub mystery_boxes: Option<crate::mystery_box::Boxes>,
    /// Counts the times [`Snapshot::mystery_boxes`] changed.
    pub mystery_boxes_revision: u64,
    /// The answer to the last mystery box request, by serial, or why it was refused
    /// (in the player's words).
    pub mystery_box: Option<(u64, Result<BoxDone, String>)>,
    /// Fished junk selling as the hub last said ([`Service::read_junk`]): `None` until
    /// read, and when the hub has none or could not be asked (no Sell button then).
    pub junk: Option<crate::junk::JunkStatus>,
    /// The answer to the last junk sale ([`Service::sell_junk`]), by serial, or why it
    /// was refused (in the player's words).
    pub junk_sale: Option<(u64, Result<crate::junk::JunkSold, String>)>,
    /// The box hunt as last read, by serial, for the server named
    /// ([`Service::read_box_hunt`]).
    pub box_hunt: Option<(u64, String, crate::mystery_box::HuntState)>,
    /// The Toy Master's toys, once read ([`Service::use_secret`]).
    pub secrets: Option<crate::toy_master::Secrets>,
    /// Counts the times [`Snapshot::secrets`] changed.
    pub secrets_revision: u64,
    /// The Toy Master's shows under way on the player's server, as the hub's last
    /// presence answer told them, and when it arrived (their `ago` is as of then).
    pub secret_shows: Option<(Instant, Vec<crate::toy_master::ShowUnderWay>)>,
    /// Counts the times [`Snapshot::secret_shows`] changed.
    pub secret_shows_revision: u64,
    /// The answer to the last request about the Toy Master's toys, by serial, or why it
    /// was refused (in the player's words).
    pub secret: Option<(u64, Result<SecretDone, String>)>,
    /// Counts crash reports the hub took ([`Service::send_crashes`]).
    pub crashes_sent: u64,
    /// What the last round of crash reports did, for the log, once a round sent,
    /// dropped or kept back anything.
    pub crash_note: Option<String>,
    /// How far the player is from their next holocron, as the hub last said (read at
    /// the registration, every few minutes, after a drop of their own and on
    /// [`Service::refresh_holocrons`]). `None` before the hub answered, or from a hub
    /// older than holocrons. Cosmetic: a failure leaves [`Snapshot::status`] alone.
    pub holocrons: Option<HolocronProgress>,
    /// The oldest SJK release the hub lets open holocrons and trade, as it last said with
    /// the progress (21/10/2026); `None` when it wants none. An older build says to update
    /// in place of its Open buttons.
    pub min_client: Option<String>,
    /// The odds the hub rolls holocrons with, one table per tier, as it last sent them
    /// with the progress; empty before, or from a hub older than them (14/10/2026).
    pub loot: Vec<crate::wire::LootTier>,
    /// The kyber crystals' odds and the person's pity counter, as the hub last sent them
    /// with the progress; `None` before, or from a hub older than crystals (21/10/2026).
    pub kyber: Option<crate::wire::KyberTables>,
    /// The SJK team's open requests for the player's logs (oldest first), as the hub
    /// last listed them, without the ones the player answered: the game asks the
    /// player about each ([`Service::answer_logs`]).
    pub log_requests: Vec<LogRequest>,
    /// What became of the last answer to a log request.
    pub log_answer: Option<ReportOutcome>,
    /// What became of the last opening of holocrons ([`Service::open_holocrons`]).
    pub opening: Option<OpenOutcome>,
    /// What became of the last reveal of sealed cards ([`Service::reveal_cards`]).
    pub reveal: Option<RevealOutcome>,
    /// The kyber shards' and the shop's tables, as the hub last sent them with the
    /// progress; `None` before, or from a hub older than them (17/10/2026).
    pub economy: Option<crate::wire::EconomyTables>,
    /// The shop's day as the hub last answered it ([`Service::read_shop`]), or why it
    /// could not be read.
    pub shop: Option<ShopState>,
    /// What became of the last shatter, craft or purchase ([`Service::shatter`],
    /// [`Service::craft`], [`Service::buy`]).
    pub trade: Option<TradeOutcome>,
    /// What became of the last tag break-down or tag craft ([`Service::shatter_tag`],
    /// [`Service::craft_tags`], 03/11/2026).
    pub tag_trade: Option<TagOutcome>,
    /// The player's collection marks (favourite and locked saber shaders, 29/10/2026), as
    /// the hub last sent them with the progress or answered a change, a change asked with
    /// [`Service::set_mark`] shown at once; `None` before, or from a hub that keeps none
    /// (its economy does not say [`crate::wire::MARKS_ABILITY`]).
    pub marks: Option<crate::wire::UnlockMarks>,
    /// Counts changes to [`Snapshot::marks`].
    pub marks_revision: u64,
    /// Why the last change of a mark failed (in the player's words), by its count: the
    /// marks are read again from the hub then.
    pub mark_refusal: Option<(u64, String)>,
    /// The player's trades with other players, as the hub last listed them
    /// ([`Service::trade_with`], 22/10/2026); `None` before the first read.
    pub trades: Option<TradesState>,
    /// The player's inventory history as read so far ([`Service::read_history`],
    /// 31/10/2026), shared so a copy of the snapshot costs nothing more; `None` before
    /// the first read.
    pub history: Option<Arc<crate::history::HistoryState>>,
    /// The SJK team's announcements as the hub last listed them and the feed relayed
    /// since, newest first (`GET /v1/announcements`, 25/10/2026): read at the
    /// registration and every 30 minutes. Empty before, or from a hub without them.
    pub announcements: Vec<Announcement>,
    /// Counts changes to [`Snapshot::announcements`].
    pub announcements_revision: u64,
    /// The player's rewards and the newest reward rounds as the hub last said (read at the
    /// registration, every 30 minutes, after a round the feed told of and on
    /// [`Service::refresh_rewards`]); `None` before, or from a hub without them.
    pub rewards: Option<Rewards>,
    /// Counts changes to [`Snapshot::rewards`].
    pub rewards_revision: u64,
    /// What became of the last claim ([`Service::claim_reward`]).
    pub claim: Option<ClaimOutcome>,
    /// What became of the SJK team's last announcement ([`Service::announce`]).
    pub announce: Option<ReportOutcome>,
    /// What became of the last news picture uploaded ([`Service::upload_news_image`]):
    /// its serial, and the picture or why it was refused.
    pub news_image: Option<(u64, Result<crate::news::NewsImage, String>)>,
}

/// The player's trades with other players, as last read (`crate::trades`).
#[derive(Clone, Debug, PartialEq)]
pub struct TradesState {
    /// Counts reads, so a reader can tell a new list from the last one.
    pub serial: u64,
    /// The hub's list, its rules and what the player traded today.
    pub list: crate::trades::TradesAnswer,
    /// When this client read it, and the hub's clock then (with it the trades'
    /// `expires` and `confirm_from` read on this client's clock).
    pub read_at: Instant,
    /// The hub's clock when it answered, unix seconds; 0 from a hub that does not say.
    pub hub_time: i64,
    /// Why the last step or read failed, in the player's words, and the serial of the
    /// step it belongs to; empty when it worked.
    pub problem: String,
    /// Counts steps taken, so a reader can tell a new problem from the last one.
    pub steps: u64,
}

impl TradesState {
    /// Whether players may trade on this hub (for this player: `staff` lets staff).
    pub fn open(&self, staff: bool) -> bool {
        self.list.open(staff)
    }

    /// The trade that runs now (asked or open), if any: a player is in at most one.
    pub fn live(&self) -> Option<&crate::trades::PlayerTrade> {
        self.list.trades.iter().find(|trade| trade.is_live())
    }

    /// The hub's clock as read with this list, to keep apart from it.
    pub fn clock(&self) -> TradesClock {
        TradesClock {
            hub_time: self.hub_time,
            read_at: self.read_at,
        }
    }

    /// The hub's clock at `now`, unix seconds, as read.
    pub fn hub_now(&self, now: Instant) -> i64 {
        self.clock().hub_now(now)
    }

    /// Seconds until hub time `at`, at `now`; 0 once it passed.
    pub fn seconds_until(&self, at: i64, now: Instant) -> u64 {
        self.clock().seconds_until(at, now)
    }

    /// [`Self::seconds_until`] to the fraction, for a bar that runs down smoothly.
    pub fn seconds_until_exact(&self, at: i64, now: Instant) -> f32 {
        self.clock().seconds_until_exact(at, now)
    }
}

/// The hub's clock as a list of trades was read: the hub's unix seconds then, and this
/// PC's instant. It turns the trades' hub times into time left on this PC's clock, so a
/// reader can keep it without the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradesClock {
    /// The hub's clock when it answered, unix seconds.
    pub hub_time: i64,
    /// When this client read it.
    pub read_at: Instant,
}

impl TradesClock {
    /// The hub's clock at `now`, unix seconds, as read.
    pub fn hub_now(&self, now: Instant) -> i64 {
        self.hub_time + now.saturating_duration_since(self.read_at).as_secs() as i64
    }

    /// Seconds until hub time `at`, at `now`; 0 once it passed.
    pub fn seconds_until(&self, at: i64, now: Instant) -> u64 {
        (at - self.hub_now(now)).max(0) as u64
    }

    /// [`Self::seconds_until`] to the fraction, for a bar that runs down smoothly. The
    /// hub's clock is read in whole seconds, so it is taken as half way into its second.
    pub fn seconds_until_exact(&self, at: i64, now: Instant) -> f32 {
        let elapsed = now.saturating_duration_since(self.read_at).as_secs_f64();
        let hub_now = self.hub_time as f64 + 0.5 + elapsed;
        (at as f64 - hub_now).max(0.0) as f32
    }
}

/// What became of a claim of a reward.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimOutcome {
    /// Counts claims asked for, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// The reward's number.
    pub id: u64,
    /// Why it failed, in the player's words; empty when the hub granted it.
    pub message: String,
}

impl ClaimOutcome {
    /// Whether the hub granted it.
    pub fn done(&self) -> bool {
        self.message.is_empty()
    }
}

/// The shop as last read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopState {
    /// Counts reads asked for, so a reader can tell a new answer from the last one.
    pub serial: u64,
    /// The day's offers; empty when the read failed.
    pub day: crate::wire::ShopDay,
    /// Why it could not be read; empty when it was.
    pub message: String,
    /// When this client read it (with [`crate::wire::ShopDay::server_time`] it says
    /// when the next day comes on this client's clock).
    pub read_at: Instant,
}

impl ShopState {
    /// Seconds until the shop's next day at `now`, as read; 0 once it is due.
    pub fn seconds_left(&self, now: Instant) -> u64 {
        let left = (self.day.next - self.day.server_time).max(0) as u64;
        left.saturating_sub(now.saturating_duration_since(self.read_at).as_secs())
    }
}

/// What a trade asked of the hub.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Trade {
    /// One duplicate into kyber shards.
    Shatter,
    /// One copy into kyber shards, the last one too (hubs whose economy says
    /// `shatter_last`, 21/10/2026).
    ShatterLast,
    /// Every duplicate into kyber shards, one copy of each kept; the outcome's `id` is
    /// empty and its `copies` how many went (21/10/2026).
    ShatterAll,
    /// A shader made with kyber shards.
    Craft,
    /// One of the shop's offers bought (with kyber shards since 17/10/2026).
    Buy,
    /// A holocron bought with credits; the outcome's `id` is its tier.
    Holocron,
}

/// What became of a shatter, a craft or a purchase.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TradeOutcome {
    /// Counts trades asked for, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// What was asked.
    pub trade: Trade,
    /// The shader's id.
    pub id: String,
    /// The shards a shatter gave or a craft cost, what an offer cost (in the shop's
    /// currency) or the credits a holocron cost; 0 when it failed.
    pub amount: u64,
    /// The copies a shatter took (every one of them for [`Trade::ShatterAll`], else one);
    /// 0 for the other trades and when it failed.
    pub copies: u32,
    /// Why it failed; empty when it worked.
    pub message: String,
}

impl TradeOutcome {
    /// Whether the hub did it.
    pub fn done(&self) -> bool {
        self.message.is_empty()
    }
}

/// A tag copies request ([`Service::shatter_tag`], [`Service::craft_tags`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TagTrade {
    /// One copy broken down into kyber shards.
    Shatter(crate::wire::TagCopy),
    /// Two copies given for a tag not held yet.
    Craft([crate::wire::TagCopy; 2]),
}

/// What became of a tag copies request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagOutcome {
    /// Counts the requests, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// What was asked.
    pub trade: TagTrade,
    /// The shards a break-down gave; 0 for a craft and when it failed.
    pub shards: u64,
    /// The tag a craft made; `None` for a break-down and when it failed.
    pub crafted: Option<crate::wire::TagCrafted>,
    /// Why it failed; empty when it worked.
    pub message: String,
}

impl TagOutcome {
    /// Whether the hub did it.
    pub fn done(&self) -> bool {
        self.message.is_empty()
    }
}

/// What became of a reveal of sealed cards.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RevealOutcome {
    /// Counts reveals asked for, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// The copies revealed; empty when none could be (or it failed).
    pub revealed: Vec<crate::wire::Revealed>,
    /// Why it failed; empty when it worked.
    pub message: String,
}

/// What became of an opening of holocrons.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenOutcome {
    /// Counts openings asked for, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// What each holocron brought, in the order asked; empty when nothing was opened.
    pub opened: Vec<crate::wire::Opened>,
    /// Why nothing was opened; empty when it worked.
    pub message: String,
}

/// How far the player's key is from its next holocron (`GET /v1/holocrons`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HolocronProgress {
    /// Seconds of active play carried toward the next holocron, when it was read.
    pub progress_secs: u64,
    /// Seconds of active play for one holocron.
    pub every_secs: u64,
    /// Holocrons dropped since the caps' day began when it was read (09:00 in Paris).
    pub today: u32,
    /// The most a key can earn in a day.
    pub daily_cap: u32,
    /// When the caps' next day begins, unix seconds; `None` from a hub before 29/10/2026.
    pub resets_at: Option<i64>,
    /// The hub's clock when it answered, unix seconds.
    pub server_time: i64,
    /// Whether the person may open holocrons now (only staff while opening is tested).
    pub can_open: bool,
    /// Today's catch-up (03/11/2026); `None` from hubs before.
    pub catchup: Option<crate::wire::Catchup>,
    /// How the hub judged the last claims' play (03/11/2026); `None` from hubs before.
    pub activity: Option<crate::wire::ActivityState>,
    /// When this client read it.
    pub read_at: Instant,
}

impl HolocronProgress {
    fn new(state: HolocronState, read_at: Instant) -> Self {
        Self {
            progress_secs: state.progress_secs,
            every_secs: state.every_secs,
            today: state.today,
            daily_cap: state.daily_cap,
            resets_at: state.resets_at,
            server_time: state.server_time,
            can_open: state.can_open,
            catchup: state.catchup,
            activity: state.activity,
            read_at,
        }
    }

    /// The active seconds the next holocron needs: the catch-up's interval while catch-up
    /// drops are left today (twice as fast), else `every_secs`.
    pub fn next_every(&self) -> u64 {
        match self.catchup {
            Some(catchup) if catchup.left > 0 && catchup.every_secs > 0 => catchup.every_secs,
            _ => self.every_secs,
        }
    }

    /// The catch-up drops left today (0 without one, and from hubs before).
    pub fn catchup_left(&self) -> u32 {
        self.catchup.map_or(0, |catchup| catchup.left)
    }

    /// Seconds of active play left to the next holocron as read: the hub's
    /// [`Self::next_every`] less `progress_secs`. It does not count down on its own; the
    /// player may be playing or not.
    pub fn remaining_secs(&self) -> u64 {
        self.next_every().saturating_sub(self.progress_secs)
    }

    /// Whether the day's cap is reached, so no holocron drops until the next day begins.
    pub fn capped(&self) -> bool {
        self.daily_cap > 0 && self.today >= self.daily_cap
    }
}

/// What became of a bug report or a world note.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportOutcome {
    /// Counts reports sent, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// The hub stored it.
    pub sent: bool,
    /// The hub's report number, or why it was not stored.
    pub message: String,
}

impl Snapshot {
    fn new(identity: &Identity) -> Self {
        let key_id = identity.key_id();
        Self {
            status: Status::Disabled,
            public_key: identity.public_key_text(),
            local_key_id: key_id.clone(),
            key_id,
            me: None,
            server: None,
            players: Vec::new(),
            map_blocks: Vec::new(),
            profiles: HashMap::new(),
            model_checks: HashMap::new(),
            profiles_revision: 0,
            me_revision: 0,
            notice: None,
            revision: 0,
            report: None,
            note: None,
            player_report: None,
            avatar: None,
            look_outcome: None,
            packs_revision: 0,
            assets_note: None,
            map_edits_revision: 0,
            map_edits_note: None,
            map_edits_published: None,
            mystery_boxes: None,
            mystery_boxes_revision: 0,
            mystery_box: None,
            junk: None,
            junk_sale: None,
            box_hunt: None,
            secrets: None,
            secrets_revision: 0,
            secret_shows: None,
            secret_shows_revision: 0,
            secret: None,
            crashes_sent: 0,
            crash_note: None,
            holocrons: None,
            min_client: None,
            loot: Vec::new(),
            log_requests: Vec::new(),
            log_answer: None,
            opening: None,
            reveal: None,
            economy: None,
            kyber: None,
            shop: None,
            trade: None,
            tag_trade: None,
            marks: None,
            marks_revision: 0,
            mark_refusal: None,
            trades: None,
            history: None,
            announcements: Vec::new(),
            announcements_revision: 0,
            rewards: None,
            rewards_revision: 0,
            claim: None,
            announce: None,
            news_image: None,
        }
    }

    /// Take `profile` as the player's own: the player's id follows the person it is
    /// (a linked key's profile is its person's).
    fn set_me(&mut self, profile: Profile) {
        if self.key_id != profile.key_id {
            self.key_id.clone_from(&profile.key_id);
        }
        self.me = Some(profile);
        self.me_revision += 1;
    }

    /// Forget the own profile (the identity off, another hub, a key unlinked): the
    /// player is this PC's key again until the hub answers.
    fn clear_me(&mut self) {
        self.me = None;
        self.me_revision += 1;
        // The marks are the person's: another person's (or none) come with the next read.
        if self.marks.take().is_some() {
            self.marks_revision += 1;
        }
        if self.key_id != self.local_key_id {
            self.key_id.clone_from(&self.local_key_id);
        }
    }

    /// The known player in `slot` whose claimed name matches `shown`, the name the
    /// game shows there. A claim for another name is somebody else's and is ignored.
    pub fn badge(&self, slot: u8, shown: &str) -> Option<&Presence> {
        self.players
            .iter()
            .find(|player| player.slot == slot && names_match(&player.claimed_name, shown))
    }
}

/// What a box hunt read first tells the hub.
#[derive(Clone, Copy)]
enum HuntReport {
    /// The player, the mark, was killed by this slot.
    Killed(u8),
    /// The player is out of the chase.
    Out(crate::mystery_box::HuntOut),
}

/// A request to the worker.
enum Command {
    Configure(Settings),
    Enter(Location),
    Leave,
    /// The in-game name the player now wears.
    Name(String),
    SetBio(String),
    SetBanner(String),
    /// The name emoji to show at the right of the player's name, empty for none.
    SetEmoji(String),
    /// The counts the client keeps for its achievements, by id.
    Achievements(BTreeMap<String, u64>),
    /// A staff request.
    Staff(StaffRequest),
    /// A staff query about logs and crash reports.
    StaffQuery(StaffQuery),
    /// The player's answer to log request `id`: the logs to send, or none to decline.
    AnswerLogs(i64, Option<LogUpload>),
    LookUp(String),
    /// The player models the game has.
    Models(Vec<String>),
    /// Ask whether the player of a slot has a model.
    CheckModel(SocketAddr, u8, String),
    Report(BugReport),
    PlayerReport(PlayerReport),
    /// A world note and the tag its picture will come with.
    Note(u64, Box<WorldNote>),
    /// The picture of the note tagged so.
    NoteImage(u64, Vec<u8>),
    /// An SJK chat message.
    Chat(String),
    /// A message in a clan chat ([`crate::channels`]).
    ClanChat(String, crate::HubChannel),
    /// The clan chats the player joined, read with the chat.
    SetClanChats(Vec<crate::HubChannel>),
    React {
        message: u64,
        emoji: String,
        active: bool,
    },
    /// Ban the player of this key from the SJK chat for 24 hours.
    ChatBan(String),
    /// An emote, for the slot the player claims, in step with another slot's or not.
    Emote(String, Option<u8>, bool),
    /// A ping at a point (whole game units) for a scope, from the slot the player claims.
    Ping([i32; 3], String),
    /// A toy event (toy, event, target slot), from the slot the player claims.
    Toy(String, String, u8),
    /// Whether the SJK chat is on.
    SetChat(bool),
    /// A new picture, a PNG already cropped and scaled (`crate::avatar`).
    SetAvatar(Vec<u8>),
    /// Take the picture down.
    RemoveAvatar,
    /// The look the player wears (the tests' way; [`Service::set_look`] leaves it in
    /// [`Worker::latest_look`]).
    #[cfg_attr(not(test), allow(dead_code))]
    Look(Look),
    /// A new look waits in [`Worker::latest_look`] (what [`Service::set_look`] sends).
    LookWaiting,
    /// The feed relayed a look event of the player's own key: its id and blade skin.
    OwnLook(u64, String),
    /// The folder to keep the hub's asset packs in.
    AssetsDir(PathBuf),
    /// The folder map edits are kept in ([`crate::map_edits`]).
    MapEditsDir(PathBuf),
    /// Look at the hub's map edits now.
    RefreshMapEdits,
    /// Open mystery box `id` on `server`.
    OpenMysteryBox {
        id: i64,
        server: String,
    },
    /// SJK's creator: place a mystery box (its id is ignored).
    PlaceMysteryBox(crate::mystery_box::MysteryBox),
    /// SJK's creator: remove mystery box `id`.
    RemoveMysteryBox(i64),
    /// Ask the hub about junk selling.
    ReadJunk,
    /// Sell these pieces of fished junk, by id.
    SellJunk(std::collections::BTreeMap<String, u32>),
    /// Read the box hunt on `server`, the player away or not.
    BoxHunt {
        server: String,
        away: bool,
    },
    /// The player pressed Use `presses` times on the Toy Master's toy `id` on `server`,
    /// in map load `round`.
    UseSecret {
        id: i64,
        server: String,
        round: i32,
        presses: u32,
    },
    /// The player takes drop `drop` of toy `id`'s show on `server`, in map load `round`.
    TakeSecretDrop {
        id: i64,
        server: String,
        round: i32,
        drop: u32,
    },
    /// SJK's creator: hide a toy.
    PlaceSecret(crate::toy_master::Placing),
    /// SJK's creator: take toy `id` away.
    RemoveSecret(i64),
    /// SJK's creator: show toy `id` only where they are, or everywhere.
    EscortSecret(i64, bool),
    /// The player, the box hunt's mark on `server`, was killed by slot `by`.
    BoxHuntKilled {
        server: String,
        by: u8,
        away: bool,
    },
    /// The player is out of the box hunt's chase on `server`.
    BoxHuntOut {
        server: String,
        out: crate::mystery_box::HuntOut,
        away: bool,
    },
    /// Staff: publish a map's edits.
    PublishMapEdits {
        map: String,
        checksum: i32,
        edits: Value,
    },
    /// The folder crash reports wait in, or none to send none.
    Crashes(Option<PathBuf>),
    /// Whether the player is actively playing (`Service::set_active`).
    Active(bool),
    /// What the player did lately, for the next active claim (`Service::add_play`).
    Play(crate::wire::PlaySignals),
    /// Break one tag copy down.
    ShatterTag(crate::wire::TagCopy),
    /// Give two tag copies for a new tag.
    CraftTags([crate::wire::TagCopy; 2]),
    /// The feed relayed a holocron drop of the player's own key.
    OwnDrop,
    /// The viewer wants fresh holocron progress.
    RefreshHolocrons,
    /// The game menu opened (`Service::menu_opened`).
    MenuOpened,
    /// Open these holocrons of the player's.
    OpenHolocrons(Vec<u64>),
    /// Reveal the player's sealed cards.
    RevealCards,
    /// Shatter one duplicate of this shader.
    Shatter(String),
    /// Shatter one copy of this shader, the last one too.
    ShatterLast(String),
    /// Shatter every duplicate, one copy of each kept.
    ShatterDuplicates,
    /// Craft this shader with shards.
    Craft(String),
    /// Set or clear a collection mark on this shader.
    Mark(String, crate::wire::Mark, bool),
    /// Read the shop's day.
    ReadShop,
    /// Buy this offer of the day.
    Buy(String),
    /// Buy a holocron of this tier with credits.
    BuyHolocron(String),
    /// Read the player's trades again (the feed told of a change, or the viewer asks).
    ReadTrades,
    /// Read the player's history: the newest page, or the one before this time.
    ReadHistory(Option<i64>),
    /// Ask the player with this hub id to trade.
    TradeAsk(String),
    /// Take a step in this trade.
    TradeStep(i64, crate::trades::TradeStep),
    /// The feed relayed the SJK team's news.
    News(crate::feed::FeedNews),
    /// The viewer wants fresh rewards.
    RefreshRewards,
    /// Claim this reward.
    ClaimReward(u64),
    /// The SJK team's announcement.
    Announce(AnnounceRequest),
    /// A news picture for an announcement: its bytes and type.
    UploadNewsImage(Vec<u8>, &'static str),
    Stop,
}

fn lock_chat(chat: &Mutex<ChatState>) -> MutexGuard<'_, ChatState> {
    crate::feed::lock(chat)
}

fn lock_feed(feed: &Mutex<FeedShared>) -> MutexGuard<'_, FeedShared> {
    crate::feed::lock(feed)
}

/// Notes whose pictures may still come, kept by tag: the newest few only.
const NOTES_AWAITING_PICTURES: usize = 8;

/// Builds the hub client for an address; `Err` for an address that cannot be used.
pub type HubFactory = Box<dyn Fn(&str) -> Result<Box<dyn Hub>, HubError> + Send>;

/// The worker's logic, advanced by [`Worker::handle`] and [`Worker::tick`] with
/// an explicit clock so it can be tested without waiting.
struct Worker {
    identity: Identity,
    make_hub: HubFactory,
    settings: Settings,
    hub: Option<Box<dyn Hub>>,
    location: Option<Location>,
    /// The server address the hub holds a claim for, to withdraw it.
    claimed: Option<String>,
    registered: bool,
    /// When the hub last took a chat message from the player, for the wait of a key
    /// that is neither verified nor staff ([`UNVERIFIED_CHAT_GAP`]).
    chat_sent: Option<Instant>,
    /// The in-game name the player wears, and the one the hub was last told.
    name: Option<String>,
    name_sent: Option<String>,
    snapshot: Arc<Mutex<Snapshot>>,
    due_register: Instant,
    due_name: Instant,
    due_claim: Instant,
    due_poll: Instant,
    due_profile: Instant,
    backoff: Duration,
    lookups: Vec<String>,
    /// Notes the hub took, as (tag, hub id), while their pictures may come.
    notes_sent: Vec<(u64, i64)>,
    /// The achievement counts the client keeps, the last ones the hub was sent, when
    /// they may be sent next, and when counts the hub held back may go again unchanged.
    counts: BTreeMap<String, u64>,
    counts_sent: Option<BTreeMap<String, u64>>,
    due_counts: Instant,
    counts_held_until: Instant,
    /// The player models the game has, the last list the hub took, and when the list
    /// may be sent next.
    models: Option<Vec<String>>,
    models_sent: Option<Vec<String>>,
    due_models: Instant,
    /// Model questions waiting for the hub.
    model_checks: Vec<(SocketAddr, u8, String)>,
    /// What staff requests brought back.
    staff: Arc<Mutex<StaffState>>,
    /// Whether the SJK chat is on (`cl_sjkChat`).
    chat_on: bool,
    /// The clan chats the player joined ([`crate::channels`]).
    clan_chats: Vec<crate::HubChannel>,
    /// What the feed thread is told to read.
    feed: Arc<Mutex<FeedShared>>,
    /// What the chat shows; the worker writes the outcome of what it sends.
    chat: Arc<Mutex<ChatState>>,
    /// What became of the last ping sent ([`Service::ping_outcome`]).
    ping_outcome: Arc<Mutex<Option<ReportOutcome>>>,
    /// The look the viewer last gave.
    look: Look,
    /// The newest look [`Service::set_look`] gave, not yet taken: the worker reads it
    /// between its other hub calls too, so a slow one does not hold a toy back for a
    /// whole round (the cardboard box's slow sync, 31/10/2026).
    latest_look: Arc<Mutex<Option<Look>>>,
    /// Since when the look waits to reach the hub (logged with its outcome).
    look_since: Option<Instant>,
    /// The claim (server, slot and name) the hub accepted last, while it holds it: a
    /// look lives on it.
    look_claim: Option<(String, u8, String)>,
    /// The look the hub holds for the key: on the accepted claim, or, with none
    /// accepted now, on a claim that may still be live.
    look_held: Held,
    /// When the next look may go.
    due_look: Instant,
    /// Blade skins the hub said the key does not hold, and the unlocks the profile
    /// listed then: they are not sent again until those change.
    look_refused: Vec<String>,
    look_refused_for: Vec<Unlock>,
    /// The hub refused `saber2` as an unknown field (a hub from before 21/10/2026): looks
    /// go without the second saber's skin until then, or until the hub changes.
    saber2_unknown: Option<Instant>,
    /// The hub refused `saber_on` or `saber2_on` as unknown fields (a hub from before
    /// 26/10/2026): looks go without the extras until then, or until the hub changes.
    extras_unknown: Option<Instant>,
    /// The hub refused a look's `tag` as an unknown field (a hub from before 27/10/2026):
    /// looks go without the tag until then, or until the hub changes.
    tag_unknown: Option<Instant>,
    /// The hub refused a look's `saber_sounds` as an unknown field (a hub from before
    /// 23/10/2026): looks go without it, so the blade and the rest still sync and other
    /// players hear the sounds that follow the blade, until the hub changes.
    look_sounds_unsupported: bool,
    /// The hub refused a look's `finisher` as an unknown field (a hub from before
    /// 04/11/2026): looks go without it until the hub changes.
    finisher_unsupported: bool,
    /// The feed id of the last look the hub took from this worker.
    look_id: Option<u64>,
    /// When the own profile may next be read early ([`Worker::profile_soon`]).
    profile_soon_after: Instant,
    /// The folder the asset packs are kept in; without one none is asked for.
    assets_dir: Option<PathBuf>,
    /// The folder map edits are kept in, and when the hub's list is next read.
    map_edits_dir: Option<PathBuf>,
    due_map_edits: Instant,
    /// The hub's map edits stamp the last presence answer carried.
    map_edits_stamp: Option<i64>,
    /// The hub's Toy Master stamp the last presence answer carried, with its server.
    secrets_stamp: Option<(String, i64)>,
    /// Serial of the last map edits publication.
    map_edits_serial: u64,
    /// When the hub's mystery boxes are next read, and the last request's serial.
    due_mystery_boxes: Instant,
    mystery_box_serial: u64,
    /// The last junk sale's serial.
    junk_serial: u64,
    box_hunt_serial: u64,
    /// When the Toy Master's toys are next read, and the last request's serial.
    due_secrets: Instant,
    secret_serial: u64,
    /// The folder crash reports wait in; without one none is sent.
    crash_dir: Option<PathBuf>,
    /// When the crash folder is next looked at.
    due_crashes: Instant,
    /// When the asset packs are next checked, and the wait after the next shortfall.
    due_assets: Instant,
    assets_backoff: Duration,
    /// Whether the player is actively playing: sent with every claim.
    active: bool,
    /// What the player did since the last accepted claim (03/11/2026), sent with an
    /// active claim and cleared once the hub accepted one.
    play: crate::wire::PlaySignals,
    /// When the progress toward the next holocron is next read, and when it may next
    /// be read early.
    due_holocrons: Instant,
    holocrons_soon_after: Instant,
    /// When the player's trades are next read: soon after registering, every
    /// [`TRADES_LIVE`] while one runs, else every [`TRADES_EVERY`].
    due_trades: Instant,
    /// When the requests for the player's logs are next looked for.
    due_log_requests: Instant,
    /// Log requests the player answered, kept out of what the hub lists until it
    /// stops listing them.
    logs_answered: Vec<i64>,
    /// When the announcements and the rewards are next read, and when the rewards may
    /// next be read early.
    due_announcements: Instant,
    due_rewards: Instant,
    rewards_soon_after: Instant,
    /// When the game menu may next bring the profile, holocrons and trades forward,
    /// and the achievement counts.
    menu_soon_after: Instant,
    menu_counts_after: Instant,
}

/// What the hub holds as the player's look.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Held {
    /// Not known: the worker just started (a client that stopped without releasing may
    /// have left a live claim with a look), a claim failed (the last one may still be
    /// live, its look with it) or a release failed. The look goes on the next accepted
    /// claim whatever it is.
    Unknown,
    /// This look; [`Look::default`] for none, as a new claim starts.
    Look(Box<Look>),
}

impl Held {
    /// No look: what a new claim starts with.
    fn none() -> Self {
        Self::Look(Box::default())
    }
}

/// What the player is told of a report or a note: what the hub stored it as, or why
/// it did not.
/// A hub from before pictures has no `/v1/avatar`: say so in words rather than as the
/// bare status its router answers with.
fn older_hub(error: HubError) -> HubError {
    match error {
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            HubError::Rejected {
                status: 404,
                code: "no_pictures".to_owned(),
                message: "this SJK hub does not take pictures yet".to_owned(),
            }
        }
        other => other,
    }
}

/// What the player reads when the hub wants a newer SJK (`client_update_required`): the
/// hub's words, which name the release, or a plain notice when it sent none.
fn update_notice(message: &str) -> String {
    if message.trim().is_empty() {
        UPDATE_NOTICE.to_owned()
    } else {
        message.to_owned()
    }
}

/// [`update_notice`] when the hub's message is empty.
const UPDATE_NOTICE: &str = "Update SJK to open holocrons: this version is too old.";

/// What the player reads when the hub did not open the holocrons.
fn opening_refusal(error: &HubError) -> String {
    match error {
        HubError::Rejected { code, message, .. } if code == "client_update_required" => {
            update_notice(message)
        }
        HubError::Rejected { code, .. } if code == "holocron_opened" => {
            "That holocron was already opened.".to_owned()
        }
        HubError::Rejected { code, .. } if code == "not_found" => {
            "That holocron is not yours any more.".to_owned()
        }
        HubError::Rejected { message, .. } => message.clone(),
        // A hub from before opening has no such route.
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub cannot open holocrons yet.".to_owned()
        }
        other => other.to_string(),
    }
}

/// Why a shatter, a craft, a purchase or a read of the shop failed, in the player's words.
fn trade_refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "not_a_duplicate" => "That is your last one: only duplicates can be shattered.",
            "comes_with_card" => "That shader comes with your card: it stays with the card.",
            "not_held" => "You hold no copy of that shader any more.",
            "no_duplicates" => "You hold no duplicates to break down.",
            "locked" => "That shader is locked: unlock it first.",
            "not_shatterable" => "That shader cannot be shattered.",
            "not_craftable" => "That shader cannot be crafted.",
            "not_enough_shards" => "Not enough kyber shards.",
            "already_owned" => "You have that shader already.",
            "not_on_offer" => "That shader is not in the shop any more: the offers changed.",
            "not_enough_credits" => "Not enough credits.",
            "already_bought" => "You bought that one today already.",
            "not_for_sale" => "That holocron is not for sale.",
            "opening_staff_only" => "Shards and the shop open with holocrons: soon.",
            "client_update_required" => return update_notice(message),
            _ => return message.clone(),
        },
        // A hub from before the economy has no such route.
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no shop yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

/// Why the hub refused a tag copies request, in the player's words.
fn tag_refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "not_a_duplicate" => "You keep one copy of every tag: only duplicates go.",
            "not_held" => "You hold no copy of that tag in that finish any more.",
            "not_a_tag" => "That is not a tag that can be broken down or crafted.",
            "bad_finish" => "That finish is not one the hub knows.",
            "finishes_differ" => "Both copies must be of the same finish.",
            "tiers_differ" => "Both copies must be of the same tier.",
            "nothing_new" => "You already have every tag of that tier in that finish.",
            "opening_staff_only" => "Shards and the shop open with holocrons: soon.",
            "client_update_required" => return update_notice(message),
            _ => return message.clone(),
        },
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub cannot break down or craft tags yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

/// Show `mark` set (`on`) or cleared on shader `id` in `snapshot` before the hub answers;
/// whether the hub is to be asked (its marks are known and this changes them).
fn mark_at_once(snapshot: &mut Snapshot, id: &str, mark: crate::wire::Mark, on: bool) -> bool {
    let Some(marks) = snapshot.marks.as_mut() else {
        return false;
    };
    if !marks.set(id, mark, on) {
        return false;
    }
    snapshot.marks_revision += 1;
    true
}

/// Why a change of a collection mark failed, in the player's words.
fn mark_refusal(error: &HubError) -> String {
    let words = match error {
        HubError::Rejected { code, message, .. } => match code.as_str() {
            "not_held" => "Only a shader you hold can be locked.",
            "bad_unlock" => "The hub does not know that shader yet.",
            "not_registered" => "Your identity is not registered at the hub yet.",
            _ => return message.clone(),
        },
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub keeps no favourites or locks yet."
        }
        other => return other.to_string(),
    };
    words.to_owned()
}

/// Why a reveal failed, as the Cards tab says it.
fn reveal_refusal(error: &HubError) -> String {
    match error {
        HubError::Rejected { code, message, .. } if code == "client_update_required" => {
            update_notice(message)
        }
        HubError::Rejected { message, .. } => message.clone(),
        // A hub from before the card catalogue has no such route.
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub cannot reveal cards yet.".to_owned()
        }
        other => other.to_string(),
    }
}

/// The second saber's own blade skin in `look`, when it wears one (not the first's, not the
/// stock blade).
fn second_skin(look: &Look) -> Option<&str> {
    look.saber2.as_deref().filter(|saber2| !saber2.is_empty())
}

/// The blade skin of `look` a hub's refusal `code` names: the second saber's for
/// `saber2_not_unlocked` and `bad_saber2`, the first's for `not_unlocked` and `bad_look`
/// (`saber`, or the second's when the first wears none: a hub before the side was named
/// answered so for either). `None` when the code names no skin the look wears.
fn refused_skin<'a>(code: &str, look: &'a Look) -> Option<&'a str> {
    let first = Some(look.saber.as_str()).filter(|saber| !saber.is_empty());
    match code {
        "saber2_not_unlocked" | "bad_saber2" => second_skin(look),
        "not_unlocked" | "bad_look" => first.or_else(|| second_skin(look)),
        _ => None,
    }
}

fn outcome_of(serial: u64, outcome: Result<String, HubError>) -> ReportOutcome {
    match outcome {
        Ok(message) => ReportOutcome {
            serial,
            sent: true,
            message,
        },
        Err(HubError::Rejected { message, .. }) => ReportOutcome {
            serial,
            sent: false,
            message,
        },
        Err(error) => ReportOutcome {
            serial,
            sent: false,
            message: error.to_string(),
        },
    }
}

fn lock(snapshot: &Mutex<Snapshot>) -> MutexGuard<'_, Snapshot> {
    snapshot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// How long a key the hub neither verifies nor made staff waits between two SJK chat
/// messages (the hub's rule, `PROTOCOL.md`, "Chat").
const UNVERIFIED_CHAT_GAP: Duration = Duration::from_secs(30);

impl Worker {
    fn new(
        identity: Identity,
        make_hub: HubFactory,
        snapshot: Arc<Mutex<Snapshot>>,
        staff: Arc<Mutex<StaffState>>,
        now: Instant,
    ) -> Self {
        Self {
            identity,
            make_hub,
            settings: Settings::default(),
            hub: None,
            location: None,
            claimed: None,
            registered: false,
            chat_sent: None,
            name: None,
            name_sent: None,
            snapshot,
            due_register: now,
            due_name: now,
            due_claim: now,
            due_poll: now,
            due_profile: now,
            backoff: RETRY_MIN,
            lookups: Vec::new(),
            notes_sent: Vec::new(),
            counts: BTreeMap::new(),
            counts_sent: None,
            due_counts: now,
            counts_held_until: now,
            models: None,
            models_sent: None,
            due_models: now,
            model_checks: Vec::new(),
            staff,
            chat_on: false,
            clan_chats: Vec::new(),
            feed: Arc::default(),
            chat: Arc::default(),
            ping_outcome: Arc::default(),
            look: Look::default(),
            latest_look: Arc::default(),
            look_since: None,
            look_claim: None,
            look_held: Held::Unknown,
            due_look: now,
            look_refused: Vec::new(),
            look_sounds_unsupported: false,
            finisher_unsupported: false,
            look_refused_for: Vec::new(),
            saber2_unknown: None,
            extras_unknown: None,
            tag_unknown: None,
            look_id: None,
            profile_soon_after: now,
            assets_dir: None,
            map_edits_dir: None,
            due_map_edits: now,
            map_edits_stamp: None,
            secrets_stamp: None,
            map_edits_serial: 0,
            due_mystery_boxes: now,
            mystery_box_serial: 0,
            junk_serial: 0,
            box_hunt_serial: 0,
            due_secrets: now,
            secret_serial: 0,
            crash_dir: None,
            due_crashes: now,
            due_assets: now,
            assets_backoff: ASSETS_RETRY_MIN,
            active: false,
            play: crate::wire::PlaySignals::default(),
            due_holocrons: now,
            holocrons_soon_after: now,
            due_trades: now,
            due_log_requests: now,
            logs_answered: Vec::new(),
            due_announcements: now,
            due_rewards: now,
            rewards_soon_after: now,
            menu_soon_after: now,
            menu_counts_after: now,
        }
    }

    /// Read the holocron progress soon, at most once every [`HOLOCRONS_SOON`], and
    /// never later than it was due anyway.
    fn holocrons_soon(&mut self, now: Instant) {
        let at = self.holocrons_soon_after.max(now);
        if at < self.due_holocrons {
            self.due_holocrons = at;
            self.holocrons_soon_after = at + HOLOCRONS_SOON;
        }
    }

    /// The game menu opened: what its pages show of the hub (the profile with its
    /// achievements and medals, the holocron progress, the trades) is read now, at most
    /// once every [`MENU_SOON`], and changed achievement counts are sent, at most once
    /// every [`MENU_COUNTS_SOON`]. A hub that refused a reading or the counts (an older
    /// hub, a quota) keeps its longer wait.
    fn menu_opened(&mut self, now: Instant) {
        if now >= self.menu_soon_after {
            self.menu_soon_after = now + MENU_SOON;
            self.due_profile = self.due_profile.min(now);
            self.due_trades = self.due_trades.min(now);
            if self.due_holocrons <= now + HOLOCRONS_EVERY {
                self.due_holocrons = self.due_holocrons.min(now);
            }
        }
        if now >= self.menu_counts_after
            && self.counts_sent.as_ref() != Some(&self.counts)
            && self.due_counts <= now + ACHIEVEMENTS_EVERY
        {
            self.menu_counts_after = now + MENU_COUNTS_SOON;
            self.due_counts = self.due_counts.min(now);
        }
    }

    /// A holocron dropped for the player's own key: their profile lists it and their
    /// progress changed, so both are read again soon.
    fn own_drop(&mut self, now: Instant) {
        self.profile_soon(now);
        self.holocrons_soon(now);
    }

    /// Read the own profile soon: its unlocks look out of date. At most once every
    /// [`PROFILE_SOON`], and never later than it was due anyway.
    fn profile_soon(&mut self, now: Instant) {
        let at = self.profile_soon_after.max(now);
        if at < self.due_profile {
            self.due_profile = at;
            self.profile_soon_after = at + PROFILE_SOON;
        }
    }

    /// The feed relayed look `id` of the player's own key, with blade skin `saber`.
    /// One the hub made after the last this worker sent, wearing another skin than
    /// the one to send, is the hub taking the skin back (staff relocked it): the
    /// profile's unlocks are out of date.
    fn own_look_seen(&mut self, id: u64, saber: &str, now: Instant) {
        if self.look_id.is_none_or(|sent| id > sent) && saber != self.look_to_send().saber {
            self.profile_soon(now);
        }
    }

    /// Tell the feed thread what to read: the hub, only while the identity is on and
    /// the hub answered the registration, and then on a game server (for its looks
    /// and emotes, whatever the chat says) or with the chat on; the server played on;
    /// and whether the chat shows. Another hub or server, or none, is a new
    /// generation: the looks read before are dropped.
    fn publish_feed(&self) {
        let reading = self.chat_on || self.location.is_some();
        let url = (self.settings.enabled && self.registered && reading && self.hub.is_some())
            .then(|| self.settings.hub_url.trim().to_owned());
        let wanted = FeedShared {
            url,
            server: self
                .location
                .as_ref()
                .map(|location| location.server.to_string()),
            chat: self.chat_on,
            channels: self.clan_chats.clone(),
            person: lock(&self.snapshot).key_id.clone(),
            generation: 0,
        };
        let mut shared = lock_feed(&self.feed);
        let generation = if (&shared.url, &shared.server) == (&wanted.url, &wanted.server) {
            shared.generation
        } else {
            shared.generation + 1
        };
        let wanted = FeedShared {
            generation,
            ..wanted
        };
        if *shared != wanted {
            *shared = wanted;
        }
    }

    /// The refusal of anything sent before the hub answered the registration, saying
    /// why: the identity off, no hub, or (most often) the registration not answered
    /// yet, just after the game starts or while a failed one waits to try again. Only
    /// the first is the player's to fix ("is identity on, cl_identity 1?" was shown
    /// to verified players who joined a server too soon).
    fn not_connected(&self) -> HubError {
        let message = if !self.settings.enabled {
            "the SJK identity is off: turn it on with cl_identity 1".to_owned()
        } else if self.hub.is_none() {
            "no SJK hub to send to: check cl_hubUrl".to_owned()
        } else if let Status::Failed(why) = &lock(&self.snapshot).status {
            format!("not connected to the hub yet ({why}): it tries again shortly")
        } else {
            "not connected to the hub yet: try again in a moment".to_owned()
        };
        HubError::NotConnected(message)
    }

    /// Put what became of a message or an emote where the chat shows it.
    fn chat_outcome(&self, outcome: Result<String, HubError>) {
        let mut chat = lock_chat(&self.chat);
        let serial = chat.outcome.as_ref().map_or(1, |last| last.serial + 1);
        chat.outcome = Some(outcome_of(serial, outcome));
    }

    /// Send an SJK chat message under the rules, once registered. A key the hub neither
    /// verifies nor made staff sends no links (GIFs are links) and one message every
    /// [`UNVERIFIED_CHAT_GAP`]: both are refused here, before the hub refuses them too.
    fn send_reaction(&mut self, message: u64, emoji: &str, active: bool) {
        let outcome = if !crate::reactions::valid(emoji) {
            Err(HubError::Protocol("Unknown chat reaction".into()))
        } else {
            match (self.hub.as_mut(), self.registered) {
                (Some(hub), true) => hub
                    .react(&self.identity, message, emoji, active)
                    .map(|_| String::new()),
                _ => Err(self.not_connected()),
            }
        };
        let mut chat = lock_chat(&self.chat);
        let serial = chat.reaction.as_ref().map_or(1, |last| last.serial + 1);
        chat.reaction = Some(outcome_of(serial, outcome));
    }

    fn send_chat(&mut self, text: &str, channel: crate::HubChannel, now: Instant) {
        let vouched = lock(&self.snapshot)
            .me
            .as_ref()
            .is_some_and(|me| me.verified || me.staff);
        let refused = |code: &str, message: String| {
            Err(HubError::Rejected {
                status: 403,
                code: code.to_owned(),
                message,
            })
        };
        let wait = self
            .chat_sent
            .map(|sent| UNVERIFIED_CHAT_GAP.saturating_sub(now.saturating_duration_since(sent)))
            .filter(|wait| !vouched && !wait.is_zero());
        let outcome = match crate::chat::check(text) {
            Err(error) => Err(HubError::Rejected {
                status: 400,
                code: error.code().to_owned(),
                message: error.message().to_owned(),
            }),
            Ok(text) if !vouched && crate::chat::has_link(&text) => refused(
                "chat_links",
                "Only verified players can send links and GIFs in SJK chat".to_owned(),
            ),
            Ok(_) if let Some(wait) = wait => refused(
                "chat_quota",
                format!(
                    "Players who are not verified send one message every 30 seconds: wait {} s",
                    wait.as_secs_f32().ceil() as u64
                ),
            ),
            Ok(text) => match (self.hub.as_mut(), self.registered) {
                (Some(hub), true) => {
                    let name = self.name.clone().unwrap_or_default();
                    let sent = hub.chat(&self.identity, &text, &name, channel);
                    if sent.is_ok() {
                        self.chat_sent = Some(now);
                    }
                    sent.map(|_| String::new())
                }
                _ => Err(self.not_connected()),
            },
        };
        self.chat_outcome(outcome);
    }

    /// Ban the player of `key_id` from the SJK chat, once registered, from a key the
    /// hub vouches for (verified or staff; the hub refuses others too).
    fn send_chat_ban(&mut self, key_id: String) {
        let trusted = lock(&self.snapshot)
            .me
            .as_ref()
            .is_some_and(|me| me.verified || me.staff);
        let result = match (self.hub.as_mut(), self.registered, trusted) {
            (Some(hub), true, true) => hub.chat_ban(&self.identity, &key_id),
            (Some(_), true, false) => Err(HubError::Rejected {
                status: 403,
                code: "not_verified".to_owned(),
                message: "only verified SJK players can ban from the chat".to_owned(),
            }),
            _ => Err(self.not_connected()),
        };
        let mut chat = lock_chat(&self.chat);
        let serial = chat.ban.as_ref().map_or(1, |last| last.serial + 1);
        chat.ban = Some(crate::BanOutcome {
            serial,
            key_id,
            result,
        });
    }

    /// Send an emote for the slot the player claims on their server, in step with
    /// slot `sync`'s when it is set.
    fn send_emote(&mut self, emote: &str, sync: Option<u8>, quiet: bool) {
        let well_formed = (1..=32).contains(&emote.len())
            && emote
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        let server = self
            .location
            .as_ref()
            .map(|location| location.server.to_string());
        let outcome = match (self.hub.as_mut(), self.registered, server) {
            _ if !well_formed => Err(HubError::Rejected {
                status: 400,
                code: "bad_emote".to_owned(),
                message: "an emote is 1 to 32 of a to z, 0 to 9 and _".to_owned(),
            }),
            (Some(_), true, None) => Err(HubError::Rejected {
                status: 403,
                code: "not_on_server".to_owned(),
                message: "join a server to emote".to_owned(),
            }),
            // A hub from before 15/10/2026 refuses `sync`, one from before 19/10/2026
            // `quiet` (`bad_body`): the emote still goes, out of step or with its sound.
            (Some(hub), true, Some(server)) => {
                match hub.emote(&self.identity, &server, emote, sync, quiet) {
                    Err(HubError::Rejected { code, .. })
                        if (sync.is_some() || quiet) && code == "bad_body" =>
                    {
                        hub.emote(&self.identity, &server, emote, None, false)
                    }
                    sent => sent,
                }
                .map(|_| String::new())
            }
            _ => Err(self.not_connected()),
        };
        self.chat_outcome(outcome);
    }

    /// Send a ping from the slot the player claims on their server; what became of it
    /// goes to [`Service::ping_outcome`].
    fn send_ping(&mut self, pos: [i32; 3], scope: &str) {
        let server = self
            .location
            .as_ref()
            .map(|location| location.server.to_string());
        let outcome = match (self.hub.as_mut(), self.registered, server) {
            (Some(_), true, None) => Err(HubError::Rejected {
                status: 403,
                code: "not_on_server".to_owned(),
                message: "join a server to ping".to_owned(),
            }),
            (Some(hub), true, Some(server)) => hub
                .ping(&self.identity, &server, pos, scope)
                .map(|_| String::new()),
            _ => Err(self.not_connected()),
        };
        let mut last = crate::feed::lock(&self.ping_outcome);
        let serial = last.as_ref().map_or(1, |last| last.serial + 1);
        *last = Some(outcome_of(serial, outcome));
    }

    /// Send a toy event from the slot the player claims on their server. Its effect was
    /// already shown here, so what the hub answers is not kept.
    fn send_toy_event(&mut self, toy: &str, event: &str, target: u8) {
        let server = self
            .location
            .as_ref()
            .map(|location| location.server.to_string());
        if let (Some(hub), true, Some(server)) = (self.hub.as_mut(), self.registered, server) {
            let _ = hub.toy_event(&self.identity, &server, toy, event, target);
        }
    }

    fn update(&self, change: impl FnOnce(&mut Snapshot)) {
        change(&mut lock(&self.snapshot));
    }

    fn failed(&mut self, error: &HubError, now: Instant) -> Instant {
        self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()));
        let due = now + self.backoff;
        self.backoff = (self.backoff * 2).min(RETRY_MAX);
        due
    }

    /// Withdraw the claim the hub holds, if any, ignoring a failure: it expires. The
    /// look on it goes with it; after a failure the hub may hold both a while longer.
    fn release(&mut self) {
        match (self.claimed.take(), self.hub.as_mut()) {
            (Some(server), Some(hub)) => {
                self.look_held = match hub.release(&self.identity, &server) {
                    Ok(()) => Held::none(),
                    Err(_) => Held::Unknown,
                };
            }
            (Some(_), None) => self.look_held = Held::Unknown,
            (None, _) => {}
        }
        self.look_claim = None;
        self.update(|snapshot| {
            snapshot.server = None;
            snapshot.players.clear();
            snapshot.map_blocks.clear();
            snapshot.revision += 1;
        });
    }

    fn handle(&mut self, command: Command, now: Instant) {
        match command {
            Command::Configure(settings) => self.configure(settings, now),
            Command::Enter(location) => {
                if self.location.as_ref() == Some(&location) {
                    return;
                }
                if self.location.as_ref().map(|old| old.server) != Some(location.server) {
                    self.release();
                }
                self.update(|snapshot| snapshot.server = Some(location.server));
                self.location = Some(location);
                self.due_claim = now;
                self.due_poll = now;
            }
            Command::Leave => {
                self.release();
                self.location = None;
            }
            Command::Name(name) => {
                let name = Some(name).filter(|name| !name.trim().is_empty());
                if name != self.name {
                    self.name = name;
                    self.due_name = now;
                }
            }
            Command::SetBio(bio) => self.set_bio(&bio),
            Command::SetBanner(id) => self.set_banner(&id),
            Command::SetEmoji(emoji) => self.set_emoji(&emoji),
            Command::Achievements(counts) => self.counts = counts,
            Command::Staff(request) => self.staff(&request),
            Command::StaffQuery(query) => self.staff_query(&query),
            Command::AnswerLogs(id, upload) => self.answer_logs(id, upload.as_ref()),
            Command::Report(mut report) => {
                if report.name.is_empty() {
                    report.name = self.name.clone().unwrap_or_default();
                }
                self.report(&report);
            }
            Command::PlayerReport(mut report) => {
                if report.name.is_empty() {
                    report.name = self.name.clone().unwrap_or_default();
                }
                self.player_report(&report);
            }
            Command::Note(tag, mut note) => {
                if note.name.is_empty() {
                    note.name = self.name.clone().unwrap_or_default();
                }
                self.note(tag, &note);
            }
            Command::NoteImage(tag, jpeg) => self.note_image(tag, &jpeg),
            Command::LookUp(key_id) => self.lookups.push(key_id),
            Command::Models(models) => {
                if self.models.as_ref() != Some(&models) {
                    self.models = Some(models);
                    self.due_models = now;
                }
            }
            Command::CheckModel(server, slot, model) => {
                let question = (server, slot, model);
                if !self.model_checks.contains(&question) {
                    self.model_checks.push(question);
                }
            }
            Command::Chat(text) => self.send_chat(&text, crate::HubChannel::Sjk, now),
            Command::ClanChat(text, channel) => self.send_chat(&text, channel, now),
            Command::SetClanChats(channels) => self.clan_chats = channels,
            Command::React {
                message,
                emoji,
                active,
            } => self.send_reaction(message, &emoji, active),
            Command::ChatBan(key_id) => self.send_chat_ban(key_id),
            Command::Emote(emote, sync, quiet) => self.send_emote(&emote, sync, quiet),
            Command::Ping(pos, scope) => self.send_ping(pos, &scope),
            Command::Toy(toy, event, target) => self.send_toy_event(&toy, &event, target),
            Command::SetChat(on) => self.chat_on = on,
            Command::SetAvatar(png) => self.change_avatar(Some(&png)),
            Command::RemoveAvatar => self.change_avatar(None),
            Command::Look(look) => self.wear(look, now),
            Command::LookWaiting => self.take_latest_look(now),
            Command::OwnLook(id, saber) => self.own_look_seen(id, &saber, now),
            Command::AssetsDir(dir) => {
                if self.assets_dir.as_ref() != Some(&dir) {
                    self.assets_dir = Some(dir);
                    self.due_assets = now;
                    self.assets_backoff = ASSETS_RETRY_MIN;
                }
            }
            Command::MapEditsDir(dir) => {
                if self.map_edits_dir.as_ref() != Some(&dir) {
                    self.map_edits_dir = Some(dir);
                    self.due_map_edits = now;
                }
            }
            Command::RefreshMapEdits => self.due_map_edits = now,
            Command::PublishMapEdits {
                map,
                checksum,
                edits,
            } => self.publish_map_edits(&map, checksum, &edits, now),
            Command::OpenMysteryBox { id, server } => self.mystery_box(now, |hub, identity| {
                hub.open_mystery_box(identity, id, &server)
                    .map(|answer| (BoxDone::Opened(answer.opened), Some(answer.profile)))
            }),
            Command::PlaceMysteryBox(placed) => self.mystery_box(now, |hub, identity| {
                hub.place_mystery_box(identity, &placed)
                    .map(|id| (BoxDone::Placed(id), None))
            }),
            Command::BoxHunt { server, away } => self.box_hunt(&server, None, away),
            Command::BoxHuntKilled { server, by, away } => {
                self.box_hunt(&server, Some(HuntReport::Killed(by)), away);
            }
            Command::BoxHuntOut { server, out, away } => {
                self.box_hunt(&server, Some(HuntReport::Out(out)), away);
            }
            Command::RemoveMysteryBox(id) => self.mystery_box(now, |hub, identity| {
                hub.remove_mystery_box(identity, id)
                    .map(|_| (BoxDone::Removed(id), None))
            }),
            Command::ReadJunk => self.read_junk(),
            Command::SellJunk(held) => self.sell_junk(&held),
            Command::UseSecret {
                id,
                server,
                round,
                presses,
            } => self.secret(now, |hub, identity| {
                hub.use_secret(identity, id, &server, round, presses)
                    .map(|used| {
                        let done = SecretDone::Used {
                            id,
                            show: used.show,
                            done: used.done,
                            found: used.profile.is_some(),
                        };
                        (done, used.profile)
                    })
            }),
            Command::TakeSecretDrop {
                id,
                server,
                round,
                drop,
            } => self.secret(now, |hub, identity| {
                hub.take_secret_drop(identity, id, &server, round, drop)
                    .map(|took| {
                        let done = SecretDone::Took {
                            id,
                            drop,
                            kind: took.kind,
                            amount: took.amount,
                            tier: took.tier,
                        };
                        (done, took.profile)
                    })
            }),
            Command::PlaceSecret(placing) => self.secret(now, |hub, identity| {
                hub.place_secret(identity, &placing)
                    .map(|id| (SecretDone::Placed(id), None))
            }),
            Command::RemoveSecret(id) => self.secret(now, |hub, identity| {
                hub.remove_secret(identity, id)
                    .map(|_| (SecretDone::Removed(id), None))
            }),
            Command::EscortSecret(id, escorted) => self.secret(now, |hub, identity| {
                hub.escort_secret(identity, id, escorted)
                    .map(|_| (SecretDone::Escorted(id, escorted), None))
            }),
            Command::Crashes(dir) => {
                if self.crash_dir != dir {
                    self.crash_dir = dir;
                    self.due_crashes = now;
                }
            }
            Command::Active(active) => self.active = active,
            Command::Play(play) => self.play.add(&play),
            Command::ShatterTag(copy) => self.tag_trade(TagTrade::Shatter(copy)),
            Command::CraftTags(copies) => self.tag_trade(TagTrade::Craft(copies)),
            Command::OwnDrop => self.own_drop(now),
            Command::RefreshHolocrons => self.holocrons_soon(now),
            Command::MenuOpened => self.menu_opened(now),
            Command::OpenHolocrons(ids) => self.open_holocrons(&ids, now),
            Command::RevealCards => self.reveal_cards(),
            Command::Shatter(id) => self.trade(Trade::Shatter, &id),
            Command::ShatterLast(id) => self.trade(Trade::ShatterLast, &id),
            Command::ShatterDuplicates => self.trade(Trade::ShatterAll, ""),
            Command::Craft(id) => self.trade(Trade::Craft, &id),
            Command::Mark(id, mark, on) => self.set_mark(&id, mark, on, now),
            Command::ReadShop => self.read_shop(now),
            Command::Buy(id) => self.trade(Trade::Buy, &id),
            Command::BuyHolocron(tier) => self.trade(Trade::Holocron, &tier),
            Command::ReadTrades => self.due_trades = now,
            Command::ReadHistory(before) => self.read_history(before),
            Command::TradeAsk(to) => self.trade_with(None, Some(&to), now),
            Command::TradeStep(id, step) => self.trade_with(Some((id, step)), None, now),
            Command::News(crate::feed::FeedNews::Announcements(list)) => {
                self.update(|snapshot| merge_announcements(snapshot, list));
            }
            Command::News(crate::feed::FeedNews::Round) | Command::RefreshRewards => {
                self.rewards_soon(now);
            }
            Command::ClaimReward(id) => self.claim_reward(id, now),
            Command::Announce(request) => self.announce(&request),
            Command::UploadNewsImage(bytes, mime) => self.upload_news_image(&bytes, mime),
            Command::Stop => self.release(),
        }
        self.publish_feed();
    }

    fn configure(&mut self, settings: Settings, now: Instant) {
        if settings == self.settings {
            return;
        }
        // Anything that changes the hub or switches the feature off ends the claim
        // first, on the hub that holds it.
        self.release();
        self.hub = None;
        self.look_sounds_unsupported = false;
        self.finisher_unsupported = false;
        self.registered = false;
        self.name_sent = None;
        self.saber2_unknown = None;
        self.extras_unknown = None;
        self.tag_unknown = None;
        self.update(Snapshot::clear_me);
        // Another hub's news, or none; a failure from before is not this hub's, and
        // the next tick says where the new settings stand.
        self.update(|snapshot| {
            if matches!(snapshot.status, Status::Failed(_)) {
                snapshot.status = Status::Registering;
            }
            if !snapshot.announcements.is_empty() {
                snapshot.announcements.clear();
                snapshot.announcements_revision += 1;
            }
            if snapshot.rewards.take().is_some() {
                snapshot.rewards_revision += 1;
            }
            // Another key's or hub's history; its count goes on, for the readers waiting.
            if let Some(history) = snapshot.history.as_mut() {
                let serial = history.serial;
                *history = Arc::new(crate::history::HistoryState {
                    serial,
                    ..Default::default()
                });
            }
        });
        if settings.enabled && !settings.hub_url.trim().is_empty() {
            match (self.make_hub)(settings.hub_url.trim()) {
                Ok(hub) => self.hub = Some(hub),
                Err(error) => {
                    self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()))
                }
            }
        }
        self.settings = settings;
        self.backoff = RETRY_MIN;
        self.due_register = now;
        self.due_claim = now;
        self.due_poll = now;
    }

    fn note(&mut self, tag: u64, note: &WorldNote) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.note(&self.identity, note),
            _ => Err(self.not_connected()),
        };
        if let Ok(id) = outcome {
            if self.notes_sent.len() >= NOTES_AWAITING_PICTURES {
                self.notes_sent.remove(0);
            }
            self.notes_sent.push((tag, id));
        }
        self.update(|snapshot| {
            let serial = snapshot.note.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.note = Some(outcome_of(serial, outcome.map(|id| format!("note #{id}"))));
        });
    }

    /// Attach `jpeg` to the note tagged `tag`, if the hub took that note. A picture
    /// that fails leaves the note as it is.
    fn note_image(&mut self, tag: u64, jpeg: &[u8]) {
        let Some(index) = self.notes_sent.iter().position(|(sent, _)| *sent == tag) else {
            return;
        };
        let (_, id) = self.notes_sent.remove(index);
        if let Some(hub) = self.hub.as_mut() {
            let _ = hub.note_image(&self.identity, id, jpeg);
        }
    }

    fn report(&mut self, report: &BugReport) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.report(&self.identity, report),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let serial = snapshot.report.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.report = Some(outcome_of(
                serial,
                outcome.map(|id| format!("report #{id}")),
            ));
        });
    }

    /// Send a player report: only once registered, and only with a key the hub's
    /// operator verified (the hub refuses the others too).
    fn player_report(&mut self, report: &PlayerReport) {
        let verified = lock(&self.snapshot)
            .me
            .as_ref()
            .is_some_and(|me| me.verified);
        let outcome = match (self.hub.as_mut(), self.registered, verified) {
            (Some(hub), true, true) => hub.player_report(&self.identity, report),
            (Some(_), true, false) => Err(HubError::Rejected {
                status: 403,
                code: "not_verified".to_owned(),
                message: "only verified SJK players can report players".to_owned(),
            }),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let serial = snapshot
                .player_report
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            snapshot.player_report = Some(outcome_of(
                serial,
                outcome.map(|id| format!("player report #{id}")),
            ));
        });
    }

    /// Send the player's new picture (`Some`, a PNG) or take it down (`None`), once
    /// registered; the profile the hub answers with replaces the player's own.
    fn change_avatar(&mut self, png: Option<&[u8]>) {
        let outcome = match (self.hub.as_mut(), self.registered, png) {
            (_, _, Some(png)) if png.len() > crate::avatar::UPLOAD_MAX => Err(HubError::Rejected {
                status: 413,
                code: "too_large".to_owned(),
                message: "the picture is larger than the hub takes".to_owned(),
            }),
            (Some(hub), true, Some(png)) => hub.set_avatar(&self.identity, png),
            (Some(hub), true, None) => hub.remove_avatar(&self.identity),
            _ => Err(self.not_connected()),
        }
        .map_err(older_hub);
        self.update(|snapshot| {
            let serial = snapshot.avatar.as_ref().map_or(1, |last| last.serial + 1);
            let outcome = outcome.map(|profile| {
                let done = if profile.avatar.is_empty() {
                    "removed"
                } else {
                    "saved"
                };
                snapshot.set_me(profile);
                done.to_owned()
            });
            snapshot.avatar = Some(outcome_of(serial, outcome));
        });
    }

    /// Open holocrons `ids` (the first [`crate::wire::OPEN_MAX`]): the outcome goes in
    /// [`Snapshot::opening`], the hub's profile after it in [`Snapshot::me`]. When the hub
    /// refuses (one was opened meanwhile, from another PC of the person's, or is gone),
    /// the own profile is read again soon so the pages show what is really left.
    fn open_holocrons(&mut self, ids: &[u64], now: Instant) {
        let ids = &ids[..ids.len().min(crate::wire::OPEN_MAX)];
        let outcome = match (self.hub.as_mut(), self.registered) {
            (_, _) if ids.is_empty() => Err(HubError::Rejected {
                status: 400,
                code: "bad_ids".to_owned(),
                message: "there is no holocron to open".to_owned(),
            }),
            (Some(hub), true) => hub.open_holocrons(&self.identity, ids),
            _ => Err(self.not_connected()),
        };
        if let Err(HubError::Rejected { code, .. }) = &outcome
            && (code == "holocron_opened" || code == "not_found")
        {
            self.profile_soon(now);
        }
        // A kyber crystal moves the person's pity counter, which comes with the progress:
        // read it again at once.
        if matches!(&outcome, Ok(answer) if answer.opened.iter().any(|opened| opened.tier.starts_with("kyber_")))
        {
            self.due_holocrons = self.due_holocrons.min(now);
        }
        self.update(|snapshot| {
            let serial = snapshot.opening.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.opening = Some(match outcome {
                Ok(answer) => {
                    snapshot.set_me(answer.profile);
                    OpenOutcome {
                        serial,
                        opened: answer.opened,
                        message: String::new(),
                    }
                }
                Err(error) => OpenOutcome {
                    serial,
                    opened: Vec::new(),
                    message: opening_refusal(&error),
                },
            });
        });
    }

    /// Reveal the player's sealed cards: the outcome goes in [`Snapshot::reveal`], the
    /// hub's profile after it in [`Snapshot::me`].
    fn reveal_cards(&mut self) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.reveal_cards(&self.identity),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let serial = snapshot.reveal.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.reveal = Some(match outcome {
                Ok(answer) => {
                    snapshot.set_me(answer.profile);
                    RevealOutcome {
                        serial,
                        revealed: answer.revealed,
                        message: String::new(),
                    }
                }
                Err(error) => RevealOutcome {
                    serial,
                    revealed: Vec::new(),
                    message: reveal_refusal(&error),
                },
            });
        });
    }

    /// Shatter, craft or buy shader `id`: the outcome goes in [`Snapshot::trade`], the
    /// hub's profile after it in [`Snapshot::me`], and the shop read last follows the
    /// balances and what is held. A purchase the day turned under is answered by reading
    /// the shop again.
    fn trade(&mut self, trade: Trade, id: &str) {
        type Done = (u64, u32, Option<u64>, Option<u64>, Profile);
        let outcome: Result<Done, HubError> = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => match trade {
                Trade::Shatter | Trade::ShatterLast => hub
                    .shatter_unlock(&self.identity, id, trade == Trade::ShatterLast)
                    .map(|answer| {
                        let shards = answer.shattered.balance;
                        (
                            answer.shattered.shards,
                            1,
                            None,
                            Some(shards),
                            answer.profile,
                        )
                    }),
                Trade::ShatterAll => hub.shatter_duplicates(&self.identity).map(|answer| {
                    (
                        answer.shards,
                        answer.copies,
                        None,
                        Some(answer.balance),
                        answer.profile,
                    )
                }),
                Trade::Craft => hub.craft_unlock(&self.identity, id).map(|answer| {
                    let shards = answer.crafted.balance;
                    (answer.crafted.cost, 0, None, Some(shards), answer.profile)
                }),
                // The balances from the profile: an offer's price is in shards on hubs
                // since 17/10/2026, in credits before.
                Trade::Buy => hub
                    .buy_offer(&self.identity, id)
                    .map(|answer| (answer.bought.price, 0, None, None, answer.profile)),
                Trade::Holocron => hub.buy_holocron(&self.identity, id).map(|answer| {
                    let credits = answer.bought.balance;
                    (answer.bought.price, 0, Some(credits), None, answer.profile)
                }),
            },
            _ => Err(self.not_connected()),
        };
        let reread =
            matches!(&outcome, Err(HubError::Rejected { code, .. }) if code == "not_on_offer");
        self.update(|snapshot| {
            let serial = snapshot.trade.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.trade = Some(match outcome {
                Ok((amount, copies, credits, shards, profile)) => {
                    if let Some(shop) = snapshot.shop.as_mut() {
                        shop.day.credits = credits.unwrap_or(profile.credits);
                        shop.day.shards = shards.unwrap_or(profile.shards);
                        let shader = trade != Trade::Holocron;
                        let held = |id: &str| profile.unlocks.iter().any(|held| held.id == id);
                        for offer in shop
                            .day
                            .offers
                            .iter_mut()
                            .filter(|offer| shader && offer.id == id)
                        {
                            // A shatter keeps what was held, but a last copy gone.
                            offer.owned = match trade {
                                Trade::Shatter | Trade::ShatterAll => offer.owned,
                                Trade::ShatterLast => offer.owned && held(id),
                                _ => true,
                            };
                            offer.bought |= trade == Trade::Buy;
                        }
                    }
                    snapshot.set_me(profile);
                    TradeOutcome {
                        serial,
                        trade,
                        id: id.to_owned(),
                        amount,
                        copies,
                        message: String::new(),
                    }
                }
                Err(error) => TradeOutcome {
                    serial,
                    trade,
                    id: id.to_owned(),
                    amount: 0,
                    copies: 0,
                    message: trade_refusal(&error),
                },
            });
        });
        if reread {
            self.read_shop(Instant::now());
        }
    }

    /// Ask the hub for a tag copies request; what became of it goes to
    /// [`Snapshot::tag_trade`] and the profile after it to the snapshot.
    fn tag_trade(&mut self, trade: TagTrade) {
        type Done = (u64, Option<crate::wire::TagCrafted>, Profile);
        let outcome: Result<Done, HubError> = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => match &trade {
                TagTrade::Shatter(copy) => hub
                    .shatter_tag(&self.identity, copy)
                    .map(|answer| (answer.shattered.shards, None, answer.profile)),
                TagTrade::Craft(copies) => hub
                    .craft_tags(&self.identity, copies)
                    .map(|answer| (0, Some(answer.crafted), answer.profile)),
            },
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let serial = snapshot
                .tag_trade
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            snapshot.tag_trade = Some(match outcome {
                Ok((shards, crafted, profile)) => {
                    snapshot.set_me(profile);
                    TagOutcome {
                        serial,
                        trade,
                        shards,
                        crafted,
                        message: String::new(),
                    }
                }
                Err(error) => TagOutcome {
                    serial,
                    trade,
                    shards: 0,
                    crafted: None,
                    message: tag_refusal(&error),
                },
            });
        });
    }

    /// Set or clear `mark` on shader `id` at the hub (the snapshot shows the change
    /// already, [`Service::set_mark`]): the hub's answer replaces the marks; a refusal is
    /// told in [`Snapshot::mark_refusal`] and the marks are read again with the progress
    /// soon, so the change shown before goes back.
    fn set_mark(&mut self, id: &str, mark: crate::wire::Mark, on: bool, now: Instant) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_mark(&self.identity, id, mark, on),
            _ => Err(self.not_connected()),
        };
        match outcome {
            Ok(marks) => self.update(|snapshot| {
                if snapshot.marks.as_ref() != Some(&marks) {
                    snapshot.marks = Some(marks);
                    snapshot.marks_revision += 1;
                }
            }),
            Err(error) => {
                let words = mark_refusal(&error);
                self.update(|snapshot| {
                    let serial = snapshot.mark_refusal.as_ref().map_or(1, |(n, _)| n + 1);
                    snapshot.mark_refusal = Some((serial, words));
                });
                self.holocrons_soon(now);
            }
        }
    }

    /// Read a page of the history (the newest, or the one before `before`) into
    /// [`Snapshot::history`], or why it could not be read.
    fn read_history(&mut self, before: Option<i64>) {
        let answer = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.history(&self.identity, before),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            Arc::make_mut(snapshot.history.get_or_insert_with(Default::default))
                .take(before, answer);
        });
    }

    /// Read the shop's day: it goes in [`Snapshot::shop`], or why not.
    fn read_shop(&mut self, now: Instant) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.shop(&self.identity),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let serial = snapshot.shop.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.shop = Some(match outcome {
                Ok(day) => ShopState {
                    serial,
                    day,
                    message: String::new(),
                    read_at: now,
                },
                Err(error) => ShopState {
                    serial,
                    day: crate::wire::ShopDay::default(),
                    message: trade_refusal(&error),
                    read_at: now,
                },
            });
        });
    }

    fn set_bio(&mut self, bio: &str) {
        // The hub's rules, checked here first so the player hears why at once.
        let bio = match crate::bio::check(bio) {
            Ok(bio) => bio,
            Err(error) => {
                self.update(|snapshot| snapshot.notice = Some(error.to_string()));
                return;
            }
        };
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_bio(&self.identity, &bio),
            _ => {
                self.update(|snapshot| {
                    snapshot.notice = Some("not connected to the hub".to_owned())
                });
                return;
            }
        };
        self.update(|snapshot| match outcome {
            Ok(profile) => {
                snapshot.set_me(profile);
                snapshot.notice = Some("saved".to_owned());
            }
            Err(error) => snapshot.notice = Some(error.to_string()),
        });
    }

    fn set_banner(&mut self, id: &str) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_banner(&self.identity, id),
            _ => {
                self.update(|snapshot| {
                    snapshot.notice = Some("not connected to the hub".to_owned())
                });
                return;
            }
        };
        self.update(|snapshot| match outcome {
            Ok(profile) => {
                snapshot.set_me(profile);
                snapshot.notice = Some("banner saved".to_owned());
            }
            Err(error) => snapshot.notice = Some(error.to_string()),
        });
    }

    fn set_emoji(&mut self, emoji: &str) {
        let emoji = match crate::name_emoji::check(emoji) {
            Ok(emoji) => emoji,
            Err(error) => {
                self.update(|snapshot| snapshot.notice = Some(error.to_string()));
                return;
            }
        };
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_emoji(&self.identity, &emoji),
            _ => {
                self.update(|snapshot| {
                    snapshot.notice = Some("not connected to the hub".to_owned())
                });
                return;
            }
        };
        self.update(|snapshot| match outcome {
            Ok(profile) => {
                snapshot.set_me(profile);
                snapshot.notice = Some(if emoji.is_empty() {
                    "emoji removed".to_owned()
                } else {
                    "emoji saved".to_owned()
                });
            }
            Err(error) => snapshot.notice = Some(error.to_string()),
        });
    }

    /// Send a staff request: only once registered, and only for a key whose profile
    /// says staff (the hub refuses the others too).
    fn staff(&mut self, request: &StaffRequest) {
        let me = lock(&self.snapshot).me.clone();
        let outcome = match (self.hub.as_mut(), self.registered, &me) {
            (Some(hub), true, Some(me)) if me.staff => hub.staff(&self.identity, request),
            (Some(_), true, _) => Err(HubError::Rejected {
                status: 403,
                code: "not_staff".to_owned(),
                message: "only SJK staff can do this".to_owned(),
            }),
            _ => Err(self.not_connected()),
        };
        let mut staff = self
            .staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        staff.serial += 1;
        staff.busy = false;
        if outcome.is_ok() && matches!(request, StaffRequest::MapBlock { .. }) {
            // The list changed at the hub: see it now, not at the next poll.
            self.due_poll = Instant::now();
        }
        match outcome {
            Ok(profiles) => {
                staff.failed = false;
                staff.message = crate::staff::done(request, profiles.len());
                if matches!(request, StaffRequest::Search(_)) {
                    staff.players = profiles;
                } else if let StaffRequest::ChatUnban { key_id } = request {
                    // Lifted: off the list of bans in force.
                    staff.chat_bans.retain(|ban| ban.key_id != *key_id);
                } else if let Some(profile) = profiles.first() {
                    // The player merged away is part of the kept one now.
                    if let StaffRequest::Merge { from, .. } = request {
                        staff.remove(from, &profile.key_id);
                    }
                    staff.replace(profile);
                    // A change to the player's own person shows on their pages at once.
                    if Some(profile.key_id.as_str()) == me.as_ref().map(|me| me.key_id.as_str()) {
                        lock(&self.snapshot).set_me(profile.clone());
                    }
                    // This PC's own key unlinked: it starts afresh, registering again.
                    if let StaffRequest::Unlink { key_id } = request
                        && *key_id == self.identity.key_id()
                    {
                        self.registered = false;
                        self.name_sent = None;
                        self.update(Snapshot::clear_me);
                    }
                }
            }
            Err(HubError::Rejected { message, .. }) => {
                staff.failed = true;
                staff.message = message;
            }
            Err(error) => {
                staff.failed = true;
                staff.message = error.to_string();
            }
        }
    }

    /// Send a staff query: only once registered, and only for a key whose profile
    /// says staff (the hub refuses the others too).
    fn staff_query(&mut self, query: &StaffQuery) {
        let staff_key = lock(&self.snapshot).me.as_ref().is_some_and(|me| me.staff);
        let outcome = match (self.hub.as_mut(), self.registered, staff_key) {
            (Some(hub), true, true) => hub.staff_query(&self.identity, query),
            (Some(_), true, false) => Err(HubError::Rejected {
                status: 403,
                code: "not_staff".to_owned(),
                message: "only SJK staff can do this".to_owned(),
            }),
            _ => Err(self.not_connected()),
        };
        let mut staff = self
            .staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        staff.serial += 1;
        staff.busy = false;
        match outcome {
            Ok(answer) => {
                staff.failed = false;
                staff.answered(query, answer);
            }
            Err(HubError::Rejected {
                status: 404, code, ..
            }) if code == "not_found" => {
                staff.failed = true;
                staff.message = match query {
                    StaffQuery::RequestLogs { .. } => "The hub does not know that player",
                    _ => "The hub no longer has it",
                }
                .to_owned();
            }
            Err(HubError::Rejected { message, .. }) => {
                staff.failed = true;
                staff.message = message;
            }
            Err(error) => {
                staff.failed = true;
                staff.message = error.to_string();
            }
        }
    }

    /// Look for the SJK team's requests for the player's logs if it is due. Like the
    /// holocrons, a hub that refuses (from before log requests) is left alone a while.
    fn keep_log_requests(&mut self, now: Instant) {
        if now < self.due_log_requests {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        match hub.log_requests(&self.identity) {
            Ok(requests) => {
                self.due_log_requests = now + LOG_REQUESTS_EVERY;
                self.logs_answered
                    .retain(|id| requests.iter().any(|request| request.id == *id));
                let open: Vec<LogRequest> = requests
                    .into_iter()
                    .filter(|request| !self.logs_answered.contains(&request.id))
                    .collect();
                self.update(|snapshot| {
                    if snapshot.log_requests != open {
                        snapshot.log_requests = open;
                    }
                });
            }
            Err(HubError::Network(_)) => self.due_log_requests = now + RETRY_MAX,
            Err(_) => self.due_log_requests = now + LOG_REQUESTS_AGAIN,
        }
    }

    /// Send the player's answer to log request `id`: `upload` (fitted here) or, with
    /// none, a refusal. The request leaves the list whatever the hub says, unless it
    /// could not be reached: a request the hub no longer holds open is done with too.
    fn answer_logs(&mut self, id: i64, upload: Option<&LogUpload>) {
        if !self.logs_answered.contains(&id) {
            self.logs_answered.push(id);
        }
        self.update(|snapshot| snapshot.log_requests.retain(|request| request.id != id));
        let outcome = match (self.hub.as_mut(), self.registered, upload) {
            (Some(hub), true, Some(upload)) => hub
                .send_logs(&self.identity, id, &upload.fitted())
                .map(|()| "logs sent".to_owned()),
            (Some(hub), true, None) => hub
                .decline_logs(&self.identity, id)
                .map(|()| "declined".to_owned()),
            _ => Err(self.not_connected()),
        };
        // Not reached: the request comes back with the next listing, to answer again.
        if matches!(outcome, Err(HubError::Network(_))) {
            self.logs_answered.retain(|answered| *answered != id);
        }
        self.update(|snapshot| {
            let serial = snapshot
                .log_answer
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            snapshot.log_answer = Some(outcome_of(serial, outcome));
        });
    }

    /// Do whatever is due at `now`; return how long to wait for the next thing.
    fn tick(&mut self, now: Instant) -> Duration {
        let wait = self.tick_due(now);
        self.publish_feed();
        wait
    }

    fn tick_due(&mut self, now: Instant) -> Duration {
        if self.saber2_unknown.is_some_and(|until| now >= until) {
            self.saber2_unknown = None;
        }
        if self.extras_unknown.is_some_and(|until| now >= until) {
            self.extras_unknown = None;
        }
        if self.tag_unknown.is_some_and(|until| now >= until) {
            self.tag_unknown = None;
        }
        if !self.settings.enabled {
            self.update(|snapshot| snapshot.status = Status::Disabled);
            return IDLE_MAX;
        }
        if self.hub.is_none() {
            if self.settings.hub_url.trim().is_empty() {
                self.update(|snapshot| snapshot.status = Status::NoHub);
            }
            return IDLE_MAX;
        }
        if !self.registered {
            if now < self.due_register {
                return self.due_register - now;
            }
            self.update(|snapshot| {
                if !matches!(snapshot.status, Status::Failed(_)) {
                    snapshot.status = Status::Registering;
                }
            });
            let name = self.name.clone();
            let outcome = self
                .hub
                .as_mut()
                .map(|hub| hub.register(&self.identity, name.as_deref()));
            match outcome {
                Some(Ok(profile)) => {
                    self.registered = true;
                    self.name_sent = name;
                    self.backoff = RETRY_MIN;
                    self.due_profile = now + PROFILE_EVERY;
                    self.due_assets = now;
                    self.due_holocrons = now;
                    self.due_trades = now;
                    self.due_log_requests = now;
                    self.due_announcements = now;
                    self.due_rewards = now;
                    self.update(|snapshot| {
                        snapshot.set_me(profile);
                        snapshot.status = Status::Online;
                    });
                }
                Some(Err(error)) => {
                    self.due_register = self.failed(&error, now);
                    return self.due_register - now;
                }
                None => {}
            }
        }
        self.run_due(now);
        self.keep_holocrons(now);
        self.look_between(now);
        self.keep_trades(now);
        self.look_between(now);
        self.keep_log_requests(now);
        self.look_between(now);
        self.keep_announcements(now);
        self.keep_rewards(now);
        self.look_between(now);
        self.keep_assets(now);
        self.look_between(now);
        self.keep_map_edits(now);
        self.keep_mystery_boxes(now);
        self.look_between(now);
        self.keep_secrets(now);
        self.send_crashes(now);
        self.look_between(now);
        let mut wait = IDLE_MAX;
        if self.location.is_some() {
            wait = wait
                .min(self.due_claim.saturating_duration_since(now))
                .min(self.due_poll.saturating_duration_since(now));
        }
        if self.name != self.name_sent {
            wait = wait.min(self.due_name.saturating_duration_since(now));
        }
        wait = wait.min(self.due_profile.saturating_duration_since(now));
        if self.counts_waiting(now) {
            wait = wait.min(self.due_counts.saturating_duration_since(now));
        } else if self.counts_waiting(self.counts_held_until) {
            let at = self.due_counts.max(self.counts_held_until);
            wait = wait.min(at.saturating_duration_since(now));
        }
        if !self.lookups.is_empty() || !self.model_checks.is_empty() {
            wait = Duration::ZERO;
        }
        if self.models_waiting() {
            wait = wait.min(self.due_models.saturating_duration_since(now));
        }
        if self.look_waiting() {
            wait = wait.min(self.due_look.saturating_duration_since(now));
        }
        if self.assets_dir.is_some() {
            wait = wait.min(self.due_assets.saturating_duration_since(now));
        }
        if self.map_edits_dir.is_some() && self.registered {
            wait = wait.min(self.due_map_edits.saturating_duration_since(now));
        }
        if self.registered {
            wait = wait.min(self.due_mystery_boxes.saturating_duration_since(now));
            wait = wait.min(self.due_secrets.saturating_duration_since(now));
        }
        if self.crash_dir.is_some() {
            wait = wait.min(self.due_crashes.saturating_duration_since(now));
        }
        wait.min(self.due_holocrons.saturating_duration_since(now))
            .min(self.due_trades.saturating_duration_since(now))
            .min(self.due_log_requests.saturating_duration_since(now))
            .min(self.due_announcements.saturating_duration_since(now))
            .min(self.due_rewards.saturating_duration_since(now))
    }

    /// Read the SJK team's announcements if it is due. A hub that refuses (from before
    /// them) is asked again in an hour, one that cannot be reached after [`RETRY_MAX`];
    /// either way the player is told nothing.
    fn keep_announcements(&mut self, now: Instant) {
        if now < self.due_announcements {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        match hub.announcements(&self.identity) {
            Ok(mut list) => {
                self.due_announcements = now + NEWS_EVERY;
                list.sort_by_key(|announcement| std::cmp::Reverse(announcement.id));
                list.truncate(ANNOUNCEMENTS_KEPT);
                self.update(|snapshot| {
                    if snapshot.announcements != list {
                        snapshot.announcements = list;
                        snapshot.announcements_revision += 1;
                    }
                });
            }
            Err(HubError::Network(_)) => self.due_announcements = now + RETRY_MAX,
            Err(_) => self.due_announcements = now + NEWS_AGAIN,
        }
    }

    /// Read the rewards soon, at most once every [`REWARDS_SOON`], and never later than
    /// they were due anyway.
    fn rewards_soon(&mut self, now: Instant) {
        let at = self.rewards_soon_after.max(now);
        if at < self.due_rewards {
            self.due_rewards = at;
            self.rewards_soon_after = at + REWARDS_SOON;
        }
    }

    /// Read the player's rewards and the newest rounds if it is due, as the
    /// announcements are read.
    fn keep_rewards(&mut self, now: Instant) {
        if now < self.due_rewards {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        match hub.rewards(&self.identity) {
            Ok(rewards) => {
                self.due_rewards = now + NEWS_EVERY;
                self.update(|snapshot| set_rewards(snapshot, rewards));
            }
            Err(HubError::Network(_)) => self.due_rewards = now + RETRY_MAX,
            Err(_) => self.due_rewards = now + NEWS_AGAIN,
        }
    }

    /// Claim reward `id`: the hub grants it and answers the profile (holding the gift
    /// holocrons it gave) and the rewards after it. A refusal says why in the player's
    /// words, and the rewards are read again soon (it was claimed from another PC, or
    /// taken back).
    fn claim_reward(&mut self, id: u64, now: Instant) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.claim_reward(&self.identity, id),
            _ => Err(self.not_connected()),
        };
        let message = match outcome {
            Ok(answer) => {
                self.update(|snapshot| {
                    snapshot.set_me(answer.profile);
                    set_rewards(snapshot, answer.rewards);
                });
                // The gift holocrons are new: their progress and pop-up follow.
                self.holocrons_soon(now);
                String::new()
            }
            Err(error) => {
                if !matches!(error, HubError::Network(_)) {
                    self.rewards_soon(now);
                }
                claim_refusal(&error)
            }
        };
        self.update(|snapshot| {
            let serial = snapshot.claim.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.claim = Some(ClaimOutcome {
                serial,
                id,
                message,
            });
        });
    }

    /// Send the creator's announcement, checked first as the hub checks it; the hub's
    /// words, or nothing, go in [`Snapshot::announce`].
    fn announce(&mut self, request: &AnnounceRequest) {
        let outcome = match crate::news::check(request) {
            Err(error) => Err(HubError::Rejected {
                status: 400,
                code: error.code().to_owned(),
                message: error.message().to_owned(),
            }),
            Ok(checked) => match (self.hub.as_mut(), self.registered) {
                (Some(hub), true) => hub.announce(&self.identity, &checked),
                _ => Err(self.not_connected()),
            },
        };
        let outcome = outcome.map(|announcement| {
            self.update(|snapshot| merge_announcements(snapshot, vec![announcement]));
            String::new()
        });
        self.update(|snapshot| {
            let serial = snapshot.announce.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.announce = Some(outcome_of(serial, outcome.map_err(announce_refusal)));
        });
    }

    fn upload_news_image(&mut self, bytes: &[u8], mime: &str) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => {
                hub.upload_news_image(&self.identity, bytes, mime)
                    .map_err(|error| match error {
                        HubError::Rejected { code, message, .. } => match code.as_str() {
                            "not_staff" | "not_registered" => {
                                "Only the SJK team can upload news pictures".to_owned()
                            }
                            "image_quota" => {
                                "Thirty pictures an hour at most: wait a while".to_owned()
                            }
                            _ if message.is_empty() => format!("the hub refused it ({code})"),
                            _ => message,
                        },
                        other => other.to_string(),
                    })
            }
            _ => Err(self.not_connected().to_string()),
        };
        self.update(|snapshot| {
            let serial = snapshot.news_image.as_ref().map_or(1, |(last, _)| last + 1);
            snapshot.news_image = Some((serial, outcome));
        });
    }

    /// Read the player's trades if it is due.
    fn keep_trades(&mut self, now: Instant) {
        if now >= self.due_trades && self.read_trades(now) {
            self.swap_seen(now);
        }
    }

    /// A trade of the player's finished with the items swapped, and this worker has not
    /// had the profile after it: the player who confirms first gets no profile in the
    /// hub's answer (the swap happens at the other's confirmation), so their collection
    /// would keep the items given away and lack the ones received until the next
    /// [`PROFILE_EVERY`] read (Sol, 25/10/2026). The profile and the holocrons are read at once.
    fn swap_seen(&mut self, now: Instant) {
        self.due_profile = now;
        self.holocrons_soon(now);
    }

    /// Read the player's trades into [`Snapshot::trades`]; the next read is due
    /// [`TRADES_LIVE`] later while one runs, else [`TRADES_EVERY`]. A hub before trades
    /// (or one that cannot be reached) is asked again much later: its list stays as it
    /// was. Says whether a trade finished with a swap since the last list (never on the
    /// first list, whose finished trades are old).
    fn read_trades(&mut self, now: Instant) -> bool {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.trades(&self.identity),
            _ => {
                self.due_trades = now + TRADES_EVERY;
                return false;
            }
        };
        match outcome {
            Ok(list) => {
                let swapped = lock(&self.snapshot).trades.as_ref().is_some_and(|last| {
                    list.trades.iter().any(|trade| {
                        trade.is_done()
                            && !last
                                .list
                                .trades
                                .iter()
                                .any(|seen| seen.id == trade.id && seen.is_done())
                    })
                });
                let live = list.trades.iter().any(|trade| trade.is_live());
                // A trade just started: the window lists the player's holocrons and
                // duplicates from their profile, so it is read again now.
                let was_live = self.with_snapshot_trades_live();
                if live && !was_live {
                    self.due_profile = now;
                }
                let staff = lock(&self.snapshot).me.as_ref().is_some_and(|me| me.staff);
                self.due_trades = now
                    + match (live, list.open(staff)) {
                        (true, _) => TRADES_LIVE,
                        (false, true) => TRADES_EVERY,
                        (false, false) => TRADES_OFF,
                    };
                let hub_time = list.now.unwrap_or_else(unix_now);
                self.update(|snapshot| {
                    let last = snapshot.trades.take();
                    let serial = last.as_ref().map_or(1, |last| last.serial + 1);
                    snapshot.trades = Some(TradesState {
                        serial,
                        list,
                        read_at: now,
                        hub_time,
                        problem: last
                            .as_ref()
                            .map(|last| last.problem.clone())
                            .unwrap_or_default(),
                        steps: last.as_ref().map_or(0, |last| last.steps),
                    });
                });
                swapped
            }
            Err(HubError::Network(_)) => {
                self.due_trades = now + RETRY_MAX;
                false
            }
            Err(_) => {
                self.due_trades = now + TRADES_EVERY;
                false
            }
        }
    }

    /// Whether the snapshot's last list of trades had one running.
    fn with_snapshot_trades_live(&self) -> bool {
        lock(&self.snapshot)
            .trades
            .as_ref()
            .is_some_and(|state| state.live().is_some())
    }

    /// Ask the player `to` to trade, or take `step` in a trade; the list is read again
    /// after either, and a refusal goes in [`TradesState::problem`]. A finished swap's
    /// profile becomes [`Snapshot::me`].
    fn trade_with(
        &mut self,
        step: Option<(i64, crate::trades::TradeStep)>,
        to: Option<&str>,
        now: Instant,
    ) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => match (&step, to) {
                (Some((id, step)), _) => hub.trade_step(&self.identity, *id, step),
                (None, Some(to)) => hub.trade_ask(&self.identity, to),
                (None, None) => return,
            },
            _ => Err(self.not_connected()),
        };
        let mut has_profile = false;
        let problem = match outcome {
            Ok(answer) => {
                if let Some(profile) = answer.profile {
                    self.update(|snapshot| snapshot.set_me(profile));
                    self.holocrons_soon(now);
                    has_profile = true;
                }
                String::new()
            }
            Err(error) => crate::trades::refusal(&error),
        };
        self.update(|snapshot| {
            let state = snapshot.trades.get_or_insert_with(|| TradesState {
                serial: 0,
                list: crate::trades::TradesAnswer::default(),
                read_at: now,
                hub_time: unix_now(),
                problem: String::new(),
                steps: 0,
            });
            state.steps += 1;
            state.problem = problem;
        });
        if self.read_trades(now) && !has_profile {
            self.swap_seen(now);
        }
    }

    /// Read the progress toward the next holocron if it is due. Cosmetic, like the asset
    /// packs: the status stays. A hub without the reading (404, from before holocrons) is
    /// asked again in an hour; any other failure (unreachable, too many requests, a
    /// restart) after [`RETRY_MAX`], so one failed reading does not leave the progress an
    /// hour old (the Esc menu's reading, which waits for a due one, could not bring it
    /// forward).
    fn keep_holocrons(&mut self, now: Instant) {
        if now < self.due_holocrons {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        match hub.holocrons(&self.identity) {
            Ok(state) => {
                self.due_holocrons = now + HOLOCRONS_EVERY;
                self.update(|snapshot| {
                    if !state.loot.is_empty() {
                        snapshot.loot.clone_from(&state.loot);
                    }
                    if state.economy.is_some() {
                        snapshot.economy.clone_from(&state.economy);
                    }
                    if state.kyber.is_some() {
                        snapshot.kyber.clone_from(&state.kyber);
                    }
                    // A hub that keeps marks sends them every time (none: an empty list);
                    // one that keeps none leaves them out.
                    if snapshot.marks != state.marks {
                        snapshot.marks.clone_from(&state.marks);
                        snapshot.marks_revision += 1;
                    }
                    snapshot.min_client.clone_from(&state.min_client);
                    snapshot.holocrons = Some(HolocronProgress::new(state, now));
                });
            }
            Err(HubError::Rejected { status: 404, .. } | HubError::Protocol(_)) => {
                self.due_holocrons = now + HOLOCRONS_AGAIN;
            }
            Err(_) => self.due_holocrons = now + RETRY_MAX,
        }
    }

    /// Check the hub's asset packs if a folder was given and the check is due: one
    /// reading of the list, and a download of each pack the folder does not hold as
    /// listed. Next time is [`ASSETS_EVERY`] later; a shortfall worth trying again
    /// soon waits [`ASSETS_RETRY_MIN`], doubling, instead. Cosmetic: the status stays.
    fn keep_assets(&mut self, now: Instant) {
        let (Some(dir), Some(hub)) = (self.assets_dir.as_ref(), self.hub.as_mut()) else {
            return;
        };
        if now < self.due_assets {
            return;
        }
        let refresh = crate::assets::refresh(hub.as_mut(), &self.identity, dir);
        let wait = match &refresh.problem {
            Some(crate::assets::Problem::Soon(_)) => {
                let wait = self.assets_backoff;
                self.assets_backoff = (self.assets_backoff * 2).min(ASSETS_EVERY);
                wait
            }
            _ => {
                self.assets_backoff = ASSETS_RETRY_MIN;
                ASSETS_EVERY
            }
        };
        self.due_assets = now + wait;
        let note = format!("{}; next check in {} s", refresh.summary(), wait.as_secs());
        self.update(|snapshot| {
            snapshot.packs_revision += refresh.written.len() as u64;
            snapshot.assets_note = Some(note);
        });
    }

    /// Bring the map edits' folder up to the hub's list once registered, if a folder was
    /// given and it is time: next time [`MAP_EDITS_EVERY`] later. Cosmetic: the status
    /// stays; what it did goes in [`Snapshot::map_edits_note`].
    fn keep_map_edits(&mut self, now: Instant) {
        let (Some(dir), Some(hub), true) = (
            self.map_edits_dir.as_ref(),
            self.hub.as_mut(),
            self.registered,
        ) else {
            return;
        };
        if now < self.due_map_edits {
            return;
        }
        self.due_map_edits = now + MAP_EDITS_EVERY;
        let refresh = crate::map_edits::refresh(hub.as_mut(), &self.identity, dir);
        let note = match &refresh.problem {
            Some(problem) => Err(problem.clone()),
            None => Ok(refresh.summary()),
        };
        self.update(|snapshot| {
            snapshot.map_edits_revision += refresh.changed as u64;
            snapshot.map_edits_note = Some(note);
        });
    }

    /// Read the hub's mystery boxes once registered, if it is time: next time
    /// [`MYSTERY_BOXES_EVERY`] later. Cosmetic: a failure keeps the last list.
    fn keep_mystery_boxes(&mut self, now: Instant) {
        if !self.registered || now < self.due_mystery_boxes {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        self.due_mystery_boxes = now + MYSTERY_BOXES_EVERY;
        if let Ok(boxes) = hub.mystery_boxes(&self.identity) {
            self.update(|snapshot| {
                if snapshot.mystery_boxes.as_ref() != Some(&boxes) {
                    snapshot.mystery_boxes = Some(boxes);
                    snapshot.mystery_boxes_revision += 1;
                }
            });
        }
    }

    /// Send a mystery box request: its outcome goes in [`Snapshot::mystery_box`], the
    /// profile after it (an opening's) in [`Snapshot::me`]. The boxes are read again
    /// at once after a placing or a removal, and after a box that is not there any more.
    fn mystery_box(
        &mut self,
        now: Instant,
        request: impl FnOnce(&mut dyn Hub, &Identity) -> Result<(BoxDone, Option<Profile>), HubError>,
    ) {
        self.mystery_box_serial += 1;
        let serial = self.mystery_box_serial;
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => request(hub.as_mut(), &self.identity),
            _ => Err(self.not_connected()),
        };
        let reread = match &outcome {
            Ok((BoxDone::Placed(_) | BoxDone::Removed(_), _)) => true,
            Err(HubError::Rejected { code, .. }) => code == "no_such_box",
            _ => false,
        };
        if reread {
            self.due_mystery_boxes = now;
        }
        self.update(|snapshot| {
            let outcome = match outcome {
                Ok((done, profile)) => {
                    if let Some(profile) = profile {
                        if let Some(shop) = snapshot.shop.as_mut() {
                            shop.day.credits = profile.credits;
                        }
                        snapshot.set_me(profile);
                    }
                    Ok(done)
                }
                Err(error) => Err(crate::mystery_box::refusal(&error)),
            };
            snapshot.mystery_box = Some((serial, outcome));
        });
    }

    /// Ask the hub about junk selling: [`Snapshot::junk`] is what it said, `None` when it
    /// could not be asked or has none (an older hub).
    fn read_junk(&mut self) {
        let status = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.junk_status(&self.identity).ok(),
            _ => None,
        };
        self.update(|snapshot| snapshot.junk = status);
    }

    /// Sell `held` (pieces of junk by id): the answer goes in [`Snapshot::junk_sale`],
    /// what is still payable in [`Snapshot::junk`] and the credits in the profile.
    fn sell_junk(&mut self, held: &std::collections::BTreeMap<String, u32>) {
        self.junk_serial += 1;
        let serial = self.junk_serial;
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.sell_junk(&self.identity, held),
            _ => Err(self.not_connected()),
        };
        self.update(|snapshot| {
            let outcome = match outcome {
                Ok(sold) => {
                    if let Some(status) = snapshot.junk.as_mut() {
                        status.left = sold.left;
                        status.balance = sold.balance;
                    }
                    let credits = u64::try_from(sold.balance).unwrap_or(0);
                    if let Some(shop) = snapshot.shop.as_mut() {
                        shop.day.credits = credits;
                    }
                    if let Some(me) = snapshot.me.as_mut() {
                        me.credits = credits;
                        snapshot.me_revision += 1;
                    }
                    Ok(sold)
                }
                Err(error) => {
                    if crate::junk::unsupported(&error) {
                        snapshot.junk = None;
                    } else if let HubError::Rejected { code, .. } = &error
                        && let Some(status) = snapshot.junk.as_mut()
                    {
                        match code.as_str() {
                            "junk_cap" => status.left = 0,
                            "junk_selling_off" => status.open = false,
                            _ => {}
                        }
                    }
                    Err(crate::junk::refusal(&error))
                }
            };
            snapshot.junk_sale = Some((serial, outcome));
        });
    }

    /// Read the Toy Master's toys once registered, if it is time: next time
    /// [`SECRETS_EVERY`] later. Cosmetic: a failure keeps the last list.
    fn keep_secrets(&mut self, now: Instant) {
        if !self.registered || now < self.due_secrets {
            return;
        }
        let Some(hub) = self.hub.as_mut() else {
            return;
        };
        self.due_secrets = now + SECRETS_EVERY;
        let server = self
            .location
            .as_ref()
            .map(|location| location.server.to_string());
        if let Ok(secrets) = hub.secrets(&self.identity, server.as_deref()) {
            self.update(|snapshot| {
                if snapshot.secrets.as_ref() != Some(&secrets) {
                    snapshot.secrets = Some(secrets);
                    snapshot.secrets_revision += 1;
                }
            });
        }
    }

    /// Send a request about the Toy Master's toys: its outcome goes in
    /// [`Snapshot::secret`], the profile after a find in [`Snapshot::me`]. The toys are
    /// read again at once after a placing or a removal, and after a toy that is gone.
    fn secret(
        &mut self,
        now: Instant,
        request: impl FnOnce(&mut dyn Hub, &Identity) -> Result<(SecretDone, Option<Profile>), HubError>,
    ) {
        self.secret_serial += 1;
        let serial = self.secret_serial;
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => request(hub.as_mut(), &self.identity),
            _ => Err(self.not_connected()),
        };
        let reread = match &outcome {
            Ok((SecretDone::Placed(_) | SecretDone::Removed(_) | SecretDone::Escorted(..), _)) => {
                true
            }
            Err(HubError::Rejected { code, .. }) => code == "no_such_secret",
            _ => false,
        };
        if reread {
            self.due_secrets = now;
        }
        self.update(|snapshot| {
            let outcome = match outcome {
                Ok((done, profile)) => {
                    if let Some(profile) = profile {
                        snapshot.set_me(profile);
                    }
                    Ok(done)
                }
                Err(error) => Err(crate::toy_master::refusal(&error)),
            };
            snapshot.secret = Some((serial, outcome));
        });
    }

    /// Read the box hunt on `server` into [`Snapshot::box_hunt`], first telling the hub
    /// `report` when given (the player, the mark, killed by a slot; or out of the chase),
    /// tried twice (a kill lost is a hunter's free roll lost), and whether the player is
    /// `away`. A failed read leaves the last one.
    fn box_hunt(&mut self, server: &str, report: Option<HuntReport>, away: bool) {
        let (Some(hub), true) = (self.hub.as_mut(), self.registered) else {
            return;
        };
        for _ in 0..2 {
            let told = match report {
                Some(HuntReport::Killed(by)) => hub.box_hunt_killed(&self.identity, server, by),
                Some(HuntReport::Out(out)) => hub.box_hunt_out(&self.identity, server, out),
                None => Ok(true),
            };
            if told.is_ok() {
                break;
            }
        }
        let Ok(state) = hub.box_hunt(&self.identity, server, away) else {
            return;
        };
        self.box_hunt_serial += 1;
        let serial = self.box_hunt_serial;
        let server = server.to_owned();
        self.update(|snapshot| snapshot.box_hunt = Some((serial, server, state)));
    }

    /// Send a map's edits for staff; the folder is read again at once after.
    fn publish_map_edits(&mut self, map: &str, checksum: i32, edits: &Value, now: Instant) {
        self.map_edits_serial += 1;
        let serial = self.map_edits_serial;
        let outcome = match self.hub.as_mut() {
            None => Err("Not connected to the SJK hub.".to_owned()),
            Some(hub) => hub
                .publish_map_edits(&self.identity, map, checksum, edits)
                .map(|revision| (map.to_owned(), checksum, revision))
                .map_err(|error| crate::map_edits::refusal(&error)),
        };
        if outcome.is_ok() {
            self.due_map_edits = now;
        }
        self.update(|snapshot| snapshot.map_edits_published = Some((serial, outcome)));
    }

    /// Send the crash reports waiting in the folder given, if it is time: next time
    /// [`CRASHES_EVERY`] later, or [`CRASHES_AGAIN`] when some had to wait.
    fn send_crashes(&mut self, now: Instant) {
        let (Some(dir), Some(hub)) = (self.crash_dir.as_ref(), self.hub.as_mut()) else {
            return;
        };
        if now < self.due_crashes {
            return;
        }
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let name = self.name.clone().unwrap_or_default();
        let round = crate::crash::send_pending(hub.as_mut(), &self.identity, dir, &name, unix);
        self.due_crashes = now
            + if round.waiting > 0 {
                CRASHES_AGAIN
            } else {
                CRASHES_EVERY
            };
        if round == crate::crash::Round::default() {
            return;
        }
        let mut note = format!("crash reports: {} sent", round.sent.len());
        if round.dropped > 0 {
            note.push_str(&format!(", {} dropped", round.dropped));
        }
        if let Some(problem) = &round.problem {
            note.push_str(&format!(", {} waiting ({problem})", round.waiting));
        }
        self.update(|snapshot| {
            snapshot.crashes_sent += round.sent.len() as u64;
            snapshot.crash_note = Some(note);
        });
    }

    /// The look to send: the one the viewer gave, without a blade skin the hub said
    /// the key does not hold while the profile's unlocks are the same.
    fn look_to_send(&mut self) -> Look {
        if !self.look_refused.is_empty() {
            let same = lock(&self.snapshot)
                .me
                .as_ref()
                .is_some_and(|me| me.unlocks == self.look_refused_for);
            if !same {
                self.look_refused.clear();
            }
        }
        let mut look = self.look.clone();
        if self.look_refused.contains(&look.saber) {
            look.saber.clear();
            look.saber_off.clear();
            look.saber_on.clear();
        }
        if self.extras_unknown.is_some() {
            look.saber_on.clear();
            look.saber2_on.clear();
        }
        // A second skin refused, or a hub that does not know the field: the second saber
        // wears the first's skin, as before `saber2` existed.
        if self.saber2_unknown.is_some()
            || look
                .saber2
                .as_ref()
                .is_some_and(|saber2| self.look_refused.contains(saber2))
        {
            look.saber2 = None;
            look.saber2_off.clear();
            look.saber2_on.clear();
        }
        if self.look_sounds_unsupported || self.look_refused.contains(&look.saber_sounds) {
            look.saber_sounds.clear();
        }
        if self.finisher_unsupported || self.look_refused.contains(&look.finisher) {
            look.finisher.clear();
        }
        if self.tag_unknown.is_some()
            || look
                .tag
                .as_ref()
                .is_some_and(|tag| self.look_refused.contains(&tag.id))
        {
            look.tag = None;
        }
        look
    }

    /// Whether the hub's claim should get another look than the one it holds.
    fn look_waiting(&mut self) -> bool {
        self.look_claim.is_some() && Held::Look(Box::new(self.look_to_send())) != self.look_held
    }

    /// Send the look for the accepted claim if it changed and one may go.
    fn send_look(&mut self, now: Instant) {
        if now < self.due_look || !self.look_waiting() {
            return;
        }
        let look = self.look_to_send();
        let (Some((server, _, _)), Some(hub)) = (self.look_claim.clone(), self.hub.as_mut()) else {
            return;
        };
        let started = Instant::now();
        let outcome = hub.look(&self.identity, &server, &look);
        // The next look leaves a second after this one's answer, not its start: it then
        // reaches the hub more than a second after this one, in another of the whole
        // seconds the hub counts (a `look_quota` refusal otherwise).
        // In whole milliseconds, which keeps the worker's timing exact under test.
        let took =
            Duration::from_millis(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));
        let now = now + took;
        self.due_look = now + LOOK_EVERY;
        match &outcome {
            Ok(id) => {
                self.look_held = Held::Look(Box::new(look.clone()));
                self.look_id = Some(*id);
            }
            // The skin is not the key's (or not the hub's): the rest of the look still
            // goes, without it, until the profile changes. Not the key's while its
            // profile said so: the profile is out of date. The hub names the side: the
            // second saber's skin is `saber2_not_unlocked` or `bad_saber2`, the first's
            // `not_unlocked` or `bad_look` (`PROTOCOL.md`, "Looks").
            Err(HubError::Rejected { code, .. }) if refused_skin(code, &look).is_some() => {
                if code.ends_with("not_unlocked") {
                    self.profile_soon(now);
                }
                self.look_refused_for = lock(&self.snapshot)
                    .me
                    .as_ref()
                    .map(|me| me.unlocks.clone())
                    .unwrap_or_default();
                if let Some(refused) = refused_skin(code, &look) {
                    self.look_refused.push(refused.to_owned());
                }
            }
            // The tag is not the key's (or not the hub's): the look goes again without it
            // until the profile changes.
            Err(HubError::Rejected { code, .. })
                if (code == "tag_not_unlocked" || code == "bad_tag") && look.tag.is_some() =>
            {
                if code == "tag_not_unlocked" {
                    self.profile_soon(now);
                }
                self.look_refused_for = lock(&self.snapshot)
                    .me
                    .as_ref()
                    .map(|me| me.unlocks.clone())
                    .unwrap_or_default();
                if let Some(tag) = look.tag.as_ref() {
                    self.look_refused.push(tag.id.clone());
                }
                self.due_look = now;
            }
            // A hub from before 27/10/2026 refuses the tag, the newest field: the look goes
            // again without it (before the older fields are suspected).
            Err(HubError::Rejected { code, .. }) if code == "bad_body" && look.tag.is_some() => {
                self.tag_unknown = Some(now + SABER2_RETRY);
                self.due_look = now;
            }
            // A hub from before 04/11/2026 refuses the finisher: the look goes again
            // without it, the newest field, before any older one is suspected.
            Err(HubError::Rejected {
                status: 400, code, ..
            }) if code == "bad_body" && !look.finisher.is_empty() => {
                self.finisher_unsupported = true;
                self.due_look = now;
            }
            // A finisher the key does not hold, or one the hub does not know: the look
            // goes again without it until the profile changes.
            Err(HubError::Rejected { code, .. })
                if (code == "finisher_not_unlocked" || code == "bad_finisher")
                    && !look.finisher.is_empty() =>
            {
                self.profile_soon(now);
                self.look_refused_for = lock(&self.snapshot)
                    .me
                    .as_ref()
                    .map(|me| me.unlocks.clone())
                    .unwrap_or_default();
                self.look_refused.push(look.finisher.clone());
                self.due_look = now;
            }
            // A hub from before 26/10/2026 refuses the extras: the look goes again without
            // them (before `saber2` is suspected, which such a hub may know).
            Err(HubError::Rejected { code, .. })
                if code == "bad_body"
                    && (!look.saber_on.is_empty() || !look.saber2_on.is_empty()) =>
            {
                self.extras_unknown = Some(now + SABER2_RETRY);
                self.due_look = now;
            }
            // A hub from before 21/10/2026 refuses `saber2`: the look goes again without it.
            Err(HubError::Rejected { code, .. }) if code == "bad_body" && look.saber2.is_some() => {
                self.saber2_unknown = Some(now + SABER2_RETRY);
                self.due_look = now;
            }
            // The sound set chosen apart from the blade is a skin the key does not hold:
            // the look goes again without it (following the blade) until the profile
            // changes.
            Err(HubError::Rejected { code, .. })
                if code == "sounds_not_unlocked" && !look.saber_sounds.is_empty() =>
            {
                self.profile_soon(now);
                self.look_refused_for = lock(&self.snapshot)
                    .me
                    .as_ref()
                    .map(|me| me.unlocks.clone())
                    .unwrap_or_default();
                self.look_refused.push(look.saber_sounds.clone());
                self.due_look = now;
            }
            // A hub from before the sounds choice refuses the field: the look goes
            // again at once without it, so the blade still reaches everyone.
            Err(HubError::Rejected {
                status: 400, code, ..
            }) if code == "bad_body" && !look.saber_sounds.is_empty() => {
                self.look_sounds_unsupported = true;
                self.due_look = now;
            }
            // The claim lapsed at the hub: the next accepted claim sends the look again,
            // and it is asked for at once rather than at the next renewal.
            Err(HubError::Rejected { code, .. }) if code == "not_on_server" => {
                self.look_claim = None;
                self.look_held = Held::none();
                self.due_claim = now;
            }
            // The key's looks came too fast: the hub counts whole seconds.
            Err(HubError::Rejected {
                status: 429, code, ..
            }) if code == "look_quota" => {
                self.due_look = now + LOOK_QUOTA_AGAIN;
            }
            // Too many: the key's look quota, or the address's allowance (429
            // `rate_limited`) shared with the chat and emotes. A signature refused
            // (401, the clock still off after the one retry) is not final either.
            Err(HubError::Rejected {
                status: 401 | 429, ..
            }) => {
                self.due_look = now + LOOK_AGAIN;
            }
            // Any other refusal (an older hub without looks) is not repeated until the
            // look or the claim changes.
            Err(HubError::Rejected { status, .. }) if (400..500).contains(status) => {
                self.look_held = Held::Look(Box::new(look.clone()));
            }
            Err(_) => self.due_look = now + LOOK_AGAIN,
        }
        let waited = self
            .look_since
            .map_or(Duration::ZERO, |since| now.saturating_duration_since(since));
        if outcome.is_ok() && !self.look_waiting() {
            self.look_since = None;
        }
        let note = format!(
            "{} ms after the change, the hub answered in {} ms",
            waited.as_millis(),
            took.as_millis()
        );
        self.update(|snapshot| {
            let serial = snapshot
                .look_outcome
                .as_ref()
                .map_or(1, |last| last.serial + 1);
            let mut reported = outcome_of(serial, outcome.map(|_| String::new()));
            reported.message = if reported.sent {
                note
            } else {
                format!("{} ({note})", reported.message)
            };
            snapshot.look_outcome = Some(reported);
        });
    }

    /// Wear `look` from now on, noting when it starts waiting for the hub.
    fn wear(&mut self, look: Look, now: Instant) {
        if look != self.look && self.look_since.is_none() {
            self.look_since = Some(now);
        }
        self.look = look;
    }

    /// Take the newest look [`Service::set_look`] left, if any.
    fn take_latest_look(&mut self, now: Instant) {
        let latest = self
            .latest_look
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(look) = latest {
            self.wear(look, now);
        }
    }

    /// Between two of a round's other hub calls: a look given meanwhile goes now if it
    /// may, rather than after the rest of the round.
    fn look_between(&mut self, now: Instant) {
        self.take_latest_look(now);
        self.send_look(now);
    }

    /// Whether the hub should hear the achievement counts at `now`: they changed since
    /// they were last sent, or one is above what the hub holds (it was held back by its
    /// hourly allowance) and the last sending is an hour old.
    fn counts_waiting(&self, now: Instant) -> bool {
        if self.counts.is_empty() {
            return false;
        }
        if self.counts_sent.as_ref() != Some(&self.counts) {
            return true;
        }
        if now < self.counts_held_until {
            return false;
        }
        let held = lock(&self.snapshot).me.as_ref().map(|me| {
            self.counts.iter().any(|(id, &count)| {
                match me.achievements.iter().find(|held| &held.id == id) {
                    Some(held) => count > held.progress && held.unlocked == 0,
                    None => count > 0,
                }
            })
        });
        held.unwrap_or(false)
    }

    /// Whether the hub should hear the model list: the game listed its models and the
    /// hub has not taken this list.
    fn models_waiting(&self) -> bool {
        self.models.is_some() && self.models != self.models_sent
    }

    /// Send the model list if it is waiting and due. A refusal (an older hub, a
    /// quota) waits [`MODELS_AGAIN`]; a hub out of reach, the backoff.
    fn send_models(&mut self, now: Instant) {
        if now < self.due_models || !self.models_waiting() {
            return;
        }
        let (Some(hub), Some(models)) = (self.hub.as_mut(), self.models.as_ref()) else {
            return;
        };
        match hub.set_models(&self.identity, models) {
            Ok(_) => self.models_sent = self.models.clone(),
            Err(HubError::Rejected { status, .. }) if (400..500).contains(&status) => {
                self.due_models = now + MODELS_AGAIN;
            }
            Err(_) => self.due_models = now + self.backoff,
        }
    }

    /// Ask the hub the model questions waiting. Nobody holding the slot is an answer
    /// with no claimed name; a hub that cannot say leaves `has` unknown.
    fn check_models(&mut self) {
        let Some(hub) = self.hub.as_mut() else { return };
        for (server, slot, model) in std::mem::take(&mut self.model_checks) {
            let check = match hub.model_check(&server.to_string(), slot, &model) {
                Ok((claimed_name, has)) => ModelCheck { claimed_name, has },
                Err(_) => ModelCheck {
                    claimed_name: String::new(),
                    has: None,
                },
            };
            let mut snapshot = lock(&self.snapshot);
            if snapshot.model_checks.len() >= MODEL_CHECKS_MAX {
                snapshot.model_checks.clear();
            }
            snapshot.model_checks.insert((server, slot, model), check);
            // The player card rebuilds on it, as on a profile.
            snapshot.profiles_revision += 1;
        }
    }

    /// Send the achievement counts if they are waiting and due. An achievement the
    /// answer unlocks brings a kyber crystal, which the profile lists: it is read soon.
    fn send_counts(&mut self, now: Instant) -> Option<HubError> {
        if now < self.due_counts || !self.counts_waiting(now) {
            return None;
        }
        let hub = self.hub.as_mut()?;
        match hub.set_achievements(&self.identity, &self.counts) {
            Ok(achievements) => {
                let unlocked = newly_unlocked(&lock(&self.snapshot), &achievements);
                set_achievements(&mut lock(&self.snapshot), achievements);
                self.counts_sent = Some(self.counts.clone());
                self.due_counts = now + ACHIEVEMENTS_EVERY;
                // Counts the hub held back wait an hour unless they change (then they
                // go at the minute, as any change).
                self.counts_held_until = now + ACHIEVEMENTS_AGAIN;
                if unlocked {
                    self.own_drop(now);
                }
                None
            }
            Err(HubError::Rejected { status, .. }) if (400..500).contains(&status) => {
                // A refusal (a quota, an older hub without achievements) is not
                // retried soon; it is not the hub being down either.
                self.counts_sent = Some(self.counts.clone());
                self.due_counts = now + ACHIEVEMENTS_AGAIN;
                None
            }
            Err(failure) => {
                self.due_counts = now + self.backoff;
                Some(failure)
            }
        }
    }

    /// A new name, the claim, the roster read and the pending lookups, where due.
    fn run_due(&mut self, now: Instant) {
        let counts_error = self.send_counts(now);
        let Some(hub) = self.hub.as_mut() else { return };
        let mut error = None;
        // A name worn since registering joins the key's history at the hub (a
        // registration of a known key only adds the name). A failure waits and retries.
        if self.name != self.name_sent && now >= self.due_name {
            match hub.register(&self.identity, self.name.as_deref()) {
                Ok(profile) => {
                    self.name_sent = self.name.clone();
                    self.due_profile = now + PROFILE_EVERY;
                    lock(&self.snapshot).set_me(profile);
                }
                Err(failure) => {
                    self.due_name = now + self.backoff;
                    error = Some(failure);
                }
            }
        }
        // The own profile again from time to time: medals and the verified flag change
        // at the hub, not here.
        if now >= self.due_profile {
            match hub.profile(&self.identity.key_id()) {
                Ok(profile) => {
                    self.due_profile = now + PROFILE_EVERY;
                    lock(&self.snapshot).set_me(profile);
                }
                Err(failure) => {
                    self.due_profile = now + self.backoff;
                    error = Some(failure);
                }
            }
        }
        if let Some(location) = self.location.clone() {
            let server = location.server.to_string();
            if now >= self.due_claim {
                match hub.claim_with(
                    &self.identity,
                    &server,
                    location.slot,
                    &location.name,
                    self.active,
                    Some(&self.play),
                ) {
                    Ok(()) => {
                        // What was sent is counted: the next claim starts afresh. An
                        // inactive claim sent nothing, so what it held waits.
                        if self.active {
                            self.play = crate::wire::PlaySignals::default();
                        }
                        self.claimed = Some(server.clone());
                        self.due_claim = now + CLAIM_EVERY;
                        // Renewing the same claim keeps its look; another server,
                        // slot or name starts with none. With no claim accepted before
                        // (released, lapsed, failed or none yet), what the hub holds
                        // stays as it was known, or unknown.
                        let claim = (server.clone(), location.slot, location.name.clone());
                        if self.look_claim.as_ref() != Some(&claim)
                            && self.look_claim.replace(claim).is_some()
                        {
                            self.look_held = Held::none();
                        }
                    }
                    Err(failure) => {
                        self.due_claim = now + self.backoff;
                        // The claim may lapse meanwhile, and its look with it, or
                        // still be live with it: what the hub holds is not known.
                        self.look_claim = None;
                        self.look_held = Held::Unknown;
                        error = Some(failure);
                    }
                }
            }
            if now >= self.due_poll {
                match hub.presence_with_blocks(&server) {
                    Ok(crate::hub::PresenceRead {
                        players,
                        blocks,
                        map_edits,
                        secrets,
                        shows,
                    }) => {
                        self.due_poll = now + POLL_EVERY;
                        // Staff published (or this is the first look on this run): read
                        // the map edits' list now, not at the next 10 minute look.
                        if map_edits.is_some() && map_edits != self.map_edits_stamp {
                            self.map_edits_stamp = map_edits;
                            self.due_map_edits = now;
                        }
                        // The toys listed for this server changed (one placed or taken
                        // away, SJK's creator came or went), or this is a new server:
                        // read them now, not at the next 10 minute look.
                        let stamp = secrets.map(|stamp| (server.clone(), stamp));
                        if stamp.is_some() && stamp != self.secrets_stamp {
                            self.secrets_stamp = stamp;
                            self.due_secrets = now;
                        }
                        let mut snapshot = lock(&self.snapshot);
                        if snapshot.secret_shows.as_ref().map(|(_, shows)| shows) != Some(&shows) {
                            snapshot.secret_shows = Some((now, shows));
                            snapshot.secret_shows_revision += 1;
                        }
                        if snapshot.players != players || snapshot.map_blocks != blocks {
                            snapshot.players = players;
                            snapshot.map_blocks = blocks;
                            snapshot.revision += 1;
                        }
                    }
                    Err(failure) => {
                        self.due_poll = now + self.backoff;
                        error = Some(failure);
                    }
                }
            }
        }
        for key_id in std::mem::take(&mut self.lookups) {
            match hub.profile(&key_id) {
                Ok(profile) => {
                    let mut snapshot = lock(&self.snapshot);
                    snapshot.profiles.insert(profile.key_id.clone(), profile);
                    snapshot.profiles_revision += 1;
                }
                Err(failure) => {
                    lock(&self.snapshot).notice = Some(failure.to_string());
                }
            }
        }
        // Models and looks are cosmetic: they do not change the status.
        self.send_models(now);
        self.check_models();
        // A look is cosmetic: what became of it is in its outcome, not the status.
        self.send_look(now);
        match error.or(counts_error) {
            Some(failure) => {
                self.backoff = (self.backoff * 2).min(RETRY_MAX);
                self.update(|snapshot| snapshot.status = Status::Failed(failure.to_string()));
            }
            None => {
                self.backoff = RETRY_MIN;
                self.update(|snapshot| {
                    if matches!(snapshot.status, Status::Failed(_)) {
                        snapshot.status = Status::Online;
                    }
                });
            }
        }
    }
}

/// Put the hub's answer about the player's achievements into their profile.
/// Take `rewards` into `snapshot`, counting a change.
fn set_rewards(snapshot: &mut Snapshot, rewards: Rewards) {
    if snapshot.rewards.as_ref() != Some(&rewards) {
        snapshot.rewards = Some(rewards);
        snapshot.rewards_revision += 1;
    }
}

/// Add the announcements the feed relayed (or the creator just made) to the snapshot's,
/// each once, newest first.
fn merge_announcements(snapshot: &mut Snapshot, relayed: Vec<Announcement>) {
    let mut changed = false;
    for announcement in relayed {
        match snapshot
            .announcements
            .iter_mut()
            .find(|known| known.id == announcement.id)
        {
            Some(known) if *known == announcement => {}
            Some(known) => {
                *known = announcement;
                changed = true;
            }
            None => {
                snapshot.announcements.push(announcement);
                changed = true;
            }
        }
    }
    if changed {
        snapshot
            .announcements
            .sort_by_key(|announcement| std::cmp::Reverse(announcement.id));
        snapshot.announcements.truncate(ANNOUNCEMENTS_KEPT);
        snapshot.announcements_revision += 1;
    }
}

/// Why a claim failed, in the player's words.
fn claim_refusal(error: &HubError) -> String {
    match error {
        HubError::Rejected { code, .. } if code == "claimed" => {
            "This reward was claimed already.".to_owned()
        }
        HubError::Rejected { code, .. } if code == "not_found" => {
            "This reward is not waiting for you any more.".to_owned()
        }
        HubError::Rejected { message, .. } if !message.is_empty() => message.clone(),
        HubError::Rejected { code, .. } => format!("The SJK hub refused it ({code})."),
        HubError::Network(_) => "Cannot reach the SJK hub: try again in a moment.".to_owned(),
        HubError::Protocol(why)
            if why.starts_with("status 404") || why.starts_with("status 405") =>
        {
            "This SJK hub has no rewards yet.".to_owned()
        }
        other => other.to_string(),
    }
}

/// An announcement's refusal with the hub's words (or its code when it sent none).
fn announce_refusal(error: HubError) -> HubError {
    match error {
        HubError::Rejected {
            status,
            code,
            message,
        } if message.is_empty() => HubError::Rejected {
            status,
            message: match code.as_str() {
                "creator_only" | "not_staff" | "staff" | "not_registered" => {
                    "Only the SJK team can announce".to_owned()
                }
                "announce_quota" => "Ten announcements an hour at most: wait a while".to_owned(),
                _ => format!("the hub refused it ({code})"),
            },
            code,
        },
        other => other,
    }
}

/// Whether `answer` holds an achievement unlocked that the snapshot's profile did not.
fn newly_unlocked(snapshot: &Snapshot, answer: &[Achievement]) -> bool {
    let Some(me) = snapshot.me.as_ref() else {
        return false;
    };
    answer.iter().any(|achievement| {
        achievement.unlocked != 0
            && !me
                .achievements
                .iter()
                .any(|held| held.id == achievement.id && held.unlocked != 0)
    })
}

fn set_achievements(snapshot: &mut Snapshot, achievements: Vec<Achievement>) {
    if let Some(me) = snapshot.me.as_mut() {
        me.achievements = achievements;
        snapshot.me_revision += 1;
    }
}

/// The running service: commands go in, a [`Snapshot`] comes out.
pub struct Service {
    commands: Sender<Command>,
    /// The worker's [`Worker::latest_look`].
    latest_look: Arc<Mutex<Option<Look>>>,
    snapshot: Arc<Mutex<Snapshot>>,
    staff: Arc<Mutex<StaffState>>,
    chat: Arc<Mutex<ChatState>>,
    emotes: Arc<Mutex<VecDeque<Emote>>>,
    looks: Arc<Mutex<VecDeque<QueuedLook>>>,
    pings: Arc<Mutex<VecDeque<QueuedPing>>>,
    toys: Arc<Mutex<VecDeque<QueuedToyEvent>>>,
    ping_outcome: Arc<Mutex<Option<ReportOutcome>>>,
    /// What the feed is told to read, for the looks' generation.
    feed: Arc<Mutex<FeedShared>>,
    /// Set to end the feed and vitals threads, which end after their request at the latest.
    feed_stop: Arc<std::sync::atomic::AtomicBool>,
    /// What the player shares on their nameplate, for the vitals thread.
    sharing: Arc<Mutex<Sharing>>,
    /// The sharers of the player's server, as the vitals thread last read them.
    vitals: Arc<Mutex<VitalsTable>>,
    /// The toy relay's queues, for its threads (`crate::relay`).
    relay: crate::relay::Shared,
    finished: Mutex<Receiver<()>>,
    /// The last tag given to a note ([`Service::note`]).
    note_tags: std::sync::atomic::AtomicU64,
}

impl Service {
    /// Start the worker thread for `identity`. `make_hub` builds a hub client for
    /// the configured address. Without a feed, the SJK chat sends but never reads.
    pub fn start(identity: Identity, make_hub: HubFactory) -> Self {
        Self::start_with_feed(identity, make_hub, None)
    }

    /// [`Service::start`] with the SJK chat's feed on a thread of its own, reading the
    /// hub through clients `make_feed_hub` builds (with a timeout longer than the
    /// hub's wait, [`crate::feed::TIMEOUT`]).
    pub fn start_with_feed(
        identity: Identity,
        make_hub: HubFactory,
        make_feed_hub: Option<HubFactory>,
    ) -> Self {
        Self::start_with_threads(identity, make_hub, make_feed_hub, None, None)
    }

    /// [`Service::start_with_feed`] with shared vitals on a thread of their own too,
    /// reading the hub through clients `make_vitals_hub` builds (with a short
    /// timeout, [`crate::vitals::TIMEOUT`]), and the toy relay on two more through the
    /// clients `relay_hubs` builds.
    pub fn start_with_threads(
        identity: Identity,
        make_hub: HubFactory,
        make_feed_hub: Option<HubFactory>,
        make_vitals_hub: Option<HubFactory>,
        relay_hubs: Option<RelayHubs>,
    ) -> Self {
        let snapshot = Arc::new(Mutex::new(Snapshot::new(&identity)));
        let staff = Arc::new(Mutex::new(StaffState::default()));
        let (commands, inbox) = channel();
        let (done, finished) = channel();
        let feed_identity = identity.clone();
        let vitals_identity = identity.clone();
        let relay_identity = identity.clone();
        let mut worker = Worker::new(
            identity,
            make_hub,
            Arc::clone(&snapshot),
            Arc::clone(&staff),
            Instant::now(),
        );
        let chat = Arc::clone(&worker.chat);
        let latest_look = Arc::clone(&worker.latest_look);
        let ping_outcome = Arc::clone(&worker.ping_outcome);
        let feed = Arc::clone(&worker.feed);
        let own_drops = commands.clone();
        let own_trades = commands.clone();
        let news = commands.clone();
        let emotes = Arc::new(Mutex::new(VecDeque::new()));
        let looks = Arc::new(Mutex::new(VecDeque::new()));
        let feed_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut pings = Arc::default();
        let mut toys = Arc::default();
        if let Some(make_feed_hub) = make_feed_hub {
            let feed = FeedWorker::new(
                feed_identity,
                make_feed_hub,
                Arc::clone(&worker.feed),
                Arc::clone(&chat),
                Arc::clone(&emotes),
                Arc::clone(&looks),
                Box::new(move || {
                    let _ = own_drops.send(Command::OwnDrop);
                }),
                Instant::now(),
            )
            .on_trade(Box::new(move || {
                let _ = own_trades.send(Command::ReadTrades);
            }));
            let mut feed = feed;
            feed.set_news(Box::new(move |relayed| {
                let _ = news.send(Command::News(relayed));
            }));
            pings = feed.pings();
            toys = feed.toys();
            let stop = Arc::clone(&feed_stop);
            let spawned = std::thread::Builder::new()
                .name("sjk-hub-feed".to_owned())
                .spawn(move || crate::feed::run(feed, &stop));
            if let Err(error) = spawned {
                lock(&snapshot).status =
                    Status::Failed(format!("cannot start the chat thread: {error}"));
            }
        }
        let sharing = Arc::new(Mutex::new(Sharing::default()));
        let vitals = Arc::new(Mutex::new(VitalsTable::default()));
        if let Some(make_vitals_hub) = make_vitals_hub {
            let worker = VitalsWorker::new(
                vitals_identity,
                make_vitals_hub,
                Arc::clone(&worker.feed),
                Arc::clone(&sharing),
                Arc::clone(&vitals),
                Instant::now(),
            );
            let stop = Arc::clone(&feed_stop);
            let spawned = std::thread::Builder::new()
                .name("sjk-hub-vitals".to_owned())
                .spawn(move || crate::vitals::run(worker, &stop));
            if let Err(error) = spawned {
                lock(&snapshot).status =
                    Status::Failed(format!("cannot start the vitals thread: {error}"));
            }
        }
        let relay: crate::relay::Shared = Arc::default();
        if let Some(RelayHubs { send, read }) = relay_hubs {
            let reader = crate::relay::Reader::new(
                relay_identity.clone(),
                read,
                Arc::clone(&worker.feed),
                Arc::clone(&relay),
                Instant::now(),
            );
            let stop = Arc::clone(&feed_stop);
            let read_spawned = std::thread::Builder::new()
                .name("sjk-hub-relay-read".to_owned())
                .spawn(move || crate::relay::run_reader(reader, &stop));
            let (feed, shared, stop) = (
                Arc::clone(&worker.feed),
                Arc::clone(&relay),
                Arc::clone(&feed_stop),
            );
            let send_spawned = std::thread::Builder::new()
                .name("sjk-hub-relay-send".to_owned())
                .spawn(move || crate::relay::run_sender(relay_identity, send, feed, shared, &stop));
            if let Err(error) = read_spawned.and(send_spawned) {
                lock(&snapshot).status =
                    Status::Failed(format!("cannot start the toy relay threads: {error}"));
            }
        }
        let spawned = std::thread::Builder::new()
            .name("sjk-identity".to_owned())
            .spawn(move || {
                loop {
                    let wait = worker.tick(Instant::now());
                    match inbox.recv_timeout(wait) {
                        Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => {
                            worker.handle(Command::Stop, Instant::now());
                            break;
                        }
                        Ok(command) => worker.handle(command, Instant::now()),
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                }
                let _ = done.send(());
            });
        if let Err(error) = spawned {
            lock(&snapshot).status =
                Status::Failed(format!("cannot start the identity thread: {error}"));
        }
        Self {
            commands,
            latest_look,
            snapshot,
            staff,
            chat,
            emotes,
            looks,
            pings,
            toys,
            ping_outcome,
            feed,
            feed_stop,
            sharing,
            vitals,
            relay,
            finished: Mutex::new(finished),
            note_tags: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Apply the player's settings; the service starts or stops talking to the hub.
    pub fn configure(&self, settings: Settings) {
        let _ = self.commands.send(Command::Configure(settings));
    }

    /// Keep the hub's asset packs in `directory` (the settings folder's
    /// [`crate::assets::FOLDER`], beside `identity.key`): once registered, at once and
    /// every 6 hours, the worker reads the hub's list and downloads each pack the folder
    /// does not hold as listed, checked against the list's size and SHA-256 and written
    /// in one rename. [`Snapshot::packs_revision`] counts the packs written. Until this
    /// is called no asset is asked for.
    pub fn keep_assets(&self, directory: PathBuf) {
        let _ = self.commands.send(Command::AssetsDir(directory));
    }

    /// Keep the hub's map edits in `directory` (the settings folder's
    /// [`crate::map_edits::FOLDER`]): once registered, at once and every 10 minutes.
    /// [`Snapshot::map_edits_revision`] counts the documents changed.
    pub fn keep_map_edits(&self, directory: PathBuf) {
        let _ = self.commands.send(Command::MapEditsDir(directory));
    }

    /// Look at the hub's map edits now rather than at the next 10 minutes (the editor
    /// turned on), once registered.
    pub fn refresh_map_edits(&self) {
        let _ = self.commands.send(Command::RefreshMapEdits);
    }

    /// Open mystery box `id`, which the player stands at on `server`: what came of it
    /// arrives in [`Snapshot::mystery_box`], the credits after in the profile.
    pub fn open_mystery_box(&self, id: i64, server: String) {
        let _ = self.commands.send(Command::OpenMysteryBox { id, server });
    }

    /// SJK's creator: place `placed` (its id is ignored); the answer arrives in
    /// [`Snapshot::mystery_box`] and the list is read again.
    pub fn place_mystery_box(&self, placed: crate::mystery_box::MysteryBox) {
        let _ = self.commands.send(Command::PlaceMysteryBox(placed));
    }

    /// SJK's creator: remove mystery box `id`.
    pub fn remove_mystery_box(&self, id: i64) {
        let _ = self.commands.send(Command::RemoveMysteryBox(id));
    }

    /// Ask the hub whether the player may sell fished junk, at what prices and how much
    /// it still pays today: the answer arrives in [`Snapshot::junk`].
    pub fn read_junk(&self) {
        let _ = self.commands.send(Command::ReadJunk);
    }

    /// Sell `held`, pieces of fished junk by id: what the hub took arrives in
    /// [`Snapshot::junk_sale`], the credits after in the profile.
    pub fn sell_junk(&self, held: std::collections::BTreeMap<String, u32>) {
        let _ = self.commands.send(Command::SellJunk(held));
    }

    /// The player pressed Use `presses` times on the Toy Master's toy `id` on `server` in
    /// map load `round` (the game's `sv_serverid`): what came of it arrives in
    /// [`Snapshot::secret`], a find's profile in [`Snapshot::me`].
    pub fn use_secret(&self, id: i64, server: String, round: i32, presses: u32) {
        let _ = self.commands.send(Command::UseSecret {
            id,
            server,
            round,
            presses,
        });
    }

    /// Take drop `drop` of toy `id`'s show on `server` in map load `round`: what it gave
    /// arrives in [`Snapshot::secret`], the profile after in [`Snapshot::me`].
    pub fn take_secret_drop(&self, id: i64, server: String, round: i32, drop: u32) {
        let _ = self.commands.send(Command::TakeSecretDrop {
            id,
            server,
            round,
            drop,
        });
    }

    /// SJK's creator: hide a toy; the answer arrives in [`Snapshot::secret`] and the list
    /// is read again.
    pub fn place_secret(&self, placing: crate::toy_master::Placing) {
        let _ = self.commands.send(Command::PlaceSecret(placing));
    }

    /// SJK's creator: take the Toy Master's toy `id` away.
    pub fn remove_secret(&self, id: i64) {
        let _ = self.commands.send(Command::RemoveSecret(id));
    }

    /// SJK's creator: show the Toy Master's toy `id` only on the server they are on
    /// (`escorted`), or on every server again.
    pub fn escort_secret(&self, id: i64, escorted: bool) {
        let _ = self.commands.send(Command::EscortSecret(id, escorted));
    }

    /// Read the box hunt on `server`, saying whether the player is `away` (in the chat,
    /// a menu, idle: the hub picks no mark among them): it arrives in
    /// [`Snapshot::box_hunt`].
    pub fn read_box_hunt(&self, server: String, away: bool) {
        let _ = self.commands.send(Command::BoxHunt { server, away });
    }

    /// The player, the box hunt's mark on `server`, was killed by the player on slot
    /// `by`; the hunt is read again after, the player `away` or not.
    pub fn box_hunt_killed(&self, server: String, by: u8, away: bool) {
        let _ = self
            .commands
            .send(Command::BoxHuntKilled { server, by, away });
    }

    /// The player is out of the box hunt's chase on `server` (killed by the mark, or
    /// caught cheating); the hunt is read again after, the player `away` or not.
    pub fn box_hunt_out(&self, server: String, out: crate::mystery_box::HuntOut, away: bool) {
        let _ = self
            .commands
            .send(Command::BoxHuntOut { server, out, away });
    }

    /// Staff: publish `edits` for `map` and `checksum`; the answer lands in
    /// [`Snapshot::map_edits_published`].
    pub fn publish_map_edits(&self, map: String, checksum: i32, edits: Value) {
        let _ = self.commands.send(Command::PublishMapEdits {
            map,
            checksum,
            edits,
        });
    }

    /// Send the crash reports waiting in `directory` (`crate::crash`) once registered,
    /// and look there again every hour; [`Snapshot::crash_note`] says what went.
    /// `None` sends none from now on.
    pub fn send_crashes(&self, directory: Option<PathBuf>) {
        let _ = self.commands.send(Command::Crashes(directory));
    }

    /// Whether the player is actively playing right now: on a live game server, not
    /// spectating, not at the intermission, not in a menu, and with input in the last
    /// two minutes. Every claim says so, and the hub counts the time between claims
    /// toward the player's next holocron only while it does (`PROTOCOL.md`,
    /// "Holocrons"). Defaults to false, which earns nothing.
    pub fn set_active(&self, active: bool) {
        let _ = self.commands.send(Command::Active(active));
    }

    /// Add what the player did lately (moves, attacks, hits, hurts, kills, deaths while
    /// actively playing) to what the next active claim says (`"play"`, 03/11/2026): the
    /// hub counts the time toward holocrons by real play, not by presence.
    pub fn add_play(&self, play: crate::wire::PlaySignals) {
        let _ = self.commands.send(Command::Play(play));
    }

    /// Break one tag copy down into kyber shards (hubs whose economy says `tag_copies`):
    /// what came of it arrives in [`Snapshot::tag_trade`], and the profile after it.
    pub fn shatter_tag(&self, copy: crate::wire::TagCopy) {
        let _ = self.commands.send(Command::ShatterTag(copy));
    }

    /// Give two tag copies of one tier and finish for a tag not held yet (hubs whose
    /// economy says `tag_copies`): what came of it arrives in [`Snapshot::tag_trade`].
    pub fn craft_tags(&self, copies: [crate::wire::TagCopy; 2]) {
        let _ = self.commands.send(Command::CraftTags(copies));
    }

    /// Ask for fresh holocron progress ([`Snapshot::holocrons`]); read at most once
    /// every 30 seconds, for a page that shows it as it opens.
    pub fn refresh_holocrons(&self) {
        let _ = self.commands.send(Command::RefreshHolocrons);
    }

    /// The game menu opened: read the profile, the holocron progress and the trades
    /// again, and send changed achievement counts, each with a short cooldown.
    pub fn menu_opened(&self) {
        let _ = self.commands.send(Command::MenuOpened);
    }

    /// Open the player's holocrons `ids` (their own, not opened yet; at most
    /// [`crate::wire::OPEN_MAX`] are taken): what they brought arrives in
    /// [`Snapshot::opening`], and the profile after it.
    pub fn open_holocrons(&self, ids: Vec<u64>) {
        let _ = self.commands.send(Command::OpenHolocrons(ids));
    }

    /// Reveal the player's sealed cards whose rarity has cards at the hub: what they
    /// became arrives in [`Snapshot::reveal`], and the profile after it.
    pub fn reveal_cards(&self) {
        let _ = self.commands.send(Command::RevealCards);
    }

    /// Shatter one duplicate of shader `id` into kyber shards (never the last copy): what
    /// came of it arrives in [`Snapshot::trade`], and the profile after it.
    pub fn shatter(&self, id: &str) {
        let _ = self.commands.send(Command::Shatter(id.to_owned()));
    }

    /// Shatter one copy of shader `id`, the last one too (hubs whose economy says
    /// `shatter_last`, 21/10/2026): what came of it arrives in [`Snapshot::trade`], and the
    /// profile after it.
    pub fn shatter_last(&self, id: &str) {
        let _ = self.commands.send(Command::ShatterLast(id.to_owned()));
    }

    /// Shatter every duplicate the player holds, one copy of each kept (hubs whose economy
    /// says `shatter_duplicates`, 21/10/2026): what came of it arrives in
    /// [`Snapshot::trade`] (its `copies` and `amount` the totals), and the profile after it.
    pub fn shatter_duplicates(&self) {
        let _ = self.commands.send(Command::ShatterDuplicates);
    }

    /// Craft shader `id` with kyber shards: what came of it arrives in
    /// [`Snapshot::trade`], and the profile after it.
    pub fn craft(&self, id: &str) {
        let _ = self.commands.send(Command::Craft(id.to_owned()));
    }

    /// Set (`on`) or clear collection mark `mark` on shader `id` (a hub whose economy says
    /// [`crate::wire::MARKS_ABILITY`], 29/10/2026). [`Snapshot::marks`] shows the change at
    /// once; the hub's answer replaces it, and a refusal puts the hub's marks back
    /// ([`Snapshot::mark_refusal`] says why). Nothing while the hub's marks are not known.
    pub fn set_mark(&self, id: &str, mark: crate::wire::Mark, on: bool) {
        if mark_at_once(&mut lock(&self.snapshot), id, mark, on) {
            let _ = self.commands.send(Command::Mark(id.to_owned(), mark, on));
        }
    }

    /// Ask for the player's rewards and the newest rounds again ([`Snapshot::rewards`]);
    /// read at most once every 30 seconds, for a window that shows them as it opens.
    pub fn refresh_rewards(&self) {
        let _ = self.commands.send(Command::RefreshRewards);
    }

    /// Claim reward `id` (one of [`Snapshot::rewards`]'s pending): what became of it
    /// arrives in [`Snapshot::claim`], the profile and the rewards after it.
    pub fn claim_reward(&self, id: u64) {
        let _ = self.commands.send(Command::ClaimReward(id));
    }

    /// The creator's announcement: checked here ([`crate::news::check`]), then sent; what
    /// became of it arrives in [`Snapshot::announce`]. The hub refuses anyone but the
    /// creator.
    pub fn announce(&self, request: AnnounceRequest) {
        let _ = self.commands.send(Command::Announce(request));
    }

    /// Upload a news picture (a PNG or JPEG, `mime`) for an announcement: the hub's id
    /// and size, or why it refused, arrive in [`Snapshot::news_image`]. The hub refuses
    /// anyone but the SJK team.
    pub fn upload_news_image(&self, bytes: Vec<u8>, mime: &'static str) {
        let _ = self.commands.send(Command::UploadNewsImage(bytes, mime));
    }

    /// Put `announcements` (oldest first, read from the hub: made while the player was
    /// away) at the end of the SJK chat, each once ([`crate::feed::post_announcements`]).
    /// Returns the ids of those the chat now holds; empty while it cannot take them (the
    /// chat off, or its first answer still to come), so they wait for a later call.
    pub fn post_announcements(&self, announcements: &[Announcement]) -> Vec<u64> {
        crate::feed::post_announcements(&self.chat, announcements)
    }

    /// Read a page of the player's history into [`Snapshot::history`]: the newest
    /// (`None`, which starts the list again), or the one before `before`
    /// ([`crate::HistoryState::before`], added to the list).
    pub fn read_history(&self, before: Option<i64>) {
        let _ = self.commands.send(Command::ReadHistory(before));
    }

    /// Read the shop's offers of the day into [`Snapshot::shop`].
    pub fn read_shop(&self) {
        let _ = self.commands.send(Command::ReadShop);
    }

    /// Buy a holocron of tier `tier` with credits: what came of it arrives in
    /// [`Snapshot::trade`] (its `id` the tier), the holocron in the profile.
    pub fn buy_holocron(&self, tier: &str) {
        let _ = self.commands.send(Command::BuyHolocron(tier.to_owned()));
    }

    /// Buy today's offer `id`: what came of it arrives in
    /// [`Snapshot::trade`], and the profile after it.
    pub fn buy(&self, id: &str) {
        let _ = self.commands.send(Command::Buy(id.to_owned()));
    }

    /// Read the player's trades with other players again ([`Snapshot::trades`]).
    pub fn read_trades(&self) {
        let _ = self.commands.send(Command::ReadTrades);
    }

    /// Ask the player whose hub id is `key_id` to trade: the trade, or why not, arrives
    /// in [`Snapshot::trades`].
    pub fn trade_ask(&self, key_id: &str) {
        let _ = self.commands.send(Command::TradeAsk(key_id.to_owned()));
    }

    /// Take `step` in trade `id`: the trade after it, or why not, arrives in
    /// [`Snapshot::trades`], and once the swap is done the profile after it.
    pub fn trade_step(&self, id: i64, step: crate::trades::TradeStep) {
        let _ = self.commands.send(Command::TradeStep(id, step));
    }

    /// The player is on a game server (call again if `slot` or `name` changes).
    pub fn enter(&self, location: Location) {
        let _ = self.commands.send(Command::Enter(location));
    }

    /// The player left their game server.
    pub fn leave(&self) {
        let _ = self.commands.send(Command::Leave);
    }

    /// The in-game name the player wears: sent with the registration and again
    /// whenever it changes, for the key's name history at the hub.
    pub fn set_name(&self, name: String) {
        let _ = self.commands.send(Command::Name(name));
    }

    /// Equip a profile banner. The hub validates supported ids and current ownership.
    pub fn set_banner(&self, id: String) {
        let _ = self.commands.send(Command::SetBanner(id));
    }

    /// Change the player's bio at the hub.
    pub fn set_bio(&self, bio: String) {
        let _ = self.commands.send(Command::SetBio(bio));
    }

    /// Change the player's name emoji at the hub (`crate::name_emoji`); empty takes it
    /// away.
    pub fn set_emoji(&self, emoji: String) {
        let _ = self.commands.send(Command::SetEmoji(emoji));
    }

    /// Make `png` (cropped and scaled to [`crate::avatar::SIZE`] square) the player's
    /// picture; the outcome arrives in [`Snapshot::avatar`], the new version in the
    /// profile's `avatar`.
    pub fn set_avatar(&self, png: Vec<u8>) {
        let _ = self.commands.send(Command::SetAvatar(png));
    }

    /// Take the player's picture down; the outcome arrives in [`Snapshot::avatar`].
    pub fn remove_avatar(&self) {
        let _ = self.commands.send(Command::RemoveAvatar);
    }

    /// The counts the client keeps for the achievements it counts, by id: sent to the
    /// hub once registered, at most once a minute, and again when the hub held part
    /// back. The hub's answer arrives in the profile's `achievements`.
    pub fn set_achievement_counts(&self, counts: BTreeMap<String, u64>) {
        let _ = self.commands.send(Command::Achievements(counts));
    }

    /// Send a staff request; its answer arrives in [`Service::staff_state`].
    pub fn staff(&self, request: StaffRequest) {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .busy = true;
        let _ = self.commands.send(Command::Staff(request));
    }

    /// Send a staff query about logs or crash reports; its answer arrives in
    /// [`Service::staff_state`], and a text opened in [`Service::take_opened`].
    pub fn staff_query(&self, query: StaffQuery) {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .busy = true;
        let _ = self.commands.send(Command::StaffQuery(query));
    }

    /// The logs or crash report a staff query opened, once.
    pub fn take_opened(&self) -> Option<OpenedText> {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .opened
            .take()
    }

    /// Answer the SJK team's log request `id`: send `upload`, or decline with `None`.
    /// The request leaves [`Snapshot::log_requests`] at once; the outcome arrives in
    /// [`Snapshot::log_answer`].
    pub fn answer_logs(&self, id: i64, upload: Option<LogUpload>) {
        lock(&self.snapshot)
            .log_requests
            .retain(|request| request.id != id);
        let _ = self.commands.send(Command::AnswerLogs(id, upload));
    }

    /// What staff requests brought back (without a text opened, which
    /// [`Service::take_opened`] hands over).
    pub fn staff_state(&self) -> StaffState {
        self.staff
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Send a bug report; its outcome arrives in [`Snapshot::report`].
    pub fn report(&self, report: BugReport) {
        let _ = self.commands.send(Command::Report(report));
    }

    /// Send a report about another player; its outcome arrives in
    /// [`Snapshot::player_report`]. Only a verified key may.
    pub fn player_report(&self, report: PlayerReport) {
        let _ = self.commands.send(Command::PlayerReport(report));
    }

    /// Send a world note; its outcome arrives in [`Snapshot::note`]. The answer tags
    /// the note for [`Service::note_image`].
    pub fn note(&self, note: WorldNote) -> u64 {
        let tag = self
            .note_tags
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let _ = self.commands.send(Command::Note(tag, Box::new(note)));
        tag
    }

    /// The picture of the note tagged `tag`, a JPEG; sent once the hub took the note.
    pub fn note_image(&self, tag: u64, jpeg: Vec<u8>) {
        let _ = self.commands.send(Command::NoteImage(tag, jpeg));
    }

    /// Fetch a player's profile (their bio) into [`Snapshot::profiles`].
    pub fn look_up(&self, key_id: String) {
        let _ = self.commands.send(Command::LookUp(key_id));
    }

    /// The player model folders the game has (`PROTOCOL.md`, "Models"), sent to the
    /// hub once registered and again whenever the list changes. Names are lower case.
    pub fn set_models(&self, models: Vec<String>) {
        let _ = self.commands.send(Command::Models(models));
    }

    /// Ask the hub whether the player of `slot` on `server` has `model`; the answer
    /// comes in [`Snapshot::model_checks`].
    pub fn check_model(&self, server: SocketAddr, slot: u8, model: String) {
        let _ = self.commands.send(Command::CheckModel(server, slot, model));
    }

    /// A copy of the current state.
    pub fn snapshot(&self) -> Snapshot {
        lock(&self.snapshot).clone()
    }

    /// Read the current state without copying it; keep `read` short, the worker
    /// waits for the lock.
    pub fn with_snapshot<R>(&self, read: impl FnOnce(&Snapshot) -> R) -> R {
        read(&lock(&self.snapshot))
    }

    /// Turn the SJK chat on or off. Off, the feed still reads the hub while the
    /// player is on a game server (for its looks and emotes) but keeps no message; in
    /// the menus it reads only with the chat on.
    pub fn set_chat(&self, on: bool) {
        let _ = self.commands.send(Command::SetChat(on));
    }

    /// Send an SJK chat message; what became of it arrives in [`ChatState::outcome`].
    /// Add or remove this identity's reaction; the authoritative feed updates counts.
    pub fn react(&self, message: u64, emoji: String, active: bool) {
        let _ = self.commands.send(Command::React {
            message,
            emoji,
            active,
        });
    }

    /// Send a checked SJK chat message through the identity worker.
    pub fn chat(&self, text: String) {
        let _ = self.commands.send(Command::Chat(text));
    }

    /// Send a message to a clan chat ([`crate::channels`]) through the identity worker,
    /// under the SJK chat's rules; what became of it arrives in [`ChatState::outcome`].
    pub fn clan_chat(&self, text: String, channel: crate::HubChannel) {
        let _ = self.commands.send(Command::ClanChat(text, channel));
    }

    /// Read the clan chats `joined` with the chat (the ones the hub lists, the player
    /// joined: [`crate::channels::joined`]).
    pub fn set_clan_chats(&self, joined: Vec<crate::HubChannel>) {
        let _ = self.commands.send(Command::SetClanChats(joined));
    }

    /// Ban the player of `key_id` from the SJK chat for 24 hours ([`crate::ban`]); what
    /// became of it arrives in [`ChatState::ban`].
    pub fn chat_ban(&self, key_id: String) {
        let _ = self.commands.send(Command::ChatBan(key_id));
    }

    /// Play an emote for the player's slot on their server, in step with slot `sync`'s
    /// when it is set; what became of it arrives in [`ChatState::outcome`].
    pub fn emote(&self, emote: String, sync: Option<u8>) {
        let _ = self.commands.send(Command::Emote(emote, sync, false));
    }

    /// Play an emote as [`Service::emote`] does, without its sound for the others (the
    /// player is away from the game).
    pub fn emote_quiet(&self, emote: String) {
        let _ = self.commands.send(Command::Emote(emote, None, true));
    }

    /// Read the chat without copying it; keep `read` short.
    pub fn with_chat<R>(&self, read: impl FnOnce(&ChatState) -> R) -> R {
        read(&lock_chat(&self.chat))
    }

    /// The emotes received since the last call, oldest first.
    pub fn take_emotes(&self) -> Vec<Emote> {
        crate::feed::lock(&self.emotes).drain(..).collect()
    }

    /// Mark `pos` (whole game units) for `scope` (`PROTOCOL.md`, "Pings") from the slot
    /// the player claims on their server; what became of it arrives in
    /// [`Service::ping_outcome`].
    pub fn ping(&self, pos: [i32; 3], scope: String) {
        let _ = self.commands.send(Command::Ping(pos, scope));
    }

    /// What became of the last ping sent; `None` before one.
    pub fn ping_outcome(&self) -> Option<ReportOutcome> {
        crate::feed::lock(&self.ping_outcome).clone()
    }

    /// The pings the feed received for `server`, the game server the player is on,
    /// since the last call, oldest first; those read for another server are dropped.
    pub fn take_pings(&self, server: Option<SocketAddr>) -> Vec<Ping> {
        crate::feed::take_pings(&self.pings, server)
    }

    /// Tell the SJK players on the player's server that the player's toy `toy` did
    /// `event` to the player in slot `target` (`PROTOCOL.md`, "Toy events"), from the
    /// slot the player claims. Nothing comes back: the sender shows the effect at once.
    pub fn toy_event(&self, toy: String, event: String, target: u8) {
        let _ = self.commands.send(Command::Toy(toy, event, target));
    }

    /// The toy events the feed received for `server`, as [`Service::take_pings`].
    pub fn take_toy_events(&self, server: Option<SocketAddr>) -> Vec<ToyEvent> {
        crate::feed::take_toy_events(&self.toys, server)
    }

    /// Relay `event` of the player's toy `toy` to the player in slot `target` at once
    /// (`PROTOCOL.md`, "Toy relay"), from the slot the player claims. With `replace`, a
    /// later one that also replaces takes its place while it waits (control frames).
    pub fn relay_toy_event(&self, toy: String, event: String, target: u8, replace: bool) {
        let (state, bell) = &*self.relay;
        crate::feed::lock(state).push(crate::relay::Outgoing {
            toy,
            event,
            target,
            replace,
        });
        bell.notify_one();
    }

    /// Whether the toy relay reads the events meant for the player (a toy that may be
    /// acted on is out); off drops the long poll at its next turn.
    pub fn set_relay_listen(&self, listen: bool) {
        crate::feed::lock(&self.relay.0).set_listen(listen);
    }

    /// The relayed toy events read for `server`, as [`Service::take_toy_events`].
    pub fn take_relay_events(&self, server: Option<SocketAddr>) -> Vec<ToyEvent> {
        let queued = std::mem::take(crate::feed::lock(&self.relay.0).inbox());
        crate::feed::take_toy_events(&Mutex::new(queued), server)
    }

    /// Whether the player takes encrypted whispers (`PROTOCOL.md`, "Whispers"): the
    /// toy relay's reader asks for them, and the hub gives the player's key to the
    /// others on the server meanwhile.
    pub fn set_whisper_listen(&self, listen: bool) {
        crate::feed::lock(&self.relay.0).set_whispers(listen);
    }

    /// Seal `whisper` and send it through the hub at once; what became of it comes back
    /// from [`Service::take_whispers_sent`] under its serial.
    pub fn send_whisper(&self, whisper: crate::whisper::Outgoing) {
        let (state, bell) = &*self.relay;
        crate::feed::lock(state).push_whisper(whisper);
        bell.notify_one();
    }

    /// What became of the whispers sent since the last call.
    pub fn take_whispers_sent(&self) -> Vec<crate::whisper::Sent> {
        crate::feed::lock(&self.relay.0).take_sent()
    }

    /// The encrypted whispers read for `server` since the last call, opened.
    pub fn take_whispers(&self, server: Option<SocketAddr>) -> Vec<crate::whisper::Received> {
        crate::feed::lock(&self.relay.0).take_received(server)
    }

    /// The look the player wears (`PROTOCOL.md`, "Looks"). The worker keeps the
    /// latest and sends it once its claim on a server is accepted, again when it
    /// changes or the claim does (another server, slot or name), at most once a
    /// second; a blade skin the hub says the key does not hold is left out until the
    /// profile changes. What became of it arrives in [`Snapshot::look_outcome`].
    pub fn set_look(&self, look: Look) {
        *self
            .latest_look
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(look);
        let _ = self.commands.send(Command::LookWaiting);
    }

    /// The looks the feed received since the last call, oldest first: those it read
    /// for `server`, the game server the player is on, under the current reading. A
    /// poll still under way when the server changed or the identity went off brings
    /// none, and [`ReceivedLooks::generation`] tells when every look had before is out
    /// of date. The newest look of the player's own key goes to the worker, which reads
    /// the own profile again soon when the hub took a skin back.
    pub fn take_looks(&self, server: Option<SocketAddr>) -> ReceivedLooks {
        let received = crate::feed::take_looks(&self.feed, &self.looks, server);
        if received.events.is_empty() {
            return received;
        }
        // The player's own id follows the person this key belongs to.
        let own_id = lock(&self.snapshot).key_id.clone();
        if let Some(own) = received
            .events
            .iter()
            .rev()
            .find(|event| event.key_id == own_id)
        {
            let _ = self
                .commands
                .send(Command::OwnLook(own.id, own.saber.clone()));
        }
        received
    }

    /// What the player shares on their nameplate (`PROTOCOL.md`, "Vitals"); the vitals
    /// thread sends it with its next request, at once when `share` or `estimates`
    /// changed. Cheap: call it every frame.
    pub fn set_sharing(&self, sharing: Sharing) {
        *crate::feed::lock(&self.sharing) = sharing;
    }

    /// Read the sharers of the player's game server, as the hub last listed them.
    pub fn with_vitals<R>(&self, read: impl FnOnce(&VitalsTable) -> R) -> R {
        read(&crate::feed::lock(&self.vitals))
    }

    /// Withdraw the claim and stop, waiting at most `timeout` for it. The feed and
    /// vitals threads are told to stop and ends after their request, without being waited for.
    pub fn shutdown(&self, timeout: Duration) {
        self.feed_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = self.commands.send(Command::Stop);
        let finished = self
            .finished
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = finished.recv_timeout(timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{FOLDER, Pack, sha256_hex, write_pack};
    use crate::ban::ChatBan;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// What the fake hub serves of announcements and rewards.
    type News = (Vec<Announcement>, Rewards);

    /// A hub that records what it is asked.
    #[derive(Clone, Default)]
    struct Fake {
        log: Arc<Mutex<Vec<String>>>,
        fail: Arc<AtomicBool>,
        /// Refuses an emote's `sync`, as a hub from before 15/10/2026 does.
        old_hub: Arc<AtomicBool>,
        roster: Arc<Mutex<Vec<Presence>>>,
        /// The most of any count the fake hub takes, as its hourly allowance would.
        cap: Arc<Mutex<Option<u64>>>,
        /// The achievements the fake hub holds, as its profiles list them.
        held: Arc<Mutex<Vec<Achievement>>>,
        /// The unlocks the fake hub holds for the player's key.
        owned: Arc<Mutex<Vec<String>>>,
        /// Refusals the next looks get, in order.
        look_refusals: Arc<Mutex<VecDeque<HubError>>>,
        /// The asset packs the fake hub serves, by name.
        packs: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
        /// Bytes every pack download gets instead of the pack's own.
        served: Arc<Mutex<Option<Vec<u8>>>>,
        /// Refusals the next readings of the asset list get, in order.
        asset_refusals: Arc<Mutex<VecDeque<HubError>>>,
        /// The holocron progress the fake hub reports; none is a hub from before
        /// holocrons, which refuses the reading.
        progress: Arc<Mutex<Option<HolocronState>>>,
        /// Refusals the next readings of the holocron progress get, in order.
        progress_refusals: Arc<Mutex<VecDeque<HubError>>>,
        /// How many times the holocron progress was asked for.
        holocron_reads: Arc<Mutex<usize>>,
        /// The open log requests the fake hub lists; none is a hub from before log
        /// requests, which refuses the reading.
        log_asks: Arc<Mutex<Option<Vec<LogRequest>>>>,
        /// How many times the log requests were looked for.
        log_ask_reads: Arc<Mutex<usize>>,
        /// How many times the player's trades were read (kept out of `log`).
        trade_reads: Arc<Mutex<usize>>,
        /// The hub says trading is off.
        trading_off: Arc<AtomicBool>,
        /// The other player confirmed last: the trade with Nib is done.
        partner_swapped: Arc<AtomicBool>,
        /// The announcements and rewards the fake hub serves; none is a hub from before
        /// them, which answers 404.
        news: Arc<Mutex<Option<News>>>,
        /// How many times the announcements and the rewards were read.
        news_reads: Arc<Mutex<(usize, usize)>>,
        /// The collection marks the fake hub keeps; none is a hub from before them, which
        /// sends none with the progress and answers a change with 404.
        marks: Arc<Mutex<Option<crate::wire::UnlockMarks>>>,
        /// The map edits stamp presence answers carry; none is a hub from before it.
        edits_stamp: Arc<Mutex<Option<i64>>>,
        /// The Toy Master stamp its presence answers carry.
        secrets_stamp: Arc<Mutex<Option<i64>>>,
    }

    impl Fake {
        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn unlocks(&self) -> Vec<Unlock> {
            self.owned
                .lock()
                .unwrap()
                .iter()
                .map(|id| Unlock {
                    holo: 0,
                    solar: 0,
                    id: id.clone(),
                    granted: 1,
                    note: String::new(),
                    count: 1,
                    medal: None,
                    card: None,
                })
                .collect()
        }

        /// The asset list readings and pack downloads asked for.
        fn asset_requests(&self) -> Vec<String> {
            self.log()
                .into_iter()
                .filter(|line| line.starts_with("asset"))
                .collect()
        }

        fn serve(&self, name: &str, bytes: &[u8]) {
            self.packs
                .lock()
                .unwrap()
                .insert(name.to_owned(), bytes.to_vec());
        }

        fn looks(&self) -> Vec<String> {
            self.log()
                .into_iter()
                .filter(|line| line.starts_with("look "))
                .collect()
        }

        fn record(&self, line: String) -> Result<(), HubError> {
            self.log.lock().unwrap().push(line);
            if self.fail.load(Ordering::SeqCst) {
                Err(HubError::Network("down".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    fn profile(name: &str) -> Profile {
        Profile {
            tag_cards: Vec::new(),
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: name.to_owned(),
            banner: "".into(),
            bio: String::new(),
            verified: false,
            tick: crate::Tick::None,
            staff: true,
            creator: false,
            jkr_creator: false,
            trust: None,
            emoji: String::new(),
            created: 0,
            names: Vec::new(),
            medals: Vec::new(),
            achievements: Vec::new(),
            avatar: String::new(),
            unlocks: Vec::new(),
            holocron_counts: crate::wire::HolocronCounts::default(),
            holocrons: Vec::new(),
            credits: 0,
            shards: 0,
            cards: Vec::new(),
            keys: Vec::new(),
        }
    }

    impl Hub for Fake {
        fn register(&mut self, _: &Identity, name: Option<&str>) -> Result<Profile, HubError> {
            let unlocks = self.unlocks();
            match name {
                Some(name) => self.record(format!("register {name}")).map(|()| Profile {
                    unlocks,
                    ..profile(name)
                }),
                None => self.record("register".to_owned()).map(|()| Profile {
                    unlocks,
                    ..profile("")
                }),
            }
        }
        fn set_bio(&mut self, _: &Identity, bio: &str) -> Result<Profile, HubError> {
            self.record(format!("bio {bio}")).map(|()| profile(""))
        }
        fn set_banner(&mut self, _: &Identity, id: &str) -> Result<Profile, HubError> {
            self.record(format!("banner {id}"))?;
            let mut profile = profile("");
            profile.banner = id.into();
            Ok(profile)
        }
        fn set_emoji(&mut self, _: &Identity, emoji: &str) -> Result<Profile, HubError> {
            self.record(format!("emoji {emoji}")).map(|()| Profile {
                emoji: emoji.to_owned(),
                ..profile("Sol")
            })
        }
        fn set_avatar(&mut self, _: &Identity, png: &[u8]) -> Result<Profile, HubError> {
            Fake::record(self, format!("avatar {} bytes", png.len())).map(|()| Profile {
                avatar: "0123456789abcdef".to_owned(),
                ..profile("Sol")
            })
        }
        fn remove_avatar(&mut self, _: &Identity) -> Result<Profile, HubError> {
            Fake::record(self, "avatar removed".to_owned()).map(|()| profile("Sol"))
        }
        fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
            self.record(format!("lookup {key_id}")).map(|()| Profile {
                achievements: self.held.lock().unwrap().clone(),
                unlocks: self.unlocks(),
                ..profile("Other")
            })
        }
        fn claim(
            &mut self,
            _: &Identity,
            server: &str,
            slot: u8,
            name: &str,
            active: bool,
        ) -> Result<(), HubError> {
            let tail = if active { " active" } else { "" };
            self.record(format!("claim {server} {slot} {name}{tail}"))
        }
        fn open_holocrons(
            &mut self,
            _: &Identity,
            ids: &[u64],
        ) -> Result<crate::wire::OpenAnswer, HubError> {
            let list: Vec<String> = ids.iter().map(u64::to_string).collect();
            self.record(format!("open {}", list.join(",")))?;
            if ids.contains(&99) {
                return Err(refused(409, "holocron_opened"));
            }
            let opened: Vec<crate::wire::Opened> = ids
                .iter()
                .map(|&id| crate::wire::Opened {
                    banner: None,
                    holocron: id,
                    // 77 is a kyber crystal.
                    tier: if id == 77 { "kyber_rare" } else { "rare" }.to_owned(),
                    card: Some(crate::wire::OpenedCard {
                        id: 1_000 + id,
                        card: None,
                        rarity: "common".to_owned(),
                        finish: "holo".to_owned(),
                        edition: "alpha".to_owned(),
                    }),
                    credits: 40,
                    unlock: None,
                    shards: 0,
                })
                .collect();
            let mut profile = profile("Sol");
            profile.credits = 40 * ids.len() as u64;
            Ok(crate::wire::OpenAnswer { opened, profile })
        }
        fn shatter_unlock(
            &mut self,
            _: &Identity,
            id: &str,
            last: bool,
        ) -> Result<crate::wire::ShatterAnswer, HubError> {
            self.record(format!("shatter {id}{}", if last { " last" } else { "" }))?;
            if id == "saber_last" && !last {
                return Err(refused(409, "not_a_duplicate"));
            }
            let mut profile = profile("Sol");
            profile.shards = 5;
            Ok(crate::wire::ShatterAnswer {
                shattered: crate::wire::Shattered {
                    id: id.to_owned(),
                    count: 1,
                    shards: 5,
                    balance: 5,
                },
                profile,
            })
        }
        fn shatter_duplicates(
            &mut self,
            _: &Identity,
        ) -> Result<crate::wire::ShatterAllAnswer, HubError> {
            self.record("shatter duplicates".to_owned())?;
            let mut profile = profile("Sol");
            profile.shards = 165;
            Ok(crate::wire::ShatterAllAnswer {
                shattered: vec![crate::wire::ShatteredCopies {
                    id: "saber_tidal".to_owned(),
                    copies: 3,
                    shards: 15,
                }],
                copies: 3,
                shards: 15,
                balance: 165,
                profile,
            })
        }
        fn craft_unlock(
            &mut self,
            _: &Identity,
            id: &str,
        ) -> Result<crate::wire::CraftAnswer, HubError> {
            self.record(format!("craft {id}"))?;
            Err(refused(409, "not_enough_shards"))
        }
        fn trades(&mut self, _: &Identity) -> Result<crate::trades::TradesAnswer, HubError> {
            *self.trade_reads.lock().unwrap() += 1;
            if self.fail.load(Ordering::SeqCst) {
                return Err(HubError::Network("down".to_owned()));
            }
            // One trade with Nib once it was asked for, open once accepted.
            let log = self.log();
            let trades = if log.iter().any(|line| line == "ask nib") {
                let accepted = log.iter().any(|line| line == "step 3 accept");
                let state = if self.partner_swapped.load(Ordering::SeqCst) {
                    "done"
                } else if accepted {
                    "open"
                } else {
                    "asked"
                };
                vec![crate::trades::PlayerTrade {
                    id: 3,
                    state: state.to_owned(),
                    asker: true,
                    expires: Some(1_791_000_030),
                    ..crate::trades::PlayerTrade::default()
                }]
            } else {
                Vec::new()
            };
            Ok(crate::trades::TradesAnswer {
                trading: if self.trading_off.load(Ordering::SeqCst) {
                    "off"
                } else {
                    "everyone"
                }
                .to_owned(),
                trades,
                now: Some(1_791_000_000),
                ..crate::trades::TradesAnswer::default()
            })
        }
        fn trade_ask(
            &mut self,
            _: &Identity,
            to: &str,
        ) -> Result<crate::trades::TradeAnswer, HubError> {
            self.record(format!("ask {to}"))?;
            if to != "nib" {
                return Err(refused(409, "they_are_busy"));
            }
            Ok(crate::trades::TradeAnswer {
                trade: crate::trades::PlayerTrade::default(),
                profile: None,
            })
        }
        fn trade_step(
            &mut self,
            _: &Identity,
            id: i64,
            step: &crate::trades::TradeStep,
        ) -> Result<crate::trades::TradeAnswer, HubError> {
            self.record(format!("step {id} {}", step.call().0))?;
            let profile = matches!(step, crate::trades::TradeStep::Confirm { .. }).then(|| {
                let mut profile = profile("Sol");
                profile.shards = 77;
                profile
            });
            Ok(crate::trades::TradeAnswer {
                trade: crate::trades::PlayerTrade::default(),
                profile,
            })
        }
        fn history(
            &mut self,
            _: &Identity,
            before: Option<i64>,
        ) -> Result<crate::history::HistoryPage, HubError> {
            self.record(format!("history {before:?}"))?;
            let line = |at| {
                crate::history::HistoryLine::Credits(crate::history::Ledger {
                    at,
                    ..Default::default()
                })
            };
            Ok(match before {
                None => crate::history::HistoryPage {
                    lines: vec![line(30), line(20)],
                    more: true,
                    before: 20,
                },
                Some(_) => crate::history::HistoryPage {
                    lines: vec![line(10)],
                    more: false,
                    before: 10,
                },
            })
        }
        fn shop(&mut self, _: &Identity) -> Result<crate::wire::ShopDay, HubError> {
            self.record("shop".to_owned())?;
            Ok(crate::wire::ShopDay {
                day: "2026-10-17".to_owned(),
                offers: vec![crate::wire::Offer {
                    id: "saber_aurora".to_owned(),
                    name: "Aurora blade".to_owned(),
                    tier: "common".to_owned(),
                    price: 30,
                    bought: false,
                    owned: false,
                }],
                next: 1_791_003_600,
                server_time: 1_791_000_000,
                credits: 300,
                shards: 45,
            })
        }
        fn buy_offer(
            &mut self,
            _: &Identity,
            id: &str,
        ) -> Result<crate::wire::BuyAnswer, HubError> {
            self.record(format!("buy {id}"))?;
            if id != "saber_aurora" {
                return Err(refused(409, "not_on_offer"));
            }
            let mut profile = profile("Sol");
            profile.credits = 300;
            profile.shards = 15;
            Ok(crate::wire::BuyAnswer {
                bought: crate::wire::Bought {
                    id: id.to_owned(),
                    name: "Aurora blade".to_owned(),
                    count: 1,
                    price: 30,
                    balance: 15,
                },
                profile,
            })
        }
        fn buy_holocron(
            &mut self,
            _: &Identity,
            tier: &str,
        ) -> Result<crate::wire::HolocronBuyAnswer, HubError> {
            self.record(format!("buy holocron {tier}"))?;
            if tier != "rare" {
                return Err(refused(400, "not_for_sale"));
            }
            let mut profile = profile("Sol");
            profile.credits = 100;
            Ok(crate::wire::HolocronBuyAnswer {
                bought: crate::wire::HolocronBought {
                    holocron: 42,
                    tier: tier.to_owned(),
                    price: 400,
                    balance: 100,
                },
                profile,
            })
        }
        fn reveal_cards(&mut self, _: &Identity) -> Result<crate::wire::RevealAnswer, HubError> {
            self.record("reveal".to_owned())?;
            Ok(crate::wire::RevealAnswer {
                revealed: vec![crate::wire::Revealed {
                    id: 1_007,
                    card: "sjk_alpha_010".to_owned(),
                    rarity: "legendary".to_owned(),
                    finish: "holo".to_owned(),
                    edition: "alpha".to_owned(),
                    unlock: None,
                }],
                profile: profile("Sol"),
            })
        }
        fn holocrons(&mut self, _: &Identity) -> Result<HolocronState, HubError> {
            // Counted apart from the log, which the other tests read whole.
            *self.holocron_reads.lock().unwrap() += 1;
            if self.fail.load(Ordering::SeqCst) {
                return Err(HubError::Network("down".to_owned()));
            }
            if let Some(refusal) = self.progress_refusals.lock().unwrap().pop_front() {
                return Err(refusal);
            }
            let state = self.progress.lock().unwrap().clone();
            let marks = self.marks.lock().unwrap().clone();
            state
                .map(|state| HolocronState { marks, ..state })
                .ok_or_else(|| refused(404, "not_found"))
        }
        fn set_mark(
            &mut self,
            _: &Identity,
            id: &str,
            mark: crate::wire::Mark,
            on: bool,
        ) -> Result<crate::wire::UnlockMarks, HubError> {
            self.record(format!("mark {id} {} {on}", mark.id()))?;
            let owned = self.owned.lock().unwrap().iter().any(|held| held == id);
            let mut marks = self.marks.lock().unwrap();
            let Some(marks) = marks.as_mut() else {
                return Err(refused(404, "not_found"));
            };
            if on && mark == crate::wire::Mark::Locked && !owned {
                return Err(refused(409, "not_held"));
            }
            marks.set(id, mark, on);
            Ok(marks.clone())
        }
        fn announcements(&mut self, _: &Identity) -> Result<Vec<Announcement>, HubError> {
            self.news_reads.lock().unwrap().0 += 1;
            let news = self.news.lock().unwrap().clone();
            news.map(|(list, _)| list)
                .ok_or_else(|| HubError::Protocol("status 404".to_owned()))
        }
        fn rewards(&mut self, _: &Identity) -> Result<Rewards, HubError> {
            self.news_reads.lock().unwrap().1 += 1;
            let news = self.news.lock().unwrap().clone();
            news.map(|(_, rewards)| rewards)
                .ok_or_else(|| HubError::Protocol("status 404".to_owned()))
        }
        fn claim_reward(
            &mut self,
            _: &Identity,
            id: u64,
        ) -> Result<crate::news::ClaimAnswer, HubError> {
            self.record(format!("claim {id}"))?;
            let mut news = self.news.lock().unwrap();
            let Some((_, rewards)) = news.as_mut() else {
                return Err(HubError::Protocol("status 404".to_owned()));
            };
            if rewards.claimed.iter().any(|reward| reward.id == id) {
                return Err(refused(409, "claimed"));
            }
            let Some(at) = rewards.pending.iter().position(|reward| reward.id == id) else {
                return Err(refused(404, "not_found"));
            };
            let mut reward = rewards.pending.remove(at);
            reward.claimed = Some(1);
            rewards.claimed.insert(0, reward);
            Ok(crate::news::ClaimAnswer {
                profile: Profile {
                    credits: 500,
                    ..profile("Sol")
                },
                rewards: rewards.clone(),
            })
        }
        fn announce(
            &mut self,
            _: &Identity,
            request: &AnnounceRequest,
        ) -> Result<Announcement, HubError> {
            self.record(format!("announce {} {}", request.title, request.version))?;
            if request.title == "refused" {
                return Err(refused(403, "creator_only"));
            }
            Ok(Announcement {
                id: 9,
                at: 1,
                kind: if request.release { "release" } else { "news" }.to_owned(),
                title: request.title.clone(),
                text: request.text.clone(),
                version: request.version.clone(),
                images: std::collections::BTreeMap::new(),
            })
        }
        fn log_requests(&mut self, _: &Identity) -> Result<Vec<LogRequest>, HubError> {
            // Counted apart from the log, which the other tests read whole.
            *self.log_ask_reads.lock().unwrap() += 1;
            if self.fail.load(Ordering::SeqCst) {
                return Err(HubError::Network("down".to_owned()));
            }
            let asks = self.log_asks.lock().unwrap().clone();
            asks.ok_or_else(|| refused(404, "not_found"))
        }
        fn send_logs(&mut self, _: &Identity, id: i64, upload: &LogUpload) -> Result<(), HubError> {
            self.record(format!("logs {id} {} {}", upload.build, upload.logs))?;
            if let Some(asks) = self.log_asks.lock().unwrap().as_mut() {
                asks.retain(|ask| ask.id != id);
            }
            Ok(())
        }
        fn decline_logs(&mut self, _: &Identity, id: i64) -> Result<(), HubError> {
            self.record(format!("decline {id}"))?;
            if let Some(asks) = self.log_asks.lock().unwrap().as_mut() {
                asks.retain(|ask| ask.id != id);
            }
            Ok(())
        }
        fn staff_query(
            &mut self,
            _: &Identity,
            query: &StaffQuery,
        ) -> Result<crate::staff::StaffAnswer, HubError> {
            use crate::logs::LogRequestRow;
            use crate::staff::StaffAnswer;
            self.record(format!("staff {query:?}"))?;
            Ok(match query {
                StaffQuery::RequestLogs { key_id, note } => {
                    StaffAnswer::LogRequest(Box::new(LogRequestRow {
                        id: 12,
                        key_id: key_id.clone(),
                        name: "Friend".into(),
                        note: note.clone(),
                        state: "open".into(),
                        ..LogRequestRow::default()
                    }))
                }
                StaffQuery::Log(id) => StaffAnswer::LogRequest(Box::new(LogRequestRow {
                    id: *id,
                    name: "Friend".into(),
                    state: "sent".into(),
                    logs: "the lines".into(),
                    ..LogRequestRow::default()
                })),
                StaffQuery::Crash(_) => return Err(refused(404, "not_found")),
                _ => StaffAnswer::LogRequests(Vec::new()),
            })
        }
        fn release(&mut self, _: &Identity, server: &str) -> Result<(), HubError> {
            self.record(format!("release {server}"))
        }
        fn set_models(&mut self, _: &Identity, models: &[String]) -> Result<u64, HubError> {
            self.record(format!("models {}", models.join(",")))
                .map(|()| models.len() as u64)
        }
        fn model_check(
            &mut self,
            server: &str,
            slot: u8,
            model: &str,
        ) -> Result<(String, Option<bool>), HubError> {
            self.record(format!("model {server} {slot} {model}"))
                .map(|()| ("^2Sol".to_owned(), Some(model == "kyle")))
        }
        fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
            self.record(format!("presence {server}"))
                .map(|()| self.roster.lock().unwrap().clone())
        }
        fn presence_with_blocks(
            &mut self,
            server: &str,
        ) -> Result<crate::hub::PresenceRead, HubError> {
            self.presence(server)
                .map(|players| crate::hub::PresenceRead {
                    players,
                    blocks: Vec::new(),
                    map_edits: *self.edits_stamp.lock().unwrap(),
                    secrets: *self.secrets_stamp.lock().unwrap(),
                    shows: Vec::new(),
                })
        }
        fn map_edits(&mut self, _: &Identity) -> Result<Vec<crate::map_edits::Entry>, HubError> {
            self.record("map edits".to_owned()).map(|()| Vec::new())
        }
        fn secrets(
            &mut self,
            _: &Identity,
            server: Option<&str>,
        ) -> Result<crate::toy_master::Secrets, HubError> {
            // Logged only where a test follows the Toy Master's stamp: the others
            // match the log line by line.
            if self.secrets_stamp.lock().unwrap().is_none() {
                return Ok(crate::toy_master::Secrets::default());
            }
            self.record(format!("secrets {}", server.unwrap_or("-")))
                .map(|()| crate::toy_master::Secrets::default())
        }
        fn report(&mut self, _: &Identity, report: &BugReport) -> Result<i64, HubError> {
            self.record(format!("report {}", report.text))?;
            self.record(format!("report name {}", report.name))
                .map(|()| 7)
        }
        fn player_report(&mut self, _: &Identity, report: &PlayerReport) -> Result<i64, HubError> {
            Fake::record(
                self,
                format!(
                    "player {} {} {} by {}",
                    report.category.code(),
                    report.slot,
                    report.target_name,
                    report.name
                ),
            )
            .map(|()| 11)
        }
        fn note(&mut self, _: &Identity, note: &WorldNote) -> Result<i64, HubError> {
            Fake::record(self, format!("note {} {}", note.text, note.shader))?;
            Fake::record(self, format!("note name {}", note.name)).map(|()| 9)
        }
        fn note_image(&mut self, _: &Identity, id: i64, jpeg: &[u8]) -> Result<(), HubError> {
            Fake::record(self, format!("image {id} {} bytes", jpeg.len()))
        }
        fn chat(
            &mut self,
            _: &Identity,
            text: &str,
            name: &str,
            channel: crate::HubChannel,
        ) -> Result<u64, HubError> {
            match channel {
                crate::HubChannel::Sjk => Fake::record(self, format!("chat {text} as {name}")),
                channel => Fake::record(self, format!("chat {text} as {name} in {channel:?}")),
            }
            .map(|()| 12)
        }
        fn react(
            &mut self,
            _: &Identity,
            message: u64,
            emoji: &str,
            active: bool,
        ) -> Result<(), HubError> {
            Fake::record(self, format!("react {message} {emoji} {active}"))
        }
        fn chat_ban(&mut self, _: &Identity, key_id: &str) -> Result<ChatBan, HubError> {
            Fake::record(self, format!("ban {key_id}")).map(|()| ChatBan {
                until: 1_000 + crate::ban::BAN_SECS,
                address: true,
            })
        }
        fn emote(
            &mut self,
            _: &Identity,
            server: &str,
            emote: &str,
            sync: Option<u8>,
            quiet: bool,
        ) -> Result<u64, HubError> {
            if (sync.is_some() || quiet) && self.old_hub.load(Ordering::SeqCst) {
                return Err(HubError::Rejected {
                    status: 400,
                    code: "bad_body".to_owned(),
                    message: "unknown field `sync`".to_owned(),
                });
            }
            let with = sync.map_or_else(String::new, |slot| format!(" with {slot}"));
            let quiet = if quiet { " quiet" } else { "" };
            Fake::record(self, format!("emote {emote} on {server}{with}{quiet}")).map(|()| 13)
        }
        fn ping(
            &mut self,
            _: &Identity,
            server: &str,
            pos: [i32; 3],
            scope: &str,
        ) -> Result<u64, HubError> {
            Fake::record(self, format!("ping {pos:?} {scope} on {server}")).map(|()| 14)
        }
        fn toy_event(
            &mut self,
            _: &Identity,
            server: &str,
            toy: &str,
            event: &str,
            target: u8,
        ) -> Result<u64, HubError> {
            Fake::record(self, format!("toy {toy} {event} {target} on {server}")).map(|()| 16)
        }
        fn look(&mut self, _: &Identity, server: &str, look: &Look) -> Result<u64, HubError> {
            let second = look
                .saber2
                .as_ref()
                .map_or_else(String::new, |saber2| format!(" second {saber2:?}"));
            let second = if look.saber_on.is_empty() && look.saber2_on.is_empty() {
                second
            } else {
                format!("{second} on {:?} {:?}", look.saber_on, look.saber2_on)
            };
            let second = match look.tag.as_ref() {
                Some(tag) => format!("{second} tag {}", tag.id),
                None => second,
            };
            let second = if look.finisher.is_empty() {
                second
            } else {
                format!("{second} finisher {}", look.finisher)
            };
            Fake::record(
                self,
                if look.saber_sounds.is_empty() {
                    format!("look {server} {:?} {}{second}", look.saber, look.illuminate)
                } else {
                    format!(
                        "look {server} {:?} {}{second} sounds {}",
                        look.saber, look.illuminate, look.saber_sounds
                    )
                },
            )?;
            if let Some(refusal) = self.look_refusals.lock().unwrap().pop_front() {
                return Err(refusal);
            }
            let owned = self.owned.lock().unwrap();
            let unheld = |skin: &str| !skin.is_empty() && !owned.iter().any(|held| held == skin);
            // As the hub: the first saber's skin is checked first, then the second's,
            // each refusal naming its side.
            let refused = if unheld(&look.saber) {
                Some("not_unlocked")
            } else if look.saber2.as_deref().is_some_and(unheld) {
                Some("saber2_not_unlocked")
            } else {
                None
            };
            if let Some(code) = refused {
                return Err(HubError::Rejected {
                    status: 403,
                    code: code.to_owned(),
                    message: "this key does not hold that unlock".to_owned(),
                });
            }
            Ok(15)
        }
        fn assets(&mut self, _: &Identity) -> Result<Vec<Pack>, HubError> {
            Fake::record(self, "assets".to_owned())?;
            if let Some(refusal) = self.asset_refusals.lock().unwrap().pop_front() {
                return Err(refusal);
            }
            let packs = self.packs.lock().unwrap();
            Ok(packs
                .iter()
                .map(|(name, bytes)| Pack {
                    name: name.clone(),
                    size: bytes.len() as u64,
                    sha256: sha256_hex(bytes),
                })
                .collect())
        }
        fn asset(&mut self, _: &Identity, name: &str) -> Result<Vec<u8>, HubError> {
            Fake::record(self, format!("asset {name}"))?;
            if let Some(bytes) = self.served.lock().unwrap().clone() {
                return Ok(bytes);
            }
            self.packs
                .lock()
                .unwrap()
                .get(name)
                .cloned()
                .ok_or_else(|| refused(404, "not_found"))
        }
        fn set_achievements(
            &mut self,
            _: &Identity,
            progress: &BTreeMap<String, u64>,
        ) -> Result<Vec<Achievement>, HubError> {
            let line: Vec<String> = progress.iter().map(|(id, n)| format!("{id}={n}")).collect();
            Fake::record(self, format!("achievements {}", line.join(",")))?;
            let cap = *self.cap.lock().unwrap();
            let held: Vec<Achievement> = progress
                .iter()
                .map(|(id, &count)| Achievement {
                    id: id.clone(),
                    progress: cap.map_or(count, |cap| count.min(cap)),
                    goal: 100,
                    unlocked: i64::from(cap.map_or(count, |cap| count.min(cap)) >= 100),
                    reward: None,
                    name: None,
                    description: None,
                    group: None,
                })
                .collect();
            *self.held.lock().unwrap() = held.clone();
            Ok(held)
        }
        fn staff(
            &mut self,
            _: &Identity,
            request: &StaffRequest,
        ) -> Result<Vec<Profile>, HubError> {
            Fake::record(self, format!("staff {request:?}"))?;
            Ok(match request {
                StaffRequest::Search(_) => vec![profile("Found"), profile("Other")],
                StaffRequest::Award { key_id, medal, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    medals: vec![crate::wire::Medal {
                        id: medal.clone(),
                        count: 1,
                        awarded: 1,
                        note: String::new(),
                        rank: 1,
                    }],
                    ..profile("Target")
                }],
                StaffRequest::MedalRank {
                    key_id,
                    medal,
                    rank,
                } => vec![Profile {
                    key_id: key_id.clone(),
                    medals: vec![crate::wire::Medal {
                        id: medal.clone(),
                        count: 1,
                        awarded: 1,
                        note: String::new(),
                        rank: *rank,
                    }],
                    ..profile("Target")
                }],
                StaffRequest::Unlock { key_id, unlock, .. } => {
                    self.owned.lock().unwrap().push(unlock.clone());
                    vec![Profile {
                        key_id: key_id.clone(),
                        unlocks: self.unlocks(),
                        ..profile("Target")
                    }]
                }
                StaffRequest::Relock { key_id, unlock } => {
                    self.owned.lock().unwrap().retain(|id| id != unlock);
                    vec![Profile {
                        key_id: key_id.clone(),
                        unlocks: self.unlocks(),
                        ..profile("Target")
                    }]
                }
                StaffRequest::HolocronGive { key_id, tier, note } => vec![Profile {
                    key_id: key_id.clone(),
                    holocron_counts: crate::wire::HolocronCounts {
                        legendary: u32::from(tier == "legendary"),
                        ..Default::default()
                    },
                    holocrons: vec![crate::wire::Holocron {
                        id: 41,
                        tier: tier.clone(),
                        dropped: 1,
                        source: "staff".to_owned(),
                        note: note.clone(),
                        opened: None,
                        traded: None,
                        traded_from_key: None,
                        traded_from: None,
                    }],
                    ..profile("Target")
                }],
                StaffRequest::HolocronRemove { key_id, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    ..profile("Target")
                }],
                StaffRequest::Credits { key_id, delta, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    credits: delta.unsigned_abs(),
                    ..profile("Target")
                }],
                StaffRequest::Shards { key_id, delta, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    shards: delta.unsigned_abs(),
                    ..profile("Target")
                }],
                StaffRequest::Unaward { key_id, .. }
                | StaffRequest::ClearAchievements { key_id, .. }
                | StaffRequest::AvatarRemove { key_id }
                | StaffRequest::AvatarBlock { key_id, .. } => vec![Profile {
                    key_id: key_id.clone(),
                    ..profile("Target")
                }],
                StaffRequest::ChatDelete { .. }
                | StaffRequest::ChatMute { .. }
                | StaffRequest::ChatUnban { .. }
                | StaffRequest::MapBlock { .. } => Vec::new(),
                StaffRequest::Verify {
                    key_id, verified, ..
                } => vec![Profile {
                    key_id: key_id.clone(),
                    verified: *verified,
                    ..profile("Target")
                }],
                StaffRequest::SetStaff { key_id, staff } => vec![Profile {
                    key_id: key_id.clone(),
                    staff: *staff,
                    ..profile("Target")
                }],
                // The kept person, with the merged-away key linked to it.
                StaffRequest::Merge { key_id, from } => vec![Profile {
                    key_id: key_id.clone(),
                    keys: vec![key_id.clone(), from.clone()],
                    ..profile("Target")
                }],
                // The person the key was linked to, without it.
                StaffRequest::Unlink { .. } => vec![Profile {
                    keys: vec!["0123456789abcdef".to_owned()],
                    ..profile("Sol")
                }],
            })
        }
    }

    fn worker(fake: &Fake, now: Instant) -> (Worker, Arc<Mutex<Snapshot>>) {
        let identity = Identity::from_seed([1; 32]);
        let snapshot = Arc::new(Mutex::new(Snapshot::new(&identity)));
        let factory = fake.clone();
        let make: HubFactory = Box::new(move |url| {
            if url.starts_with("https://") {
                Ok(Box::new(factory.clone()))
            } else {
                Err(HubError::Network("bad address".to_owned()))
            }
        });
        (
            Worker::new(
                identity,
                make,
                Arc::clone(&snapshot),
                Arc::new(Mutex::new(StaffState::default())),
                now,
            ),
            snapshot,
        )
    }

    fn on(url: &str) -> Settings {
        Settings {
            enabled: true,
            hub_url: url.to_owned(),
        }
    }

    fn here(slot: u8, name: &str) -> Location {
        Location {
            server: "1.2.3.4:29070".parse().unwrap(),
            slot,
            name: name.to_owned(),
        }
    }

    fn counts(pairs: &[(&str, u64)]) -> BTreeMap<String, u64> {
        pairs.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect()
    }

    fn sent_counts(fake: &Fake) -> Vec<String> {
        fake.log()
            .into_iter()
            .filter(|line| line.starts_with("achievements"))
            .collect()
    }

    fn holocron_reads(fake: &Fake) -> usize {
        *fake.holocron_reads.lock().unwrap()
    }

    fn progress(secs: u64) -> HolocronState {
        HolocronState {
            progress_secs: secs,
            every_secs: 1_800,
            today: 2,
            daily_cap: 8,
            resets_at: Some(1_791_010_800),
            server_time: 1_791_000_000,
            can_open: true,
            min_client: None,
            loot: Vec::new(),
            economy: None,
            kyber: None,
            marks: None,
            catchup: None,
            activity: None,
        }
    }

    fn claims(fake: &Fake) -> Vec<String> {
        fake.log()
            .into_iter()
            .filter(|line| line.starts_with("claim"))
            .collect()
    }

    #[test]
    fn claims_say_active_only_while_the_player_is() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let at = |seconds| t0 + Duration::from_secs(seconds);
        // Telling the service sends nothing by itself; the next claim carries it.
        worker.handle(Command::Active(true), at(10));
        assert_eq!(claims(&fake).len(), 1);
        worker.tick(at(46));
        worker.handle(Command::Active(false), at(50));
        worker.tick(at(91));
        worker.handle(Command::Active(true), at(100));
        worker.tick(at(136));
        assert_eq!(
            claims(&fake),
            [
                "claim 1.2.3.4:29070 3 Sol",
                "claim 1.2.3.4:29070 3 Sol active",
                "claim 1.2.3.4:29070 3 Sol",
                "claim 1.2.3.4:29070 3 Sol active",
            ]
        );
    }

    #[test]
    fn holocron_progress_is_read_after_registering_and_every_few_minutes() {
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(600));
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        assert!(lock(&snapshot).holocrons.is_none());
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(holocron_reads(&fake), 1);
        let read = lock(&snapshot).holocrons.expect("the progress");
        assert_eq!(
            (
                read.progress_secs,
                read.every_secs,
                read.today,
                read.daily_cap
            ),
            (600, 1_800, 2, 8)
        );
        assert_eq!(read.remaining_secs(), 1_200);
        assert!(!read.capped());
        assert_eq!(read.server_time, 1_791_000_000);
        worker.tick(t0 + HOLOCRONS_EVERY - Duration::from_secs(1));
        assert_eq!(holocron_reads(&fake), 1, "not yet");
        *fake.progress.lock().unwrap() = Some(HolocronState {
            today: 8,
            ..progress(1_790)
        });
        worker.tick(t0 + HOLOCRONS_EVERY);
        assert_eq!(holocron_reads(&fake), 2);
        let again = lock(&snapshot).holocrons.expect("the progress");
        assert_eq!(again.remaining_secs(), 10);
        assert!(again.capped(), "8 of 8 today");
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    /// The game menu opening reads the profile, the holocron progress and the trades at
    /// once, at most every [`MENU_SOON`], and sends changed achievement counts, at most
    /// every [`MENU_COUNTS_SOON`].
    #[test]
    fn the_game_menu_opening_reads_its_pages_soon_with_a_cooldown() {
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(0));
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 3)])), t0);
        worker.tick(t0);
        let trades = |fake: &Fake| *fake.trade_reads.lock().unwrap();
        let reads = |fake: &Fake| {
            (
                profile_reads(fake),
                holocron_reads(fake),
                trades(fake),
                sent_counts(fake).len(),
            )
        };
        let start = reads(&fake);
        let at = |seconds| t0 + Duration::from_secs(seconds);
        worker.handle(Command::Achievements(counts(&[("kills_100", 4)])), at(5));
        worker.handle(Command::MenuOpened, at(5));
        worker.tick(at(5));
        assert_eq!(
            reads(&fake),
            (start.0 + 1, start.1 + 1, start.2 + 1, start.3 + 1)
        );
        // Opened again within the cooldown: nothing more.
        worker.handle(Command::Achievements(counts(&[("kills_100", 5)])), at(10));
        worker.handle(Command::MenuOpened, at(10));
        worker.tick(at(10));
        assert_eq!(
            reads(&fake),
            (start.0 + 1, start.1 + 1, start.2 + 1, start.3 + 1)
        );
        // After it the pages are read again; the counts wait for their half minute.
        worker.handle(Command::MenuOpened, at(20));
        worker.tick(at(20));
        assert_eq!(
            reads(&fake),
            (start.0 + 2, start.1 + 2, start.2 + 2, start.3 + 1)
        );
        worker.handle(Command::MenuOpened, at(35));
        worker.tick(at(35));
        assert_eq!(
            reads(&fake),
            (start.0 + 3, start.1 + 3, start.2 + 3, start.3 + 2)
        );
        assert_eq!(
            sent_counts(&fake).last().unwrap(),
            "achievements kills_100=5"
        );
    }

    #[test]
    fn a_hub_without_holocrons_is_asked_again_in_an_hour_and_the_status_stays() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(holocron_reads(&fake), 1);
        assert!(lock(&snapshot).holocrons.is_none());
        assert_eq!(lock(&snapshot).status, Status::Online);
        worker.tick(t0 + HOLOCRONS_AGAIN - Duration::from_secs(1));
        assert_eq!(holocron_reads(&fake), 1);
        // Asking for it (a page opening) reads it now, once the half minute is out.
        worker.handle(Command::RefreshHolocrons, t0 + Duration::from_secs(10));
        worker.tick(t0 + Duration::from_secs(10));
        assert_eq!(holocron_reads(&fake), 2);
        *fake.progress.lock().unwrap() = Some(progress(0));
        worker.tick(t0 + Duration::from_secs(10) + HOLOCRONS_AGAIN);
        assert!(lock(&snapshot).holocrons.is_some());
    }

    #[test]
    fn a_failing_hub_is_asked_again_soon_without_a_failed_status_from_the_reading() {
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(0));
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(holocron_reads(&fake), 1);
        fake.fail.store(true, Ordering::SeqCst);
        worker.tick(t0 + HOLOCRONS_EVERY);
        let reads = holocron_reads(&fake);
        assert_eq!(reads, 2);
        worker.tick(t0 + HOLOCRONS_EVERY + Duration::from_secs(1));
        assert_eq!(holocron_reads(&fake), reads, "waits for the retry");
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + HOLOCRONS_EVERY + RETRY_MAX);
        assert_eq!(holocron_reads(&fake), reads + 1);
        assert!(lock(&snapshot).holocrons.is_some());
    }

    #[test]
    fn a_refused_reading_of_a_hub_with_holocrons_is_tried_again_soon() {
        // Too many requests or a hub restarting is no hub without holocrons: the
        // progress is read again after the short retry, not in an hour, and the Esc
        // menu's reading can bring it forward again.
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(0));
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(holocron_reads(&fake), 1);
        for refusal in [refused(429, "rate_limited"), refused(502, "error")] {
            fake.progress_refusals.lock().unwrap().push_back(refusal);
        }
        worker.tick(t0 + HOLOCRONS_EVERY);
        assert_eq!(holocron_reads(&fake), 2);
        worker.tick(t0 + HOLOCRONS_EVERY + RETRY_MAX);
        assert_eq!(holocron_reads(&fake), 3);
        *fake.progress.lock().unwrap() = Some(progress(1_500));
        let menu = t0 + HOLOCRONS_EVERY + RETRY_MAX + Duration::from_secs(20);
        worker.handle(Command::MenuOpened, menu);
        worker.tick(menu);
        assert_eq!(holocron_reads(&fake), 4);
        assert_eq!(lock(&snapshot).holocrons.unwrap().remaining_secs(), 300);
    }

    #[test]
    fn a_drop_of_the_own_key_reads_the_profile_and_progress_soon_at_most_twice_a_minute() {
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(0));
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!((profile_reads(&fake), holocron_reads(&fake)), (0, 1));
        let at = |seconds| t0 + Duration::from_secs(seconds);
        worker.handle(Command::OwnDrop, at(5));
        worker.tick(at(5));
        assert_eq!((profile_reads(&fake), holocron_reads(&fake)), (1, 2));
        // Another within the half minute waits for it.
        worker.handle(Command::OwnDrop, at(15));
        worker.tick(at(15));
        worker.tick(at(34));
        assert_eq!((profile_reads(&fake), holocron_reads(&fake)), (1, 2));
        worker.tick(at(35));
        assert_eq!((profile_reads(&fake), holocron_reads(&fake)), (2, 3));
        // Asking for the progress alone is throttled the same way.
        worker.handle(Command::RefreshHolocrons, at(40));
        worker.tick(at(40));
        assert_eq!(holocron_reads(&fake), 3);
        worker.tick(at(65));
        assert_eq!(holocron_reads(&fake), 4);
        assert_eq!(
            profile_reads(&fake),
            2,
            "a refresh is not a profile reading"
        );
    }

    #[test]
    fn log_requests_are_listed_answered_once_and_older_hubs_left_alone() {
        let fake = Fake::default();
        let ask = |id| LogRequest {
            id,
            from: "Sol".into(),
            note: "the freeze".into(),
            created: 1,
        };
        *fake.log_asks.lock().unwrap() = Some(vec![ask(3), ask(4)]);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(*fake.log_ask_reads.lock().unwrap(), 1);
        assert_eq!(lock(&snapshot).log_requests, vec![ask(3), ask(4)]);
        // Sent: it leaves the list, and the hub hears the logs fitted.
        worker.handle(
            Command::AnswerLogs(
                3,
                Some(LogUpload {
                    logs: "line\r\nnext\u{0}".into(),
                    build: "2026.1011.2".into(),
                    ..LogUpload::default()
                }),
            ),
            t0,
        );
        assert!(
            fake.log()
                .contains(&"logs 3 2026.1011.2 line\nnext".to_owned())
        );
        assert_eq!(lock(&snapshot).log_requests, vec![ask(4)]);
        assert_eq!(
            lock(&snapshot).log_answer.clone().unwrap().message,
            "logs sent"
        );
        // Declined, then a listing that still has it (read before the answer) keeps it out.
        worker.handle(Command::AnswerLogs(4, None), t0);
        assert!(fake.log().contains(&"decline 4".to_owned()));
        fake.log_asks.lock().unwrap().as_mut().unwrap().push(ask(4));
        worker.tick(t0 + LOG_REQUESTS_EVERY);
        assert!(lock(&snapshot).log_requests.is_empty());
        // A new one shows at the next look.
        fake.log_asks.lock().unwrap().as_mut().unwrap().push(ask(5));
        worker.tick(t0 + LOG_REQUESTS_EVERY * 2);
        assert_eq!(lock(&snapshot).log_requests, vec![ask(5)]);
        // A hub from before log requests is asked again hours later only.
        *fake.log_asks.lock().unwrap() = None;
        let t1 = t0 + LOG_REQUESTS_EVERY * 3;
        worker.tick(t1);
        let reads = *fake.log_ask_reads.lock().unwrap();
        worker.tick(t1 + LOG_REQUESTS_EVERY);
        assert_eq!(*fake.log_ask_reads.lock().unwrap(), reads);
        worker.tick(t1 + LOG_REQUESTS_AGAIN);
        assert_eq!(*fake.log_ask_reads.lock().unwrap(), reads + 1);
    }

    #[test]
    fn an_answer_the_hub_could_not_hear_comes_back_to_answer_again() {
        let fake = Fake::default();
        let ask = LogRequest {
            id: 8,
            from: "Sol".into(),
            note: String::new(),
            created: 1,
        };
        *fake.log_asks.lock().unwrap() = Some(vec![ask.clone()]);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::AnswerLogs(8, None), t0);
        assert!(!lock(&snapshot).log_answer.clone().unwrap().sent);
        assert!(lock(&snapshot).log_requests.is_empty());
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + RETRY_MAX + LOG_REQUESTS_EVERY);
        assert_eq!(lock(&snapshot).log_requests, vec![ask]);
    }

    #[test]
    fn staff_queries_go_for_staff_only_and_opened_logs_are_handed_over() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        worker.handle(
            Command::StaffQuery(StaffQuery::RequestLogs {
                key_id: "0123456789abcdef".into(),
                note: "your freeze".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Asked Friend for their logs");
        assert_eq!(staff.lock().unwrap().log_requests[0].id, 12);
        worker.handle(Command::StaffQuery(StaffQuery::Log(12)), t0);
        let opened = staff.lock().unwrap().opened.take().unwrap();
        assert_eq!(opened.text, "the lines");
        assert_eq!(staff.lock().unwrap().log_requests[0].state, "sent");
        worker.handle(Command::StaffQuery(StaffQuery::Crash(1)), t0);
        assert!(staff.lock().unwrap().failed);
        assert_eq!(staff.lock().unwrap().message, "The hub no longer has it");
        // A plain key is refused here, before the hub hears it.
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        let before = fake.log().len();
        worker.handle(Command::StaffQuery(StaffQuery::CrashGroups), t0);
        assert_eq!(fake.log().len(), before);
        assert_eq!(staff.lock().unwrap().message, "only SJK staff can do this");
    }

    #[test]
    fn staff_give_and_remove_a_holocron_and_the_answer_replaces_the_profile() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::HolocronGive {
                key_id: me.clone(),
                tier: "legendary".into(),
                note: "For the fog bug".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Gave a legendary holocron");
        let own = lock(&snapshot).me.clone().unwrap();
        assert_eq!(own.holocron_counts.legendary, 1);
        assert_eq!(own.holocrons[0].note, "For the fog bug");
        assert_eq!(staff.lock().unwrap().players[0].holocrons[0].id, 41);
        worker.handle(
            Command::Staff(StaffRequest::HolocronRemove { key_id: me, id: 41 }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Took back holocron #41");
        assert!(lock(&snapshot).me.clone().unwrap().holocrons.is_empty());
        // Only staff may: a plain key is refused here, before the hub hears it.
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        let before = fake.log().len();
        worker.handle(
            Command::Staff(StaffRequest::HolocronGive {
                key_id: "0123456789abcdef".into(),
                tier: "rare".into(),
                note: String::new(),
            }),
            t0,
        );
        assert_eq!(fake.log().len(), before);
        assert!(staff.lock().unwrap().failed);
    }

    fn chat_outcome(worker: &Worker) -> ReportOutcome {
        lock_chat(&worker.chat).outcome.clone().unwrap()
    }

    #[test]
    fn a_hub_without_pictures_is_said_in_words() {
        let older = older_hub(HubError::Protocol("status 404, not JSON".to_owned()));
        assert!(
            matches!(&older, HubError::Rejected { message, .. } if message.contains("does not take pictures")),
            "{older:?}"
        );
        assert!(matches!(
            older_hub(HubError::Protocol("status 405, not JSON".to_owned())),
            HubError::Rejected { .. }
        ));
        // Other answers keep what they say.
        assert!(matches!(
            older_hub(HubError::Protocol("status 500, not JSON".to_owned())),
            HubError::Protocol(_)
        ));
        assert!(matches!(
            older_hub(HubError::Network("down".to_owned())),
            HubError::Network(_)
        ));
    }

    /// Shattering, crafting, the shop and buying wait for the registration; a shatter
    /// keeps the shards and the profile, a refusal is said in the player's words, a
    /// purchase marks the offer bought and held and moves the shop's credits, and an offer
    /// gone from the shop reads the shop again.
    #[test]
    fn trades_with_players_are_read_asked_and_stepped() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // Read once registered: the hub lets everyone trade, and nothing runs.
        let read = lock(&snapshot).trades.clone().unwrap();
        assert!(read.open(false) && read.live().is_none());
        assert_eq!(read.hub_now(t0 + Duration::from_secs(4)), 1_791_000_004);
        let reads = |fake: &Fake| *fake.trade_reads.lock().unwrap();
        assert_eq!(reads(&fake), 1);
        // A refusal reaches the player in their words, and the list is read again.
        worker.handle(Command::TradeAsk("fox".to_owned()), t0);
        let busy = lock(&snapshot).trades.clone().unwrap();
        assert_eq!(busy.problem, "That player is in a trade already.");
        assert_eq!((busy.steps, reads(&fake)), (1, 2));
        let own = format!("lookup {}", Identity::from_seed([1; 32]).key_id());
        let lookups = |fake: &Fake| fake.log().iter().filter(|line| **line == own).count();
        let before = lookups(&fake);
        worker.handle(Command::TradeAsk("nib".to_owned()), t0);
        // A trade started: the own profile (the holocrons to offer) is read at once.
        worker.tick(t0);
        assert_eq!(lookups(&fake), before + 1);
        let asked = lock(&snapshot).trades.clone().unwrap();
        assert!(asked.problem.is_empty());
        assert_eq!(
            asked.live().map(|trade| trade.state.as_str()),
            Some("asked")
        );
        assert_eq!(
            asked.seconds_until(1_791_000_030, t0 + Duration::from_secs(10)),
            20
        );
        // While one runs it is read every few seconds; the feed's word reads it at once.
        worker.tick(t0 + TRADES_LIVE);
        assert_eq!(reads(&fake), 4);
        worker.handle(Command::ReadTrades, t0 + TRADES_LIVE);
        worker.tick(t0 + TRADES_LIVE);
        assert_eq!(reads(&fake), 5);
        worker.handle(
            Command::TradeStep(3, crate::trades::TradeStep::Accept),
            t0 + TRADES_LIVE,
        );
        assert!(
            lock(&snapshot)
                .trades
                .clone()
                .unwrap()
                .live()
                .unwrap()
                .is_open()
        );
        // The swap's profile becomes the player's.
        worker.handle(
            Command::TradeStep(3, crate::trades::TradeStep::Confirm { rev: 2 }),
            t0 + TRADES_LIVE,
        );
        assert_eq!(lock(&snapshot).me.as_ref().map(|me| me.shards), Some(77));
    }

    /// The player who confirms first gets no profile from the hub: the swap happens at
    /// the other player's confirmation. The list that first shows the trade done reads
    /// the profile at once, so the collection does not wait for the ten-minute read
    /// (Sol, in game, 25/10/2026); later lists of the same trade read nothing more.
    #[test]
    fn a_swap_finished_by_the_other_player_reads_the_profile_at_once() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::TradeAsk("nib".to_owned()), t0);
        worker.handle(Command::TradeStep(3, crate::trades::TradeStep::Accept), t0);
        worker.tick(t0);
        let own = format!("lookup {}", Identity::from_seed([1; 32]).key_id());
        let lookups = |fake: &Fake| fake.log().iter().filter(|line| **line == own).count();
        let t1 = t0 + Duration::from_secs(60);
        worker.tick(t1);
        let before = lookups(&fake);
        let holocron_reads = |fake: &Fake| *fake.holocron_reads.lock().unwrap();
        let progress_before = holocron_reads(&fake);
        assert!(worker.due_profile > t1, "no profile read is due yet");
        // Nib confirms last; the feed tells of the change.
        fake.partner_swapped.store(true, Ordering::SeqCst);
        worker.handle(Command::ReadTrades, t1);
        assert_eq!(
            worker.tick(t1),
            Duration::ZERO,
            "the worker goes on at once"
        );
        worker.tick(t1);
        assert_eq!(
            lookups(&fake),
            before + 1,
            "the swap reads the profile at once"
        );
        assert!(
            holocron_reads(&fake) > progress_before,
            "the holocron progress too"
        );
        // The same done trade in later lists reads nothing more.
        worker.handle(Command::ReadTrades, t1);
        worker.tick(t1);
        worker.tick(t1);
        assert_eq!(lookups(&fake), before + 1);
    }

    /// Trading off on the hub: the list is not read again every ten minutes (Sol's
    /// review, 24/10/2026), but trading switched on reaches a running client within
    /// [`TRADES_OFF`] (Sol's review, 25/10/2026).
    #[test]
    fn trading_off_slows_the_reads() {
        let fake = Fake::default();
        fake.trading_off.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let reads = |fake: &Fake| *fake.trade_reads.lock().unwrap();
        assert_eq!(reads(&fake), 1);
        assert!(!lock(&snapshot).trades.clone().unwrap().open(true));
        for minutes in [10, 20] {
            worker.tick(t0 + Duration::from_secs(minutes * 60));
        }
        assert_eq!(reads(&fake), 1, "not read every ten minutes while off");
        // The hub turns trading on: the client sees it at the next slow read, no restart.
        fake.trading_off.store(false, Ordering::SeqCst);
        worker.tick(t0 + TRADES_OFF);
        assert_eq!(reads(&fake), 2);
        assert!(lock(&snapshot).trades.clone().unwrap().open(true));
        // The feed's trades event still reads it at once.
        worker.handle(Command::ReadTrades, t0 + TRADES_OFF);
        worker.tick(t0 + TRADES_OFF);
        assert_eq!(reads(&fake), 3);
    }

    /// Collection marks (29/10/2026): an older hub sends none, so nothing changes; a hub
    /// that keeps them sends them with the progress, a change shows at once and the hub's
    /// answer replaces it, and a refusal says why and reads the hub's marks back.
    #[test]
    fn collection_marks_come_with_the_progress_and_change_at_once() {
        use crate::wire::{Mark, UnlockMarks};
        let fake = Fake::default();
        *fake.progress.lock().unwrap() = Some(progress(0));
        fake.owned.lock().unwrap().push("saber_frost".to_owned());
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).marks, None, "an older hub keeps none");
        assert!(
            !mark_at_once(&mut lock(&snapshot), "saber_frost", Mark::Favourite, true),
            "nothing to ask an older hub"
        );
        assert!(fake.log().iter().all(|line| !line.starts_with("mark")));
        // The hub keeps marks: read with the progress.
        *fake.marks.lock().unwrap() = Some(UnlockMarks {
            favourite: vec!["saber_storm".to_owned()],
            locked: Vec::new(),
        });
        let t1 = t0 + HOLOCRONS_EVERY;
        worker.tick(t1);
        let revision = lock(&snapshot).marks_revision;
        assert!(
            lock(&snapshot)
                .marks
                .as_ref()
                .unwrap()
                .favourite("saber_storm")
        );
        // A lock shows at once, then the hub's answer stands.
        assert!(mark_at_once(
            &mut lock(&snapshot),
            "saber_frost",
            Mark::Locked,
            true
        ));
        assert!(
            lock(&snapshot)
                .marks
                .as_ref()
                .unwrap()
                .locked("saber_frost")
        );
        assert!(lock(&snapshot).marks_revision > revision);
        assert!(
            !mark_at_once(&mut lock(&snapshot), "saber_frost", Mark::Locked, true),
            "set already: nothing to ask"
        );
        worker.handle(
            Command::Mark("saber_frost".to_owned(), Mark::Locked, true),
            t1,
        );
        assert!(
            fake.log()
                .contains(&"mark saber_frost locked true".to_owned())
        );
        assert_eq!(
            fake.marks.lock().unwrap().clone().unwrap().locked,
            ["saber_frost"]
        );
        assert_eq!(lock(&snapshot).mark_refusal, None);
        // The hub refuses a lock on a shader not held: why, and its marks again soon.
        assert!(mark_at_once(
            &mut lock(&snapshot),
            "saber_void",
            Mark::Locked,
            true
        ));
        let reads = holocron_reads(&fake);
        worker.handle(
            Command::Mark("saber_void".to_owned(), Mark::Locked, true),
            t1,
        );
        let refusal = lock(&snapshot).mark_refusal.clone().unwrap();
        assert_eq!(
            refusal,
            (1, "Only a shader you hold can be locked.".to_owned())
        );
        worker.tick(t1 + HOLOCRONS_SOON);
        assert!(holocron_reads(&fake) > reads, "read again");
        let marks = lock(&snapshot).marks.clone().unwrap();
        assert!(!marks.locked("saber_void") && marks.locked("saber_frost"));
    }

    /// The history (31/10/2026) waits for the registration, its pages follow on, and a
    /// read of the newest starts the list again.
    #[test]
    fn the_history_is_read_page_by_page() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::ReadHistory(None), t0);
        let first = lock(&snapshot).history.clone().unwrap();
        assert_eq!(first.serial, 1);
        assert!(first.message.contains("cl_identity 1"));
        assert!(
            fake.log().is_empty(),
            "nothing goes before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::ReadHistory(None), t0);
        worker.handle(Command::ReadHistory(Some(20)), t0);
        let read = lock(&snapshot).history.clone().unwrap();
        assert_eq!((read.serial, read.lines.len(), read.more), (3, 3, false));
        assert!(read.message.is_empty());
        worker.handle(Command::ReadHistory(None), t0);
        let again = lock(&snapshot).history.clone().unwrap();
        assert_eq!((again.serial, again.lines.len(), again.more), (4, 2, true));
        assert!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("history"))
                .eq(["history None", "history Some(20)", "history None"].iter())
        );
    }

    #[test]
    fn trades_wait_for_the_registration_and_the_shop_follows_them() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Shatter("saber_tidal".to_owned()), t0);
        let first = lock(&snapshot).trade.clone().unwrap();
        assert!(!first.done() && first.message.contains("cl_identity 1"));
        assert!(
            fake.log().is_empty(),
            "nothing goes before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Shatter("saber_tidal".to_owned()), t0);
        let shattered = lock(&snapshot).trade.clone().unwrap();
        assert_eq!(
            (shattered.serial, shattered.trade, shattered.amount),
            (2, Trade::Shatter, 5)
        );
        assert!(shattered.done());
        assert_eq!(lock(&snapshot).me.as_ref().map(|me| me.shards), Some(5));
        worker.handle(Command::Shatter("saber_last".to_owned()), t0);
        assert_eq!(
            lock(&snapshot).trade.clone().unwrap().message,
            "That is your last one: only duplicates can be shattered."
        );
        // Asked for, the last copy goes too (21/10/2026); every duplicate at once says
        // how many copies went and what they gave.
        worker.handle(Command::ShatterLast("saber_last".to_owned()), t0);
        let last = lock(&snapshot).trade.clone().unwrap();
        assert!(last.done());
        assert_eq!(
            (last.trade, last.copies, last.amount),
            (Trade::ShatterLast, 1, 5)
        );
        worker.handle(Command::ShatterDuplicates, t0);
        let all = lock(&snapshot).trade.clone().unwrap();
        assert_eq!(
            (all.trade, all.id.as_str(), all.copies, all.amount),
            (Trade::ShatterAll, "", 3, 15)
        );
        assert_eq!(lock(&snapshot).me.as_ref().map(|me| me.shards), Some(165));
        worker.handle(Command::Craft("saber_circuit".to_owned()), t0);
        let craft = lock(&snapshot).trade.clone().unwrap();
        assert_eq!(
            (craft.trade, craft.message.as_str()),
            (Trade::Craft, "Not enough kyber shards.")
        );
        worker.handle(Command::ReadShop, t0);
        let shop = lock(&snapshot).shop.clone().unwrap();
        assert_eq!((shop.serial, shop.day.offers.len()), (1, 1));
        assert_eq!(shop.seconds_left(t0), 3_600);
        assert_eq!(shop.seconds_left(t0 + Duration::from_secs(4_000)), 0);
        worker.handle(Command::Buy("saber_aurora".to_owned()), t0);
        let bought = lock(&snapshot).trade.clone().unwrap();
        assert_eq!((bought.trade, bought.amount), (Trade::Buy, 30));
        let shop = lock(&snapshot).shop.clone().unwrap();
        assert!(shop.day.offers[0].bought && shop.day.offers[0].owned);
        assert_eq!((shop.day.credits, shop.day.shards), (300, 15));
        // A holocron for credits: the shop's credits follow, no offer changes.
        worker.handle(Command::BuyHolocron("rare".to_owned()), t0);
        let holocron = lock(&snapshot).trade.clone().unwrap();
        assert_eq!(
            (holocron.trade, holocron.id.as_str(), holocron.amount),
            (Trade::Holocron, "rare", 400)
        );
        assert_eq!(lock(&snapshot).shop.clone().unwrap().day.credits, 100);
        worker.handle(Command::BuyHolocron("mythical".to_owned()), t0);
        assert_eq!(
            lock(&snapshot).trade.clone().unwrap().message,
            "That holocron is not for sale."
        );
        worker.handle(Command::Buy("saber_gone".to_owned()), t0);
        assert_eq!(
            lock(&snapshot).trade.clone().unwrap().message,
            "That shader is not in the shop any more: the offers changed."
        );
        assert_eq!(
            lock(&snapshot).shop.clone().unwrap().serial,
            2,
            "read again"
        );
        assert_eq!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("shatter")
                    || line.starts_with("craft")
                    || line.starts_with("buy")
                    || *line == "shop")
                .count(),
            11
        );
        assert!(
            fake.log()
                .iter()
                .any(|line| line == "shatter saber_last last")
        );
    }

    /// Revealing waits for the registration, then keeps what the copies became and the
    /// profile after; each reveal counts up.
    #[test]
    fn sealed_cards_reveal_once_registered() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::RevealCards, t0);
        let first = lock(&snapshot).reveal.clone().unwrap();
        assert!(first.revealed.is_empty() && first.message.contains("cl_identity 1"));
        assert!(
            fake.log().is_empty(),
            "nothing goes before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::RevealCards, t0);
        let revealed = lock(&snapshot).reveal.clone().unwrap();
        assert_eq!(revealed.serial, 2);
        assert!(revealed.message.is_empty());
        assert_eq!(revealed.revealed[0].card, "sjk_alpha_010");
        assert!(fake.log().iter().any(|line| line == "reveal"));
    }

    /// Opening waits for the registration, sends at most twenty ids, keeps what they
    /// brought and the profile after; a refusal says why in words and reads the profile
    /// again soon.
    #[test]
    fn holocrons_open_once_registered_and_what_they_brought_is_kept() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::OpenHolocrons(vec![7]), t0);
        let first = lock(&snapshot).opening.clone().unwrap();
        assert!(first.opened.is_empty() && first.message.contains("cl_identity 1"));
        assert!(
            fake.log().is_empty(),
            "nothing goes before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let ids: Vec<u64> = (1..=25).collect();
        worker.handle(Command::OpenHolocrons(ids), t0);
        let opened = lock(&snapshot).opening.clone().unwrap();
        assert_eq!(opened.serial, 2);
        assert!(opened.message.is_empty());
        assert_eq!(opened.opened.len(), crate::wire::OPEN_MAX, "twenty at most");
        assert_eq!(opened.opened[0].card.as_ref().unwrap().id, 1_001);
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().credits, 800);
        assert!(
            fake.log()
                .iter()
                .any(|line| line == "open 1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20")
        );
        assert!(
            worker.due_holocrons > t0,
            "holocrons leave the progress (and the kyber pity) alone"
        );
        worker.handle(Command::OpenHolocrons(vec![77]), t0);
        assert!(
            worker.due_holocrons <= t0,
            "a kyber crystal moved the pity counter: the progress is read again at once"
        );
        worker.handle(Command::OpenHolocrons(vec![99]), t0);
        let refused = lock(&snapshot).opening.clone().unwrap();
        assert_eq!(refused.serial, 4);
        assert!(refused.opened.is_empty());
        assert_eq!(refused.message, "That holocron was already opened.");
        assert!(
            worker.due_profile <= t0 + PROFILE_SOON,
            "the profile is read again soon"
        );
        worker.handle(Command::OpenHolocrons(Vec::new()), t0);
        assert_eq!(
            lock(&snapshot).opening.clone().unwrap().message,
            "there is no holocron to open"
        );
    }

    #[test]
    fn a_picture_is_sent_once_registered_and_its_profile_kept() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::SetAvatar(vec![1; 300]), t0);
        let first = lock(&snapshot).avatar.clone().unwrap();
        assert!(!first.sent && first.message.contains("cl_identity 1"));
        assert!(
            fake.log().is_empty(),
            "nothing goes before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetAvatar(vec![1; 300]), t0);
        let saved = lock(&snapshot).avatar.clone().unwrap();
        assert_eq!(
            (saved.serial, saved.sent, saved.message.as_str()),
            (2, true, "saved")
        );
        assert_eq!(
            lock(&snapshot).me.as_ref().unwrap().avatar,
            "0123456789abcdef"
        );
        // Larger than the hub takes: refused here, nothing sent.
        worker.handle(
            Command::SetAvatar(vec![0; crate::avatar::UPLOAD_MAX + 1]),
            t0,
        );
        let big = lock(&snapshot).avatar.clone().unwrap();
        assert!(!big.sent && big.serial == 3);
        worker.handle(Command::RemoveAvatar, t0);
        let removed = lock(&snapshot).avatar.clone().unwrap();
        assert_eq!((removed.sent, removed.message.as_str()), (true, "removed"));
        assert!(lock(&snapshot).me.as_ref().unwrap().avatar.is_empty());
        let sent: Vec<String> = fake
            .log()
            .into_iter()
            .filter(|line| line.starts_with("avatar"))
            .collect();
        assert_eq!(sent, ["avatar 300 bytes", "avatar removed"]);
        // A refusal is the hub's message.
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::SetAvatar(vec![1; 10]), t0);
        let failed = lock(&snapshot).avatar.clone().unwrap();
        assert!(!failed.sent && failed.message.contains("down"));
    }

    #[test]
    fn chat_needs_the_rules_and_a_registration() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Chat("hello".into()), t0);
        let first = chat_outcome(&worker);
        assert!(!first.sent && first.message.contains("cl_identity 1"));
        worker.handle(Command::Name("^2Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // The rules are checked here first; nothing goes.
        worker.handle(Command::Chat("hi a\u{200D}b".into()), t0);
        let refused = chat_outcome(&worker);
        assert_eq!(
            (refused.serial, refused.sent, refused.message.as_str()),
            (2, false, crate::chat::ChatError::Characters.message())
        );
        worker.handle(Command::Chat("  gg   wp ".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "chat gg wp as ^2Sol");
        let sent = chat_outcome(&worker);
        assert!(sent.sent);
        assert_eq!(sent.serial, 3);
        // Neither verified nor staff: no links, and the next message waits 30 seconds.
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        worker.handle(Command::Chat("see discord.gg/abc".into()), t0);
        let link = chat_outcome(&worker);
        assert!(
            !link.sent && link.message.contains("links and GIFs"),
            "{}",
            link.message
        );
        let later = t0 + Duration::from_secs(12);
        worker.handle(Command::Chat("again".into()), later);
        let wait = chat_outcome(&worker);
        assert!(
            !wait.sent && wait.message.ends_with("wait 18 s"),
            "{}",
            wait.message
        );
        worker.handle(Command::Chat("again".into()), t0 + Duration::from_secs(30));
        assert!(chat_outcome(&worker).sent);
        assert_eq!(fake.log().last().unwrap(), "chat again as ^2Sol");
    }

    #[test]
    fn a_clan_chat_message_goes_to_its_channel_and_the_joined_ones_are_read() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Name("^2Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(
            Command::ClanChat("  gg  ".into(), crate::HubChannel::Sith),
            t0,
        );
        assert_eq!(fake.log().last().unwrap(), "chat gg as ^2Sol in Sith");
        assert!(chat_outcome(&worker).sent);
        worker.handle(Command::SetChat(true), t0);
        worker.handle(Command::SetClanChats(vec![crate::HubChannel::Jof]), t0);
        worker.tick(t0);
        assert_eq!(lock_feed(&worker.feed).channels, [crate::HubChannel::Jof]);
    }

    #[test]
    fn reactions_use_the_worker_but_not_text_sending_or_its_outcome() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(
            Command::React {
                message: 7,
                emoji: "heart".into(),
                active: true,
            },
            t0,
        );
        assert_eq!(fake.log().last().unwrap(), "react 7 heart true");
        assert!(lock_chat(&worker.chat).reaction.as_ref().unwrap().sent);
        assert!(lock_chat(&worker.chat).outcome.is_none());
        let before = fake.log().len();
        worker.handle(
            Command::React {
                message: 7,
                emoji: "invalid".into(),
                active: true,
            },
            t0,
        );
        assert_eq!(fake.log().len(), before);
        assert!(!lock_chat(&worker.chat).reaction.as_ref().unwrap().sent);
    }

    #[test]
    fn emotes_need_a_server_and_a_well_formed_id() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Emote("wave".into(), None, false), t0);
        let nowhere = chat_outcome(&worker);
        assert!(!nowhere.sent && nowhere.message.contains("server"));
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Emote("Wave!".into(), None, false), t0);
        assert!(!chat_outcome(&worker).sent);
        assert!(!fake.log().iter().any(|line| line.starts_with("emote")));
        worker.handle(Command::Emote("wave".into(), None, false), t0);
        assert_eq!(fake.log().last().unwrap(), "emote wave on 1.2.3.4:29070");
        assert!(chat_outcome(&worker).sent);
        worker.handle(Command::Emote("wave".into(), Some(5), false), t0);
        assert_eq!(
            fake.log().last().unwrap(),
            "emote wave on 1.2.3.4:29070 with 5"
        );
        worker.handle(Command::Emote("dance".into(), None, true), t0);
        assert_eq!(
            fake.log().last().unwrap(),
            "emote dance on 1.2.3.4:29070 quiet"
        );
        // A hub that does not know `sync` or `quiet` still gets the emote, out of
        // step or with its sound.
        fake.old_hub.store(true, Ordering::SeqCst);
        worker.handle(Command::Emote("bow".into(), Some(5), false), t0);
        assert_eq!(fake.log().last().unwrap(), "emote bow on 1.2.3.4:29070");
        assert!(chat_outcome(&worker).sent);
        worker.handle(Command::Emote("dance".into(), None, true), t0);
        assert_eq!(fake.log().last().unwrap(), "emote dance on 1.2.3.4:29070");
    }

    #[test]
    fn pings_need_a_server() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Ping([1, 2, 3], "all".into()), t0);
        let outcome = || crate::feed::lock(&worker.ping_outcome).clone().unwrap();
        let nowhere = outcome();
        assert!(!nowhere.sent && nowhere.message.contains("server"));
        assert!(!fake.log().iter().any(|line| line.starts_with("ping")));
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Ping([1, -2, 3], "red".into()), t0);
        assert_eq!(
            fake.log().last().unwrap(),
            "ping [1, -2, 3] red on 1.2.3.4:29070"
        );
        let sent = crate::feed::lock(&worker.ping_outcome).clone().unwrap();
        assert!(sent.sent && sent.serial == nowhere.serial + 1);
    }

    #[test]
    fn toy_events_go_from_the_claimed_slot_only_on_a_server() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let spotted = || Command::Toy("cardboard_box".into(), "spotted".into(), 5);
        worker.handle(spotted(), t0);
        assert!(!fake.log().iter().any(|line| line.starts_with("toy")));
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(spotted(), t0);
        assert_eq!(
            fake.log().last().unwrap(),
            "toy cardboard_box spotted 5 on 1.2.3.4:29070"
        );
    }

    #[test]
    fn the_feed_runs_when_registered_with_chat_on_or_on_a_server() {
        let fake = Fake::default();
        fake.fail.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        let shared = Arc::clone(&worker.feed);
        let url = || lock_feed(&shared).url.clone();
        worker.handle(Command::SetChat(true), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(url(), None, "not registered yet");
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(30));
        assert_eq!(url().as_deref(), Some("https://hub"));
        assert_eq!(lock_feed(&shared).server, None);
        assert!(lock_feed(&shared).chat);
        // In the menus the chat off stops the reading.
        worker.handle(Command::SetChat(false), t0);
        assert_eq!(url(), None);
        // On a server it reads anyway, for the looks and emotes, the chat hidden.
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        assert_eq!(url().as_deref(), Some("https://hub"));
        assert_eq!(lock_feed(&shared).server.as_deref(), Some("1.2.3.4:29070"));
        assert!(!lock_feed(&shared).chat);
        worker.handle(Command::SetChat(true), t0);
        assert!(url().is_some() && lock_feed(&shared).chat);
        worker.handle(Command::SetChat(false), t0);
        worker.handle(Command::Leave, t0);
        assert_eq!(url(), None, "back in the menus with the chat off");
        worker.handle(Command::SetChat(true), t0);
        assert!(url().is_some());
        worker.handle(Command::Configure(Settings::default()), t0);
        assert_eq!(url(), None, "identity off");
    }

    #[test]
    fn another_server_or_the_identity_off_is_a_new_reading_for_the_looks() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        let shared = Arc::clone(&worker.feed);
        let generation = || lock_feed(&shared).generation;
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let first = generation();
        // The chat, a slot or a name change nothing of what is read.
        worker.handle(Command::SetChat(true), t0);
        worker.handle(Command::Enter(here(4, "Sol")), t0);
        assert_eq!(generation(), first);
        let elsewhere = Location {
            server: "5.6.7.8:29070".parse().unwrap(),
            ..here(4, "Sol")
        };
        worker.handle(Command::Enter(elsewhere), t0);
        let second = generation();
        assert!(second > first);
        worker.handle(Command::Configure(Settings::default()), t0);
        assert!(generation() > second, "identity off");
    }

    fn lit(saber: &str) -> Look {
        Look {
            saber: saber.to_owned(),
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
        }
    }

    #[test]
    fn the_look_goes_once_the_claim_is_accepted_and_at_most_once_a_second() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        // Without a claim there is nowhere to wear it.
        worker.handle(Command::Look(lit("")), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert!(fake.looks().is_empty());
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log()[1..],
            [
                "claim 1.2.3.4:29070 3 Sol",
                "presence 1.2.3.4:29070",
                "look 1.2.3.4:29070 \"\" true",
            ]
        );
        assert!(lock(&snapshot).look_outcome.as_ref().unwrap().sent);
        // Changes within the second wait, and only the latest goes.
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.handle(Command::Look(lit("saber_sun")), ms(200));
        assert_eq!(worker.tick(ms(200)), Duration::from_millis(800));
        worker.handle(Command::Look(Look::default()), ms(500));
        worker.tick(ms(500));
        worker.handle(Command::Look(lit("saber_sun")), ms(700));
        worker.tick(ms(700));
        assert_eq!(fake.looks().len(), 1);
        worker.tick(ms(1_000));
        assert_eq!(
            fake.looks(),
            [
                "look 1.2.3.4:29070 \"\" true",
                "look 1.2.3.4:29070 \"saber_sun\" true"
            ]
        );
        // The same look is not sent again, nor with the claim renewed.
        worker.handle(Command::Look(lit("saber_sun")), ms(2_000));
        worker.tick(ms(2_000));
        worker.tick(t0 + Duration::from_secs(46));
        assert_eq!(fake.looks().len(), 2);
    }

    #[test]
    fn a_refusal_drops_only_the_side_the_hub_names() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_glitch".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        // The first saber's skin is not held, the second's is: only the first goes.
        worker.handle(
            Command::Look(Look {
                saber2: Some("saber_glitch".to_owned()),
                ..lit("saber_sun")
            }),
            t0,
        );
        worker.tick(t0);
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.tick(ms(1_000));
        assert_eq!(
            fake.looks(),
            [
                "look 1.2.3.4:29070 \"saber_sun\" true second \"saber_glitch\"",
                "look 1.2.3.4:29070 \"\" true second \"saber_glitch\"",
            ]
        );
        // A hub from before the side was named answers `not_unlocked` for the second's
        // skin too: with no first skin worn, that is the second's.
        let old = Look {
            saber: String::new(),
            illuminate: true,
            saber2: Some("saber_moon".to_owned()),
            ..lit("")
        };
        assert_eq!(refused_skin("not_unlocked", &old), Some("saber_moon"));
        assert_eq!(refused_skin("saber2_not_unlocked", &lit("saber_sun")), None);
        assert_eq!(refused_skin("bad_body", &old), None);
    }

    #[test]
    fn a_second_sabers_skin_falls_back_when_refused_or_unknown() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let dual = |saber2: &str| Look {
            saber2: Some(saber2.to_owned()),
            ..lit("saber_sun")
        };
        // A second skin the key does not hold: the look goes again without it, the first
        // skin kept.
        worker.handle(Command::Look(dual("saber_glitch")), t0);
        worker.tick(t0);
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.tick(ms(1_000));
        assert_eq!(
            fake.looks(),
            [
                "look 1.2.3.4:29070 \"saber_sun\" true second \"saber_glitch\"",
                "look 1.2.3.4:29070 \"saber_sun\" true",
            ]
        );
        // The stock second blade goes; a hub that does not know `saber2` (400 `bad_body`)
        // keeps the look it holds, and the looks after it go without the second skin.
        worker.handle(Command::Look(dual("")), ms(2_000));
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(HubError::Rejected {
                status: 400,
                code: "bad_body".to_owned(),
                message: "unknown field".to_owned(),
            });
        worker.tick(ms(2_000));
        worker.handle(
            Command::Look(Look {
                illuminate: false,
                ..dual("")
            }),
            ms(3_000),
        );
        worker.tick(ms(3_000));
        assert_eq!(
            fake.looks()[2..],
            [
                "look 1.2.3.4:29070 \"saber_sun\" true second \"\"",
                "look 1.2.3.4:29070 \"saber_sun\" false",
            ]
        );
        // After a while the second skin is tried again (the hub may have been updated), and
        // the look goes again with it.
        worker.tick(ms(3_000) + SABER2_RETRY);
        assert_eq!(
            fake.looks()[4..],
            ["look 1.2.3.4:29070 \"saber_sun\" false second \"\""]
        );
    }

    #[test]
    fn extras_go_with_the_look_and_wait_when_the_hub_does_not_know_them() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let flare = Look {
            saber_on: vec!["flare_trail".to_owned()],
            ..lit("saber_sun")
        };
        // A hub from before 26/10/2026 refuses the field (400 `bad_body`): the look goes
        // again at once without the extras, the skin kept.
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(HubError::Rejected {
                status: 400,
                code: "bad_body".to_owned(),
                message: "unknown field".to_owned(),
            });
        worker.handle(Command::Look(flare.clone()), t0);
        worker.tick(t0);
        worker.tick(t0 + Duration::from_millis(1_000));
        assert_eq!(
            fake.looks(),
            [
                "look 1.2.3.4:29070 \"saber_sun\" true on [\"flare_trail\"] []",
                "look 1.2.3.4:29070 \"saber_sun\" true",
            ]
        );
        // After a while they are tried again (the hub may have been updated).
        worker.tick(t0 + Duration::from_millis(1_000) + SABER2_RETRY);
        assert_eq!(
            fake.looks()[2..],
            ["look 1.2.3.4:29070 \"saber_sun\" true on [\"flare_trail\"] []"]
        );
    }

    #[test]
    fn a_new_claim_gets_the_look_again_and_no_look_sends_nothing() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        // A client that stopped without releasing may have left a live claim with a
        // look: the first accepted claim gets the look, even none.
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"\" false"]);
        // Another slot: a new claim, which starts with no look, so none goes.
        let t1 = t0 + Duration::from_secs(5);
        worker.handle(Command::Enter(here(4, "Sol")), t1);
        worker.tick(t1);
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        assert_eq!(fake.looks().len(), 1);
        worker.handle(Command::Look(lit("")), t1);
        worker.tick(t1);
        assert_eq!(fake.looks().len(), 2);
        // Another slot again: the new claim gets the look again.
        let t2 = t1 + Duration::from_secs(5);
        worker.handle(Command::Enter(here(5, "Sol")), t2);
        worker.tick(t2);
        assert_eq!(fake.log().last().unwrap(), "look 1.2.3.4:29070 \"\" true");
        assert_eq!(fake.looks().len(), 3);
        // Another server: the old claim is withdrawn and the new one gets it.
        let elsewhere = Location {
            server: "5.6.7.8:29070".parse().unwrap(),
            ..here(5, "Sol")
        };
        let t3 = t2 + Duration::from_secs(5);
        worker.handle(Command::Enter(elsewhere), t3);
        worker.tick(t3);
        assert_eq!(fake.log().last().unwrap(), "look 5.6.7.8:29070 \"\" true");
        // A claim that failed may have lapsed: the next accepted one sends it again.
        fake.fail.store(true, Ordering::SeqCst);
        let t4 = t3 + Duration::from_secs(46);
        worker.tick(t4);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t4 + Duration::from_secs(30));
        assert_eq!(fake.looks().len(), 5);
        // Leaving needs nothing: the release drops it.
        worker.handle(Command::Leave, t4 + Duration::from_secs(31));
        assert_eq!(fake.log().last().unwrap(), "release 5.6.7.8:29070");
        // Back on the server: a new claim after a release starts with no look.
        let t5 = t4 + Duration::from_secs(40);
        worker.handle(Command::Look(Look::default()), t5);
        worker.handle(Command::Enter(here(5, "Sol")), t5);
        worker.tick(t5);
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        assert_eq!(fake.looks().len(), 5);
    }

    #[test]
    fn after_a_failed_claim_the_look_goes_whatever_it_is() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        // The renewal fails (it timed out, say) while the hub still holds the claim and
        // its look; meanwhile the look goes back to none.
        fake.fail.store(true, Ordering::SeqCst);
        let t1 = t0 + Duration::from_secs(46);
        worker.tick(t1);
        worker.handle(Command::Look(Look::default()), t1);
        fake.fail.store(false, Ordering::SeqCst);
        // The same claim is accepted again: the hub may still hold the Sun, so none goes.
        let t2 = t1 + Duration::from_secs(30);
        worker.tick(t2);
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"\" false"
        );
        assert_eq!(fake.looks().len(), 2);
        // Known again: not repeated.
        worker.tick(t2 + Duration::from_secs(46));
        assert_eq!(fake.looks().len(), 2);
    }

    fn reward(id: u64) -> crate::news::Reward {
        crate::news::Reward {
            id,
            round: 1,
            round_title: "Contributor rewards #1".to_owned(),
            reason: "Fixed the fog".to_owned(),
            credits: 500,
            ..crate::news::Reward::default()
        }
    }

    fn news_item(id: u64) -> Announcement {
        Announcement {
            id,
            at: 1,
            kind: "news".to_owned(),
            title: format!("News {id}"),
            text: "Text".to_owned(),
            version: String::new(),
            images: std::collections::BTreeMap::new(),
        }
    }

    /// Announcements and rewards are read at the registration and every 30 minutes; a
    /// hub without them (404) is left alone an hour and nothing is said; the feed's
    /// announcements join the list once, and a round reads the rewards again soon.
    #[test]
    fn announcements_and_rewards_are_read_at_the_registration_and_kept() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(*fake.news_reads.lock().unwrap(), (1, 1));
        assert!(lock(&snapshot).announcements.is_empty() && lock(&snapshot).rewards.is_none());
        assert_eq!(lock(&snapshot).status, Status::Online, "no error said");
        worker.tick(t0 + Duration::from_secs(1_900));
        assert_eq!(*fake.news_reads.lock().unwrap(), (1, 1), "an hour's rest");
        *fake.news.lock().unwrap() = Some((
            vec![news_item(2), news_item(5)],
            Rewards {
                pending: vec![reward(7)],
                ..Rewards::default()
            },
        ));
        let later = t0 + Duration::from_secs(3_700);
        worker.tick(later);
        let shown = lock(&snapshot).clone();
        let ids: Vec<u64> = shown.announcements.iter().map(|a| a.id).collect();
        assert_eq!(ids, [5, 2], "newest first");
        assert_eq!(shown.rewards.as_ref().unwrap().pending.len(), 1);
        let revision = shown.announcements_revision;
        worker.handle(
            Command::News(crate::feed::FeedNews::Announcements(vec![
                news_item(5),
                news_item(6),
            ])),
            later,
        );
        let shown = lock(&snapshot).clone();
        let ids: Vec<u64> = shown.announcements.iter().map(|a| a.id).collect();
        assert_eq!(ids, [6, 5, 2]);
        assert_eq!(shown.announcements_revision, revision + 1);
        // A round: the rewards are read again at once, then not before 30 seconds.
        worker.handle(Command::News(crate::feed::FeedNews::Round), later);
        worker.tick(later);
        assert_eq!(fake.news_reads.lock().unwrap().1, 3);
        worker.handle(Command::RefreshRewards, later + Duration::from_secs(5));
        worker.tick(later + Duration::from_secs(5));
        assert_eq!(fake.news_reads.lock().unwrap().1, 3);
        worker.tick(later + Duration::from_secs(31));
        assert_eq!(fake.news_reads.lock().unwrap().1, 4);
    }

    /// A claim takes the hub's profile and rewards after it; a second says it was claimed
    /// already in the player's words, and the rewards are read again.
    #[test]
    fn a_claim_takes_the_profile_and_the_rewards_after_it() {
        let fake = Fake::default();
        *fake.news.lock().unwrap() = Some((
            Vec::new(),
            Rewards {
                pending: vec![reward(7), reward(8)],
                ..Rewards::default()
            },
        ));
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::ClaimReward(7), t0);
        let first = lock(&snapshot).claim.clone().unwrap();
        assert!(
            !first.done() && fake.log().is_empty(),
            "not before the registration"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::ClaimReward(7), t0);
        let claimed = lock(&snapshot).clone();
        assert!(claimed.claim.as_ref().unwrap().done());
        assert_eq!(claimed.claim.as_ref().unwrap().serial, 2);
        assert_eq!(claimed.me.as_ref().map(|me| me.credits), Some(500));
        let rewards = claimed.rewards.unwrap();
        assert_eq!(rewards.pending.len(), 1);
        assert_eq!(rewards.claimed[0].id, 7);
        worker.handle(Command::ClaimReward(7), t0);
        assert_eq!(
            lock(&snapshot).claim.clone().unwrap().message,
            "This reward was claimed already."
        );
        worker.handle(Command::ClaimReward(99), t0);
        assert_eq!(
            lock(&snapshot).claim.clone().unwrap().message,
            "This reward is not waiting for you any more."
        );
    }

    /// The creator's announcement is checked before it goes; the hub's refusal is said in
    /// words, and one it took joins the list.
    #[test]
    fn an_announcement_is_checked_sent_and_kept() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let request = |title: &str| AnnounceRequest {
            release: true,
            title: title.to_owned(),
            text: "New things".to_owned(),
            version: "2026.1025.1".to_owned(),
        };
        worker.handle(
            Command::Announce(AnnounceRequest {
                version: "soon".to_owned(),
                ..request("SJK")
            }),
            t0,
        );
        let refused_here = lock(&snapshot).announce.clone().unwrap();
        assert!(!refused_here.sent);
        assert_eq!(
            refused_here.message,
            crate::news::AnnounceError::Version.message()
        );
        assert!(!fake.log().iter().any(|line| line.starts_with("announce")));
        worker.handle(Command::Announce(request("refused")), t0);
        assert_eq!(
            lock(&snapshot).announce.clone().unwrap().message,
            "Only the SJK team can announce"
        );
        worker.handle(Command::Announce(request("SJK 2026.1025.1")), t0);
        let sent = lock(&snapshot).clone();
        assert!(sent.announce.unwrap().sent);
        assert_eq!(sent.announcements[0].title, "SJK 2026.1025.1");
    }

    fn refused(status: u16, code: &str) -> HubError {
        HubError::Rejected {
            status,
            code: code.to_owned(),
            message: String::new(),
        }
    }

    #[test]
    fn a_look_refused_for_too_many_or_the_signature_goes_again_later() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let sent = fake.looks().len();
        for (status, code) in [(429, "rate_limited"), (429, "look_quota"), (401, "clock")] {
            fake.look_refusals
                .lock()
                .unwrap()
                .push_back(refused(status, code));
        }
        let mut at = t0 + LOOK_EVERY;
        worker.handle(Command::Look(lit("")), at);
        // The key's own quota counts whole seconds: a second more is enough.
        let waits = [
            ("rate_limited", LOOK_AGAIN),
            ("look_quota", LOOK_QUOTA_AGAIN),
            ("clock", LOOK_AGAIN),
        ];
        for (n, (code, wait)) in waits.into_iter().enumerate() {
            worker.tick(at);
            assert_eq!(fake.looks().len(), sent + n + 1, "{code}");
            assert!(!lock(&snapshot).look_outcome.clone().unwrap().sent);
            // Not again within the wait, then again.
            worker.tick(at + wait - Duration::from_millis(1));
            assert_eq!(fake.looks().len(), sent + n + 1, "{code}");
            at += wait + Duration::from_millis(5);
        }
        worker.tick(at);
        assert_eq!(fake.looks().len(), sent + 4);
        assert!(lock(&snapshot).look_outcome.clone().unwrap().sent);
        // Another refusal (an older hub) is not repeated.
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(refused(404, "not_found"));
        worker.handle(Command::Look(Look::default()), at);
        worker.tick(at + LOOK_AGAIN);
        worker.tick(at + LOOK_AGAIN * 3);
        assert_eq!(fake.looks().len(), sent + 5);
    }

    /// A look [`Service::set_look`] leaves in the shared slot is taken between the
    /// round's hub calls and goes at once; its outcome says how long it waited.
    #[test]
    fn a_look_left_in_the_slot_goes_at_the_next_tick_and_reports_its_wait() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let sent = fake.looks().len();
        *worker.latest_look.lock().unwrap() = Some(lit(""));
        worker.handle(Command::LookWaiting, t0 + Duration::from_millis(300));
        worker.tick(t0 + LOOK_EVERY);
        assert_eq!(fake.looks().len(), sent + 1);
        assert!(worker.latest_look.lock().unwrap().is_none());
        let outcome = lock(&snapshot).look_outcome.clone().unwrap();
        assert!(outcome.sent);
        assert!(outcome.message.starts_with("70"), "{}", outcome.message);
    }

    /// A look refused because the claim lapsed at the hub asks for the claim again at
    /// once, rather than at the next renewal 45 seconds later.
    #[test]
    fn a_lapsed_claim_is_asked_again_at_once() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(refused(403, "not_on_server"));
        let at = t0 + LOOK_EVERY;
        worker.handle(Command::Look(lit("saber_sun")), at);
        worker.tick(at);
        assert!(worker.look_claim.is_none());
        assert!(worker.due_claim <= at + Duration::from_millis(50));
    }

    /// A hub from before finishers (04/11/2026) refuses `finisher` as an unknown field:
    /// the look goes again at once without it, and later looks leave it out. A finisher
    /// the key does not hold is left out the same way, the rest of the look kept.
    #[test]
    fn a_finisher_an_older_hub_refuses_or_the_key_lacks_is_left_out() {
        let t0 = Instant::now();
        for (status, code) in [(400, "bad_body"), (403, "finisher_not_unlocked")] {
            let fake = Fake::default();
            fake.owned.lock().unwrap().push("saber_sun".to_owned());
            let (mut worker, _) = worker(&fake, t0);
            worker.handle(Command::Configure(on("https://hub")), t0);
            worker.handle(Command::Enter(here(3, "Sol")), t0);
            worker.tick(t0);
            let sent = fake.looks().len();
            fake.look_refusals
                .lock()
                .unwrap()
                .push_back(refused(status, code));
            let at = t0 + LOOK_EVERY;
            let finishing = Look {
                finisher: "finisher_disintegrate".to_owned(),
                ..lit("saber_sun")
            };
            worker.handle(Command::Look(finishing.clone()), at);
            worker.tick(at);
            worker.tick(at);
            assert_eq!(
                fake.looks()[sent..],
                [
                    "look 1.2.3.4:29070 \"saber_sun\" true finisher finisher_disintegrate",
                    "look 1.2.3.4:29070 \"saber_sun\" true",
                ],
                "{code}"
            );
            // Another look still goes without it.
            let at = at + LOOK_EVERY;
            worker.handle(
                Command::Look(Look {
                    illuminate: false,
                    ..finishing
                }),
                at,
            );
            worker.tick(at);
            assert_eq!(
                fake.looks().last().unwrap(),
                "look 1.2.3.4:29070 \"saber_sun\" false",
                "{code}"
            );
        }
    }

    /// A hub from before the sounds choice refuses `saber_sounds` as an unknown field:
    /// the look goes again at once without it, and later looks leave it out, so the
    /// blade still reaches everyone (who hear the sounds that follow it). A sound set the
    /// key does not hold is left out the same way, the blade kept.
    #[test]
    fn sounds_an_older_hub_refuses_or_the_key_lacks_are_left_out_and_the_blade_goes() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let sent = fake.looks().len();
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(refused(400, "bad_body"));
        let at = t0 + LOOK_EVERY;
        let sounds = Look {
            saber_sounds: "stock".to_owned(),
            ..lit("saber_sun")
        };
        worker.handle(Command::Look(sounds.clone()), at);
        worker.tick(at);
        worker.tick(at);
        assert_eq!(
            fake.looks()[sent..],
            [
                "look 1.2.3.4:29070 \"saber_sun\" true sounds stock",
                "look 1.2.3.4:29070 \"saber_sun\" true",
            ]
        );
        // Another choice still goes without it to this hub.
        let at = at + LOOK_EVERY;
        worker.handle(
            Command::Look(Look {
                illuminate: false,
                ..sounds.clone()
            }),
            at,
        );
        worker.tick(at);
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"saber_sun\" false"
        );

        // A newer hub: a sound set the key lacks is refused and left out, the blade kept.
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let (mut worker, _) = self::worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let sent = fake.looks().len();
        fake.look_refusals
            .lock()
            .unwrap()
            .push_back(refused(403, "sounds_not_unlocked"));
        let at = t0 + LOOK_EVERY;
        worker.handle(
            Command::Look(Look {
                saber_sounds: "saber_void".to_owned(),
                ..lit("saber_sun")
            }),
            at,
        );
        worker.tick(at);
        worker.tick(at);
        assert_eq!(
            fake.looks()[sent..],
            [
                "look 1.2.3.4:29070 \"saber_sun\" true sounds saber_void",
                "look 1.2.3.4:29070 \"saber_sun\" true",
            ]
        );
    }

    fn tagged(id: &str) -> Look {
        Look {
            tag: Some(crate::PlacedTag {
                finish: None,
                id: id.to_owned(),
                map: "mp/ffa3".to_owned(),
                at: [1.0, 2.0, 3.0],
                normal: [0.0, 0.0, 1.0],
                spin: 90.0,
            }),
            ..lit("saber_sun")
        }
    }

    /// A hub from before tags refuses `tag` as an unknown field: the look goes again at
    /// once without it, and later looks leave it out. A tag the key does not hold is
    /// left out the same way, the blade kept.
    #[test]
    fn a_tag_an_older_hub_refuses_or_the_key_lacks_is_left_out_and_the_blade_goes() {
        for code in ["bad_body", "tag_not_unlocked", "bad_tag"] {
            let fake = Fake::default();
            fake.owned.lock().unwrap().push("saber_sun".to_owned());
            let t0 = Instant::now();
            let (mut worker, _) = worker(&fake, t0);
            worker.handle(Command::Configure(on("https://hub")), t0);
            worker.handle(Command::Enter(here(3, "Sol")), t0);
            worker.tick(t0);
            let sent = fake.looks().len();
            let status = if code == "tag_not_unlocked" { 403 } else { 400 };
            fake.look_refusals
                .lock()
                .unwrap()
                .push_back(refused(status, code));
            let at = t0 + LOOK_EVERY;
            worker.handle(Command::Look(tagged("tag_ez")), at);
            worker.tick(at);
            worker.tick(at);
            assert_eq!(
                fake.looks()[sent..],
                [
                    "look 1.2.3.4:29070 \"saber_sun\" true tag tag_ez",
                    "look 1.2.3.4:29070 \"saber_sun\" true",
                ],
                "{code}"
            );
        }
    }

    fn profile_reads(fake: &Fake) -> usize {
        fake.log()
            .iter()
            .filter(|line| line.starts_with("lookup"))
            .count()
    }

    #[test]
    fn a_refused_skin_reads_the_own_profile_soon_at_most_twice_a_minute() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        // The profile listed the Sun, but staff took it back: the hub refuses it.
        worker.handle(Command::Look(lit("saber_sun")), t0);
        assert_eq!(
            worker.tick(t0),
            Duration::ZERO,
            "the profile is due at once"
        );
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        assert_eq!(profile_reads(&fake), 0);
        let ms = |ms| t0 + Duration::from_millis(ms);
        worker.tick(ms(10));
        assert_eq!(profile_reads(&fake), 1);
        // Refused again: not before 30 seconds from the last early reading.
        worker.handle(Command::Look(lit("saber_moon")), ms(2_000));
        worker.tick(ms(2_000));
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"saber_moon\" true"
        );
        worker.tick(ms(29_000));
        assert_eq!(profile_reads(&fake), 1);
        worker.tick(ms(30_010));
        assert_eq!(profile_reads(&fake), 2);
    }

    #[test]
    fn the_hub_taking_the_own_skin_back_reads_the_own_profile_soon() {
        let fake = Fake::default();
        fake.owned.lock().unwrap().push("saber_sun".to_owned());
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        // The fake hub numbers it 15. The feed brings it back: nothing to do.
        let t1 = t0 + Duration::from_secs(1);
        worker.handle(Command::OwnLook(15, "saber_sun".into()), t1);
        // An older one, or a newer one with the same skin, is not news either.
        worker.handle(Command::OwnLook(12, String::new()), t1);
        worker.handle(Command::OwnLook(16, "saber_sun".into()), t1);
        worker.tick(t1);
        assert_eq!(profile_reads(&fake), 0);
        // Staff relock it: the hub's event takes the skin off.
        worker.handle(Command::OwnLook(20, String::new()), t1);
        worker.tick(t1);
        assert_eq!(profile_reads(&fake), 1);
        // Another within the 30 seconds waits for them.
        worker.handle(Command::OwnLook(21, String::new()), t1);
        worker.tick(t1 + Duration::from_secs(29));
        assert_eq!(profile_reads(&fake), 1);
        worker.tick(t1 + Duration::from_secs(30));
        assert_eq!(profile_reads(&fake), 2);
    }

    #[test]
    fn a_blade_skin_the_key_lacks_is_left_out_until_the_profile_changes() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.handle(Command::Look(lit("saber_sun")), t0);
        worker.tick(t0);
        assert_eq!(fake.looks(), ["look 1.2.3.4:29070 \"saber_sun\" true"]);
        let outcome = lock(&snapshot).look_outcome.clone().unwrap();
        assert!(!outcome.sent);
        // A second later the Illuminate still goes, without the skin, and then nothing.
        let t1 = t0 + Duration::from_secs(1);
        worker.tick(t1);
        worker.tick(t1 + Duration::from_secs(5));
        assert_eq!(
            fake.looks()[1..],
            ["look 1.2.3.4:29070 \"\" true".to_owned()]
        );
        // Another skin is tried and refused; back to the first, which stays out: the
        // hub already holds the look without a skin, so nothing goes.
        worker.handle(
            Command::Look(lit("saber_moon")),
            t1 + Duration::from_secs(5),
        );
        worker.tick(t1 + Duration::from_secs(5));
        worker.handle(Command::Look(lit("saber_sun")), t1 + Duration::from_secs(7));
        worker.tick(t1 + Duration::from_secs(7));
        worker.tick(t1 + Duration::from_secs(8));
        assert_eq!(
            fake.looks()[2..],
            ["look 1.2.3.4:29070 \"saber_moon\" true".to_owned()]
        );
        // Staff unlock it on the player's own key: the profile changes, it goes.
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Unlock {
                key_id: me,
                unlock: "saber_sun".into(),
                note: String::new(),
            }),
            t1 + Duration::from_secs(9),
        );
        worker.tick(t1 + Duration::from_secs(9));
        assert_eq!(
            fake.looks().last().unwrap(),
            "look 1.2.3.4:29070 \"saber_sun\" true"
        );
        assert!(lock(&snapshot).look_outcome.clone().unwrap().sent);
    }

    #[test]
    fn staff_unlock_and_relock_change_the_profile_they_answer() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Unlock {
                key_id: me.clone(),
                unlock: "saber_sun".into(),
                note: "Thanks".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Unlocked saber_sun");
        let unlocks = |snapshot: &Mutex<Snapshot>| lock(snapshot).me.clone().unwrap().unlocks;
        assert_eq!(unlocks(&snapshot)[0].id, "saber_sun");
        worker.handle(
            Command::Staff(StaffRequest::Relock {
                key_id: me,
                unlock: "saber_sun".into(),
            }),
            t0,
        );
        assert_eq!(staff.lock().unwrap().message, "Took back saber_sun");
        assert!(unlocks(&snapshot).is_empty());
        assert_eq!(staff.lock().unwrap().players[0].unlocks, []);
    }

    #[test]
    fn achievement_counts_go_to_the_hub_at_most_once_a_minute() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 3)])), t0);
        worker.tick(t0);
        assert_eq!(sent_counts(&fake), ["achievements kills_100=3"]);
        let me = lock(&snapshot).me.clone().unwrap();
        assert_eq!(me.achievements[0].progress, 3);
        // The same counts are not sent again; new ones wait for the minute.
        worker.tick(t0 + Duration::from_secs(30));
        worker.handle(
            Command::Achievements(counts(&[("kills_100", 4)])),
            t0 + Duration::from_secs(30),
        );
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(sent_counts(&fake).len(), 1);
        worker.tick(t0 + Duration::from_secs(61));
        assert_eq!(
            sent_counts(&fake).last().unwrap(),
            "achievements kills_100=4"
        );
        worker.tick(t0 + Duration::from_secs(5_000));
        assert_eq!(sent_counts(&fake).len(), 2, "nothing waits");
    }

    #[test]
    fn counts_the_hub_held_back_are_sent_again_an_hour_later() {
        let fake = Fake::default();
        *fake.cap.lock().unwrap() = Some(2);
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 5)])), t0);
        worker.tick(t0);
        worker.tick(t0 + Duration::from_secs(120));
        assert_eq!(sent_counts(&fake).len(), 1, "held back: waits the hour");
        *fake.cap.lock().unwrap() = None;
        worker.tick(t0 + Duration::from_secs(3_601));
        assert_eq!(sent_counts(&fake).len(), 2);
        worker.tick(t0 + Duration::from_secs(8_000));
        assert_eq!(sent_counts(&fake).len(), 2, "all taken: nothing waits");
    }

    #[test]
    fn counts_that_change_after_a_hold_go_at_the_minute() {
        // Report 149 (26/10/2026): an achievement unlocked during the hour after a hold
        // waited that hour, its kyber crystal with it.
        let fake = Fake::default();
        *fake.cap.lock().unwrap() = Some(2);
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 5)])), t0);
        worker.tick(t0);
        assert_eq!(sent_counts(&fake).len(), 1);
        *fake.cap.lock().unwrap() = None;
        let later = t0 + Duration::from_secs(30);
        worker.handle(
            Command::Achievements(counts(&[("kills_100", 5), ("duel_flawless", 1)])),
            later,
        );
        let wait = worker.tick(later);
        assert!(wait <= Duration::from_secs(30), "{wait:?}");
        worker.tick(t0 + Duration::from_secs(61));
        assert_eq!(
            sent_counts(&fake).last().unwrap(),
            "achievements duel_flawless=1,kills_100=5"
        );
    }

    #[test]
    fn an_unlock_reads_the_profile_soon_for_its_kyber_crystal() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Achievements(counts(&[("kills_100", 99)])), t0);
        worker.tick(t0);
        let reads = profile_reads(&fake);
        // Short of the goal: the profile keeps its ten minutes.
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(profile_reads(&fake), reads);
        worker.handle(
            Command::Achievements(counts(&[("kills_100", 100)])),
            t0 + Duration::from_secs(40),
        );
        worker.tick(t0 + Duration::from_secs(61));
        assert_eq!(
            sent_counts(&fake).last().unwrap(),
            "achievements kills_100=100"
        );
        worker.tick(t0 + Duration::from_secs(62));
        assert_eq!(
            profile_reads(&fake),
            reads + 1,
            "the crystal is listed soon"
        );
        // Unlocked already: a later answer reads nothing more.
        worker.handle(
            Command::Achievements(counts(&[("kills_100", 101)])),
            t0 + Duration::from_secs(130),
        );
        worker.tick(t0 + Duration::from_secs(130));
        worker.tick(t0 + Duration::from_secs(200));
        assert_eq!(sent_counts(&fake).len(), 3);
        assert_eq!(profile_reads(&fake), reads + 1);
    }

    #[test]
    fn a_bio_breaking_the_rules_is_refused_before_the_hub_hears_it() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBio("hi a\u{200D}b".to_owned()), t0);
        assert!(!fake.log().iter().any(|line| line.starts_with("bio")));
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some(crate::bio::BioError::Character.message())
        );
        worker.handle(Command::SetBio("  hello   there ".to_owned()), t0);
        assert_eq!(fake.log().last().unwrap(), "bio hello there");
    }

    #[test]
    fn profile_banner_equipment_uses_the_worker_and_authoritative_profile() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::SetBanner("banner_jedi_archives".into()), t0);
        assert!(!fake.log().iter().any(|line| line.starts_with("banner")));
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBanner("banner_jedi_archives".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "banner banner_jedi_archives");
        assert_eq!(
            lock(&snapshot).me.as_ref().unwrap().banner.as_ref(),
            "banner_jedi_archives"
        );
        worker.handle(Command::SetBanner(String::new()), t0);
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().banner.as_ref(), "");
    }

    #[test]
    fn a_name_emoji_is_checked_then_sent_and_the_profile_takes_it() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetEmoji("🦊🦊".to_owned()), t0);
        assert!(!fake.log().iter().any(|line| line.starts_with("emoji")));
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some(crate::name_emoji::EmojiError.message())
        );
        worker.handle(Command::SetEmoji(" 🦊 ".to_owned()), t0);
        assert_eq!(fake.log().last().unwrap(), "emoji 🦊");
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().emoji, "🦊");
        worker.handle(Command::SetEmoji(String::new()), t0);
        assert_eq!(fake.log().last().unwrap(), "emoji ");
        assert_eq!(lock(&snapshot).notice.as_deref(), Some("emoji removed"));
    }

    #[test]
    fn the_own_id_follows_the_person_the_key_belongs_to() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let local = Identity::from_seed([1; 32]).key_id();
        assert_eq!(
            lock(&snapshot).key_id,
            local,
            "this PC's key until the hub answers"
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // The fake hub answers for person 0123456789abcdef, as for a linked key.
        let read = lock(&snapshot).clone();
        assert_eq!(read.key_id, "0123456789abcdef");
        assert_eq!(read.local_key_id, local, "the key file's id stays");
        // The feed is told, so the player's own drops are still theirs.
        assert_eq!(lock_feed(&worker.feed).person, "0123456789abcdef");
        // The identity off: this PC's key again.
        worker.handle(
            Command::Configure(Settings {
                enabled: false,
                ..on("https://hub")
            }),
            t0,
        );
        let read = lock(&snapshot).clone();
        assert!(read.me.is_none());
        assert_eq!(read.key_id, local);
        assert_eq!(lock_feed(&worker.feed).person, local);
    }

    #[test]
    fn staff_verify_merge_and_unlink_update_the_list_they_answer() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let state = || staff.lock().unwrap().clone();
        {
            let mut staff = staff.lock().unwrap();
            staff.players = vec![
                Profile {
                    key_id: "aaaaaaaaaaaaaaaa".into(),
                    ..profile("Kept")
                },
                Profile {
                    key_id: "bbbbbbbbbbbbbbbb".into(),
                    keys: vec!["bbbbbbbbbbbbbbbb".into(), "cccccccccccccccc".into()],
                    ..profile("Reset")
                },
                Profile {
                    key_id: "dddddddddddddddd".into(),
                    ..profile("Else")
                },
            ];
        }
        worker.handle(
            Command::Staff(StaffRequest::Verify {
                key_id: "aaaaaaaaaaaaaaaa".into(),
                verified: true,
                tick: None,
            }),
            t0,
        );
        assert_eq!(state().message, "Verified");
        assert!(state().players[0].verified);
        // Merging by a linked key of the merged-away person takes that person out.
        worker.handle(
            Command::Staff(StaffRequest::Merge {
                key_id: "aaaaaaaaaaaaaaaa".into(),
                from: "cccccccccccccccc".into(),
            }),
            t0,
        );
        let merged = state();
        assert_eq!(merged.message, "Merged the other player into this one");
        let ids: Vec<&str> = merged.players.iter().map(|p| p.key_id.as_str()).collect();
        assert_eq!(ids, ["aaaaaaaaaaaaaaaa", "dddddddddddddddd"]);
        assert_eq!(
            merged.players[0].keys,
            ["aaaaaaaaaaaaaaaa", "cccccccccccccccc"]
        );
        // Unlinking another key leaves this PC registered.
        let registers = || {
            fake.log()
                .iter()
                .filter(|l| l.starts_with("register"))
                .count()
        };
        let before = registers();
        worker.handle(
            Command::Staff(StaffRequest::Unlink {
                key_id: "cccccccccccccccc".into(),
            }),
            t0,
        );
        assert_eq!(state().message, "Unlinked the key: it starts afresh");
        worker.tick(t0);
        assert_eq!(registers(), before);
        // Unlinking this PC's own key: it starts afresh and registers again.
        let local = Identity::from_seed([1; 32]).key_id();
        worker.handle(
            Command::Staff(StaffRequest::Unlink {
                key_id: local.clone(),
            }),
            t0,
        );
        assert!(lock(&snapshot).me.is_none());
        assert_eq!(lock(&snapshot).key_id, local);
        worker.tick(t0);
        assert_eq!(registers(), before + 1);
        assert!(lock(&snapshot).me.is_some());
    }

    #[test]
    fn staff_requests_need_a_staff_profile_and_update_what_they_change() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        let staff = Arc::clone(&worker.staff);
        let state = || staff.lock().unwrap().clone();
        worker.handle(Command::Staff(StaffRequest::Search("so".into())), t0);
        let found = state();
        assert_eq!(found.players.len(), 2);
        assert_eq!(found.message, "2 players found");
        assert!(!found.failed);
        // A change to another key replaces it in the list; to the player's own key,
        // their profile too.
        let me = lock(&snapshot).me.clone().unwrap().key_id;
        worker.handle(
            Command::Staff(StaffRequest::Award {
                key_id: me.clone(),
                medal: "bug_hunter".into(),
                note: String::new(),
                rank: 1,
            }),
            t0,
        );
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().medals.len(), 1);
        assert_eq!(state().message, "Gave bug_hunter");
        worker.handle(
            Command::Staff(StaffRequest::MedalRank {
                key_id: me.clone(),
                medal: "bug_hunter".into(),
                rank: 3,
            }),
            t0,
        );
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().medals[0].rank, 3);
        assert_eq!(state().message, "Set bug_hunter to rank 3");
        // A key that is not staff is refused here, before the hub hears it.
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        let before = fake.log().len();
        worker.handle(Command::Staff(StaffRequest::Search(String::new())), t0);
        assert_eq!(fake.log().len(), before);
        assert!(state().failed);
        assert_eq!(state().message, "only SJK staff can do this");
    }

    /// A worker told to keep the asset packs in a fresh folder and switched on; the
    /// folder's parent goes when the [`tempfile::TempDir`] does.
    fn keeping_assets(
        fake: &Fake,
        now: Instant,
    ) -> (Worker, Arc<Mutex<Snapshot>>, tempfile::TempDir, PathBuf) {
        let (mut worker, snapshot) = worker(fake, now);
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(FOLDER);
        worker.handle(Command::AssetsDir(dir.clone()), now);
        worker.handle(Command::Configure(on("https://hub")), now);
        (worker, snapshot, root, dir)
    }

    #[test]
    fn asset_packs_are_downloaded_once_registered_and_again_only_when_they_change() {
        let fake = Fake::default();
        fake.serve("sjk_skins", b"sun v1");
        let t0 = Instant::now();
        let (mut worker, snapshot, _root, dir) = keeping_assets(&fake, t0);
        worker.tick(t0);
        assert_eq!(fake.log()[0], "register");
        assert_eq!(fake.asset_requests(), ["assets", "asset sjk_skins"]);
        assert_eq!(std::fs::read(dir.join("sjk_skins.pk3")).unwrap(), b"sun v1");
        assert_eq!(lock(&snapshot).packs_revision, 1);
        assert!(
            lock(&snapshot)
                .assets_note
                .as_ref()
                .unwrap()
                .contains("sjk_skins")
        );
        // Not again for six hours.
        worker.tick(t0 + Duration::from_secs(3_600));
        assert_eq!(fake.asset_requests().len(), 2);
        // Then the list again; the pack matches, so it is not downloaded.
        let t1 = t0 + ASSETS_EVERY;
        worker.tick(t1);
        assert_eq!(
            fake.asset_requests(),
            ["assets", "asset sjk_skins", "assets"]
        );
        assert_eq!(lock(&snapshot).packs_revision, 1);
        // A new version of the pack is downloaded and replaces the old.
        fake.serve("sjk_skins", b"sun v2");
        let t2 = t1 + ASSETS_EVERY;
        worker.tick(t2);
        assert_eq!(fake.asset_requests()[3..], ["assets", "asset sjk_skins"]);
        assert_eq!(std::fs::read(dir.join("sjk_skins.pk3")).unwrap(), b"sun v2");
        assert_eq!(lock(&snapshot).packs_revision, 2);
        // A pack the hub no longer lists stays for offline use.
        fake.packs.lock().unwrap().clear();
        worker.tick(t2 + ASSETS_EVERY);
        assert_eq!(fake.asset_requests().len(), 6);
        assert!(dir.join("sjk_skins.pk3").exists());
        assert_eq!(lock(&snapshot).packs_revision, 2);
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn a_pack_unlike_its_listing_is_refused_and_the_cached_one_kept() {
        let fake = Fake::default();
        fake.serve("sjk_skins", b"sun v2");
        let t0 = Instant::now();
        let (mut worker, snapshot, _root, dir) = keeping_assets(&fake, t0);
        write_pack(&dir, "sjk_skins", b"sun v1").unwrap();
        // The same length, other bytes: the hash differs.
        *fake.served.lock().unwrap() = Some(b"sun v3".to_vec());
        worker.tick(t0);
        assert_eq!(fake.asset_requests(), ["assets", "asset sjk_skins"]);
        assert_eq!(std::fs::read(dir.join("sjk_skins.pk3")).unwrap(), b"sun v1");
        assert!(!dir.join(".sjk_skins.pk3.part").exists());
        assert_eq!(lock(&snapshot).packs_revision, 0);
        let note = lock(&snapshot).assets_note.clone().unwrap();
        assert!(note.contains("SHA-256"), "{note}");
        assert_eq!(lock(&snapshot).status, Status::Online, "cosmetic");
        // Tried again a minute later; another length is refused too.
        worker.tick(t0 + Duration::from_secs(59));
        assert_eq!(fake.asset_requests().len(), 2);
        *fake.served.lock().unwrap() = Some(b"sun v2, longer".to_vec());
        let t1 = t0 + ASSETS_RETRY_MIN;
        worker.tick(t1);
        assert_eq!(fake.asset_requests().len(), 4);
        assert!(
            lock(&snapshot)
                .assets_note
                .as_ref()
                .unwrap()
                .contains("14 bytes")
        );
        assert_eq!(std::fs::read(dir.join("sjk_skins.pk3")).unwrap(), b"sun v1");
        // The right bytes, two minutes later, are written.
        *fake.served.lock().unwrap() = None;
        worker.tick(t1 + ASSETS_RETRY_MIN * 2);
        assert_eq!(std::fs::read(dir.join("sjk_skins.pk3")).unwrap(), b"sun v2");
        assert_eq!(lock(&snapshot).packs_revision, 1);
    }

    #[test]
    fn asset_check_failures_back_off_and_success_resets_the_wait() {
        let fake = Fake::default();
        fake.serve("sjk_skins", b"sun");
        for refusal in [
            HubError::Network("down".to_owned()),
            refused(401, "bad_signature"),
            refused(503, "error"),
            refused(429, "rate_limited"),
        ] {
            fake.asset_refusals.lock().unwrap().push_back(refusal);
        }
        let t0 = Instant::now();
        let (mut worker, snapshot, _root, _dir) = keeping_assets(&fake, t0);
        worker.tick(t0);
        assert_eq!(fake.asset_requests(), ["assets"]);
        assert_eq!(lock(&snapshot).status, Status::Online, "cosmetic");
        // The worker wakes for the next check.
        assert!(worker.tick(t0 + Duration::from_secs(30)) <= Duration::from_secs(30));
        let mut at = t0;
        for (n, wait) in [60, 120, 240, 480].into_iter().enumerate() {
            worker.tick(at + Duration::from_secs(wait - 1));
            assert_eq!(fake.asset_requests().len(), n + 1, "not before {wait} s");
            at += Duration::from_secs(wait);
            worker.tick(at);
        }
        assert_eq!(
            fake.asset_requests()[4..],
            ["assets", "asset sjk_skins"],
            "the fifth reading succeeds"
        );
        assert_eq!(lock(&snapshot).packs_revision, 1);
        // Success: the next check is six hours on, and the next failure waits a minute.
        fake.asset_refusals
            .lock()
            .unwrap()
            .push_back(HubError::Network("down".to_owned()));
        worker.tick(at + ASSETS_EVERY - Duration::from_secs(1));
        assert_eq!(fake.asset_requests().len(), 6);
        at += ASSETS_EVERY;
        worker.tick(at);
        assert_eq!(fake.asset_requests().len(), 7);
        worker.tick(at + ASSETS_RETRY_MIN);
        assert_eq!(fake.asset_requests().len(), 8);
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn an_older_hub_without_assets_is_asked_again_in_six_hours() {
        let fake = Fake::default();
        fake.asset_refusals
            .lock()
            .unwrap()
            .push_back(refused(404, "not_found"));
        let t0 = Instant::now();
        let (mut worker, snapshot, _root, dir) = keeping_assets(&fake, t0);
        worker.tick(t0);
        worker.tick(t0 + ASSETS_RETRY_MIN);
        worker.tick(t0 + Duration::from_secs(3_600));
        assert_eq!(fake.asset_requests(), ["assets"]);
        assert!(
            lock(&snapshot)
                .assets_note
                .as_ref()
                .unwrap()
                .contains("not_found")
        );
        worker.tick(t0 + ASSETS_EVERY);
        assert_eq!(fake.asset_requests(), ["assets", "assets"]);
        assert!(crate::assets::cached_packs(&dir).is_empty());
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn no_asset_is_asked_for_while_off_unregistered_or_without_a_folder() {
        let fake = Fake::default();
        fake.serve("sjk_skins", b"sun");
        let t0 = Instant::now();
        // Registered, but told no folder.
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.tick(t0 + ASSETS_EVERY);
        assert!(fake.log().contains(&"register".to_owned()));
        assert!(fake.asset_requests().is_empty());
        // A folder, with the identity off: nothing at all.
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join(FOLDER);
        let t1 = t0 + ASSETS_EVERY * 2;
        worker.handle(
            Command::Configure(Settings {
                enabled: false,
                hub_url: "https://hub".into(),
            }),
            t1,
        );
        worker.handle(Command::AssetsDir(dir.clone()), t1);
        let before = fake.log().len();
        worker.tick(t1);
        worker.tick(t1 + ASSETS_EVERY);
        assert_eq!(fake.log().len(), before);
        // On, but the registration fails: no asset request either.
        fake.fail.store(true, Ordering::SeqCst);
        let t2 = t1 + ASSETS_EVERY * 2;
        worker.handle(Command::Configure(on("https://hub")), t2);
        worker.tick(t2);
        assert!(fake.asset_requests().is_empty());
        assert!(!dir.exists());
        // Registered: checked at once.
        fake.fail.store(false, Ordering::SeqCst);
        let t3 = t2 + Duration::from_secs(30);
        worker.tick(t3);
        assert_eq!(fake.asset_requests(), ["assets", "asset sjk_skins"]);
        assert_eq!(lock(&snapshot).packs_revision, 1);
        // Off and on again: checked again at once; the pack matches.
        worker.handle(Command::Configure(Settings::default()), t3);
        worker.tick(t3);
        worker.handle(Command::Configure(on("https://hub")), t3);
        worker.tick(t3);
        assert_eq!(
            fake.asset_requests(),
            ["assets", "asset sjk_skins", "assets"]
        );
        assert_eq!(lock(&snapshot).packs_revision, 1);
    }

    #[test]
    fn nothing_is_sent_while_disabled_or_without_a_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::Disabled);
        worker.handle(Command::Configure(on("")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::NoHub);
        worker.handle(
            Command::Configure(Settings {
                enabled: false,
                hub_url: "https://h".into(),
            }),
            t0,
        );
        worker.tick(t0);
        assert!(fake.log().is_empty());
    }

    #[test]
    fn it_registers_before_it_claims_and_repeats_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log(),
            [
                "register",
                "claim 1.2.3.4:29070 3 Sol",
                "presence 1.2.3.4:29070",
                // What the hub held for the key before the start is not known.
                "look 1.2.3.4:29070 \"\" false"
            ]
        );
        assert_eq!(lock(&snapshot).status, Status::Online);
        // Nothing is due for a while, then the roster, then the claim.
        worker.tick(t0 + POLL_EVERY - Duration::from_secs(1));
        assert_eq!(fake.log().len(), 4);
        worker.tick(t0 + POLL_EVERY);
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        worker.tick(t0 + Duration::from_secs(46));
        assert!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("claim"))
                .count()
                == 2
        );
    }

    #[test]
    fn a_new_map_edits_stamp_in_presence_reads_the_list_at_once() {
        let fake = Fake::default();
        *fake.edits_stamp.lock().unwrap() = Some(4);
        let dir = std::env::temp_dir().join(format!(
            "sjk-identity-edits-stamp-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let t0 = Instant::now();
        let (mut worker, _snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::MapEditsDir(dir.clone()), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let reads = |fake: &Fake| fake.log().iter().filter(|l| *l == "map edits").count();
        worker.tick(t0);
        worker.tick(t0 + Duration::from_millis(10));
        // The look at start, and one for the first stamp seen on this run.
        let start = reads(&fake);
        assert!((1..=2).contains(&start), "{:?}", fake.log());
        // The same stamp: nothing more until the 10 minute look.
        worker.tick(t0 + POLL_EVERY);
        worker.tick(t0 + POLL_EVERY + Duration::from_millis(10));
        assert_eq!(reads(&fake), start);
        // Staff published: the next presence answer brings the list at once.
        *fake.edits_stamp.lock().unwrap() = Some(5);
        worker.tick(t0 + POLL_EVERY * 2);
        worker.tick(t0 + POLL_EVERY * 2 + Duration::from_millis(10));
        assert_eq!(reads(&fake), start + 1, "{:?}", fake.log());
        // A hub from before the stamp changes nothing.
        *fake.edits_stamp.lock().unwrap() = None;
        worker.tick(t0 + POLL_EVERY * 3);
        worker.tick(t0 + POLL_EVERY * 3 + Duration::from_millis(10));
        assert_eq!(reads(&fake), start + 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A new Toy Master stamp in a presence answer (a toy placed or taken away, SJK's
    /// creator came or went) reads the toys at once, for the server the player is on.
    #[test]
    fn a_new_secrets_stamp_in_presence_reads_the_toys_at_once() {
        let fake = Fake::default();
        *fake.secrets_stamp.lock().unwrap() = Some(1);
        let t0 = Instant::now();
        let (mut worker, _snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        let reads = |fake: &Fake| {
            fake.log()
                .iter()
                .filter(|line| line.starts_with("secrets "))
                .cloned()
                .collect::<Vec<_>>()
        };
        worker.tick(t0);
        worker.tick(t0 + Duration::from_millis(10));
        let start = reads(&fake);
        assert!(!start.is_empty(), "{:?}", fake.log());
        assert!(
            start.last().is_some_and(|line| line != "secrets -"),
            "{start:?}"
        );
        // The same stamp: nothing more until the 10 minute look.
        worker.tick(t0 + POLL_EVERY);
        worker.tick(t0 + POLL_EVERY + Duration::from_millis(10));
        assert_eq!(reads(&fake).len(), start.len());
        // The creator came: the next presence answer brings the toys at once.
        *fake.secrets_stamp.lock().unwrap() = Some(2);
        worker.tick(t0 + POLL_EVERY * 2);
        worker.tick(t0 + POLL_EVERY * 2 + Duration::from_millis(10));
        assert_eq!(reads(&fake).len(), start.len() + 1, "{:?}", fake.log());
    }

    #[test]
    fn leaving_or_switching_off_withdraws_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Leave, t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Configure(Settings::default()), t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        let before = fake.log().len();
        worker.tick(t0 + Duration::from_secs(500));
        assert_eq!(fake.log().len(), before);
    }

    #[test]
    fn a_new_name_or_slot_is_claimed_at_once_and_the_same_one_is_not_repeated() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let claims = |fake: &Fake| fake.log().iter().filter(|l| l.starts_with("claim")).count();
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 1);
        worker.handle(Command::Enter(here(3, "Fox")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 2);
        assert_eq!(
            fake.log()
                .iter()
                .filter(|l| l.starts_with("release"))
                .count(),
            0
        );
    }

    #[test]
    fn failures_back_off_and_recovery_clears_the_status() {
        let fake = Fake::default();
        fake.fail.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        let wait = worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert_eq!(wait, RETRY_MIN);
        worker.tick(t0 + Duration::from_secs(5));
        assert_eq!(fake.log().len(), 1, "no retry before the backoff ends");
        let wait = worker.tick(t0 + Duration::from_secs(11));
        assert_eq!(wait, RETRY_MIN * 2);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn a_bad_address_is_reported_not_used() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("http://hub.example")), t0);
        worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert!(fake.log().is_empty());
    }

    #[test]
    fn badges_match_by_slot_and_name_only() {
        let fake = Fake::default();
        *fake.roster.lock().unwrap() = vec![Presence {
            slot: 3,
            claimed_name: "^1Sol".to_owned(),
            key_id: "0123456789abcdef".to_owned(),
            name: "Sol".to_owned(),
            verified: true,
            tick: crate::Tick::None,
            trust: None,
            emoji: String::new(),
            medals: Vec::new(),
            avatar: String::new(),
            look: None,
            whisper_key: None,
            client: String::new(),
        }];
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(1, "Me")), t0);
        worker.tick(t0);
        let snapshot = lock(&snapshot);
        assert!(snapshot.badge(3, "sol").unwrap().verified);
        assert!(
            snapshot.badge(3, "Fox").is_none(),
            "another name in the slot"
        );
        assert!(snapshot.badge(4, "Sol").is_none(), "another slot");
    }

    #[test]
    fn bio_changes_and_lookups_go_through_the_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::SetBio("x".into()), t0);
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some("not connected to the hub")
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBio("hello".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "bio hello");
        assert_eq!(lock(&snapshot).notice.as_deref(), Some("saved"));
        worker.handle(Command::LookUp("0123456789abcdef".into()), t0);
        worker.tick(t0);
        assert!(lock(&snapshot).profiles.contains_key("0123456789abcdef"));
    }

    #[test]
    fn the_model_list_goes_once_registered_and_checks_come_back() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let list = vec!["kyle".to_owned(), "wolfie".to_owned()];
        worker.handle(Command::Models(list.clone()), t0);
        worker.tick(t0);
        assert!(fake.log().is_empty(), "nothing before the identity is on");
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("models"))
                .count(),
            1
        );
        // The same list again is not sent again; a new one is.
        worker.handle(Command::Models(list), t0);
        worker.tick(t0);
        worker.handle(Command::Models(vec!["kyle".to_owned()]), t0);
        worker.tick(t0);
        let sent: Vec<String> = fake
            .log()
            .into_iter()
            .filter(|line| line.starts_with("models"))
            .collect();
        assert_eq!(sent, ["models kyle,wolfie", "models kyle"]);
        let server: SocketAddr = "127.0.0.1:29070".parse().unwrap();
        worker.handle(Command::CheckModel(server, 3, "kyle".to_owned()), t0);
        worker.handle(Command::CheckModel(server, 3, "reborn".to_owned()), t0);
        worker.tick(t0);
        let checks = lock(&snapshot).model_checks.clone();
        let has = |model: &str| checks[&(server, 3, model.to_owned())].has;
        assert_eq!((has("kyle"), has("reborn")), (Some(true), Some(false)));
        assert_eq!(
            checks[&(server, 3, "kyle".to_owned())].claimed_name,
            "^2Sol"
        );
    }

    #[test]
    fn notes_go_to_the_hub_and_their_pictures_follow() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let note = WorldNote {
            text: "too shiny".to_owned(),
            shader: "textures/vjun/newfloor_vjun".to_owned(),
            ..WorldNote::default()
        };
        // Before registering, a note is refused and its picture goes nowhere.
        worker.handle(Command::Note(1, Box::new(note.clone())), t0);
        worker.handle(Command::NoteImage(1, vec![0; 4]), t0);
        assert!(!lock(&snapshot).note.clone().unwrap().sent);
        assert!(fake.log().is_empty());
        worker.handle(Command::Name("^1Sol".to_owned()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Note(2, Box::new(note)), t0);
        let outcome = lock(&snapshot).note.clone().unwrap();
        assert!(outcome.sent);
        assert_eq!((outcome.serial, outcome.message.as_str()), (2, "note #9"));
        // A picture for an unknown tag is dropped; the note's goes to its hub id once.
        worker.handle(Command::NoteImage(5, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        let log = fake.log();
        assert_eq!(
            log.iter()
                .filter(|line| line.starts_with("image"))
                .collect::<Vec<_>>(),
            ["image 9 4 bytes"]
        );
        assert!(log.contains(&"note too shiny textures/vjun/newfloor_vjun".to_owned()));
        // The note carries the name the player wears.
        assert!(log.contains(&"note name ^1Sol".to_owned()), "{log:?}");
    }

    #[test]
    fn bug_reports_go_to_the_hub_once_registered() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let report = BugReport {
            text: "The door flickers".into(),
            ..BugReport::default()
        };
        // Before registering, nothing is sent and the outcome says why.
        worker.handle(Command::Report(report.clone()), t0);
        let first = lock(&snapshot).report.clone().unwrap();
        assert!(!first.sent && first.message.contains("cl_identity 1"));
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Report(report), t0);
        assert!(fake.log().contains(&"report The door flickers".to_owned()));
        let outcome = lock(&snapshot).report.clone().unwrap();
        assert_eq!((outcome.serial, outcome.sent), (first.serial + 1, true));
        assert_eq!(outcome.message, "report #7");
    }

    #[test]
    fn a_refusal_before_the_registration_says_why() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _snapshot) = worker(&fake, t0);
        worker.handle(Command::Chat("hello".into()), t0);
        assert!(chat_outcome(&worker).message.contains("cl_identity 1"));
        // On, the registration not answered yet: nothing for the player to change.
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Chat("hello".into()), t0);
        let waiting = chat_outcome(&worker);
        assert!(
            !waiting.sent
                && waiting
                    .message
                    .starts_with("not connected to the hub yet (")
        );
        assert!(!waiting.message.contains("cl_identity"));
        // Refused here, never by the hub: no hub code in what anyone prints.
        let error = worker.not_connected();
        assert!(matches!(error, HubError::NotConnected(_)));
        assert!(
            error
                .to_string()
                .starts_with("not connected to the hub yet (")
        );
        // A failure is the last hub's: new settings start without it.
        assert!(matches!(lock(&_snapshot).status, Status::Failed(_)));
        worker.handle(Command::Configure(on("https://other-hub")), t0);
        assert!(!matches!(lock(&_snapshot).status, Status::Failed(_)));
    }

    #[test]
    fn chat_bans_go_only_from_a_verified_or_staff_key() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let ban = |worker: &Worker| lock_chat(&worker.chat).ban.clone().unwrap();
        worker.handle(Command::ChatBan("feedfacefeedface".into()), t0);
        assert!(matches!(
            ban(&worker).result,
            Err(HubError::NotConnected(_))
        ));
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        // Registered but not vouched for: nothing goes, and the outcome says why.
        worker.handle(Command::ChatBan("feedfacefeedface".into()), t0);
        let refused = ban(&worker);
        assert_eq!(refused.serial, 2);
        assert!(
            matches!(&refused.result, Err(HubError::Rejected { code, .. }) if code == "not_verified"),
            "{refused:?}"
        );
        assert!(!fake.log().iter().any(|line| line.starts_with("ban")));
        // Staff may, as verified players may.
        lock(&snapshot).me.as_mut().unwrap().staff = true;
        worker.handle(Command::ChatBan("feedfacefeedface".into()), t0);
        let banned = ban(&worker);
        assert_eq!(
            (banned.serial, banned.key_id.as_str(), banned.result.clone()),
            (
                3,
                "feedfacefeedface",
                Ok(ChatBan {
                    until: 1_000 + crate::ban::BAN_SECS,
                    address: true
                })
            )
        );
        lock(&snapshot).me.as_mut().unwrap().staff = false;
        lock(&snapshot).me.as_mut().unwrap().verified = true;
        worker.handle(Command::ChatBan("feedfacefeedface".into()), t0);
        assert!(ban(&worker).result.is_ok());
        assert_eq!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("ban"))
                .count(),
            2
        );
    }

    #[test]
    fn player_reports_go_only_from_a_verified_key() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let report = PlayerReport {
            category: crate::report::Category::Cheating,
            text: "Speed hacking all round".into(),
            slot: 5,
            target_name: "^1Troll".into(),
            ..PlayerReport::default()
        };
        worker.handle(Command::PlayerReport(report.clone()), t0);
        let first = lock(&snapshot).player_report.clone().unwrap();
        assert!(!first.sent && first.message.contains("cl_identity 1"));
        worker.handle(Command::Name("^2Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        // Registered but not verified: nothing goes, and the outcome says why.
        worker.handle(Command::PlayerReport(report.clone()), t0);
        let unverified = lock(&snapshot).player_report.clone().unwrap();
        assert!(!unverified.sent && unverified.message.contains("verified"));
        assert!(!fake.log().iter().any(|line| line.starts_with("player")));
        lock(&snapshot).me.as_mut().unwrap().verified = true;
        worker.handle(Command::PlayerReport(report), t0);
        let sent = lock(&snapshot).player_report.clone().unwrap();
        assert_eq!(
            (sent.serial, sent.sent, sent.message.as_str()),
            (3, true, "player report #11")
        );
        // It carries the name the reporter wears.
        assert!(
            fake.log()
                .contains(&"player cheating 5 ^1Troll by ^2Sol".to_owned()),
            "{:?}",
            fake.log()
        );
    }

    #[test]
    fn the_worn_name_goes_with_the_registration_and_again_when_it_changes() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(fake.log(), ["register ^1Sol"]);
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().name, "^1Sol");
        // The same name again sends nothing; an empty one is no name.
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.tick(t0);
        assert_eq!(fake.log().len(), 1);
        worker.handle(Command::Name("Fox".into()), t0);
        let wait = worker.tick(t0);
        assert_eq!(fake.log().last().unwrap(), "register Fox");
        assert_eq!(wait, IDLE_MAX, "nothing else is due");
        // A failure retries after the backoff, not at once.
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::Name("Wolf".into()), t0);
        worker.tick(t0);
        let count = fake.log().len();
        worker.tick(t0 + Duration::from_secs(1));
        assert_eq!(fake.log().len(), count);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(30));
        assert_eq!(fake.log().last().unwrap(), "register Wolf");
    }

    #[test]
    fn the_own_profile_is_read_again_now_and_then() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        let wait = worker.tick(t0);
        assert_eq!(fake.log(), ["register"]);
        assert!(wait <= PROFILE_EVERY);
        worker.tick(t0 + PROFILE_EVERY - Duration::from_secs(1));
        assert_eq!(fake.log().len(), 1, "not yet");
        worker.tick(t0 + PROFILE_EVERY);
        let own = format!("lookup {}", Identity::from_seed([1; 32]).key_id());
        assert_eq!(fake.log().last(), Some(&own));
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().name, "Other");
        worker.tick(t0 + PROFILE_EVERY + Duration::from_secs(1));
        assert_eq!(fake.log().len(), 2, "and then not for a while");
    }

    /// A hub that wants a newer SJK (`client_update_required`) is told as an update
    /// notice, in its own words when it sent some, by opening, the shop and revealing.
    #[test]
    fn a_hub_wanting_a_newer_sjk_reads_as_an_update_notice() {
        let refused = |message: &str| HubError::Rejected {
            status: 403,
            code: "client_update_required".to_owned(),
            message: message.to_owned(),
        };
        let words = "Update SJK to open holocrons: this needs SJK 2026.1022.1 or newer.";
        for read in [opening_refusal, trade_refusal, reveal_refusal] {
            assert_eq!(read(&refused(words)), words);
            assert_eq!(read(&refused("")), UPDATE_NOTICE);
        }
    }
}
