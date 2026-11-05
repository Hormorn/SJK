//! The hub's wire format (`PROTOCOL.md` in Sol-Vulpes/SJK-hub): signed
//! requests, the JSON the hub answers with, and how a claim's name is matched
//! to the name the game shows.

use crate::keys::Identity;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// How far a player who is neither verified nor staff is on the way to green
/// (`PROTOCOL.md`, "Trust"): an empty tick, a green one, or a red mark while a chat ban
/// holds them. Hubs before 16/10/2026 send none.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Trust {
    /// Registered less than 30 days ago (or since their last chat ban), or with less
    /// than 10 hours of play since.
    New,
    /// The wait and the play are done: only staff may chat-ban them.
    Trusted,
    /// A chat ban holds them now.
    Banned,
    /// A level a later hub added, shown as none.
    #[serde(other)]
    Unknown,
}

/// The tick after a person's name (`PROTOCOL.md`, "Ticks"): one per person, the hub's
/// `tick` field, which it leaves out for the plain gold one and for none. Read it only
/// through [`Tick::shown`], which tells those two apart by the verified flag.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Tick {
    /// No tick: not verified (and the hub's empty `tick` before [`Tick::shown`]).
    #[default]
    #[serde(rename = "")]
    None,
    /// The plain gold tick of a verified player.
    Gold,
    /// A verified player of JoF (Jedi of Freedom, its Academy too): cyan.
    Jof,
    /// A verified player of the Sith clan `{{S}}`: crimson.
    Sith,
    /// A verified player of the JAWA clan: leather.
    Jawa,
    /// JKR's creator: rust.
    Jkr,
    /// SJK staff.
    Staff,
    /// SJK's creator: the solar tick.
    Creator,
    /// A tick a later hub added, shown as gold while verified.
    #[serde(other)]
    Other,
}

impl Tick {
    /// The tick to draw for a person whose hub `tick` is `self` and who is `verified`
    /// or not: gold for a verified one without another, none otherwise.
    pub fn shown(self, verified: bool) -> Self {
        match self {
            Self::None | Self::Other if verified => Self::Gold,
            Self::None | Self::Other => Self::None,
            tick => tick,
        }
    }

    /// The groups staff verify a player with, in the order the Staff page offers them.
    pub const GROUPS: [Self; 4] = [Self::Gold, Self::Jof, Self::Sith, Self::Jawa];

    /// This group as `POST /v1/staff/verify`'s `tick` says it; `None` for a tick that
    /// is not a group (staff, creators, none).
    pub fn group_wire(self) -> Option<&'static str> {
        match self {
            Self::Gold => Some(""),
            Self::Jof => Some("jof"),
            Self::Sith => Some("sith"),
            Self::Jawa => Some("jawa"),
            _ => None,
        }
    }

    /// The tick's name as players read it.
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Gold | Self::Other => "Gold",
            Self::Jof => "JoF",
            Self::Sith => "Sith",
            Self::Jawa => "Jawa",
            Self::Jkr => "JKR creator",
            Self::Staff => "SJK staff",
            Self::Creator => "SJK creator",
        }
    }
}

/// A player's public profile as the hub serves it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Profile {
    /// Equipped cover id, shared across UI snapshots instead of allocating on clone.
    /// Empty on older hubs or when unequipped; JSON remains an ordinary string.
    #[serde(default, deserialize_with = "shared_banner")]
    pub banner: std::sync::Arc<str>,
    /// First 16 hex digits of the SHA-256 of the public key.
    pub key_id: String,
    /// The public key, unpadded base64url.
    pub key: String,
    /// Display name: the name the operator gave the key, else the in-game name it
    /// last wore; empty while the hub has seen neither.
    pub name: String,
    /// Free text the player wrote about themselves.
    pub bio: String,
    /// Whether the hub's operator vouches for this key.
    pub verified: bool,
    /// Whether the hub's operator made this key staff (absent from older hubs).
    #[serde(default)]
    pub staff: bool,
    /// Whether this is SJK's creator: staff too, with their own mark (the hub's
    /// `PROTOCOL.md`, "Creator"; absent when false and from older hubs).
    #[serde(default)]
    pub creator: bool,
    /// Whether this is JKR's creator: a mark of their own and no staff rights (the
    /// hub's `PROTOCOL.md`, "JKR creator"; absent when false and from older hubs).
    #[serde(default)]
    pub jkr_creator: bool,
    /// The player's tick as the hub sends it (absent from older hubs); draw
    /// [`Tick::shown`].
    #[serde(default)]
    pub tick: Tick,
    /// The player's trust, `None` when verified or staff (and from older hubs).
    #[serde(default)]
    pub trust: Option<Trust>,
    /// The player's name emoji (`crate::name_emoji`), shown at the right of their name;
    /// empty for none (and from hubs older than name emoji). Show it only through
    /// [`crate::name_emoji::for_display`].
    #[serde(default)]
    pub emoji: String,
    /// Registration time, unix seconds.
    pub created: i64,
    /// In-game names the key has worn, most recent first (absent from hubs older
    /// than the name history).
    #[serde(default)]
    pub names: Vec<WornName>,
    /// Medals the SJK team gave the key, in the hub's catalogue order (absent from
    /// hubs older than medals).
    #[serde(default)]
    pub medals: Vec<Medal>,
    /// Achievements with a count above 0, in the hub's catalogue order (absent from
    /// hubs older than achievements).
    #[serde(default)]
    pub achievements: Vec<Achievement>,
    /// Unlockables the key holds, in the hub's catalogue order (absent from hubs
    /// older than unlocks).
    #[serde(default)]
    pub unlocks: Vec<Unlock>,
    /// The version of the key's picture (`crate::avatar`), empty for none (and from hubs
    /// older than pictures).
    #[serde(default)]
    pub avatar: String,
    /// How many holocrons of each tier the key has earned (absent from hubs older
    /// than holocrons, which count as none).
    #[serde(default)]
    pub holocron_counts: HolocronCounts,
    /// The key's recent holocrons, newest first, at most [`HOLOCRONS_LISTED`] (absent
    /// from hubs older than holocrons).
    #[serde(default)]
    pub holocrons: Vec<Holocron>,
    /// The person's credits (`PROTOCOL.md`, "Holocrons", "Opening"): what opening
    /// holocrons brought, never below 0 (absent from hubs older than opening).
    #[serde(default)]
    pub credits: u64,
    /// The person's kyber shards (`PROTOCOL.md`, "Shards and the shop", 17/10/2026): what
    /// shattering duplicates and holocrons' shard piles brought, never below 0 (absent from
    /// hubs older than them).
    #[serde(default)]
    pub shards: u64,
    /// The person's cards, one entry per kind of copy held, the best first (absent from
    /// hubs older than opening).
    #[serde(default)]
    pub cards: Vec<CardStack>,
    /// The person's keys: the main key first, then the keys linked to it (absent from
    /// hubs older than linked keys). A linked key signs as its person, so the hub
    /// answers the person's [`Profile::key_id`] for it everywhere.
    #[serde(default)]
    pub keys: Vec<String>,
    /// The kinds of card a key may stick as a tag (`PROTOCOL.md`, "Unlocks", Tags; since
    /// 31/10/2026, `["credits"]`), the same in every profile. Hubs before leave it out and
    /// refuse a card's id or a finish in a look's tag: a client offers neither then.
    #[serde(default)]
    pub tag_cards: Vec<String>,
}

/// Decode once on the worker; cloning profiles shares immutable banner storage.
fn shared_banner<'de, D: serde::Deserializer<'de>>(
    input: D,
) -> Result<std::sync::Arc<str>, D::Error> {
    String::deserialize(input).map(std::sync::Arc::from)
}

impl Profile {
    /// The keys linked to the person's main key: every one of [`Profile::keys`] but the
    /// main key (the first, which is also [`Profile::key_id`]).
    pub fn linked_keys(&self) -> impl Iterator<Item = &str> {
        self.keys
            .iter()
            .map(String::as_str)
            .filter(move |key| *key != self.key_id)
    }

    /// Whether `key_id` is this person's: their main key or one linked to it.
    pub fn has_key(&self, key_id: &str) -> bool {
        self.key_id == key_id || self.keys.iter().any(|key| key == key_id)
    }
}

/// The most holocrons a profile lists, as the hub caps them.
pub const HOLOCRONS_LISTED: usize = 100;

/// How many holocrons of each tier a key holds unopened (`PROTOCOL.md`, "Holocrons"):
/// what it can still open. All four are always present on the wire; a count the client
/// does not know is none.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct HolocronCounts {
    /// Uncommon holocrons.
    #[serde(default)]
    pub uncommon: u32,
    /// Rare holocrons.
    #[serde(default)]
    pub rare: u32,
    /// Legendary holocrons.
    #[serde(default)]
    pub legendary: u32,
    /// Mythical holocrons.
    #[serde(default)]
    pub mythical: u32,
}

impl HolocronCounts {
    /// The count of the tier whose wire id is `tier` (`uncommon`, `rare`, `legendary`,
    /// `mythical`); 0 for an id this client does not know.
    pub fn of(&self, tier: &str) -> u32 {
        match tier {
            "uncommon" => self.uncommon,
            "rare" => self.rare,
            "legendary" => self.legendary,
            "mythical" => self.mythical,
            _ => 0,
        }
    }

    /// Every holocron the key has earned.
    pub fn total(&self) -> u32 {
        self.uncommon
            .saturating_add(self.rare)
            .saturating_add(self.legendary)
            .saturating_add(self.mythical)
    }
}

/// One holocron a key earned by playing, or that staff gave. Opening it
/// ([`crate::Hub::open_holocrons`]) brings a card and credits; an opened one stays
/// listed, with [`Holocron::opened`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Holocron {
    /// Its number at the hub, which only rises: a client remembers the highest it has
    /// shown.
    pub id: u64,
    /// Its tier's wire id; a client shows only the ids it knows.
    pub tier: String,
    /// When it dropped, unix seconds.
    #[serde(default)]
    pub dropped: i64,
    /// `play`, or `staff` for a gift.
    #[serde(default)]
    pub source: String,
    /// Public plain text from the team, only on a gift; often empty. Never markup.
    #[serde(default)]
    pub note: String,
    /// When it was opened, unix seconds; `None` while it can still be opened (and from
    /// hubs older than opening).
    #[serde(default)]
    pub opened: Option<i64>,
    /// When it last changed hands in a trade between players, unix seconds; `None` when
    /// it never did (and from hubs before 22/10/2026). A traded holocron waits the hub's
    /// resale lock before it trades again (`crate::trades`).
    #[serde(default)]
    pub traded: Option<i64>,
    /// The person it came from in that trade, by key; `None` when never traded.
    #[serde(default)]
    pub traded_from_key: Option<String>,
    /// Their name (colour codes kept; show it only through [`crate::chat::for_display`]),
    /// when the hub knows it.
    #[serde(default)]
    pub traded_from: Option<String>,
}

impl Holocron {
    /// Whether it can still be opened.
    pub fn is_sealed(&self) -> bool {
        self.opened.is_none()
    }
}

/// The most holocrons one opening takes (`POST /v1/holocrons/open`).
pub const OPEN_MAX: usize = 20;

