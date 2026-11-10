//! The Holocrons tab's opening (`docs/cards.md`, "Opening"): Open (the oldest holocron of
//! the tier chosen) or Open all (up to twenty, the oldest first) asks the hub, and while
//! it answers the tab's 3D holocron glides to the middle of the window and hums
//! ([`super::unseal`]). Then each holocron opened plays its ceremony over the darkened
//! page, one after another:
//!
//! - the holocron spins up, blazes and cracks open in a flash of its tier's colour
//!   ([`super::unseal`]);
//! - out of the flash a strip of what its tier can drop runs past a gold marker and slows
//!   to a stop on the prize, as a loot crate does ([`super::strip`]): a tick a tile, a
//!   colour band under each, the prize lit and the others dimmed once it rests; a card
//!   shows there as its credits, a stack of coins and the amount, with the card tucked
//!   face down in a corner in its rarity's colour (18/10/2026);
//! - it bursts open in its tier's light: rings run out, rays turn, the gem flares and
//!   goes;
//! - a card comes as its pack: the credits' tile rises where the strip stopped, counting
//!   up, then slides aside while the tucked card slides out to the middle, grows and
//!   flips over to face the player in its rarity's colour and its finish's light
//!   ([`crate::cards::look`]), a Holo, Solar, Legendary or Mythical one in a flourish of its light;
//!   then the kyber shards slide out on the other side and count up (a shader instead of
//!   a card shows its blade, framed in its tier's colour, and what it is; a pile of kyber
//!   shards its crystals and how many);
//! - a Solar blade card brings its blade beside it (20/10/2026): the blade's tile slides
//!   out on the right with the shards, and its words say "...and the Sun blade drops with
//!   it!" in place of what the finish is;
//! - its words fade up: the card's rarity and finish, its edition, what the finish is,
//!   that it stays sealed until the first set of cards is out, and "1 of 3".
//!
//! Enter, Space, Right or a click skip to the burst, finish the entrance at once, then
//! take Next (Done on the last); Escape ends the ceremony (what was opened is the hub's
//! already). Openings of more than three crack the first holocron, run their strips five
//! at a time and then lay out everything found ([`super::batch`]). A refusal or no answer
//! shows why, and the page reads the hub again. `debug_open` plays made-up openings the same way
//! ([`crate::cards::rehearsal`]).
//!
//! While the ceremony shows the page under it is not drawn: text draws above every shape,
//! so the page's words would show through the dark.

use super::strip;
use super::*;
use crate::audio::ui_cues::{self, Cue};
use crate::cards::{self, Finish, Rarity};
use crate::holocrons::gem;
use crate::menu::sjk::{Frame, color, key_hint, key_hint_width, kit, text};
use crate::menu_widgets::TextFamily;
use crate::unlockables::Unlockable;
use sjk_ui::{Color, DrawCommand, FontWeight, TextAlign};

/// The ceremony's Next (or Done) button.
pub(crate) const NEXT_TOKEN: u16 = 1_322;
/// How long the hub has to answer before the page says it did not.
const ANSWER_WAIT: Duration = Duration::from_secs(15);
/// The holocron cracking open before its strip ([`super::unseal`]), in seconds.
pub(super) const UNSEAL: f32 = 1.7;
/// The crate spin after it, in seconds.
const SPIN: f32 = 6.0;
/// The share of the spin the strip runs; the rest it rests on the prize, lit.
const SPIN_RUN: f32 = 0.88;
/// The unseal and the spin: the entrance begins this far into a holocron's ceremony.
const BEFORE: f32 = UNSEAL + SPIN;
/// A shader's or a pile's entrance: the burst, the prize turning in, its count, the words.
const ENTRANCE: f32 = 2.4;
const BURST_END: f32 = 0.75;
const TURN: [f32; 2] = [0.55, 1.25];
const COUNT: [f32; 2] = [1.2, 1.9];
const WORDS: f32 = 1.85;
/// A card's entrance, as its pack (18/10/2026; seconds into it): the credits' tile rising
/// in where the strip stopped, its credits counting, the tile sliding aside while the
/// tucked card slides out to the middle, the card flipping over, its flourish, the kyber
/// shards' tile sliding out and counting, the words, and the whole entrance.
const PACK_IN: [f32; 2] = [0.35, 0.8];
const PACK_COUNT: [f32; 2] = [0.5, 1.05];
const PACK_OUT: [f32; 2] = [1.05, 1.5];
const FLIP: [f32; 2] = [1.5, 1.95];
const FLOURISH: f32 = 0.8;
const SHARDS_OUT: [f32; 2] = [1.95, 2.35];
const SHARDS_COUNT: [f32; 2] = [2.05, 2.6];
/// A Solar blade card's blade sliding out beside the shards (20/10/2026).
const BONUS_OUT: [f32; 2] = [2.1, 2.5];
const CARD_WORDS: f32 = 2.55;
const CARD_ENTRANCE: f32 = 3.1;
/// The card pack's tiles: their size, their gap from the card and how far the shards'
/// slides (frame pixels).
const PACK_TILE: [f32; 2] = [230.0, 214.0];
const PACK_GAP: f32 = 46.0;
const PACK_SLIDE: f32 = 80.0;
/// The card tucked in the credits' tile's corner: its middle from the tile's, its width
/// (frame pixels) and its turn (radians, clockwise).
const TUCKED_AT: [f32; 2] = [PACK_TILE[0] * 0.5 - 34.0, -PACK_TILE[1] * 0.5 + 36.0];
const TUCKED_WIDTH: f32 = 78.0;
const TUCKED_TURN: f32 = 0.24;
/// More than this many opened at once run five strips at a time ([`super::batch`]).
pub(super) const ONE_BY_ONE: usize = 3;
/// The card on show: its middle and width (frame pixels); a revealed card's face is
/// bigger, to be read, and higher.
const CARD_X: f32 = 960.0;
const CARD_TOP: f32 = 190.0;
const CARD_WIDTH: f32 = 300.0;
const FACE_TOP: f32 = 62.0;
const FACE_WIDTH: f32 = 400.0;
/// The words' lines under the card.
const CREDITS_Y: f32 = 636.0;
const TITLE_Y: f32 = 700.0;
const EDITION_Y: f32 = 752.0;
const ABOUT_Y: f32 = 790.0;
const SEALED_Y: f32 = 822.0;
const BUTTON_Y: f32 = 880.0;
/// Rays of the burst.
const RAYS: usize = 12;
/// The crate strip: its top and height, and the band it shows through (frame pixels).
const STRIP_Y: f32 = 380.0;
const STRIP_HEIGHT: f32 = strip::TILE * cards::look::ASPECT;
const STRIP_LEFT: f32 = 40.0;
const STRIP_RIGHT: f32 = 1_880.0;
/// The colour band under each tile, as a loot crate's: its gap and height.
pub(super) const BAND_GAP: f32 = 10.0;
pub(super) const BAND: f32 = 6.0;
/// A shader's showpiece: its top, width and height.
const SHADER_TOP: f32 = 250.0;
const SHADER_WIDTH: f32 = 520.0;
const SHADER_HEIGHT: f32 = 300.0;

/// One holocron opened, as the ceremony shows it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    /// The holocron's number at the hub (it seeds the crate spin).
    pub(crate) holocron: u64,
    /// Position among every tier the tab lists ([`holocrons::slot`]): a kyber crystal's past the holocrons'.
    pub(crate) tier: usize,
    pub(crate) rarity: &'static Rarity,
    pub(crate) finish: &'static Finish,
    /// "Alpha Edition", or empty.
    pub(crate) edition: String,
    pub(crate) credits: u64,
    /// The kyber shards that came with it, beside the credits (the card pack, 17/10/2026;
    /// 0 while the hub's economy is off, and then the credits show alone).
    pub(crate) shards: u64,
    /// The card it is, picked by the hub; `None` while sealed.
    pub(crate) card: Option<String>,
    /// The shader that came with it: a Solar copy of a blade card drops its blade too
    /// (20/10/2026); `None` for every other card.
    pub(crate) bonus: Option<Found>,
}

impl Shown {
    /// Where the card stands: its top and width (frame pixels), bigger for a face the
    /// packs have.
    fn place(&self, faces: &cards::faces::Faces) -> (f32, f32) {
        if cards::shown(faces, self.card.as_deref()).is_some() {
            (FACE_TOP, FACE_WIDTH)
        } else {
            (CARD_TOP, CARD_WIDTH)
        }
    }
}

impl Shown {
    /// What the hub said one holocron brought; `None` for a tier, rarity or finish the
    /// client does not know.
    pub(crate) fn of(opened: &sjk_identity::Opened) -> Option<Self> {
        // A shader instead of a card is a [`Prize::Shader`].
        let held = opened.card.as_ref()?;
        Self::of_card(opened, held, opened.credits, opened.shards)
    }

    /// Card copy `held` from holocron `opened`, shown with `credits` and `shards` (a Beta
    /// holocron's bonus card comes with neither).
    fn of_card(
        opened: &sjk_identity::Opened,
        held: &sjk_identity::OpenedCard,
        credits: u64,
        shards: u64,
    ) -> Option<Self> {
        Some(Self {
            holocron: opened.holocron,
            tier: holocrons::any(&opened.tier)?.slot,
            rarity: Rarity::from_id(&held.rarity)?,
            finish: Finish::from_id(&held.finish)?,
            edition: cards::edition_name(&held.edition),
            credits,
            shards,
            card: held.card.clone(),
            bonus: None,
        })
    }
}

/// A shader an opening brought, instead of a card.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Found {
    /// The holocron's number at the hub (it seeds the crate spin).
    pub(crate) holocron: u64,
    /// Position among every tier the tab lists ([`holocrons::slot`]): a kyber crystal's past the holocrons'.
    pub(crate) tier: usize,
    /// The shader, when this client knows it; `None` for one newer than this client
    /// (the hub's catalogue grows before every player updates): it is shown by the
    /// hub's name for it, and its blade if the hub's pack brought one.
    pub(crate) skin: Option<&'static Unlockable>,
    /// The tag it is instead, when it is one this client knows (`tags.rs`, 27/10/2026):
    /// the hub answers a tag dropped as it answers a shader.
    pub(crate) tag: Option<&'static crate::tags::Tag>,
    /// Its id and name as the hub sent them.
    pub(crate) id: String,
    pub(crate) name: String,
    /// How many of it the player holds now, this one included.
    pub(crate) count: u32,
    /// What it is when this client does not know it ([`unknown_kind`]): "saber shader",
    /// "tag", "card"...; the words a fallback card shows (Kushina, 30/10/2026: loot newer
    /// than the client showed as a dark, empty shader card).
    pub(crate) unknown: &'static str,
}

/// What a prize this client does not know is, from the hub's id: its unlock ids start
/// with their kind (`saber_`, `tag_`, `banner_`, `toy_`); "prize" for anything else.
pub(crate) fn unknown_kind(id: &str) -> &'static str {
    const KINDS: [(&str, &str); 4] = [
        ("saber_", "saber shader"),
        ("tag_", "tag"),
        ("banner_", "profile banner"),
        ("toy_", "toy"),
    ];
    KINDS
        .iter()
        .find(|(prefix, _)| id.starts_with(prefix))
        .map_or("prize", |(_, kind)| kind)
}

impl Found {
    /// The shader `unlock` the hub said holocron `opened` brought; `None` for a tier the
    /// client does not know. One it does not know still shows, by the hub's name.
    fn of(opened: &sjk_identity::Opened, unlock: &sjk_identity::OpenedUnlock) -> Option<Self> {
        let unknown = unknown_kind(&unlock.id);
        let name = if unlock.name.trim().is_empty() {
            format!("A new {unknown}")
        } else {
            unlock.name.trim().to_owned()
        };
        // A tag that came Holo or Solar (31/10/2026) says so: "Holo Gonk tag".
        let name = match unlock
            .finish
            .as_deref()
            .and_then(crate::cards::Finish::from_id)
            .filter(|finish| finish.index > 0)
        {
            Some(finish) => format!("{} {name}", finish.name),
            None => name,
        };
        Some(Self {
            holocron: opened.holocron,
            tier: holocrons::any(&opened.tier)?.slot,
            skin: crate::unlockables::blade_skin(&unlock.id),
            tag: crate::tags::get(&unlock.id),
            id: unlock.id.clone(),
            name,
            count: unlock.count,
            unknown,
        })
    }

    /// A card this client cannot show (a rarity or finish newer than it): still a prize,
    /// shown as an unknown card rather than dropped.
    fn unknown_card(opened: &sjk_identity::Opened) -> Option<Self> {
        let held = opened.card.as_ref()?;
        Some(Self {
            holocron: opened.holocron,
            tier: holocrons::any(&opened.tier)?.slot,
            skin: None,
            tag: None,
            id: held.card.clone().unwrap_or_else(|| "card".to_owned()),
            name: "A new card".to_owned(),
            count: 1,
            unknown: "card",
        })
    }

