//! Talking to the hub: the [`Hub`] operations and their HTTPS implementation.

use crate::assets::{PACK_MAX, Pack};
use crate::ban::ChatBan;
use crate::crash::CrashReport;
use crate::keys::{Identity, random_bytes};
use crate::logs::{
    CrashDetail, CrashGroups, CrashSummaries, LogRequest, LogRequestRow, LogRequestRows,
    LogRequests, LogUpload,
};
use crate::report::{BugReport, PlayerReport, WorldNote};
use crate::staff::{ChatBanRows, StaffAnswer, StaffQuery, StaffRequest};
use crate::vitals::Sharing;
use crate::wire::{
    Achievement, Achievements, Feed, HolocronState, Look, OpenAnswer, Players, Presence, Profile,
    RelayBatch, SharedVitals, authorization,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Longest the hub may take to answer.
const TIMEOUT: Duration = Duration::from_secs(10);
/// Largest answer read, in bytes.
const ANSWER_MAX: u64 = 256 * 1024;
/// Largest answer to a staff query: one request's logs, up to 2 000 000 characters of
/// any script, escaped in JSON.
const STAFF_ANSWER_MAX: u64 = 16 * 1024 * 1024;
/// Longest a log upload may take (up to 4 MiB on a slow line).
const LOGS_TIMEOUT: Duration = Duration::from_secs(120);
/// Longest a pack's download may take (up to [`PACK_MAX`] bytes).
const ASSET_TIMEOUT: Duration = Duration::from_secs(120);

/// Why a hub request failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HubError {
    /// The hub could not be reached.
    Network(String),
    /// The hub answered with an error.
    Rejected {
        /// HTTP status.
        status: u16,
        /// The hub's machine-readable code (`name_taken`, `slot_taken`, ...).
        code: String,
        /// The hub's text.
        message: String,
    },
    /// The answer was not what `PROTOCOL.md` describes.
    Protocol(String),
    /// Nothing was sent: the client is not connected to a hub (the identity is off,
    /// there is no hub, or the registration has not gone through yet), for this reason.
    NotConnected(String),
}