/// Copies of one kind a person holds (`PROTOCOL.md`, "Holocrons", "Opening"): a card
/// (or a sealed one), its rarity, finish and edition, and how many.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CardStack {
    /// The card's id in the catalogue; `None` for a sealed card, whose face is chosen
    /// when it is revealed.
    #[serde(default)]
    pub card: Option<String>,
    /// `common`, `uncommon`, `rare`, `legendary` or `mythical` (19/10/2026); a client
    /// shows only the ids it knows.
    pub rarity: String,
    /// `normal`, `holo` or `solar`.
    pub finish: String,
    /// The era it was opened in (`alpha`).
    #[serde(default)]
    pub edition: String,
    /// How many such copies.
    #[serde(default)]
    pub count: u32,
    /// When the first of them was obtained, unix seconds.
    #[serde(default)]
    pub first: i64,
    /// When the newest of them was obtained, unix seconds.
    #[serde(default)]
    pub last: i64,
}

/// The copy one opening brought.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct OpenedCard {
    /// The copy's number at the hub.
    pub id: u64,
    /// The card it is, picked from the hub's catalogue; `None` while sealed (no card of
    /// its rarity yet, or a hub from before the catalogue).
    #[serde(default)]
    pub card: Option<String>,
    /// The card's rarity (as [`CardStack::rarity`]).
    pub rarity: String,
    /// The copy's finish (as [`CardStack::finish`]).
    pub finish: String,
    /// The copy's edition (as [`CardStack::edition`]).
    #[serde(default)]
    pub edition: String,
}

/// What opening one holocron brought.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Opened {
    /// Independent profile-banner bonus, beside the ordinary prize; absent on old hubs.
    #[serde(default)]
    pub banner: Option<OpenedUnlock>,
    /// The holocron opened.
    pub holocron: u64,
    /// Its tier's wire id.
    pub tier: String,
    /// The (sealed) card it held; `None` when it held a shader or a pile of shards
    /// instead.
    #[serde(default)]
    pub card: Option<OpenedCard>,
    /// The credits it held.
    #[serde(default)]
    pub credits: u64,
    /// The shader it held instead of a card (14/10/2026), or beside its card: a Solar
    /// copy of a blade card drops its blade too (20/10/2026), and then both are set.
    #[serde(default)]
    pub unlock: Option<OpenedUnlock>,
    /// The kyber shards it held instead of a card (17/10/2026); 0 for a card or a shader
    /// (the hub leaves the field out then).
    #[serde(default)]
    pub shards: u64,
}

/// A shader an opening brought.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct OpenedUnlock {
    /// The unlockable's id (`saber_tidal`).
    pub id: String,
    /// Its name ("Tidal blade").
    #[serde(default)]
    pub name: String,
    /// How many of it the key holds now, this one included.
    #[serde(default = "one")]
    pub count: u32,
    /// A dropped tag's finish when it is not Normal (`holo` or `solar`, since 31/10/2026);
    /// `None` for a Normal one and a shader, and from hubs before.
    #[serde(default)]
    pub finish: Option<String>,
}

/// A sealed copy the hub revealed (`POST /v1/cards/reveal`): the card it turned out to be.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Revealed {
    /// The copy's number at the hub.
    pub id: u64,
    /// The card it is now.
    pub card: String,
    /// Its rarity, as it was sealed.
    pub rarity: String,
    /// Its finish, as it was sealed.
    pub finish: String,
    /// Its edition, as it was sealed.
    #[serde(default)]
    pub edition: String,
    /// The shader it dropped as it was revealed: a Solar copy of a blade card drops its
    /// blade (20/10/2026); `None` otherwise, and from older hubs.
    #[serde(default)]
    pub unlock: Option<OpenedUnlock>,
}

/// The hub's answer to `POST /v1/cards/reveal`: the copies revealed (none when no sealed
/// copy has a card of its rarity yet) and the person's profile after.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct RevealAnswer {
    /// The copies revealed, oldest first.
    pub revealed: Vec<Revealed>,
    /// The person's profile after the reveal.
    pub profile: Profile,
}

/// The hub's answer to `POST /v1/holocrons/open`: what each holocron brought, in the
/// order asked, and the person's profile after.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct OpenAnswer {
    /// What each holocron brought.
    pub opened: Vec<Opened>,
    /// The person's profile after the opening.
    pub profile: Profile,
}

/// The hub's answer to `GET /v1/holocrons` (`PROTOCOL.md`, "Holocrons"): how far the
/// own key is from its next holocron, and the odds the hub rolls with.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct HolocronState {
    /// Seconds of active play carried toward the next holocron.
    pub progress_secs: u64,
    /// Seconds of active play for one holocron.
    pub every_secs: u64,
    /// Holocrons dropped since the caps' day began (09:00 in Paris; hubs before 29/10/2026:
    /// in the last 24 hours).
    #[serde(default)]
    pub today: u32,
    /// The most holocrons a key can earn in a day.
    #[serde(default)]
    pub daily_cap: u32,
    /// When the caps' next day begins and `today` starts again from 0, unix seconds
    /// (29/10/2026); `None` from hubs before, whose caps looked back a rolling 24 hours.
    #[serde(default)]
    pub resets_at: Option<i64>,
    /// The hub's clock, unix seconds.
    #[serde(default)]
    pub server_time: i64,
    /// Whether the person may open holocrons now: only staff while opening is tested.
    /// Hubs before it leave it out, read as `false`.
    #[serde(default)]
    pub can_open: bool,
    /// The oldest SJK release the hub lets open holocrons and trade (21/10/2026; the hub's
    /// `PROTOCOL.md`, "Minimum client"); `None` when it wants none, and from hubs before.
    #[serde(default)]
    pub min_client: Option<String>,
    /// Every odds table the hub rolls with, one per holocron tier in the tiers' order
    /// (14/10/2026); empty from hubs before.
    #[serde(default)]
    pub loot: Vec<LootTier>,
    /// The kyber shards' and the shop's tables (17/10/2026); `None` from hubs before.
    #[serde(default)]
    pub economy: Option<EconomyTables>,
    /// The kyber crystals' odds and the person's pity counter (21/10/2026; the hub's
    /// `PROTOCOL.md`, "Kyber crystals"); `None` from hubs before.
    #[serde(default)]
    pub kyber: Option<KyberTables>,
    /// The person's collection marks (29/10/2026; the hub's `PROTOCOL.md`, "Collection
    /// marks"): their favourite and locked saber shaders. `None` from hubs before, which
    /// keep none (their economy does not say [`MARKS_ABILITY`]).
    #[serde(default)]
    pub marks: Option<UnlockMarks>,
    /// Today's catch-up (03/11/2026, Rollembones' idea; the hub's `PROTOCOL.md`,
    /// "Holocrons", Catch-up): after a short day the first holocrons of the next come
    /// twice as fast. `None` from hubs before.
    #[serde(default)]
    pub catchup: Option<Catchup>,
    /// How the hub judged the key's last claims' play (03/11/2026; "Holocrons", Play
    /// signals). `None` from hubs before.
    #[serde(default)]
    pub activity: Option<ActivityState>,
}

impl HolocronState {
    /// The active seconds the next holocron needs: the catch-up's interval while some
    /// catch-up drops are left today, else [`Self::every_secs`].
    pub fn next_every(&self) -> u64 {
        match &self.catchup {
            Some(catchup) if catchup.left > 0 && catchup.every_secs > 0 => catchup.every_secs,
            _ => self.every_secs,
        }
    }
}

/// Today's holocron catch-up as the hub counts it: `pool` drops the previous day left
/// short of the cap (at most `max`), `left` of them still to come today at `every_secs`
/// of active play each. Catch-up drops count toward the daily cap.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Catchup {
    /// Today's pool.
    #[serde(default)]
    pub pool: u32,
    /// How many are left today.
    #[serde(default)]
    pub left: u32,
    /// The active seconds a catch-up holocron needs.
    #[serde(default)]
    pub every_secs: u64,
    /// The largest pool a day can bring.
    #[serde(default)]
    pub max: u32,
}

/// How the hub judges play from the claims' signals ([`PlaySignals`]).
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ActivityState {
    /// The percent of time the key's last active claim earned: 100 at full pace, the
    /// peaceful share without a fight lately, 0 idle. `None` until the hub saw signals.
    #[serde(default)]
    pub pace: Option<u32>,
    /// When the key last fought (hit, was hit, killed or died), unix seconds; 0 never.
    #[serde(default)]
    pub last_combat: i64,
    /// How long a fight keeps the pace full, in seconds.
    #[serde(default)]
    pub combat_window: i64,
    /// The percent a peaceful claim earns (moving, no fight within the window).
    #[serde(default)]
    pub peaceful: u32,
    /// The distance a claim's moves must add up to, in game units, to count as moving.
    #[serde(default)]
    pub move_min: u64,
}

/// What the player did since the last accepted claim, sent with an active claim
/// (`"play"`, 03/11/2026): the hub counts the time toward holocrons by them, not by
/// presence alone.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct PlaySignals {
    /// Horizontal distance travelled, game units.
    pub moved: u64,
    /// Attack and alt attack presses.
    pub attacks: u64,
    /// Hits on foes (the game's hit counter).
    pub hits: u64,
    /// Times another player hurt the player.
    pub hurt: u64,
    /// Kills of other players.
    pub kills: u64,
    /// Deaths at another player's hand.
    pub deaths: u64,
}

impl PlaySignals {
    /// `other` added to these, saturating.
    pub fn add(&mut self, other: &Self) {
        self.moved = self.moved.saturating_add(other.moved);
        self.attacks = self.attacks.saturating_add(other.attacks);
        self.hits = self.hits.saturating_add(other.hits);
        self.hurt = self.hurt.saturating_add(other.hurt);
        self.kills = self.kills.saturating_add(other.kills);
        self.deaths = self.deaths.saturating_add(other.deaths);
    }

    /// Whether any fight shows: a hit, a hurt, a kill or a death.
    pub fn combat(&self) -> bool {
        self.hits > 0 || self.hurt > 0 || self.kills > 0 || self.deaths > 0
    }
}

/// The economy's ability that says the hub breaks down, crafts and trades tag copies
/// (03/11/2026, Blep's idea).
pub const TAG_COPIES_ABILITY: &str = "tag_copies";

/// One copy of a tag in a finish (`normal`, `holo`, `solar`), as the tag requests name it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct TagCopy {
    /// The tag's id.
    pub id: String,
    /// Its finish.
    #[serde(default = "normal_finish")]
    pub finish: String,
}

impl TagCopy {
    /// A copy of tag `id` in `finish`.
    pub fn new(id: &str, finish: &str) -> Self {
        Self {
            id: id.to_owned(),
            finish: finish.to_owned(),
        }
    }
}

fn normal_finish() -> String {
    "normal".to_owned()
}

/// What breaking a tag copy gives: `shatter` by tag tier, common to mythical, times the
/// copy's finish's multiplier (`finish`, by finish id).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct TagTables {
    /// Shards by tier.
    #[serde(default)]
    pub shatter: Vec<u64>,
    /// Multipliers by finish id.
    #[serde(default)]
    pub finish: std::collections::BTreeMap<String, u64>,
}

impl TagTables {
    /// The shards a copy of tier index `tier` in `finish` breaks into, if the tables say.
    pub fn yield_of(&self, tier: usize, finish: &str) -> Option<u64> {
        let base = *self.shatter.get(tier)?;
        let times = self.finish.get(finish).copied().unwrap_or(1);
        Some(base.saturating_mul(times))
    }
}

