//! The Collection's Forge tab (`docs/sjk-ui.md`, Collection; `docs/cards.md`, "The
//! Forge"; Sol's request of 21/10/2026: "a better interface for the shop, for crafting
//! sabers and for disassembling them", a tab of its own). Everything the holocron economy
//! trades, in three sections switched by the chips under the headline:
//!
//! - **Shop:** the player's four offers of the day as large cards with their live blades,
//!   price and state, the countdown to midnight in Paris, and the holocrons credits buy.
//! - **Craft:** every saber shader of the trade the player does not hold, as a grid or a
//!   list (V, `ui_shaderList` as on Shaders), with the tier chips, an Affordable chip and a
//!   search; the one chosen big on the right with its blade, its parts, its cost against
//!   the player's kyber shards, and Craft.
//! - **Break down:** the player's own copies of shaders of the trade, the duplicates first
//!   with their count; the one chosen big on the right with what breaking a copy gives, and
//!   Break down a copy; Break down all duplicates keeps one of each. The last copy breaks
//!   down too on a hub that says so (`shatter_last`, 21/10/2026: Sol's promise of breaking
//!   down any saber shader a player owns, never a card's or a medal's), after a stronger
//!   warning; against an older hub only duplicates are listed.
//!
//! Every purchase, craft and break down asks twice, as the shop always did: the first
//! press arms the button ("Confirm: ..."), a second within five seconds sends it. The
//! last copy and all duplicates ask with a card over the page instead. The balances stand
//! at the top right, with how each is earned in their hover cards.
//!
//! Keys: Tab and Shift+Tab (or 1, 2, 3) change section; the arrows choose; Enter or Space
//! presses the chosen thing's button; V switches the grid and the list, `/` (or Ctrl+F)
//! types in the search, T steps the tier, A shows the affordable only (Craft), D the
//! duplicates only and B breaks down all duplicates (Break down); Ctrl+Tab changes tab
//! and Escape leaves, as on every tab. Since 29/10/2026 (collection marks,
//! `crate::shader_marks`) F stars the chosen shader and L locks it, on a hub that keeps
//! them, and Shift+F shows Break down's favourites alone: favourites are listed first, and
//! a locked shader is never broken down (its button says so) nor counted among all
//! duplicates.
//!
//! Since 10/11/2026 (Sol: "make crafting and the shop more relevant, people barely use
//! it") a fourth section, **Fuse**, shows on a hub whose economy says `fuse`: a tier's
//! chip, its duplicates as cards (the most duplicated first), the copies the Forge
//! proposes picked on them (never a last copy, [`crate::forge_fuse::propose`]), and on the
//! right what the fusion gives and Fuse (pressed twice); a click or Space picks a copy or
//! lets it go, + and - change the chosen one's, P proposes again, T steps the tier and 4
//! shows the section. The shader gained is revealed on the card. The Shop marks the deal
//! of the day (its old price struck), a wished offer and a Beta offer's edition, lists
//! the wishlist (on a hub that keeps one, [`crate::wishlist`]), and each offer and Craft's
//! chosen shader carry a wishlist toggle (W on Craft). Shop and Fuse carry a gold dot when
//! something waits there ([`crate::forge_alert`]).
//!
//! Since 05/11/2026 (Atrosex's idea, hubs whose economy says `shatter_many`) Break down
//! has a Select chip (S): while it is on, a click on a shader (or Space on the chosen one)
//! picks a copy of it, + and - (or the stepper) pick more or fewer of the chosen one's
//! copies, and the button breaks every picked copy down at once after a card asks;
//! Escape stops picking. A locked shader is never picked, a last copy only on a hub that
//! breaks last copies down.

use super::*;
use crate::audio::ui_cues::{self, Cue};
use crate::unlockables::{Rarity, Unlockable};
use std::time::Duration;

/// The Forge's tokens on the Collection page's canvas, clear of the page's others (to
/// 8 007) and of the row of tabs (2 000 on).
pub(super) const SECTION_BASE: u16 = 9_000;
pub(crate) const SHARDS_TOKEN: u16 = 9_005;
pub(super) const CREDITS_TOKEN: u16 = 9_006;
pub(super) const OFFER_BASE: u16 = 9_010;
pub(super) const OFFER_BUY_BASE: u16 = 9_020;
pub(super) const HOLOCRON_BASE: u16 = 9_030;
pub(super) const HOLOCRON_BUY_BASE: u16 = 9_035;
pub(super) const TIER_BASE: u16 = 9_040;
pub(super) const ONLY_TOKEN: u16 = 9_046;
pub(super) const SEARCH_TOKEN: u16 = 9_047;
pub(super) const LIST_TOKEN: u16 = 9_048;
pub(super) const GRID_TOKEN: u16 = 9_049;
pub(super) const ACTION_TOKEN: u16 = 9_050;
pub(super) const ALL_TOKEN: u16 = 9_051;
pub(super) const ASK_YES_TOKEN: u16 = 9_052;
pub(super) const ASK_NO_TOKEN: u16 = 9_053;
/// The chosen shader's star and padlock, and Break down's Favourites chip (29/10/2026).
pub(super) const STAR_TOKEN: u16 = 9_054;
pub(super) const PADLOCK_TOKEN: u16 = 9_055;
pub(super) const FAVOURITES_TOKEN: u16 = 9_056;
/// Break down's Select chip and the picked copies' stepper (05/11/2026).
pub(super) const SELECT_TOKEN: u16 = 9_057;
pub(super) const PICK_LESS_TOKEN: u16 = 9_058;
pub(super) const PICK_MORE_TOKEN: u16 = 9_059;
/// Fuse's tier chips (10/11/2026), one a tier of [`crate::forge_fuse::TIERS`].
pub(super) const FUSE_TIER_BASE: u16 = 9_060;
/// Each offer's wishlist toggle, and Craft's chosen shader's (10/11/2026).
pub(super) const OFFER_WISH_BASE: u16 = 9_065;
pub(super) const WISH_TOKEN: u16 = 9_070;
/// Fuse's Propose again.
pub(super) const PROPOSE_TOKEN: u16 = 9_071;
pub(super) const TILE_BASE: u16 = 9_100;
/// The most things a section lists (every blade skin fits).
pub(super) const TILES_MAX: usize = 400;
const _: () = assert!(TILE_BASE as usize + TILES_MAX <= u16::MAX as usize);
const _: () = assert!(unlockables::ALL.len() <= TILES_MAX);

/// Whether `token` is one of the Forge's.
pub(super) fn forge_token(token: u16) -> bool {
    (SECTION_BASE..TILE_BASE + TILES_MAX as u16).contains(&token)
}

/// The most shaders picked at once (the hub takes up to 256 in one request).
pub(super) const PICKS_MAX: usize = 256;

/// How long an armed button waits for its second press.
pub(super) const CONFIRM: Duration = Duration::from_secs(5);
/// The grid's columns and the lines it shows; the list's rows shown.
pub(super) const COLUMNS: usize = 5;
pub(super) const LINES: usize = 3;
pub(super) const ROWS: usize = 6;

/// The Forge's sections, in the chips' order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Section {
    /// The day's offers and the holocrons for credits.
    #[default]
    Shop,
    /// Shaders made with kyber shards.
    Craft,
    /// Shaders turned into kyber shards.
    Break,
    /// Duplicates of one tier fused into a shader of the next (10/11/2026, a hub whose
    /// economy says `fuse`).
    Fuse,
}

impl Section {
    pub(crate) const ALL: [Self; 4] = [Self::Shop, Self::Craft, Self::Break, Self::Fuse];

    /// The sections shown: Fuse only on a hub that fuses (`fuse`).
    pub(crate) fn shown(fuse: bool) -> &'static [Self] {
        if fuse { &Self::ALL } else { &Self::ALL[..3] }
    }

    /// The chip's name.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Shop => "Shop",
            Self::Craft => "Craft",
            Self::Break => "Break down",
            Self::Fuse => "Fuse",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Shop => 0,
            Self::Craft => 1,
            Self::Break => 2,
            Self::Fuse => 3,
        }
    }

    /// The section after this one (`forward`) or before it among the three first, wrapping.
    pub(crate) fn next(self, forward: bool) -> Self {
        self.next_in(forward, false)
    }

    /// The section after this one (`forward`) or before it among those shown (`fuse`: Fuse
    /// too), wrapping.
    pub(crate) fn next_in(self, forward: bool, fuse: bool) -> Self {
        let shown = Self::shown(fuse);
        let count = shown.len();
        let at = self.index().min(count - 1);
        let index = if forward {
            (at + 1) % count
        } else {
            (at + count - 1) % count
        };
        shown[index]
    }
}

/// Why the Forge cannot show what the player holds, or `None` when it can.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) enum Closed {
    /// It shows: the hub sent the player's profile and its economy.
    #[default]
    Open,
    /// `cl_identity` is off.
    IdentityOff,
    /// No hub is set, or it has not answered yet.
    Waiting,
    /// The hub has no economy (a hub before 17/10/2026).
    NoEconomy,
}

/// What the Forge knows of the identity and the hub this frame ([`ForgeData::of`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ForgeData {
    pub(crate) closed: Closed,
    /// The player's credits and kyber shards, once known.
    pub(crate) credits: Option<u64>,
    pub(crate) shards: Option<u64>,
    /// Whether the hub lets the player trade now (`economy::may_act`).
    pub(crate) can_trade: bool,
    /// Why not, in a line, while the hub said so (opening for staff, an old build).
    pub(crate) why_not: Option<String>,
    /// The hub's trade abilities: the last copy breaks down, every duplicate at once,
    /// the copies picked of several shaders at once (05/11/2026).
    pub(crate) last: bool,
    pub(crate) all: bool,
    pub(crate) many: bool,
    /// The shop's day as the hub last answered it, once asked.
    pub(crate) shop: Option<sjk_identity::ShopState>,
    /// What became of the last trade.
    pub(crate) trade: Option<sjk_identity::TradeOutcome>,
    /// The duplicates a fusion of each tier takes, Common to Mythical, 0 for none
    /// (10/11/2026; all 0 on a hub that does not say `fuse`).
    pub(crate) fuse: [u32; crate::economy::TIERS],
    /// The player's wishlist (10/11/2026); `None` on a hub that keeps none.
    pub(crate) wishlist: Option<Vec<String>>,
}

impl ForgeData {
    /// Whether the hub fuses some tier: the Fuse section shows.
    pub(crate) fn fuses(&self) -> bool {
        self.fuse.iter().any(|need| *need > 0)
    }

    /// The duplicates a fusion of `tier` takes; `None` when the hub does not fuse it.
    pub(crate) fn fuse_need(&self, tier: Rarity) -> Option<u32> {
        let need = self.fuse.get(tier as usize).copied().unwrap_or(0);
        (need > 0 && crate::forge_fuse::next_tier(tier).is_some()).then_some(need)
    }

    /// Whether shader `id` is on the wishlist.
    pub(crate) fn wished(&self, id: &str) -> bool {
        self.wishlist
            .as_ref()
            .is_some_and(|list| list.iter().any(|wished| wished == id))
    }

    /// Whether the wishlist is full.
    pub(crate) fn wishlist_full(&self) -> bool {
        self.wishlist
            .as_ref()
            .is_some_and(|list| list.len() >= crate::wishlist::MAX)
    }
}

impl ForgeData {
    /// From `cl_identity` (`enabled`) and the identity service's snapshot.
    pub(crate) fn of(enabled: bool, snapshot: &sjk_identity::Snapshot) -> Self {
        if !enabled {
            return Self {
                closed: Closed::IdentityOff,
                ..Self::default()
            };
        }
        let Some(me) = snapshot.me.as_ref() else {
            return Self {
                closed: Closed::Waiting,
                ..Self::default()
            };
        };
        let progress = snapshot.holocrons.as_ref();
        let economy = snapshot.economy.as_ref();
        let closed = match (progress, economy) {
            (None, _) => Closed::Waiting,
            (Some(_), None) => Closed::NoEconomy,
            (Some(_), Some(_)) => Closed::Open,
        };
        let can_trade = crate::economy::may_act(snapshot);
        let why_not = if can_trade || closed != Closed::Open {
            None
        } else if let Some(min) = crate::economy::update_to(snapshot) {
            Some(format!(
                "Update SJK to trade: the Forge needs SJK {min} or newer."
            ))
        } else {
            Some(crate::terms::say("The Forge opens with holocrons: soon.").into_owned())
        };
        Self {
            closed,
            credits: Some(me.credits),
            shards: Some(me.shards),
            can_trade,
            why_not,
            last: economy.is_some_and(|tables| tables.can("shatter_last")),
            all: economy.is_some_and(|tables| tables.can("shatter_duplicates")),
            many: economy.is_some_and(|tables| tables.can("shatter_many")),
            shop: snapshot.shop.clone(),
            trade: snapshot.trade.clone(),
            fuse: std::array::from_fn(|tier| {
                economy
                    .and_then(|tables| tables.fuse_count(tier))
                    .unwrap_or(0)
            }),
            wishlist: economy.and_then(|tables| tables.wishlist.clone()),
        }
    }