    /// Whether this client knows nothing of it: no shader, tag or banner of its id.
    pub(super) fn is_unknown(&self) -> bool {
        self.skin.is_none()
            && self.tag.is_none()
            && crate::profile_card::banner::get(&self.id).is_none()
    }

    /// The colour it is shown in: its tier's, or the holocron's for one this client
    /// does not know.
    pub(super) fn colour(&self) -> Color {
        if let Some(banner) = crate::profile_card::banner::get(&self.id) {
            return banner.tier.colour();
        }
        match (self.skin, self.tag) {
            (Some(skin), _) => skin.tier.colour(),
            (None, Some(tag)) => tag.tier.colour(),
            (None, None) => holocrons::slot(self.tier).colour,
        }
    }

    /// What it is, in two words ("saber shader", "tag").
    pub(super) fn kind(&self) -> &'static str {
        if crate::profile_card::banner::get(&self.id).is_some() {
            "bonus profile banner"
        } else if self.tag.is_some() {
            "tag"
        } else if self.skin.is_some() {
            "saber shader"
        } else {
            self.unknown
        }
    }
}

/// A pile of kyber shards an opening brought, instead of a card (17/10/2026).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Pile {
    /// The holocron's number at the hub (it seeds the crate spin).
    pub(crate) holocron: u64,
    /// Position among every tier the tab lists ([`holocrons::slot`]): a kyber crystal's past the holocrons'.
    pub(crate) tier: usize,
    /// How many shards.
    pub(crate) shards: u64,
}

/// What one holocron brought: a card with credits, a shader, or a pile of kyber shards.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Prize {
    Card(Shown),
    Shader(Found),
    Shards(Pile),
}

impl Prize {
    /// Everything one holocron brought, each its own reveal: its prize ([`Prize::of`]),
    /// then a Beta holocron's extras (the hub's `PROTOCOL.md`, "Seasons", 10/11/2026), the
    /// group ordered so its best prize comes last. An Alpha holocron gives its one prize.
    pub(crate) fn all(opened: &sjk_identity::Opened) -> Vec<Self> {
        let mut prizes: Vec<Self> = Self::of(opened).into_iter().collect();
        if opened.extra.is_empty() {
            return prizes;
        }
        for extra in &opened.extra {
            let prize = match (&extra.unlock, &extra.card) {
                (Some(unlock), _) => Found::of(opened, unlock).map(Self::Shader),
                (None, Some(card)) => Shown::of_card(opened, card, 0, 0).map(Self::Card),
                (None, None) if extra.shards > 0 => holocrons::any(&opened.tier).map(|tier| {
                    Self::Shards(Pile {
                        holocron: opened.holocron,
                        tier: tier.slot,
                        shards: extra.shards,
                    })
                }),
                (None, None) => None,
            };
            prizes.extend(prize);
        }
        // Stable: equal ones keep the hub's order (its card first).
        prizes.sort_by_key(Self::rank);
        prizes
    }

    /// How fine a prize is, for showing a holocron's best last: a pile of shards least,
    /// then a card by its rarity then finish, a shader or tag by its tier.
    fn rank(&self) -> (usize, usize) {
        match self {
            Self::Card(shown) => (shown.rarity.index + 1, shown.finish.index),
            Self::Shader(found) => {
                let tier = found
                    .skin
                    .map(|skin| skin.tier)
                    .or_else(|| found.tag.map(|tag| tag.tier));
                (tier.map_or(1, |tier| tier as usize + 1), 0)
            }
            Self::Shards(_) => (0, 0),
        }
    }

    /// A second reveal, without replacing any part of the ordinary prize.
    fn bonus(opened: &sjk_identity::Opened) -> Option<Self> {
        opened
            .banner
            .as_ref()
            .and_then(|banner| Found::of(opened, banner))
            .map(Self::Shader)
    }
    /// What the hub said one holocron brought; `None` for a tier, rarity or finish the
    /// client does not know. A shader it does not know still shows, by the hub's name.
    pub(crate) fn of(opened: &sjk_identity::Opened) -> Option<Self> {
        if let Some(unlock) = &opened.unlock {
            let found = Found::of(opened, unlock)?;
            // A card beside it is a Solar blade card bringing its blade (20/10/2026): the
            // card shows, its blade with it. A card this client cannot show leaves the
            // blade alone.
            return Some(match Shown::of(opened) {
                Some(shown) => Self::Card(Shown {
                    bonus: Some(found),
                    ..shown
                }),
                None => Self::Shader(found),
            });
        }
        if opened.card.is_none() && opened.shards > 0 {
            return Some(Self::Shards(Pile {
                holocron: opened.holocron,
                tier: holocrons::any(&opened.tier)?.slot,
                shards: opened.shards,
            }));
        }
        Shown::of(opened)
            .map(Self::Card)
            .or_else(|| Found::unknown_card(opened).map(Self::Shader))
    }

    pub(super) fn tier(&self) -> usize {
        match self {
            Self::Card(shown) => shown.tier,
            Self::Shader(found) => found.tier,
            Self::Shards(pile) => pile.tier,
        }
    }

    pub(super) fn holocron(&self) -> u64 {
        match self {
            Self::Card(shown) => shown.holocron,
            Self::Shader(found) => found.holocron,
            Self::Shards(pile) => pile.holocron,
        }
    }

    /// How long its entrance takes, and when its words come up (seconds into it): a card's
    /// pack takes longer than a shader or a pile.
    pub(super) fn entrance(&self) -> [f32; 2] {
        match self {
            Self::Card(_) => [CARD_ENTRANCE, CARD_WORDS],
            Self::Shader(_) | Self::Shards(_) => [ENTRANCE, WORDS],
        }
    }

    /// The tile the crate spin ends on.
    pub(super) fn item(&self) -> strip::Item {
        match self {
            Self::Card(shown) => strip::Item::Card {
                rarity: shown.rarity.index,
                finish: shown.finish.index,
                tier: shown.tier,
                credits: shown.credits,
            },
            Self::Shader(found) if crate::profile_card::banner::get(&found.id).is_some() => {
                strip::Item::Banner(crate::profile_card::banner::get(&found.id).unwrap())
            }
            Self::Shader(found) => match (found.skin, found.tag) {
                (Some(skin), _) => strip::Item::Shader(skin),
                (None, Some(tag)) => strip::Item::Tag(tag),
                (None, None) => strip::Item::New(found.kind()),
            },
            Self::Shards(pile) => strip::Item::Shards {
                tier: pile.tier,
                shards: pile.shards,
            },
        }
    }
}

/// Where the ceremony stands.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Ceremony {
    /// The hub was asked to open holocrons of tier `tier` (the first of them), at `since`.
    Waiting { tier: usize, since: Instant },
    /// Showing `shown[index]`, its entrance begun at `since`.
    Showing {
        shown: Vec<Prize>,
        index: usize,
        since: Instant,
    },
    /// More than [`ONE_BY_ONE`] opened at once: the first cracks open, their strips run
    /// five at a time, then everything found is laid out ([`super::batch`]), all on one
    /// clock begun at `since`.
    Batch { shown: Vec<Prize>, since: Instant },
    /// Nothing was opened: why.
    Failed { message: String, since: Instant },
}

impl Ceremony {
    /// How far into the holocron (or the batch) on show the ceremony is, in seconds.
    fn elapsed(&self, now: Instant) -> f32 {
        let since = match self {
            Self::Waiting { since, .. }
            | Self::Showing { since, .. }
            | Self::Batch { since, .. }
            | Self::Failed { since, .. } => *since,
        };
        now.saturating_duration_since(since).as_secs_f32()
    }

    /// The tier of the holocron the 3D one stands for: the one asked for, or opened.
    pub(crate) fn stage_tier(&self) -> Option<usize> {
        match self {
            Self::Waiting { tier, .. } => Some(*tier),
            Self::Showing { shown, index, .. } => shown.get(*index).map(Prize::tier),
            Self::Batch { shown, .. } => shown.first().map(Prize::tier),
            Self::Failed { .. } => None,
        }
    }
}

/// The cues the holocron (or round of strips) on show has played, so each plays once, and
/// the tile under each strip's marker last frame (a tick a tile).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Played {
    pub(super) charged: bool,
    pub(super) cracked: bool,
    pub(super) tick: [usize; super::batch::ROWS],
    /// A bit a strip that has come to rest.
    pub(super) landed: u8,
    pub(super) entered: bool,
    /// The batch's round these are of.
    pub(super) round: usize,
}

/// The line under a Solar blade card telling the blade it brought (20/10/2026): "...and
/// the Sun blade drops with it! Yours now: wear it in the Collection."
pub(super) fn bonus_line(found: &Found) -> String {
    let held = match (found.count, found.skin) {
        (count, _) if count > 1 => format!("You hold {count} of it now."),
        (_, Some(_)) => "Yours now: wear it in the Collection.".to_owned(),
        (_, None) => "Update SJK to wear it.".to_owned(),
    };
    format!("...and the {} drops with it! {held}", found.name)
}

/// The cue a card's entrance plays: the medals' fanfare for a Solar finish or a Legendary
/// or Mythical card, the unlocks' chime otherwise.
fn cue(prize: &Prize) -> Cue {
    match prize {
        Prize::Card(shown) if shown.finish.index == 2 || shown.rarity.index >= 3 => Cue::Medal,
        Prize::Shader(found)
            if found
                .skin
                .map(|skin| skin.tier)
                .or(found.tag.map(|tag| tag.tier))
                .or(crate::profile_card::banner::get(&found.id).map(|banner| banner.tier))
                .is_some_and(|tier| tier >= crate::unlockables::Rarity::Legendary) =>
        {
            Cue::Medal
        }
        _ => Cue::Achievement,
    }
}

impl Panel {
    /// Whether the ceremony (or the wait for the hub) shows.
    pub(crate) fn opening(&self) -> bool {
        self.ceremony.is_some()
    }

    /// The holocrons Open would open: the oldest of the tier chosen; none while the hub
    /// keeps opening to staff.
    pub(super) fn open_one(&self) -> Option<Vec<u64>> {
        if !self.data.can_open {
            return None;
        }
        self.data
            .sealed_of(self.selected)
            .first()
            .map(|id| vec![*id])
    }

    /// The holocrons Open all would open: every one listed, the oldest first, at most
    /// as many as one opening takes; on a kyber crystal's row, every crystal instead.
    pub(super) fn open_every(&self) -> Option<Vec<u64>> {
        if !self.data.can_open {
            return None;
        }
        let mut ids = self.data.sealed_kind(self.selected);
        ids.truncate(sjk_identity::OPEN_MAX);
        (!ids.is_empty()).then_some(ids)
    }

    /// Ask the hub to open `ids` (`Open` or `Open all`): the page waits for its answer.
    pub(crate) fn ask(&mut self, ids: Vec<u64>, now: Instant) -> PanelAction {
        if self.ceremony.is_some() || self.data.state != State::Known || !self.data.can_open {
            return PanelAction::None;
        }
        let first = ids.first().copied();
        let tier = first
            .and_then(|id| self.data.slot_of(id))
            .unwrap_or(self.selected);
        self.ceremony = Some(Ceremony::Waiting { tier, since: now });
        self.glide_from = Some(now);
        ui_cues::post(Cue::Click);
        PanelAction::Open(ids)
    }

    /// The identity could not take the request: say why.
    pub(crate) fn refused(&mut self, message: &str, now: Instant) {
        self.glide_from = None;
        self.ceremony = Some(Ceremony::Failed {
            message: crate::terms::say_owned(message.to_owned()),
            since: now,
        });
    }

    /// Follow the hub's answer: `outcome` is the identity's last opening (read every
    /// frame while the page waits; a new serial is this page's answer).
    pub(crate) fn follow_opening(
        &mut self,
        now: Instant,
        outcome: Option<&sjk_identity::OpenOutcome>,
    ) {
        let serial = outcome.map_or(0, |outcome| outcome.serial);
        let Some(Ceremony::Waiting { since, .. }) = &self.ceremony else {
            self.opening_serial = serial;
            return;
        };
        let since = *since;
        match outcome.filter(|_| serial > self.opening_serial) {
            Some(outcome) => {
                self.opening_serial = serial;
                // The page reads the hub's counts again at once.
                self.refreshed = None;
                let shown: Vec<Prize> = outcome.opened.iter().flat_map(Prize::all).collect();
                let bonuses: Vec<Prize> = outcome.opened.iter().filter_map(Prize::bonus).collect();
                if shown.is_empty() && !bonuses.is_empty() {
                    self.show(bonuses, now);
                } else if shown.is_empty() {
                    let message = if outcome.message.is_empty() {
                        "Nothing this client knows came out.".to_owned()
                    } else {
                        outcome.message.clone()
                    };
                    self.refused(&message, now);
                } else {
                    self.show(shown, now);
                    self.bonus_prizes = bonuses;
                }
            }
            None if now.saturating_duration_since(since) >= ANSWER_WAIT => {
                self.refreshed = None;
                self.refused("The SJK hub did not answer. Try again later.", now);
            }
            None => {}
        }
    }