/// A tag copy the hub broke down (`POST /v1/tags/shatter`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TagShattered {
    /// The tag's id.
    pub id: String,
    /// The copy's finish.
    #[serde(default = "normal_finish")]
    pub finish: String,
    /// How many copies of the tag the person holds now.
    #[serde(default)]
    pub count: u32,
    /// The shards it gave.
    #[serde(default)]
    pub shards: u64,
    /// The person's shards after.
    #[serde(default)]
    pub balance: u64,
}

/// The hub's answer to `POST /v1/tags/shatter`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TagShatterAnswer {
    /// What it gave.
    pub shattered: TagShattered,
    /// The person's profile after.
    pub profile: Profile,
}

/// A tag the hub crafted from two copies (`POST /v1/tags/craft`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TagCrafted {
    /// The new tag's id.
    pub id: String,
    /// Its name.
    #[serde(default)]
    pub name: String,
    /// Its tier.
    #[serde(default)]
    pub tier: String,
    /// Its finish, the copies' own.
    #[serde(default = "normal_finish")]
    pub finish: String,
    /// The two copies given for it.
    #[serde(default)]
    pub gave: Vec<TagCopy>,
}

/// The hub's answer to `POST /v1/tags/craft`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TagCraftAnswer {
    /// What was made.
    pub crafted: TagCrafted,
    /// The person's profile after.
    pub profile: Profile,
}

/// The economy's ability that says the hub keeps collection marks ([`UnlockMarks`]).
pub const MARKS_ABILITY: &str = "marks";

/// A person's collection marks as the hub keeps them (`PROTOCOL.md`, "Collection marks",
/// 29/10/2026): their favourite saber shaders (listed first) and the ones they locked,
/// which the hub never shatters or lets go in a trade until they are unlocked. Each list
/// in the catalogue's order.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct UnlockMarks {
    /// The favourite shaders' ids.
    #[serde(default)]
    pub favourite: Vec<String>,
    /// The locked shaders' ids.
    #[serde(default)]
    pub locked: Vec<String>,
}

impl UnlockMarks {
    /// Whether shader `id` is a favourite.
    pub fn favourite(&self, id: &str) -> bool {
        self.favourite.iter().any(|known| known == id)
    }

    /// Whether shader `id` is locked.
    pub fn locked(&self, id: &str) -> bool {
        self.locked.iter().any(|known| known == id)
    }

    /// `mark` set (`on`) or cleared on shader `id`, as the hub will answer it (the
    /// client shows a change before the hub's answer); whether anything changed.
    pub fn set(&mut self, id: &str, mark: Mark, on: bool) -> bool {
        let list = match mark {
            Mark::Favourite => &mut self.favourite,
            Mark::Locked => &mut self.locked,
        };
        let at = list.iter().position(|known| known == id);
        match (at, on) {
            (Some(_), true) | (None, false) => false,
            (Some(at), false) => {
                list.remove(at);
                true
            }
            (None, true) => {
                list.push(id.to_owned());
                true
            }
        }
    }
}

/// A collection mark (`PROTOCOL.md`, "Collection marks").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Mark {
    /// A star: the shader is listed first.
    Favourite,
    /// A padlock: the shader is never broken down or traded until unlocked.
    Locked,
}

impl Mark {
    /// Its wire id.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Favourite => "favourite",
            Self::Locked => "locked",
        }
    }
}

/// The hub's answer to `POST /v1/unlocks/marks`: every mark after.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MarksAnswer {
    /// The person's marks after the change.
    pub marks: UnlockMarks,
}

/// The kyber crystals' tables as the hub sends them with the progress (`PROTOCOL.md`,
/// "Kyber crystals", 21/10/2026). A crystal is a holocron row of tier `kyber_uncommon` to
/// `kyber_mythical`, earned from achievements; opening one always gives a saber shader.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct KyberTables {
    /// Each crystal tier's odds, the commonest first.
    #[serde(default)]
    pub tiers: Vec<KyberTier>,
    /// The person's pity counter; `None` when the hub keeps none.
    #[serde(default)]
    pub pity: Option<KyberPity>,
}

/// One crystal tier's odds.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct KyberTier {
    /// The tier's wire id (`kyber_rare`).
    pub tier: String,
    /// The shader's tier per mille, common to mythical (summing to 1000).
    #[serde(default)]
    pub shader_tier: Vec<u32>,
}

/// The person's pity counter: the `every`-th crystal in a row without a Legendary or
/// better shader gives one.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct KyberPity {
    /// How many crystals in a row make a Legendary or better sure.
    #[serde(default)]
    pub every: u32,
    /// Crystals opened since the last that gave Legendary or better (0 to `every - 1`).
    #[serde(default)]
    pub since: u32,
}

/// The economy's tables as the hub sends them with the progress (`PROTOCOL.md`, "Shards
/// and the shop", 17/10/2026), each by saber shader tier, common to mythical.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct EconomyTables {
    /// The shards shattering one duplicate gives.
    #[serde(default)]
    pub shatter: Vec<u64>,
    /// The shards crafting one costs.
    #[serde(default)]
    pub craft: Vec<u64>,
    /// What one costs in the shop, in [`EconomyTables::shop_currency`].
    #[serde(default)]
    pub shop: Vec<u64>,
    /// What the shop's prices are in: `shards` (since 17/10/2026); empty from hubs before,
    /// which sold for credits.
    #[serde(default)]
    pub shop_currency: String,
    /// What a holocron costs in credits, by tier id (`uncommon`, `rare`); a tier left out
    /// is not for sale. Empty from hubs before.
    #[serde(default)]
    pub holocrons: std::collections::BTreeMap<String, u64>,
    /// What crafting a shader costs while the person holds a copy of its blade card, in
    /// per cent of the craft cost, by the best copy's finish id (`normal`, `holo`,
    /// `solar`; 21/10/2026). Empty from hubs before, which take the full cost.
    #[serde(default)]
    pub blade_card_craft: std::collections::BTreeMap<String, u64>,
    /// What the hub's trade can do beyond its first form (21/10/2026): `shatter_last`
    /// (the last copy shatters when asked, [`crate::Service::shatter_last`]) and
    /// `shatter_duplicates` ([`crate::Service::shatter_duplicates`]). Empty from hubs
    /// before.
    #[serde(default)]
    pub abilities: Vec<String>,
    /// What breaking a tag copy gives (03/11/2026, with [`TAG_COPIES_ABILITY`]); `None`
    /// from hubs before.
    #[serde(default)]
    pub tags: Option<TagTables>,
}

impl EconomyTables {
    /// Whether the hub's trade can do `ability` (`shatter_last`, `shatter_duplicates`).
    pub fn can(&self, ability: &str) -> bool {
        self.abilities.iter().any(|known| known == ability)
    }
}

/// A duplicate the hub shattered (`POST /v1/unlocks/shatter`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Shattered {
    /// The shader's id.
    pub id: String,
    /// How many of it the person holds now.
    #[serde(default)]
    pub count: u32,
    /// The shards it gave.
    #[serde(default)]
    pub shards: u64,
    /// The person's shards after.
    #[serde(default)]
    pub balance: u64,
}

/// The hub's answer to `POST /v1/unlocks/shatter`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ShatterAnswer {
    /// What the shatter gave.
    pub shattered: Shattered,
    /// The person's profile after.
    pub profile: Profile,
}

/// The copies of one shader the hub shattered at once (`POST
/// /v1/unlocks/shatter-duplicates`, 21/10/2026).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ShatteredCopies {
    /// The shader's id.
    pub id: String,
    /// How many copies it gave up (one is kept).
    #[serde(default)]
    pub copies: u32,
    /// The shards they gave.
    #[serde(default)]
    pub shards: u64,
}

/// The hub's answer to `POST /v1/unlocks/shatter-duplicates`: every duplicate shattered,
/// one copy of each kept.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ShatterAllAnswer {
    /// Each shader that gave up copies.
    #[serde(default)]
    pub shattered: Vec<ShatteredCopies>,
    /// The copies shattered in all.
    #[serde(default)]
    pub copies: u32,
    /// The shards they gave in all.
    #[serde(default)]
    pub shards: u64,
    /// The person's shards after.
    #[serde(default)]
    pub balance: u64,
    /// The person's profile after.
    pub profile: Profile,
}

/// A shader the hub crafted with the person's shards (`POST /v1/unlocks/craft`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Crafted {
    /// The shader's id.
    pub id: String,
    /// Its name ("Circuit blade").
    #[serde(default)]
    pub name: String,
    /// How many of it the person holds now.
    #[serde(default = "one")]
    pub count: u32,
    /// The shards it cost.
    #[serde(default)]
    pub cost: u64,
    /// The person's shards after.
    #[serde(default)]
    pub balance: u64,
}

/// The hub's answer to `POST /v1/unlocks/craft`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct CraftAnswer {
    /// What was made.
    pub crafted: Crafted,
    /// The person's profile after.
    pub profile: Profile,
}

/// One of the shop's offers of the day, as the person asking sees it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Offer {
    /// The shader's id.
    pub id: String,
    /// Its name ("Aurora blade").
    #[serde(default)]
    pub name: String,
    /// Its tier's id (`common` to `mythical`).
    #[serde(default)]
    pub tier: String,
    /// Its price, in the shop's currency ([`EconomyTables::shop_currency`]).
    #[serde(default)]
    pub price: u64,
    /// Whether the person bought it today.
    #[serde(default)]
    pub bought: bool,
    /// Whether the person holds it already (then the hub will not sell it).
    #[serde(default)]
    pub owned: bool,
}

/// The hub's answer to `GET /v1/shop` (`PROTOCOL.md`, "Shards and the shop"): the day's
/// offers (each person's own since 21/10/2026, the same for everyone before), and when
/// the next day's come.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct ShopDay {
    /// The day in Paris, `YYYY-MM-DD`.
    #[serde(default)]
    pub day: String,
    /// The offers, one Common, one Uncommon, one Rare and one Legendary or Mythical.
    #[serde(default)]
    pub offers: Vec<Offer>,
    /// When the offers change, unix seconds.
    #[serde(default)]
    pub next: i64,
    /// The hub's clock when it answered.
    #[serde(default)]
    pub server_time: i64,
    /// The person's credits then.
    #[serde(default)]
    pub credits: u64,
    /// The person's kyber shards then.
    #[serde(default)]
    pub shards: u64,
}

/// A shader the person bought in the shop (`POST /v1/shop/buy`).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Bought {
    /// The shader's id.
    pub id: String,
    /// Its name.
    #[serde(default)]
    pub name: String,
    /// How many of it the person holds now.
    #[serde(default = "one")]
    pub count: u32,
    /// What it cost, in the shop's currency.
    #[serde(default)]
    pub price: u64,
    /// The person's balance of that currency after.
    #[serde(default)]
    pub balance: u64,
}

/// The hub's answer to `POST /v1/shop/buy`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct BuyAnswer {
    /// What was bought.
    pub bought: Bought,
    /// The person's profile after.
    pub profile: Profile,
}

/// A holocron the person bought with credits (`POST /v1/holocrons/buy`, 17/10/2026).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct HolocronBought {
    /// The new holocron's id.
    #[serde(default)]
    pub holocron: i64,
    /// Its tier's id.
    #[serde(default)]
    pub tier: String,
    /// The credits it cost.
    #[serde(default)]
    pub price: u64,
    /// The person's credits after.
    #[serde(default)]
    pub balance: u64,
}