    /// The live identity's: `enabled` is `cl_identity`.
    pub(crate) fn read(enabled: bool) -> Self {
        crate::player_identity::with_snapshot(|snapshot| Self::of(enabled, snapshot)).unwrap_or(
            Self {
                closed: Closed::IdentityOff,
                ..Self::default()
            },
        )
    }
}

/// What a section's list shows: a tier, a section's own chip (Affordable on Craft,
/// Duplicates on Break down) and the words searched.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ForgeFilter {
    pub(super) shaders: ShaderFilter,
    pub(super) only: bool,
}

/// A shader the Craft section lists: not held, of the trade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Craftable {
    pub(super) skin: &'static Unlockable,
    /// What crafting it costs, lowered by a copy of its blade card the player holds.
    pub(super) cost: u64,
    /// That copy's finish and how much it took off, in per cent (`None`: the full cost).
    pub(super) off: Option<(&'static crate::cards::Finish, u64)>,
    /// A favourite of the player's (a wish: they lack it), listed first.
    pub(super) favourite: bool,
}

/// A shader the Break down section lists: the player's own copies of one of the trade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Owned {
    pub(super) skin: &'static Unlockable,
    /// Copies held by grant, drops, crafts and purchases (a medal's or a card's is not).
    pub(super) count: u32,
    /// The kyber shards breaking one copy down gives.
    pub(super) gives: u64,
    /// Its collection marks (29/10/2026): a favourite (listed first), locked (never
    /// broken down until unlocked), New, worn on either saber.
    pub(super) favourite: bool,
    pub(super) locked: bool,
    pub(super) new: bool,
    pub(super) worn: bool,
}

impl Owned {
    /// Whether it is the player's only copy.
    pub(super) fn last(self) -> bool {
        self.count <= 1
    }
}

/// Every shader the Craft section may list for `holdings`, before the filter: of the trade
/// and not held at all (a card's or a medal's copy counts as held), the commonest tier
/// first, so what the player's shards reach comes first (costs rise with the tier), the
/// favourites among `marks` before the rest. A copy among `cards` (the player's) of a
/// shader's blade card crafts it for less ([`crate::economy::craft_price`], 21/10/2026).
pub(super) fn craftable(
    holdings: &Holdings<'_>,
    cards: &[sjk_identity::CardStack],
    marks: &crate::shader_marks::Marks,
) -> Vec<Craftable> {
    let blade_cards = if cards.is_empty() {
        Vec::new()
    } else {
        crate::economy::blade_cards(cards, &crate::cards::faces::current())
    };
    let mut list: Vec<Craftable> = unlockables::blade_skins_by_tier()
        .iter()
        .rev()
        .copied()
        .filter(|skin| crate::economy::in_trade(skin) && holdings.unlock(skin.id).is_none())
        .map(|skin| {
            let card = blade_cards
                .iter()
                .find(|(id, _)| *id == skin.id)
                .map(|(_, finish)| *finish);
            let (cost, off) = crate::economy::craft_price(skin, card);
            Craftable {
                skin,
                cost,
                off,
                favourite: marks.favourite(skin.id),
            }
        })
        .collect();
    crate::shader_marks::favourites_first(&mut list, marks, |item| item.skin.id);
    list
}

/// The shaders of the trade the player holds that `filter`'s search names (hub report
/// #305). Craft never lists them, since crafting refuses a held shader (Sol, 17/10/2026),
/// so a search for one says it is held instead of finding nothing. Empty without a search.
pub(super) fn held_matches(
    holdings: &Holdings<'_>,
    filter: &ShaderFilter,
    marks: &crate::shader_marks::Marks,
) -> Vec<&'static Unlockable> {
    if filter.query.trim().is_empty() {
        return Vec::new();
    }
    unlockables::blade_skins_by_tier()
        .iter()
        .rev()
        .copied()
        .filter(|skin| crate::economy::in_trade(skin) && holdings.unlock(skin.id).is_some())
        .filter(|skin| filter.passes(skin, true, marks.favourite(skin.id)))
        .collect()
}

/// The Craft intro while a search also names held shaders (`held`, not empty) that the
/// list leaves out: the first two by name, then how many more.
pub(super) fn held_note(held: &[&'static Unlockable]) -> String {
    let names: Vec<&str> = held.iter().take(2).map(|skin| skin.name).collect();
    let more = held.len().saturating_sub(names.len());
    let mut line = format!("Yours already, so not listed: {}", names.join(", "));
    if more > 0 {
        line.push_str(&format!(" and {more} more"));
    }
    line.push('.');
    line
}

impl Panel {
    /// The player's card copies in `inputs`, while the identity is on (empty before the
    /// hub sent their profile; a world shot's made-up ones in tests).
    pub(super) fn held_cards<'s>(
        &'s self,
        inputs: &'s Inputs<'_>,
    ) -> &'s [sjk_identity::CardStack] {
        #[cfg(test)]
        if let Some((list, _, _)) = &self.preview_cards {
            return list;
        }
        inputs
            .snapshot
            .filter(|_| inputs.enabled)
            .and_then(|snapshot| snapshot.me.as_ref())
            .map_or(&[], |me| me.cards.as_slice())
    }
}

/// Every shader the Break down section may list for `holdings`, the favourites among
/// `marks` first, then in each part the duplicates first (each group the rarest first):
/// the player's own copies of shaders of the trade, the last ones only when the hub breaks
/// them down (`last`). A locked one is listed, with its padlock, so it can be unlocked.
pub(super) fn owned(
    holdings: &Holdings<'_>,
    last: bool,
    marks: &crate::shader_marks::Marks,
) -> Vec<Owned> {
    let mut list: Vec<Owned> = unlockables::blade_skins_by_tier()
        .iter()
        .copied()
        .filter(|skin| crate::economy::in_trade(skin))
        .filter_map(|skin| {
            let unlock = holdings
                .unlock(skin.id)
                .filter(|unlock| unlock.medal.is_none() && unlock.card.is_none())?;
            (unlock.count > 1 || (last && unlock.count == 1)).then(|| Owned {
                skin,
                count: unlock.count,
                gives: crate::economy::shatter_yield(skin.tier),
                favourite: marks.favourite(skin.id),
                locked: marks.locked(skin.id),
                new: marks.is_new(skin.id),
                worn: marks.worn(skin.id),
            })
        })
        .collect();
    list.sort_by_key(|owned| owned.last());
    crate::shader_marks::favourites_first(&mut list, marks, |item| item.skin.id);
    list
}

/// The duplicates `list` holds that may go: how many copies would go (one of each kept),
/// of how many shaders, for how many kyber shards. A locked shader's never go.
pub(super) fn duplicates(list: &[Owned]) -> (u32, usize, u64) {
    list.iter()
        .filter(|owned| owned.count > 1 && !owned.locked)
        .fold((0, 0, 0), |(copies, shaders, shards), owned| {
            let spare = owned.count - 1;
            (
                copies + spare,
                shaders + 1,
                shards + u64::from(spare) * owned.gives,
            )
        })
}

/// What a button pressed once waits to send.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Press {
    /// Offer `n` of the day.
    Offer(usize),
    /// Holocron `n` of [`crate::economy::holocrons_for_sale`].
    Holocron(usize),
    /// Craft this shader.
    Craft(&'static str),
    /// Break a copy of this shader down.
    Break(&'static str),
    /// Fuse the picked duplicates (10/11/2026).
    Fuse,
}

/// The card over the page that asks before a trade that cannot be pressed twice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Ask {
    /// Break down the player's only copy of this shader; `yes` has the focus.
    Last {
        skin: &'static Unlockable,
        yes: bool,
    },
    /// Break down every duplicate; `yes` has the focus.
    All { yes: bool },
    /// Break down a copy of each picked shader (05/11/2026); `yes` has the focus.
    Picked { yes: bool },
}

impl Ask {
    fn yes(self) -> bool {
        match self {
            Self::Last { yes, .. } | Self::All { yes } | Self::Picked { yes } => yes,
        }
    }

    fn with_focus(self, yes: bool) -> Self {
        match self {
            Self::Last { skin, .. } => Self::Last { skin, yes },
            Self::All { .. } => Self::All { yes },
            Self::Picked { .. } => Self::Picked { yes },
        }
    }
}

/// What can an offer's button do for the player now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Can {
    /// Buy it (it asks to confirm first).
    Buy,
    /// Bought today already.
    Bought,
    /// Held already: the hub sells no duplicate.
    Owned,
    /// Less than its price.
    Short(u64),
}

/// What `offer` allows a player holding `purse` of the shop's currency.
pub(super) fn can(offer: &sjk_identity::Offer, purse: u64) -> Can {
    if offer.bought {
        Can::Bought
    } else if offer.owned {
        Can::Owned
    } else if purse < offer.price {
        Can::Short(offer.price - purse)
    } else {
        Can::Buy
    }
}

/// The shader tiers by the hub's ids, Common to Mythical.
const TIER_IDS: [(&str, Rarity); 5] = [
    ("common", Rarity::Common),
    ("uncommon", Rarity::Uncommon),
    ("rare", Rarity::Rare),
    ("legendary", Rarity::Legendary),
    ("mythical", Rarity::Mythical),
];

/// The tier of an offer: the catalogue's for a shader this client knows, else the hub's
/// word, else Common.
pub(super) fn tier_of(offer: &sjk_identity::Offer) -> Rarity {
    unlockables::find(&offer.id).map_or_else(
        || {
            TIER_IDS
                .iter()
                .find(|(id, _)| *id == offer.tier)
                .map_or(Rarity::Common, |(_, tier)| *tier)
        },
        |skin| skin.tier,
    )
}

/// "17/10/2026" from the hub's "2026-10-17"; the hub's words when they are not a date.
pub(super) fn day_text(day: &str) -> String {
    let mut parts = day.split('-');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day), None)
            if year.len() == 4 && month.len() == 2 && day.len() == 2 =>
        {
            format!("{day}/{month}/{year}")
        }
        _ => day.to_owned(),
    }
}