    /// Play the ceremony of `shown` from the first.
    pub(crate) fn show(&mut self, shown: Vec<Prize>, now: Instant) {
        self.bonus_prizes.clear();
        self.played = Played::default();
        // The glide and the hum carry on from the wait (none for a rehearsal).
        self.waited = self.glide_from.map_or(0.0, |from| {
            now.saturating_duration_since(from).as_secs_f32()
        });
        self.ceremony = Some(if shown.len() > ONE_BY_ONE {
            Ceremony::Batch { shown, since: now }
        } else {
            Ceremony::Showing {
                shown,
                index: 0,
                since: now,
            }
        });
    }

    /// Enter, Space, Right or a click: finish the entrance, else go to the next card or
    /// end the ceremony after the last (or after a refusal).
    pub(super) fn advance(&mut self, now: Instant) {
        let back = |since: &mut Instant, to: f32| {
            *since = now
                .checked_sub(Duration::from_secs_f32(to))
                .unwrap_or(*since);
        };
        match &mut self.ceremony {
            Some(Ceremony::Showing {
                shown,
                index,
                since,
            }) => {
                let elapsed = now.saturating_duration_since(*since).as_secs_f32();
                let [entrance, _] = shown[*index].entrance();
                if elapsed < BEFORE {
                    // The holocron opens and the crate stops where it was going: on to
                    // the burst.
                    back(since, BEFORE);
                    self.played.charged = true;
                    self.played.cracked = true;
                } else if elapsed < BEFORE + entrance {
                    back(since, BEFORE + entrance);
                } else if *index + 1 < shown.len() {
                    *index += 1;
                    *since = now;
                    self.played = Played::default();
                    ui_cues::post(Cue::Click);
                } else {
                    self.finish_prizes(now);
                }
            }
            Some(Ceremony::Batch { shown, since }) => {
                let elapsed = now.saturating_duration_since(*since).as_secs_f32();
                match super::batch::skip(elapsed, shown.len()) {
                    Some(to) => {
                        back(since, to);
                        self.played.charged = true;
                        self.played.cracked = true;
                    }
                    None => self.finish_prizes(now),
                }
            }
            Some(Ceremony::Failed { .. }) => self.end_ceremony(),
            _ => {}
        }
    }

    fn finish_prizes(&mut self, now: Instant) {
        if self.bonus_prizes.is_empty() {
            self.end_ceremony();
        } else {
            let bonuses = std::mem::take(&mut self.bonus_prizes);
            self.show(bonuses, now);
        }
    }