/// The hub's answer to `POST /v1/holocrons/buy`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct HolocronBuyAnswer {
    /// What was bought.
    pub bought: HolocronBought,
    /// The person's profile after.
    pub profile: Profile,
}

/// One holocron tier's odds as the hub rolls them (`PROTOCOL.md`, "Holocrons"), every
/// share per mille.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct LootTier {
    /// Independent banner bonus chance per mille, absent on older hubs.
    #[serde(default)]
    pub banner: u32,
    /// The tier's wire id.
    pub tier: String,
    /// How often a drop is this tier.
    pub drop: u32,
    /// The card's rarity (common to mythical; common to legendary before 19/10/2026).
    pub rarity: Vec<u32>,
    /// The card's finish (normal, holo, solar).
    pub finish: Vec<u32>,
    /// The credits a card opening gives: the least and the most.
    pub credits: [u64; 2],
    /// How often an opening brings a saber shader instead of the card and credits.
    #[serde(default)]
    pub shader: u32,
    /// The shader's tier then (common to mythical).
    #[serde(default)]
    pub shader_tier: Vec<u32>,
    /// How often an opening brings a pile of kyber shards instead of the card and credits
    /// (17/10/2026; 0 from hubs before).
    #[serde(default)]
    pub shard_pile: u32,
    /// How often an opening brings a tag instead of the card and credits (27/10/2026; 0
    /// from hubs before).
    #[serde(default)]
    pub tag: u32,
    /// The shards a pile holds: the least and the most.
    #[serde(default)]
    pub shards: [u64; 2],
    /// The shards a card brings beside its credits, the card pack: the least and the most
    /// (17/10/2026; none from hubs before).
    #[serde(default)]
    pub card_shards: [u64; 2],
    /// How often a dropped tag comes Holo and Solar, per mille, in that order (since
    /// 31/10/2026; none from hubs before, whose tags are all Normal).
    #[serde(default)]
    pub tag_finish: [u32; 2],
}

/// A holocron dropping for a key, as the feed relays it (`PROTOCOL.md`, "The feed").
/// Legendary and mythical drops reach every reader; uncommon and rare ones only the
/// reader whose key it is.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct DropEvent {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When it dropped, unix seconds.
    pub at: i64,
    /// The key it dropped for.
    pub key_id: String,
    /// That key's hub display name, with colour codes. Show it only through
    /// [`crate::chat::for_display`].
    pub name: String,
    /// The tier's wire id.
    pub tier: String,
    /// Whether the hub's operator vouches for the key.
    #[serde(default)]
    pub verified: bool,
}

/// One unlockable a key holds (`PROTOCOL.md`, "Unlocks"): granted by the hub's
/// operator or staff, or brought by a medal or a card the key holds. A client shows and wears only the ids it knows.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Unlock {
    /// The unlockable's id (`saber_sun`).
    pub id: String,
    /// When it was granted, unix seconds.
    #[serde(default)]
    pub granted: i64,
    /// Public plain text from the team, often empty. Never markup.
    #[serde(default)]
    pub note: String,
    /// How many the key holds (14/10/2026: a holocron may drop the same shader again);
    /// 1 from hubs before.
    #[serde(default = "one")]
    pub count: u32,
    /// The medal that brings it (`bug_hunter`), when the key holds it only through that
    /// medal: it goes with the medal. `None` for a grant by hand, and from hubs before
    /// 10/10/2026.
    #[serde(default)]
    pub medal: Option<String>,
    /// The card that brings it (`sjk_alpha_000`), when the key holds it only through a
    /// copy of that card: it goes with the last copy, as a medal's blade goes with the
    /// medal. `None` otherwise, and from hubs before 20/10/2026.
    #[serde(default)]
    pub card: Option<String>,
    /// How many of its copies are Holo and Solar, among [`Self::count`] (a tag's, since
    /// 31/10/2026: holocrons drop tags in a finish now and then); 0 from hubs before.
    #[serde(default)]
    pub holo: u32,
    /// How many are Solar, among [`Self::count`].
    #[serde(default)]
    pub solar: u32,
}

/// What a player wears that others draw (`PROTOCOL.md`, "Looks"). The default is
/// the stock blade with the holocron out, which is also what a claim without a
/// look means.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Look {
    /// A blade-skin unlock id, or empty for the stock blade.
    #[serde(default)]
    pub saber: String,
    /// Whether the Illuminate holocron is lit.
    #[serde(default)]
    pub illuminate: bool,
    /// The blade skin's options its wearer switched off (since 11/10/2026; absent, and
    /// empty, from older hubs and from wearers with every part on).
    #[serde(default)]
    pub saber_off: Vec<String>,
    /// The toys the player has out that others draw, by id (`cardboard_box`; since
    /// 16/10/2026; absent, and empty, from older hubs and from players with none out).
    #[serde(default)]
    pub toys: Vec<String>,
    /// The second saber's blade skin (since 21/10/2026): `None` (absent) when both sabers
    /// wear `saber`, else a blade-skin unlock id or empty for the stock blade.
    #[serde(default)]
    pub saber2: Option<String>,
    /// The second saber's skin's options its wearer switched off; empty when absent.
    #[serde(default)]
    pub saber2_off: Vec<String>,
    /// The blade skin's extras its wearer switched on (since 26/10/2026): parts that are
    /// off until switched on and that only a holder of the skin's card in its finish may
    /// switch on (the Sun's: a Solar copy of SJK Alpha 000). The hub relays only those
    /// the wearer may wear; absent, and empty, from older hubs and wearers with none on.
    #[serde(default)]
    pub saber_on: Vec<String>,
    /// The second saber's skin's extras switched on; empty when absent.
    #[serde(default)]
    pub saber2_on: Vec<String>,
    /// The saber sounds the player chose apart from their blade (since 23/10/2026):
    /// empty to follow the blade skin (absent from older hubs and clients), `stock` for
    /// the retail sounds, or a blade-skin unlock id whose sounds they hear.
    #[serde(default)]
    pub saber_sounds: String,
    /// The tag the player stuck on the world (since 27/10/2026); `None` when absent.
    #[serde(default)]
    pub tag: Option<PlacedTag>,
    /// The finisher the player wears (since 04/11/2026): what their saber kills do to
    /// the body, a finisher's unlock id; empty for none (absent from older hubs and
    /// clients).
    #[serde(default)]
    pub finisher: String,
}

/// A tag stuck on the world (`PROTOCOL.md`, "Looks", since 27/10/2026): a tag unlock the
/// wearer holds (or a free one), on map `map`, centred at `at` on a surface facing
/// `normal`, turned `spin` degrees about it. The hub checks the numbers are finite, so
/// the look stays `Eq`. Since 31/10/2026 it may be a card the wearer holds (a credits
/// card, its id in place of the tag's) and may have a finish.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlacedTag {
    /// The tag's unlock id, or since 31/10/2026 a card's id (`sjk_alpha_003`).
    pub id: String,
    /// The map it is stuck on (`mp/ffa3`).
    pub map: String,
    /// Its centre, in world units.
    pub at: [f32; 3],
    /// The surface's normal, a unit vector.
    pub normal: [f32; 3],
    /// Its turn about the normal, in degrees from 0 up to 360.
    pub spin: f32,
    /// Its finish, `holo` or `solar` (since 31/10/2026); `None` for Normal, which is
    /// also how a hub before sends every tag. Sent only when set: a hub before refuses it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<String>,
}

impl Eq for PlacedTag {}

/// A look worn on a game server, as the feed relays it when it changes: the slot and
/// name come from the wearer's claim, as an emote's do.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct LookEvent {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The wearer's slot on the server, from their claim.
    pub slot: u8,
    /// The name the wearer claimed the game shows there.
    pub claimed_name: String,
    /// The wearer's key id.
    pub key_id: String,
    /// The blade-skin unlock id, or empty for the stock blade.
    #[serde(default)]
    pub saber: String,
    /// Whether the Illuminate holocron is lit.
    #[serde(default)]
    pub illuminate: bool,
    /// The blade skin's options its wearer switched off; empty when absent.
    #[serde(default)]
    pub saber_off: Vec<String>,
    /// The toys the wearer has out; empty when absent.
    #[serde(default)]
    pub toys: Vec<String>,
    /// The second saber's blade skin; `None` when both wear `saber`.
    #[serde(default)]
    pub saber2: Option<String>,
    /// The second saber's skin's options switched off; empty when absent.
    #[serde(default)]
    pub saber2_off: Vec<String>,
    /// The blade skin's extras switched on; empty when absent.
    #[serde(default)]
    pub saber_on: Vec<String>,
    /// The second saber's skin's extras switched on; empty when absent.
    #[serde(default)]
    pub saber2_on: Vec<String>,
    /// The saber sounds the wearer chose; empty (follow the blade) when absent.
    #[serde(default)]
    pub saber_sounds: String,
    /// The tag the wearer stuck on the world; `None` when absent.
    #[serde(default)]
    pub tag: Option<PlacedTag>,
    /// The finisher the wearer wears (since 04/11/2026); empty when absent.
    #[serde(default)]
    pub finisher: String,
}

impl LookEvent {
    /// The look it carries.
    pub fn look(&self) -> Look {
        Look {
            saber: self.saber.clone(),
            illuminate: self.illuminate,
            saber_off: self.saber_off.clone(),
            toys: self.toys.clone(),
            saber2: self.saber2.clone(),
            saber2_off: self.saber2_off.clone(),
            saber_on: self.saber_on.clone(),
            saber2_on: self.saber2_on.clone(),
            saber_sounds: self.saber_sounds.clone(),
            tag: self.tag.clone(),
            finisher: self.finisher.clone(),
        }
    }
}

/// Something one SJK player's toy did to another player (`PROTOCOL.md`, "Toy events"),
/// as the feed relays it: the sender's slot and name come from their claim, as an
/// emote's do. The first is `cardboard_box`'s `spotted`: the sender lifted the box off
/// the player in `target`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ToyEvent {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The sender's slot on the server, from their claim.
    pub slot: u8,
    /// The name the sender claimed the game shows there.
    pub claimed_name: String,
    /// The sender's key id.
    pub key_id: String,
    /// The toy's id (`cardboard_box`).
    pub toy: String,
    /// What happened (`spotted`).
    pub event: String,
    /// The slot of the player it happened to.
    pub target: u8,
}

/// An answer of the toy relay (`PROTOCOL.md`, "Toy relay"): the events for the
/// reader's own slot, and the id to read after next.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct RelayBatch {
    /// The events, oldest first.
    pub events: Vec<ToyEvent>,
    /// The encrypted whispers sealed to the reading key, oldest first, when asked for
    /// (`PROTOCOL.md`, "Whispers").
    #[serde(default)]
    pub whispers: Vec<SealedWhisper>,
    /// The id the next read starts after.
    pub next: u64,
}

/// An encrypted whisper as the hub hands it to its reader (`PROTOCOL.md`, "Whispers"):
/// open it with [`crate::whisper::open`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SealedWhisper {
    /// Its id in the relay's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The sender's slot and the name they claimed it under.
    pub slot: u8,
    /// The name the sender's claim was made under.
    pub claimed_name: String,
    /// The sender's key id (their person's).
    pub key_id: String,
    /// The key that sealed it (unpadded base64url).
    pub from: String,
    /// The sealed whisper.
    #[serde(rename = "box")]
    pub boxed: String,
}