impl std::fmt::Display for HubError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(why) => write!(f, "cannot reach the hub: {why}"),
            Self::Rejected { message, code, .. } => {
                write!(f, "the hub refused it ({code}): {message}")
            }
            Self::Protocol(why) => write!(f, "unexpected answer from the hub: {why}"),
            Self::NotConnected(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for HubError {}

/// What `GET /v1/presence` answered ([`Hub::presence_with_blocks`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PresenceRead {
    /// The live claims on game servers, one per SJK player seen there.
    pub players: Vec<Presence>,
    /// The players SJK staff blocked for everyone on a server until its map changes.
    pub blocks: Vec<crate::MapBlock>,
    /// The hub's map edits stamp, which grows with every change to any map's edits.
    pub map_edits: Option<i64>,
    /// The maps on which SJK's creator blocked server remaps for every SJK client
    /// (`PROTOCOL.md`, "Remap blocks"); `None` from a hub before them.
    pub remap_blocks: Option<Vec<String>>,
    /// The hub's Toy Master stamp for this server (since 03/11/2026): when it changes,
    /// the toys listed for it changed (one placed or removed, SJK's creator came or went).
    pub secrets: Option<i64>,
    /// The Toy Master's shows under way on this server.
    pub shows: Vec<crate::toy_master::ShowUnderWay>,
}
/// What the client asks of a hub (`PROTOCOL.md`, "Endpoints").
pub trait Hub: Send {
    /// Register the identity's key, or fetch its profile if it is known, telling
    /// the hub the in-game `name` the player wears, which it keeps in the key's
    /// name history.
    fn register(&mut self, identity: &Identity, name: Option<&str>) -> Result<Profile, HubError>;
    /// Set the identity's bio (the name is the one worn in game).
    fn set_bio(&mut self, identity: &Identity, bio: &str) -> Result<Profile, HubError>;
    /// Send the counts the client keeps for the achievements it counts (`id` to
    /// count); the hub answers with the key's achievements as it now holds them.
    fn set_achievements(
        &mut self,
        _identity: &Identity,
        _progress: &BTreeMap<String, u64>,
    ) -> Result<Vec<Achievement>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send achievements".to_owned(),
        ))
    }
    /// A staff request signed by the identity (`PROTOCOL.md`, "Staff"): the players a
    /// search found, or the changed key's profile.
    fn staff(
        &mut self,
        _identity: &Identity,
        _request: &StaffRequest,
    ) -> Result<Vec<Profile>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send staff requests".to_owned(),
        ))
    }
    /// A staff search (`StaffRequest::Search` or `SearchIn`): the players found and where
    /// those in a game are playing. By default the players [`Hub::staff`] answers, playing
    /// nowhere.
    fn staff_search(
        &mut self,
        identity: &Identity,
        request: &StaffRequest,
    ) -> Result<crate::staff::Found, HubError> {
        Ok(crate::staff::Found {
            players: self.staff(identity, request)?,
            playing: Vec::new(),
        })
    }
    /// The maps the hub keeps edits for (`PROTOCOL.md`, "Map edits").
    fn map_edits(
        &mut self,
        _identity: &Identity,
    ) -> Result<Vec<crate::map_edits::Entry>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read map edits".to_owned(),
        ))
    }
    /// One map's edits for one BSP checksum, as the hub answers them.
    fn map_edits_doc(
        &mut self,
        _identity: &Identity,
        _map: &str,
        _checksum: i32,
    ) -> Result<Value, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read map edits".to_owned(),
        ))
    }
    /// Staff: keep `edits` for `map` and `checksum` (none deletes them), answering the
    /// hub's new revision (0 when deleted).
    fn publish_map_edits(
        &mut self,
        _identity: &Identity,
        _map: &str,
        _checksum: i32,
        _edits: &Value,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send map edits".to_owned(),
        ))
    }
    /// A staff query signed by the identity (`PROTOCOL.md`, "Log requests"): asking a
    /// player for their logs, reading the requests and their logs, and crash reports.
    fn staff_query(
        &mut self,
        _identity: &Identity,
        _query: &StaffQuery,
    ) -> Result<StaffAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send staff queries".to_owned(),
        ))
    }
    /// The SJK team's open requests for the identity's logs, oldest first.
    fn log_requests(&mut self, _identity: &Identity) -> Result<Vec<LogRequest>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read log requests".to_owned(),
        ))
    }
    /// Send `upload` (already [`LogUpload::fitted`]) for request `id`.
    fn send_logs(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _upload: &LogUpload,
    ) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not send logs".to_owned(),
        ))
    }
    /// Send `upload` (already [`LogUpload::fitted`]) unasked, with the player's `note`;
    /// the hub answers with the id it keeps them under (`PROTOCOL.md`, "Log requests").
    fn send_own_logs(
        &mut self,
        _identity: &Identity,
        _upload: &LogUpload,
        _note: &str,
    ) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send logs".to_owned(),
        ))
    }
    /// Decline request `id`.
    fn decline_logs(&mut self, _identity: &Identity, _id: i64) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not send logs".to_owned(),
        ))
    }
    /// Send an SJK chat message (already checked by [`crate::chat::check`]) signed by
    /// the identity, with the in-game `name` worn (may be empty), in `channel` (the SJK
    /// chat or a clan chat, [`crate::channels`]); the hub answers with its id.
    fn chat(
        &mut self,
        _identity: &Identity,
        _text: &str,
        _name: &str,
        _channel: crate::HubChannel,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send chat".to_owned(),
        ))
    }
    /// Set the signed identity's reaction on an existing chat message.
    fn react(
        &mut self,
        _identity: &Identity,
        _message: u64,
        _emoji: &str,
        _active: bool,
    ) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "This hub client does not support chat reactions".into(),
        ))
    }
    /// Ban the player of `key_id` from the SJK chat for 24 hours (`POST /v1/chat/ban`):
    /// only a verified or staff identity may, and only a player the hub does not vouch
    /// for ([`crate::ban`]).
    fn chat_ban(&mut self, _identity: &Identity, _key_id: &str) -> Result<ChatBan, HubError> {
        Err(HubError::Protocol(
            "this hub client does not ban from the chat".to_owned(),
        ))
    }
    /// Play `emote` for the slot the identity claims on `server`, in step with the
    /// player in slot `sync` when it is set, without its sound when `quiet`.
    fn emote(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _emote: &str,
        _sync: Option<u8>,
        _quiet: bool,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send emotes".to_owned(),
        ))
    }
    /// Mark `pos` (whole game units) for `scope` from the slot the identity claims on
    /// `server` (`PROTOCOL.md`, "Pings"); the hub answers with the ping's feed id.
    fn ping(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _pos: [i32; 3],
        _scope: &str,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send pings".to_owned(),
        ))
    }
    /// Tell the SJK players on `server` that the identity's toy `toy` did `event` to
    /// the player in slot `target` (`PROTOCOL.md`, "Toy events"); the hub answers with
    /// the event's feed id.
    fn toy_event(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _toy: &str,
        _event: &str,
        _target: u8,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send toy events".to_owned(),
        ))
    }
    /// Relay `event` of the identity's toy `toy` to the player in slot `target` of
    /// `server` at once (`PROTOCOL.md`, "Toy relay"); the hub answers with its relay id.
    fn relay_send(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _toy: &str,
        _event: &str,
        _target: u8,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not relay toy events".to_owned(),
        ))
    }
    /// The relayed toy events after `after` for the identity's own slot on `server`,
    /// waiting up to `wait` seconds for one (`PROTOCOL.md`, "Toy relay"); with
    /// `whispers`, also the encrypted whispers sealed to the identity ("Whispers").
    fn relay_read(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _after: u64,
        _wait: u64,
        _whispers: bool,
    ) -> Result<RelayBatch, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the toy relay".to_owned(),
        ))
    }
    /// Send the encrypted whisper `boxed`, sealed to the key `to`, to the player in slot
    /// `target` of `server` (`PROTOCOL.md`, "Whispers"); the hub answers with its id.
    fn whisper_send(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _target: u8,
        _to: &str,
        _boxed: &str,
    ) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send whispers".to_owned(),
        ))
    }
    /// Tell the hub which player models the game has (`PROTOCOL.md`, "Models"); the
    /// hub answers how many names it kept.
    fn set_models(&mut self, _identity: &Identity, _models: &[String]) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send models".to_owned(),
        ))
    }
    /// Whether the player claiming `slot` on `server` has `model` (`PROTOCOL.md`,
    /// "Models"): their claimed name and the answer, `None` when they sent no list.
    fn model_check(
        &mut self,
        _server: &str,
        _slot: u8,
        _model: &str,
    ) -> Result<(String, Option<bool>), HubError> {
        Err(HubError::Protocol(
            "this hub client does not check models".to_owned(),
        ))
    }
    /// Share (or stop sharing) the player's vitals on `server` and read the sharers
    /// there (`PROTOCOL.md`, "Vitals").
    fn vitals(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _sharing: &Sharing,
    ) -> Result<Vec<SharedVitals>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not share vitals".to_owned(),
        ))
    }
    /// Wear `look` for the claim the identity holds on `server` (`PROTOCOL.md`,
    /// "Looks"); the hub answers with the feed id of its event.
    fn look(&mut self, _identity: &Identity, _server: &str, _look: &Look) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send looks".to_owned(),
        ))
    }
    /// What is new after `after` (chat, the clan chats `channels`, and the emotes and
    /// looks of `server`), waiting up to `wait` seconds at the hub for something to come.
    fn feed(
        &mut self,
        _identity: &Identity,
        _after: u64,
        _server: Option<&str>,
        _channels: &[crate::HubChannel],
        _wait: u64,
    ) -> Result<Feed, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the feed".to_owned(),
        ))
    }
    /// The SJK team's newest announcements (`GET /v1/announcements`, 25/10/2026), newest
    /// first.
    fn announcements(
        &mut self,
        _identity: &Identity,
    ) -> Result<Vec<crate::news::Announcement>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read announcements".to_owned(),
        ))
    }
    /// The person's rewards and the newest reward rounds (`GET /v1/rewards`, 25/10/2026).
    fn rewards(&mut self, _identity: &Identity) -> Result<crate::news::Rewards, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read rewards".to_owned(),
        ))
    }
    /// A one-use link that signs the browser in to the players' site
    /// (`POST /v1/site/sign-in`, 09/11/2026).
    fn site_sign_in(&mut self, _identity: &Identity) -> Result<crate::site::SignInLink, HubError> {
        Err(HubError::Protocol(
            "this hub client has no players' site".to_owned(),
        ))
    }
    /// Type a link code the players' site made (`POST /v1/site/link`, 09/11/2026): the
    /// account it joins this key to, and with `confirm` the key joins it.
    fn site_link(
        &mut self,
        _identity: &Identity,
        _code: &str,
        _confirm: bool,
    ) -> Result<crate::site::Linked, HubError> {
        Err(HubError::Protocol(
            "this hub client has no players' site".to_owned(),
        ))
    }
    /// Claim reward `id` (`POST /v1/rewards/claim`, 25/10/2026): the hub grants it and
    /// answers the person's profile and rewards after it.
    fn claim_reward(
        &mut self,
        _identity: &Identity,
        _id: u64,
    ) -> Result<crate::news::ClaimAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not claim rewards".to_owned(),
        ))
    }
    /// The SJK team's announcement (`POST /v1/staff/announce`, 31/10/2026; the
    /// creator's `POST /v1/creator/announce` on an older hub), checked by
    /// [`crate::news::check`] first; the hub answers it as stored.
    fn announce(
        &mut self,
        _identity: &Identity,
        _request: &crate::news::AnnounceRequest,
    ) -> Result<crate::news::Announcement, HubError> {
        Err(HubError::Protocol(
            "this hub client does not announce".to_owned(),
        ))
    }
    /// Store a news picture for an announcement (`PUT /v1/staff/news-image`): `bytes`,
    /// a PNG or a JPEG as `mime` says.
    fn upload_news_image(
        &mut self,
        _identity: &Identity,
        _bytes: &[u8],
        _mime: &str,
    ) -> Result<crate::news::NewsImage, HubError> {
        Err(HubError::Protocol(
            "this hub client does not upload news pictures".to_owned(),
        ))
    }
    /// News picture `id` as the hub serves it (`GET /v1/news-image/<id>`).
    fn news_image(&mut self, _id: &str) -> Result<Vec<u8>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read news pictures".to_owned(),
        ))
    }
    /// Make `png` (already cropped and scaled, `crate::avatar`) the identity's picture;
    /// the hub answers with the key's profile, whose `avatar` is the new version.
    fn set_avatar(&mut self, _identity: &Identity, _png: &[u8]) -> Result<Profile, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send pictures".to_owned(),
        ))
    }
    /// Take the identity's picture down; the hub answers with the key's profile.
    fn remove_avatar(&mut self, _identity: &Identity) -> Result<Profile, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send pictures".to_owned(),
        ))
    }
    /// Equip a profile banner, or empty to unequip; the hub validates ownership.
    fn set_banner(&mut self, _identity: &Identity, _id: &str) -> Result<Profile, HubError> {
        Err(HubError::Protocol(
            "profile banners need an updated hub".to_owned(),
        ))
    }
    /// Set the identity's name emoji (`crate::name_emoji`, checked; empty takes it
    /// away), leaving its bio and name as they are; the hub answers with the key's
    /// profile.
    fn set_emoji(&mut self, _identity: &Identity, _emoji: &str) -> Result<Profile, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send name emoji".to_owned(),
        ))
    }
    /// The PNG of `version` of `key_id`'s picture, as the hub serves it.
    fn avatar(&mut self, _key_id: &str, _version: &str) -> Result<Vec<u8>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read pictures".to_owned(),
        ))
    }
    /// The asset packs the hub serves (`PROTOCOL.md`, "Assets"), sorted by name.
    fn assets(&mut self, _identity: &Identity) -> Result<Vec<Pack>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read assets".to_owned(),
        ))
    }
    /// The bytes of asset pack `name`, at most [`PACK_MAX`].
    fn asset(&mut self, _identity: &Identity, _name: &str) -> Result<Vec<u8>, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read assets".to_owned(),
        ))
    }
    /// Any player's public profile.
    fn profile(&mut self, key_id: &str) -> Result<Profile, HubError>;
    /// Say the identity's player is in `slot` of `server` as `name`. `active` says the
    /// player is playing and not idle (`PROTOCOL.md`, "Holocrons"): the hub counts the
    /// time since the last claim toward the key's next holocron only then. A hub
    /// without holocrons ignores it.
    fn claim(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
        active: bool,
    ) -> Result<(), HubError>;
    /// [`Hub::claim`] with what the player did since the last accepted claim (`play`,
    /// 03/11/2026: the hub counts the time by real play), sent only with `active`. A hub
    /// client that does not send signals claims without them.
    fn claim_with(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
        active: bool,
        _play: Option<&crate::wire::PlaySignals>,
    ) -> Result<(), HubError> {
        self.claim(identity, server, slot, name, active)
    }
    /// How far the identity's key is from its next holocron (`GET /v1/holocrons`).
    fn holocrons(&mut self, _identity: &Identity) -> Result<HolocronState, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read holocrons".to_owned(),
        ))
    }
    /// Open the identity's holocrons `ids` (1 to [`crate::wire::OPEN_MAX`], its own, not
    /// opened yet), all or none (`POST /v1/holocrons/open`): each brings a sealed card and
    /// credits. The hub answers what each brought and the person's profile after.
    fn open_holocrons(
        &mut self,
        _identity: &Identity,
        _ids: &[u64],
    ) -> Result<OpenAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not open holocrons".to_owned(),
        ))
    }

    /// Reveal the identity's sealed card copies whose rarity has cards in the hub's
    /// catalogue (`POST /v1/cards/reveal`). The hub answers what each became and the
    /// person's profile after.
    fn reveal_cards(
        &mut self,
        _identity: &Identity,
    ) -> Result<crate::wire::RevealAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not reveal cards".to_owned(),
        ))
    }

    /// Shatter one copy of shader `id` into kyber shards (`POST /v1/unlocks/shatter`,
    /// 17/10/2026): never the last copy unless `last` (21/10/2026, hubs whose economy says
    /// `shatter_last`).
    fn shatter_unlock(
        &mut self,
        _identity: &Identity,
        _id: &str,
        _last: bool,
    ) -> Result<crate::wire::ShatterAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not shatter shaders".to_owned(),
        ))
    }

    /// Shatter every duplicate, one copy of each kept (`POST
    /// /v1/unlocks/shatter-duplicates`, 21/10/2026).
    fn shatter_duplicates(
        &mut self,
        _identity: &Identity,
    ) -> Result<crate::wire::ShatterAllAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not shatter shaders".to_owned(),
        ))
    }

    /// Shatter the copies `items` names, several shaders at once, last copies too when
    /// `last` (`POST /v1/unlocks/shatter-many`, 05/11/2026, hubs whose economy says
    /// `shatter_many`). A refusal comes with the shader the hub stopped at, when it said.
    fn shatter_many(
        &mut self,
        _identity: &Identity,
        _items: &[(String, u32)],
        _last: bool,
    ) -> Result<crate::wire::ShatterAllAnswer, (HubError, Option<String>)> {
        Err((
            HubError::Protocol("this hub client does not shatter shaders".to_owned()),
            None,
        ))
    }

    /// Craft shader `id` with kyber shards (`POST /v1/unlocks/craft`, 17/10/2026).
    fn craft_unlock(
        &mut self,
        _identity: &Identity,
        _id: &str,
    ) -> Result<crate::wire::CraftAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not craft shaders".to_owned(),
        ))
    }

    /// Fuse duplicates into a shader of the next tier the person lacks (`POST
    /// /v1/unlocks/fuse`, 10/11/2026, hubs whose economy says `fuse`): `ids` names each
    /// shader once per copy given, all of one tier.
    fn fuse_unlocks(
        &mut self,
        _identity: &Identity,
        _ids: &[String],
    ) -> Result<crate::wire::FuseAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not fuse shaders".to_owned(),
        ))
    }

    /// Replace the person's wishlist with `ids`, up to three saber shaders (`PUT
    /// /v1/shop/wishlist`, 10/11/2026, hubs whose economy sends `wishlist`).
    fn set_wishlist(
        &mut self,
        _identity: &Identity,
        _ids: &[String],
    ) -> Result<crate::wire::WishlistAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not keep wishlists".to_owned(),
        ))
    }

    /// Break one copy of a tag down into kyber shards (`POST /v1/tags/shatter`,
    /// 03/11/2026, hubs whose economy says `tag_copies`).
    fn shatter_tag(
        &mut self,
        _identity: &Identity,
        _copy: &crate::wire::TagCopy,
    ) -> Result<crate::wire::TagShatterAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not break tags down".to_owned(),
        ))
    }

    /// Give two tag copies of one tier and finish for a tag not held yet (`POST
    /// /v1/tags/craft`, 03/11/2026).
    fn craft_tags(
        &mut self,
        _identity: &Identity,
        _copies: &[crate::wire::TagCopy; 2],
    ) -> Result<crate::wire::TagCraftAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not craft tags".to_owned(),
        ))
    }

    /// Spend a blank tag on GIPHY GIF `giphy`, a GIF tag for good (`POST /v1/tags/gif`,
    /// 10/11/2026, verified players on hubs whose profiles list `gif_tags`).
    fn make_gif_tag(
        &mut self,
        _identity: &Identity,
        _giphy: &str,
    ) -> Result<crate::wire::GifTagAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not make GIF tags".to_owned(),
        ))
    }

    /// Set (`on`) or clear `mark` on saber shader `id` (`POST /v1/unlocks/marks`,
    /// 29/10/2026, hubs whose economy says `marks`): the hub answers every mark after.
    fn set_mark(
        &mut self,
        _identity: &Identity,
        _id: &str,
        _mark: crate::wire::Mark,
        _on: bool,
    ) -> Result<crate::wire::UnlockMarks, HubError> {
        Err(HubError::Protocol(
            "this hub client does not keep collection marks".to_owned(),
        ))
    }

    /// The shop's offers of the day (`GET /v1/shop`, 17/10/2026).
    fn shop(&mut self, _identity: &Identity) -> Result<crate::wire::ShopDay, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the shop".to_owned(),
        ))
    }

    /// Buy today's offer `id` (`POST /v1/shop/buy`, 17/10/2026).
    fn buy_offer(
        &mut self,
        _identity: &Identity,
        _id: &str,
    ) -> Result<crate::wire::BuyAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not buy".to_owned(),
        ))
    }

    /// Whether this key may sell fished junk, the prices and what is still payable
    /// today (`GET /v1/fishing/junk`, 03/11/2026).
    fn junk_status(&mut self, _identity: &Identity) -> Result<crate::junk::JunkStatus, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read junk selling".to_owned(),
        ))
    }

    /// Sell `held`, pieces of fished junk by id (`POST /v1/fishing/junk/sell`,
    /// 03/11/2026): the hub answers what it took.
    fn sell_junk(
        &mut self,
        _identity: &Identity,
        _held: &std::collections::BTreeMap<String, u32>,
    ) -> Result<crate::junk::JunkSold, HubError> {
        Err(HubError::Protocol(
            "this hub client does not sell junk".to_owned(),
        ))
    }

    /// Every placed mystery box, the price and the odds (`GET /v1/mystery-boxes`,
    /// 27/10/2026).
    fn mystery_boxes(
        &mut self,
        _identity: &Identity,
    ) -> Result<crate::mystery_box::Boxes, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read mystery boxes".to_owned(),
        ))
    }

    /// Open mystery box `id`, which the player stands at on `server`
    /// (`POST /v1/mystery-box/open`).
    fn open_mystery_box(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _server: &str,
    ) -> Result<crate::mystery_box::OpenAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not open mystery boxes".to_owned(),
        ))
    }

    /// The box hunt on `server` as the player reads it (`POST /v1/box-hunt`), saying whether the player is
    /// `away` (in the chat, a menu, idle: the hub picks no mark among them).
    fn box_hunt(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _away: bool,
    ) -> Result<crate::mystery_box::HuntState, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the box hunt".to_owned(),
        ))
    }

    /// The player, the box hunt's mark on `server`, was killed by the player on slot
    /// `by` (`POST /v1/box-hunt/killed`); whether the hub counted it.
    fn box_hunt_killed(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _by: u8,
    ) -> Result<bool, HubError> {
        Err(HubError::Protocol(
            "this hub client does not report box hunt kills".to_owned(),
        ))
    }

    /// The quests on `server` as the player reads them (`POST /v1/quests`), saying
    /// whether the player is `away` (in the chat, a menu, idle: the hub gives them none).
    fn quests(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _away: bool,
    ) -> Result<crate::quests::QuestState, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read quests".to_owned(),
        ))
    }

    /// The player, a target of the quest on `server`, was hugged or kissed as it asks
    /// by the player on slot `by` (`POST /v1/quests/done`); whether the hub counted it.
    fn quest_done(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _by: u8,
    ) -> Result<bool, HubError> {
        Err(HubError::Protocol(
            "this hub client does not report quests".to_owned(),
        ))
    }

    /// The player is out of the box hunt's chase on `server` (`POST /v1/box-hunt/out`):
    /// killed by the mark, caught cheating (a teleport, noclip), or as the mark typing in
    /// the chat too long; whether the hub counted it.
    fn box_hunt_out(
        &mut self,
        _identity: &Identity,
        _server: &str,
        _out: crate::mystery_box::HuntOut,
    ) -> Result<bool, HubError> {
        Err(HubError::Protocol(
            "this hub client does not report the box hunt's chase".to_owned(),
        ))
    }

    /// The WHEEL OF DOOM's price, cooldown and segments (`GET /v1/wheel-of-doom`,
    /// 07/11/2026).
    fn wheel_of_doom(
        &mut self,
        _identity: &Identity,
    ) -> Result<crate::wheel_of_doom::Wheel, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the wheel of doom".to_owned(),
        ))
    }

    /// Spin the WHEEL OF DOOM, the player on `server` (`POST /v1/wheel-of-doom/spin`).
    fn spin_wheel_of_doom(
        &mut self,
        _identity: &Identity,
        _server: &str,
    ) -> Result<crate::wheel_of_doom::SpinAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not spin the wheel of doom".to_owned(),
        ))
    }

    /// SJK's creator: place a mystery box (`POST /v1/creator/mystery-box`); answers its id.
    fn place_mystery_box(
        &mut self,
        _identity: &Identity,
        _placed: &crate::mystery_box::MysteryBox,
    ) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not place mystery boxes".to_owned(),
        ))
    }

    /// SJK's creator: remove mystery box `id` (`POST /v1/creator/mystery-box/remove`).
    fn remove_mystery_box(&mut self, _identity: &Identity, _id: i64) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not remove mystery boxes".to_owned(),
        ))
    }

    /// Every toy the Toy Master hid (`GET /v1/secrets`, 30/10/2026), with the ones shown
    /// only where SJK's creator is when the player is on `server` with them (03/11/2026).
    fn secrets(
        &mut self,
        _identity: &Identity,
        _server: Option<&str>,
    ) -> Result<crate::toy_master::Secrets, HubError> {
        Err(HubError::Protocol(
            "this hub client does not read the Toy Master's toys".to_owned(),
        ))
    }

    /// The player pressed Use `presses` times on toy `id` on `server` in map load `round`
    /// (`POST /v1/secrets/use`).
    fn use_secret(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _server: &str,
        _round: i32,
        _presses: u32,
    ) -> Result<crate::toy_master::Used, HubError> {
        Err(HubError::Protocol(
            "this hub client does not use the Toy Master's toys".to_owned(),
        ))
    }

    /// Take drop `drop` of toy `id`'s show on `server` in map load `round`
    /// (`POST /v1/secrets/take`).
    fn take_secret_drop(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _server: &str,
        _round: i32,
        _drop: u32,
    ) -> Result<crate::toy_master::Took, HubError> {
        Err(HubError::Protocol(
            "this hub client does not take the Toy Master's drops".to_owned(),
        ))
    }

    /// SJK's creator: hide a toy (`POST /v1/creator/secret`); answers its id.
    fn place_secret(
        &mut self,
        _identity: &Identity,
        _placing: &crate::toy_master::Placing,
    ) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not hide toys".to_owned(),
        ))
    }

    /// SJK's creator: take toy `id` away (`POST /v1/creator/secret/remove`).
    fn remove_secret(&mut self, _identity: &Identity, _id: i64) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not remove toys".to_owned(),
        ))
    }

    /// SJK's creator: show toy `id` only where the creator is, or everywhere again
    /// (`POST /v1/creator/secret/escort`, 03/11/2026).
    fn escort_secret(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _escorted: bool,
    ) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not escort toys".to_owned(),
        ))
    }

    /// Buy a holocron of tier `tier` with credits (`POST /v1/holocrons/buy`, 17/10/2026).
    fn buy_holocron(
        &mut self,
        _identity: &Identity,
        _tier: &str,
    ) -> Result<crate::wire::HolocronBuyAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not buy holocrons".to_owned(),
        ))
    }

    /// A page of the player's history (`GET /v1/history`, 31/10/2026): the newest, or
    /// the one before `before` (unix seconds, [`crate::HistoryPage::before`]).
    fn history(
        &mut self,
        _identity: &Identity,
        _before: Option<i64>,
    ) -> Result<crate::history::HistoryPage, HubError> {
        Err(HubError::Protocol(
            "status 404: this hub client does not read the history".to_owned(),
        ))
    }

    /// The player's trades, what they traded today and the rules (`GET /v1/trades`,
    /// 22/10/2026).
    fn trades(&mut self, _identity: &Identity) -> Result<crate::trades::TradesAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not trade".to_owned(),
        ))
    }

    /// Ask the player whose hub id is `to` to trade (`POST /v1/trades`, 22/10/2026).
    fn trade_ask(
        &mut self,
        _identity: &Identity,
        _to: &str,
    ) -> Result<crate::trades::TradeAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not trade".to_owned(),
        ))
    }

    /// Take step `step` in trade `id` (`POST /v1/trades/<id>/<action>`, 22/10/2026).
    fn trade_step(
        &mut self,
        _identity: &Identity,
        _id: i64,
        _step: &crate::trades::TradeStep,
    ) -> Result<crate::trades::TradeAnswer, HubError> {
        Err(HubError::Protocol(
            "this hub client does not trade".to_owned(),
        ))
    }
    /// Withdraw the identity's claim on `server`.
    fn release(&mut self, identity: &Identity, server: &str) -> Result<(), HubError>;
    /// The live claims on `server`.
    fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError>;
    /// The live claims on `server`, the players staff blocked there for everyone
    /// (`PROTOCOL.md`, "Map blocks"; hubs before map blocks give none) and the hub's map
    /// edits stamp (`PROTOCOL.md`, "Map edits"; `None` from hubs before it).
    fn presence_with_blocks(&mut self, server: &str) -> Result<PresenceRead, HubError> {
        self.presence(server).map(|players| PresenceRead {
            players,
            ..PresenceRead::default()
        })
    }
    /// Send a world note signed by the identity; the hub answers with its number.
    fn note(&mut self, _identity: &Identity, _note: &WorldNote) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send notes".to_owned(),
        ))
    }
    /// Attach a JPEG to note `id`, signed by the identity that sent the note.
    fn note_image(&mut self, _identity: &Identity, _id: i64, _jpeg: &[u8]) -> Result<(), HubError> {
        Err(HubError::Protocol(
            "this hub client does not send pictures".to_owned(),
        ))
    }
    /// Send a bug report signed by the identity; the hub answers with its number.
    fn report(&mut self, _identity: &Identity, _report: &BugReport) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send reports".to_owned(),
        ))
    }
    /// Send anonymous settings usage, unsigned (`PROTOCOL.md`, "Settings usage"):
    /// `body` is [`crate::usage::body`]'s. Answers how many events the hub kept.
    fn settings_usage(&mut self, _body: &Value) -> Result<u64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send settings usage".to_owned(),
        ))
    }
    /// Send a crash report (already [`CrashReport::fitted`]) signed by the identity;
    /// the hub answers with its number.
    fn crash(&mut self, _identity: &Identity, _report: &CrashReport) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send crash reports".to_owned(),
        ))
    }
    /// Send a report about another player, signed by the identity (a verified key);
    /// the hub answers with its number.
    fn player_report(
        &mut self,
        _identity: &Identity,
        _report: &PlayerReport,
    ) -> Result<i64, HubError> {
        Err(HubError::Protocol(
            "this hub client does not send player reports".to_owned(),
        ))
    }
}