    pub(super) fn done_label(&self) -> &'static str {
        if self.bonus_prizes.is_empty() {
            "Done"
        } else {
            "Bonus banners"
        }
    }

    /// End the ceremony (Escape, or Done): the page shows again, read afresh.
    pub(crate) fn end_ceremony(&mut self) {
        self.bonus_prizes.clear();
        self.glide_from = None;
        if self.ceremony.take().is_some() {
            ui_cues::post(Cue::Back);
            self.refreshed = None;
        }
    }

    /// A key while the ceremony shows.
    pub(super) fn ceremony_key(&mut self, key: KeyCode, now: Instant) -> PanelAction {
        match key {
            KeyCode::Escape => self.end_ceremony(),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space | KeyCode::ArrowRight => {
                self.advance(now);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// Draw the ceremony over the whole window.
    pub(super) fn draw_ceremony(&mut self, frame: &Frame, viewport: [f32; 2], now: Instant) {
        let Some(ceremony) = self.ceremony.clone() else {
            return;
        };
        let elapsed = self.ceremony_elapsed(&ceremony, now);
        // The world shows round the holocron while it is asked for and cracks open.
        let unsealing = match &ceremony {
            Ceremony::Waiting { .. } => true,
            Ceremony::Showing { .. } | Ceremony::Batch { .. } => elapsed < UNSEAL,
            Ceremony::Failed { .. } => false,
        };
        if !unsealing {
            let _ = self.ui.draw_list_mut().push(DrawCommand::SolidRect {
                rect: sjk_ui::Rect::new(0.0, 0.0, viewport[0], viewport[1]),
                color: color::alpha(color::SPACE, 0.93),
            });
        }
        let seconds = self.seconds(now);
        self.follow_skins();
        match &ceremony {
            Ceremony::Waiting { tier, .. } => self.draw_waiting(frame, viewport, *tier),
            Ceremony::Failed { message, .. } => self.failed(frame, message),
            Ceremony::Batch { shown, .. } => {
                self.draw_batch(frame, viewport, shown, elapsed, seconds);
            }
            Ceremony::Showing { shown, index, .. } if elapsed < UNSEAL => {
                let tier = shown[*index].tier();
                let of = (shown.len() > 1).then_some((*index, shown.len()));
                self.draw_unseal(
                    frame,
                    viewport,
                    tier,
                    elapsed,
                    format_args!("{}", holocrons::slot(tier).title()),
                    of,
                );
            }
            Ceremony::Showing { shown, index, .. } if elapsed < BEFORE => {
                self.spin(frame, viewport, &shown[*index], elapsed - UNSEAL, seconds);
            }
            Ceremony::Showing { shown, index, .. } => {
                let entered = elapsed - BEFORE;
                if !self.played.entered {
                    self.played.entered = true;
                    ui_cues::post(cue(&shown[*index]));
                }
                match &shown[*index] {
                    Prize::Card(card) => self.entrance(frame, card, entered, seconds),
                    Prize::Shader(found) => self.entrance_shader(frame, found, entered, seconds),
                    Prize::Shards(pile) => self.entrance_shards(frame, pile, entered),
                }
                if entered >= shown[*index].entrance()[1] {
                    match &shown[*index] {
                        Prize::Card(_) => self.words(frame, shown, *index, entered),
                        Prize::Shader(found) => {
                            self.shader_words(frame, found, shown.len(), *index, entered);
                        }
                        Prize::Shards(pile) => {
                            self.shards_words(frame, pile, shown.len(), *index, entered);
                        }
                    }
                }
            }
        }
        self.ceremony_keys(frame, &ceremony, elapsed);
    }

    /// The page's clock in seconds, for the finishes' motion.
    fn seconds(&self, now: Instant) -> f32 {
        #[cfg(test)]
        if let Some(seconds) = self.shot_seconds {
            return seconds;
        }
        now.saturating_duration_since(self.epoch).as_secs_f32()
    }

    /// How far into the holocron (or batch) on show `ceremony` is at `now`, in seconds. A
    /// world shot's time is into the entrance (negative: into the unseal and spin before
    /// it), or for a batch from its start.
    pub(super) fn ceremony_elapsed(&self, ceremony: &Ceremony, now: Instant) -> f32 {
        let elapsed = ceremony.elapsed(now);
        #[cfg(test)]
        if let Some(shot) = self.shot_elapsed {
            return match ceremony {
                Ceremony::Batch { .. } => shot,
                _ => BEFORE + shot,
            };
        }
        elapsed
    }

    /// Nothing was opened, and why.
    fn failed(&mut self, frame: &Frame, message: &str) {
        let s = frame.s;
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("Nothing was opened"),
            frame.rect(CARD_X - 500.0, 420.0, 1_000.0, 48.0),
            38.0 * s,
            color::TEXT,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{message}"),
            frame.rect(CARD_X - 500.0, 480.0, 1_000.0, 28.0),
            19.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            "Close",
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// The burst: the tier's gem flares and goes while rings run out and rays turn, round
    /// `middle` (frame pixels), `elapsed` seconds in.
    fn burst(&mut self, frame: &Frame, middle: [f32; 2], tier: &holocrons::Tier, elapsed: f32) {
        let s = frame.s;
        let [cx, cy] = frame.point(middle[0], middle[1]);
        // The burst: the gem flares and goes while rings run out and rays turn.
        if elapsed < BURST_END + 0.6 {
            let t = (elapsed / BURST_END).clamp(0.0, 1.0);
            let fade = 1.0 - ((elapsed - BURST_END) / 0.6).clamp(0.0, 1.0);
            glow(
                &mut self.ui,
                frame,
                middle,
                120.0 + 260.0 * t,
                tier.colour,
                0.5 * fade,
            );
            for ring in 0..3 {
                let r = (t - ring as f32 * 0.18).clamp(0.0, 1.0);
                if r <= 0.0 {
                    continue;
                }
                let _ = self.ui.draw_list_mut().push(DrawCommand::Arc {
                    center: [cx, cy],
                    radius: (60.0 + 340.0 * r) * s,
                    width: (6.0 - 4.0 * r) * s,
                    start: 0.0,
                    sweep: std::f32::consts::TAU,
                    color: tier.colour_alpha((1.0 - r) * 0.8 * fade),
                    knockout: None,
                });
            }
            for ray in 0..RAYS {
                let angle = ray as f32 / RAYS as f32 * std::f32::consts::TAU + elapsed * 0.8;
                let (sin, cos) = angle.sin_cos();
                let inner = (40.0 + 80.0 * t) * s;
                let outer = (120.0 + 420.0 * t) * s;
                let _ = self.ui.draw_list_mut().push(DrawCommand::Line {
                    from: [cx + cos * inner, cy + sin * inner],
                    to: [cx + cos * outer, cy + sin * outer],
                    width: 3.0 * s,
                    color: tier.colour_alpha(0.55 * fade * (1.0 - t * 0.5)),
                });
            }
            let gem_fade = 1.0 - ((elapsed - 0.25) / 0.45).clamp(0.0, 1.0);
            if gem_fade > 0.0 {
                gem::draw(
                    self.ui.draw_list_mut(),
                    [cx, cy],
                    (70.0 + 50.0 * t) * s,
                    tier.colour,
                    gem_fade,
                    gem::ROWS,
                );
            }
        }
    }

    /// A card's entrance as its pack (18/10/2026): the burst; the credits' tile rising in
    /// the middle, where the strip stopped, its card tucked face down in its corner, and
    /// its credits counting up; the tile sliding aside to the left while the card slides
    /// out of its corner to the middle, growing and straightening, and flips over to face
    /// the player (a Holo, Solar, Legendary or Mythical one in a flourish of its light); then the
    /// kyber shards' tile sliding out on the right and counting up (none while the hub's
    /// economy is off).
    fn entrance(&mut self, frame: &Frame, shown: &Shown, elapsed: f32, seconds: f32) {
        let tier = holocrons::slot(shown.tier);
        let faces = cards::faces::current();
        let (top, card_width) = shown.place(&faces);
        let height = card_width * cards::look::ASPECT;
        let middle = top + height * 0.5;
        self.burst(frame, [CARD_X, top + height * 0.47], tier, elapsed);
        let span = |[from, to]: [f32; 2]| ((elapsed - from) / (to - from)).clamp(0.0, 1.0);
        let ease = |t: f32| 1.0 - (1.0 - t).powi(3);
        let rise = ease(span(PACK_IN));
        let out = smooth(span(PACK_OUT));
        let flip = span(FLIP);
        if elapsed >= FLIP[1] {
            flourish(
                &mut self.ui,
                frame,
                [CARD_X, middle],
                [card_width, height],
                shown,
                cards::tone_in(&faces, shown.rarity, shown.card.as_deref()).colour,
                (elapsed - FLIP[1]) / FLOURISH,
            );
        }
        let reach = card_width * 0.5 + PACK_GAP + PACK_TILE[0] * 0.5;
        let tile = [CARD_X - reach * out, middle + 30.0 * (1.0 - rise)];
        if rise > 0.0 {
            let counted = (shown.credits as f32 * span(PACK_COUNT)).round() as u64;
            pack_tile(&mut self.ui, frame, tile, Currency::Credits, counted, rise);
            if flip < 0.5 {
                // Face down, out of the tile's corner to the middle, then turning edge-on.
                let from = [tile[0] + TUCKED_AT[0], tile[1] + TUCKED_AT[1]];
                let lerp = |a: f32, b: f32| a + (b - a) * out;
                tucked_card(
                    &mut self.ui,
                    frame,
                    [lerp(from[0], CARD_X), lerp(from[1], middle)],
                    lerp(TUCKED_WIDTH, card_width),
                    lerp(TUCKED_TURN, 0.0),
                    1.0 - smooth(flip * 2.0),
                    [shown.tier, shown.finish.index],
                    rise,
                );
            }
        }
        if flip < 0.5
            && let Some(prize) = cards::shown(&faces, shown.card.as_deref())
        {
            // The prize's picture as an invisible quad until the card turns face up, so
            // it is asked for from the burst on (pictures load when drawn) and the reveal
            // shows it rather than its dark window.
            let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                rect: frame.rect(CARD_X, top, 1.0, 1.0),
                texture: cards::faces::texture(prize.index),
                color: Color::new(0.0, 0.0, 0.0, 0.0),
            });
        }
        if flip >= 0.5 {
            // Face up: from edge-on to facing the player.
            let turn = smooth((flip - 0.5) * 2.0);
            cards::look::draw(
                &mut self.ui,
                frame,
                [CARD_X - card_width * 0.5, top, card_width],
                cards::look::Card {
                    rarity: shown.rarity,
                    finish: shown.finish,
                    edition: &shown.edition,
                    face: cards::shown(&faces, shown.card.as_deref()),
                    tilt: None,
                    dim: false,
                    label: false,
                },
                seconds,
                1.0,
                turn,
            );
        }
        // With a blade beside the shards, the two stand one over the other.
        let apart = if shown.shards > 0 && shown.bonus.is_some() {
            PACK_TILE[1] * 0.5 + 12.0
        } else {
            0.0
        };
        let slide = ease(span(SHARDS_OUT));
        if shown.shards > 0 && slide > 0.0 {
            let counted = (shown.shards as f32 * span(SHARDS_COUNT)).round() as u64;
            pack_tile(
                &mut self.ui,
                frame,
                [CARD_X + reach - PACK_SLIDE * (1.0 - slide), middle - apart],
                Currency::Shards,
                counted,
                slide,
            );
        }
        let slide = ease(span(BONUS_OUT));
        if let Some(found) = shown.bonus.as_ref().filter(|_| slide > 0.0) {
            self.bonus_tile(
                frame,
                [CARD_X + reach - PACK_SLIDE * (1.0 - slide), middle + apart],
                found,
                slide,
                seconds,
            );
        }
    }

    /// The blade a Solar blade card brought (20/10/2026), as a tile of its pack centred on
    /// `centre` (frame pixels): its blade, framed in its tier's colour, its name and how
    /// many the player holds.
    fn bonus_tile(
        &mut self,
        frame: &Frame,
        centre: [f32; 2],
        found: &Found,
        alpha: f32,
        seconds: f32,
    ) {
        let s = frame.s;
        let [width, height] = PACK_TILE;
        let [left, top] = [centre[0] - width * 0.5, centre[1] - height * 0.5];
        let colour = found.colour();
        glow(&mut self.ui, frame, centre, 150.0, colour, 0.22 * alpha);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PushOpacity(alpha));
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(left + 4.0, top + 10.0, width, height),
            radius: 18.0 * s,
            color: color::alpha(color::SPACE, 0.6),
        });
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(left, top, width, height),
            radius: 18.0 * s,
            color: Color::new(0.04, 0.06, 0.12, 0.95),
        });
        let look = self.skins.get(&found.id);
        let _ = crate::console::collection_panel::swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [left + 14.0, top + 26.0, width - 28.0, 70.0],
            look,
            true,
            seconds,
            0.0,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: frame.rect(left, top, width, height),
            radius: 18.0 * s,
            width: 2.0 * s,
            color: color::alpha(colour, 0.85),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("+{}", found.name),
            frame.rect(left, top + 108.0, width, 40.0),
            28.0 * s,
            colour,
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let label = if found.count > 1 {
            format!("{}, x{}", found.kind(), found.count)
        } else {
            format!("{}, new", found.kind())
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{label}"),
            frame.rect(left, top + 158.0, width, 26.0),
            17.0 * s,
            color::MUTED,
            FontWeight::Regular,
            TextAlign::Center,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
    }

    /// The card's words, fading up, and the button.
    fn words(&mut self, frame: &Frame, shown: &[Prize], index: usize, elapsed: f32) {
        let s = frame.s;
        let Prize::Card(card) = &shown[index] else {
            return;
        };
        let alpha = ((elapsed - CARD_WORDS) / 0.4).clamp(0.0, 1.0);
        let fade = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * alpha);
        let faces = cards::faces::current();
        let face = cards::shown(&faces, card.card.as_deref());
        let stack = cards::Stack {
            rarity: card.rarity,
            finish: card.finish,
            edition: card.edition.clone(),
            card: card.card.clone(),
            count: 1,
            first: 0,
        };
        let title = stack.name_in(&faces);
        let tone = stack.tone_in(&faces);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{title}"),
            frame.rect(CARD_X - 500.0, TITLE_Y, 1_000.0, 50.0),
            42.0 * s,
            fade(if card.finish.index == 2 {
                color::GOLD_BRIGHT
            } else {
                tone.colour
            }),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let tier = holocrons::slot(card.tier);
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}{}From {} {}",
                if stack.sealed() {
                    String::new()
                } else {
                    format!("{}, {}. ", tone.name, card.finish.name)
                },
                if card.edition.is_empty() {
                    String::new()
                } else {
                    format!("{}. ", card.edition)
                },
                tier.article(),
                tier.title()
            ),
            frame.rect(CARD_X - 500.0, EDITION_Y, 1_000.0, 28.0),
            19.0 * s,
            fade(color::TEXT),
            FontWeight::Regular,
            TextAlign::Center,
        );
        // A Solar blade card's blade is told in place of what the finish is.
        let (about, about_colour) = match &card.bonus {
            Some(found) => (bonus_line(found), found.colour()),
            None => (card.finish.about.to_owned(), color::MUTED),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{about}"),
            frame.rect(CARD_X - 600.0, ABOUT_Y, 1_200.0, 26.0),
            17.0 * s,
            fade(about_colour),
            FontWeight::Regular,
            TextAlign::Center,
        );
        // Its kind and collection, no number (Sol, 20/10/2026).
        let last_line = match face {
            Some(face) => format!("{}, {}", face.face.kind.name(), face.face.set_name),
            None if stack.sealed() => "Sealed: its card is picked when it is revealed.".to_owned(),
            None => "Its face is not in your card pack yet.".to_owned(),
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{last_line}"),
            frame.rect(CARD_X - 500.0, SEALED_Y, 1_000.0, 26.0),
            17.0 * s,
            fade(color::QUIET),
            FontWeight::Regular,
            TextAlign::Center,
        );
        if shown.len() > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{} of {}", index + 1, shown.len()),
                frame.rect(
                    CARD_X - 300.0,
                    if face.is_some() { 20.0 } else { 120.0 },
                    600.0,
                    34.0,
                ),
                24.0 * s,
                fade(color::MUTED),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        let last = index + 1 == shown.len();
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            if last {
                if self.bonus_prizes.is_empty() {
                    "Done"
                } else {
                    "Bonus banners"
                }
            } else {
                "Next"
            },
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// The crate spin, `elapsed` seconds in: out of the holocron's flash a strip of what
    /// its tier can drop runs past the marker, a tick a tile, each tile trailing its
    /// colour at speed over its colour band, and slows to a stop on the prize; at rest the
    /// prize lifts and lights up (rays behind a grand one) while the others dim.
    fn spin(
        &mut self,
        frame: &Frame,
        viewport: [f32; 2],
        prize: &Prize,
        elapsed: f32,
        seconds: f32,
    ) {
        let s = frame.s;
        let tier = holocrons::slot(prize.tier());
        let geometry = strip::SINGLE;
        let item = prize.item();
        let tiles = strip::strip(prize.tier(), prize.holocron(), item);
        let run = strip::run(prize.holocron(), geometry);
        let running = SPIN * SPIN_RUN;
        let t = elapsed / running;
        let distance = run * strip::travelled(t);
        let speed = run * strip::speed(t, running);
        let under = strip::under_marker(distance, geometry);
        if under != self.played.tick[0] {
            self.played.tick[0] = under;
            ui_cues::post(Cue::CrateTick);
        }
        let settled = elapsed >= running;
        if settled && self.played.landed & 1 == 0 {
            self.played.landed |= 1;
            ui_cues::post(Cue::CrateLand);
        }
        // How lit the prize is at rest (0 to 1, quickly).
        let lit = if settled {
            smooth((elapsed - running) / (0.35 * (SPIN - running)))
        } else {
            0.0
        };
        let middle = [CARD_X, STRIP_Y + STRIP_HEIGHT * 0.5];
        glow(&mut self.ui, frame, middle, 420.0, tier.colour, 0.16);
        let appear = (elapsed / 0.35).clamp(0.0, 1.0);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", tier.title()),
            frame.rect(CARD_X - 500.0, STRIP_Y - 120.0, 1_000.0, 50.0),
            38.0 * s,
            tier.colour_alpha(appear),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        if lit > 0.0 {
            prize_light(&mut self.ui, frame, middle, item, lit, seconds, 1.0);
        }
        let pitch = geometry.pitch();
        let height = STRIP_HEIGHT;
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PushOpacity(appear));
        let _ = list.push(DrawCommand::PushClip(frame.rect(
            STRIP_LEFT,
            STRIP_Y - 30.0,
            STRIP_RIGHT - STRIP_LEFT,
            height + 60.0,
        )));
        for (at, tile) in tiles.iter().enumerate() {
            let x = CARD_X - strip::TILE * 0.5 + at as f32 * pitch - distance;
            if x + strip::TILE + SMEAR_MAX < STRIP_LEFT || x > STRIP_RIGHT {
                continue;
            }
            // The tile under the marker lifts a little; at rest the prize more.
            let near = (1.0 - ((x + strip::TILE * 0.5 - CARD_X).abs() / pitch)).clamp(0.0, 1.0);
            let won = at == strip::WIN;
            let lift = if won && settled {
                10.0 + 8.0 * lit
            } else {
                10.0 * near
            };
            let dim = if won { 1.0 } else { 1.0 - 0.65 * lit };
            self.strip_tile(
                frame,
                *tile,
                [x, STRIP_Y - lift, strip::TILE],
                near,
                dim,
                speed,
                seconds,
            );
        }
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::PopClip);
        // The strip fades into the dark at both ends.
        strip_ends(
            &mut self.ui,
            frame,
            [STRIP_LEFT, STRIP_RIGHT],
            STRIP_Y - 30.0,
            height + 60.0,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
        marker(
            &mut self.ui,
            frame,
            CARD_X,
            [STRIP_Y - 36.0, STRIP_Y + height + 36.0],
            color::alpha(color::GOLD_BRIGHT, 0.9),
            1.0,
        );
        // The holocron's flash fades over the strip.
        super::unseal::flash(
            &mut self.ui,
            viewport,
            tier.colour,
            1.0 - elapsed / super::unseal::FLASH_OUT,
        );
    }

    /// One tile of a strip at `[x, y, width]` (frame pixels), `dim` its opacity, trailing
    /// its colour behind it at `speed` (frame pixels a second, the strip running left),
    /// over its colour band.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn strip_tile(
        &mut self,
        frame: &Frame,
        item: strip::Item,
        [x, y, width]: [f32; 3],
        near: f32,
        dim: f32,
        speed: f32,
        seconds: f32,
    ) {
        let s = frame.s;
        let height = width * cards::look::ASPECT;
        let colour = item.colour();
        let list = self.ui.draw_list_mut();
        if dim < 1.0 {
            let _ = list.push(DrawCommand::PushOpacity(dim));
        }
        let _ = list.push(DrawCommand::RoundedRect {
            rect: frame.rect(x, y + height + BAND_GAP * width / strip::TILE, width, BAND),
            radius: BAND * 0.5 * s,
            color: color::alpha(colour, 0.9),
        });
        self.tile(frame, item, [x, y, width], near, seconds);
        // At speed its colour streaks across it and trails it (the strip runs left),
        // longer and stronger the faster it runs.
        let trail = (speed * 0.03).min(SMEAR_MAX) * width / strip::TILE;
        if trail > 4.0 {
            let strength = (speed / 3_000.0).min(1.0) * 0.35;
            let list = self.ui.draw_list_mut();
            for (rect, start, end) in [
                (
                    frame.rect(x + width * 0.15, y, width * 0.85, height),
                    0.0,
                    strength,
                ),
                (
                    frame.rect(x + width, y + height * 0.1, trail, height * 0.8),
                    strength,
                    0.0,
                ),
            ] {
                let _ = list.push(DrawCommand::GradientRect {
                    rect,
                    radius: 0.0,
                    gradient: sjk_ui::Gradient {
                        start: color::alpha(colour, start),
                        end: color::alpha(colour, end),
                        vertical: false,
                    },
                });
            }
        }
        if dim < 1.0 {
            let _ = self.ui.draw_list_mut().push(DrawCommand::PopOpacity);
        }
    }

    /// One tile of the strip at `[x, y, width]` (frame pixels): a card's credits with the
    /// card tucked face down in its corner in its rarity and finish, a shader's blade
    /// framed in its tier's colour with its name, or a pile of kyber shards. `near` (0 to
    /// 1) is how close it is to the marker.
    pub(super) fn tile(
        &mut self,
        frame: &Frame,
        item: strip::Item,
        [x, y, width]: [f32; 3],
        near: f32,
        seconds: f32,
    ) {
        if width < MINI {
            self.mini_tile(frame, item, [x, y, width]);
            return;
        }
        let s = frame.s;
        // Everything inside a tile is laid out for the strip's own width.
        let k = width / strip::TILE;
        let height = width * cards::look::ASPECT;
        let rect = frame.rect(x, y, width, height);
        match item {
            strip::Item::Card {
                finish,
                tier,
                credits,
                ..
            } => {
                // Its credits, a stack of coins and the amount, washed from above in the
                // holocron's colour, the card tucked face down in the top right corner.
                let colour = item.colour();
                let wash = holocrons::slot(tier).colour;
                let list = self.ui.draw_list_mut();
                let _ = list.push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let _ = list.push(DrawCommand::GradientRect {
                    rect,
                    radius: 14.0 * k * s,
                    gradient: sjk_ui::Gradient {
                        start: color::alpha(wash, 0.26 + 0.08 * near),
                        end: color::alpha(wash, 0.03),
                        vertical: true,
                    },
                });
                credits_stack(
                    &mut self.ui,
                    frame,
                    [x + width * 0.44, y + height * 0.44],
                    40.0 * k,
                    1.0,
                );
                tucked_card(
                    &mut self.ui,
                    frame,
                    [x + width - 34.0 * k, y + 42.0 * k],
                    58.0 * k,
                    TUCKED_TURN,
                    1.0,
                    [tier, finish],
                    1.0,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(colour, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("+{}", cards::credits_text(credits)),
                    frame.rect(x, y + height - 78.0 * k, width, 38.0 * k),
                    30.0 * k * s,
                    color::GOLD_BRIGHT,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("credits"),
                    frame.rect(x, y + height - 38.0 * k, width, 22.0 * k),
                    15.0 * k * s,
                    color::MUTED,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
            strip::Item::Shader(skin) => {
                let colour = skin.tier.colour();
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let look = self.skins.get(skin.id);
                let _ = crate::console::collection_panel::swatch::small_swatch(
                    self.ui.draw_list_mut(),
                    frame,
                    [
                        x + 10.0 * k,
                        y + height * 0.5 - 38.0 * k,
                        width - 20.0 * k,
                        76.0 * k,
                    ],
                    look,
                    true,
                    seconds,
                    0.0,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(colour, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{} {}", skin.tier.symbol(), skin.tier.label()),
                    frame.rect(x, y + 12.0 * k, width, 24.0 * k),
                    16.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{}", skin.name),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
            strip::Item::Shards { tier, shards } => {
                let colour = item.colour();
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                glow(
                    &mut self.ui,
                    frame,
                    [x + width * 0.5, y + height * 0.5],
                    60.0 * k,
                    colour,
                    0.25 + 0.15 * near,
                );
                crystals(
                    &mut self.ui,
                    frame,
                    [x + width * 0.5, y + height * 0.5 + 6.0 * k],
                    92.0 * k,
                    1.0,
                );
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(colour, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("Kyber shards"),
                    frame.rect(x, y + 12.0 * k, width, 24.0 * k),
                    16.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                let (least, most) =
                    crate::loot_odds::current().shards[tier.min(holocrons::COUNT - 1)];
                let words = if shards > 0 {
                    format!("+{}", crate::economy::shards_text(shards))
                } else {
                    format!("{least} to {most}")
                };
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("{words}"),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
            strip::Item::Tag(tag) => {
                let colour = item.colour();
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(colour, 0.7 + 0.3 * near),
                });
                let side = (width - 24.0 * k).min(height - 70.0 * k);
                self.prize_tag(
                    frame,
                    tag,
                    [x + (width - side) * 0.5, y + 14.0 * k, side, side],
                    1.0,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("Tag"),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
            strip::Item::Banner(banner) => {
                crate::profile_card::banner::draw(&mut self.ui, rect, banner.id);
            }
            strip::Item::New(kind) => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
                    rect,
                    radius: 14.0 * k * s,
                    color: color::alpha(color::SPACE, 0.9),
                });
                let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                    rect,
                    radius: 14.0 * k * s,
                    width: (2.0 + near) * s,
                    color: color::alpha(color::GOLD, 0.7 + 0.3 * near),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("?"),
                    frame.rect(x, y + height * 0.5 - 40.0 * k, width, 70.0 * k),
                    60.0 * k * s,
                    color::GOLD,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
                text(
                    &mut self.ui,
                    TextFamily::Body,
                    format_args!("New {}", short_kind(kind)),
                    frame.rect(x, y + height - 40.0 * k, width, 26.0 * k),
                    17.0 * k * s,
                    color::TEXT,
                    FontWeight::Regular,
                    TextAlign::Center,
                );
            }
        }
    }

    /// A shader's burst and blade: the tier's light, then the blade swatch growing in
    /// large, framed in the shader's tier colour.
    fn entrance_shader(&mut self, frame: &Frame, found: &Found, elapsed: f32, seconds: f32) {
        let s = frame.s;
        let tier = holocrons::slot(found.tier);
        let colour = found.colour();
        let middle = [CARD_X, SHADER_TOP + SHADER_HEIGHT * 0.5];
        self.burst(frame, middle, tier, elapsed);
        let turn = ((elapsed - TURN[0]) / (TURN[1] - TURN[0])).clamp(0.0, 1.0);
        if turn <= 0.0 {
            return;
        }
        let eased = 1.0 - (1.0 - turn).powi(3);
        let width = SHADER_WIDTH * (0.85 + 0.15 * eased);
        let height = SHADER_HEIGHT * (0.85 + 0.15 * eased);
        let rect = [
            CARD_X - width * 0.5,
            SHADER_TOP + (SHADER_HEIGHT - height) * 0.5,
            width,
            height,
        ];
        glow(&mut self.ui, frame, middle, 300.0, colour, 0.3 * eased);
        let area = frame.rect(rect[0], rect[1], rect[2], rect[3]);
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: area,
            radius: 22.0 * s,
            color: color::alpha(color::SPACE, 0.95 * eased),
        });
        if let Some(banner) = crate::profile_card::banner::get(&found.id) {
            crate::profile_card::banner::draw_rounded(&mut self.ui, area, banner.id, 22.0 * s);
            return;
        }
        // A tag stands as its picture.
        if let Some(tag) = found.tag {
            let side = (height - 60.0).min(width - 60.0);
            self.prize_tag(
                frame,
                tag,
                [CARD_X - side * 0.5, middle[1] - side * 0.5, side, side],
                eased,
            );
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: area,
                radius: 22.0 * s,
                width: 3.0 * s,
                color: color::alpha(colour, eased),
            });
            return;
        }
        // The blade the hub's pack brought, even for a shader this client does not list.
        let look = self.skins.get(&found.id);
        // Nothing this client can draw it with: a fallback picture, not an empty swatch.
        if look.is_none() && found.is_unknown() {
            self.unknown_picture(frame, colour, middle, height, eased);
            let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
                rect: area,
                radius: 22.0 * s,
                width: 3.0 * s,
                color: color::alpha(colour, eased),
            });
            return;
        }
        if let Some(look) = look {
            crate::blade_pictures::focus(look.id);
        }
        let _ = crate::console::collection_panel::swatch::small_swatch(
            self.ui.draw_list_mut(),
            frame,
            [
                rect[0] + 24.0,
                rect[1] + height * 0.5 - 90.0,
                rect[2] - 48.0,
                180.0,
            ],
            look,
            true,
            seconds,
            0.0,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: area,
            radius: 22.0 * s,
            width: 3.0 * s,
            color: color::alpha(colour, eased),
        });
    }

    /// The picture of a prize this client does not know (Kushina, 30/10/2026): a large
    /// question mark in a ring of `colour`, centred on `middle`, `height` tall at most.
    fn unknown_picture(
        &mut self,
        frame: &Frame,
        colour: Color,
        middle: [f32; 2],
        height: f32,
        alpha: f32,
    ) {
        let s = frame.s;
        let side = (height * 0.55).min(220.0);
        let ring = frame.rect(
            middle[0] - side * 0.5,
            middle[1] - side * 0.5 - 20.0,
            side,
            side,
        );
        let _ = self.ui.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: ring,
            radius: side * 0.5 * s,
            color: color::alpha(colour, 0.12 * alpha),
        });
        let _ = self.ui.draw_list_mut().push(DrawCommand::Border {
            rect: ring,
            radius: side * 0.5 * s,
            width: 3.0 * s,
            color: color::alpha(colour, 0.8 * alpha),
        });
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("?"),
            frame.rect(
                middle[0] - side * 0.5,
                middle[1] - side * 0.35 - 20.0,
                side,
                side * 0.7,
            ),
            side * 0.6 * s,
            color::alpha(colour, alpha),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("Update SJK to see it"),
            frame.rect(middle[0] - 300.0, middle[1] + side * 0.5, 600.0, 30.0),
            20.0 * s,
            color::alpha(color::TEXT, alpha),
            FontWeight::Regular,
            TextAlign::Center,
        );
    }

    /// Tag `tag`'s picture over `[x, y, width, height]` at `alpha`, or its name in its
    /// tier's colour while the hub's pack has not brought the picture.
    pub(super) fn prize_tag(
        &mut self,
        frame: &Frame,
        tag: &crate::tags::Tag,
        [x, y, width, height]: [f32; 4],
        alpha: f32,
    ) {
        match crate::tag_icons::texture(tag.id) {
            Some(texture) => {
                let _ = self.ui.draw_list_mut().push(DrawCommand::TexturedQuad {
                    rect: frame.rect(x, y, width, height),
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, alpha),
                });
            }
            None => {
                let tier = tag.tier.colour();
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("{}", tag.name.trim_end_matches(" tag")),
                    frame.rect(x, y + height * 0.5 - 20.0, width, 40.0),
                    (height * 0.18).clamp(14.0, 34.0) * frame.s,
                    Color::new(tier.r, tier.g, tier.b, alpha),
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
    }

    /// A pile of shards' burst and crystals: the tier's light, then three kyber crystals
    /// growing in, glowing, and the shards counting up under them.
    fn entrance_shards(&mut self, frame: &Frame, pile: &Pile, elapsed: f32) {
        let tier = holocrons::slot(pile.tier);
        let middle = [CARD_X, SHADER_TOP + SHADER_HEIGHT * 0.5];
        self.burst(frame, middle, tier, elapsed);
        let turn = ((elapsed - TURN[0]) / (TURN[1] - TURN[0])).clamp(0.0, 1.0);
        if turn <= 0.0 {
            return;
        }
        let eased = 1.0 - (1.0 - turn).powi(3);
        let kyber = cards::shard::KYBER;
        glow(&mut self.ui, frame, middle, 300.0, kyber, 0.35 * eased);
        crystals(
            &mut self.ui,
            frame,
            [CARD_X, middle[1] + 10.0],
            SHADER_HEIGHT * (0.7 + 0.2 * eased),
            eased,
        );
        if elapsed >= COUNT[0] {
            let t = ((elapsed - COUNT[0]) / (COUNT[1] - COUNT[0])).clamp(0.0, 1.0);
            let counted = (pile.shards as f32 * t).round() as u64;
            shards_line(
                &mut self.ui,
                frame,
                CARD_X,
                CREDITS_Y,
                &format!("+{}", crate::economy::shards_text(counted)),
                38.0,
                (t * 3.0).min(1.0),
            );
        }
    }

    /// What a pile of shards is and does, fading up, and the button.
    fn shards_words(
        &mut self,
        frame: &Frame,
        pile: &Pile,
        count: usize,
        index: usize,
        entered: f32,
    ) {
        let s = frame.s;
        let alpha = ((entered - WORDS) / 0.4).clamp(0.0, 1.0);
        let fade = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * alpha);
        let tier = holocrons::slot(pile.tier);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!(
                "{} kyber shard{}",
                crate::economy::shards_text(pile.shards),
                if pile.shards == 1 { "" } else { "s" }
            ),
            frame.rect(CARD_X - 500.0, TITLE_Y, 1_000.0, 50.0),
            42.0 * s,
            fade(cards::shard::KYBER),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("From {} {}", tier.article(), tier.title()),
            frame.rect(CARD_X - 500.0, EDITION_Y, 1_000.0, 28.0),
            19.0 * s,
            fade(color::TEXT),
            FontWeight::Regular,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                crate::terms::say(
                    "Kyber shards craft the saber shader you choose, among those holocrons drop."
                )
            ),
            frame.rect(CARD_X - 600.0, ABOUT_Y, 1_200.0, 26.0),
            17.0 * s,
            fade(color::MUTED),
            FontWeight::Regular,
            TextAlign::Center,
        );
        if let Some(held) = self.data.shards {
            text(
                &mut self.ui,
                TextFamily::Body,
                format_args!(
                    "You hold {} kyber shards now.",
                    crate::economy::shards_text(held)
                ),
                frame.rect(CARD_X - 500.0, SEALED_Y, 1_000.0, 26.0),
                17.0 * s,
                fade(color::QUIET),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        if count > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{} of {count}", index + 1),
                frame.rect(CARD_X - 300.0, 120.0, 600.0, 34.0),
                24.0 * s,
                fade(color::MUTED),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            if index + 1 == count {
                if self.bonus_prizes.is_empty() {
                    "Done"
                } else {
                    "Bonus banners"
                }
            } else {
                "Next"
            },
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// What the shader is, fading up, and the button.
    fn shader_words(
        &mut self,
        frame: &Frame,
        found: &Found,
        count: usize,
        index: usize,
        entered: f32,
    ) {
        let s = frame.s;
        let alpha = ((entered - WORDS) / 0.4).clamp(0.0, 1.0);
        let fade = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * alpha);
        let tier = holocrons::slot(found.tier);
        text(
            &mut self.ui,
            TextFamily::Display,
            format_args!("{}", found.name),
            frame.rect(CARD_X - 500.0, TITLE_Y, 1_000.0, 50.0),
            42.0 * s,
            fade(found.colour()),
            FontWeight::Semibold,
            TextAlign::Center,
        );
        let kind = if let Some(banner) = crate::profile_card::banner::get(&found.id) {
            format!("{} bonus profile banner", banner.tier.label())
        } else {
            match (found.skin, found.tag) {
                (Some(skin), _) => {
                    format!("{} {} saber shader", skin.tier.symbol(), skin.tier.label())
                }
                (None, Some(tag)) => format!("{} {} tag", tag.tier.symbol(), tag.tier.label()),
                (None, None) => {
                    let kind = found.kind();
                    let article = if kind.starts_with(['a', 'e', 'i', 'o', 'u']) {
                        "An"
                    } else {
                        "A"
                    };
                    format!("{article} {kind} new to this version of SJK")
                }
            }
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{kind}. From {} {}", tier.article(), tier.title()),
            frame.rect(CARD_X - 500.0, EDITION_Y, 1_000.0, 28.0),
            19.0 * s,
            fade(color::TEXT),
            FontWeight::Regular,
            TextAlign::Center,
        );
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!(
                "{}",
                if crate::profile_card::banner::get(&found.id).is_some() {
                    "Equip it in Collection, Banners. It appears on your profile and player cards."
                } else {
                    match (found.skin, found.tag) {
                        (Some(skin), _) => skin.description,
                        (None, Some(_)) => {
                            "Stick it on walls and floors with toy_tag; everyone on the map sees it."
                        }
                        (None, None) => "It is yours: update SJK to see it in your Collection.",
                    }
                }
            ),
            frame.rect(CARD_X - 600.0, ABOUT_Y, 1_200.0, 26.0),
            17.0 * s,
            fade(color::MUTED),
            FontWeight::Regular,
            TextAlign::Center,
        );
        let held = if crate::profile_card::banner::get(&found.id).is_some() {
            format!(
                "Yours, {} copies. Equip one in Collection, Banners.",
                found.count
            )
        } else {
            match (found.count, found.skin, found.tag) {
                (count, _, _) if count > 1 => format!("You hold {count} of it now."),
                (_, Some(_), _) => "Yours now: wear it in the Collection.".to_owned(),
                (_, None, Some(_)) => {
                    "Yours now: choose it on the Collection's Tags tab.".to_owned()
                }
                // Its line above says where it will show.
                (_, None, None) => String::new(),
            }
        };
        text(
            &mut self.ui,
            TextFamily::Body,
            format_args!("{held}"),
            frame.rect(CARD_X - 500.0, SEALED_Y, 1_000.0, 26.0),
            17.0 * s,
            fade(color::QUIET),
            FontWeight::Regular,
            TextAlign::Center,
        );
        if count > 1 {
            text(
                &mut self.ui,
                TextFamily::Display,
                format_args!("{} of {count}", index + 1),
                frame.rect(CARD_X - 300.0, 120.0, 600.0, 34.0),
                24.0 * s,
                fade(color::MUTED),
                FontWeight::Regular,
                TextAlign::Center,
            );
        }
        kit::button(
            &mut self.ui,
            frame,
            [CARD_X - 110.0, BUTTON_Y, 220.0, 48.0],
            if index + 1 == count {
                if self.bonus_prizes.is_empty() {
                    "Done"
                } else {
                    "Bonus banners"
                }
            } else {
                "Next"
            },
            true,
            true,
            true,
            NEXT_TOKEN,
        );
    }

    /// Take the loaded blade skins when a pack brought new ones (the tiles' blades).
    fn follow_skins(&mut self) {
        if self.skins.generation() != crate::sjk_packs::generation() {
            self.skins = crate::sjk_packs::skins();
        }
    }

    /// The keys under the ceremony.
    fn ceremony_keys(&mut self, frame: &Frame, ceremony: &Ceremony, elapsed: f32) {
        let s = frame.s;
        let action = match ceremony {
            Ceremony::Waiting { .. } => None,
            Ceremony::Failed { .. } => Some("close"),
            Ceremony::Showing { shown, index, .. } => {
                Some(if elapsed < BEFORE + shown[*index].entrance()[0] {
                    "skip"
                } else if index + 1 < shown.len() {
                    "next"
                } else if !self.bonus_prizes.is_empty() {
                    "bonus banners"
                } else {
                    "done"
                })
            }
            Ceremony::Batch { shown, .. } => {
                Some(if super::batch::summing_up(elapsed, shown.len()) {
                    if self.bonus_prizes.is_empty() {
                        "done"
                    } else {
                        "bonus banners"
                    }
                } else {
                    "skip"
                })
            }
        };
        let mut keys: [(&[&str], &str); 2] = [(&["Esc"], "close"); 2];
        let mut count = 0;
        if let Some(action) = action {
            keys[count] = (&["Enter"], action);
            count += 1;
        }
        keys[count] = (&["Esc"], "close");
        count += 1;
        let keys = &keys[..count];
        let gap = 30.0 * s;
        let width: f32 = keys
            .iter()
            .map(|(caps, action)| key_hint_width(caps, action, s))
            .sum::<f32>()
            + gap * (keys.len() - 1) as f32;
        let [right, y] = frame.point(1_824.0, super::view::KEYS_Y);
        let mut x = right - width;
        for (caps, action) in keys {
            x = key_hint(&mut self.ui, caps, action, x, y, s) + gap;
        }
    }
}