/// A milestone of the player's own play (`PROTOCOL.md`, "Achievements"): how far its
/// count is and when it was unlocked. Recognition only, it grants nothing.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Achievement {
    /// The achievement's id (`first_blood`, `kills_100`, ...).
    pub id: String,
    /// The count the hub took, at most `goal`.
    pub progress: u64,
    /// The count that unlocks it.
    #[serde(default)]
    pub goal: u64,
    /// When the count first reached the goal, unix seconds; 0 while it has not.
    #[serde(default)]
    pub unlocked: i64,
    /// The kyber crystal tier it brings once (`kyber_rare`; 21/10/2026); `None` from hubs
    /// before, whose players read the client's own table.
    #[serde(default)]
    pub reward: Option<String>,
    /// For an achievement the hub alone knows (since 30/10/2026, the Toy Master's: a
    /// client's catalogue cannot hold a secret), sent only once it is unlocked: its name,
    /// what it was for and the group it shows in ([`TOY_MASTER`]). `None` for the others.
    #[serde(default)]
    pub name: Option<String>,
    /// What it was for, in a sentence (an achievement the hub alone knows).
    #[serde(default)]
    pub description: Option<String>,
    /// The group it shows in (an achievement the hub alone knows).
    #[serde(default)]
    pub group: Option<String>,
}

/// The group of the achievements the Toy Master's secrets unlock ([`Achievement::group`]).
pub const TOY_MASTER: &str = "toy_master";

/// The hub's answer to `POST /v1/staff/search`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Players {
    /// The players found, most recently seen first.
    pub players: Vec<Profile>,
}

/// The hub's answer to `PUT /v1/achievements`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Achievements {
    /// The key's achievements, as a profile lists them.
    pub achievements: Vec<Achievement>,
}

/// One medal the SJK team gave a key: recognition, and some bring a saber blade (an
/// [`Unlock`] with `medal`) while held.
///
/// A profile carries every field; a presence entry only `id`, `count` and `rank`, the
/// rest keeping their defaults. A client shows only the ids it knows.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Medal {
    /// The medal's id (`early_tester`, `bug_hunter`, ...): a wire id, which keeps its
    /// old wording ("Early Tester" reads "Tester" since 27/10/2026).
    pub id: String,
    /// How many times it was given: 1 unless the medal is repeatable.
    #[serde(default = "one")]
    pub count: u32,
    /// When it was last given, unix seconds; 0 when not sent (presence).
    #[serde(default)]
    pub awarded: i64,
    /// Public plain text from the team about why, often empty. Never markup.
    #[serde(default)]
    pub note: String,
    /// The rank the SJK team set, [`MEDAL_RANK_MIN`] to [`MEDAL_RANK_MAX`] (since
    /// 27/10/2026): 1 from older hubs, and a value out of range is brought into it.
    #[serde(default = "rank_one", deserialize_with = "medal_rank")]
    pub rank: u8,
}

/// The lowest medal rank (Issued).
pub const MEDAL_RANK_MIN: u8 = 1;
/// The highest medal rank (Solar).
pub const MEDAL_RANK_MAX: u8 = 5;

fn one() -> u32 {
    1
}

fn rank_one() -> u8 {
    MEDAL_RANK_MIN
}

/// A medal's rank as sent, clamped into 1..=5; a rank that is not a number reads as 1.
fn medal_rank<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u8, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value.as_f64() {
        Some(rank) => rank.clamp(f64::from(MEDAL_RANK_MIN), f64::from(MEDAL_RANK_MAX)) as u8,
        None => MEDAL_RANK_MIN,
    })
}

/// One in-game name a key has worn, as the hub saw it in registrations and claims.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct WornName {
    /// The name with its colour codes.
    pub name: String,
    /// When the hub first saw it, unix seconds.
    pub first_seen: i64,
    /// When the hub last saw it, unix seconds.
    pub last_seen: i64,
}

/// A player SJK staff blocked for everyone on a game server until its map changes
/// (`PROTOCOL.md`, "Map blocks"), as presence gives it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct MapBlock {
    /// The map, as the game names it.
    pub map: String,
    /// The server's `sv_serverid` for that map load.
    pub serverid: i64,
    /// The blocked player's slot.
    pub slot: u8,
    /// The name the game showed in `slot` when they were blocked.
    pub name: String,
    /// When, in Unix seconds.
    #[serde(default)]
    pub at: i64,
}

/// One live claim on a game server and who made it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Presence {
    /// The game slot the claimant says they hold.
    pub slot: u8,
    /// The name the claimant says the game shows for them.
    pub claimed_name: String,
    /// The claimant's key id.
    pub key_id: String,
    /// The claimant's display name.
    pub name: String,
    /// Whether the hub's operator vouches for the claimant.
    pub verified: bool,
    /// The claimant's tick as the hub sends it (absent from older hubs); draw
    /// [`Tick::shown`].
    #[serde(default)]
    pub tick: Tick,
    /// The claimant's trust, `None` when verified or staff (and from older hubs).
    #[serde(default)]
    pub trust: Option<Trust>,
    /// The claimant's name emoji, empty for none (and from older hubs).
    #[serde(default)]
    pub emoji: String,
    /// The claimant's medals, ids and counts only (absent from hubs older than
    /// medals), so the scoreboard shows them without a request per player.
    #[serde(default)]
    pub medals: Vec<Medal>,
    /// The version of the claimant's picture, empty for none (and from hubs older than
    /// pictures), so a client fetches it without a profile per player.
    #[serde(default)]
    pub avatar: String,
    /// What the claimant wears, when their claim has a look (absent otherwise, and
    /// from hubs older than looks).
    #[serde(default)]
    pub look: Option<Look>,
    /// The key the claimant takes encrypted whispers with (unpadded base64url), while
    /// they do (`PROTOCOL.md`, "Whispers"); absent otherwise, and from older hubs.
    #[serde(default)]
    pub whisper_key: Option<String>,
    /// The client the claim came from, as chat messages name it (`ChatMessage::client`):
    /// `sjk`, `ejk`, empty for another (and from hubs before 27/10/2026).
    #[serde(default)]
    pub client: String,
}

/// One SJK chat message (`PROTOCOL.md`, "Chat"), as the feed carries it. Show its
/// `name` and `text` only through [`crate::chat::for_display`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct ChatMessage {
    /// Shared emoji reactions; absent from older hubs.
    #[serde(default)]
    pub reactions: crate::reactions::State,
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The sender's key id.
    pub key_id: String,
    /// The in-game name the sender wore, else their hub name.
    pub name: String,
    /// Whether the hub's operator vouches for the sender.
    #[serde(default)]
    pub verified: bool,
    /// Whether the sender is SJK staff.
    #[serde(default)]
    pub staff: bool,
    /// Whether the sender is SJK's creator.
    #[serde(default)]
    pub creator: bool,
    /// The sender's tick when they sent it (absent from older hubs); draw
    /// [`Tick::shown`].
    #[serde(default)]
    pub tick: Tick,
    /// The sender's trust when they sent it, `None` when verified or staff.
    #[serde(default)]
    pub trust: Option<Trust>,
    /// The sender's name emoji when they sent it, empty for none.
    #[serde(default)]
    pub emoji: String,
    /// The client it was sent from, as the hub read it from the sender's User-Agent:
    /// `sjk`, `ejk` (JoF EternalJK), empty for another or from hubs before it.
    #[serde(default)]
    pub client: String,
    /// What they said.
    pub text: String,
    /// The channel it was said in: the SJK chat (the hub leaves the field out) or a
    /// clan chat ([`crate::channels`]).
    #[serde(default)]
    pub channel: crate::channels::HubChannel,
    /// Set when this "message" is a holocron drop the feed relayed (never read from
    /// the wire): `text` is empty and the viewer words the line.
    #[serde(skip)]
    pub holocron: Option<DropMark>,
    /// Set when this "message" is the SJK team's (an announcement or a reward round,
    /// [`crate::news`]; never read from the wire): `text` is empty and the viewer words
    /// the line.
    #[serde(skip)]
    pub official: Option<crate::news::Official>,
}

/// What marks a [`ChatMessage`] as a holocron drop, or a grand find of one opened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DropMark {
    /// The tier's wire id.
    pub tier: String,
    /// The drop is the reader's own.
    pub own: bool,
    /// Set when the line is a grand find ([`FindEvent`]): what the holocron gave.
    pub find: Option<FindMark>,
}

/// What a holocron someone opened gave, for a grand find's line ([`FindEvent`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindMark {
    /// A shader (`true`) or a card.
    pub shader: bool,
    /// The shader's name (empty for a card).
    pub name: String,
    /// The card's rarity or the shader's tier, as a wire id.
    pub rarity: String,
    /// The card's finish as a wire id (empty for a shader).
    pub finish: String,
    /// The card's id (empty for a shader or a sealed card): its face may name a rank
    /// in place of its rarity.
    pub card: String,
    /// The shader's unlock id (empty for a card, or from a hub that sends none): the
    /// chat draws that blade beside the line.
    pub skin: String,
}

/// A grand find (`PROTOCOL.md`, "The feed", `finds`; since 17/10/2026): a holocron someone
/// opened gave a Legendary, Mythical or Solar card, or a Legendary or Mythical shader.
/// Every reader is told of it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct FindEvent {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When it was opened, unix seconds.
    pub at: i64,
    /// The opener's key.
    pub key_id: String,
    /// The opener's hub display name, with colour codes. Show it only through
    /// [`crate::chat::for_display`].
    pub name: String,
    /// Whether the hub's operator vouches for the opener's key.
    #[serde(default)]
    pub verified: bool,
    /// The holocron's tier.
    pub tier: String,
    /// `card` or `shader`.
    pub prize: String,
    /// The shader's unlock id, or the card's id (empty while sealed).
    #[serde(default)]
    pub item: String,
    /// The shader's name (empty for a card).
    #[serde(default)]
    pub item_name: String,
    /// The card's rarity or the shader's tier.
    #[serde(default)]
    pub rarity: String,
    /// The card's finish (empty for a shader).
    #[serde(default)]
    pub finish: String,
}

/// One emote on a game server (`PROTOCOL.md`, "Emotes"): the slot and name come from
/// the sender's claim, so they say who the hub believes is in that slot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Emote {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The sender's slot on the server, from their claim.
    pub slot: u8,
    /// The name the sender claimed the game shows there.
    pub claimed_name: String,
    /// The sender's key id.
    pub key_id: String,
    /// The emote's id (`wave`); a client plays the ids it has.
    pub emote: String,
    /// The slot whose emote the sender joined in step (`sjkemote sync`), if any;
    /// absent from hubs before 15/10/2026.
    #[serde(default)]
    pub sync: Option<u8>,
    /// Played without its sound: the sender is away from the game; absent (false)
    /// from hubs before 19/10/2026.
    #[serde(default)]
    pub quiet: bool,
}