/// How long until the next offers, as players read it: "5 h 12 min", "12 min", "less
/// than a minute".
pub(super) fn countdown_text(seconds: u64) -> String {
    let minutes = seconds.div_ceil(60);
    match (minutes / 60, minutes % 60) {
        (0, 0) | (0, 1) if seconds < 60 => "less than a minute".to_owned(),
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

/// `amount` of the shop's currency in words: "75 kyber shards", "250 credits".
pub(super) fn priced(amount: u64) -> String {
    if crate::economy::shop_in_shards() {
        format!("{} kyber shards", crate::economy::shards_text(amount))
    } else {
        format!("{} credits", crate::cards::credits_text(amount))
    }
}

/// The holocron tier called `id` (this client's list), if it knows it.
pub(super) fn holocron_tier(id: &str) -> Option<&'static crate::holocrons::Tier> {
    crate::holocrons::TIERS.iter().find(|tier| tier.id == id)
}

/// The Forge's state on the Collection page.
#[derive(Debug, Default)]
pub(super) struct Forge {
    pub(super) section: Section,
    pub(super) data: ForgeData,
    /// The shop's choice: an offer, then the holocrons for credits.
    pub(super) shop_selected: usize,
    /// The thing chosen on Craft and on Break down (in the list the filter shows), and the
    /// first line (grid) or row (list) each shows.
    pub(super) craft: usize,
    pub(super) craft_first: usize,
    pub(super) breaks: usize,
    pub(super) break_first: usize,
    /// What the last frame listed, for keys and clicks.
    pub(super) craft_list: Vec<Craftable>,
    pub(super) break_list: Vec<Owned>,
    /// Every shader Break down may list, past the filter (what the picks are read from).
    pub(super) break_all: Vec<Owned>,
    /// Break down's Select is on (05/11/2026): a click picks instead of only choosing.
    pub(super) selecting: bool,
    /// The shaders picked on Break down, in the order picked, with the copies picked of
    /// each.
    pub(super) picked: Vec<(&'static str, u32)>,
    /// Every duplicate the player holds, past the filter: copies, shaders, shards.
    pub(super) spare: (u32, usize, u64),
    pub(super) craft_filter: ForgeFilter,
    pub(super) break_filter: ForgeFilter,
    /// The search field of the section on show takes the typing.
    pub(super) search_focus: bool,
    /// A button pressed once, waiting for its second press, and since when.
    pub(super) armed: Option<(Press, Instant)>,
    /// The last trade sent from the Forge: the outcome's serial before it, and what.
    pub(super) sent: Option<(u64, Press)>,
    /// All duplicates sent (their outcome has no shader).
    pub(super) sent_all: Option<u64>,
    /// The picked copies sent: the outcome's serial before, and the last copies among
    /// them (for the console to take off the saber once gone).
    pub(super) sent_many: Option<(u64, Vec<&'static str>)>,
    /// The card asking before the last copy or all duplicates.
    pub(super) ask: Option<Ask>,
    /// The shop was asked for since the Shop section showed, and the day read again.
    pub(super) shop_asked: bool,
    pub(super) shop_reread: Option<u64>,
    /// Last copies just broken down, for the console to take off the saber if worn.
    pub(super) unequip: Vec<&'static str>,
    /// A shader to choose once the section lists it (the Shaders tab's pointers).
    pub(super) want: Option<String>,
    /// The outcome serial a last copy's answer was looked for at.
    pub(super) unequip_seen: u64,
    /// Fuse (10/11/2026): the tier shown (its place in [`crate::forge_fuse::TIERS`]), its
    /// duplicates as the last frame listed them, the one chosen and the first line or row
    /// shown, the copies picked, and the shaders of the next tier left to gain.
    pub(super) fuse_tier: usize,
    pub(super) fuse_list: Vec<crate::forge_fuse::Spare>,
    pub(super) fuse: usize,
    pub(super) fuse_first: usize,
    pub(super) fuse_picks: Vec<(&'static str, u32)>,
    pub(super) fuse_targets: usize,
    /// The picks are to be proposed again (the tier changed, or a fusion went).
    pub(super) fuse_stale: bool,
    /// The shader the last fusion gave, revealed on Fuse's card until the picks change.
    pub(super) fused: Option<&'static Unlockable>,
}

impl Forge {
    /// The filter of the section on show (Shop has none).
    pub(super) fn filter_mut(&mut self) -> &mut ForgeFilter {
        match self.section {
            Section::Break => &mut self.break_filter,
            _ => &mut self.craft_filter,
        }
    }

    pub(super) fn filter(&self) -> &ForgeFilter {
        match self.section {
            Section::Break => &self.break_filter,
            _ => &self.craft_filter,
        }
    }

    /// How many things the section on show lists.
    pub(super) fn listed(&self) -> usize {
        match self.section {
            Section::Shop => self.shop_choices(),
            Section::Craft => self.craft_list.len(),
            Section::Break => self.break_list.len(),
            Section::Fuse => self.fuse_list.len(),
        }
    }

    /// The chosen thing of the section on show.
    fn chosen_mut(&mut self) -> &mut usize {
        match self.section {
            Section::Shop => &mut self.shop_selected,
            Section::Craft => &mut self.craft,
            Section::Break => &mut self.breaks,
            Section::Fuse => &mut self.fuse,
        }
    }

    /// The sections shown, Fuse on a hub that fuses.
    pub(super) fn sections(&self) -> &'static [Section] {
        Section::shown(self.data.fuses())
    }

    /// Fuse's tier.
    pub(super) fn fuse_tier(&self) -> Rarity {
        crate::forge_fuse::TIERS[self.fuse_tier.min(crate::forge_fuse::TIERS.len() - 1)]
    }

    /// The copies picked on Fuse in all.
    pub(super) fn fuse_picked(&self) -> u32 {
        self.fuse_picks.iter().map(|(_, copies)| copies).sum()
    }

    /// The copies picked of shader `id` on Fuse, 0 when none.
    pub(super) fn fuse_copies(&self, id: &str) -> u32 {
        self.fuse_picks
            .iter()
            .find(|(picked, _)| *picked == id)
            .map_or(0, |(_, copies)| *copies)
    }

    /// Whether the picks may be sent as a fusion, or why not.
    pub(super) fn fuse_check(&self) -> Result<(), crate::forge_fuse::Block> {
        let tier = self.fuse_tier();
        crate::forge_fuse::check(
            tier,
            &self.fuse_picks,
            &self.fuse_list,
            self.data.fuse_need(tier),
            self.fuse_targets,
        )
    }

    /// Take Fuse's list for this frame (`list`, the tier's spare duplicates, `targets`
    /// shaders of the next tier left): the picks proposed again when stale, else kept to
    /// what may still go.
    pub(super) fn follow_fuse(&mut self, list: Vec<crate::forge_fuse::Spare>, targets: usize) {
        self.fuse_list = list;
        self.fuse_targets = targets;
        let need = self.data.fuse_need(self.fuse_tier()).unwrap_or(0);
        if self.fuse_stale {
            self.fuse_stale = false;
            self.fuse_picks = crate::forge_fuse::propose(&self.fuse_list, need);
            return;
        }
        let list = &self.fuse_list;
        self.fuse_picks.retain_mut(|(id, copies)| {
            let most = list
                .iter()
                .find(|spare| spare.skin.id == *id)
                .map_or(0, |spare| spare.spare());
            *copies = (*copies).min(most);
            *copies > 0
        });
    }

    /// Show Fuse's tier `index` (of [`crate::forge_fuse::TIERS`]): its proposal comes.
    pub(super) fn set_fuse_tier(&mut self, index: usize) -> PanelAction {
        if index >= crate::forge_fuse::TIERS.len() {
            return PanelAction::None;
        }
        if self.fuse_tier != index {
            ui_cues::post(Cue::Click);
        }
        self.fuse_tier = index;
        self.fuse_stale = true;
        self.fused = None;
        self.fuse = 0;
        self.fuse_first = 0;
        self.armed = None;
        PanelAction::None
    }

    /// Step Fuse's tier to the next one the hub fuses (the previous with `back`).
    fn cycle_fuse_tier(&mut self, back: bool) -> PanelAction {
        let count = crate::forge_fuse::TIERS.len();
        for step in 1..=count {
            let index = if back {
                (self.fuse_tier + count * 2 - step) % count
            } else {
                (self.fuse_tier + step) % count
            };
            if self
                .data
                .fuse_need(crate::forge_fuse::TIERS[index])
                .is_some()
            {
                return self.set_fuse_tier(index);
            }
        }
        PanelAction::None
    }

    /// Pick a copy of Fuse's shader at `index`, or let it go when picked; no more than the
    /// fusion takes, never its last copy.
    fn toggle_fuse_pick(&mut self, index: usize) {
        let Some(spare) = self.fuse_list.get(index).copied() else {
            return;
        };
        self.armed = None;
        self.fused = None;
        if let Some(at) = self
            .fuse_picks
            .iter()
            .position(|(id, _)| *id == spare.skin.id)
        {
            self.fuse_picks.remove(at);
            ui_cues::post(Cue::Back);
            return;
        }
        let need = self.data.fuse_need(self.fuse_tier()).unwrap_or(0);
        if spare.spare() > 0 && self.fuse_picked() < need {
            self.fuse_picks.push((spare.skin.id, 1));
            ui_cues::post(Cue::Click);
        }
    }

    /// One copy more (`more`) or less of Fuse's chosen shader (+ and -, or the stepper).
    pub(super) fn step_fuse(&mut self, more: bool) -> PanelAction {
        let Some(spare) = self.fuse_list.get(self.fuse).copied() else {
            return PanelAction::None;
        };
        let need = self.data.fuse_need(self.fuse_tier()).unwrap_or(0);
        let room = self.fuse_picked() < need;
        let at = self
            .fuse_picks
            .iter()
            .position(|(id, _)| *id == spare.skin.id);
        let changed = match (at, more) {
            (Some(at), true) if room && self.fuse_picks[at].1 < spare.spare() => {
                self.fuse_picks[at].1 += 1;
                true
            }
            (Some(at), false) => {
                self.fuse_picks[at].1 -= 1;
                if self.fuse_picks[at].1 == 0 {
                    self.fuse_picks.remove(at);
                }
                true
            }
            (None, true) if room && spare.spare() > 0 => {
                self.fuse_picks.push((spare.skin.id, 1));
                true
            }
            _ => false,
        };
        if changed {
            self.armed = None;
            self.fused = None;
            ui_cues::post(Cue::Click);
        }
        PanelAction::None
    }

    /// Propose Fuse's picks again (P).
    pub(super) fn propose_fuse(&mut self) -> PanelAction {
        let need = self.data.fuse_need(self.fuse_tier()).unwrap_or(0);
        self.fuse_picks = crate::forge_fuse::propose(&self.fuse_list, need);
        self.armed = None;
        self.fused = None;
        ui_cues::post(Cue::Click);
        PanelAction::None
    }

    /// Fuse's button: asks twice, then sends the picks.
    fn press_fuse(&mut self, now: Instant) -> PanelAction {
        if self.fuse_check().is_err() {
            self.armed = None;
            return PanelAction::None;
        }
        self.press(Press::Fuse, now)
    }

    /// Add shader `id` to the wishlist (`on`) or take it off, when the hub keeps one; a
    /// full wishlist takes nothing more.
    pub(super) fn wish(&self, id: &'static str) -> PanelAction {
        if self.data.wishlist.is_none() {
            return PanelAction::None;
        }
        let on = !self.data.wished(id);
        if on && self.data.wishlist_full() {
            return PanelAction::None;
        }
        ui_cues::post(Cue::Click);
        PanelAction::Wish(id, on)
    }

    /// The day's offers, once read.
    pub(super) fn offers(&self) -> &[sjk_identity::Offer] {
        self.data
            .shop
            .as_ref()
            .map_or(&[], |state| state.day.offers.as_slice())
    }

    /// Whether the shop's day was read.
    pub(super) fn shop_read(&self) -> bool {
        self.data
            .shop
            .as_ref()
            .is_some_and(|state| state.message.is_empty())
    }

    /// How many things the shop offers to choose: the offers, then the holocrons for
    /// credits once the day is read.
    pub(super) fn shop_choices(&self) -> usize {
        let offers = self.offers().len().min(4);
        if self.shop_read() {
            offers + crate::economy::holocrons_for_sale().len()
        } else {
            offers
        }
    }

    /// What the player holds of the shop's currency: kyber shards (credits on hubs before
    /// 17/10/2026).
    pub(super) fn purse(&self) -> u64 {
        if crate::economy::shop_in_shards() {
            self.data.shards.unwrap_or(0)
        } else {
            self.data.credits.unwrap_or(0)
        }
    }

    /// Whether a trade sent from the Forge waits for the hub's answer.
    pub(super) fn waiting(&self) -> bool {
        let serial = self.data.trade.as_ref().map_or(0, |trade| trade.serial);
        self.sent.is_some_and(|(asked, _)| asked == serial)
            || self.sent_all.is_some_and(|asked| asked == serial)
            || self
                .sent_many
                .as_ref()
                .is_some_and(|(asked, _)| *asked == serial)
    }

    /// Whether `press` is armed at `now`.
    pub(super) fn is_armed(&self, press: Press, now: Instant) -> bool {
        self.armed.is_some_and(|(armed, at)| {
            armed == press && now.saturating_duration_since(at) < CONFIRM
        })
    }

    /// The hub's answer to the last trade sent from the Forge, once it came: the trade
    /// and what was asked. A banner broken down on the Banners tab (05/11/2026) is never
    /// the Forge's.
    pub(super) fn answer(&self) -> Option<(&sjk_identity::TradeOutcome, Option<Press>)> {
        let outcome = self
            .data
            .trade
            .as_ref()
            .filter(|outcome| crate::profile_card::banner::get(&outcome.id).is_none())?;
        if let Some((asked, press)) = self.sent
            && outcome.serial > asked
            && outcome.trade != sjk_identity::Trade::ShatterAll
            && outcome.trade != sjk_identity::Trade::ShatterMany
        {
            return Some((outcome, Some(press)));
        }
        if let Some(asked) = self.sent_all
            && outcome.serial > asked
            && outcome.trade == sjk_identity::Trade::ShatterAll
        {
            return Some((outcome, None));
        }
        if let Some((asked, _)) = &self.sent_many
            && outcome.serial > *asked
            && outcome.trade == sjk_identity::Trade::ShatterMany
        {
            return Some((outcome, None));
        }
        None
    }

    /// The serial of the last outcome, 0 before any.
    fn serial(&self) -> u64 {
        self.data.trade.as_ref().map_or(0, |trade| trade.serial)
    }

    /// Press `press` at `now`: the first press arms it, a second within [`CONFIRM`] sends
    /// it. Nothing while another trade waits for the hub.
    pub(super) fn press(&mut self, press: Press, now: Instant) -> PanelAction {
        if self.waiting() || !self.data.can_trade {
            return PanelAction::None;
        }
        if !self.is_armed(press, now) {
            self.armed = Some((press, now));
            ui_cues::post(Cue::Click);
            return PanelAction::None;
        }
        self.armed = None;
        ui_cues::post(Cue::Click);
        self.send(press)
    }

    /// Send `press` to the hub.
    fn send(&mut self, press: Press) -> PanelAction {
        self.sent = Some((self.serial(), press));
        match press {
            Press::Offer(index) => self.offers().get(index).map_or(PanelAction::None, |offer| {
                PanelAction::Buy(offer.id.clone())
            }),
            Press::Holocron(index) => crate::economy::holocrons_for_sale()
                .get(index)
                .map_or(PanelAction::None, |(tier, _)| {
                    PanelAction::BuyHolocron((*tier).to_owned())
                }),
            Press::Craft(id) => PanelAction::Craft(id),
            Press::Break(id) => PanelAction::Shatter(id),
            Press::Fuse => PanelAction::Fuse(crate::forge_fuse::ids(&self.fuse_picks)),
        }
    }

    /// The chosen shop thing's button.
    fn press_shop(&mut self, index: usize, now: Instant) -> PanelAction {
        self.shop_selected = index;
        let offers = self.offers().len().min(4);
        if index < offers {
            let Some(offer) = self.offers().get(index) else {
                return PanelAction::None;
            };
            if can(offer, self.purse()) != Can::Buy {
                return PanelAction::None;
            }
            return self.press(Press::Offer(index), now);
        }
        let holocron = index - offers;
        let Some((_, price)) = crate::economy::holocrons_for_sale().get(holocron).copied() else {
            return PanelAction::None;
        };
        if self.data.credits.unwrap_or(0) < price || !self.shop_read() {
            return PanelAction::None;
        }
        self.press(Press::Holocron(holocron), now)
    }

    /// The chosen Craft shader's Craft.
    fn press_craft(&mut self, now: Instant) -> PanelAction {
        let Some(chosen) = self.craft_list.get(self.craft).copied() else {
            return PanelAction::None;
        };
        if self.data.shards.unwrap_or(0) < chosen.cost {
            return PanelAction::None;
        }
        self.press(Press::Craft(chosen.skin.id), now)
    }

    /// The chosen Break down shader's button: a duplicate asks twice; the last copy asks
    /// with the card; a locked one does nothing (its button says why).
    fn press_break(&mut self, now: Instant) -> PanelAction {
        let Some(chosen) = self.break_list.get(self.breaks).copied() else {
            return PanelAction::None;
        };
        if chosen.locked {
            self.armed = None;
            return PanelAction::None;
        }
        if chosen.last() {
            if self.data.last && self.data.can_trade && !self.waiting() {
                self.armed = None;
                self.ask = Some(Ask::Last {
                    skin: chosen.skin,
                    yes: false,
                });
                ui_cues::post(Cue::Click);
            }
            return PanelAction::None;
        }
        self.press(Press::Break(chosen.skin.id), now)
    }

    /// Break down all duplicates: asks with the card first.
    pub(super) fn ask_all(&mut self) -> PanelAction {
        if self.data.all && self.data.can_trade && !self.waiting() && self.spare.0 > 0 {
            self.armed = None;
            self.ask = Some(Ask::All { yes: true });
            ui_cues::post(Cue::Click);
        }
        PanelAction::None
    }

    /// The picked shaders, as Break down lists them past the filter, in the order picked,
    /// each with the copies picked.
    pub(super) fn picked_items(&self) -> Vec<(Owned, u32)> {
        self.picked
            .iter()
            .filter_map(|(id, copies)| {
                self.break_all
                    .iter()
                    .find(|item| item.skin.id == *id)
                    .map(|item| (*item, *copies))
            })
            .collect()
    }

    /// The copies picked of shader `id`, 0 when it is not picked.
    pub(super) fn picked_copies(&self, id: &str) -> u32 {
        self.picked
            .iter()
            .find(|(picked, _)| *picked == id)
            .map_or(0, |(_, copies)| *copies)
    }

    /// What the picks give: shaders, copies, kyber shards, and how many shaders lose
    /// their last copy.
    pub(super) fn picked_sum(&self) -> (usize, u32, u64, usize) {
        let items = self.picked_items();
        (
            items.len(),
            items.iter().map(|(_, copies)| copies).sum(),
            items
                .iter()
                .map(|(item, copies)| u64::from(*copies) * item.gives)
                .sum(),
            items
                .iter()
                .filter(|(item, copies)| *copies >= item.count)
                .count(),
        )
    }

    /// The most copies of `item` that may be picked: every one on a hub that breaks last
    /// copies down, else all but one; none of a locked shader.
    pub(super) fn pick_max(&self, item: Owned) -> u32 {
        if item.locked {
            0
        } else if self.data.last {
            item.count
        } else {
            item.count.saturating_sub(1)
        }
    }

    /// Keep the picks to what may still go: a shader no longer held or now locked is
    /// dropped, its copies kept within what is held; nothing is picked while Select is off.
    pub(super) fn prune_picks(&mut self) {
        if !self.data.many {
            self.selecting = false;
        }
        if !self.selecting {
            self.picked.clear();
            return;
        }
        let keep: Vec<(&'static str, u32)> = self
            .picked_items()
            .into_iter()
            .filter_map(|(item, copies)| {
                let most = self.pick_max(item);
                (most > 0).then_some((item.skin.id, copies.min(most)))
            })
            .collect();
        self.picked = keep;
    }

    /// Switch Break down's Select (S) on or off; off drops the picks.
    pub(super) fn toggle_select(&mut self) -> PanelAction {
        if self.section != Section::Break || !self.data.many {
            return PanelAction::None;
        }
        self.selecting = !self.selecting;
        self.armed = None;
        if !self.selecting {
            self.picked.clear();
        }
        ui_cues::post(Cue::Click);
        PanelAction::None
    }

    /// Pick a copy of the Break down shader at `index` (or let it go when picked).
    fn toggle_pick(&mut self, index: usize) {
        let Some(item) = self.break_list.get(index).copied() else {
            return;
        };
        if let Some(at) = self.picked.iter().position(|(id, _)| *id == item.skin.id) {
            self.picked.remove(at);
            ui_cues::post(Cue::Back);
        } else if self.pick_max(item) > 0 && self.picked.len() < PICKS_MAX {
            self.picked.push((item.skin.id, 1));
            ui_cues::post(Cue::Click);
        }
    }

    /// One copy more (`more`) or less of the chosen Break down shader, picking it first
    /// when it is not; within 1 and [`Forge::pick_max`] (+ and -, or the stepper).
    pub(super) fn step_pick(&mut self, more: bool) -> PanelAction {
        if !self.selecting || self.section != Section::Break {
            return PanelAction::None;
        }
        let Some(item) = self.break_list.get(self.breaks).copied() else {
            return PanelAction::None;
        };
        let most = self.pick_max(item);
        let room = self.picked.len() < PICKS_MAX;
        match self.picked.iter_mut().find(|(id, _)| *id == item.skin.id) {
            Some((_, copies)) => {
                let next = if more {
                    (*copies + 1).min(most)
                } else {
                    copies.saturating_sub(1).max(1)
                };
                if next != *copies {
                    *copies = next;
                    ui_cues::post(Cue::Click);
                }
            }
            None if more && most > 0 && room => {
                self.picked.push((item.skin.id, 1));
                ui_cues::post(Cue::Click);
            }
            None => {}
        }
        PanelAction::None
    }

    /// Break down what is picked: asks with the card first.
    fn ask_picked(&mut self) -> PanelAction {
        if self.data.many && self.data.can_trade && !self.waiting() && !self.picked.is_empty() {
            self.armed = None;
            self.ask = Some(Ask::Picked { yes: true });
            ui_cues::post(Cue::Click);
        }
        PanelAction::None
    }

    /// The card answered: yes sends it.
    fn answer_ask(&mut self, yes: bool) -> PanelAction {
        let Some(ask) = self.ask.take() else {
            return PanelAction::None;
        };
        ui_cues::post(if yes { Cue::Click } else { Cue::Back });
        if !yes || self.waiting() {
            return PanelAction::None;
        }
        match ask {
            Ask::Last { skin, .. } => {
                self.sent = Some((self.serial(), Press::Break(skin.id)));
                PanelAction::ShatterLast(skin.id)
            }
            Ask::All { .. } => {
                self.sent_all = Some(self.serial());
                PanelAction::ShatterAll
            }
            Ask::Picked { .. } => {
                let items = self.picked_items();
                let lasts: Vec<&'static str> = items
                    .iter()
                    .filter(|(item, copies)| *copies >= item.count)
                    .map(|(item, _)| item.skin.id)
                    .collect();
                // `last` only when a last copy is among them, as the one-shader button.
                let last = !lasts.is_empty();
                self.sent_many = Some((self.serial(), lasts));
                PanelAction::ShatterMany(
                    items
                        .iter()
                        .map(|(item, copies)| (item.skin.id, *copies))
                        .collect(),
                    last,
                )
            }
        }
    }

    /// The press of the section's main button (Enter).
    fn activate(&mut self, now: Instant) -> PanelAction {
        match self.section {
            Section::Shop => self.press_shop(self.shop_selected, now),
            Section::Craft => self.press_craft(now),
            Section::Break if self.selecting => self.ask_picked(),
            Section::Break => self.press_break(now),
            Section::Fuse => self.press_fuse(now),
        }
    }

    /// Show `section`: its choice and search kept, the shop read on its first showing.
    pub(super) fn show(&mut self, section: Section) {
        if self.section != section {
            self.leave_search();
            self.armed = None;
            ui_cues::post(Cue::Click);
            // Fuse proposes its picks afresh each time it shows.
            if section == Section::Fuse {
                self.fuse_stale = true;
                self.fused = None;
            }
        }
        self.section = section;
        if section != Section::Shop {
            self.shop_asked = false;
        }
    }

    /// Whether the shop should be read now: the Shop section shows and has not asked
    /// since it showed, or its day has turned (once a day).
    pub(super) fn shop_due(&mut self, now: Instant) -> bool {
        if self.section != Section::Shop || !self.data.can_trade {
            return false;
        }
        if !self.shop_asked {
            self.shop_asked = true;
            return true;
        }
        let Some(state) = &self.data.shop else {
            return false;
        };
        if !state.message.is_empty() || state.seconds_left(now) > 0 {
            return false;
        }
        if self.shop_reread == Some(state.serial) {
            return false;
        }
        self.shop_reread = Some(state.serial);
        true
    }

    /// The search field lets go of the typing.
    pub(super) fn leave_search(&mut self) {
        if self.search_focus {
            self.search_focus = false;
            crate::text_entry::deselect();
        }
    }

    /// Put the typing in the section's search field.
    fn focus_search(&mut self) -> PanelAction {
        if self.section == Section::Shop {
            return PanelAction::None;
        }
        self.search_focus = true;
        let query = &self.filter().shaders.query;
        crate::text_entry::select_all(query);
        PanelAction::None
    }

    /// The list changed: back to its top.
    fn filter_changed(&mut self) -> PanelAction {
        *self.chosen_mut() = 0;
        match self.section {
            Section::Break => self.break_first = 0,
            Section::Fuse => self.fuse_first = 0,
            _ => self.craft_first = 0,
        }
        self.armed = None;
        PanelAction::None
    }

    /// Show one tier: the next (the previous with `back`), All after the last.
    fn cycle_tier(&mut self, back: bool) -> PanelAction {
        let filter = &mut self.filter_mut().shaders;
        let at = filter
            .tier
            .and_then(|tier| FILTER_TIERS.iter().position(|&t| t == tier))
            .map_or(0, |index| index + 1);
        let count = FILTER_TIERS.len() + 1;
        let next = if back {
            (at + count - 1) % count
        } else {
            (at + 1) % count
        };
        filter.tier = next.checked_sub(1).map(|index| FILTER_TIERS[index]);
        self.filter_changed()
    }

    /// Show tier chip `index` (0 All).
    pub(super) fn set_tier(&mut self, index: usize) -> PanelAction {
        self.filter_mut().shaders.tier = index.checked_sub(1).map(|index| FILTER_TIERS[index]);
        self.filter_changed()
    }

    /// Break down's Favourites chip (Shift+F): its favourites alone, or all again.
    pub(super) fn toggle_favourites(&mut self, kept: bool) -> PanelAction {
        if self.section != Section::Break || (!kept && !self.break_filter.shaders.favourite) {
            return PanelAction::None;
        }
        let filter = &mut self.break_filter.shaders;
        filter.favourite = !filter.favourite;
        self.filter_changed()
    }

    /// The shader chosen in the section on show, and whether the player holds it (Break
    /// down's are theirs, Craft's are not): what F and L mark.
    pub(super) fn chosen_shader(&self) -> Option<(&'static str, bool)> {
        match self.section {
            Section::Craft => self
                .craft_list
                .get(self.craft)
                .map(|item| (item.skin.id, false)),
            Section::Break => self
                .break_list
                .get(self.breaks)
                .map(|item| (item.skin.id, true)),
            Section::Fuse => self
                .fuse_list
                .get(self.fuse)
                .map(|item| (item.skin.id, true)),
            Section::Shop => None,
        }
    }

    /// The section's own chip: Affordable or Duplicates.
    fn toggle_only(&mut self) -> PanelAction {
        let filter = self.filter_mut();
        filter.only = !filter.only;
        self.filter_changed()
    }

    /// Move the choice `by` things in a list of `count`, stopping at the ends.
    fn step(&mut self, by: isize, count: usize) {
        if count == 0 {
            return;
        }
        let chosen = self.chosen_mut();
        let at = (*chosen).min(count - 1) as isize;
        *chosen = (at + by).clamp(0, count as isize - 1) as usize;
        self.armed = None;
    }

    /// An arrow in the section on show; `grid` whether Craft and Break down show the grid.
    fn arrow(&mut self, key: KeyCode, grid: bool) {
        let count = self.listed();
        let line = if grid && self.section != Section::Shop {
            COLUMNS as isize
        } else {
            1
        };
        match (self.section, key) {
            (Section::Shop, KeyCode::ArrowLeft | KeyCode::ArrowUp) => self.step(-1, count),
            (Section::Shop, _) => self.step(1, count),
            (_, KeyCode::ArrowLeft) if grid => self.step(-1, count),
            (_, KeyCode::ArrowRight) if grid => self.step(1, count),
            (_, KeyCode::ArrowUp) => self.step(-line, count),
            (_, KeyCode::ArrowDown) => {
                // A short last line takes Down to its last thing.
                let at = *self.chosen_mut();
                let lines = |index: usize| index / line as usize;
                if count > 0 && lines(at) < lines(count - 1) {
                    self.step(line, count);
                }
            }
            _ => {}
        }
    }

    /// A key while the Forge shows; `grid` whether Craft and Break down show the grid.
    pub(super) fn key(
        &mut self,
        text: Option<&str>,
        key: KeyCode,
        shift: bool,
        control: bool,
        grid: bool,
    ) -> PanelAction {
        let now = Instant::now();
        if let Some(ask) = self.ask {
            return match key {
                KeyCode::Escape => self.answer_ask(false),
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    self.answer_ask(ask.yes())
                }
                KeyCode::Tab | KeyCode::ArrowLeft | KeyCode::ArrowRight => {
                    self.ask = Some(ask.with_focus(!ask.yes()));
                    PanelAction::None
                }
                _ => PanelAction::None,
            };
        }
        if self.search_focus {
            match key {
                KeyCode::Escape => {
                    self.leave_search();
                    if !self.filter().shaders.query.is_empty() {
                        self.filter_mut().shaders.query.clear();
                        return self.filter_changed();
                    }
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => self.leave_search(),
                KeyCode::ArrowUp | KeyCode::ArrowDown => self.arrow(key, grid),
                _ => {
                    let query = &mut self.filter_mut().shaders.query;
                    if crate::text_entry::edit(query, key, text, shift, control, SEARCH_MAX, true) {
                        return self.filter_changed();
                    }
                }
            }
            return PanelAction::None;
        }
        let listing = matches!(self.section, Section::Craft | Section::Break);
        let picking = self.section == Section::Break && self.selecting;
        let fusing = self.section == Section::Fuse;
        // + and - as typed, whatever the layout puts them on (a Slovak keyboard's + is on
        // the 1 key, which otherwise changes section).
        if picking || fusing {
            match text {
                Some("+") if fusing => return self.step_fuse(true),
                Some("-") if fusing => return self.step_fuse(false),
                Some("+") => return self.step_pick(true),
                Some("-") => return self.step_pick(false),
                _ => {}
            }
        }
        match key {
            // Escape stops picking first, then leaves.
            KeyCode::Escape if picking => self.toggle_select(),
            KeyCode::Escape => PanelAction::Close,
            KeyCode::Tab => {
                self.show(self.section.next_in(!shift, self.data.fuses()));
                PanelAction::None
            }
            KeyCode::Digit1 | KeyCode::Digit2 | KeyCode::Digit3 => {
                self.show(match key {
                    KeyCode::Digit1 => Section::Shop,
                    KeyCode::Digit2 => Section::Craft,
                    _ => Section::Break,
                });
                PanelAction::None
            }
            KeyCode::Digit4 if self.data.fuses() => {
                self.show(Section::Fuse);
                PanelAction::None
            }
            KeyCode::Space if fusing => {
                self.toggle_fuse_pick(self.fuse);
                PanelAction::None
            }
            KeyCode::KeyT if fusing => self.cycle_fuse_tier(shift),
            KeyCode::KeyP if fusing && !control => self.propose_fuse(),
            KeyCode::Equal | KeyCode::NumpadAdd if fusing => self.step_fuse(true),
            KeyCode::Minus | KeyCode::NumpadSubtract if fusing => self.step_fuse(false),
            KeyCode::KeyW if self.section == Section::Craft && !control => {
                match self.craft_list.get(self.craft) {
                    Some(item) => self.wish(item.skin.id),
                    None => PanelAction::None,
                }
            }
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                self.arrow(key, grid);
                PanelAction::None
            }
            KeyCode::Home => {
                *self.chosen_mut() = 0;
                PanelAction::None
            }
            KeyCode::End => {
                *self.chosen_mut() = self.listed().saturating_sub(1);
                PanelAction::None
            }
            KeyCode::Space if picking => {
                self.toggle_pick(self.breaks);
                PanelAction::None
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => self.activate(now),
            KeyCode::KeyV if listing || fusing => PanelAction::ShaderView(!grid),
            KeyCode::Slash if listing => self.focus_search(),
            KeyCode::KeyF if listing && control => self.focus_search(),
            KeyCode::KeyT if listing => self.cycle_tier(shift),
            KeyCode::KeyA if self.section == Section::Craft => self.toggle_only(),
            KeyCode::KeyD if self.section == Section::Break => self.toggle_only(),
            KeyCode::KeyB if self.section == Section::Break => self.ask_all(),
            KeyCode::KeyS if self.section == Section::Break && !control => self.toggle_select(),
            KeyCode::Equal | KeyCode::NumpadAdd if picking => self.step_pick(true),
            KeyCode::Minus | KeyCode::NumpadSubtract if picking => self.step_pick(false),
            _ => PanelAction::None,
        }
    }

    /// A click on (or the pointer resting on) the Forge's `token`; whether it was one of
    /// the Forge's. `activate` for a click.
    pub(super) fn token(
        &mut self,
        token: u16,
        activate: bool,
        now: Instant,
    ) -> Option<PanelAction> {
        let within = |base: u16, count: usize| {
            token
                .checked_sub(base)
                .map(usize::from)
                .filter(|index| *index < count)
        };
        if let Some(ask) = self.ask {
            // Only the card's buttons answer while it asks.
            return Some(match token {
                ASK_YES_TOKEN if activate => self.answer_ask(true),
                ASK_NO_TOKEN if activate => self.answer_ask(false),
                ASK_YES_TOKEN | ASK_NO_TOKEN => {
                    self.ask = Some(ask.with_focus(token == ASK_YES_TOKEN));
                    PanelAction::None
                }
                _ => PanelAction::None,
            });
        }
        // Resting on a shop offer chooses it, as on every tab; a shader tile waits for a click.
        let offers = self.offers().len().min(4);
        if let Some(index) = within(OFFER_BASE, offers)
            .or_else(|| within(HOLOCRON_BASE, 2).map(|index| index + offers))
            .filter(|_| self.section == Section::Shop)
        {
            if self.shop_selected != index {
                self.armed = None;
            }
            self.shop_selected = index.min(self.shop_choices().saturating_sub(1));
            return Some(PanelAction::None);
        }
        // A shader tile is chosen (and previewed) by a click only, as on Shaders: resting
        // the pointer on the way to Break down never changes it (Creyon, 31/10/2026).
        if let Some(index) =
            within(TILE_BASE, self.listed()).filter(|_| self.section != Section::Shop && activate)
        {
            if *self.chosen_mut() != index {
                self.armed = None;
            }
            *self.chosen_mut() = index;
            // While Select is on, a click picks it too (or lets it go); on Fuse always.
            if self.section == Section::Break && self.selecting {
                self.toggle_pick(index);
            } else if self.section == Section::Fuse {
                self.toggle_fuse_pick(index);
            }
            return Some(PanelAction::None);
        }
        if !activate {
            return forge_token(token).then_some(PanelAction::None);
        }
        if token != SEARCH_TOKEN {
            self.leave_search();
        }
        if let Some(index) = within(SECTION_BASE, self.sections().len()) {
            self.show(Section::ALL[index]);
            return Some(PanelAction::None);
        }
        if let Some(index) = within(FUSE_TIER_BASE, crate::forge_fuse::TIERS.len())
            .filter(|_| self.section == Section::Fuse)
        {
            return Some(self.set_fuse_tier(index));
        }
        if let Some(index) =
            within(OFFER_WISH_BASE, offers).filter(|_| self.section == Section::Shop)
        {
            let id = self
                .offers()
                .get(index)
                .and_then(|offer| unlockables::find(&offer.id))
                .map(|skin| skin.id);
            return Some(id.map_or(PanelAction::None, |id| self.wish(id)));
        }
        if let Some(index) = within(OFFER_BUY_BASE, offers) {
            return Some(self.press_shop(index, now));
        }
        if let Some(index) = within(HOLOCRON_BUY_BASE, 2) {
            return Some(self.press_shop(offers + index, now));
        }
        if let Some(index) = within(TIER_BASE, 6) {
            return Some(self.set_tier(index));
        }
        Some(match token {
            ONLY_TOKEN => self.toggle_only(),
            // The Panel answers the star, the padlock and the Favourites chip, which need
            // the marks.
            STAR_TOKEN | PADLOCK_TOKEN | FAVOURITES_TOKEN => return None,
            SEARCH_TOKEN => self.focus_search(),
            LIST_TOKEN => PanelAction::ShaderView(false),
            GRID_TOKEN => PanelAction::ShaderView(true),
            ACTION_TOKEN => self.activate(now),
            ALL_TOKEN => self.ask_all(),
            SELECT_TOKEN => self.toggle_select(),
            PICK_MORE_TOKEN if self.section == Section::Fuse => self.step_fuse(true),
            PICK_LESS_TOKEN if self.section == Section::Fuse => self.step_fuse(false),
            PICK_MORE_TOKEN => self.step_pick(true),
            PICK_LESS_TOKEN => self.step_pick(false),
            PROPOSE_TOKEN => self.propose_fuse(),
            WISH_TOKEN => match self.craft_list.get(self.craft) {
                Some(item) if self.section == Section::Craft => self.wish(item.skin.id),
                _ => PanelAction::None,
            },
            SHARDS_TOKEN | CREDITS_TOKEN => PanelAction::None,
            _ => return None,
        })
    }

    /// The wheel over the section's list: a line (grid) or a row a notch, the chosen thing
    /// kept in view.
    pub(super) fn wheel(&mut self, up: bool, grid: bool) {
        let count = self.listed();
        let (per, shown) = if grid { (COLUMNS, LINES) } else { (1, ROWS) };
        let lines = count.div_ceil(per);
        let last = lines.saturating_sub(shown);
        let first = match self.section {
            Section::Shop => return,
            Section::Craft => &mut self.craft_first,
            Section::Break => &mut self.break_first,
            Section::Fuse => &mut self.fuse_first,
        };
        *first = if up {
            first.saturating_sub(1)
        } else {
            (*first + 1).min(last)
        };
        let first = *first;
        let chosen = self.chosen_mut();
        let line = (*chosen / per).clamp(first, first + shown - 1);
        *chosen = (line * per + *chosen % per).min(count.saturating_sub(1));
    }

    /// Follow the hub's answers: a last copy broken down is noted for the console, which
    /// takes it off the saber when worn ([`Forge::take_unequip`]).
    pub(super) fn follow(&mut self) {
        let Some(outcome) = self.data.trade.as_ref() else {
            return;
        };
        if outcome.serial <= self.unequip_seen {
            return;
        }
        self.unequip_seen = outcome.serial;
        if outcome.done() && outcome.trade == sjk_identity::Trade::ShatterLast {
            self.unequip
                .extend(unlockables::find(&outcome.id).map(|skin| skin.id));
        }
        if outcome.done()
            && outcome.trade == sjk_identity::Trade::ShatterMany
            && let Some((asked, lasts)) = &self.sent_many
            && outcome.serial > *asked
        {
            // Everything picked went: the last copies among them come off the saber.
            self.unequip.extend(lasts.iter().copied());
            self.picked.clear();
            self.selecting = false;
        }
        // A fusion went: the shader gained is revealed, and the next picks proposed.
        if outcome.trade == sjk_identity::Trade::Fuse
            && self
                .sent
                .is_some_and(|(asked, press)| press == Press::Fuse && outcome.serial > asked)
        {
            self.fuse_stale = true;
            if outcome.done() {
                self.fused = unlockables::find(&outcome.id);
            }
        }
    }

    /// A shader whose last copy the hub just broke down, each once.
    pub(super) fn take_unequip(&mut self) -> Option<&'static str> {
        (!self.unequip.is_empty()).then(|| self.unequip.remove(0))
    }
}

impl Panel {
    /// F, Shift+F and L in the Forge (29/10/2026, collection marks): star or lock the
    /// chosen shader, or show Break down's favourites alone; `None` for another key.
    pub(super) fn forge_mark_key(&mut self, key: KeyCode, shift: bool) -> Option<PanelAction> {
        match key {
            KeyCode::KeyF if shift => Some(self.forge.toggle_favourites(self.marks.kept)),
            KeyCode::KeyF => Some(self.forge_mark(sjk_identity::Mark::Favourite)),
            KeyCode::KeyL => Some(self.forge_mark(sjk_identity::Mark::Locked)),
            _ => None,
        }
    }

    /// Star or lock the shader chosen in the Forge (the padlock only for one the player
    /// holds); it stays chosen wherever the favourites move it.
    pub(super) fn forge_mark(&mut self, mark: sjk_identity::Mark) -> PanelAction {
        let Some((id, owned)) = self.forge.chosen_shader() else {
            return PanelAction::None;
        };
        let action = super::mark_action(&self.marks, id, mark, owned);
        if matches!(action, PanelAction::Mark { .. }) {
            self.forge.want = Some(id.to_owned());
            self.forge.armed = None;
        }
        action
    }

    /// The Forge's data this frame (the console's, [`ForgeData::read`]).
    pub(crate) fn set_forge(&mut self, data: ForgeData) {
        #[cfg(test)]
        if let Some(pinned) = &self.preview_forge {
            self.forge.data = pinned.clone();
            self.forge.follow();
            return;
        }
        self.forge.data = data;
        self.forge.follow();
    }

    /// Show the Forge's `section` when the Forge is next on show (the Holocrons tab's
    /// Shop, the Shaders tab's pointers).
    pub(crate) fn show_forge(&mut self, section: Section) {
        self.forge.show(section);
    }

    /// Choose `id` in the Forge's `section` (Craft or Break down) and show it: the Shaders
    /// tab's "Craft in the Forge". The filter is cleared so it is listed.
    pub(super) fn forge_on(&mut self, section: Section, id: &str) {
        self.forge.show(section);
        let filter = self.forge.filter_mut();
        *filter = ForgeFilter::default();
        self.forge.want = Some(id.to_owned());
        self.tab = Tab::Forge;
    }

    /// Whether the Forge wants the shop read now.
    pub(crate) fn forge_shop_due(&mut self, now: Instant) -> bool {
        self.open && self.tab == Tab::Forge && self.forge.shop_due(now)
    }

    /// The shader whose last copy the hub just broke down, once.
    pub(crate) fn take_forge_unequip(&mut self) -> Option<&'static str> {
        self.forge.take_unequip()
    }

    /// The Forge's section on show.
    #[cfg(test)]
    pub(crate) fn forge_section(&self) -> Section {
        self.forge.section
    }

    /// Show `data` in place of the live identity's on the Forge (`None` lets go), for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn preview_forge(&mut self, data: Option<ForgeData>) {
        self.preview_forge = data;
    }

    /// Put the Forge in the state a world shot wants: the thing `chosen`, `armed`'s button
    /// waiting for its second press, `ask` asking, a trade asked so its answer shows.
    #[cfg(test)]
    pub(crate) fn forge_for_shot(&mut self, chosen: usize, armed: bool, ask: bool, asked: bool) {
        let forge = &mut self.forge;
        *forge.chosen_mut() = chosen;
        forge.armed = None;
        forge.ask = None;
        let press = match forge.section {
            Section::Shop => {
                let offers = forge.offers().len().min(4);
                if chosen < offers {
                    Press::Offer(chosen)
                } else {
                    Press::Holocron(chosen - offers)
                }
            }
            Section::Craft => forge
                .craft_list
                .get(chosen)
                .map_or(Press::Offer(0), |item| Press::Craft(item.skin.id)),
            Section::Break => forge
                .break_list
                .get(chosen)
                .map_or(Press::Offer(0), |item| Press::Break(item.skin.id)),
            Section::Fuse => Press::Fuse,
        };
        if armed {
            forge.armed = Some((press, Instant::now()));
        }
        if ask {
            forge.ask = match forge.break_list.get(chosen) {
                Some(item) if item.last() => Some(Ask::Last {
                    skin: item.skin,
                    yes: false,
                }),
                _ => Some(Ask::All { yes: true }),
            };
        }
        forge.sent = None;
        forge.sent_all = None;
        if asked && let Some(trade) = &forge.data.trade {
            if trade.trade == sjk_identity::Trade::ShatterAll {
                forge.sent_all = Some(trade.serial - 1);
            } else {
                forge.sent = Some((trade.serial - 1, press));
            }
        }
    }

    /// Choose shader `id` in the Forge's section on its next frame, for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_want_for_shot(&mut self, id: &str) {
        self.forge.want = Some(id.to_owned());
    }

    /// Type `query` in the Forge's search (and its own chip), for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_search_for_shot(&mut self, query: &str, only: bool) {
        let filter = self.forge.filter_mut();
        filter.shaders.query = query.to_owned();
        filter.only = only;
    }

    /// Light Break down's Favourites chip (or not), for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_favourites_for_shot(&mut self, on: bool) {
        self.forge.break_filter.shaders.favourite = on;
        self.forge.breaks = 0;
        self.forge.break_first = 0;
    }

    /// Choose shader `id` on Break down, as the last frame listed it, and ask before its
    /// last copy goes, for a world shot.
    #[cfg(test)]
    pub(crate) fn forge_ask_last_for_shot(&mut self, id: &str) {
        let forge = &mut self.forge;
        if let Some(at) = forge.break_list.iter().position(|item| item.skin.id == id) {
            forge.breaks = at;
            forge.ask = Some(Ask::Last {
                skin: forge.break_list[at].skin,
                yes: false,
            });
        }
    }
}

