//! Player identity for SJK: an Ed25519 key the player keeps, and the client side
//! of the SJK hub's protocol (`PROTOCOL.md` in Sol-Vulpes/SJK-hub) that turns it
//! into a profile and a badge on game servers.
//!
//! The crate has no window, renderer or game-network knowledge. The viewer
//! creates a [`Service`], feeds it the player's [`Settings`] and [`Location`],
//! and reads its [`Snapshot`].

#![warn(missing_docs)]

pub mod assets;
pub mod avatar;
pub mod ban;
pub mod bio;
pub mod channels;
pub mod chat;
pub mod crash;
pub mod emoji;
pub mod feed;
pub mod history;
pub mod hub;
pub mod junk;
pub mod key_login;
mod keys;
pub mod logs;
pub mod map_edits;
pub mod mystery_box;
pub mod name_emoji;
pub mod news;
pub mod news_markup;
pub mod quests;
pub mod reactions;
pub mod relay;
pub mod report;
pub mod service;
pub mod staff;
pub mod toy_master;
pub mod trades;
pub mod usage;
pub mod vitals;
pub mod wheel_of_doom;
pub mod whisper;
pub mod wire;

pub use assets::Pack;
pub use ban::{BanOutcome, ChatBan};
pub use channels::{HubChannel, ListedChannel};
pub use crash::CrashReport;
pub use feed::{ChatState, ReceivedLooks};
pub use history::{HistoryLine, HistoryPage, HistoryState};
pub use hub::{HttpHub, Hub, HubError, valid_base_url};
pub use keys::{Identity, KeyError};
pub use logs::{LogRequest, LogUpload};
pub use news::{
    AnnounceError, AnnounceRequest, Announcement, ClaimAnswer, FeedAnnouncement, FeedRound,
    Official, Reward, Rewards, Round, RoundPerson,
};
pub use relay::RelayHubs;
pub use report::{
    BugReport, Category, NoteModel, NoteModelShader, NoteRemap, PlayerReport, ReportedWhisper,
    WorldNote,
};
pub use service::{
    ClaimOutcome, HolocronProgress, HubFactory, Location, ModelCheck, OpenOutcome, ReportOutcome,
    RevealOutcome, Service, Settings, ShopState, Snapshot, Status, TagOutcome, TagTrade, Trade,
    TradeOutcome, TradesClock,
};
pub use staff::{
    ChatBanRow, Found, GiftLog, GiftRow, OpenedText, Playing, SearchScope, StaffAnswer, StaffQuery,
    StaffRequest, StaffState,
};
pub use trades::{
    PlayerTrade, TradeAnswer, TradeEvent, TradeItem, TradePartner, TradeRules, TradeSide,
    TradeStep, TradeToday, TradesAnswer,
};
pub use vitals::{Sharing, VitalsTable};
pub use wire::{
    Achievement, ActivityState, BANNER_SHATTER_ABILITY, BannerTables, Bought, BuyAnswer, CardStack,
    Catchup, ChatMessage, CraftAnswer, Crafted, DropEvent, DropMark, EconomyTables, Emote, Feed,
    FindEvent, FindMark, HOLOCRONS_LISTED, Holocron, HolocronBought, HolocronBuyAnswer,
    HolocronCounts, HolocronState, KyberPity, KyberTables, KyberTier, Look, LookEvent, LootTier,
    MARKS_ABILITY, MEDAL_RANK_MAX, MEDAL_RANK_MIN, MapBlock, Mark, MarksAnswer, Medal, OPEN_MAX,
    Offer, OpenAnswer, Opened, OpenedCard, OpenedUnlock, People, Person, Ping, PlacedTag,
    PlaySignals, Presence, Profile, RelayBatch, RevealAnswer, Revealed, SealedWhisper,
    SharedVitals, ShatterAllAnswer, ShatterAnswer, Shattered, ShatteredCopies, ShopDay,
    TAG_COPIES_ABILITY, TagCopy, TagCraftAnswer, TagCrafted, TagShatterAnswer, TagShattered,
    TagTables, Tick, ToyEvent, Trust, Unlock, UnlockMarks, Vitals, WornName, names_match,
    normal_form,
};