/// One ping on a game server (`PROTOCOL.md`, "Pings"): a point a player marked in the
/// world. The slot and name come from the sender's claim; `scope` is the sender's own
/// word for who should see it, which the reader checks against what its game shows.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Ping {
    /// Its id in the feed's sequence.
    pub id: u64,
    /// When the hub took it, unix seconds.
    pub at: i64,
    /// The sender's slot on the server, from their claim.
    pub slot: u8,
    /// The name the sender claimed the game shows there.
    pub claimed_name: String,
    /// The sender's key id.
    pub key_id: String,
    /// The point, in whole game units.
    pub pos: [i32; 3],
    /// Who the sender meant it for (`all`, `red`, `blue`, `lone`, `double`, `spec`).
    pub scope: String,
}

/// The values a player shares on their nameplate (`PROTOCOL.md`, "Vitals"): their
/// game's own numbers, as the server sends them to that player.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct Vitals {
    /// The game server's time (milliseconds) of the snapshot they come from.
    pub t: i32,
    /// Health (`STAT_HEALTH`).
    pub health: i32,
    /// Shield (`STAT_ARMOR`).
    pub armor: i32,
    /// Force (`fd.forcePower`).
    pub force: i32,
    /// `STAT_MAX_HEALTH`.
    pub max_health: i32,
    /// `fd.forcePowerMax`.
    pub max_force: i32,
}

/// One player in the hub's answer to `POST /v1/vitals`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SharedVitals {
    /// Their slot on the server, from their claim.
    pub slot: u8,
    /// The name they claimed the game shows there.
    pub claimed_name: String,
    /// Their id at the hub.
    pub key_id: String,
    /// They share their values.
    pub share: bool,
    /// They see everyone's estimated bars (which makes them share).
    pub estimates: bool,
    /// Seconds since the hub took the entry.
    #[serde(default)]
    pub age: i64,
    /// Their values; `None` while they have none (dead, spectating).
    #[serde(default)]
    pub vitals: Option<Vitals>,
}

/// The hub's answer to `GET /v1/feed` (`PROTOCOL.md`, "The feed").
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Feed {
    /// Replacement reaction state for retained messages; absent on older hubs.
    #[serde(default)]
    pub reaction_updates: Vec<crate::reactions::Update>,
    /// The `after` of the next request.
    pub next: u64,
    /// New messages, oldest first.
    #[serde(default)]
    pub chat: Vec<ChatMessage>,
    /// New emotes on the server asked about.
    #[serde(default)]
    pub emotes: Vec<Emote>,
    /// New pings on the server asked about (absent from hubs older than pings).
    #[serde(default)]
    pub pings: Vec<Ping>,
    /// New looks on the server asked about (absent from hubs older than looks).
    #[serde(default)]
    pub looks: Vec<LookEvent>,
    /// New toy events on the server asked about (absent from hubs older than them).
    #[serde(default)]
    pub toys: Vec<ToyEvent>,
    /// New holocron drops this reader is told of (absent from hubs older than
    /// holocrons).
    #[serde(default)]
    pub drops: Vec<DropEvent>,
    /// New grand finds (absent from hubs before 17/10/2026, and when none).
    #[serde(default)]
    pub finds: Vec<FindEvent>,
    /// Changes to the reader's own trades (absent from hubs before 22/10/2026, and when
    /// none).
    #[serde(default)]
    pub trades: Vec<crate::trades::TradeEvent>,
    /// New announcements, oldest first (absent from hubs before 25/10/2026, and when
    /// none).
    #[serde(default)]
    pub announcements: Vec<crate::news::FeedAnnouncement>,
    /// New reward rounds, oldest first (absent from hubs before 25/10/2026, and when
    /// none).
    #[serde(default)]
    pub rounds: Vec<crate::news::FeedRound>,
    /// Messages staff deleted.
    #[serde(default)]
    pub deleted: Vec<u64>,
    /// Keys that read the feed in the last minute.
    #[serde(default)]
    pub online: u32,
    /// Who reads the chat now and who read it last (absent from hubs that do not
    /// list them, and from answers the hub leaves them out of).
    #[serde(default)]
    pub people: Option<People>,
    /// The clan chats the reader may read ([`crate::channels`]; absent from hubs before
    /// 03/11/2026, and for a reader who may read none).
    #[serde(default)]
    pub channels: Vec<crate::channels::ListedChannel>,
}

/// The feed's `people` (`PROTOCOL.md`, "The feed"): the keys reading the feed now and
/// the ones that read it most recently.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct People {
    /// Keys that read the feed in the last minute, the same ones `online` counts.
    #[serde(default)]
    pub online: Vec<Person>,
    /// Keys that read it before that, most recently seen first.
    #[serde(default)]
    pub recent: Vec<Person>,
}

/// One SJK player of [`People`].
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct Person {
    /// Their key id.
    pub key_id: String,
    /// Their display name, as the hub's profiles say.
    #[serde(default)]
    pub name: String,
    /// Whether the hub's operator vouches for them.
    #[serde(default)]
    pub verified: bool,
    /// Whether they are SJK staff.
    #[serde(default)]
    pub staff: bool,
    /// Whether they are SJK's creator.
    #[serde(default)]
    pub creator: bool,
    /// Their tick (absent from older hubs); draw [`Tick::shown`].
    #[serde(default)]
    pub tick: Tick,
    /// Their trust, `None` when verified or staff.
    #[serde(default)]
    pub trust: Option<Trust>,
    /// Their name emoji, empty for none.
    #[serde(default)]
    pub emoji: String,
    /// Their picture's version, `""` for none.
    #[serde(default)]
    pub avatar: String,
    /// When they last read the feed, unix seconds.
    #[serde(default)]
    pub seen: i64,
    /// They hold a live claim marked active: they are playing a match now.
    #[serde(default)]
    pub playing: bool,
}

/// The text a request's signature covers.
pub fn signed_text(
    method: &str,
    path_and_query: &str,
    ts: i64,
    nonce: &str,
    body: &[u8],
) -> String {
    let hex: String = Sha256::digest(body)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    format!("SJK-HUB-v1\n{method}\n{path_and_query}\n{ts}\n{nonce}\n{hex}")
}

/// The `Authorization` header value for a request signed by `identity`.
pub fn authorization(
    identity: &Identity,
    method: &str,
    path_and_query: &str,
    body: &[u8],
    ts: i64,
    nonce_bytes: [u8; 12],
) -> String {
    let nonce = B64.encode(nonce_bytes);
    let signature = identity.sign_text(&signed_text(method, path_and_query, ts, &nonce, body));
    format!(
        "SJK-Sig key={}, ts={ts}, nonce={nonce}, sig={signature}",
        identity.public_key_text()
    )
}