/// An offer of the day for the world shots and the tests: shader `id` of `tier`, at `price`.
#[cfg(test)]
pub(crate) fn sample_offer(id: &str, tier: &str, price: u64) -> sjk_identity::Offer {
    sjk_identity::Offer {
        id: id.to_owned(),
        name: unlockables::find(id).map_or(String::new(), |skin| skin.name.to_owned()),
        tier: tier.to_owned(),
        price,
        bought: false,
        owned: false,
        deal: false,
        wished: false,
        edition: None,
    }
}

#[cfg(test)]
use sample_offer as offer;

/// A snapshot of a player the hub lets trade, holding `unlocks`, on a hub that fuses three
/// of each tier but Mythical (five Legendary) and keeps an empty wishlist; no shop read.
#[cfg(test)]
pub(crate) fn sample_snapshot(unlocks: Vec<sjk_identity::Unlock>) -> sjk_identity::Snapshot {
    let mut me: sjk_identity::Profile = serde_json::from_str(
        r#"{"key_id":"0123456789abcdef","key":"k","name":"Sol","bio":"","verified":false,
            "created":1}"#,
    )
    .expect("a profile");
    me.unlocks = unlocks;
    let mut snapshot = sjk_identity::Snapshot::blank(&sjk_identity::Identity::from_seed([7; 32]));
    snapshot.status = sjk_identity::Status::Online;
    snapshot.me = Some(me);
    snapshot.holocrons = Some(sjk_identity::HolocronProgress {
        progress_secs: 0,
        every_secs: 1_800,
        today: 0,
        daily_cap: 8,
        resets_at: None,
        server_time: 1_791_000_000,
        can_open: true,
        catchup: None,
        activity: None,
        read_at: Instant::now(),
    });
    snapshot.economy = Some(sjk_identity::EconomyTables {
        shatter: vec![5, 10, 25, 60, 150],
        craft: vec![40, 100, 300, 800, 2_000],
        shop: vec![30, 75, 225, 600, 1_500],
        shop_currency: "shards".to_owned(),
        abilities: vec!["fuse".to_owned()],
        fuse: vec![3, 3, 3, 5, 0],
        wishlist: Some(Vec::new()),
        ..Default::default()
    });
    snapshot
}