/// The longest streak a tile trails at speed (frame pixels, for the strip's own tile).
const SMEAR_MAX: f32 = 150.0;
/// Tiles narrower than this (frame pixels; the stacked strips' of an opening of many) are
/// drawn light: their colour, a mark and a blade or a symbol, not a whole card.
const MINI: f32 = 120.0;

impl Panel {
    /// A light tile at `[x, y, width]` (frame pixels): a dark plate washed from below in
    /// its colour, edged in it, with a card's credits (a stack of coins, the card tucked in
    /// its corner and the amount) or a shader's blade, or "?" for a shader this client does
    /// not know.
    fn mini_tile(&mut self, frame: &Frame, item: strip::Item, [x, y, width]: [f32; 3]) {
        let s = frame.s;
        let k = width / strip::STACKED.tile;
        let height = width * cards::look::ASPECT;
        let colour = item.colour();
        let rect = frame.rect(x, y, width, height);
        let list = self.ui.draw_list_mut();
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius: 8.0 * k * s,
            color: color::alpha(color::SPACE, 0.92),
        });
        let _ = list.push(DrawCommand::GradientRect {
            rect,
            radius: 8.0 * k * s,
            gradient: sjk_ui::Gradient {
                start: color::alpha(colour, 0.04),
                end: color::alpha(colour, 0.34),
                vertical: true,
            },
        });
        match item {
            strip::Item::Card {
                finish,
                tier,
                credits,
                ..
            } => {
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                credits_stack(
                    &mut self.ui,
                    frame,
                    [x + width * 0.42, y + height * 0.42],
                    21.0 * k,
                    1.0,
                );
                tucked_card(
                    &mut self.ui,
                    frame,
                    [x + width - 17.0 * k, y + 22.0 * k],
                    30.0 * k,
                    TUCKED_TURN,
                    1.0,
                    [tier, finish],
                    1.0,
                );
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("+{}", cards::credits_text(credits)),
                    frame.rect(x, y + height - 32.0 * k, width, 24.0 * k),
                    18.0 * k * s,
                    color::GOLD_BRIGHT,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
            strip::Item::Shader(_) => {
                // A blade across the tile: its glow, its white core, a short hilt.
                let from = frame.point(x + width * 0.34, y + height * 0.76);
                let to = frame.point(x + width * 0.70, y + height * 0.2);
                let hilt = frame.point(x + width * 0.26, y + height * 0.88);
                let _ = list.push(DrawCommand::Line {
                    from,
                    to,
                    width: 12.0 * k * s,
                    color: color::alpha(colour, 0.45),
                });
                let _ = list.push(DrawCommand::Line {
                    from,
                    to,
                    width: 3.5 * k * s,
                    color: color::alpha(sjk_ui::Color::new(1.0, 1.0, 1.0, 1.0), 0.95),
                });
                let _ = list.push(DrawCommand::Line {
                    from: hilt,
                    to: from,
                    width: 5.0 * k * s,
                    color: color::alpha(color::MUTED, 0.9),
                });
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
            }
            strip::Item::Shards { .. } => {
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                crystals(
                    &mut self.ui,
                    frame,
                    [x + width * 0.5, y + height * 0.5],
                    56.0 * k,
                    1.0,
                );
            }
            strip::Item::Tag(tag) => {
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                let side = (width - 12.0 * k).min(height - 12.0 * k);
                self.prize_tag(
                    frame,
                    tag,
                    [
                        x + (width - side) * 0.5,
                        y + (height - side) * 0.5,
                        side,
                        side,
                    ],
                    1.0,
                );
            }
            strip::Item::Banner(banner) => {
                crate::profile_card::banner::draw(&mut self.ui, rect, banner.id);
            }
            strip::Item::New(_) => {
                let _ = list.push(DrawCommand::Border {
                    rect,
                    radius: 8.0 * k * s,
                    width: 2.0 * s,
                    color: color::alpha(colour, 0.8),
                });
                text(
                    &mut self.ui,
                    TextFamily::Display,
                    format_args!("?"),
                    frame.rect(x, y + height * 0.5 - 22.0 * k, width, 44.0 * k),
                    36.0 * k * s,
                    colour,
                    FontWeight::Semibold,
                    TextAlign::Center,
                );
            }
        }
    }
}