/// The form two player names are compared in: lower case, Quake colour codes
/// removed, only letters and digits kept (the hub uses the same rule for
/// display names).
pub fn normal_form(name: &str) -> String {
    let mut out = String::new();
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '^' {
            // `Q_IsColorString`: `^` plus any character but another `^`.
            if chars.peek().is_some_and(|&next| next != '^') {
                let _ = chars.next();
            }
        } else if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Whether a claim's name and the name the game shows are one player's name.
///
/// The game server cleans names (length, spaces, duplicates), so the exact text
/// can differ; the normal forms must agree and must not be empty.
pub fn names_match(claimed: &str, shown: &str) -> bool {
    let claimed = normal_form(claimed);
    !claimed.is_empty() && claimed == normal_form(shown)
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_opening_may_hold_a_shader_instead_of_a_card() {
        let opened: Opened = serde_json::from_str(
            r#"{"holocron":8,"tier":"uncommon","credits":0,
                "unlock":{"id":"saber_tidal","name":"Tidal blade","count":2}}"#,
        )
        .unwrap();
        assert_eq!(opened.card, None);
        assert_eq!(
            opened.unlock,
            Some(OpenedUnlock {
                finish: None,
                id: "saber_tidal".into(),
                name: "Tidal blade".into(),
                count: 2
            })
        );
        // A card opening has no shader, and an unlock without a count is one.
        let card: Opened = serde_json::from_str(
            r#"{"holocron":9,"tier":"rare","credits":40,
                "card":{"id":3,"card":null,"rarity":"common","finish":"normal","edition":"alpha"}}"#,
        )
        .unwrap();
        assert!(card.card.is_some() && card.unlock.is_none());
        let unlock: Unlock = serde_json::from_str(r#"{"id":"saber_sun"}"#).unwrap();
        assert_eq!(unlock.count, 1);
    }

    /// A Solar blade card brings its card and its blade together (20/10/2026), in an
    /// opening and in a reveal; a reveal from an older hub carries no shader.
    #[test]
    fn a_solar_blade_card_brings_its_blade_beside_it() {
        let opened: Opened = serde_json::from_str(
            r#"{"holocron":10,"tier":"mythical","credits":300,
                "card":{"id":4,"card":"sjk_blades_007","rarity":"rare","finish":"solar",
                        "edition":"alpha"},
                "unlock":{"id":"saber_sun","name":"Sun blade","count":1}}"#,
        )
        .unwrap();
        assert_eq!(
            opened.card.as_ref().and_then(|card| card.card.as_deref()),
            Some("sjk_blades_007")
        );
        assert_eq!(
            opened.unlock.as_ref().map(|unlock| unlock.id.as_str()),
            Some("saber_sun")
        );
        let revealed: Vec<Revealed> = serde_json::from_str(
            r#"[{"id":5,"card":"sjk_blades_007","rarity":"rare","finish":"solar",
                 "edition":"alpha","unlock":{"id":"saber_sun","name":"Sun blade","count":2}},
                {"id":6,"card":"sjk_alpha_010","rarity":"legendary","finish":"normal"}]"#,
        )
        .unwrap();
        assert_eq!(
            revealed[0].unlock,
            Some(OpenedUnlock {
                finish: None,
                id: "saber_sun".into(),
                name: "Sun blade".into(),
                count: 2
            })
        );
        assert_eq!(revealed[1].unlock, None);
    }

    use super::*;

    /// A request every implementation of `PROTOCOL.md` must sign to this exact
    /// header; the hub's tests carry the same vector.
    #[test]
    fn the_known_answer_header_matches_the_protocol() {
        let identity = Identity::from_seed([7; 32]);
        assert_eq!(
            authorization(
                &identity,
                "POST",
                "/v1/claim",
                br#"{"server":"1.2.3.4:29070","slot":3,"name":"Sol"}"#,
                1_000_000,
                [1; 12]
            ),
            KNOWN_ANSWER
        );
    }

    const KNOWN_ANSWER: &str = "SJK-Sig key=6kpsY-KcUgq-9VB7Ey7F-ZVHdq6-vnuSQh7qaRRG0iw, ts=1000000, nonce=AQEBAQEBAQEBAQEB, sig=IKLlpqaSWIXqXK7Q77SsGJiE4BIdUTjkSWv2zyr0951WKByhEShSL-Ow70Rgc3znokl0Sf4rQK2VjNkPEm9uCQ";

    #[test]
    fn a_feed_answer_parses() {
        let feed: Feed = serde_json::from_str(
            r#"{"next":14,"chat":[{"id":12,"at":5,"key_id":"aa","name":"^2Sol",
                "verified":true,"staff":false,"text":"gg"}],
                "emotes":[{"id":13,"at":6,"slot":3,"claimed_name":"^2Sol","key_id":"aa",
                "emote":"wave"}],"deleted":[9],"online":7}"#,
        )
        .unwrap();
        assert_eq!(feed.next, 14);
        assert_eq!(feed.chat[0].text, "gg");
        assert!(feed.chat[0].verified);
        assert_eq!(
            (feed.emotes[0].slot, feed.emotes[0].emote.as_str()),
            (3, "wave")
        );
        assert_eq!((feed.deleted.as_slice(), feed.online), (&[9][..], 7));
        // Lists a hub leaves out are empty.
        let quiet: Feed = serde_json::from_str(r#"{"next":3}"#).unwrap();
        assert!(quiet.chat.is_empty() && quiet.emotes.is_empty() && quiet.deleted.is_empty());
        assert_eq!(quiet.people, None, "a hub that does not list them");
    }

    #[test]
    fn a_feed_answer_with_people_parses() {
        let feed: Feed = serde_json::from_str(
            r#"{"next":3,"online":2,"people":{"online":[{"key_id":"aa","name":"^2Sol",
                "verified":true,"staff":true,"avatar":"0123456789abcdef","seen":100,
                "playing":true},{"key_id":"bb","name":"Fox","seen":90}],
                "recent":[{"key_id":"cc","name":"Kyle","seen":40}]}}"#,
        )
        .unwrap();
        let people = feed.people.expect("people");
        assert_eq!(people.online.len(), 2);
        let sol = &people.online[0];
        assert!(sol.verified && sol.staff && sol.playing);
        assert_eq!((sol.avatar.as_str(), sol.seen), ("0123456789abcdef", 100));
        // Fields a hub leaves out are off or empty.
        let fox = &people.online[1];
        assert!(!fox.verified && !fox.playing && fox.avatar.is_empty());
        assert_eq!(people.recent[0].name, "Kyle");
        let empty: Feed = serde_json::from_str(r#"{"next":3,"people":{}}"#).unwrap();
        assert_eq!(empty.people, Some(People::default()));
    }

    #[test]
    fn unlocks_and_looks_parse_and_older_hubs_send_none() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "unlocks":[{"id":"saber_sun","granted":1791000000,"note":"Thanks"},
                           {"id":"from_the_future","granted":7},
                           {"id":"saber_glitch","granted":8,"note":"","medal":"bug_hunter"},
                           {"id":"saber_sun","granted":9,"card":"sjk_alpha_000"}]}"#,
        )
        .unwrap();
        assert_eq!(
            profile.unlocks,
            [
                Unlock {
                    holo: 0,
                    solar: 0,
                    id: "saber_sun".to_owned(),
                    granted: 1_791_000_000,
                    note: "Thanks".to_owned(),
                    count: 1,
                    medal: None,
                    card: None,
                },
                Unlock {
                    holo: 0,
                    solar: 0,
                    id: "from_the_future".to_owned(),
                    granted: 7,
                    note: String::new(),
                    count: 1,
                    medal: None,
                    card: None,
                },
                Unlock {
                    holo: 0,
                    solar: 0,
                    id: "saber_glitch".to_owned(),
                    granted: 8,
                    note: String::new(),
                    count: 1,
                    medal: Some("bug_hunter".to_owned()),
                    card: None,
                },
                Unlock {
                    holo: 0,
                    solar: 0,
                    id: "saber_sun".to_owned(),
                    granted: 9,
                    note: String::new(),
                    count: 1,
                    medal: None,
                    card: Some("sjk_alpha_000".to_owned()),
                },
            ]
        );
        let old: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}"#,
        )
        .unwrap();
        assert!(old.unlocks.is_empty(), "an older hub sends no unlocks");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "look":{"saber":"saber_sun","illuminate":true}}"#,
        )
        .unwrap();
        assert_eq!(
            presence.look,
            Some(Look {
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
            })
        );
        let options: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "look":{"saber":"saber_sun","illuminate":false,"saber_off":["glint","haze"]}}"#,
        )
        .unwrap();
        assert_eq!(
            options.look.unwrap().saber_off,
            ["glint", "haze"],
            "the options switched off, since 11/10/2026"
        );
        let bare: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false}"#,
        )
        .unwrap();
        assert_eq!(bare.look, None, "a claim without a look, or an older hub");
        let feed: Feed = serde_json::from_str(
            r#"{"next":15,"looks":[{"id":15,"at":9,"slot":3,"claimed_name":"^2Sol",
                "key_id":"aa","saber":"","illuminate":true}]}"#,
        )
        .unwrap();
        let event = &feed.looks[0];
        assert_eq!((event.id, event.slot), (15, 3));
        assert_eq!(event.claimed_name, "^2Sol");
        assert_eq!(
            event.look(),
            Look {
                saber: String::new(),
                illuminate: true,
                saber_off: Vec::new(),
                toys: Vec::new(),
                saber2: None,
                saber2_off: Vec::new(),
                saber_on: Vec::new(),
                saber2_on: Vec::new(),
                tag: None,
                saber_sounds: String::new(),
                finisher: String::new(),
            }
        );
        let options: Feed = serde_json::from_str(
            r#"{"next":16,"looks":[{"id":16,"at":9,"slot":3,"claimed_name":"^2Sol",
                "key_id":"aa","saber":"saber_sun","illuminate":false,"saber_off":["wisps"]}]}"#,
        )
        .unwrap();
        assert_eq!(options.looks[0].look().saber_off, ["wisps"]);
        // The sounds chosen apart from the blade (23/10/2026); absent above: follow it.
        let sounds: Feed = serde_json::from_str(
            r#"{"next":17,"looks":[{"id":17,"at":9,"slot":3,"claimed_name":"^2Sol",
                "key_id":"aa","saber":"saber_sun","illuminate":false,"saber_sounds":"stock"}]}"#,
        )
        .unwrap();
        assert_eq!(sounds.looks[0].look().saber_sounds, "stock");
        let quiet: Feed = serde_json::from_str(r#"{"next":3,"emotes":[]}"#).unwrap();
        assert!(quiet.looks.is_empty(), "an older hub sends no looks");
    }

    #[test]
    fn a_feed_answer_with_pings_parses() {
        let feed: Feed = serde_json::from_str(
            r#"{"next":17,"pings":[{"id":17,"at":9,"slot":3,"claimed_name":"^2Sol",
                "key_id":"aa","pos":[10,-20,30],"scope":"red"}]}"#,
        )
        .unwrap();
        let ping = &feed.pings[0];
        assert_eq!(
            (ping.slot, ping.pos, ping.scope.as_str()),
            (3, [10, -20, 30], "red")
        );
        let quiet: Feed = serde_json::from_str(r#"{"next":3}"#).unwrap();
        assert!(quiet.pings.is_empty(), "an older hub sends no pings");
    }

    #[test]
    fn holocrons_parse_in_profiles_and_older_hubs_send_none() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "holocron_counts":{"uncommon":4,"rare":2,"legendary":1,"mythical":0},
                "holocrons":[{"id":9,"tier":"legendary","dropped":1791000000,"source":"play","note":""},
                             {"id":7,"tier":"rare","dropped":1790990000,"source":"staff","note":"Thanks"},
                             {"id":3,"tier":"from_the_future","dropped":1}]}"#,
        )
        .unwrap();
        assert_eq!(
            profile.holocron_counts,
            HolocronCounts {
                uncommon: 4,
                rare: 2,
                legendary: 1,
                mythical: 0
            }
        );
        assert_eq!(profile.holocron_counts.total(), 7);
        assert_eq!(profile.holocron_counts.of("rare"), 2);
        assert_eq!(profile.holocron_counts.of("from_the_future"), 0);
        let ids: Vec<u64> = profile
            .holocrons
            .iter()
            .map(|holocron| holocron.id)
            .collect();
        assert_eq!(ids, [9, 7, 3]);
        assert_eq!(profile.holocrons[1].source, "staff");
        assert_eq!(profile.holocrons[1].note, "Thanks");
        assert_eq!(profile.holocrons[2].source, "", "a missing field is empty");
        let old: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}"#,
        )
        .unwrap();
        assert_eq!(old.holocron_counts, HolocronCounts::default());
        assert!(old.holocrons.is_empty(), "an older hub sends none");
    }

    #[test]
    fn a_persons_keys_parse_and_older_hubs_send_none() {
        let person: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "keys":["aa","cc","dd"]}"#,
        )
        .unwrap();
        assert_eq!(person.keys, ["aa", "cc", "dd"]);
        assert_eq!(person.linked_keys().collect::<Vec<_>>(), ["cc", "dd"]);
        assert!(person.has_key("aa") && person.has_key("dd"));
        assert!(!person.has_key("ee"));
        let old: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}"#,
        )
        .unwrap();
        assert!(old.keys.is_empty(), "an older hub sends no keys");
        assert_eq!(old.linked_keys().count(), 0);
        assert!(old.has_key("aa"));
    }

    #[test]
    fn the_holocron_state_and_drop_events_parse() {
        let state: HolocronState = serde_json::from_str(
            r#"{"progress_secs":600,"every_secs":1800,"today":3,"daily_cap":8,"server_time":1791000000}"#,
        )
        .unwrap();
        assert_eq!(
            state,
            HolocronState {
                progress_secs: 600,
                every_secs: 1800,
                today: 3,
                daily_cap: 8,
                resets_at: None,
                server_time: 1_791_000_000,
                can_open: false,
                min_client: None,
                loot: Vec::new(),
                economy: None,
                kyber: None,
                marks: None,
                catchup: None,
                activity: None,
            }
        );
        // Since 03/11/2026: the catch-up and how the last claims' play was judged.
        let caught: HolocronState = serde_json::from_str(
            r#"{"progress_secs":600,"every_secs":1800,
                "catchup":{"pool":4,"left":3,"every_secs":900,"max":4},
                "activity":{"pace":50,"last_combat":0,"combat_window":600,"peaceful":50,
                    "move_min":200},
                "economy":{"abilities":["tag_copies"],
                    "tags":{"shatter":[2,5,12,30,75],"finish":{"normal":1,"holo":5,"solar":25}}}}"#,
        )
        .unwrap();
        assert_eq!(caught.next_every(), 900);
        assert_eq!(caught.activity.unwrap().pace, Some(50));
        let economy = caught.economy.unwrap();
        assert!(economy.can(TAG_COPIES_ABILITY));
        assert_eq!(economy.tags.unwrap().yield_of(1, "holo"), Some(25));
        let spent: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"catchup":{"pool":4,"left":0,"every_secs":900},
                "activity":{"pace":null}}"#,
        )
        .unwrap();
        assert_eq!(spent.next_every(), 1_800);
        assert_eq!(spent.activity.unwrap().pace, None);
        let answer: TagCraftAnswer = serde_json::from_str(
            r#"{"crafted":{"id":"tag_kyber","name":"Kyber tag","tier":"uncommon","finish":"holo",
                "gave":[{"id":"tag_gonk","finish":"holo"},{"id":"tag_gonk"}]},
                "profile":{"key_id":"0123456789abcdef","key":"k","name":"Sol","bio":"",
                    "verified":false,"created":1}}"#,
        )
        .unwrap();
        assert_eq!(answer.crafted.gave[1], TagCopy::new("tag_gonk", "normal"));
        // Since 29/10/2026 the hub says when its caps' day ends (09:00 in Paris).
        let day: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"today":8,"daily_cap":8,
                "resets_at":1793260800,"server_time":1793250000}"#,
        )
        .unwrap();
        assert_eq!(day.resets_at, Some(1_793_260_800));
        let odds: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"server_time":1,"loot":[{"tier":"uncommon",
                "drop":600,"rarity":[700,240,55,5],"finish":[970,30,0],"credits":[10,20],
                "shader":300,"shader_tier":[750,220,30,0,0]}]}"#,
        )
        .unwrap();
        assert_eq!(odds.loot[0].tier, "uncommon");
        assert_eq!((odds.loot[0].credits, odds.loot[0].shader), ([10, 20], 300));
        // Hubs before shard piles leave them out.
        assert_eq!((odds.loot[0].shard_pile, odds.loot[0].shards), (0, [0, 0]));
        let economy: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"server_time":1,"loot":[{"tier":"uncommon",
                "drop":600,"rarity":[700,240,55,5],"finish":[970,30,0],"credits":[10,20],
                "shader":300,"shader_tier":[750,220,30,0,0],"shard_pile":100,"shards":[5,15]}],
                "economy":{"shatter":[5,10,25,60,150],"craft":[40,100,300,800,2000],
                "shop":[250,600,1500,4000,10000]}}"#,
        )
        .unwrap();
        assert_eq!(
            (economy.loot[0].shard_pile, economy.loot[0].shards),
            (100, [5, 15])
        );
        let tables = economy.economy.unwrap();
        assert_eq!(
            (tables.shatter[0], tables.craft[4], tables.shop[2]),
            (5, 2_000, 1_500)
        );
        // A hub before the shop sold for shards says neither its currency nor holocrons.
        assert!(tables.shop_currency.is_empty() && tables.holocrons.is_empty());
        let shards: EconomyTables = serde_json::from_str(
            r#"{"shatter":[5,10,25,60,150],"craft":[40,100,300,800,2000],
                "shop":[30,75,225,600,1500],"shop_currency":"shards",
                "holocrons":{"uncommon":150,"rare":400}}"#,
        )
        .unwrap();
        assert_eq!(shards.shop_currency, "shards");
        assert!(shards.blade_card_craft.is_empty());
        let cards: EconomyTables =
            serde_json::from_str(r#"{"blade_card_craft":{"normal":75,"holo":50,"solar":50}}"#)
                .unwrap();
        assert_eq!(cards.blade_card_craft["holo"], 50);
        assert_eq!(
            (shards.holocrons["uncommon"], shards.holocrons["rare"]),
            (150, 400)
        );
        // Hubs before 21/10/2026 say no abilities; later ones name theirs.
        assert!(!shards.can("shatter_last") && !shards.can("shatter_duplicates"));
        let forge: EconomyTables = serde_json::from_str(
            r#"{"shatter":[5,10,25,60,150],"abilities":["shatter_last","shatter_duplicates"]}"#,
        )
        .unwrap();
        assert!(forge.can("shatter_last") && forge.can("shatter_duplicates"));
        // A hub before collection marks (29/10/2026) neither says the ability nor sends
        // them; a later one does both.
        assert!(!forge.can(MARKS_ABILITY));
        assert_eq!(economy.marks, None);
        let marked: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,
                "economy":{"abilities":["shatter_last","marks"]},
                "marks":{"favourite":["saber_storm"],"locked":["saber_frost"]}}"#,
        )
        .unwrap();
        assert!(marked.economy.unwrap().can(MARKS_ABILITY));
        let mut marks = marked.marks.unwrap();
        assert!(marks.favourite("saber_storm") && !marks.locked("saber_storm"));
        assert!(marks.locked("saber_frost"));
        assert!(!marks.set("saber_frost", Mark::Locked, true), "set already");
        assert!(marks.set("saber_frost", Mark::Locked, false));
        assert!(marks.set("saber_tidal", Mark::Favourite, true));
        assert_eq!(marks.favourite, ["saber_storm", "saber_tidal"]);
        assert!(!marks.locked("saber_frost"));
        let answer: MarksAnswer =
            serde_json::from_str(r#"{"marks":{"favourite":[],"locked":["saber_frost"]}}"#).unwrap();
        assert!(answer.marks.locked("saber_frost"));
        let all: ShatterAllAnswer = serde_json::from_str(
            r#"{"shattered":[{"id":"saber_tidal","copies":2,"shards":10}],"copies":2,
                "shards":10,"balance":45,
                "profile":{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}}"#,
        )
        .unwrap();
        assert_eq!((all.copies, all.shards, all.balance), (2, 10, 45));
        assert_eq!(all.shattered[0].id, "saber_tidal");
        let bought: HolocronBuyAnswer = serde_json::from_str(
            r#"{"bought":{"holocron":42,"tier":"rare","price":400,"balance":100},
                "profile":{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}}"#,
        )
        .unwrap();
        assert_eq!((bought.bought.holocron, bought.bought.price), (42, 400));
        let staff: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"today":0,"daily_cap":8,"server_time":1,"can_open":true}"#,
        )
        .unwrap();
        assert!(staff.can_open);
        let gated: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"server_time":1,"can_open":true,"min_client":"2026.1022.1"}"#,
        )
        .unwrap();
        assert_eq!(gated.min_client.as_deref(), Some("2026.1022.1"));
        // Kyber crystals (21/10/2026): their odds and the person's pity, beside `loot`.
        let kyber: HolocronState = serde_json::from_str(
            r#"{"progress_secs":0,"every_secs":1800,"server_time":1,
                "kyber":{"tiers":[{"tier":"kyber_uncommon","shader_tier":[450,400,120,25,5]},
                {"tier":"kyber_rare","shader_tier":[200,400,300,85,15]}],
                "pity":{"every":10,"since":3}}}"#,
        )
        .unwrap();
        let tables = kyber.kyber.unwrap();
        assert_eq!(tables.tiers[1].tier, "kyber_rare");
        assert_eq!(tables.tiers[0].shader_tier, [450, 400, 120, 25, 5]);
        assert_eq!(
            tables.pity,
            Some(KyberPity {
                every: 10,
                since: 3
            })
        );
        assert!(kyber.loot.is_empty(), "the holocrons' loot is untouched");
        let achievement: Achievement = serde_json::from_str(
            r#"{"id":"kills_100","progress":100,"goal":100,"unlocked":5,"reward":"kyber_rare"}"#,
        )
        .unwrap();
        assert_eq!(achievement.reward.as_deref(), Some("kyber_rare"));
        let older: Achievement =
            serde_json::from_str(r#"{"id":"kills_100","progress":3,"goal":100}"#).unwrap();
        assert_eq!(older.reward, None, "hubs before crystals say no reward");
        let feed: Feed = serde_json::from_str(
            r#"{"next":21,"drops":[{"id":20,"at":1791000001,"key_id":"aa","name":"^2Sol",
                "tier":"legendary","verified":true},
                {"id":21,"at":1791000002,"key_id":"bb","name":"Fox","tier":"rare"}]}"#,
        )
        .unwrap();
        assert_eq!(feed.drops.len(), 2);
        assert_eq!(
            (feed.drops[0].tier.as_str(), feed.drops[0].verified),
            ("legendary", true)
        );
        assert!(!feed.drops[1].verified, "absent is not verified");
        let quiet: Feed = serde_json::from_str(r#"{"next":3}"#).unwrap();
        assert!(quiet.drops.is_empty(), "an older hub sends no drops");
        // A chat message is never a drop, whatever the wire says.
        let message: ChatMessage = serde_json::from_str(
            r#"{"id":1,"at":2,"key_id":"aa","name":"x","text":"y","holocron":{"tier":"rare","own":true}}"#,
        )
        .unwrap();
        assert_eq!(message.holocron, None);
    }

    #[test]
    fn names_match_through_colours_and_case() {
        assert!(names_match("^1Sol", "sol"));
        assert!(names_match("S.o.l", "^2SOL"));
        assert!(!names_match("Sol", "Fox"));
        assert!(!names_match("^1^2", "^3"));
    }

    #[test]
    fn profiles_and_presence_parse() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5}"#,
        )
        .unwrap();
        assert!(profile.verified);
        assert!(profile.names.is_empty(), "an older hub sends no history");
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":false,"created":5,
                "names":[{"name":"^1Sol","first_seen":1,"last_seen":9}]}"#,
        )
        .unwrap();
        assert_eq!(profile.names[0].name, "^1Sol");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false}"#,
        )
        .unwrap();
        assert_eq!(presence.slot, 3);
        assert!(presence.medals.is_empty(), "an older hub sends no medals");
        assert!(presence.avatar.is_empty(), "an older hub sends no picture");
        assert!(profile.avatar.is_empty());
        let pictured: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":false,"created":5,
                "avatar":"0123456789abcdef"}"#,
        )
        .unwrap();
        assert_eq!(pictured.avatar, "0123456789abcdef");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "avatar":"fedcba9876543210"}"#,
        )
        .unwrap();
        assert_eq!(presence.avatar, "fedcba9876543210");
    }

    #[test]
    fn medals_parse_in_profiles_and_presence() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "medals":[{"id":"early_tester","count":1,"awarded":1791000000,"note":""},
                          {"id":"bug_hunter","count":2,"awarded":1791300000,"note":"The ^1fog^7 bug"},
                          {"id":"from_the_future","count":1,"awarded":1,"note":""}]}"#,
        )
        .unwrap();
        assert!(profile.names.is_empty());
        let ids: Vec<&str> = profile
            .medals
            .iter()
            .map(|medal| medal.id.as_str())
            .collect();
        assert_eq!(ids, ["early_tester", "bug_hunter", "from_the_future"]);
        assert_eq!(profile.medals[1].count, 2);
        assert_eq!(profile.medals[1].awarded, 1_791_300_000);
        assert_eq!(profile.medals[1].note, "The ^1fog^7 bug");
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "medals":[{"id":"early_tester","count":1},{"id":"jof_clan"}]}"#,
        )
        .unwrap();
        assert_eq!(
            presence.medals,
            [
                Medal {
                    id: "early_tester".to_owned(),
                    count: 1,
                    awarded: 0,
                    note: String::new(),
                    rank: 1,
                },
                Medal {
                    id: "jof_clan".to_owned(),
                    count: 1,
                    awarded: 0,
                    note: String::new(),
                    rank: 1,
                },
            ]
        );
    }

    #[test]
    fn medal_ranks_default_to_one_and_are_clamped() {
        let profile: Profile = serde_json::from_str(
            r#"{"key_id":"aa","key":"bb","name":"Sol","bio":"","verified":true,"created":5,
                "medals":[{"id":"early_tester","count":1,"awarded":1,"note":"","rank":3},
                          {"id":"bug_hunter","count":1,"awarded":1,"note":""},
                          {"id":"supporter","count":1,"awarded":1,"note":"","rank":9},
                          {"id":"early_contributor","count":1,"awarded":1,"note":"","rank":0},
                          {"id":"x","rank":-4},
                          {"id":"y","rank":"high"}]}"#,
        )
        .unwrap();
        let ranks: Vec<u8> = profile.medals.iter().map(|medal| medal.rank).collect();
        assert_eq!(ranks, [3, 1, 5, 1, 1, 1]);
        let presence: Presence = serde_json::from_str(
            r#"{"slot":3,"claimed_name":"x","key_id":"aa","name":"Sol","verified":false,
                "medals":[{"id":"early_tester","count":1,"rank":5},{"id":"bug_hunter","count":1}]}"#,
        )
        .unwrap();
        let ranks: Vec<u8> = presence.medals.iter().map(|medal| medal.rank).collect();
        assert_eq!(ranks, [5, 1]);
    }
}