/// Whether `url` may be used as the hub's address: `https://host[:port]`, or
/// plain `http://` only for this machine (local testing). No path: requests are
/// signed with their path, which must be the one the hub sees.
pub fn valid_base_url(url: &str) -> bool {
    let (secure, authority) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return false;
    };
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    if authority.is_empty() || authority.contains(['/', '?', '#', '@', ' ']) {
        return false;
    }
    let host = match authority.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or_default(),
        None => authority.split(':').next().unwrap_or_default(),
    };
    !host.is_empty() && (secure || matches!(host, "localhost" | "127.0.0.1" | "::1"))
}

/// A JSON answer of `status`, read up to [`ANSWER_MAX`].
fn read_json(
    response: &mut ureq::http::Response<ureq::Body>,
    status: u16,
) -> Result<Value, HubError> {
    read_json_up_to(response, status, ANSWER_MAX)
}

/// [`read_json`] of an answer of at most `limit` bytes.
fn read_json_up_to(
    response: &mut ureq::http::Response<ureq::Body>,
    status: u16,
    limit: u64,
) -> Result<Value, HubError> {
    let text = response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_string()
        .map_err(|error| HubError::Network(error.to_string()))?;
    serde_json::from_str(&text)
        .map_err(|_| HubError::Protocol(format!("status {status}, not JSON")))
}

/// The error after the hub refused the timestamp twice.
fn clock_disagrees() -> HubError {
    HubError::Protocol("the hub's clock keeps disagreeing".to_owned())
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs() as i64)
}

/// The hub over HTTPS.
pub struct HttpHub {
    base: String,
    agent: ureq::Agent,
    /// Added to the local clock to get the hub's, learned from a `clock` refusal.
    clock_offset: i64,
    /// The shader the last refusal named (`id`, a `shatter-many` one since 05/11/2026).
    refused_id: Option<String>,
}

impl HttpHub {
    /// A client for the hub at `base_url` (`https://hub.example`, no trailing slash needed).
    pub fn new(base_url: &str, user_agent: &str) -> Result<Self, HubError> {
        Self::with_timeout(base_url, user_agent, TIMEOUT)
    }