/// A kind's last word, for a strip tile's short label ("profile banner" -> "banner").
fn short_kind(kind: &str) -> &str {
    kind.rsplit(' ').next().unwrap_or(kind)
}

/// Ease in and out.
pub(super) fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The light behind a prize at rest at `middle` (frame pixels), `lit` (0 to 1) of the way
/// up, `scale` against the strip's own: a glow of its colour breathing, and for a grand
/// one slow rays turning.
pub(super) fn prize_light(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    middle: [f32; 2],
    item: strip::Item,
    lit: f32,
    seconds: f32,
    scale: f32,
) {
    let s = frame.s;
    let colour = item.colour();
    let pulse = 0.5 + 0.5 * (seconds * 4.0).sin();
    glow(
        canvas,
        frame,
        middle,
        230.0 * scale,
        colour,
        (0.35 + 0.15 * pulse) * lit,
    );
    if !item.grand() {
        return;
    }
    const GRAND_RAYS: usize = 16;
    let [cx, cy] = frame.point(middle[0], middle[1]);
    for ray in 0..GRAND_RAYS {
        let angle = ray as f32 / GRAND_RAYS as f32 * std::f32::consts::TAU + seconds * 0.4;
        let (sin, cos) = angle.sin_cos();
        let (inner, outer) = (140.0 * scale * s, (300.0 + 80.0 * pulse) * scale * s);
        let _ = canvas.draw_list_mut().push(DrawCommand::Line {
            from: [cx + cos * inner, cy + sin * inner],
            to: [cx + cos * outer, cy + sin * outer],
            width: 4.0 * scale * s,
            color: color::alpha(colour, 0.4 * lit),
        });
    }
}