/// The day the shop shows in a world shot and the tests: four offers, one of each slot,
/// the next day `left` seconds away.
#[cfg(test)]
pub(crate) fn sample_day(left: i64) -> sjk_identity::ShopState {
    sjk_identity::ShopState {
        serial: 1,
        day: sjk_identity::ShopDay {
            day: "2026-10-21".to_owned(),
            offers: vec![
                offer("saber_aurora", "common", 30),
                offer("saber_sakura", "uncommon", 75),
                offer("saber_lavalamp", "rare", 225),
                offer("saber_phoenix", "mythical", 1_500),
            ],
            next: 1_791_000_000 + left,
            server_time: 1_791_000_000,
            credits: 300,
            shards: 300,
        },
        message: String::new(),
        read_at: Instant::now(),
    }
}

/// The Forge's data in a world shot and the tests: the hub open to the player, its
/// abilities as `last`, `all` and `many`, the day `left` seconds from turning.
#[cfg(test)]
pub(crate) fn sample_data(shards: u64, credits: u64, left: i64, abilities: bool) -> ForgeData {
    ForgeData {
        closed: Closed::Open,
        credits: Some(credits),
        shards: Some(shards),
        can_trade: true,
        why_not: None,
        last: abilities,
        all: abilities,
        many: abilities,
        shop: Some(sample_day(left)),
        trade: None,
        fuse: if abilities { [3, 3, 3, 5, 0] } else { [0; 5] },
        wishlist: abilities.then(Vec::new),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::collection_panel::tests::inputs;

    fn copy(id: &str, count: u32) -> sjk_identity::Unlock {
        sjk_identity::Unlock {
            holo: 0,
            solar: 0,
            id: id.to_owned(),
            granted: 1,
            note: String::new(),
            count,
            medal: None,
            card: None,
        }
    }

    fn fonts() -> crate::text::FontAtlas {
        crate::text::load_modern(1.0, None).expect("Inter")
    }

    /// Draw `panel` once with `held` in a window of `viewport`.
    fn build(panel: &mut Panel, held: &[sjk_identity::Unlock], viewport: [f32; 2]) {
        let fonts = fonts();
        panel.build(&inputs(Holdings::Known(held), ""), &fonts.font, viewport);
        assert!(!panel.overflowed(), "{viewport:?}");
    }

    /// The Forge drawn once on `section` with `held` and `data`, so its lists are known.
    fn forge(section: Section, held: &[sjk_identity::Unlock], data: ForgeData) -> Panel {
        let mut panel = Panel::new();
        panel.open(Tab::Forge, true, true, ReturnTarget::MainMenu);
        panel.show_forge(section);
        panel.preview_forge(Some(data));
        panel.set_forge(ForgeData::default());
        build(&mut panel, held, [1920.0, 1080.0]);
        panel
    }

    fn key(panel: &mut Panel, key: KeyCode) -> PanelAction {
        panel.forge.key(None, key, false, false, false)
    }

    /// Creyon, 31/10/2026: a shader tile is chosen by a click only; the pointer resting
    /// on another tile on the way to Break down leaves the choice alone.
    #[test]
    fn resting_on_a_tile_never_chooses_it() {
        let held = [copy("saber_aurora", 2), copy("saber_tidal", 3)];
        let mut panel = forge(Section::Break, &held, ForgeData::default());
        assert!(panel.forge.listed() >= 2);
        let now = Instant::now();
        assert_eq!(panel.forge.breaks, 0);
        let _ = panel.forge.token(TILE_BASE + 1, false, now);
        assert_eq!(panel.forge.breaks, 0, "resting chose it");
        let _ = panel.forge.token(TILE_BASE + 1, true, now);
        assert_eq!(panel.forge.breaks, 1, "a click chooses it");
    }

    #[test]
    fn sections_cycle_and_the_texts_read_as_players_read_them() {
        assert_eq!(Section::Shop.next(true), Section::Craft);
        assert_eq!(Section::Break.next(true), Section::Shop);
        assert_eq!(Section::Shop.next(false), Section::Break);
        assert_eq!(day_text("2026-10-21"), "21/10/2026");
        assert_eq!(day_text("soon"), "soon");
        assert_eq!(countdown_text(30), "less than a minute");
        assert_eq!(countdown_text(61), "2 min");
        assert_eq!(countdown_text(5 * 3_600 + 12 * 60), "5 h 12 min");
        let mut lava = offer("saber_lavalamp", "rare", 225);
        assert_eq!(can(&lava, 200), Can::Short(25));
        lava.owned = true;
        assert_eq!(can(&lava, 9_999), Can::Owned);
        assert_eq!(tier_of(&offer("saber_sun", "common", 1)), Rarity::Mythical);
    }

    /// Craft lists only shaders of the trade not held at all, the commonest first; Break
    /// down lists the player's own copies, duplicates first, and the last ones only on a
    /// hub that breaks them down; a medal's or a card's copy never.
    #[test]
    fn the_lists_follow_what_is_held_and_what_the_hub_can_do() {
        let held = [
            copy("saber_aurora", 1),
            copy("saber_tidal", 3),
            sjk_identity::Unlock {
                medal: Some("bug_hunter".to_owned()),
                ..copy("saber_glitch", 2)
            },
            sjk_identity::Unlock {
                card: Some("sjk_alpha_000".to_owned()),
                ..copy("saber_sun", 1)
            },
        ];
        let holdings = Holdings::Known(&held);
        let none = crate::shader_marks::Marks::default();
        let craft = craftable(&holdings, &[], &none);
        assert!(craft.iter().all(|item| crate::economy::in_trade(item.skin)));
        for id in ["saber_aurora", "saber_tidal", "saber_glitch", "saber_sun"] {
            assert!(craft.iter().all(|item| item.skin.id != id), "{id}");
        }
        assert!(
            craft
                .windows(2)
                .all(|pair| pair[0].skin.tier <= pair[1].skin.tier)
        );
        let ids = |list: Vec<Owned>| list.iter().map(|item| item.skin.id).collect::<Vec<_>>();
        assert_eq!(
            ids(owned(&holdings, true, &none)),
            ["saber_tidal", "saber_aurora"]
        );
        assert_eq!(ids(owned(&holdings, false, &none)), ["saber_tidal"]);
        assert_eq!(duplicates(&owned(&holdings, true, &none)), (2, 1, 10));
    }

    /// Hub report #305: a search for a shader the player holds finds it among the held
    /// ones, which Craft leaves out; no search, or one for a shader they lack, finds none.
    #[test]
    fn a_search_names_the_held_shader_craft_leaves_out() {
        let held = [copy("saber_kyber_filament", 1)];
        let holdings = Holdings::Known(&held);
        let none = crate::shader_marks::Marks::default();
        let search = |query: &str| ShaderFilter {
            query: query.to_owned(),
            ..ShaderFilter::default()
        };
        let craft = craftable(&holdings, &[], &none);
        assert!(
            craft
                .iter()
                .all(|item| item.skin.id != "saber_kyber_filament")
        );
        let found = held_matches(&holdings, &search("kyber fil"), &none);
        assert_eq!(
            found.iter().map(|skin| skin.id).collect::<Vec<_>>(),
            ["saber_kyber_filament"]
        );
        assert!(held_matches(&holdings, &search(""), &none).is_empty());
        assert!(held_matches(&holdings, &search("aurora"), &none).is_empty());
        assert_eq!(
            held_note(&found),
            "Yours already, so not listed: Kyber Filament blade."
        );
        // "kyber" also lists the kyber shaders the player lacks; the held one is named.
        let kyber = held_matches(&holdings, &search("kyber"), &none);
        assert_eq!(kyber.len(), 1);
        assert!(
            craftable(&holdings, &[], &none)
                .iter()
                .any(|item| search("kyber").passes(item.skin, false, false))
        );
    }

    /// Collection marks (29/10/2026): favourites first, the marks on each item, and a
    /// locked duplicate never counted among all duplicates.
    #[test]
    fn favourites_lead_and_a_locked_duplicate_is_kept() {
        let held = [
            copy("saber_tidal", 3),
            copy("saber_sakura", 2),
            copy("saber_aurora", 1),
        ];
        let holdings = Holdings::Known(&held);
        let marks = crate::shader_marks::Marks::shown(
            &["saber_aurora"],
            &["saber_tidal"],
            &["saber_sakura"],
            Some("saber_aurora"),
        );
        let list = owned(&holdings, true, &marks);
        let ids: Vec<_> = list.iter().map(|item| item.skin.id).collect();
        assert_eq!(ids[0], "saber_aurora", "the favourite first: {ids:?}");
        let aurora = list[0];
        assert!(aurora.favourite && aurora.worn && aurora.last() && !aurora.locked);
        let tidal = list
            .iter()
            .find(|item| item.skin.id == "saber_tidal")
            .unwrap();
        assert!(tidal.locked && !tidal.new);
        assert!(
            list.iter()
                .any(|item| item.skin.id == "saber_sakura" && item.new)
        );
        // Sakura's one spare copy goes; Tidal's two stay.
        assert_eq!(duplicates(&list).0, 1);
        let craft = craftable(&holdings, &[], &marks);
        assert!(craft.iter().all(|item| !item.favourite), "Aurora is held");
        let wish = crate::shader_marks::Marks::shown(&["saber_phoenix"], &[], &[], None);
        let craft = craftable(&holdings, &[], &wish);
        assert_eq!(craft[0].skin.id, "saber_phoenix", "a wish leads Craft");
        assert!(craft[0].favourite);
    }

    /// A locked shader's Break down does nothing, nor does B when only locked duplicates
    /// are left; F and L ask the hub for the chosen shader, on a hub that keeps marks, and
    /// Shift+F shows the favourites alone.
    #[test]
    fn a_locked_shader_is_not_broken_down_and_the_keys_mark() {
        let held = [copy("saber_tidal", 3)];
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        panel.preview_marks = Some(crate::shader_marks::Marks::shown(
            &[],
            &["saber_tidal"],
            &[],
            None,
        ));
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert!(panel.forge.break_list[0].locked);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.armed, None, "nothing arms");
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.spare.0, 0);
        let _ = key(&mut panel, KeyCode::KeyB);
        assert_eq!(panel.forge.ask, None, "no duplicate may go");
        let press =
            |panel: &mut Panel, code: KeyCode, shift: bool| panel.forge_mark_key(code, shift);
        assert_eq!(
            press(&mut panel, KeyCode::KeyL, false),
            Some(PanelAction::Mark {
                id: "saber_tidal",
                mark: sjk_identity::Mark::Locked,
                on: false,
            })
        );
        assert_eq!(
            press(&mut panel, KeyCode::KeyF, false),
            Some(PanelAction::Mark {
                id: "saber_tidal",
                mark: sjk_identity::Mark::Favourite,
                on: true,
            })
        );
        assert_eq!(
            press(&mut panel, KeyCode::KeyF, true),
            Some(PanelAction::None)
        );
        assert!(panel.forge.break_filter.shaders.favourite);
        // Without the hub's marks, nothing to mark.
        panel.preview_marks = Some(crate::shader_marks::Marks::default());
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert_eq!(
            press(&mut panel, KeyCode::KeyL, false),
            Some(PanelAction::None)
        );
    }

    /// A purchase, a craft and a duplicate's break down ask twice and wait for the hub; a
    /// press is refused while the hub keeps the Forge closed or the purse falls short.
    #[test]
    fn trades_ask_twice_and_wait_for_the_hub() {
        let held = [copy("saber_tidal", 3)];
        let mut panel = forge(Section::Shop, &held, sample_data(300, 300, 3_600, true));
        assert!(panel.forge_shop_due(Instant::now()), "read once shown");
        assert!(!panel.forge_shop_due(Instant::now()));
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::Buy("saber_aurora".to_owned())
        );
        assert!(panel.forge.waiting());
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::None,
            "on its way"
        );
        // The Mythical is too dear: nothing to arm.
        panel.forge.sent = None;
        for _ in 0..3 {
            let _ = key(&mut panel, KeyCode::ArrowRight);
        }
        assert_eq!(panel.forge.shop_selected, 3);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.armed, None);
        // The Uncommon holocron for credits, past the offers.
        let _ = key(&mut panel, KeyCode::ArrowRight);
        let _ = key(&mut panel, KeyCode::Enter);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::BuyHolocron("uncommon".to_owned())
        );
        // Craft: the commonest first, crafted on the second press.
        let mut panel = forge(Section::Craft, &held, sample_data(300, 300, 3_600, true));
        let first = panel.forge.craft_list[0].skin.id;
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::Craft(first));
        let mut poor = forge(Section::Craft, &held, sample_data(0, 0, 3_600, true));
        assert_eq!(key(&mut poor, KeyCode::Enter), PanelAction::None);
        assert_eq!(poor.forge.armed, None, "too dear to arm");
        // Break down a duplicate twice; closed by the hub, nothing arms.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::Shatter("saber_tidal")
        );
        let closed = ForgeData {
            can_trade: false,
            ..sample_data(0, 0, 3_600, true)
        };
        let mut panel = forge(Section::Break, &held, closed);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.armed, None);
        assert!(!panel.forge_shop_due(Instant::now()));
    }

    /// The last copy and every duplicate ask with a card: the quieter choice has the focus
    /// for the last copy, Escape keeps it, the stronger choice sends it; a hub without the
    /// abilities lists no last copy and offers no Break down all.
    /// A banner broken down on the Banners tab (05/11/2026) is never shown as the answer
    /// to the Forge's last trade.
    #[test]
    fn a_banners_outcome_is_not_the_forges() {
        let held = [copy("saber_tidal", 3)];
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        panel.forge.sent = Some((0, Press::Break("saber_tidal")));
        let outcome = |id: &str| sjk_identity::TradeOutcome {
            serial: 1,
            trade: sjk_identity::Trade::Shatter,
            id: id.to_owned(),
            amount: 5,
            copies: 1,
            message: String::new(),
        };
        panel.forge.data.trade = Some(outcome("banner_jedi_archives"));
        assert!(panel.forge.answer().is_none());
        panel.forge.data.trade = Some(outcome("saber_tidal"));
        assert!(panel.forge.answer().is_some());
    }

    #[test]
    fn the_last_copy_and_all_duplicates_ask_with_a_card() {
        let held = [copy("saber_tidal", 3), copy("saber_aurora", 1)];
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        assert_eq!(panel.forge.break_list.len(), 2);
        let _ = key(&mut panel, KeyCode::ArrowDown);
        assert!(panel.forge.break_list[panel.forge.breaks].last());
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert!(matches!(
            panel.forge.ask,
            Some(Ask::Last { yes: false, .. })
        ));
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::None,
            "Keep it"
        );
        assert_eq!(panel.forge.ask, None);
        let _ = key(&mut panel, KeyCode::Enter);
        let _ = key(&mut panel, KeyCode::Tab);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::ShatterLast("saber_aurora")
        );
        // The hub's answer takes the blade off the saber, once.
        let mut done = sample_data(5, 0, 3_600, true);
        done.trade = Some(sjk_identity::TradeOutcome {
            serial: 1,
            trade: sjk_identity::Trade::ShatterLast,
            id: "saber_aurora".to_owned(),
            amount: 5,
            copies: 1,
            message: String::new(),
        });
        panel.preview_forge(Some(done));
        panel.set_forge(ForgeData::default());
        assert_eq!(panel.take_forge_unequip(), Some("saber_aurora"));
        assert_eq!(panel.take_forge_unequip(), None);
        panel.set_forge(ForgeData::default());
        assert_eq!(panel.take_forge_unequip(), None, "once a trade");
        // Every duplicate: B asks, Escape cancels, Enter (the focus on yes) sends.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::KeyB);
        assert_eq!(panel.forge.ask, Some(Ask::All { yes: true }));
        assert_eq!(key(&mut panel, KeyCode::Escape), PanelAction::None);
        assert_eq!(panel.forge.ask, None);
        let _ = key(&mut panel, KeyCode::KeyB);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::ShatterAll);
        // An older hub: duplicates only, no card.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, false));
        assert_eq!(panel.forge.break_list.len(), 1);
        let _ = key(&mut panel, KeyCode::KeyB);
        assert_eq!(panel.forge.ask, None);
    }

    /// Atrosex, 05/11/2026: with Select on (S), a click or Space picks shaders, Enter
    /// asks with the card and sends a copy of each at once; the hub's answer takes a last
    /// copy off the saber and ends the picking. Escape stops picking before it leaves; a
    /// locked shader is never picked, and a hub without `shatter_many` has no Select.
    #[test]
    fn several_shaders_are_picked_and_break_down_at_once() {
        let held = [
            copy("saber_tidal", 3),
            copy("saber_aurora", 1),
            copy("saber_circuit", 2),
        ];
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let tidal = panel
            .forge
            .break_list
            .iter()
            .position(|item| item.skin.id == "saber_tidal")
            .unwrap();
        let aurora = panel
            .forge
            .break_list
            .iter()
            .position(|item| item.skin.id == "saber_aurora")
            .unwrap();
        // Picking is off: a click only chooses.
        let _ = panel
            .forge
            .token(TILE_BASE + tidal as u16, true, Instant::now());
        assert!(panel.forge.picked.is_empty());
        let _ = key(&mut panel, KeyCode::KeyS);
        assert!(panel.forge.selecting);
        let _ = panel
            .forge
            .token(TILE_BASE + tidal as u16, true, Instant::now());
        panel.forge.breaks = aurora;
        let _ = key(&mut panel, KeyCode::Space);
        assert_eq!(
            panel.forge.picked,
            [("saber_tidal", 1), ("saber_aurora", 1)]
        );
        // A second click lets one go; a third picks it again.
        let _ = panel
            .forge
            .token(TILE_BASE + tidal as u16, true, Instant::now());
        assert_eq!(panel.forge.picked, [("saber_aurora", 1)]);
        let _ = panel
            .forge
            .token(TILE_BASE + tidal as u16, true, Instant::now());
        // + takes more of the chosen one's copies, up to all three; - gives one back.
        for _ in 0..5 {
            let _ = key(&mut panel, KeyCode::Equal);
        }
        assert_eq!(panel.forge.picked_copies("saber_tidal"), 3);
        let _ = key(&mut panel, KeyCode::Minus);
        let (picked, copies, gives, lasts) = panel.forge.picked_sum();
        assert_eq!((picked, copies, lasts), (2, 3, 1));
        assert!(gives > 0);
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert_eq!(
            panel.forge.picked,
            [("saber_aurora", 1), ("saber_tidal", 2)],
            "kept"
        );
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.ask, Some(Ask::Picked { yes: true }));
        assert_eq!(
            key(&mut panel, KeyCode::Escape),
            PanelAction::None,
            "Cancel"
        );
        assert_eq!(panel.forge.ask, None);
        let _ = key(&mut panel, KeyCode::Enter);
        assert_eq!(
            key(&mut panel, KeyCode::Enter),
            PanelAction::ShatterMany(vec![("saber_aurora", 1), ("saber_tidal", 2)], true)
        );
        let mut done = sample_data(5, 0, 3_600, true);
        done.trade = Some(sjk_identity::TradeOutcome {
            serial: 1,
            trade: sjk_identity::Trade::ShatterMany,
            id: String::new(),
            amount: 10,
            copies: 2,
            message: String::new(),
        });
        panel.preview_forge(Some(done));
        panel.set_forge(ForgeData::default());
        assert_eq!(panel.take_forge_unequip(), Some("saber_aurora"));
        assert_eq!(panel.take_forge_unequip(), None);
        assert!(!panel.forge.selecting && panel.forge.picked.is_empty());
        // Escape stops picking first; a second Escape leaves.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::KeyS);
        let _ = key(&mut panel, KeyCode::Space);
        assert_eq!(key(&mut panel, KeyCode::Escape), PanelAction::None);
        assert!(!panel.forge.selecting && panel.forge.picked.is_empty());
        assert_eq!(key(&mut panel, KeyCode::Escape), PanelAction::Close);
        // A locked shader is never picked.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::KeyS);
        let mut locked = panel.forge.break_list[tidal];
        locked.locked = true;
        panel.forge.break_list[tidal] = locked;
        let _ = panel
            .forge
            .token(TILE_BASE + tidal as u16, true, Instant::now());
        assert!(panel.forge.picked.is_empty());
        // A hub that keeps last copies: all but one at most.
        let keeps = ForgeData {
            last: false,
            ..sample_data(0, 0, 3_600, true)
        };
        let mut panel = forge(Section::Break, &held, keeps);
        let _ = key(&mut panel, KeyCode::KeyS);
        let tidal = panel
            .forge
            .break_list
            .iter()
            .position(|item| item.skin.id == "saber_tidal")
            .unwrap();
        panel.forge.breaks = tidal;
        for _ in 0..5 {
            let _ = key(&mut panel, KeyCode::Equal);
        }
        assert_eq!(panel.forge.picked_copies("saber_tidal"), 2);
        // + and - as typed come first, whatever key brings them (a Slovak layout's +).
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::KeyS);
        panel.forge.breaks = tidal;
        let _ = key(&mut panel, KeyCode::Space);
        let _ = panel
            .forge
            .key(Some("+"), KeyCode::Digit1, false, false, true);
        assert_eq!(panel.forge.picked_copies("saber_tidal"), 2);
        assert_eq!(panel.forge_section(), Section::Break, "not the Shop");
        let _ = panel
            .forge
            .key(Some("-"), KeyCode::Slash, false, false, true);
        assert_eq!(panel.forge.picked_copies("saber_tidal"), 1);
        assert!(!panel.forge.search_focus, "not the search");
        // An older hub: no Select.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, false));
        let _ = key(&mut panel, KeyCode::KeyS);
        assert!(!panel.forge.selecting);
    }

    /// Tab and the digits change section; the filter's chips and the search narrow the
    /// list.
    #[test]
    fn sections_filters_and_search() {
        let held = [copy("saber_tidal", 3)];
        let mut panel = forge(Section::Shop, &held, sample_data(40, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::Tab);
        assert_eq!(panel.forge_section(), Section::Craft);
        let _ = key(&mut panel, KeyCode::Digit3);
        assert_eq!(panel.forge_section(), Section::Break);
        let _ = key(&mut panel, KeyCode::Digit2);
        let _ = key(&mut panel, KeyCode::KeyA);
        assert!(panel.forge.craft_filter.only);
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert!(!panel.forge.craft_list.is_empty());
        assert!(panel.forge.craft_list.iter().all(|item| item.cost <= 40));
        let _ = key(&mut panel, KeyCode::Slash);
        assert!(panel.forge.search_focus);
        for _ in 0..3 {
            let _ = panel
                .forge
                .key(Some("z"), KeyCode::KeyZ, false, false, false);
        }
        assert_eq!(panel.forge.craft_filter.shaders.query, "zzz");
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert!(panel.forge.craft_list.is_empty());
        let _ = key(&mut panel, KeyCode::Escape);
        assert!(!panel.forge.search_focus);
        assert!(panel.forge.craft_filter.shaders.query.is_empty());
        assert_eq!(key(&mut panel, KeyCode::Escape), PanelAction::Close);
    }

    /// The first `n` shaders of the trade of `tier`, as the catalogue lists them.
    fn of_tier(tier: Rarity, n: usize) -> Vec<&'static str> {
        unlockables::blade_skins_by_tier()
            .iter()
            .filter(|skin| skin.tier == tier && crate::economy::in_trade(skin))
            .take(n)
            .map(|skin| skin.id)
            .collect()
    }

    /// Fuse (10/11/2026): shown only on a hub that fuses; the Forge proposes the exact
    /// count from the biggest piles, a click lets a copy go or picks one, the button asks
    /// twice and sends each shader once per copy, and the shader gained is revealed.
    #[test]
    fn duplicates_fuse_after_two_presses_and_the_gain_is_revealed() {
        let commons = of_tier(Rarity::Common, 2);
        let held = [copy(commons[0], 3), copy(commons[1], 2)];
        // An older hub: no Fuse section, 4 changes nothing, Tab skips it.
        let mut old = forge(Section::Break, &held, sample_data(0, 0, 3_600, false));
        assert_eq!(old.forge.sections().len(), 3);
        let _ = key(&mut old, KeyCode::Digit4);
        assert_eq!(old.forge_section(), Section::Break);
        let _ = key(&mut old, KeyCode::Tab);
        assert_eq!(old.forge_section(), Section::Shop);
        old.show_forge(Section::Fuse);
        build(&mut old, &held, [1920.0, 1080.0]);
        assert_eq!(old.forge_section(), Section::Shop, "Fuse gives way");
        // A hub that fuses three Commons.
        let mut panel = forge(Section::Break, &held, sample_data(0, 0, 3_600, true));
        let _ = key(&mut panel, KeyCode::Tab);
        assert_eq!(panel.forge_section(), Section::Fuse);
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert!(panel.fuse_ready, "three spare Commons fuse");
        assert_eq!(panel.forge.fuse_tier(), Rarity::Common);
        assert_eq!(panel.forge.fuse_list.len(), 2);
        assert_eq!(panel.forge.fuse_picks, [(commons[0], 2), (commons[1], 1)]);
        assert_eq!(panel.forge.fuse_check(), Ok(()));
        // A click on the first card lets its copies go: one short.
        let now = Instant::now();
        let _ = panel.forge.token(TILE_BASE, true, now);
        assert_eq!(panel.forge.fuse_picked(), 1);
        assert!(panel.forge.fuse_check().is_err());
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        assert_eq!(panel.forge.armed, None, "nothing to arm");
        // + picks a copy of the chosen one, never its last.
        let _ = panel
            .forge
            .key(Some("+"), KeyCode::Equal, false, false, false);
        let _ = panel
            .forge
            .key(Some("+"), KeyCode::Equal, false, false, false);
        let _ = panel
            .forge
            .key(Some("+"), KeyCode::Equal, false, false, false);
        assert_eq!(
            panel.forge.fuse_copies(commons[0]),
            2,
            "its last copy stays"
        );
        assert_eq!(panel.forge.fuse_picked(), 3);
        // P proposes again; Enter twice sends.
        let _ = key(&mut panel, KeyCode::KeyP);
        assert_eq!(panel.forge.fuse_picked(), 3);
        assert_eq!(key(&mut panel, KeyCode::Enter), PanelAction::None);
        let PanelAction::Fuse(ids) = key(&mut panel, KeyCode::Enter) else {
            panic!("the fusion goes on the second press");
        };
        assert_eq!(ids, [commons[0], commons[0], commons[1]]);
        assert!(panel.forge.waiting());
        // The hub's answer: the shader gained is revealed, and the picks come again.
        let gained = of_tier(Rarity::Uncommon, 1)[0];
        let serial = panel.forge.sent.map_or(0, |(asked, _)| asked) + 1;
        panel.preview_forge(Some(ForgeData {
            trade: Some(sjk_identity::TradeOutcome {
                serial,
                trade: sjk_identity::Trade::Fuse,
                id: gained.to_owned(),
                amount: 0,
                copies: 3,
                message: String::new(),
            }),
            ..sample_data(0, 0, 3_600, true)
        }));
        panel.set_forge(ForgeData::default());
        assert_eq!(panel.forge.fused.map(|skin| skin.id), Some(gained));
        assert!(panel.forge.fuse_stale);
        build(&mut panel, &held, [1920.0, 1080.0]);
        // T steps to the next tier the hub fuses: no Uncommon duplicates.
        let _ = key(&mut panel, KeyCode::KeyT);
        build(&mut panel, &held, [1920.0, 1080.0]);
        assert_eq!(panel.forge.fuse_tier(), Rarity::Uncommon);
        assert!(panel.forge.fused.is_none());
        assert!(panel.forge.fuse_list.is_empty());
        assert_eq!(
            panel.forge.fuse_check(),
            Err(crate::forge_fuse::Block::Short { have: 0, need: 3 })
        );
    }

    /// The wishlist (10/11/2026): an offer's toggle and Craft's W add a shader and take it
    /// off; a full list takes nothing more; a hub without one shows no toggle.
    #[test]
    fn the_wishlist_toggles_from_the_shop_and_craft() {
        let held = [copy("saber_tidal", 3)];
        let mut panel = forge(Section::Shop, &held, sample_data(300, 300, 3_600, true));
        let now = Instant::now();
        assert_eq!(
            panel.forge.token(OFFER_WISH_BASE, true, now),
            Some(PanelAction::Wish("saber_aurora", true))
        );
        let full = ForgeData {
            wishlist: Some(vec![
                "saber_phoenix".to_owned(),
                "saber_sakura".to_owned(),
                "saber_lavalamp".to_owned(),
            ]),
            ..sample_data(300, 300, 3_600, true)
        };
        let mut panel = forge(Section::Shop, &held, full);
        assert_eq!(
            panel.forge.token(OFFER_WISH_BASE, true, now),
            Some(PanelAction::None),
            "full"
        );
        assert_eq!(
            panel.forge.token(OFFER_WISH_BASE + 1, true, now),
            Some(PanelAction::Wish("saber_sakura", false)),
            "one on it comes off"
        );
        let mut craft = forge(Section::Craft, &held, sample_data(300, 300, 3_600, true));
        let first = craft.forge.craft_list[0].skin.id;
        assert_eq!(
            key(&mut craft, KeyCode::KeyW),
            PanelAction::Wish(first, true)
        );
        let mut old = forge(Section::Craft, &held, sample_data(300, 300, 3_600, false));
        assert_eq!(key(&mut old, KeyCode::KeyW), PanelAction::None);
        assert!(super::super::forge_view::wishlist_words(&[]).contains("empty"));
        let aurora = unlockables::find("saber_aurora").unwrap().name;
        assert!(
            super::super::forge_view::wishlist_words(&["saber_aurora".to_owned()])
                .contains(&format!("(1 of 3): {aurora}."))
        );
    }

    /// The Shop's dot (10/11/2026): a wished offer, or the deal within reach.
    #[test]
    fn the_shop_chip_carries_a_dot_when_an_offer_waits() {
        let held = [copy("saber_tidal", 3)];
        let quiet = forge(Section::Shop, &held, sample_data(300, 300, 3_600, true));
        assert!(!quiet.forge_shop_waits());
        let with = |deal: bool, wished: bool, shards: u64| {
            let mut data = sample_data(shards, 300, 3_600, true);
            if let Some(shop) = data.shop.as_mut() {
                shop.day.offers[3].deal = deal;
                shop.day.offers[3].price = 750;
                shop.day.offers[3].wished = wished;
            }
            forge(Section::Shop, &held, data)
        };
        assert!(with(true, false, 750).forge_shop_waits());
        assert!(!with(true, false, 749).forge_shop_waits(), "out of reach");
        assert!(with(false, true, 0).forge_shop_waits());
    }

    /// Every section and state fits the canvas in every window shape.
    #[test]
    fn every_section_fits() {
        let commons = of_tier(Rarity::Common, 6);
        let mut held = vec![copy("saber_tidal", 3), copy("saber_aurora", 1)];
        held.extend(commons.iter().map(|id| copy(id, 4)));
        // The deal, a wished offer and a Beta one, and a full wishlist of long names.
        let mut marked = sample_data(300, 300, 3_600, true);
        if let Some(shop) = marked.shop.as_mut() {
            shop.day.offers[0].deal = true;
            shop.day.offers[0].edition = Some("beta".to_owned());
            shop.day.offers[1].wished = true;
            shop.day.offers[2].edition = Some("beta".to_owned());
        }
        let mut longest: Vec<&'static Unlockable> = unlockables::blade_skins().collect();
        longest.sort_by_key(|skin| std::cmp::Reverse(skin.name.len()));
        marked.wishlist = Some(
            longest
                .iter()
                .take(3)
                .map(|skin| skin.id.to_owned())
                .collect(),
        );
        let held = held.as_slice();
        for viewport in [
            [1920.0, 1080.0],
            [3840.0, 2160.0],
            [1440.0, 1080.0],
            [2560.0, 1080.0],
        ] {
            for section in Section::ALL {
                for data in [
                    sample_data(300, 300, 3_600, true),
                    marked.clone(),
                    ForgeData {
                        closed: Closed::NoEconomy,
                        ..ForgeData::default()
                    },
                    ForgeData {
                        shop: None,
                        ..sample_data(0, 0, 0, false)
                    },
                ] {
                    let mut panel = forge(section, held, data);
                    build(&mut panel, held, viewport);
                    if section == Section::Fuse {
                        // The shader a fusion gave, revealed.
                        panel.forge.fused = unlockables::find(longest[0].id);
                        build(&mut panel, held, viewport);
                    }
                }
            }
        }
    }
}