    /// [`HttpHub::new`] whose requests may take `timeout` (the feed's long poll).
    pub fn with_timeout(
        base_url: &str,
        user_agent: &str,
        timeout: Duration,
    ) -> Result<Self, HubError> {
        let base = base_url.trim().trim_end_matches('/').to_owned();
        if !valid_base_url(&base) {
            return Err(HubError::Network(
                "the hub address must start with https://".to_owned(),
            ));
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .user_agent(user_agent)
            .http_status_as_error(false)
            .build()
            .into();
        Ok(Self {
            base,
            agent,
            clock_offset: 0,
            refused_id: None,
        })
    }

    /// One request, signed when `signer` is given. Retries once with the hub's
    /// clock when it refuses the timestamp.
    fn send(
        &mut self,
        signer: Option<&Identity>,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, HubError> {
        let body = body.map(|value| value.to_string()).unwrap_or_default();
        self.send_bytes(signer, method, path, body.as_bytes(), "application/json")
    }

    /// [`HttpHub::send`] with a raw body of `content_type`.
    fn send_bytes(
        &mut self,
        signer: Option<&Identity>,
        method: &str,
        path: &str,
        body: &[u8],
        content_type: &str,
    ) -> Result<Value, HubError> {
        self.send_with(signer, method, path, body, content_type, ANSWER_MAX, None)
    }

    /// [`HttpHub::send_bytes`] reading an answer of up to `limit` bytes, with its own
    /// `timeout` when given.
    #[allow(clippy::too_many_arguments)]
    fn send_with(
        &mut self,
        signer: Option<&Identity>,
        method: &str,
        path: &str,
        body: &[u8],
        content_type: &str,
        limit: u64,
        timeout: Option<Duration>,
    ) -> Result<Value, HubError> {
        for attempt in 0..2 {
            let url = format!("{}{path}", self.base);
            let header = signer
                .map(|identity| self.authorization(identity, method, path, body))
                .transpose()?;
            let mut response =
                self.call(method, &url, header.as_deref(), body, content_type, timeout)?;
            let status = response.status().as_u16();
            let value = read_json_up_to(&mut response, status, limit)?;
            if status < 400 {
                return Ok(value);
            }
            if let Some(refusal) = self.refusal(status, &value, attempt == 0) {
                return Err(refusal);
            }
        }
        Err(clock_disagrees())
    }

    /// The `Authorization` header of a request signed by `identity` now, by the hub's
    /// clock as far as it is known.
    fn authorization(
        &self,
        identity: &Identity,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<String, HubError> {
        let nonce = random_bytes::<12>().map_err(|error| HubError::Network(error.to_string()))?;
        Ok(authorization(
            identity,
            method,
            path,
            body,
            unix_now() + self.clock_offset,
            nonce,
        ))
    }

    /// An unsigned `GET` of `path` whose answer is bytes, at most `limit` of them; an
    /// error answer is the hub's JSON, as for [`HttpHub::send`].
    fn fetch(&mut self, path: &str, limit: u64) -> Result<Vec<u8>, HubError> {
        let url = format!("{}{path}", self.base);
        let mut response = self
            .agent
            .get(&url)
            .call()
            .map_err(|error| HubError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        if status < 400 {
            return response
                .body_mut()
                .with_config()
                .limit(limit)
                .read_to_vec()
                .map_err(|error| HubError::Network(error.to_string()));
        }
        let value = read_json(&mut response, status)?;
        Err(self
            .refusal(status, &value, false)
            .unwrap_or_else(clock_disagrees))
    }

    /// The error of an answer of `status` 400 or more carrying `value`; `None` when the
    /// hub refused the timestamp and `may_retry`: its clock is learned and the request
    /// should go again.
    fn refusal(&mut self, status: u16, value: &Value, may_retry: bool) -> Option<HubError> {
        let code = value["error"].as_str().unwrap_or("error").to_owned();
        self.refused_id = value["id"].as_str().map(str::to_owned);
        if code == "clock"
            && may_retry
            && let Some(server_time) = value["server_time"].as_i64()
        {
            self.clock_offset = server_time - unix_now();
            return None;
        }
        // A ban says when it ends; the player reads it in their own time.
        let message = match value["until"].as_i64() {
            Some(until) if code == "chat_banned" => crate::ban::banned_message(until),
            _ => value["message"].as_str().unwrap_or_default().to_owned(),
        };
        Some(HubError::Rejected {
            status,
            code,
            message,
        })
    }

    /// A signed `GET` of raw bytes, at most `limit` of them, with its own `timeout`
    /// (an asset pack). An error answer is read as [`HttpHub::send`] reads it,
    /// retrying once with the hub's clock.
    fn get_bytes(
        &mut self,
        identity: &Identity,
        path: &str,
        limit: u64,
        timeout: Duration,
    ) -> Result<Vec<u8>, HubError> {
        for attempt in 0..2 {
            let url = format!("{}{path}", self.base);
            let header = self.authorization(identity, "GET", path, b"")?;
            let mut response = self
                .agent
                .get(&url)
                .header("Authorization", header.as_str())
                .config()
                .timeout_global(Some(timeout))
                .build()
                .call()
                .map_err(|error| HubError::Network(error.to_string()))?;
            let status = response.status().as_u16();
            if status < 400 {
                // One byte over the limit is read, so a longer answer is an error.
                return match response
                    .body_mut()
                    .with_config()
                    .limit(limit + 1)
                    .read_to_vec()
                {
                    Ok(bytes) if bytes.len() as u64 <= limit => Ok(bytes),
                    Ok(_) | Err(ureq::Error::BodyExceedsLimit(_)) => {
                        Err(HubError::Protocol("pack too large".to_owned()))
                    }
                    Err(error) => Err(HubError::Network(error.to_string())),
                };
            }
            let value = read_json(&mut response, status)?;
            if let Some(refusal) = self.refusal(status, &value, attempt == 0) {
                return Err(refusal);
            }
        }
        Err(clock_disagrees())
    }

    fn call(
        &self,
        method: &str,
        url: &str,
        authorization: Option<&str>,
        body: &[u8],
        content_type: &str,
        timeout: Option<Duration>,
    ) -> Result<ureq::http::Response<ureq::Body>, HubError> {
        let network = |error: ureq::Error| HubError::Network(error.to_string());
        let with_header =
            |builder: ureq::RequestBuilder<ureq::typestate::WithBody>| match authorization {
                Some(value) => builder.header("Authorization", value),
                None => builder,
            };
        if let Some(timeout) = timeout {
            return with_header(self.agent.post(url))
                .header("Content-Type", content_type)
                .config()
                .timeout_global(Some(timeout))
                .build()
                .send(body)
                .map_err(network);
        }
        match method {
            "GET" => {
                let mut request = self.agent.get(url);
                if let Some(value) = authorization {
                    request = request.header("Authorization", value);
                }
                request.call().map_err(network)
            }
            "DELETE" => {
                let mut request = self.agent.delete(url).force_send_body();
                if let Some(value) = authorization {
                    request = request.header("Authorization", value);
                }
                request
                    .header("Content-Type", content_type)
                    .send(body)
                    .map_err(network)
            }
            "PUT" => with_header(self.agent.put(url))
                .header("Content-Type", content_type)
                .send(body)
                .map_err(network),
            _ => with_header(self.agent.post(url))
                .header("Content-Type", content_type)
                .send(body)
                .map_err(network),
        }
    }
}

/// A note's remaps and model (`PROTOCOL.md`, "World notes"), added to `body` only when
/// it has them: a hub from before them refuses the fields, and still takes a note
/// without them.
fn note_context(body: &mut Value, note: &WorldNote) {
    if !note.remapped.is_empty() {
        body["remapped"] = json!(note.remapped);
    }
    if !note.remaps.is_empty() {
        body["remaps"] = note
            .remaps
            .iter()
            .map(|remap| json!({"source": remap.source, "from": remap.from, "to": remap.to}))
            .collect();
    }
    if let Some(model) = &note.model {
        let mut value = json!({
            "path": model.path,
            "classname": model.classname,
            "shaders": model
                .shaders
                .iter()
                .map(|shader| json!({
                    "surface": shader.surface,
                    "shader": shader.shader,
                    "remapped": shader.remapped,
                }))
                .collect::<Vec<_>>(),
        });
        if let Some(entity) = model.entity {
            value["entity"] = json!(entity);
        }
        if let Some(origin) = model.origin {
            value["origin"] = json!(origin);
        }
        if !model.skin.is_empty() {
            value["skin"] = json!(model.skin);
        }
        body["model"] = value;
    }
}

/// `POST /v1/player-report`'s body: empty optional fields are left out.
fn player_report_body(report: &PlayerReport) -> Value {
    let mut body = json!({
        "category": report.category.code(),
        "text": report.text,
        "server": report.server,
        "slot": report.slot,
        "target_name": report.target_name,
        "map": report.map,
        "build": report.build,
    });
    for (key, value) in [
        ("target_key_id", &report.target_key_id),
        ("server_name", &report.server_name),
        ("name", &report.name),
    ] {
        if !value.is_empty() {
            body[key] = json!(value);
        }
    }
    if let Some(time) = report.level_time {
        body["level_time"] = json!(time);
    }
    if let Some(whisper) = &report.whisper {
        use base64::Engine;
        body["whisper"] = json!({
            "from": whisper.from,
            "time": whisper.time,
            "text": whisper.text,
            "signature": base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(whisper.signature),
        });
    }
    body
}

/// A game server's address as a query value: digits and dots as they are, the rest
/// (`:`, an IPv6 address's brackets) percent-encoded.
fn encode_server(server: &str) -> String {
    server
        .bytes()
        .map(|byte| match byte {
            b'0'..=b'9' | b'.' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// A model name for a query: letters, digits, `_`, `-` and `.` as they are, the rest
/// percent-encoded.
fn encode_model(model: &str) -> String {
    model
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// `GET /v1/feed`'s path and query: `channels` only when a clan chat is wanted, so a
/// hub from before clan chats sees the query it always did.
fn feed_path(
    after: u64,
    server: Option<&str>,
    channels: &[crate::HubChannel],
    wait: u64,
) -> String {
    let mut path = format!("/v1/feed?after={after}&wait={wait}");
    if let Some(server) = server {
        path.push_str("&server=");
        path.push_str(&encode_server(server));
    }
    if let Some(channels) = crate::channels::wanted_query(channels) {
        path.push_str("&channels=");
        path.push_str(&channels);
    }
    path
}

/// `POST /v1/look`'s body: exactly the server and the look's fields. `saber_off` goes only
/// when it names an option (and with a blade skin): a hub from before 11/10/2026 refuses
/// the field, so a look with every part on stays as it was. `toys` goes only when a toy
/// is out, for the same reason (hubs before 16/10/2026). `saber2` goes only when the
/// second saber wears another skin than the first, and `saber2_off` only when it names an
/// option of that skin (hubs before 21/10/2026). `saber_sounds` goes only when the player
/// chose sounds apart from their blade (hubs before it refuse the field, and the service
/// then sends the look without it). `saber_on` and `saber2_on` go only when they name an
/// extra of their saber's skin (hubs before 26/10/2026 refuse them, and the service then
/// sends the look without them). `tag` goes only while a tag is stuck (hubs before
/// 27/10/2026 refuse it, and the service then sends the look without it). `finisher` goes
/// only while one is worn (hubs before 04/11/2026 refuse it, and the service then sends
/// the look without it).
fn look_body(server: &str, look: &Look) -> Value {
    let mut body = json!({ "server": server, "saber": look.saber, "illuminate": look.illuminate });
    if !look.saber.is_empty() && !look.saber_off.is_empty() {
        body["saber_off"] = json!(look.saber_off);
    }
    if !look.saber.is_empty() && !look.saber_on.is_empty() {
        body["saber_on"] = json!(look.saber_on);
    }
    if !look.toys.is_empty() {
        body["toys"] = json!(look.toys);
    }
    if let Some(saber2) = look.saber2.as_ref() {
        body["saber2"] = json!(saber2);
        if !saber2.is_empty() && !look.saber2_off.is_empty() {
            body["saber2_off"] = json!(look.saber2_off);
        }
        if !saber2.is_empty() && !look.saber2_on.is_empty() {
            body["saber2_on"] = json!(look.saber2_on);
        }
    }
    if !look.saber_sounds.is_empty() {
        body["saber_sounds"] = json!(look.saber_sounds);
    }
    if let Some(tag) = look.tag.as_ref() {
        body["tag"] = json!(tag);
    }
    if !look.finisher.is_empty() {
        body["finisher"] = json!(look.finisher);
    }
    body
}

/// `POST /v1/claim`'s body. `active` is sent only when true: the field is optional and
/// absent means false, so a hub from before holocrons never sees an unknown field.
#[cfg(test)]
fn claim_body(server: &str, slot: u8, name: &str, active: bool) -> Value {
    claim_body_with(server, slot, name, active, None)
}

/// [`claim_body`] with the play signals (`"play"`, 03/11/2026), only beside `active`.
fn claim_body_with(
    server: &str,
    slot: u8,
    name: &str,
    active: bool,
    play: Option<&crate::wire::PlaySignals>,
) -> Value {
    let mut body = json!({"server": server, "slot": slot, "name": name});
    if active {
        body["active"] = json!(true);
        if let Some(play) = play {
            body["play"] = json!(play);
        }
    }
    body
}

/// `POST /v1/holocrons/open`'s path and body.
/// The body of a creator's announcement (`PROTOCOL.md`, "Announcements"): `version` only
/// with a release.
fn announce_body(request: &crate::news::AnnounceRequest) -> Value {
    if request.release {
        json!({
            "kind": "release",
            "title": request.title,
            "text": request.text,
            "version": request.version,
        })
    } else {
        json!({ "kind": "news", "title": request.title, "text": request.text })
    }
}

fn open_call(ids: &[u64]) -> (&'static str, Value) {
    ("/v1/holocrons/open", json!({ "ids": ids }))
}

/// A staff request's path and body.
fn staff_call(request: &StaffRequest) -> (&'static str, Value) {
    match request {
        StaffRequest::Search(query) => ("/v1/staff/search", json!({ "query": query })),
        // A hub from before 08/11/2026 refuses `scope`: a plain search sends none.
        StaffRequest::SearchIn { query, scope } => (
            "/v1/staff/search",
            match scope.wire() {
                (scope, Some(server)) => {
                    json!({ "query": query, "scope": scope, "server": server })
                }
                (scope, None) => json!({ "query": query, "scope": scope }),
            },
        ),
        StaffRequest::GiftUndo { id, .. } => ("/v1/staff/gift-undo", json!({ "id": id })),
        // Rank 1 sends no rank, as before ranks: a hub from before 28/10/2026 refuses one.
        StaffRequest::Award {
            key_id,
            medal,
            note,
            rank: 1,
        } => (
            "/v1/staff/award",
            json!({ "key_id": key_id, "medal": medal, "note": note }),
        ),
        StaffRequest::Award {
            key_id,
            medal,
            note,
            rank,
        } => (
            "/v1/staff/award",
            json!({ "key_id": key_id, "medal": medal, "note": note, "rank": rank }),
        ),
        StaffRequest::Unaward { key_id, medal } => (
            "/v1/staff/unaward",
            json!({ "key_id": key_id, "medal": medal }),
        ),
        StaffRequest::MedalRank {
            key_id,
            medal,
            rank,
        } => (
            "/v1/staff/medal-rank",
            json!({ "key_id": key_id, "medal": medal, "rank": rank }),
        ),
        StaffRequest::Unlock {
            key_id,
            unlock,
            note,
        } => (
            "/v1/staff/unlock",
            json!({ "key_id": key_id, "unlock": unlock, "note": note }),
        ),
        StaffRequest::Relock { key_id, unlock } => (
            "/v1/staff/relock",
            json!({ "key_id": key_id, "unlock": unlock }),
        ),
        StaffRequest::ClearAchievements { key_id, id } => (
            "/v1/staff/clear-achievements",
            json!({ "key_id": key_id, "id": id }),
        ),
        StaffRequest::ChatDelete { id } => ("/v1/staff/chat-delete", json!({ "id": id })),
        StaffRequest::ChatMute { key_id, muted } => (
            "/v1/staff/chat-mute",
            json!({ "key_id": key_id, "muted": muted }),
        ),
        StaffRequest::ChatUnban { key_id } => ("/v1/staff/chat-unban", json!({ "key_id": key_id })),
        StaffRequest::MapBlock {
            server,
            map,
            serverid,
            slot,
            name,
            blocked,
        } => (
            "/v1/staff/map-block",
            json!({
                "server": server, "map": map, "serverid": serverid,
                "slot": slot, "name": name, "blocked": blocked
            }),
        ),
        StaffRequest::RemapBlock { map, blocked } => (
            "/v1/staff/remap-block",
            json!({ "map": map, "blocked": blocked }),
        ),
        StaffRequest::AvatarRemove { key_id } => {
            ("/v1/staff/avatar-remove", json!({ "key_id": key_id }))
        }
        StaffRequest::AvatarBlock { key_id, blocked } => (
            "/v1/staff/avatar-block",
            json!({ "key_id": key_id, "blocked": blocked }),
        ),
        StaffRequest::HolocronGive { key_id, tier, note } => (
            "/v1/staff/holocron-give",
            json!({ "key_id": key_id, "tier": tier, "note": note }),
        ),
        StaffRequest::HolocronRemove { key_id, id } => (
            "/v1/staff/holocron-remove",
            json!({ "key_id": key_id, "id": id }),
        ),
        StaffRequest::Verify {
            key_id,
            verified,
            tick,
        } => (
            "/v1/staff/verify",
            match tick.and_then(crate::Tick::group_wire) {
                Some(tick) => json!({ "key_id": key_id, "verified": verified, "tick": tick }),
                None => json!({ "key_id": key_id, "verified": verified }),
            },
        ),
        StaffRequest::SetStaff { key_id, staff } => (
            "/v1/staff/staff",
            json!({ "key_id": key_id, "staff": staff }),
        ),
        StaffRequest::Merge { key_id, from } => {
            ("/v1/staff/merge", json!({ "key_id": key_id, "from": from }))
        }
        StaffRequest::Unlink { key_id } => ("/v1/staff/unlink", json!({ "key_id": key_id })),
        StaffRequest::Credits {
            key_id,
            delta,
            note,
        } => (
            "/v1/staff/credits",
            json!({ "key_id": key_id, "delta": delta, "note": note }),
        ),
        StaffRequest::Shards {
            key_id,
            delta,
            note,
        } => (
            "/v1/staff/shards",
            json!({ "key_id": key_id, "delta": delta, "note": note }),
        ),
    }
}

/// A staff query's path and body.
fn staff_query_call(query: &StaffQuery) -> (&'static str, Value) {
    match query {
        StaffQuery::RequestLogs { key_id, note } => (
            "/v1/staff/log-request",
            json!({ "key_id": key_id, "note": note }),
        ),
        StaffQuery::LogRequests => ("/v1/staff/log-requests", json!({})),
        StaffQuery::Log(id) => ("/v1/staff/log", json!({ "id": id })),
        StaffQuery::CrashGroups => ("/v1/staff/crash-groups", json!({})),
        StaffQuery::Crashes(signature) => ("/v1/staff/crashes", json!({ "signature": signature })),
        StaffQuery::Crash(id) => ("/v1/staff/crash", json!({ "id": id })),
        StaffQuery::ChatBans => ("/v1/staff/chat-bans", json!({})),
        StaffQuery::Gifts => ("/v1/staff/gifts", json!({})),
    }
}

fn parse<T: DeserializeOwned>(value: Value) -> Result<T, HubError> {
    serde_json::from_value(value).map_err(|error| HubError::Protocol(error.to_string()))
}

impl Hub for HttpHub {
    fn register(&mut self, identity: &Identity, name: Option<&str>) -> Result<Profile, HubError> {
        let body = match name {
            Some(name) => json!({ "name": name }),
            None => json!({}),
        };
        parse(self.send(Some(identity), "POST", "/v1/register", Some(body))?)
    }

    fn report(&mut self, identity: &Identity, report: &BugReport) -> Result<i64, HubError> {
        let mut body = json!({
            "text": report.text,
            "map": report.map,
            "build": report.build,
            "server": report.server,
        });
        if !report.name.is_empty() {
            body["name"] = json!(report.name);
        }
        let answer = self.send(Some(identity), "POST", "/v1/report", Some(body))?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the report answer has no id".to_owned()))
    }

    fn settings_usage(&mut self, body: &Value) -> Result<u64, HubError> {
        let answer = self.send(None, "POST", "/v1/usage/settings", Some(body.clone()))?;
        Ok(answer["stored"].as_u64().unwrap_or(0))
    }

    fn crash(&mut self, identity: &Identity, report: &CrashReport) -> Result<i64, HubError> {
        let answer = self.send(Some(identity), "POST", "/v1/crash", Some(report.body()))?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the crash report answer has no id".to_owned()))
    }

    fn player_report(
        &mut self,
        identity: &Identity,
        report: &PlayerReport,
    ) -> Result<i64, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/player-report",
            Some(player_report_body(report)),
        )?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the player report answer has no id".to_owned()))
    }

    fn note(&mut self, identity: &Identity, note: &WorldNote) -> Result<i64, HubError> {
        let mut body = json!({
            "text": note.text,
            "map": note.map,
            "build": note.build,
            "server": note.server,
            "view": note.view,
            "hit": note.hit,
            "normal": note.normal,
            "shader": note.shader,
            "surface": note.surface,
            "lighting": note.lighting,
            "distance": note.distance,
            "entity": note.entity,
        });
        if !note.name.is_empty() {
            body["name"] = json!(note.name);
        }
        note_context(&mut body, note);
        let answer = self.send(Some(identity), "POST", "/v1/note", Some(body))?;
        answer
            .get("id")
            .and_then(serde_json::Value::as_i64)
            .ok_or_else(|| HubError::Protocol("the note answer has no id".to_owned()))
    }

    fn note_image(&mut self, identity: &Identity, id: i64, jpeg: &[u8]) -> Result<(), HubError> {
        let path = format!("/v1/note/{id}/image");
        self.send_bytes(Some(identity), "PUT", &path, jpeg, "image/jpeg")
            .map(|_| ())
    }

    fn set_bio(&mut self, identity: &Identity, bio: &str) -> Result<Profile, HubError> {
        let body = json!({ "bio": bio });
        parse(self.send(Some(identity), "PUT", "/v1/profile", Some(body))?)
    }

    fn set_banner(&mut self, identity: &Identity, id: &str) -> Result<Profile, HubError> {
        parse(self.send(
            Some(identity),
            "PUT",
            "/v1/profile/banner",
            Some(json!({"id": id})),
        )?)
    }

    fn set_emoji(&mut self, identity: &Identity, emoji: &str) -> Result<Profile, HubError> {
        let body = json!({ "emoji": emoji });
        parse(self.send(Some(identity), "PUT", "/v1/profile", Some(body))?)
    }

    fn set_achievements(
        &mut self,
        identity: &Identity,
        progress: &BTreeMap<String, u64>,
    ) -> Result<Vec<Achievement>, HubError> {
        let body = json!({ "progress": progress });
        let answer: Achievements =
            parse(self.send(Some(identity), "PUT", "/v1/achievements", Some(body))?)?;
        Ok(answer.achievements)
    }

    fn staff(
        &mut self,
        identity: &Identity,
        request: &StaffRequest,
    ) -> Result<Vec<Profile>, HubError> {
        let (path, body) = staff_call(request);
        let answer = self.send(Some(identity), "POST", path, Some(body))?;
        match request {
            StaffRequest::Search(_) | StaffRequest::SearchIn { .. } => {
                let found: Players = parse(answer)?;
                Ok(found.players)
            }
            // The chat's moderation answers `{}`.
            StaffRequest::ChatDelete { .. }
            | StaffRequest::ChatMute { .. }
            | StaffRequest::ChatUnban { .. }
            | StaffRequest::MapBlock { .. }
            | StaffRequest::RemapBlock { .. } => Ok(Vec::new()),
            _ => Ok(vec![parse(answer)?]),
        }
    }

    fn staff_search(
        &mut self,
        identity: &Identity,
        request: &StaffRequest,
    ) -> Result<crate::staff::Found, HubError> {
        let (path, body) = staff_call(request);
        parse(self.send(Some(identity), "POST", path, Some(body))?)
    }

    fn staff_query(
        &mut self,
        identity: &Identity,
        query: &StaffQuery,
    ) -> Result<StaffAnswer, HubError> {
        let (path, body) = staff_query_call(query);
        let body = body.to_string();
        let answer = self.send_with(
            Some(identity),
            "POST",
            path,
            body.as_bytes(),
            "application/json",
            STAFF_ANSWER_MAX,
            None,
        )?;
        Ok(match query {
            StaffQuery::RequestLogs { .. } | StaffQuery::Log(_) => {
                StaffAnswer::LogRequest(Box::new(parse::<LogRequestRow>(answer)?))
            }
            StaffQuery::LogRequests => {
                StaffAnswer::LogRequests(parse::<LogRequestRows>(answer)?.requests)
            }
            StaffQuery::CrashGroups => {
                StaffAnswer::CrashGroups(parse::<CrashGroups>(answer)?.groups)
            }
            StaffQuery::Crashes(_) => {
                StaffAnswer::Crashes(parse::<CrashSummaries>(answer)?.crashes)
            }
            StaffQuery::Crash(_) => StaffAnswer::Crash(Box::new(parse::<CrashDetail>(answer)?)),
            StaffQuery::ChatBans => StaffAnswer::ChatBans(parse::<ChatBanRows>(answer)?.bans),
            StaffQuery::Gifts => StaffAnswer::Gifts(parse::<crate::staff::GiftLog>(answer)?),
        })
    }

    fn log_requests(&mut self, identity: &Identity) -> Result<Vec<LogRequest>, HubError> {
        let answer: LogRequests =
            parse(self.send(Some(identity), "GET", "/v1/log-requests", None)?)?;
        Ok(answer.requests)
    }

    fn send_logs(
        &mut self,
        identity: &Identity,
        id: i64,
        upload: &LogUpload,
    ) -> Result<(), HubError> {
        let body = upload.body(id).to_string();
        self.send_with(
            Some(identity),
            "POST",
            "/v1/logs",
            body.as_bytes(),
            "application/json",
            ANSWER_MAX,
            Some(LOGS_TIMEOUT),
        )
        .map(|_| ())
    }

    fn send_own_logs(
        &mut self,
        identity: &Identity,
        upload: &LogUpload,
        note: &str,
    ) -> Result<i64, HubError> {
        #[derive(serde::Deserialize)]
        struct Kept {
            id: i64,
        }
        let body = upload.own_body(note).to_string();
        let answer: Kept = parse(self.send_with(
            Some(identity),
            "POST",
            "/v1/logs",
            body.as_bytes(),
            "application/json",
            ANSWER_MAX,
            Some(LOGS_TIMEOUT),
        )?)?;
        Ok(answer.id)
    }

    fn decline_logs(&mut self, identity: &Identity, id: i64) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "POST",
            "/v1/logs/decline",
            Some(json!({ "request": id })),
        )
        .map(|_| ())
    }

    fn set_avatar(&mut self, identity: &Identity, png: &[u8]) -> Result<Profile, HubError> {
        parse(self.send_bytes(Some(identity), "PUT", "/v1/avatar", png, "image/png")?)
    }

    fn remove_avatar(&mut self, identity: &Identity) -> Result<Profile, HubError> {
        parse(self.send(Some(identity), "DELETE", "/v1/avatar", None)?)
    }

    fn avatar(&mut self, key_id: &str, version: &str) -> Result<Vec<u8>, HubError> {
        if !crate::avatar::valid_key_id(key_id) || !crate::avatar::valid_version(version) {
            return Err(HubError::Protocol(
                "a picture is named by a key id and a version".to_owned(),
            ));
        }
        let png = self.fetch(
            &crate::avatar::path(key_id, version),
            crate::avatar::ANSWER_MAX,
        )?;
        if !png.starts_with(crate::avatar::PNG_SIGNATURE) {
            return Err(HubError::Protocol("the picture is not a PNG".to_owned()));
        }
        Ok(png)
    }

    fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
        if key_id.len() != 16 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(HubError::Protocol("a key id is 16 hex digits".to_owned()));
        }
        parse(self.send(None, "GET", &format!("/v1/profile/{key_id}"), None)?)
    }

    fn claim(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
        active: bool,
    ) -> Result<(), HubError> {
        self.claim_with(identity, server, slot, name, active, None)
    }

    fn claim_with(
        &mut self,
        identity: &Identity,
        server: &str,
        slot: u8,
        name: &str,
        active: bool,
        play: Option<&crate::wire::PlaySignals>,
    ) -> Result<(), HubError> {
        let body = claim_body_with(server, slot, name, active, play);
        self.send(Some(identity), "POST", "/v1/claim", Some(body))
            .map(|_| ())
    }

    fn holocrons(&mut self, identity: &Identity) -> Result<HolocronState, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/holocrons", None)?)
    }

    fn open_holocrons(&mut self, identity: &Identity, ids: &[u64]) -> Result<OpenAnswer, HubError> {
        let (path, body) = open_call(ids);
        parse(self.send(Some(identity), "POST", path, Some(body))?)
    }

    fn reveal_cards(&mut self, identity: &Identity) -> Result<crate::wire::RevealAnswer, HubError> {
        parse(self.send(Some(identity), "POST", "/v1/cards/reveal", Some(json!({})))?)
    }

    fn shatter_unlock(
        &mut self,
        identity: &Identity,
        id: &str,
        last: bool,
    ) -> Result<crate::wire::ShatterAnswer, HubError> {
        // `last` only when asked: hubs before 21/10/2026 refuse a field they do not know.
        let body = if last {
            json!({ "id": id, "last": true })
        } else {
            json!({ "id": id })
        };
        parse(self.send(Some(identity), "POST", "/v1/unlocks/shatter", Some(body))?)
    }

    fn shatter_duplicates(
        &mut self,
        identity: &Identity,
    ) -> Result<crate::wire::ShatterAllAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/unlocks/shatter-duplicates",
            Some(json!({})),
        )?)
    }

    fn shatter_many(
        &mut self,
        identity: &Identity,
        items: &[(String, u32)],
        last: bool,
    ) -> Result<crate::wire::ShatterAllAnswer, (HubError, Option<String>)> {
        let items: Vec<_> = items
            .iter()
            .map(|(id, copies)| json!({ "id": id, "copies": copies }))
            .collect();
        self.refused_id = None;
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/unlocks/shatter-many",
            Some(json!({ "items": items, "last": last })),
        );
        answer
            .and_then(parse)
            .map_err(|error| (error, self.refused_id.take()))
    }

    fn craft_unlock(
        &mut self,
        identity: &Identity,
        id: &str,
    ) -> Result<crate::wire::CraftAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/unlocks/craft",
            Some(json!({ "id": id })),
        )?)
    }

    fn fuse_unlocks(
        &mut self,
        identity: &Identity,
        ids: &[String],
    ) -> Result<crate::wire::FuseAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/unlocks/fuse",
            Some(json!({ "ids": ids })),
        )?)
    }

    fn set_wishlist(
        &mut self,
        identity: &Identity,
        ids: &[String],
    ) -> Result<crate::wire::WishlistAnswer, HubError> {
        let answer = self.send(
            Some(identity),
            "PUT",
            "/v1/shop/wishlist",
            Some(json!({ "ids": ids })),
        )?;
        // An answer without the list (or none at all) keeps the list sent.
        Ok(serde_json::from_value(answer).unwrap_or_default())
    }

    fn shatter_tag(
        &mut self,
        identity: &Identity,
        copy: &crate::wire::TagCopy,
    ) -> Result<crate::wire::TagShatterAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/tags/shatter",
            Some(json!(copy)),
        )?)
    }

    fn craft_tags(
        &mut self,
        identity: &Identity,
        copies: &[crate::wire::TagCopy; 2],
    ) -> Result<crate::wire::TagCraftAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/tags/craft",
            Some(json!({ "copies": copies })),
        )?)
    }

    fn make_gif_tag(
        &mut self,
        identity: &Identity,
        giphy: &str,
    ) -> Result<crate::wire::GifTagAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/tags/gif",
            Some(json!({ "giphy": giphy })),
        )?)
    }

    fn set_mark(
        &mut self,
        identity: &Identity,
        id: &str,
        mark: crate::wire::Mark,
        on: bool,
    ) -> Result<crate::wire::UnlockMarks, HubError> {
        let answer: crate::wire::MarksAnswer = parse(self.send(
            Some(identity),
            "POST",
            "/v1/unlocks/marks",
            Some(json!({ "id": id, "mark": mark.id(), "on": on })),
        )?)?;
        Ok(answer.marks)
    }

    fn shop(&mut self, identity: &Identity) -> Result<crate::wire::ShopDay, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/shop", None)?)
    }

    fn announcements(
        &mut self,
        identity: &Identity,
    ) -> Result<Vec<crate::news::Announcement>, HubError> {
        let answer: crate::news::Announcements =
            parse(self.send(Some(identity), "GET", "/v1/announcements", None)?)?;
        Ok(answer.announcements)
    }

    fn rewards(&mut self, identity: &Identity) -> Result<crate::news::Rewards, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/rewards", None)?)
    }

    fn site_sign_in(&mut self, identity: &Identity) -> Result<crate::site::SignInLink, HubError> {
        parse(self.send(Some(identity), "POST", "/v1/site/sign-in", Some(json!({})))?)
    }

    fn site_link(
        &mut self,
        identity: &Identity,
        code: &str,
        confirm: bool,
    ) -> Result<crate::site::Linked, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/site/link",
            Some(json!({ "code": code, "confirm": confirm })),
        )?)
    }

    fn claim_reward(
        &mut self,
        identity: &Identity,
        id: u64,
    ) -> Result<crate::news::ClaimAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/rewards/claim",
            Some(json!({ "id": id })),
        )?)
    }

    fn announce(
        &mut self,
        identity: &Identity,
        request: &crate::news::AnnounceRequest,
    ) -> Result<crate::news::Announcement, HubError> {
        match self.send(
            Some(identity),
            "POST",
            "/v1/staff/announce",
            Some(announce_body(request)),
        ) {
            // A hub before 31/10/2026 takes announcements from the creator only.
            Err(HubError::Rejected { status: 404, .. }) => parse(self.send(
                Some(identity),
                "POST",
                "/v1/creator/announce",
                Some(announce_body(request)),
            )?),
            answer => parse(answer?),
        }
    }

    fn upload_news_image(
        &mut self,
        identity: &Identity,
        bytes: &[u8],
        mime: &str,
    ) -> Result<crate::news::NewsImage, HubError> {
        parse(self.send_bytes(Some(identity), "PUT", "/v1/staff/news-image", bytes, mime)?)
    }

    fn news_image(&mut self, id: &str) -> Result<Vec<u8>, HubError> {
        if !crate::news_markup::image_id_ok(id) {
            return Err(HubError::Protocol(
                "a news picture is named by 16 hex digits".to_owned(),
            ));
        }
        self.fetch(
            &crate::news::news_image_path(id),
            crate::news::NEWS_IMAGE_MAX,
        )
    }

    fn buy_offer(
        &mut self,
        identity: &Identity,
        id: &str,
    ) -> Result<crate::wire::BuyAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/shop/buy",
            Some(json!({ "id": id })),
        )?)
    }

    fn mystery_boxes(
        &mut self,
        identity: &Identity,
    ) -> Result<crate::mystery_box::Boxes, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/mystery-boxes", None)?)
    }

    fn wheel_of_doom(
        &mut self,
        identity: &Identity,
    ) -> Result<crate::wheel_of_doom::Wheel, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/wheel-of-doom", None)?)
    }

    fn spin_wheel_of_doom(
        &mut self,
        identity: &Identity,
        server: &str,
    ) -> Result<crate::wheel_of_doom::SpinAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/wheel-of-doom/spin",
            Some(json!({ "server": server })),
        )?)
    }

    fn junk_status(&mut self, identity: &Identity) -> Result<crate::junk::JunkStatus, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/fishing/junk", None)?)
    }

    fn sell_junk(
        &mut self,
        identity: &Identity,
        held: &std::collections::BTreeMap<String, u32>,
    ) -> Result<crate::junk::JunkSold, HubError> {
        let body = crate::junk::sale_body(held)
            .ok_or_else(|| HubError::Protocol("no junk to sell".to_owned()))?;
        parse(self.send(Some(identity), "POST", "/v1/fishing/junk/sell", Some(body))?)
    }

    fn open_mystery_box(
        &mut self,
        identity: &Identity,
        id: i64,
        server: &str,
    ) -> Result<crate::mystery_box::OpenAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/mystery-box/open",
            Some(json!({ "box": id, "server": server })),
        )?)
    }

    fn box_hunt(
        &mut self,
        identity: &Identity,
        server: &str,
        away: bool,
    ) -> Result<crate::mystery_box::HuntState, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/box-hunt",
            Some(json!({ "server": server, "away": away })),
        )?)
    }

    fn quests(
        &mut self,
        identity: &Identity,
        server: &str,
        away: bool,
    ) -> Result<crate::quests::QuestState, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/quests",
            Some(json!({ "server": server, "away": away })),
        )?)
    }

    fn quest_done(&mut self, identity: &Identity, server: &str, by: u8) -> Result<bool, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/quests/done",
            Some(json!({ "server": server, "by": by })),
        )?;
        Ok(answer
            .get("counted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn box_hunt_killed(
        &mut self,
        identity: &Identity,
        server: &str,
        by: u8,
    ) -> Result<bool, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/box-hunt/killed",
            Some(json!({ "server": server, "by": by })),
        )?;
        Ok(answer
            .get("counted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn box_hunt_out(
        &mut self,
        identity: &Identity,
        server: &str,
        out: crate::mystery_box::HuntOut,
    ) -> Result<bool, HubError> {
        let body = match out {
            crate::mystery_box::HuntOut::Dead { by } => {
                json!({ "server": server, "why": "dead", "by": by })
            }
            crate::mystery_box::HuntOut::Cheat => json!({ "server": server, "why": "cheat" }),
            crate::mystery_box::HuntOut::Chat => json!({ "server": server, "why": "chat" }),
        };
        let answer = self.send(Some(identity), "POST", "/v1/box-hunt/out", Some(body))?;
        Ok(answer
            .get("counted")
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn place_mystery_box(
        &mut self,
        identity: &Identity,
        placed: &crate::mystery_box::MysteryBox,
    ) -> Result<i64, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/creator/mystery-box",
            Some(json!({
                "map": placed.map,
                "checksum": placed.checksum,
                "server": placed.server,
                "origin": placed.origin,
                "yaw": placed.yaw,
            })),
        )?;
        answer["id"]
            .as_i64()
            .ok_or_else(|| HubError::Protocol("a placed box without its id".to_owned()))
    }

    fn remove_mystery_box(&mut self, identity: &Identity, id: i64) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "POST",
            "/v1/creator/mystery-box/remove",
            Some(json!({ "id": id })),
        )
        .map(|_| ())
    }

    fn secrets(
        &mut self,
        identity: &Identity,
        server: Option<&str>,
    ) -> Result<crate::toy_master::Secrets, HubError> {
        let path = match server {
            Some(server) => format!("/v1/secrets?server={}", encode_server(server)),
            None => "/v1/secrets".to_owned(),
        };
        parse(self.send(Some(identity), "GET", &path, None)?)
    }

    fn use_secret(
        &mut self,
        identity: &Identity,
        id: i64,
        server: &str,
        round: i32,
        presses: u32,
    ) -> Result<crate::toy_master::Used, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/secrets/use",
            Some(json!({ "secret": id, "server": server, "round": round, "presses": presses })),
        )?)
    }

    fn take_secret_drop(
        &mut self,
        identity: &Identity,
        id: i64,
        server: &str,
        round: i32,
        drop: u32,
    ) -> Result<crate::toy_master::Took, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/secrets/take",
            Some(json!({ "secret": id, "server": server, "round": round, "drop": drop })),
        )?)
    }

    fn place_secret(
        &mut self,
        identity: &Identity,
        placing: &crate::toy_master::Placing,
    ) -> Result<i64, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/creator/secret",
            Some(json!({
                "kind": placing.kind,
                "map": placing.map,
                "checksum": placing.checksum,
                "server": placing.server,
                "origin": placing.origin,
                "yaw": placing.yaw,
                "escorted": placing.escorted,
            })),
        )?;
        answer["id"]
            .as_i64()
            .ok_or_else(|| HubError::Protocol("a hidden toy without its id".to_owned()))
    }

    fn remove_secret(&mut self, identity: &Identity, id: i64) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "POST",
            "/v1/creator/secret/remove",
            Some(json!({ "id": id })),
        )
        .map(|_| ())
    }

    fn escort_secret(
        &mut self,
        identity: &Identity,
        id: i64,
        escorted: bool,
    ) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "POST",
            "/v1/creator/secret/escort",
            Some(json!({ "id": id, "escorted": escorted })),
        )
        .map(|_| ())
    }

    fn buy_holocron(
        &mut self,
        identity: &Identity,
        tier: &str,
    ) -> Result<crate::wire::HolocronBuyAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/holocrons/buy",
            Some(json!({ "tier": tier })),
        )?)
    }

    fn trades(&mut self, identity: &Identity) -> Result<crate::trades::TradesAnswer, HubError> {
        parse(self.send(Some(identity), "GET", "/v1/trades", None)?)
    }

    fn history(
        &mut self,
        identity: &Identity,
        before: Option<i64>,
    ) -> Result<crate::history::HistoryPage, HubError> {
        let path = crate::history::path(before);
        parse(self.send(Some(identity), "GET", &path, None)?)
    }

    fn trade_ask(
        &mut self,
        identity: &Identity,
        to: &str,
    ) -> Result<crate::trades::TradeAnswer, HubError> {
        parse(self.send(
            Some(identity),
            "POST",
            "/v1/trades",
            Some(json!({ "to": to })),
        )?)
    }

    fn trade_step(
        &mut self,
        identity: &Identity,
        id: i64,
        step: &crate::trades::TradeStep,
    ) -> Result<crate::trades::TradeAnswer, HubError> {
        let (action, body) = step.call();
        let path = format!("/v1/trades/{id}/{action}");
        parse(self.send(Some(identity), "POST", &path, Some(body))?)
    }

    fn release(&mut self, identity: &Identity, server: &str) -> Result<(), HubError> {
        self.send(
            Some(identity),
            "DELETE",
            "/v1/claim",
            Some(json!({"server": server})),
        )
        .map(|_| ())
    }

    fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
        let query = encode_server(server);
        let answer = self.send(None, "GET", &format!("/v1/presence?server={query}"), None)?;
        parse(answer["players"].clone())
    }

    fn presence_with_blocks(&mut self, server: &str) -> Result<PresenceRead, HubError> {
        let query = encode_server(server);
        let answer = self.send(None, "GET", &format!("/v1/presence?server={query}"), None)?;
        let players = parse(answer["players"].clone())?;
        // A hub from before map blocks leaves them out; a malformed list is none.
        let blocks = serde_json::from_value(answer["blocks"].clone()).unwrap_or_default();
        // Shows under way: a malformed list is none (the shows are cosmetic).
        let shows = serde_json::from_value(answer["shows"].clone()).unwrap_or_default();
        Ok(PresenceRead {
            players,
            blocks,
            map_edits: answer["map_edits"].as_i64(),
            // A hub from before remap blocks leaves them out, and a malformed list is
            // none: the client keeps the list it last saw.
            remap_blocks: serde_json::from_value(answer["remap_blocks"].clone()).ok(),
            secrets: answer["secrets"].as_i64(),
            shows,
        })
    }

    fn set_models(&mut self, identity: &Identity, models: &[String]) -> Result<u64, HubError> {
        let body = json!({ "models": models });
        let answer = self.send(Some(identity), "PUT", "/v1/models", Some(body))?;
        answer["stored"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the models answer has no count".to_owned()))
    }

    fn model_check(
        &mut self,
        server: &str,
        slot: u8,
        model: &str,
    ) -> Result<(String, Option<bool>), HubError> {
        let path = format!(
            "/v1/models?server={}&slot={slot}&model={}",
            encode_server(server),
            encode_model(model)
        );
        let answer = self.send(None, "GET", &path, None)?;
        let claimed_name = answer["claimed_name"]
            .as_str()
            .ok_or_else(|| HubError::Protocol("the model answer has no name".to_owned()))?;
        Ok((claimed_name.to_owned(), answer["has"].as_bool()))
    }

    fn chat(
        &mut self,
        identity: &Identity,
        text: &str,
        name: &str,
        channel: crate::HubChannel,
    ) -> Result<u64, HubError> {
        let mut body = json!({ "text": text });
        if !name.is_empty() {
            body["name"] = json!(name);
        }
        // The SJK chat goes as it always did, for hubs before clan chats.
        if channel.is_clan() {
            body["channel"] = json!(channel.wire());
        }
        let answer = self.send(Some(identity), "POST", "/v1/chat", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the chat answer has no id".to_owned()))
    }

    fn react(
        &mut self,
        identity: &Identity,
        message: u64,
        emoji: &str,
        active: bool,
    ) -> Result<(), HubError> {
        let answer = self
            .send(
                Some(identity),
                "POST",
                "/v1/chat/reaction",
                Some(json!({"message":message,"emoji":emoji,"active":active})),
            )
            .map_err(|error| match error {
                HubError::Rejected {
                    status: 404,
                    ref code,
                    ..
                } if code != "not_found" => {
                    HubError::Protocol("Chat reactions need an updated hub".into())
                }
                other => other,
            })?;
        answer["changed"]
            .as_bool()
            .ok_or_else(|| HubError::Protocol("Reaction answer has no changed flag".into()))?;
        Ok(())
    }

    fn chat_ban(&mut self, identity: &Identity, key_id: &str) -> Result<ChatBan, HubError> {
        let body = json!({ "key_id": key_id });
        let answer = self.send(Some(identity), "POST", "/v1/chat/ban", Some(body))?;
        let until = answer["until"]
            .as_i64()
            .ok_or_else(|| HubError::Protocol("the ban answer has no until".to_owned()))?;
        Ok(ChatBan {
            until,
            address: answer["address"].as_bool().unwrap_or(false),
        })
    }

    fn emote(
        &mut self,
        identity: &Identity,
        server: &str,
        emote: &str,
        sync: Option<u8>,
        quiet: bool,
    ) -> Result<u64, HubError> {
        let mut body = json!({ "server": server, "emote": emote });
        if let Some(sync) = sync {
            body["sync"] = json!(sync);
        }
        if quiet {
            body["quiet"] = json!(true);
        }
        let answer = self.send(Some(identity), "POST", "/v1/emote", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the emote answer has no id".to_owned()))
    }

    fn ping(
        &mut self,
        identity: &Identity,
        server: &str,
        pos: [i32; 3],
        scope: &str,
    ) -> Result<u64, HubError> {
        let body = json!({ "server": server, "pos": pos, "scope": scope });
        let answer = self.send(Some(identity), "POST", "/v1/ping", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the ping answer has no id".to_owned()))
    }

    fn toy_event(
        &mut self,
        identity: &Identity,
        server: &str,
        toy: &str,
        event: &str,
        target: u8,
    ) -> Result<u64, HubError> {
        let body = json!({ "server": server, "toy": toy, "event": event, "target": target });
        let answer = self.send(Some(identity), "POST", "/v1/toy", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the toy event answer has no id".to_owned()))
    }

    fn relay_send(
        &mut self,
        identity: &Identity,
        server: &str,
        toy: &str,
        event: &str,
        target: u8,
    ) -> Result<u64, HubError> {
        let body = json!({ "server": server, "toy": toy, "event": event, "target": target });
        let answer = self.send(Some(identity), "POST", "/v1/relay", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the relay answer has no id".to_owned()))
    }

    fn relay_read(
        &mut self,
        identity: &Identity,
        server: &str,
        after: u64,
        wait: u64,
        whispers: bool,
    ) -> Result<RelayBatch, HubError> {
        let path = format!(
            "/v1/relay?after={after}&wait={wait}&server={}{}",
            encode_server(server),
            if whispers { "&whispers=1" } else { "" }
        );
        parse(self.send(Some(identity), "GET", &path, None)?)
    }

    fn whisper_send(
        &mut self,
        identity: &Identity,
        server: &str,
        target: u8,
        to: &str,
        boxed: &str,
    ) -> Result<u64, HubError> {
        let body = json!({ "server": server, "target": target, "to": to, "box": boxed });
        let answer = self.send(Some(identity), "POST", "/v1/whisper", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the whisper answer has no id".to_owned()))
    }

    fn vitals(
        &mut self,
        identity: &Identity,
        server: &str,
        sharing: &Sharing,
    ) -> Result<Vec<SharedVitals>, HubError> {
        let mut body = json!({
            "server": server,
            "share": sharing.share,
            "estimates": sharing.estimates,
        });
        if let Some(vitals) = sharing.vitals.filter(|_| sharing.share) {
            body["vitals"] = serde_json::to_value(vitals)
                .map_err(|error| HubError::Protocol(error.to_string()))?;
        }
        let answer = self.send(Some(identity), "POST", "/v1/vitals", Some(body))?;
        parse(answer["players"].clone())
    }

    fn look(&mut self, identity: &Identity, server: &str, look: &Look) -> Result<u64, HubError> {
        let body = look_body(server, look);
        let answer = self.send(Some(identity), "POST", "/v1/look", Some(body))?;
        answer["id"]
            .as_u64()
            .ok_or_else(|| HubError::Protocol("the look answer has no id".to_owned()))
    }

    fn assets(&mut self, identity: &Identity) -> Result<Vec<Pack>, HubError> {
        crate::assets::manifest(self.send(Some(identity), "GET", "/v1/assets", None)?)
    }

    fn map_edits(&mut self, identity: &Identity) -> Result<Vec<crate::map_edits::Entry>, HubError> {
        crate::map_edits::list(self.send(Some(identity), "GET", "/v1/map-edits", None)?)
    }

    fn map_edits_doc(
        &mut self,
        identity: &Identity,
        map: &str,
        checksum: i32,
    ) -> Result<Value, HubError> {
        if !crate::map_edits::valid_map(map) {
            return Err(HubError::Protocol(format!("not a map name: {map}")));
        }
        let path = format!("/v1/map-edits/doc?map={map}&checksum={checksum}");
        self.send(Some(identity), "GET", &path, None)
    }

    fn publish_map_edits(
        &mut self,
        identity: &Identity,
        map: &str,
        checksum: i32,
        edits: &Value,
    ) -> Result<u64, HubError> {
        let answer = self.send(
            Some(identity),
            "POST",
            "/v1/staff/map-edits",
            Some(json!({ "map": map, "checksum": checksum, "edits": edits })),
        )?;
        answer
            .get("revision")
            .and_then(Value::as_u64)
            .ok_or_else(|| HubError::Protocol("the map edits answer has no revision".to_owned()))
    }

    fn asset(&mut self, identity: &Identity, name: &str) -> Result<Vec<u8>, HubError> {
        if !crate::assets::valid_name(name) {
            return Err(HubError::Protocol(
                "a pack name is 1 to 32 of a to z, 0 to 9 and _".to_owned(),
            ));
        }
        let path = format!("/v1/assets/{name}");
        self.get_bytes(identity, &path, PACK_MAX, ASSET_TIMEOUT)
    }

    fn feed(
        &mut self,
        identity: &Identity,
        after: u64,
        server: Option<&str>,
        channels: &[crate::HubChannel],
        wait: u64,
    ) -> Result<Feed, HubError> {
        let path = feed_path(after, server, channels, wait);
        parse(self.send(Some(identity), "GET", &path, None)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hub_address_must_be_https_or_local() {
        assert!(valid_base_url("https://hub.example"));
        assert!(valid_base_url("https://hub.example:8443/"));
        assert!(valid_base_url("http://localhost:8787"));
        assert!(valid_base_url("http://127.0.0.1:8787"));
        assert!(valid_base_url("http://[::1]:8787"));
        assert!(!valid_base_url("https://hub.example/prefix"));
        assert!(!valid_base_url("http://127.0.0.1.evil.example"));
        assert!(!valid_base_url("http://localhost.evil.example:80"));
        assert!(!valid_base_url("http://hub.example"));
        assert!(!valid_base_url("hub.example"));
        assert!(!valid_base_url("https://"));
        assert!(!valid_base_url("https://user@hub.example"));
        assert!(!valid_base_url("ftp://hub.example"));
        assert!(!valid_base_url(""));
    }

    #[test]
    fn a_plain_http_address_is_refused_at_construction() {
        assert!(HttpHub::new("http://hub.example", "t").is_err());
        assert!(HttpHub::new("https://hub.example/", "t").is_ok());
    }

    #[test]
    fn a_player_report_sends_the_protocols_fields_only() {
        let report = PlayerReport {
            category: crate::report::Category::Griefing,
            text: "Team killing all match".to_owned(),
            server: "1.2.3.4:29070".to_owned(),
            slot: 5,
            target_name: "^1Troll".to_owned(),
            level_time: Some(754),
            ..PlayerReport::default()
        };
        let body = player_report_body(&report);
        assert_eq!(
            body,
            json!({"category": "griefing", "text": "Team killing all match",
                   "server": "1.2.3.4:29070", "slot": 5, "target_name": "^1Troll",
                   "map": "", "build": "", "level_time": 754})
        );
        let named = PlayerReport {
            target_key_id: "0123456789abcdef".to_owned(),
            server_name: "^4JoF".to_owned(),
            name: "^2Sol".to_owned(),
            ..report
        };
        let body = player_report_body(&named);
        assert_eq!(body["target_key_id"], "0123456789abcdef");
        assert_eq!(body["server_name"], "^4JoF");
        assert_eq!(body["name"], "^2Sol");
        let whispered = PlayerReport {
            whisper: Some(crate::ReportedWhisper {
                from: "a2V5".to_owned(),
                time: 1_000,
                text: "buy credits".to_owned(),
                signature: [0xff; 64],
            }),
            ..named
        };
        let body = player_report_body(&whispered);
        assert_eq!(
            body["whisper"],
            json!({"from": "a2V5", "time": 1_000, "text": "buy credits",
                   "signature": "_".repeat(85) + "w"})
        );
    }

    #[test]
    fn a_note_sends_its_remaps_and_model_only_when_it_has_them() {
        let mut body = json!({"text": "too shiny"});
        note_context(&mut body, &WorldNote::default());
        assert_eq!(
            body,
            json!({"text": "too shiny"}),
            "an old hub still takes it"
        );
        let note = WorldNote {
            remapped: "textures/video/raven".into(),
            remaps: vec![crate::NoteRemap {
                source: "server".into(),
                from: "textures/imperial/basic_floor".into(),
                to: "textures/video/raven".into(),
            }],
            model: Some(crate::NoteModel {
                path: "models/map_objects/taspir/lamp.md3".into(),
                classname: "misc_model_static".into(),
                entity: Some(42),
                origin: Some([1.0, 2.0, 3.0]),
                skin: String::new(),
                shaders: vec![crate::NoteModelShader {
                    surface: "lamp".into(),
                    shader: "models/map_objects/taspir/lamp".into(),
                    remapped: String::new(),
                }],
            }),
            ..WorldNote::default()
        };
        note_context(&mut body, &note);
        assert_eq!(body["remapped"], "textures/video/raven");
        assert_eq!(
            body["remaps"],
            json!([{"source": "server", "from": "textures/imperial/basic_floor",
                    "to": "textures/video/raven"}])
        );
        assert_eq!(
            body["model"],
            json!({"path": "models/map_objects/taspir/lamp.md3",
                   "classname": "misc_model_static", "entity": 42, "origin": [1.0, 2.0, 3.0],
                   "shaders": [{"surface": "lamp", "shader": "models/map_objects/taspir/lamp",
                                "remapped": ""}]})
        );
    }

    #[test]
    fn the_feed_path_carries_after_wait_and_the_server() {
        assert_eq!(feed_path(0, None, &[], 25), "/v1/feed?after=0&wait=25");
        assert_eq!(
            feed_path(14, Some("1.2.3.4:29070"), &[], 25),
            "/v1/feed?after=14&wait=25&server=1.2.3.4%3A29070"
        );
        assert_eq!(
            feed_path(1, Some("[::1]:29070"), &[], 0),
            "/v1/feed?after=1&wait=0&server=%5B%3A%3A1%5D%3A29070"
        );
        // The clan chats joined (`PROTOCOL.md`, "Clan chats").
        assert_eq!(
            feed_path(
                3,
                None,
                &[crate::HubChannel::Jof, crate::HubChannel::Sith],
                25
            ),
            "/v1/feed?after=3&wait=25&channels=jof,sith"
        );
    }

    /// A message refused for a ban says, in local time, when the ban ends; other
    /// refusals keep the hub's words.
    #[test]
    fn a_chat_ban_refusal_says_when_it_ends() {
        let mut hub = HttpHub::new("https://hub.example", "test").unwrap();
        let refused = hub.refusal(
            403,
            &json!({"error": "chat_banned", "message": "banned", "until": 1_792_240_320}),
            true,
        );
        let Some(HubError::Rejected { code, message, .. }) = refused else {
            panic!("{refused:?}");
        };
        assert_eq!(code, "chat_banned");
        assert_eq!(message, crate::ban::banned_message(1_792_240_320));
        let refused = hub.refusal(
            403,
            &json!({"error": "chat_muted", "message": "muted", "until": 5}),
            true,
        );
        assert!(matches!(refused, Some(HubError::Rejected { message, .. }) if message == "muted"));
    }

    #[test]
    fn a_staff_map_block_sends_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::MapBlock {
                server: "1.2.3.4:29070".into(),
                map: "mp/ffa3".into(),
                serverid: 77,
                slot: 5,
                name: "^1Troll".into(),
                blocked: true,
            }),
            (
                "/v1/staff/map-block",
                json!({"server": "1.2.3.4:29070", "map": "mp/ffa3", "serverid": 77,
                       "slot": 5, "name": "^1Troll", "blocked": true})
            )
        );
        let block: crate::MapBlock = serde_json::from_value(
            json!({"map": "mp/ffa3", "serverid": 77, "slot": 5, "name": "^1Troll", "at": 9}),
        )
        .unwrap();
        assert_eq!(
            (block.slot, block.serverid, block.name.as_str()),
            (5, 77, "^1Troll")
        );
    }

    #[test]
    fn scoped_searches_and_gift_undos_send_the_protocols_fields() {
        use crate::staff::SearchScope;
        // A plain search sends no scope, so a hub from before scopes still takes it.
        assert_eq!(
            staff_call(&StaffRequest::Search("so".into())).1,
            json!({"query": "so"})
        );
        assert_eq!(
            staff_call(&StaffRequest::SearchIn {
                query: String::new(),
                scope: SearchScope::Online,
            }),
            ("/v1/staff/search", json!({"query": "", "scope": "online"}))
        );
        assert_eq!(
            staff_call(&StaffRequest::SearchIn {
                query: "fox".into(),
                scope: SearchScope::Server("1.2.3.4:29070".into()),
            })
            .1,
            json!({"query": "fox", "scope": "server", "server": "1.2.3.4:29070"})
        );
        assert_eq!(
            staff_call(&StaffRequest::SearchIn {
                query: String::new(),
                scope: SearchScope::Staff,
            })
            .1,
            json!({"query": "", "scope": "staff"})
        );
        assert_eq!(
            staff_call(&StaffRequest::GiftUndo {
                id: 812,
                key_id: "0123456789abcdef".into(),
            }),
            ("/v1/staff/gift-undo", json!({"id": 812}))
        );
        assert_eq!(
            staff_query_call(&StaffQuery::Gifts),
            ("/v1/staff/gifts", json!({}))
        );
        // The answers: where the players found play, and the gifts with what is left.
        let found: crate::staff::Found = parse(json!({
            "players": [],
            "playing": [{"key_id": "0123456789abcdef", "server": "1.2.3.4:29070",
                         "slot": 3, "name": "^1Fox"}]
        }))
        .unwrap();
        assert_eq!(found.playing[0].slot, 3);
        let old: crate::staff::Found = parse(json!({"players": []})).unwrap();
        assert!(old.playing.is_empty(), "an older hub sends no playing");
        let log: crate::staff::GiftLog = parse(json!({
            "gifts": [{"id": 9, "action": "award", "key_id": "0123456789abcdef",
                       "name": "Fox", "what": "bug_hunter", "rank": 2, "note": "",
                       "created": 1_791_000_000, "undone": false}],
            "per_day": 20, "left": 19
        }))
        .unwrap();
        assert_eq!((log.per_day, log.left), (Some(20), Some(19)));
        assert_eq!(log.gifts[0].rank, Some(2));
        let creator: crate::staff::GiftLog = parse(json!({"gifts": []})).unwrap();
        assert_eq!(creator.left, None, "the creator has no limit");
    }

    #[test]
    fn staff_chat_requests_send_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::ChatDelete { id: 12 }),
            ("/v1/staff/chat-delete", json!({"id": 12}))
        );
        assert_eq!(
            staff_call(&StaffRequest::ChatMute {
                key_id: "0123456789abcdef".into(),
                muted: true
            }),
            (
                "/v1/staff/chat-mute",
                json!({"key_id": "0123456789abcdef", "muted": true})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::ChatUnban {
                key_id: "0123456789abcdef".into()
            }),
            (
                "/v1/staff/chat-unban",
                json!({"key_id": "0123456789abcdef"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Search("so".into())),
            ("/v1/staff/search", json!({"query": "so"}))
        );
        assert_eq!(
            staff_call(&StaffRequest::AvatarRemove {
                key_id: "0123456789abcdef".into()
            }),
            (
                "/v1/staff/avatar-remove",
                json!({"key_id": "0123456789abcdef"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::AvatarBlock {
                key_id: "0123456789abcdef".into(),
                blocked: true
            }),
            (
                "/v1/staff/avatar-block",
                json!({"key_id": "0123456789abcdef", "blocked": true})
            )
        );
    }

    /// A look's finisher (04/11/2026) goes only while one is worn.
    #[test]
    fn a_look_body_names_the_finisher_only_when_one_is_worn() {
        let plain = look_body("1.2.3.4:29070", &Look::default());
        assert!(plain.get("finisher").is_none());
        let finishing = Look {
            finisher: "finisher_disintegrate".to_owned(),
            ..Look::default()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &finishing)["finisher"],
            "finisher_disintegrate"
        );
    }

    #[test]
    fn a_look_sends_exactly_the_protocols_fields() {
        let look = Look {
            saber: "saber_sun".to_owned(),
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
        assert_eq!(
            look_body("1.2.3.4:29070", &look),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true})
        );
        // The options switched off go only when there are some, with a blade skin.
        let off = Look {
            saber_off: vec!["glint".to_owned(), "haze".to_owned()],
            ..look.clone()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &off),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                "saber_off": ["glint", "haze"]})
        );
        let stock = Look {
            saber: String::new(),
            ..off
        };
        assert!(
            look_body("1.2.3.4:29070", &stock)
                .get("saber_off")
                .is_none()
        );
        assert_eq!(
            look_body("[::1]:29070", &Look::default()).to_string(),
            r#"{"illuminate":false,"saber":"","server":"[::1]:29070"}"#
        );
        // The toys out go only when there are some, with or without a blade skin.
        let boxed = Look {
            toys: vec!["cardboard_box".to_owned()],
            ..Look::default()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &boxed),
            json!({"server": "1.2.3.4:29070", "saber": "", "illuminate": false,
                "toys": ["cardboard_box"]})
        );
        // The extras switched on go only when there are some, each with its saber's skin.
        let extras = Look {
            saber_on: vec!["flare_trail".to_owned()],
            saber2: Some(String::new()),
            saber2_on: vec!["heat_wake".to_owned()],
            ..look.clone()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &extras),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                "saber_on": ["flare_trail"], "saber2": ""})
        );
        assert!(
            look_body(
                "1.2.3.4:29070",
                &Look {
                    saber: String::new(),
                    ..extras
                }
            )
            .get("saber_on")
            .is_none()
        );
        // The second saber's skin goes only when it differs, its options only with a skin.
        let dual = Look {
            saber2: Some("saber_glitch".to_owned()),
            saber2_off: vec!["haze".to_owned()],
            saber_on: Vec::new(),
            saber2_on: Vec::new(),
            ..look.clone()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &dual),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                "saber2": "saber_glitch", "saber2_off": ["haze"]})
        );
        let stock_second = Look {
            saber2: Some(String::new()),
            ..dual
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &stock_second),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                "saber2": ""})
        );
        // The sounds chosen apart from the blade go only when chosen, stock or a skin's.
        for sounds in ["stock", "saber_void"] {
            let chosen = Look {
                saber_sounds: sounds.to_owned(),
                ..look.clone()
            };
            assert_eq!(
                look_body("1.2.3.4:29070", &chosen),
                json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                    "saber_sounds": sounds})
            );
        }
        // A stuck tag goes whole, only while there is one.
        let tagged = Look {
            tag: Some(crate::PlacedTag {
                finish: None,
                id: "tag_gg".to_owned(),
                map: "mp/ffa3".to_owned(),
                at: [120.5, -64.0, 24.0],
                normal: [0.0, 0.0, 1.0],
                spin: 90.0,
            }),
            ..look.clone()
        };
        assert_eq!(
            look_body("1.2.3.4:29070", &tagged),
            json!({"server": "1.2.3.4:29070", "saber": "saber_sun", "illuminate": true,
                "tag": {"id": "tag_gg", "map": "mp/ffa3", "at": [120.5, -64.0, 24.0],
                    "normal": [0.0, 0.0, 1.0], "spin": 90.0}})
        );
    }

    #[test]
    fn an_announcement_sends_the_protocols_fields() {
        let news = crate::news::AnnounceRequest {
            release: false,
            title: "Hello".into(),
            text: "Line\n\n- point".into(),
            version: String::new(),
        };
        assert_eq!(
            announce_body(&news),
            json!({"kind": "news", "title": "Hello", "text": "Line\n\n- point"})
        );
        let release = crate::news::AnnounceRequest {
            release: true,
            version: "2026.1025.1".into(),
            ..news
        };
        assert_eq!(
            announce_body(&release),
            json!({"kind": "release", "title": "Hello", "text": "Line\n\n- point",
                "version": "2026.1025.1"})
        );
    }

    #[test]
    fn staff_unlocks_send_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::Unlock {
                key_id: "0123456789abcdef".into(),
                unlock: "saber_sun".into(),
                note: "Thanks".into(),
            }),
            (
                "/v1/staff/unlock",
                json!({"key_id": "0123456789abcdef", "unlock": "saber_sun", "note": "Thanks"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Relock {
                key_id: "0123456789abcdef".into(),
                unlock: "saber_sun".into(),
            }),
            (
                "/v1/staff/relock",
                json!({"key_id": "0123456789abcdef", "unlock": "saber_sun"})
            )
        );
    }

    #[test]
    fn staff_medal_ranks_send_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::MedalRank {
                key_id: "0123456789abcdef".into(),
                medal: "early_tester".into(),
                rank: 4,
            }),
            (
                "/v1/staff/medal-rank",
                json!({"key_id": "0123456789abcdef", "medal": "early_tester", "rank": 4})
            )
        );
    }

    #[test]
    fn an_award_sends_its_rank_only_above_one() {
        let award = |rank| {
            staff_call(&StaffRequest::Award {
                key_id: "0123456789abcdef".into(),
                medal: "early_tester".into(),
                note: String::new(),
                rank,
            })
            .1
        };
        assert_eq!(
            award(1),
            json!({"key_id": "0123456789abcdef", "medal": "early_tester", "note": ""}),
            "a hub from before 28/10/2026 refuses a rank"
        );
        assert_eq!(award(4)["rank"], 4);
    }

    #[test]
    fn a_claim_says_active_only_when_it_is() {
        assert_eq!(
            claim_body("1.2.3.4:29070", 3, "Sol", false),
            json!({"server": "1.2.3.4:29070", "slot": 3, "name": "Sol"}),
            "the body of a hub from before holocrons"
        );
        assert_eq!(
            claim_body("1.2.3.4:29070", 3, "Sol", true),
            json!({"server": "1.2.3.4:29070", "slot": 3, "name": "Sol", "active": true})
        );
        let play = crate::wire::PlaySignals {
            moved: 900,
            attacks: 3,
            hits: 2,
            ..Default::default()
        };
        assert_eq!(
            claim_body_with("1.2.3.4:29070", 3, "Sol", true, Some(&play)),
            json!({"server": "1.2.3.4:29070", "slot": 3, "name": "Sol", "active": true,
                "play": {"moved": 900, "attacks": 3, "hits": 2, "hurt": 0, "kills": 0,
                    "deaths": 0, "bites": 0}})
        );
        assert_eq!(
            claim_body_with("1.2.3.4:29070", 3, "Sol", false, Some(&play)),
            json!({"server": "1.2.3.4:29070", "slot": 3, "name": "Sol"}),
            "signals go only with an active claim"
        );
    }

    #[test]
    fn opening_sends_the_ids_and_reads_what_they_brought() {
        assert_eq!(
            open_call(&[7, 9]),
            ("/v1/holocrons/open", json!({ "ids": [7, 9] }))
        );
        let answer: OpenAnswer = serde_json::from_str(
            r#"{"opened":[{"holocron":7,"tier":"rare","card":{"id":31,"rarity":"uncommon","finish":"holo","edition":"alpha"},"credits":42}],
                "profile":{"key_id":"0123456789abcdef","key":"k","name":"Sol","bio":"","verified":false,"created":1,
                  "credits":42,"cards":[{"card":null,"rarity":"uncommon","finish":"holo","edition":"alpha","count":1,"first":5,"last":5}],
                  "holocron_counts":{"uncommon":0,"rare":0,"legendary":0,"mythical":0},
                  "holocrons":[{"id":7,"tier":"rare","dropped":1,"source":"play","note":"","opened":5}]}}"#,
        )
        .unwrap();
        assert_eq!(answer.opened.len(), 1);
        let opened = &answer.opened[0];
        assert_eq!(
            (opened.holocron, opened.tier.as_str(), opened.credits),
            (7, "rare", 42)
        );
        assert_eq!(
            (
                opened.card.as_ref().unwrap().id,
                opened.card.as_ref().unwrap().rarity.as_str(),
                opened.card.as_ref().unwrap().finish.as_str(),
                opened.card.as_ref().unwrap().edition.as_str()
            ),
            (31, "uncommon", "holo", "alpha")
        );
        assert_eq!(answer.profile.credits, 42);
        assert_eq!(answer.profile.cards[0].card, None, "sealed");
        assert_eq!(answer.profile.cards[0].count, 1);
        assert_eq!(answer.profile.holocrons[0].opened, Some(5));
        assert!(!answer.profile.holocrons[0].is_sealed());
    }

    #[test]
    fn staff_holocron_requests_send_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::HolocronGive {
                key_id: "0123456789abcdef".into(),
                tier: "legendary".into(),
                note: "For the fog bug".into(),
            }),
            (
                "/v1/staff/holocron-give",
                json!({"key_id": "0123456789abcdef", "tier": "legendary",
                       "note": "For the fog bug"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::HolocronRemove {
                key_id: "0123456789abcdef".into(),
                id: 41,
            }),
            (
                "/v1/staff/holocron-remove",
                json!({"key_id": "0123456789abcdef", "id": 41})
            )
        );
    }

    #[test]
    fn staff_verify_merge_and_unlink_send_the_protocols_fields() {
        assert_eq!(
            staff_call(&StaffRequest::Verify {
                key_id: "0123456789abcdef".into(),
                verified: true,
                tick: None,
            }),
            (
                "/v1/staff/verify",
                json!({"key_id": "0123456789abcdef", "verified": true})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Verify {
                key_id: "0123456789abcdef".into(),
                verified: true,
                tick: Some(crate::Tick::Sith),
            }),
            (
                "/v1/staff/verify",
                json!({"key_id": "0123456789abcdef", "verified": true, "tick": "sith"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Merge {
                key_id: "0123456789abcdef".into(),
                from: "fedcba9876543210".into(),
            }),
            (
                "/v1/staff/merge",
                json!({"key_id": "0123456789abcdef", "from": "fedcba9876543210"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::SetStaff {
                key_id: "0123456789abcdef".into(),
                staff: false,
            }),
            (
                "/v1/staff/staff",
                json!({"key_id": "0123456789abcdef", "staff": false})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Unlink {
                key_id: "fedcba9876543210".into(),
            }),
            ("/v1/staff/unlink", json!({"key_id": "fedcba9876543210"}))
        );
        assert_eq!(
            staff_call(&StaffRequest::Credits {
                key_id: "0123456789abcdef".into(),
                delta: -20,
                note: "refund".into(),
            }),
            (
                "/v1/staff/credits",
                json!({"key_id": "0123456789abcdef", "delta": -20, "note": "refund"})
            )
        );
        assert_eq!(
            staff_call(&StaffRequest::Shards {
                key_id: "0123456789abcdef".into(),
                delta: 15,
                note: String::new(),
            }),
            (
                "/v1/staff/shards",
                json!({"key_id": "0123456789abcdef", "delta": 15, "note": ""})
            )
        );
        assert_eq!(
            staff_query_call(&StaffQuery::ChatBans),
            ("/v1/staff/chat-bans", json!({}))
        );
    }

    #[test]
    fn staff_queries_send_the_protocols_fields() {
        assert_eq!(
            staff_query_call(&StaffQuery::RequestLogs {
                key_id: "0123456789abcdef".into(),
                note: "your freeze".into(),
            }),
            (
                "/v1/staff/log-request",
                json!({"key_id": "0123456789abcdef", "note": "your freeze"})
            )
        );
        assert_eq!(
            staff_query_call(&StaffQuery::LogRequests),
            ("/v1/staff/log-requests", json!({}))
        );
        assert_eq!(
            staff_query_call(&StaffQuery::Log(4)),
            ("/v1/staff/log", json!({"id": 4}))
        );
        assert_eq!(
            staff_query_call(&StaffQuery::CrashGroups),
            ("/v1/staff/crash-groups", json!({}))
        );
        assert_eq!(
            staff_query_call(&StaffQuery::Crashes(String::new())),
            ("/v1/staff/crashes", json!({"signature": ""}))
        );
        assert_eq!(
            staff_query_call(&StaffQuery::Crash(7)),
            ("/v1/staff/crash", json!({"id": 7}))
        );
    }

    #[test]
    fn a_log_upload_sends_the_protocols_fields() {
        let upload = LogUpload {
            logs: "a\n".into(),
            build: "2026.1011.2".into(),
            os: "Windows 11".into(),
            gpu: "RTX".into(),
        };
        assert_eq!(
            upload.body(5),
            json!({"request": 5, "logs": "a\n", "build": "2026.1011.2",
                   "os": "Windows 11", "gpu": "RTX"})
        );
    }

    #[test]
    fn a_key_id_is_checked_before_a_request_is_made() {
        let mut hub = HttpHub::new("https://hub.example", "t").unwrap();
        assert!(matches!(hub.profile("../etc"), Err(HubError::Protocol(_))));
        assert!(matches!(
            hub.avatar("../etc", "0123456789abcdef"),
            Err(HubError::Protocol(_))
        ));
        assert!(matches!(
            hub.avatar("0123456789abcdef", "../x"),
            Err(HubError::Protocol(_))
        ));
        let identity = Identity::from_seed([7; 32]);
        assert!(matches!(
            hub.asset(&identity, "../etc"),
            Err(HubError::Protocol(_))
        ));
    }
}