/// A strip's ends fading into the dark: from `left` to `right` (frame pixels), the band
/// from `top`, `height` tall.
pub(super) fn strip_ends(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [left, right]: [f32; 2],
    top: f32,
    height: f32,
) {
    let width = (right - left) * 0.28;
    let dark = color::alpha(color::SPACE, 0.93);
    let clear = color::alpha(color::SPACE, 0.0);
    for (x, start, end) in [(left, dark, clear), (right - width, clear, dark)] {
        let _ = canvas.draw_list_mut().push(DrawCommand::GradientRect {
            rect: frame.rect(x, top, width, height),
            radius: 0.0,
            gradient: sjk_ui::Gradient {
                start,
                end,
                vertical: false,
            },
        });
    }
}

/// A strip's marker at `x` from `top` to `bottom` (frame pixels): a line with a point
/// above and below, the points `scale` against the strip's own.
pub(super) fn marker(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    x: f32,
    [top, bottom]: [f32; 2],
    colour: Color,
    scale: f32,
) {
    let s = frame.s;
    let [mx, top] = frame.point(x, top);
    let [_, bottom] = frame.point(x, bottom);
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::Line {
        from: [mx, top],
        to: [mx, bottom],
        width: 3.0 * s,
        color: colour,
    });
    for (y, down) in [(top, 1.0_f32), (bottom, -1.0)] {
        for side in [-1.0_f32, 1.0] {
            let _ = list.push(DrawCommand::Line {
                from: [mx, y],
                to: [mx + side * 14.0 * scale * s, y - down * 16.0 * scale * s],
                width: 3.0 * s,
                color: colour,
            });
        }
    }
}

/// A soft round glow of `colour` at `centre` (frame pixels), `radius` across its half,
/// `strength` its alpha at the middle: discs stacked from the rim inwards.
pub(super) fn glow(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    radius: f32,
    colour: Color,
    strength: f32,
) {
    const LAYERS: usize = 8;
    for layer in 0..LAYERS {
        let r = radius * (1.0 - layer as f32 / LAYERS as f32);
        let _ = canvas.draw_list_mut().push(DrawCommand::RoundedRect {
            rect: frame.rect(x - r, y - r, r * 2.0, r * 2.0),
            radius: r * frame.s,
            color: color::alpha(colour, strength / LAYERS as f32),
        });
    }
}

/// Three kyber crystals standing together round `middle` (frame pixels), the tallest
/// `height` high in the middle and two smaller at its sides, at `alpha`.
pub(super) fn crystals(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    middle: [f32; 2],
    height: f32,
    alpha: f32,
) {
    let s = frame.s;
    let kyber = cards::shard::KYBER;
    let lit = Color::new(kyber.r, kyber.g, kyber.b, alpha);
    // The side crystals first, so the middle one stands before them.
    for (dx, scale, drop) in [(-0.27, 0.62, 0.2), (0.25, 0.7, 0.16), (0.0, 1.0, 0.0)] {
        let tall = height * scale;
        let wide = cards::shard::width(tall);
        let [x, y] = frame.point(
            middle[0] + dx * height - wide * 0.5,
            middle[1] - height * 0.5 + height * drop + (height - tall) * 0.5,
        );
        cards::shard::draw(canvas.draw_list_mut(), [x, y], tall * s, lit);
    }
}

/// What a card pack's tile holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Currency {
    Credits,
    Shards,
}

/// One tile of a card pack centred on `centre` (frame pixels), at `alpha`: the dark glass
/// with an edge in the currency's colour, its mark, `+amount` and what it is.
fn pack_tile(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    currency: Currency,
    amount: u64,
    alpha: f32,
) {
    let s = frame.s;
    let [width, height] = PACK_TILE;
    let [left, top] = [centre[0] - width * 0.5, centre[1] - height * 0.5];
    let (accent, bright, words, label) = match currency {
        Currency::Credits => (
            color::GOLD,
            color::GOLD_BRIGHT,
            cards::credits_text(amount),
            "credits",
        ),
        Currency::Shards => (
            cards::shard::KYBER,
            cards::shard::KYBER,
            crate::economy::shards_text(amount),
            if amount == 1 {
                "kyber shard"
            } else {
                "kyber shards"
            },
        ),
    };
    let fade = |colour: Color, share: f32| {
        Color::new(colour.r, colour.g, colour.b, colour.a * share * alpha)
    };
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::RoundedRect {
        rect: frame.rect(left + 4.0, top + 10.0, width, height),
        radius: 18.0 * s,
        color: fade(color::SPACE, 0.6),
    });
    let _ = list.push(DrawCommand::RoundedRect {
        rect: frame.rect(left, top, width, height),
        radius: 18.0 * s,
        color: fade(Color::new(0.04, 0.06, 0.12, 1.0), 0.95),
    });
    let _ = list.push(DrawCommand::Border {
        rect: frame.rect(left, top, width, height),
        radius: 18.0 * s,
        width: 1.5 * s,
        color: fade(accent, 0.6),
    });
    match currency {
        Currency::Credits => {
            credits_stack(canvas, frame, [centre[0] - 6.0, top + 54.0], 36.0, alpha);
        }
        Currency::Shards => {
            let mark = 64.0;
            let at = frame.point(centre[0] - cards::shard::width(mark) * 0.5, top + 30.0);
            let _ = cards::shard::draw(list, at, mark * s, fade(accent, 1.0));
        }
    }
    text(
        canvas,
        TextFamily::Display,
        format_args!("+{words}"),
        frame.rect(left, top + 104.0, width, 50.0),
        40.0 * s,
        fade(bright, 1.0),
        FontWeight::Semibold,
        TextAlign::Center,
    );
    text(
        canvas,
        TextFamily::Body,
        format_args!("{label}"),
        frame.rect(left, top + 158.0, width, 26.0),
        17.0 * s,
        fade(color::MUTED, 1.0),
        FontWeight::Regular,
        TextAlign::Center,
    );
}

/// The kyber shard's mark and `words` centred on `x` (frame pixels), as [`credits_line`]
/// lays out the credits, in the kyber cyan.
pub(crate) fn shards_line(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    x: f32,
    y: f32,
    words: &str,
    size: f32,
    alpha: f32,
) {
    let s = frame.s;
    let mark = size * 0.95;
    let gap = size * 0.3;
    let words_width = crate::text::display_width(words, size) * crate::text::style::current().scale;
    let total = cards::shard::width(mark) + gap + words_width;
    let left = x - total * 0.5;
    let kyber = cards::shard::KYBER;
    let [mx, my] = frame.point(left, y + (size * 1.25 - mark) * 0.5);
    cards::shard::draw(
        canvas.draw_list_mut(),
        [mx, my],
        mark * s,
        Color::new(kyber.r, kyber.g, kyber.b, alpha),
    );
    text(
        canvas,
        TextFamily::Display,
        format_args!("{words}"),
        frame.rect(
            left + cards::shard::width(mark) + gap,
            y,
            words_width + size,
            size * 1.25,
        ),
        size * s,
        Color::new(kyber.r, kyber.g, kyber.b, alpha),
        FontWeight::Semibold,
        TextAlign::Start,
    );
}

/// The credits' mark and `words` centred on `x` (frame pixels), the mark as tall as the
/// words' capitals, at `alpha`.
pub(crate) fn credits_line(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    x: f32,
    y: f32,
    words: &str,
    size: f32,
    alpha: f32,
) {
    let s = frame.s;
    let mark = size * 0.8;
    let gap = size * 0.28;
    let words_width = crate::text::display_width(words, size) * crate::text::style::current().scale;
    let total = cards::mark::width(mark) + gap + words_width;
    let left = x - total * 0.5;
    let gold = Color::new(color::GOLD.r, color::GOLD.g, color::GOLD.b, alpha);
    let [mx, my] = frame.point(left, y + (size * 1.25 - mark) * 0.5);
    cards::mark::draw(canvas.draw_list_mut(), [mx, my], mark * s, gold);
    text(
        canvas,
        TextFamily::Display,
        format_args!("{words}"),
        frame.rect(
            left + cards::mark::width(mark) + gap,
            y,
            words_width + size,
            size * 1.25,
        ),
        size * s,
        Color::new(
            color::GOLD_BRIGHT.r,
            color::GOLD_BRIGHT.g,
            color::GOLD_BRIGHT.b,
            alpha,
        ),
        FontWeight::Semibold,
        TextAlign::Start,
    );
}

/// Coins smaller than this (their half across, frame pixels) and cards narrower than
/// [`LIGHT_CARD`] are drawn light, in fewer shapes: the stacked strips' tiles are many.
const LIGHT_COIN: f32 = 30.0;
const LIGHT_CARD: f32 = 40.0;

/// A coin of the credits centred on `centre` (frame pixels), `radius` its half across, at
/// `alpha`: a deep gold disc and its gold rim, and on the one face up a fine ring inside
/// it and the credits' mark ([`cards::mark`]).
fn coin(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    radius: f32,
    alpha: f32,
    face_up: bool,
) {
    let s = frame.s;
    let fade = |colour: Color, share: f32| {
        Color::new(colour.r, colour.g, colour.b, colour.a * share * alpha)
    };
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::RoundedRect {
        rect: frame.rect(x - radius, y - radius, radius * 2.0, radius * 2.0),
        radius: radius * s,
        color: fade(Color::new(0.30, 0.21, 0.06, 1.0), 1.0),
    });
    // Rings as borders, not arcs: a tile half out of a strip clips them.
    let ring = |share: f32| {
        frame.rect(
            x - radius * share,
            y - radius * share,
            radius * share * 2.0,
            radius * share * 2.0,
        )
    };
    let _ = list.push(DrawCommand::Border {
        rect: ring(1.0),
        radius: radius * s,
        width: (radius * 0.12 * s).max(1.0),
        color: fade(color::GOLD, 1.0),
    });
    if !face_up {
        return;
    }
    if radius >= LIGHT_COIN {
        let _ = list.push(DrawCommand::Border {
            rect: ring(0.72),
            radius: radius * 0.72 * s,
            width: (radius * 0.04 * s).max(1.0),
            color: fade(color::GOLD_BRIGHT, 0.5),
        });
    }
    let mark = radius;
    let _ = cards::mark::draw(
        list,
        frame.point(x - cards::mark::width(mark) * 0.5, y - mark * 0.5),
        mark * s,
        fade(color::GOLD_BRIGHT, 1.0),
    );
}

/// A stack of credits centred on `centre` (frame pixels), its coins `radius` their half
/// across, at `alpha`: two coins showing behind and under the face up one in front (one
/// behind it for a small stack, as the stacked strips' light tiles draw it, to keep their
/// shapes few).
pub(super) fn credits_stack(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    [x, y]: [f32; 2],
    radius: f32,
    alpha: f32,
) {
    let behind: &[(f32, f32)] = if radius < LIGHT_COIN {
        &[(0.3, 0.26)]
    } else {
        &[(0.42, 0.34), (0.22, 0.17)]
    };
    for &(dx, dy) in behind {
        coin(
            canvas,
            frame,
            [x + dx * radius, y + dy * radius],
            radius,
            alpha * 0.85,
            false,
        );
    }
    coin(canvas, frame, [x, y], radius, alpha, true);
}

/// A card lying face down centred on `centre` (frame pixels), `width` wide, turned
/// `angle` radians clockwise and `squeeze` (0 to 1) of its width across as it flips over,
/// at `alpha`: its shadow and its back, a dark plate washed and edged in the colour of the
/// holocron it came out of (gold edged for Solar, a rainbow across it for Holo) with that
/// holocron's gem at its heart (a narrow one, [`LIGHT_CARD`], is its plate and edge
/// alone). `[tier, finish]` are positions in [`holocrons::TIERS`] and
/// [`cards::FINISHES`].
#[allow(clippy::too_many_arguments)]
pub(super) fn tucked_card(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    centre: [f32; 2],
    width: f32,
    angle: f32,
    squeeze: f32,
    [tier, finish]: [usize; 2],
    alpha: f32,
) {
    if alpha <= 0.0 || squeeze <= 0.01 {
        return;
    }
    let s = frame.s;
    let tier = holocrons::slot(tier);
    let edge = if finish == 2 {
        color::GOLD_BRIGHT
    } else {
        tier.colour
    };
    let height = width * cards::look::ASPECT;
    let [cx, cy] = frame.point(centre[0], centre[1]);
    let (sin, cos) = angle.sin_cos();
    let light = width < LIGHT_CARD;
    let faded = alpha < 1.0;
    let list = canvas.draw_list_mut();
    if faded {
        let _ = list.push(DrawCommand::PushOpacity(alpha.clamp(0.0, 1.0)));
    }
    // Turned about its middle, after narrowing across it for the flip.
    let _ = list.push(DrawCommand::PushTransform([
        cos * squeeze,
        -sin,
        cx - cos * squeeze * cx + sin * cy,
        sin * squeeze,
        cos,
        cy - sin * squeeze * cx - cos * cy,
        0.0,
        0.0,
        1.0,
    ]));
    let [left, top] = [centre[0] - width * 0.5, centre[1] - height * 0.5];
    let rect = frame.rect(left, top, width, height);
    let radius = width * 0.07 * s;
    let unit = width / 220.0;
    if light {
        // Its plate in the holocron's colour, deepened, and its edge.
        let deep = |c: f32| 0.035 + (c - 0.035) * 0.42;
        let colour = tier.colour;
        let _ = list.push(DrawCommand::RoundedRect {
            rect,
            radius,
            color: Color::new(deep(colour.r), deep(colour.g), deep(colour.b), 0.98),
        });
        if finish == 1 {
            for (band, hue) in cards::look::HOLO_BANDS.iter().step_by(2).enumerate() {
                let _ = list.push(DrawCommand::SolidRect {
                    rect: frame.rect(
                        left + width * 0.16,
                        top + height * (0.66 + 0.08 * band as f32),
                        width * 0.68,
                        height * 0.05,
                    ),
                    color: Color::new(hue[0], hue[1], hue[2], 0.6),
                });
            }
        }
        let _ = list.push(DrawCommand::Border {
            rect,
            radius,
            width: (1.5 * s).max(1.0),
            color: edge,
        });
        let _ = list.push(DrawCommand::PopTransform);
        if faded {
            let _ = list.push(DrawCommand::PopOpacity);
        }
        return;
    }
    let _ = list.push(DrawCommand::RoundedRect {
        rect: frame.rect(left + 4.0 * unit, top + 8.0 * unit, width, height),
        radius,
        color: Color::new(0.0, 0.0, 0.0, 0.45),
    });
    let _ = list.push(DrawCommand::RoundedRect {
        rect,
        radius,
        color: Color::new(0.035, 0.05, 0.1, 0.98),
    });
    let _ = list.push(DrawCommand::GradientRect {
        rect,
        radius,
        gradient: sjk_ui::Gradient {
            start: tier.colour_alpha(0.5),
            end: tier.colour_alpha(0.12),
            vertical: true,
        },
    });
    if finish == 1 {
        for (band, hue) in cards::look::HOLO_BANDS.iter().enumerate() {
            let _ = list.push(DrawCommand::SolidRect {
                rect: frame.rect(
                    left + width * 0.14,
                    top + height * (0.76 + 0.02 * band as f32),
                    width * 0.72,
                    height * 0.014,
                ),
                color: Color::new(hue[0], hue[1], hue[2], 0.45),
            });
        }
    }
    let _ = list.push(DrawCommand::Border {
        rect,
        radius,
        width: (5.0 * unit * s).max(1.2),
        color: edge,
    });
    let _ = list.push(DrawCommand::Border {
        rect: frame.rect(
            left + 14.0 * unit,
            top + 14.0 * unit,
            width - 28.0 * unit,
            height - 28.0 * unit,
        ),
        radius: (radius - 9.0 * unit * s).max(1.0),
        width: (2.0 * unit * s).max(1.0),
        color: Color::new(1.0, 1.0, 1.0, 0.16),
    });
    gem::draw(
        list,
        [cx, cy - height * 0.03 * s],
        width * 0.24 * s,
        tier.colour,
        1.0,
        if width >= 200.0 { gem::ROWS } else { 6 },
    );
    let _ = list.push(DrawCommand::PopTransform);
    if faded {
        let _ = list.push(DrawCommand::PopOpacity);
    }
}

/// The light a card flips over into, `t` (0 to 1) of the way through: a glow of its
/// colour, a ring running out from round the card at `middle` (`[width, height]` its
/// size, frame pixels) and rays turning, gold for Solar and the rainbow for Holo, else
/// `tone` (its rank's or rarity's colour); only for a Holo, Solar, Legendary or Mythical
/// card.
fn flourish(
    canvas: &mut crate::menu_widgets::MenuCanvas,
    frame: &Frame,
    middle: [f32; 2],
    [width, height]: [f32; 2],
    shown: &Shown,
    tone: Color,
    t: f32,
) {
    let (finish, rarity) = (shown.finish.index, shown.rarity.index);
    if (finish == 0 && rarity < 3) || !(0.0..1.0).contains(&t) {
        return;
    }
    let s = frame.s;
    let colour = if finish == 2 {
        color::GOLD_BRIGHT
    } else {
        tone
    };
    let fade = 1.0 - t;
    glow(
        canvas,
        frame,
        middle,
        height * (0.6 + 0.3 * t),
        colour,
        0.5 * fade,
    );
    let [cx, cy] = frame.point(middle[0], middle[1]);
    let list = canvas.draw_list_mut();
    let _ = list.push(DrawCommand::Arc {
        center: [cx, cy],
        radius: (height * 0.5 + 260.0 * t) * s,
        width: (6.0 * fade).max(1.0) * s,
        start: 0.0,
        sweep: std::f32::consts::TAU,
        color: color::alpha(colour, 0.85 * fade),
        knockout: None,
    });
    const FLOURISH_RAYS: usize = 18;
    let bands = &cards::look::HOLO_BANDS;
    for ray in 0..FLOURISH_RAYS {
        let angle = ray as f32 / FLOURISH_RAYS as f32 * std::f32::consts::TAU + t * 0.9;
        let (sin, cos) = angle.sin_cos();
        let hue = if finish == 1 {
            let [r, g, b] = bands[ray % bands.len()];
            Color::new(r, g, b, 1.0)
        } else {
            colour
        };
        let inner = (width * 0.45 + 40.0 * t) * s;
        let outer = (width * 0.62 + 240.0 * smooth(t * 1.6)) * s;
        let _ = list.push(DrawCommand::Line {
            from: [cx + cos * inner, cy + sin * inner],
            to: [cx + cos * outer, cy + sin * outer],
            width: 4.0 * s,
            color: color::alpha(hue, 0.6 * fade),
        });
    }
}

#[cfg(test)]
mod bonus_tests {
    use super::*;
    fn prizes(opened: &sjk_identity::Opened) -> Vec<Prize> {
        Prize::of(opened)
            .into_iter()
            .chain(Prize::bonus(opened))
            .collect()
    }

    #[test]
    fn twenty_openings_and_twenty_bonuses_reveal_in_complete_bounded_groups() {
        let now = Instant::now();
        let mut panel = Panel::new();
        panel.ceremony = Some(Ceremony::Waiting {
            tier: 0,
            since: now,
        });
        let opened = (1..=sjk_identity::OPEN_MAX)
            .map(|id| {
                serde_json::from_value(serde_json::json!({
                    "holocron":id,"tier":"rare","credits":42,
                    "card":{"id":id,"rarity":"rare","finish":"normal","edition":"alpha"},
                    "banner":{"id":"banner_jedi_archives","name":"Jedi Archives","count":id}
                }))
                .unwrap()
            })
            .collect();
        panel.follow_opening(
            now,
            Some(&sjk_identity::OpenOutcome {
                serial: 1,
                opened,
                message: String::new(),
            }),
        );
        let Some(Ceremony::Batch { shown, .. }) = &panel.ceremony else {
            panic!("ordinary group")
        };
        assert_eq!(shown.len(), sjk_identity::OPEN_MAX);
        assert!(shown.iter().all(|prize| matches!(prize, Prize::Card(_))));
        assert_eq!(panel.bonus_prizes.len(), sjk_identity::OPEN_MAX);
        assert_eq!(panel.done_label(), "Bonus banners");
        panel.shot_elapsed = Some(100.0);
        for viewport in [[1920.0, 1080.0], [1024.0, 768.0]] {
            panel.build(viewport);
            assert!(!panel.overflowed(), "ordinary rewards at {viewport:?}");
        }
        let mut advance_at = now;
        for _ in 0..20 {
            panel.advance(advance_at);
            advance_at += Duration::from_millis(1);
            if panel.bonus_prizes.is_empty() {
                break;
            }
        }
        assert!(panel.bonus_prizes.is_empty(), "the bonus group starts");
        let Some(Ceremony::Batch { shown, .. }) = &panel.ceremony else {
            panic!("bonus group")
        };
        assert_eq!(shown.len(), sjk_identity::OPEN_MAX);
        assert!(
            shown
                .iter()
                .all(|prize| matches!(prize.item(), strip::Item::Banner(_)))
        );
        assert_eq!(panel.done_label(), "Done");
        for viewport in [[1920.0, 1080.0], [1024.0, 768.0]] {
            panel.build(viewport);
            assert!(!panel.overflowed(), "bonus covers at {viewport:?}");
        }
        panel.end_ceremony();
        assert!(panel.bonus_prizes.is_empty());
        assert!(panel.ceremony.is_none());
    }

    #[test]
    fn the_bonus_reveal_keeps_the_card_credits_shards_and_solar_unlock() {
        let opened: sjk_identity::Opened = serde_json::from_value(serde_json::json!({
            "holocron":1,"tier":"rare","credits":42,"shards":4,
            "card":{"id":1,"rarity":"rare","finish":"solar","edition":"alpha"},
            "unlock":{"id":"saber_sun","name":"Sun","count":1},
            "banner":{"id":"banner_jedi_archives","name":"Jedi Archives","count":2}
        }))
        .unwrap();
        let prizes = prizes(&opened);
        assert_eq!(prizes.len(), 2);
        let Prize::Card(card) = &prizes[0] else {
            panic!("ordinary card first")
        };
        assert_eq!(card.credits, 42);
        assert_eq!(card.shards, 4);
        assert_eq!(card.bonus.as_ref().unwrap().id, "saber_sun");
        let Prize::Shader(banner) = &prizes[1] else {
            panic!("banner bonus second")
        };
        assert_eq!(banner.id, "banner_jedi_archives");
        assert_eq!(banner.count, 2);
        assert_eq!(banner.kind(), "bonus profile banner");
    }

    #[test]
    fn a_beta_holocron_reveals_each_prize_its_best_last() {
        let opened: sjk_identity::Opened = serde_json::from_value(serde_json::json!({
            "holocron":3,"tier":"mythical","credits":300,
            "card":{"id":9,"rarity":"legendary","finish":"holo","edition":"beta"},
            "extra":[
                {"kind":"shader","unlock":{"id":"saber_tidal","name":"Tidal blade","count":1}},
                {"kind":"shards","shards":40},
                {"kind":"card","card":{"id":10,"rarity":"rare","finish":"normal","edition":"beta"}}
            ]
        }))
        .unwrap();
        let prizes = Prize::all(&opened);
        assert_eq!(prizes.len(), 4);
        assert!(matches!(&prizes[0], Prize::Shards(pile) if pile.shards == 40));
        let Some(Prize::Card(best)) = prizes.last() else {
            panic!("the Legendary card last")
        };
        assert_eq!((best.rarity.id, best.credits), ("legendary", 300));
        assert!(prizes.iter().any(|prize| matches!(prize,
            Prize::Card(card) if card.rarity.id == "rare" && card.credits == 0)));
        // An Alpha holocron: its one prize, as before.
        let mut alpha = opened.clone();
        alpha.extra.clear();
        assert_eq!(Prize::all(&alpha), vec![Prize::of(&alpha).unwrap()]);
    }

    #[test]
    fn old_hub_and_non_card_prizes_still_reveal_with_optional_bonus() {
        for ordinary in [
            serde_json::json!({"shards":12}),
            serde_json::json!({"unlock":{"id":"saber_sun","name":"Sun","count":1}}),
        ] {
            let mut json = serde_json::json!({"holocron":1,"tier":"rare"});
            json.as_object_mut()
                .unwrap()
                .extend(ordinary.as_object().unwrap().clone());
            let old: sjk_identity::Opened = serde_json::from_value(json.clone()).unwrap();
            assert!(old.banner.is_none());
            assert_eq!(prizes(&old).len(), 1);
            json["banner"] =
                serde_json::json!({"id":"banner_hyperspace","name":"Hyperspace","count":1});
            let new: sjk_identity::Opened = serde_json::from_value(json).unwrap();
            let prizes = prizes(&new);
            assert_eq!(prizes.len(), 2);
            assert_eq!(prizes[0], Prize::of(&old).unwrap());
        }
    }
}
