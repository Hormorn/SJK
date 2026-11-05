//! The Collection screen (`docs/sjk-ui.md`, Collection): what an SJK player collects,
//! under one row of tabs. Medals (given by the SJK team, hanging from a rail),
//! Achievements (a wall of medallions by category), Shaders (the saber's looks: the
//! stock blade and every blade skin, owned or locked), Toys (the Illuminate holocron)
//! Nameplates (ornaments for the plate over the player's head; none yet) and Cards (the
//! sealed cards opening holocrons brought, and the credits), and since 21/10/2026 the
//! Forge between them (the shop, crafting and breaking shaders down, [`forge`]). Every
//! tab shows its things on the left, each in its place from the start (the ones the
//! player lacks dimmed), and the one chosen up close on the right; on Shaders, Toys and
//! Nameplates that is the player's own model, on the menu map's stage or in a live
//! preview over a match ([`Backstage`]), wearing the shader chosen, a locked one too
//! (only on this screen: nothing is sent).
//!
//! In the SJK UI it is the main page's and the game menu's Collection, a screen of its
//! own ([`crate::profile_hub`]); in the classic menus the `collection`, `achievements`
//! and `unlockables` commands open it on its own. Like the Profile page it lives in the
//! console and has the SJK UI's look in every menu style ([`view`]).
//!
//! Keys: the arrows choose (on Achievements Up and Down move between the categories),
//! Tab and Shift+Tab walk the things, Enter or Space equips or unequips the shader
//! chosen or lights and puts out the Illuminate holocron, Ctrl+Tab changes tab (the
//! switch's, [`crate::profile_hub`]), Escape goes back. The pointer chooses by
//! hovering; a click on a button acts, on a tab shows it.

use crate::achievements::Standing;
use crate::menu_widgets::{BACK_TOKEN, MenuCanvas};
use crate::player_menu::ReturnTarget;
use crate::profile_hub::Tab;
use crate::unlockables::{self, Holdings};
use sjk_identity::Snapshot;
use sjk_ui::{InputEvent, UiEventKind};
use std::time::Instant;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[path = "collection_achievements.rs"]
mod achievements;
#[path = "collection_banners.rs"]
mod banners;
#[path = "collection_cards.rs"]
mod cards;
#[path = "collection_finishers.rs"]
mod finishers;
#[path = "collection_forge.rs"]
pub(crate) mod forge;
#[path = "collection_forge_view.rs"]
mod forge_view;
#[path = "collection_history.rs"]
pub(crate) mod history;
#[path = "collection_history_view.rs"]
mod history_view;
#[path = "collection_hover.rs"]
mod hover;
#[path = "collection_loadouts.rs"]
pub(crate) mod loadouts;
#[path = "collection_marks_view.rs"]
mod marks_view;
#[path = "collection_medals.rs"]
mod medals;
#[path = "collection_nameplates.rs"]
mod nameplates;
#[path = "collection_saber_lab_builder.rs"]
mod saber_lab_builder;
#[path = "collection_saber_lab_editor.rs"]
mod saber_lab_editor;
#[path = "collection_saber_lab.rs"]
mod saber_lab_tab;
#[path = "collection_shaders.rs"]
mod shaders;
#[path = "collection_spin.rs"]
mod spin;
#[path = "collection_swatch.rs"]
pub(crate) mod swatch;
#[path = "collection_tags.rs"]
mod tags;
#[path = "collection_toys.rs"]
mod toys;
#[path = "collection_view.rs"]
mod view;

pub(crate) use saber_lab_tab::{LAB_SABER_AREA, LabStage, LabView};
pub(crate) use spin::SaberTurn;
pub(crate) use toys::{COMMANDS as TOY_COMMANDS, options_on as toy_options, unseen as toys_unseen};
pub(crate) use view::{MODEL_AREA, SABER_AREA, row_labels};

/// The things' tokens: the medals, the achievements, the shaders' rows, the shader's
/// Equip or Unequip, the toy and its switch.
/// Draw commands the page holds a frame: twice the most a world shot found (`world_shot_forge.rs`,
/// `duel6_sjk_forge_drawn_swatches_fit`).
pub(crate) const DRAW_COMMANDS: usize = 4_096;
const MEDAL_BASE: u16 = 1_200;
/// The achievements' medallions (6000 to 6199, room to grow; at 1240 they ran into the
/// toys' tokens past 60) and the board's pages' switch.
const ACHIEVEMENT_BASE: u16 = 6_000;
const ACHIEVEMENT_PAGE_BASE: u16 = 6_200;
/// Most medallions the wall shows: the catalogue's, then the Toy Master's the player
/// found (`achievements::secret_kind`), up to the pages' switch after them.
const WALL_MAX: usize = (ACHIEVEMENT_PAGE_BASE - ACHIEVEMENT_BASE) as usize;
const _: () = assert!(crate::achievements::ALL.len() + 8 <= WALL_MAX);
/// The shaders' rows have a range of their own, 7000 to 7299, for a catalogue of up
/// to 299 skins (the stock blade is row 0).
const SHADER_BASE: u16 = 7_000;
const SHADER_LIMIT: u16 = 7_300;
/// The click every page plays for a press.
fn ui_cues_click() {
    crate::audio::ui_cues::post(crate::audio::ui_cues::Cue::Click);
}

const WEAR_TOKEN: u16 = 1_300;
const TOY_TOKEN: u16 = 1_310;
const TOY_SWITCH_TOKEN: u16 = 1_311;
/// The chosen toy's card's second switch: other players' toy seen or not.
const TOY_SIGHT_TOKEN: u16 = 1_305;
/// The chosen toy's card's own switches after it (the Hax's trap), one token each.
const TOY_OPTION_TOKEN: u16 = 1_390;
/// Most own switches a toy's card holds.
const TOY_OPTION_TOKENS: usize = 4;
/// The cardboard box's niche on Toys (Illuminate's is [`TOY_TOKEN`]).
const BOX_TOKEN: u16 = 1_314;
/// Order 66's niche on Toys.
const ORDER_TOKEN: u16 = 1_315;
/// The Hax's niche on Toys.
const HAX_TOKEN: u16 = 1_318;
/// The Ragdoll's niche on Toys.
const RAGDOLL_TOKEN: u16 = 1_309;
/// The Drone's niche on Toys.
const DRONE_TOKEN: u16 = 1_308;
/// The Boombox's niche on Toys.
const BOOMBOX_TOKEN: u16 = 1_307;
/// The Monitor's niche on Toys.
const MONITOR_TOKEN: u16 = 1_303;
/// Box hunt's niche on Toys.
const BOX_HUNT_TOKEN: u16 = 1_304;
/// The Banana peel's niche on Toys.
const BANANA_TOKEN: u16 = 1_375;
/// Fishing's niche on Toys.
const FISHING_TOKEN: u16 = 1_378;
/// Mining's niche on Toys (after the toy cards' own switches, [`TOY_OPTION_TOKEN`]).
const MINING_TOKEN: u16 = 1_394;
/// The Fishing card's Sell junk button: past the nameplate styles' and the toy
/// options' tokens, which a click reads by range.
const SELL_JUNK_TOKEN: u16 = 1_395;
/// Fishing's place among the toys (`collection_toys::TOYS`).
const FISHING_TOY: usize = 10;
/// Mining's, whose card sells rubble by the same button token.
const MINING_TOY: usize = 11;
/// The Toys tab's "?", the Toy Master's (`collection_toys::Panel::toy_master_badge`): its
/// hover card tells of the toys hidden on maps and names those the player found.
const TOY_MASTER_TOKEN: u16 = 1_376;
/// The Shaders tab's lone saber, where the pointer takes hold of it to turn it.
const SABER_TOKEN: u16 = 1_316;
/// The shader card's way to the Forge: Craft in the Forge (one not held) or Break down
/// in the Forge (a copy of the player's own).
const FORGE_LINK_TOKEN: u16 = 1_317;
/// The Nameplates tab's See and share nameplates switch.
const SHARE_TOKEN: u16 = 1_319;
/// The Nameplates tab's style choices, one per [`crate::hud::nameplate::Style`].
const STYLE_BASE: u16 = 1_380;
/// The target switch over the lone saber: the first saber (or half), the second (1_303,
/// 1_304, 1_307 and 1_308 are the Monitor's, Box hunt's, the Boombox's and the Drone's).
const TARGET_FIRST_TOKEN: u16 = 1_301;
const TARGET_SECOND_TOKEN: u16 = 1_302;
/// The "Both" box beside the switch: Equip and Unequip dress both sabers (both halves).
const BOTH_TOKEN: u16 = 1_306;
/// How far (frame pixels) a press on the lone saber may move and still be a click.
const CLICK_SLOP: f32 = 6.0;
/// The toys the Toys tab shows: Illuminate, the cardboard box, Order 66, the Hax, the
/// Ragdoll, the Drone, the Boombox, the Monitor, Box hunt, the Banana peel, Fishing,
/// then Mining.
const TOY_COUNT: usize = 12;
/// Each toy's row, in the order of [`Panel::toy`].
const TOY_TOKENS: [u16; TOY_COUNT] = [
    TOY_TOKEN,
    BOX_TOKEN,
    ORDER_TOKEN,
    HAX_TOKEN,
    RAGDOLL_TOKEN,
    DRONE_TOKEN,
    BOOMBOX_TOKEN,
    MONITOR_TOKEN,
    BOX_HUNT_TOKEN,
    BANANA_TOKEN,
    FISHING_TOKEN,
    MINING_TOKEN,
];
/// Each toy row's own switch (25/10/2026): it turns that toy on or off in place, a token
/// a toy from here on, below [`FAVOURITE_TOKEN`].
const TOY_SWITCH_BASE: u16 = 1_360;
const _: () = assert!(TOY_SWITCH_BASE + TOY_COUNT as u16 <= FAVOURITE_TOKEN);
const _: () = assert!(
    SELL_JUNK_TOKEN >= STYLE_BASE + crate::hud::nameplate::Style::ALL.len() as u16
        && SELL_JUNK_TOKEN >= TOY_OPTION_TOKEN + TOY_OPTION_TOKENS as u16
);
/// The chosen toy's key, which a click or K waits for a key to bind, and Order 66's
/// second (the key that executes the order).
const BIND_TOKEN: u16 = 1_377;
const BIND_SECOND_TOKEN: u16 = 1_379;
/// The chosen shader's star and padlock on the Shaders tab's card (29/10/2026, collection
/// marks), and Mark all seen beside the New count.
const FAVOURITE_TOKEN: u16 = 1_372;
const LOCK_TOKEN: u16 = 1_373;
const SEEN_ALL_TOKEN: u16 = 1_374;
/// The Shaders tab's view switches: the list and the wall of tiles.
const LIST_VIEW_TOKEN: u16 = 1_312;
const GRID_VIEW_TOKEN: u16 = 1_313;
/// The chosen shader's options (its parts the wearer may switch off), one each.
const OPTION_BASE: u16 = 1_320;
/// The Cards tab's Reveal button.
const REVEAL_TOKEN: u16 = 1_350;
const _: () = assert!(OPTION_BASE as usize + OPTION_TOKENS <= REVEAL_TOKEN as usize);
/// The Cards tab's tiles (19/10/2026, the binder): one token each in the list the filter
/// shows, up to [`CARD_TILES_MAX`]; then its filter's chips (All, Owned and Missing,
/// then the rarity, the kind and the finish, each cycling).
const CARD_BASE: u16 = 7_400;
pub(super) const CARD_TILES_MAX: usize = 600;
const CARD_SHOW_BASE: u16 = 8_000;
const CARD_RARITY_TOKEN: u16 = 8_003;
const CARD_KIND_TOKEN: u16 = 8_004;
const CARD_FINISH_TOKEN: u16 = 8_005;
/// The card looked at closely: the darkened window under it (a click closes it), and
/// the card up close, where a click opens it (20/10/2026).
const CARD_INSPECT_TOKEN: u16 = 8_006;
const CARD_CLOSE_UP_TOKEN: u16 = 8_007;
/// Under a blade card held, up close: the way to its shader, on the Forge's Craft while
/// the player lacks it (21/10/2026, where its copy crafts the shader for less), else on
/// the Shaders tab.
const CARD_SHADER_TOKEN: u16 = 8_008;
/// Under the card up close and looked at closely, when more than one finish is held: a
/// chip each for Normal, Holo and Solar, showing the card in it (01/11/2026).
const CARD_LOOK_BASE: u16 = 8_010;
const _: () = assert!(CARD_BASE as usize + CARD_TILES_MAX <= CARD_SHOW_BASE as usize);
/// The Tags tab's tiles, one to each tag of `tags::CATALOGUE` (27/10/2026), and its card's
/// Choose button.
const BANNER_BASE: u16 = 8_200;
const BANNER_WEAR: u16 = 8_299;
const TAG_BASE: u16 = 8_100;
const TAG_CHOOSE_TOKEN: u16 = 8_099;
/// The Tags tab's finish chips, Normal, Holo and Solar (31/10/2026).
const TAG_FINISH_BASE: u16 = 8_090;
/// The Tags tab card's Break down and Craft buttons (tag copies, 03/11/2026).
const TAG_BREAK_TOKEN: u16 = 8_096;
const TAG_CRAFT_TOKEN: u16 = 8_097;
const OPTION_TOKENS: usize =
    crate::blade_skin_file::extras::MAX_OPTIONS + crate::blade_extras::MAX_EXTRAS;
/// The Shaders tab's rows: the stock blade, then every blade skin, the rarest first
/// ([`unlockables::blade_skins_by_tier`]).
const SHADER_ROWS: usize = 1 + unlockables::ALL.len();
/// The rows the tab may hold: [`SHADER_ROWS`] and the Saber Lab's blade while the lab is
/// on ([`crate::saber_lab`]).
const SHADER_SLOTS: usize = SHADER_ROWS + 1;
const _: () = assert!(SHADER_BASE as usize + SHADER_SLOTS <= SHADER_LIMIT as usize);
/// The Shaders tab's filter (14/10/2026): its tier chips (All, then the five tiers, and
/// Unique's for a player who holds a Unique shader), the owned-only chip and the search
/// field.
const FILTER_TIER_BASE: u16 = 7_300;
const FILTER_OWNED_TOKEN: u16 = 7_310;
const _: () = assert!(
    FILTER_TIER_BASE as usize + STRIP_TIERS_WITH_UNIQUE.len() < FILTER_OWNED_TOKEN as usize
);
const SEARCH_TOKEN: u16 = 7_311;
/// The chroma-only chip (25/10/2026), and the list's column heads that sort it, one to
/// each [`ShaderSort`].
const FILTER_CHROMA_TOKEN: u16 = 7_312;
const SORT_BASE: u16 = 7_313;
const _: () = assert!(SORT_BASE as usize + ShaderSort::ALL.len() < CARD_BASE as usize);
/// The favourites-only chip (29/10/2026, collection marks).
const FILTER_FAVOURITE_TOKEN: u16 = 7_320;
const _: () =
    assert!(SORT_BASE as usize + ShaderSort::ALL.len() <= FILTER_FAVOURITE_TOKEN as usize);
const _: () = assert!(FILTER_FAVOURITE_TOKEN < CARD_BASE);
/// The most characters a search holds.
const SEARCH_MAX: usize = 24;
/// The tiers the Forge's chips filter by, after All, the commonest first, as its Craft
/// list stands. The Forge never lists Unique (no Unique shader is traded).
const FILTER_TIERS: [unlockables::Rarity; 5] = [
    unlockables::Rarity::Common,
    unlockables::Rarity::Uncommon,
    unlockables::Rarity::Rare,
    unlockables::Rarity::Legendary,
    unlockables::Rarity::Mythical,
];

/// The Shaders tab's tier strip (25/10/2026), after All: the rarest first, as the wall
/// stands, and Unique's first of all for a player who holds a Unique shader.
const STRIP_TIERS: [unlockables::Rarity; 5] = [
    unlockables::Rarity::Mythical,
    unlockables::Rarity::Legendary,
    unlockables::Rarity::Rare,
    unlockables::Rarity::Uncommon,
    unlockables::Rarity::Common,
];
const STRIP_TIERS_WITH_UNIQUE: [unlockables::Rarity; 6] = [
    unlockables::Rarity::Unique,
    unlockables::Rarity::Mythical,
    unlockables::Rarity::Legendary,
    unlockables::Rarity::Rare,
    unlockables::Rarity::Uncommon,
    unlockables::Rarity::Common,
];

/// What the Shaders tab lists: a tier, the shaders held only, the chromas only, the
/// favourites only (29/10/2026), and words of a name (or a tier) or a blade's number the
/// search names. The stock blade is always the first row.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ShaderFilter {
    pub(super) tier: Option<unlockables::Rarity>,
    pub(super) owned: bool,
    pub(super) chroma: bool,
    pub(super) favourite: bool,
    pub(super) query: String,
}

impl ShaderFilter {
    /// Whether anything is filtered out.
    pub(super) fn active(&self) -> bool {
        self.tier.is_some()
            || self.owned
            || self.chroma
            || self.favourite
            || !self.query.trim().is_empty()
    }

    /// Whether `skin` (held or not, a favourite or not) stays in the list.
    pub(super) fn passes(
        &self,
        skin: &unlockables::Unlockable,
        owned: bool,
        favourite: bool,
    ) -> bool {
        if self.tier.is_some_and(|tier| tier != skin.tier)
            || (self.owned && !owned)
            || (self.chroma && !skin.chroma)
            || (self.favourite && !favourite)
        {
            return false;
        }
        let query = self.query.trim().to_lowercase();
        // A number finds that blade alone ("42", "042" or "#42").
        let digits = query.strip_prefix('#').unwrap_or(&query);
        if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return digits.parse() == Ok(skin.number);
        }
        query.is_empty()
            || skin.name.to_lowercase().contains(&query)
            || skin.tier.label().to_lowercase().contains(&query)
            || crate::editions::name(skin.edition)
                .to_lowercase()
                .contains(&query)
            || skin.smith.to_lowercase().contains(&query)
    }
}

/// How the Shaders tab's list orders its rows (S, or a click on a column's head); the
/// wall of tiles always stands by tier. The blade numbers' order only while the numbers
/// show (`debug_collectibles`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ShaderSort {
    /// The rarest first, under a heading to each tier.
    #[default]
    Tier,
    /// By name.
    Name,
    /// The ones held first, then by tier.
    Yours,
    /// By blade number.
    Number,
}

impl ShaderSort {
    pub(super) const ALL: [Self; 4] = [Self::Tier, Self::Name, Self::Yours, Self::Number];

    /// The next order S gives; past Yours, the numbers' when they show, else Tier again.
    fn next(self, numbers: bool) -> Self {
        match self {
            Self::Tier => Self::Name,
            Self::Name => Self::Yours,
            Self::Yours if numbers => Self::Number,
            Self::Yours | Self::Number => Self::Tier,
        }
    }
}

/// Archived (25/10/2026, replacing `ui_shaderView`, whose 0 meant the list): how the
/// Shaders tab and the Forge show the shaders, 0 tiles (the wall, the Forge's grid of
/// cards), 1 the list.
pub(crate) const SHADER_VIEW_CVAR: &str = "ui_shaderList";

/// The blade skin of shader row `row` with no filter, for a player who holds no Unique
/// shader: `None` for the stock blade (row 0) or past the end. The page itself reads the
/// filtered list ([`Panel::skin_at`]).
#[cfg(test)]
fn shader_skin(row: usize) -> Option<&'static unlockables::Unlockable> {
    row.checked_sub(1)
        .and_then(|index| Holdings::Known(&[]).listed().nth(index))
}

/// What the console does after the page handled an event.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum PanelAction {
    None,
    Close,
    /// Set `cg_saberSkin` to this id (`""` for the stock blade).
    Wear(&'static str),
    /// Set `cg_saberFinisher` to this finisher's id (`""` for none, `crate::finishers`).
    WearFinisher(&'static str),
    /// Set `cg_saberSkin2` to this (`""`: the first saber's skin, `stock`, or an id).
    WearSecond(&'static str),
    /// Set `cg_saberSkin` to `first` and `cg_saberSkin2` to `second` (what the second wore,
    /// following the first): two sabers or a staff, the second keeping its look.
    WearBoth {
        first: &'static str,
        second: &'static str,
    },
    /// Light the Illuminate holocron (`true`) or put it out: the same as `toy_illuminate`,
    /// set rather than toggled.
    Illuminate(bool),
    /// Get in the cardboard box (`true`) or out: the same as `toy_box`, set rather than
    /// toggled.
    CardboardBox(bool),
    /// Turn Order 66 on (`true`) or off: the same as `toy_order66`, set rather than
    /// toggled.
    Order66(bool),
    /// Turn the Hax on (`true`) or off: the same as `toy_hax`, set rather than toggled.
    Hax(bool),
    /// Go limp (`true`) or get up: the same as `toy_ragdoll`, set rather than toggled.
    Ragdoll(bool),
    /// Turn the Drone on (`true`) or off: the same as `toy_drone`, set rather than toggled.
    Drone(bool),
    /// Carry the Boombox (`true`) or put it away: the same as `toy_boombox`, set rather
    /// than toggled.
    Boombox(bool),
    /// Turn the Monitor on (`true`) or off: the same as `toy_monitor`, set rather than
    /// toggled.
    Monitor(bool),
    /// Turn Box hunt on (`true`) or off: the same as `toy_boxhunt`, set rather than
    /// toggled.
    BoxHunt(bool),
    /// Turn the Banana peel on (`true`) or off: the same as `toy_banana`, set rather than
    /// toggled.
    Banana(bool),
    /// Get the fishing rod out (`true`) or put it away: the same as `toy_fishing`, set
    /// rather than toggled.
    Fishing(bool),
    /// Take the pickaxe in hand (`true`) or put it away: the same as `toy_mining`, set
    /// rather than toggled.
    Mining(bool),
    /// Sell the fished junk to the hub: the same as `fishing_sell`.
    SellJunk,
    /// Sell the dug rubble to the hub: the same as `mining_sell`.
    SellRubble,
    /// Set `cg_tag` to this tag's id: the one `toy_tag` sticks.
    /// Pick a tag (or a card) for `toy_tag`, in a finish (`cg_tag`, `cg_tagFinish`).
    ChooseTag(String, &'static str),
    /// Break one tag copy down into kyber shards (a hub whose economy says `tag_copies`).
    ShatterTag(sjk_identity::TagCopy),
    /// Give two tag copies of one tier and finish for a tag not held yet.
    CraftTags([sjk_identity::TagCopy; 2]),
    /// Ask the hub to equip an owned banner, empty to unequip.
    Banner(&'static str),
    /// Show a tab another page draws: the Holocrons page.
    Hub(Tab),
    /// Switch blade skin `skin`'s option `option` on or off (`cg_saberSkinOptions`).
    SkinOption {
        skin: &'static str,
        option: String,
        on: bool,
    },
    /// Set [`SHADER_VIEW_CVAR`]: the grid of cards (`true`) or the rack.
    ShaderView(bool),
    /// Break one duplicate of this shader down into kyber shards (`crate::economy`).
    Shatter(&'static str),
    /// Break this shader down, the last copy too (a hub that says `shatter_last`).
    ShatterLast(&'static str),
    /// Break every duplicate down, one copy of each kept.
    ShatterAll,
    /// Craft this shader with kyber shards.
    Craft(&'static str),
    /// Ask the hub to sell the player this offer of the day.
    Buy(String),
    /// Ask the hub to sell the player a holocron of this tier for credits.
    BuyHolocron(String),
    /// Ask the hub to reveal the player's sealed cards.
    RevealCards,
    /// See and share nameplates on (`true`) or off: `cg_nameplate`
    /// ([`crate::hud::nameplate::see_and_share`]).
    SeeAndShare(bool),
    /// The nameplates' look: `cg_nameplateStyle` ([`crate::hud::nameplate::Style`]).
    NameplateStyle(crate::hud::nameplate::Style),
    /// Other players' toy seen (`true`) or hidden on this PC: set the toy's `cvar`
    /// ([`crate::toy_sight`]).
    ToySight {
        cvar: &'static str,
        seen: bool,
    },
    /// Carry out what the Loadouts card asked: wear, save, rename or empty a slot
    /// ([`loadouts::Request`]).
    Loadout(loadouts::Request),
    /// One of the chosen toy's own switches on its card (the Hax's trap) on (`true`) or
    /// off: set its `cvar`.
    ToyOption {
        cvar: &'static str,
        on: bool,
    },
    /// Bind `key` (its console name) to a toy's `command`, in place of the first key it
    /// had, as the key-binding editor does.
    Bind {
        command: &'static str,
        key: String,
    },
    /// Clear the first key bound to a toy's command.
    Unbind(&'static str),
    /// Set (`on`) or clear a collection mark on this shader at the hub (29/10/2026): a
    /// star or a padlock.
    Mark {
        id: &'static str,
        mark: sjk_identity::Mark,
        on: bool,
    },
}

/// What the marks a frame shows were read from ([`Panel::follow_marks`]).
#[derive(Clone, Debug, PartialEq)]
struct MarksFrom {
    /// The identity's marks' and own profile's revisions.
    hub: Option<(u64, u64)>,
    /// The seen-list's revision.
    seen: u64,
    worn: [Option<&'static str>; 2],
    debug: bool,
    /// How many unlocks the holdings list (they change with the profile, and in tests).
    known: usize,
}

/// What a press of the star (`Favourite`) or the padlock (`Locked`) does on shader `id`
/// (`owned` whether the player holds it): set the mark it lacks, or clear the one it has;
/// nothing while the hub keeps no marks, nor a lock on a shader not held.
pub(super) fn mark_action(
    marks: &crate::shader_marks::Marks,
    id: &'static str,
    mark: sjk_identity::Mark,
    owned: bool,
) -> PanelAction {
    if !marks.kept {
        return PanelAction::None;
    }
    let on = match mark {
        sjk_identity::Mark::Favourite => !marks.favourite(id),
        sjk_identity::Mark::Locked => !marks.locked(id),
    };
    if on && mark == sjk_identity::Mark::Locked && !owned {
        return PanelAction::None;
    }
    ui_cues_click();
    PanelAction::Mark { id, mark, on }
}

/// Where the player's model stands behind Shaders, Toys and Nameplates this frame, as
/// the renderer says ([`crate::menu_stage`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum Backstage {
    /// Nowhere: the page draws the thing itself.
    #[default]
    None,
    /// On the menu map's stage, right of the page; `head` is where the top of its head
    /// is in the window, when known.
    World { head: Option<[f32; 2]> },
    /// In the live preview the page shows at [`MODEL_AREA`], once a frame of it is drawn.
    Preview { ready: bool },
}

/// The blade the model on the stage holds while the page shows it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PreviewSkin {
    /// What the player wears ([`crate::GpuState::local_saber_skin`]).
    Worn,
    /// The stock blade.
    Stock,
    /// This blade skin, owned or not: only drawn here, never sent.
    Skin(&'static str),
}

/// What the page wants the stage to show.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StageWish {
    pub(crate) skin: PreviewSkin,
    /// The saber on its own, turned so, in place of the model holding it (Shaders).
    pub(crate) saber: Option<SaberTurn>,
    /// The shader shown goes on the second saber (or the staff's other half), the first
    /// keeping what it wears.
    pub(crate) on_second: bool,
    /// The shader shown goes on both sabers (both halves): the "Both" box is ticked.
    pub(crate) on_both: bool,
    /// The Illuminate holocron floats by the model, lit.
    pub(crate) holocron: bool,
    /// The Saber Lab tab: the model holds the saber and swings it, the saber floats
    /// beside it too ([`crate::saber_lab`]).
    pub(crate) lab: Option<LabStage>,
}

/// What the page knows this frame.
pub(crate) struct Inputs<'a> {
    /// `cl_identity`.
    pub(crate) enabled: bool,
    pub(crate) snapshot: Option<&'a Snapshot>,
    /// The unlocks the player's own hub profile lists, or why they are not known.
    pub(crate) holdings: Holdings<'a>,
    /// `cg_saberSkin`.
    pub(crate) setting: &'a str,
    /// Every achievement with the player's progress.
    pub(crate) standings: &'a [Standing],
    /// The Illuminate holocron is lit now (the client's own state, kept in the console
    /// by the viewer each frame).
    pub(crate) illuminate: bool,
    /// The player is in the cardboard box now (kept as [`Self::illuminate`] is).
    pub(crate) cardboard_box: bool,
    /// The Order 66 toy is on now (kept as [`Self::illuminate`] is).
    pub(crate) order66: bool,
    /// The Hax toy is on now (kept as [`Self::illuminate`] is).
    pub(crate) hax: bool,
    /// The player is limp now (the Ragdoll, kept as [`Self::illuminate`] is).
    pub(crate) ragdoll: bool,
    /// The Drone toy is on now (kept as [`Self::illuminate`] is).
    pub(crate) drone: bool,
    /// The player carries the Boombox now (kept as [`Self::illuminate`] is).
    pub(crate) boombox: bool,
    /// The Monitor toy is on now (kept as [`Self::illuminate`] is).
    pub(crate) monitor: bool,
    /// Box hunt is on now (kept as [`Self::illuminate`] is).
    pub(crate) box_hunt: bool,
    /// The Banana peel toy is on now (kept as [`Self::illuminate`] is).
    pub(crate) banana: bool,
    /// The fishing rod is out now (kept as [`Self::illuminate`] is).
    pub(crate) fishing: bool,
    /// The pickaxe is in hand now (kept as [`Self::illuminate`] is).
    pub(crate) mining: bool,
    /// The Fishing card's catches and junk sale, as the viewer last gave them; none
    /// lists nothing.
    pub(crate) fishing_card: Option<&'a crate::fishing_sell::Card>,
    /// The Mining card's finds and rubble sale, the same way.
    pub(crate) mining_card: Option<&'a crate::fishing_sell::Card>,
    /// The `name` cvar, for the nameplate.
    pub(crate) name: &'a str,
    /// The stock blade's colour (`color1`), for its swatch.
    pub(crate) stock: sjk_ui::Color,
    /// [`SHADER_VIEW_CVAR`]: the Shaders tab shows the grid of cards.
    pub(crate) grid: bool,
    /// `cg_saberSkinOptions`: the skins' parts switched off.
    pub(crate) options: &'a str,
    /// `cg_saberSkin2`: what a second saber wears ([`crate::looks::SECOND_CVAR`]).
    pub(crate) second: &'a str,
    /// What the player holds that a second skin can go on (two sabers, a staff): the
    /// shader card offers its button, named for it.
    pub(crate) pair: crate::saber_skins::Pair,
    /// See and share nameplates (`cg_nameplate`): the player sees nameplates and shares
    /// their health, shield and Force.
    pub(crate) share: bool,
    /// The nameplates' look (`cg_nameplateStyle`), which the Nameplates tab draws and
    /// offers to change.
    pub(crate) nameplate_style: crate::hud::nameplate::Style,
    /// The keys bound to the toys' commands, as players read them ("F5", "F5 / K"), in
    /// the order of [`toys::COMMANDS`]; empty for none (or none known).
    pub(crate) toy_keys: &'a [String],
    /// The toys whose other players' are hidden on this PC, as bits in the Toys tab's
    /// order (each one's "See" switch off, [`crate::toy_sight`]).
    pub(crate) toys_unseen: u16,
    /// The toys' own switches on their cards that are on, as bits in the order of
    /// [`toys::options_on`] (the Hax's trap, then its trap while away).
    pub(crate) toy_options: u16,
    /// `cg_tag`: the tag `toy_tag` sticks.
    pub(crate) tag: &'a str,
    /// `cg_tagFinish`: the finish it sticks it in, when held.
    pub(crate) tag_finish: &'a str,
    /// `cg_saberFinisher`: the finisher worn, when held (`crate::finishers`).
    pub(crate) finisher: &'a str,
}

/// What the last frame showed of a shader's row, for keys and clicks.
#[derive(Clone, Copy, Debug, Default)]
struct ShaderRow {
    /// What Enter does on it: `None` for a locked one or one already worn by the stock
    /// row.
    wear: Option<&'static str>,
    /// What its "2nd saber" button sets `cg_saberSkin2` to (`None`: no button, for a locked
    /// one), and whether the second saber wears it now.
    second: Option<&'static str>,
    second_worn: bool,
    /// What Equip does on it with the "Both" box ticked: the first's and the second's
    /// settings (`None`: nothing, locked or already worn by both as the stock blade), and
    /// whether both wear it now.
    both: Option<(&'static str, &'static str)>,
    both_worn: bool,
}

pub(crate) struct Panel {
    open: bool,
    owns_console: bool,
    /// Shown as the SJK UI's Collection screen: its way back names where it returns.
    hub: bool,
    back: ReturnTarget,
    tab: Tab,
    ui: MenuCanvas,
    /// The thing chosen on each tab: a medal in the order shown, an achievement in the
    /// catalogue's order, a shader's row.
    medal: usize,
    achievement: usize,
    /// The achievements on the wall as the last frame laid them out, in its reading
    /// order (`collection_achievements.rs`): the arrows and Tab follow it.
    achievement_slots: Vec<achievements::Slot>,
    /// The achievements board's page on show.
    achievement_page: crate::achievements::Page,
    shader: usize,
    /// The toy chosen on Toys: 0 Illuminate, 1 the cardboard box, 2 Order 66, 3 the Hax,
    /// 4 the Ragdoll, 5 the Drone, 6 the Boombox, 7 the Monitor, 8 Box hunt, 9 the Banana
    /// peel, 10 Fishing, 11 Mining.
    toy: usize,
    /// The banner chosen on Banners, which banners the profile holds and the one worn.
    banner: usize,
    banner_owned: [bool; crate::profile_card::banner::ALL.len()],
    banner_current: &'static str,
    /// The tag chosen on Tags, in the wall's order (the catalogue's tags, then the
    /// cards), the things the last frame listed and what the player held of each (Enter
    /// picks only one held).
    tag: usize,
    tag_entries: Vec<tags::Entry>,
    tag_held: Vec<crate::tags::Held>,
    /// The wall's lines as the last frame laid them out, the first shown, and whether the
    /// next frame brings the choice into view (an arrow moved it).
    tag_lines: Vec<tags::Line>,
    tag_first: usize,
    tag_follow: bool,
    /// The finish shown on the card (a place in `cards::FINISHES`), the entry it was set
    /// for, and the tag in use (`cg_tag`) as the last frame read it.
    tag_finish: usize,
    tag_finish_for: Option<String>,
    tag_in_use: Option<String>,
    /// The tag copies tools as the last frame offered them, and the button pressed once
    /// that waits for its second press (03/11/2026).
    tag_tools: tags::Tools,
    tag_confirm: Option<tags::Confirm>,
    /// The player was in the cardboard box, as the last frame's inputs said.
    box_on: bool,
    /// Order 66 was on, as the last frame's inputs said.
    order66_on: bool,
    /// The Hax was on, as the last frame's inputs said.
    hax_on: bool,
    /// The player was limp, as the last frame's inputs said.
    ragdoll_on: bool,
    /// The Drone was on, as the last frame's inputs said.
    drone_on: bool,
    /// The Boombox was carried, as the last frame's inputs said.
    boombox_on: bool,
    /// The Monitor was on, as the last frame's inputs said.
    monitor_on: bool,
    /// Box hunt was on, as the last frame's inputs said.
    box_hunt_on: bool,
    /// The Banana peel was on, as the last frame's inputs said.
    banana_on: bool,
    /// The fishing rod was out, as the last frame's inputs said.
    fishing_on: bool,
    /// The pickaxe was in hand, as the last frame's inputs said.
    mining_on: bool,
    /// See and share nameplates was on, as the last frame's inputs said.
    share_on: bool,
    /// The toys (bits by the Toys tab's order) whose other players' are hidden here, as
    /// the last frame's inputs said.
    toys_unseen: u16,
    /// The toys' own switches that are on, as the last frame's inputs said.
    toy_options: u16,
    /// The Loadouts card (Shaders, Tags and Nameplates open it) and the slots it shows,
    /// as the console last read them ([`Self::set_loadouts`]).
    loadouts: loadouts::Card,
    loadout_slots: [Option<loadouts::Loadout>; loadouts::SLOTS],
    /// The tile chosen on Cards (in the list the filter shows), how many tiles the last
    /// frame's list held, and the first line of them shown (the grid scrolls a line at a
    /// time).
    card: usize,
    cards_shown: usize,
    card_first: usize,
    /// Whether the chosen tile is looked at closely, large over the darkened tab
    /// (`collection_cards.rs`, `card_inspect_view`).
    card_inspect: bool,
    /// The finish (its index) the chosen tile is shown in, for the tile it was chosen on
    /// (`self.card` then): its best held otherwise.
    card_look: Option<(usize, usize)>,
    /// The Cards tab's filter, and the tiles it showed last frame (for the hover cards).
    card_filter: crate::cards::binder::Filter,
    card_tiles: Vec<crate::cards::binder::Tile>,
    /// The shader of the blade card up close last frame, when one is held (its button
    /// goes there), whether the Forge crafts it for the player (the button goes to the
    /// Forge's Craft then), and the shader the Shaders tab turns to on its next frame.
    card_blade: Option<&'static unlockables::Unlockable>,
    card_blade_craft: bool,
    shader_focus: Option<&'static str>,
    /// What the Saber Lab tab asks of the stage, what a press on it holds, the style the
    /// stage's model holds its hilts in, its blade among the lab's
    /// ([`crate::saber_lab::blades`]) and the worn skin as the last frame's inputs said
    /// (`cg_saberSkin`, for its Equip).
    lab: LabStage,
    lab_drag: Option<saber_lab_tab::LabDrag>,
    lab_held: crate::player_menu::SaberStyle,
    lab_blade: usize,
    lab_setting: String,
    /// Whether the player may wear the blade the Saber Lab tab shows, as its last frame
    /// found.
    lab_wearable: bool,
    /// The Saber Lab's blade-skin editor while it is open, and why it could not open.
    lab_edit: Option<saber_lab_editor::LabEdit>,
    lab_edit_problem: Option<String>,
    /// The Saber Lab's builder, and whether the tab shows the stage's choices instead.
    lab_build: saber_lab_builder::LabBuild,
    lab_stage_view: bool,
    /// How many sealed copies the last frame offered to reveal (0: no Reveal button).
    revealable: u32,
    /// The identity's last reveal when Reveal was pressed: the line under the button
    /// says what the next one brought.
    reveal_asked: Option<u64>,
    /// The first of the Shaders tab's lines shown (its headings, lines of tiles or rows;
    /// it scrolls a line at a time), the lines as the last frame laid them out and how many
    /// of them it showed.
    shader_first: usize,
    shader_lines: Vec<shaders::Line>,
    shader_shown: usize,
    /// How the list orders its rows, and whether the blades' numbers showed last frame
    /// (their order is offered only then).
    shader_sort: ShaderSort,
    shader_numbers: bool,
    /// The Shaders tab shows the wall of tiles, as the last frame's inputs said (or a
    /// switch since); else the list. The Forge's grid of cards follows it too.
    grid: bool,
    /// What the Shaders tab dresses while the player holds two sabers or a staff: the
    /// second saber (or the staff's other half) when true, else the first. The target
    /// switch over the saber, or a click on a saber, chooses it.
    second_target: bool,
    /// The "Both" box: Equip and Unequip dress both sabers (both halves) at once. Kept for
    /// the session, unticked by choosing one side.
    both: bool,
    /// What the player holds that a second skin can go on, as last built.
    pair: crate::saber_skins::Pair,
    /// Where a press on the lone saber began, to tell a click (which chooses the saber
    /// it lands on) from a drag (which turns them).
    saber_press: Option<[f32; 2]>,
    /// The side changed: the next build chooses the row that side wears, so the card and
    /// the preview show what it has on rather than the other side's choice.
    jump_to_worn: bool,
    /// The chosen toy's key waiting to be bound: its command's place in the toy's
    /// commands (0 the switch's, 1 Order 66's order), until a key, Escape or a click.
    bind_wait: Option<usize>,
    /// How many medals the last frame showed.
    medals_shown: usize,
    shader_rows: [ShaderRow; SHADER_SLOTS],
    /// The shaders the tab lists now (the filter's), the stock blade's row 0 before them,
    /// and how many rows that makes; read again each frame.
    shader_list: Vec<&'static unlockables::Unlockable>,
    /// The ones of them the player holds, as the last frame's rows said (a lock needs one).
    shader_owned: Vec<&'static str>,
    shader_count: usize,
    /// The Shaders tab shows the finishers rather than the saber's shaders (its kinds'
    /// pills, `collection_finishers.rs`), and the finisher worn as last drawn.
    finishers_shown: bool,
    finisher_worn: Option<&'static str>,
    shader_filter: ShaderFilter,
    /// The Shaders tab offers Unique's chip: the player holds a Unique shader, as the last
    /// frame's holdings said ([`Panel::tier_chips`]).
    unique_chip: bool,
    /// The search field takes the typing.
    search_focus: bool,
    /// The chosen shader's options the last frame showed (its skin, the option's id and
    /// whether it is on), for clicks and the number keys.
    shader_options: Vec<(&'static str, String, bool)>,
    /// Where the model stands this frame.
    backstage: Backstage,
    /// The thing the pointer rests on, for its hover card.
    hover: crate::menu::sjk::hover::Hover,
    /// Where the pointer is (window pixels), while it is over the window.
    pointer: Option<[f32; 2]>,
    /// The cards answering the pointer on Cards, each eased in and out.
    tilt: crate::tile_shine::HoverEase,
    /// The Shaders tab's lone saber's turn, and the window pixels to a frame pixel the
    /// last frame was drawn at (a drag turns it by frame pixels).
    spin: spin::Spin,
    scale: f32,
    /// The Forge tab: its section, lists, filters and trades ([`forge`]).
    forge: forge::Forge,
    /// Where the chosen shader's way to the Forge leads, as the last frame drew it.
    forge_link: Option<(forge::Section, &'static str)>,
    /// Whether the hub's economy is known (the Shaders tab's way to the Forge shows then).
    trade_known: bool,
    /// What the Shaders tab and the Forge show on each shader (worn, favourite, locked,
    /// New; `crate::shader_marks`), and what it was read from, read again only when that
    /// changes.
    marks: crate::shader_marks::Marks,
    marks_from: Option<MarksFrom>,
    /// The shader chosen on Shaders or in the Forge while it was New: seen once it has
    /// stayed chosen a second (`shader_marks::Look`), not when a sweep passes over it.
    viewing: crate::shader_marks::Look,
    /// A click just chose a shader (on Shaders or a Forge tile): a New one is seen at once,
    /// its sticker gone, without waiting its second (Creyon, 31/10/2026).
    clicked_shader: bool,
    /// The count of the last refused change of a mark the console was told of.
    mark_refusal_seen: u64,
    /// The marks a world shot or a test shows in place of the live ones.
    #[cfg(test)]
    pub(crate) preview_marks: Option<crate::shader_marks::Marks>,
    /// How much wider than the display family the page's text face is drawn.
    widen: f32,
    epoch: Instant,
    /// The blade skins the hub's packs brought, the swatches' looks (`sjk_packs.rs`);
    /// taken again only when they change ([`Panel::follow_skins`]).
    pub(super) skins: std::sync::Arc<crate::saber_skins::LoadedSkins>,
    /// The History tab: its filter, rows and reads ([`history`]).
    history: history::History,
    /// What a world shot shows in place of the own profile's unlocks.
    #[cfg(test)]
    pub(crate) preview: Option<Vec<sjk_identity::Unlock>>,
    /// The time a world shot draws the swatches at, instead of the page's.
    #[cfg(test)]
    pub(crate) shot_seconds: Option<f32>,
    /// What a world shot shows in place of the own profile's cards and credits.
    #[cfg(test)]
    pub(crate) preview_cards: Option<(Vec<sjk_identity::CardStack>, u64, u64)>,
    /// What a world shot shows on the Forge in place of the live identity's.
    #[cfg(test)]
    pub(crate) preview_forge: Option<forge::ForgeData>,
    /// Whether a world shot shows the Shaders tab's way to the Forge.
    #[cfg(test)]
    trade_pin: Option<bool>,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel {
    pub(crate) fn new() -> Self {
        Self {
            open: false,
            owns_console: false,
            hub: false,
            back: ReturnTarget::MainMenu,
            tab: Tab::Medals,
            // #117 (Nib): the Forge's Craft grid on its Legendary and Mythical lines, its
            // fifteen swatches drawn (the menu pictures on, or before their live pictures are
            // ready), took up to about 2 100 commands; 2 048 dropped the last ones on some
            // frames, which flickered.
            // The Shaders tab's wall shows up to thirty tiles, each its name on two lines
            // and its state: 320 text runs leave the rest of the page its room.
            // The marks (29/10/2026) add a sticker and a pill to a tile at most.
            ui: MenuCanvas::with_capacities(400, 220, DRAW_COMMANDS),
            medal: 0,
            // The first frame chooses the one nearest to unlocking.
            achievement: usize::MAX,
            achievement_slots: Vec::with_capacity(crate::achievements::ALL.len()),
            achievement_page: crate::achievements::Page::General,
            shader: 0,
            toy: 0,
            banner: 0,
            banner_owned: [false; crate::profile_card::banner::ALL.len()],
            banner_current: "",
            tag: 0,
            tag_entries: Vec::new(),
            tag_held: Vec::new(),
            tag_lines: Vec::new(),
            tag_first: 0,
            tag_follow: false,
            tag_finish: 0,
            tag_finish_for: None,
            tag_tools: tags::Tools::default(),
            tag_confirm: None,
            tag_in_use: None,
            box_on: false,
            order66_on: false,
            hax_on: false,
            ragdoll_on: false,
            drone_on: false,
            boombox_on: false,
            monitor_on: false,
            box_hunt_on: false,
            banana_on: false,
            fishing_on: false,
            mining_on: false,
            share_on: false,
            toys_unseen: 0,
            toy_options: 0,
            loadouts: loadouts::Card::default(),
            loadout_slots: Default::default(),
            card: 0,
            cards_shown: 0,
            card_first: 0,
            card_inspect: false,
            card_look: None,
            card_filter: Default::default(),
            card_tiles: Vec::new(),
            card_blade: None,
            card_blade_craft: false,
            shader_focus: None,
            revealable: 0,
            reveal_asked: None,
            lab: LabStage::default(),
            lab_drag: None,
            lab_held: crate::player_menu::SaberStyle::Single,
            lab_blade: 0,
            lab_setting: String::new(),
            lab_wearable: false,
            lab_edit: None,
            lab_edit_problem: None,
            lab_build: saber_lab_builder::LabBuild::default(),
            lab_stage_view: false,
            shader_first: 0,
            shader_lines: Vec::new(),
            shader_shown: 0,
            shader_sort: ShaderSort::default(),
            shader_numbers: false,
            grid: false,
            second_target: false,
            both: false,
            pair: crate::saber_skins::Pair::None,
            saber_press: None,
            jump_to_worn: false,
            bind_wait: None,
            medals_shown: 0,
            shader_rows: [ShaderRow::default(); SHADER_SLOTS],
            // Until the first frame reads the holdings: what a player who holds nothing sees.
            shader_list: Holdings::IdentityOff.listed().collect(),
            shader_owned: Vec::new(),
            shader_count: 1 + Holdings::IdentityOff.listed_count(),
            finishers_shown: false,
            finisher_worn: None,
            shader_filter: ShaderFilter::default(),
            unique_chip: false,
            search_focus: false,
            shader_options: Vec::new(),
            backstage: Backstage::None,
            hover: Default::default(),
            pointer: None,
            tilt: crate::tile_shine::HoverEase::default(),
            spin: spin::Spin::default(),
            scale: 1.0,
            forge: forge::Forge::default(),
            forge_link: None,
            trade_known: false,
            marks: Default::default(),
            marks_from: None,
            viewing: crate::shader_marks::Look::default(),
            clicked_shader: false,
            mark_refusal_seen: 0,
            #[cfg(test)]
            preview_marks: None,
            widen: 1.0,
            epoch: Instant::now(),
            skins: Default::default(),
            history: Default::default(),
            #[cfg(test)]
            preview: None,
            #[cfg(test)]
            shot_seconds: None,
            #[cfg(test)]
            preview_cards: None,
            #[cfg(test)]
            preview_forge: None,
            #[cfg(test)]
            trade_pin: None,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// Take the loaded blade skins when a pack brought new ones (one atomic read a
    /// frame otherwise).
    pub(crate) fn follow_skins(&mut self) {
        if self.skins.generation() != crate::sjk_packs::generation() {
            self.skins = crate::sjk_packs::skins();
        }
        self.follow_lab_editor();
    }

    /// Show the page on `tab` (one of the Collection's), on its own (`hub` false: the
    /// classic menus' commands) or as the SJK UI's Collection screen returning to
    /// `back`; `owns_console` when the console was closed before it. The things chosen
    /// on each tab stay as they were.
    /// The loadouts kept, for the Loadouts card; `note` says what the last request did.
    pub(crate) fn set_loadouts(&mut self, store: &loadouts::Store, note: Option<&str>) {
        self.loadout_slots = store.slots.clone();
        if let Some(note) = note {
            self.loadouts.told(note);
        }
    }

    /// Open the Loadouts card with `store`'s slots, its second one chosen, for a shot.
    #[cfg(test)]
    pub(crate) fn open_loadouts_for_shot(&mut self, store: &loadouts::Store) {
        self.loadouts.open();
        self.set_loadouts(store, Some("Wearing Duels"));
    }

    /// Whether the tab on show offers the Loadouts card: those where a look is chosen.
    fn offers_loadouts(&self) -> bool {
        matches!(self.tab, Tab::Shaders | Tab::Tags | Tab::Nameplates)
    }

    /// What the Loadouts card answered, as the page's action.
    fn loadout_reply(reply: loadouts::Reply) -> PanelAction {
        match reply {
            loadouts::Reply::Ask(request) => PanelAction::Loadout(request),
            loadouts::Reply::None | loadouts::Reply::Closed => PanelAction::None,
        }
    }

    pub(crate) fn open(&mut self, tab: Tab, owns_console: bool, hub: bool, back: ReturnTarget) {
        if !self.open {
            // Shaders opens on what the player wears.
            self.shader = usize::MAX;
        }
        self.open = true;
        self.loadouts.close();
        self.owns_console = owns_console;
        self.hub = hub;
        self.back = back;
        self.show(tab);
    }

    /// Turn to `tab`, if it is one of the Collection's this page draws (Holocrons is
    /// the Holocrons page).
    pub(crate) fn show(&mut self, tab: Tab) {
        if tab.screen() == crate::profile_hub::Screen::Collection && tab != Tab::Holocrons {
            if tab != self.tab {
                self.leave_viewing();
                self.leave_search();
                self.forge.leave_search();
                self.forge.ask = None;
                self.card_inspect = false;
                self.bind_wait = None;
            }
            self.tab = tab;
        }
    }

    /// The tab on show (or shown last).
    pub(crate) fn tab(&self) -> Tab {
        self.tab
    }

    /// Whether the Toys tab's chosen toy is Fishing, whose card lists the catches.
    pub(crate) fn fishing_chosen(&self) -> bool {
        self.toy == FISHING_TOY
    }

    /// Whether the Toys tab's chosen toy is Mining, whose card lists the finds.
    pub(crate) fn mining_chosen(&self) -> bool {
        self.toy == MINING_TOY
    }

    /// Whether the page is the SJK UI's Collection screen.
    pub(crate) fn is_hub(&self) -> bool {
        self.hub
    }

    /// Where the Collection screen returns.
    pub(crate) fn back(&self) -> ReturnTarget {
        self.back
    }

    /// Hide the page; returns whether it had opened the console.
    pub(crate) fn close(&mut self) -> bool {
        self.leave_viewing();
        let owned = self.open && self.owns_console;
        // Shown again, the History tab reads the newest lines.
        self.history.shown = false;
        self.close_lab_editor();
        self.lab_build.close();
        self.open = false;
        self.owns_console = false;
        self.backstage = Backstage::None;
        owned
    }

    /// Choose the Saber Lab's blade on Shaders: the first row after the stock blade while
    /// the lab is on, shown whatever the filter.
    pub(crate) fn choose_saber_lab(&mut self) {
        if crate::saber_lab::active() {
            self.shader = 1;
            self.shader_first = 0;
        }
    }

    /// Where the model stands this frame.
    pub(crate) fn set_backstage(&mut self, backstage: Backstage) {
        self.backstage = backstage;
    }

    /// Show `marks` in place of the live ones (`None` lets go), for a world shot.
    #[cfg(test)]
    pub(crate) fn marks_for_shot(&mut self, marks: Option<crate::shader_marks::Marks>) {
        self.preview_marks = marks;
        self.marks_from = None;
    }

    /// Whether the hub's economy is known this frame: the Shaders tab's way to the Forge
    /// shows then.
    pub(crate) fn set_trade_known(&mut self, known: bool) {
        #[cfg(test)]
        let known = self.trade_pin.unwrap_or(known);
        self.trade_known = known;
    }

    /// Show the Shaders tab's way to the Forge (`known`) whatever the hub says, for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn trade_known_for_shot(&mut self, known: bool) {
        self.trade_pin = Some(known);
        self.trade_known = known;
    }

    /// What the stage should show while the page is open on Shaders, Toys or
    /// Nameplates.
    pub(crate) fn stage_wish(&self) -> Option<StageWish> {
        if !self.open {
            return None;
        }
        match self.tab {
            Tab::Shaders => Some(StageWish {
                skin: match self.shader {
                    0 => PreviewSkin::Stock,
                    row => self
                        .skin_at(row)
                        .map_or(PreviewSkin::Worn, |skin| PreviewSkin::Skin(skin.id)),
                },
                saber: Some(self.spin.turn()),
                on_second: self.second_target,
                on_both: self.both_sides(),
                holocron: false,
                lab: None,
            }),
            Tab::Toys => Some(StageWish {
                skin: PreviewSkin::Worn,
                saber: None,
                on_second: false,
                on_both: false,
                holocron: self.toy == 0,
                lab: None,
            }),
            Tab::Nameplates => Some(StageWish {
                skin: PreviewSkin::Worn,
                saber: None,
                on_second: false,
                on_both: false,
                holocron: false,
                lab: None,
            }),
            Tab::SaberLab => self.lab_wish(),
            _ => None,
        }
    }

    pub(crate) fn draw_list(&self) -> &sjk_ui::DrawList {
        self.ui.draw_list()
    }

    /// Whether the last frame ran out of room on the canvas.
    #[cfg(test)]
    pub(crate) fn overflowed(&self) -> bool {
        self.ui.overflowed()
    }

    /// What Enter (or a click on the button) does on the tab on show.
    /// The chosen toy's "See" switch: other players' toy hidden on this PC, or seen again.
    fn toy_sight(&self) -> PanelAction {
        let Some(cvar) = toys::sight_cvar(self.toy) else {
            return PanelAction::None;
        };
        PanelAction::ToySight {
            cvar,
            seen: self.toys_unseen & 1 << self.toy != 0,
        }
    }

    /// The chosen toy's own switch `index` on its card, the other way round.
    fn toy_option(&self, index: usize) -> PanelAction {
        let Some((cvar, bit)) = toys::option(self.toy, index) else {
            return PanelAction::None;
        };
        PanelAction::ToyOption {
            cvar,
            on: self.toy_options & 1 << bit == 0,
        }
    }

    fn activate(&mut self, illuminate: bool) -> PanelAction {
        match self.tab {
            Tab::Shaders => self
                .shader_rows
                .get(self.shader)
                .copied()
                .map_or(PanelAction::None, |row| self.wear_action(row)),
            Tab::Toys if self.toy == 1 => PanelAction::CardboardBox(!self.box_on),
            Tab::Toys if self.toy == 2 => PanelAction::Order66(!self.order66_on),
            Tab::Toys if self.toy == 3 => PanelAction::Hax(!self.hax_on),
            Tab::Toys if self.toy == 4 => PanelAction::Ragdoll(!self.ragdoll_on),
            Tab::Toys if self.toy == 5 => PanelAction::Drone(!self.drone_on),
            Tab::Toys if self.toy == 6 => PanelAction::Boombox(!self.boombox_on),
            Tab::Toys if self.toy == 7 => PanelAction::Monitor(!self.monitor_on),
            Tab::Toys if self.toy == 8 => PanelAction::BoxHunt(!self.box_hunt_on),
            Tab::Toys if self.toy == 9 => PanelAction::Banana(!self.banana_on),
            Tab::Toys if self.toy == FISHING_TOY => PanelAction::Fishing(!self.fishing_on),
            Tab::Toys if self.toy == MINING_TOY => PanelAction::Mining(!self.mining_on),
            Tab::Toys => PanelAction::Illuminate(!illuminate),
            Tab::Nameplates => PanelAction::SeeAndShare(!self.share_on),
            Tab::Tags => self.choose_tag(),
            Tab::Banners => self.wear_banner(),
            Tab::Cards => {
                self.card_inspect = self.cards_shown > 0;
                PanelAction::None
            }
            Tab::SaberLab => self.lab_equip(&self.lab_setting),
            _ => PanelAction::None,
        }
    }

    /// What Equip (or Enter) does on `row`: wear it on the target, the first saber or the
    /// second (or the staff's other half). While the second has no choice of its own it
    /// follows the first, so dressing the first dresses both, as `saberskin` does.
    fn wear_action(&self, row: ShaderRow) -> PanelAction {
        if self.both_sides() {
            return row
                .both
                .map_or(PanelAction::None, |(first, second)| PanelAction::WearBoth {
                    first,
                    second,
                });
        }
        if self.second_target {
            return row
                .second
                .map_or(PanelAction::None, PanelAction::WearSecond);
        }
        row.wear.map_or(PanelAction::None, PanelAction::Wear)
    }

    /// Choose the target saber (or half) the Shaders tab dresses: one side, so the "Both"
    /// box is unticked.
    fn set_target(&mut self, second: bool) -> PanelAction {
        let second = second && self.pair != crate::saber_skins::Pair::None;
        if second != self.second_target || self.both {
            self.jump_to_worn = true;
        }
        self.second_target = second;
        self.both = false;
        PanelAction::None
    }

    /// Tick or untick the "Both" box.
    fn toggle_both(&mut self) -> PanelAction {
        if self.pair != crate::saber_skins::Pair::None {
            self.both = !self.both;
            self.jump_to_worn = true;
        }
        PanelAction::None
    }

    /// Equip dresses both sabers (both halves): the box is ticked and they are held.
    fn both_sides(&self) -> bool {
        self.both && self.pair != crate::saber_skins::Pair::None
    }

    /// How many things the tab on show offers to choose.
    fn things(&self) -> usize {
        match self.tab {
            Tab::Medals => self.medals_shown,
            Tab::Achievements => crate::achievements::ALL.len(),
            Tab::Shaders => self.shader_count,
            Tab::Cards => self.cards_shown,
            Tab::Toys => TOY_COUNT,
            Tab::Tags => self.tag_entries.len().max(1),
            Tab::Banners => crate::profile_card::banner::ALL.len(),
            _ => 0,
        }
    }

    /// The chosen thing of the tab on show, by its index.
    fn chosen_mut(&mut self) -> Option<&mut usize> {
        match self.tab {
            Tab::Medals => Some(&mut self.medal),
            Tab::Achievements => Some(&mut self.achievement),
            Tab::Shaders => Some(&mut self.shader),
            Tab::Cards => Some(&mut self.card),
            Tab::Toys => Some(&mut self.toy),
            Tab::Tags => Some(&mut self.tag),
            Tab::Banners => Some(&mut self.banner),
            _ => None,
        }
    }

    /// Show the achievements board's `page`, its nearest to unlocking chosen.
    fn set_achievement_page(&mut self, page: crate::achievements::Page) {
        if self.achievement_page != page {
            self.achievement_page = page;
            // Not on the new page's wall: the board chooses afresh.
            self.achievement = usize::MAX;
        }
    }

    /// Move the choice `by` things: wrapping round for Tab, stopping at the ends for
    /// the arrows.
    fn step(&mut self, by: isize, wrap: bool) {
        if self.tab == Tab::Achievements && !self.achievement_slots.is_empty() {
            self.achievement =
                achievements::sideways(&self.achievement_slots, self.achievement, by, wrap);
            return;
        }
        let count = self.things() as isize;
        let Some(chosen) = self.chosen_mut() else {
            return;
        };
        if count == 0 {
            return;
        }
        let at = (*chosen as isize).min(count - 1);
        let next = if wrap {
            (at + by).rem_euclid(count)
        } else {
            (at + by).clamp(0, count - 1)
        };
        *chosen = next as usize;
    }

    /// A key; `illuminate` is whether the holocron is lit.
    pub(crate) fn handle_key(
        &mut self,
        event: &KeyEvent,
        shift: bool,
        control: bool,
        illuminate: bool,
    ) -> PanelAction {
        if event.state != ElementState::Pressed {
            return PanelAction::None;
        }
        let PhysicalKey::Code(key) = event.physical_key else {
            return PanelAction::None;
        };
        self.hover.keyed();
        if self.loadouts.is_open() {
            let reply = self.loadouts.key(
                key,
                event.text.as_deref(),
                shift,
                control,
                &self.loadout_slots,
            );
            return Self::loadout_reply(reply);
        }
        if key == KeyCode::KeyP
            && !control
            && self.offers_loadouts()
            && !self.search_focus
            && self.bind_wait.is_none()
        {
            self.loadouts.open();
            return PanelAction::None;
        }
        if self.tab == Tab::Forge {
            if !control
                && !self.forge.search_focus
                && self.forge.ask.is_none()
                && let Some(action) = self.forge_mark_key(key, shift)
            {
                return action;
            }
            let action = self
                .forge
                .key(event.text.as_deref(), key, shift, control, self.grid);
            if let PanelAction::ShaderView(grid) = action {
                return self.set_grid(grid);
            }
            return action;
        }
        if self.tab == Tab::History
            && !control
            && let Some(action) = self.history_key(key, shift)
        {
            return action;
        }
        if self.tab == Tab::Cards && self.card_inspect {
            return self.key(key, shift, illuminate);
        }
        if self.tab == Tab::Cards && !control {
            let faces = crate::cards::faces::current();
            let filter = &mut self.card_filter;
            match key {
                KeyCode::KeyO => filter.cycle_show(shift),
                KeyCode::KeyT => filter.cycle_rarity(shift),
                KeyCode::KeyK => filter.cycle_kind(&faces, shift),
                KeyCode::KeyF => filter.cycle_finish(shift),
                KeyCode::KeyV => {
                    self.cycle_card_look(shift);
                    return PanelAction::None;
                }
                KeyCode::KeyI => return self.activate(illuminate),
                _ => return self.key(key, shift, illuminate),
            }
            return self.card_filter_changed();
        }
        if self.tab == Tab::Toys
            && let Some(slot) = self.bind_wait
        {
            let name = crate::input::keys::key_name(event).map(|name| name.as_str().to_owned());
            return self.bind_key(key, name, slot);
        }
        if self.tab == Tab::Shaders {
            if self.search_focus {
                return self.search_key(event, key, shift, control);
            }
            match key {
                KeyCode::Slash => return self.focus_search(),
                KeyCode::KeyF if control => return self.focus_search(),
                KeyCode::KeyF if shift => return self.toggle_favourites_only(),
                KeyCode::KeyF => return self.toggle_mark(sjk_identity::Mark::Favourite),
                KeyCode::KeyL if !control => {
                    return self.toggle_mark(sjk_identity::Mark::Locked);
                }
                KeyCode::KeyN if !control => return self.see_all(),
                KeyCode::KeyT => return self.cycle_tier(shift),
                KeyCode::KeyO => return self.toggle_owned(),
                KeyCode::KeyC if !control => return self.toggle_chroma(),
                KeyCode::KeyS if !self.grid && !control => {
                    return self.sort_by(self.shader_sort.next(self.shader_numbers));
                }
                _ => {}
            }
        }
        if self.tab == Tab::SaberLab
            && let Some(action) = self
                .lab_edit_key(event, key, shift, control)
                .or_else(|| self.lab_build_key(event, key, shift, control))
                .or_else(|| self.lab_key(key))
        {
            return action;
        }
        self.key(key, shift, illuminate)
    }

    /// A key (`name`: its console name, when it has one) while the chosen toy's key waits
    /// to be bound (`slot`: which of its commands): Escape lets go, Backspace or Delete
    /// clears the key, any other binds it. A key the key-binding editor keeps locked
    /// (Escape's, the console's) is never bound
    /// ([`crate::console::ViewerConsole::rebind_action`] refuses it).
    fn bind_key(&mut self, key: KeyCode, name: Option<String>, slot: usize) -> PanelAction {
        self.bind_wait = None;
        let Some(command) = toys::command(self.toy, slot) else {
            return PanelAction::None;
        };
        match key {
            KeyCode::Escape => PanelAction::None,
            KeyCode::Backspace | KeyCode::Delete => PanelAction::Unbind(command),
            _ => name.map_or(PanelAction::None, |key| PanelAction::Bind { command, key }),
        }
    }

    /// The skin of shader row `row` in the list the tab shows: `None` for the stock blade
    /// (row 0) or past the end.
    fn skin_at(&self, row: usize) -> Option<&'static unlockables::Unlockable> {
        row.checked_sub(1)
            .and_then(|index| self.shader_list.get(index))
            .copied()
    }

    /// The list changed: back to the top, and the rows read again next frame.
    fn filter_changed(&mut self) -> PanelAction {
        self.shader = 0;
        self.shader_first = 0;
        PanelAction::None
    }

    /// The list or the wall is about to stand in another order: the shader chosen stays
    /// chosen, wherever its row goes (the stock blade is row 0 in every order).
    fn keep_chosen(&mut self) {
        self.shader_focus = self.skin_at(self.shader).map(|skin| skin.id);
        if self.shader_focus.is_none() {
            self.shader = 0;
        }
        self.shader_first = 0;
    }

    /// Order the list the next way (S), or as the column head `sort` asks.
    fn sort_by(&mut self, sort: ShaderSort) -> PanelAction {
        if self.shader_sort != sort {
            self.keep_chosen();
            self.shader_sort = sort;
        }
        PanelAction::None
    }

    /// Show only the chromas, or all again.
    fn toggle_chroma(&mut self) -> PanelAction {
        self.shader_filter.chroma = !self.shader_filter.chroma;
        self.filter_changed()
    }

    /// The Cards tab's filter changed: back to its first tile.
    /// Go `by` tiles on from the card looked at closely, stopping at the ends; the grid
    /// follows it when it is put back.
    fn inspect_step(&mut self, by: isize) {
        let last = self.cards_shown.saturating_sub(1) as isize;
        self.card = (self.card as isize + by).clamp(0, last) as usize;
    }

    /// How many finishes the chosen tile holds copies in.
    pub(super) fn chosen_finishes_held(&self) -> usize {
        self.card_tiles
            .get(self.card)
            .map_or(0, |tile| tile.finishes_held().len())
    }

    /// The finish the chosen tile is shown in, when one was chosen for it.
    fn card_look_for(&self) -> Option<usize> {
        self.card_look
            .filter(|(card, _)| *card == self.card)
            .map(|(_, finish)| finish)
    }

    /// Show the chosen tile in the next finish held (the previous with `back`), round.
    fn cycle_card_look(&mut self, back: bool) {
        let Some(tile) = self.card_tiles.get(self.card) else {
            return;
        };
        let held = tile.finishes_held();
        if held.len() < 2 {
            return;
        }
        let now = tile
            .in_finish(self.card_look_for())
            .map_or(0, |stack| stack.finish.index);
        let at = held.iter().position(|finish| *finish == now).unwrap_or(0);
        let next = if back {
            (at + held.len() - 1) % held.len()
        } else {
            (at + 1) % held.len()
        };
        self.card_look = Some((self.card, held[next]));
    }

    fn card_filter_changed(&mut self) -> PanelAction {
        self.card = 0;
        self.card_first = 0;
        PanelAction::None
    }

    /// The search field lets go of the typing (what it holds stays).
    fn leave_search(&mut self) {
        if self.search_focus {
            self.search_focus = false;
            crate::text_entry::deselect();
        }
    }

    /// Put the typing in the search field (`/` or Ctrl+F).
    fn focus_search(&mut self) -> PanelAction {
        self.search_focus = true;
        crate::text_entry::select_all(&self.shader_filter.query);
        PanelAction::None
    }

    /// The tiers the Shaders tab's strip offers after All: [`STRIP_TIERS`], and Unique's
    /// first only for a player who holds a Unique shader.
    fn tier_chips(&self) -> &'static [unlockables::Rarity] {
        if self.unique_chip {
            &STRIP_TIERS_WITH_UNIQUE
        } else {
            &STRIP_TIERS
        }
    }

    /// Show one tier of shaders: the next chip (the previous with Shift), All after the
    /// last.
    fn cycle_tier(&mut self, back: bool) -> PanelAction {
        let tiers = self.tier_chips();
        let at = self
            .shader_filter
            .tier
            .and_then(|tier| tiers.iter().position(|&t| t == tier))
            .map_or(0, |index| index + 1);
        let count = tiers.len() + 1;
        let next = if back {
            (at + count - 1) % count
        } else {
            (at + 1) % count
        };
        self.shader_filter.tier = next.checked_sub(1).map(|index| tiers[index]);
        self.filter_changed()
    }

    /// Show only the shaders held, or all again.
    fn toggle_owned(&mut self) -> PanelAction {
        self.shader_filter.owned = !self.shader_filter.owned;
        self.filter_changed()
    }

    /// Show only the favourites, or all again (Shift+F); only while the hub keeps marks.
    fn toggle_favourites_only(&mut self) -> PanelAction {
        if !self.marks.kept && !self.shader_filter.favourite {
            return PanelAction::None;
        }
        self.shader_filter.favourite = !self.shader_filter.favourite;
        self.filter_changed()
    }

    /// Star or unstar (F), lock or unlock (L) the shader chosen on the Shaders tab: a blade
    /// skin, and only one the player holds for a lock, while the hub keeps marks.
    fn toggle_mark(&mut self, mark: sjk_identity::Mark) -> PanelAction {
        let Some(skin) = self.skin_at(self.shader) else {
            return PanelAction::None;
        };
        let owned = self.shader_list_owned(skin.id);
        mark_action(&self.marks, skin.id, mark, owned)
    }

    /// Whether the player holds shader `id`, as the last frame's rows said.
    fn shader_list_owned(&self, id: &'static str) -> bool {
        self.shader_owned.contains(&id)
    }

    /// Mark every New shader seen (N, or Mark all seen).
    fn see_all(&mut self) -> PanelAction {
        if self.marks.new_count() > 0 {
            crate::shader_marks::see_all(self.marks.new_ids());
            ui_cues_click();
        }
        PanelAction::None
    }

    /// The marks for `inputs` this frame: read again only when what they come from
    /// changed (the hub's marks, the own profile, the seen-list, the blades worn).
    fn follow_marks(&mut self, inputs: &Inputs<'_>) {
        #[cfg(test)]
        if let Some(marks) = &self.preview_marks {
            self.marks = marks.clone();
            return;
        }
        let owned = match inputs.holdings {
            Holdings::Known(owned) => owned,
            _ => &[],
        };
        let worn = crate::shader_marks::worn_of(inputs.setting, inputs.second, owned);
        let snapshot = inputs.snapshot.filter(|_| inputs.enabled);
        let from = MarksFrom {
            hub: snapshot.map(|snapshot| (snapshot.marks_revision, snapshot.me_revision)),
            seen: crate::shader_marks::revision(),
            worn,
            debug: inputs.holdings.debug(),
            known: owned.len(),
        };
        if self.marks_from.as_ref() == Some(&from) {
            return;
        }
        let kept = snapshot.is_some_and(|snapshot| {
            snapshot
                .economy
                .as_ref()
                .is_some_and(|economy| economy.can(sjk_identity::MARKS_ABILITY))
        });
        self.marks = crate::shader_marks::with_seen(|seen| {
            crate::shader_marks::Marks::of(
                snapshot.and_then(|snapshot| snapshot.marks.as_ref()),
                kept,
                owned,
                seen,
                from.debug,
                worn,
            )
        });
        self.marks_from = Some(from);
    }

    /// The shader chosen on Shaders or in the Forge is now `chosen` (each frame): a New one
    /// is seen once it has stayed chosen a second.
    fn view(&mut self, chosen: Option<&'static str>) {
        let new = chosen.is_some_and(|id| self.marks.is_new(id));
        if std::mem::take(&mut self.clicked_shader)
            && let Some(id) = chosen.filter(|_| new)
        {
            crate::shader_marks::see(id);
            return;
        }
        if let Some(id) = self.viewing.observe(chosen, new, self.look_clock()) {
            crate::shader_marks::see(id);
        }
    }

    /// The milliseconds the New marks' second is counted on ([`Self::view`]).
    fn look_clock(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Why the hub refused the last change of a mark, once, when it is new since the last
    /// call (the console says it).
    pub(crate) fn take_mark_refusal(
        &mut self,
        snapshot: Option<&sjk_identity::Snapshot>,
    ) -> Option<String> {
        let (serial, words) = snapshot?.mark_refusal.as_ref()?;
        if *serial <= self.mark_refusal_seen {
            return None;
        }
        self.mark_refusal_seen = *serial;
        Some(words.clone())
    }

    /// The choice left the page (another tab, the page closed): the shader it was on is
    /// seen only if it had stayed chosen a second.
    fn leave_viewing(&mut self) {
        if let Some(id) = self.viewing.leave(self.look_clock()) {
            crate::shader_marks::see(id);
        }
    }

    /// A key while the search field has the typing: Escape or Enter leave it (Escape
    /// first clears what was typed, as the other fields do), the rest edit it.
    fn search_key(
        &mut self,
        event: &KeyEvent,
        key: KeyCode,
        shift: bool,
        control: bool,
    ) -> PanelAction {
        match key {
            KeyCode::Escape => {
                self.leave_search();
                if !self.shader_filter.query.is_empty() {
                    self.shader_filter.query.clear();
                    return self.filter_changed();
                }
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Tab => self.leave_search(),
            KeyCode::ArrowUp | KeyCode::ArrowDown => {
                return self.key(key, shift, false);
            }
            _ => {
                if crate::text_entry::edit(
                    &mut self.shader_filter.query,
                    key,
                    event.text.as_deref(),
                    shift,
                    control,
                    SEARCH_MAX,
                    true,
                ) {
                    return self.filter_changed();
                }
            }
        }
        PanelAction::None
    }

    /// Switch the Shaders tab to the wall of tiles (`true`) or the list, the shader chosen
    /// kept (the list may stand in another order).
    fn set_grid(&mut self, grid: bool) -> PanelAction {
        if self.grid != grid && self.tab == Tab::Shaders {
            self.keep_chosen();
        }
        self.grid = grid;
        PanelAction::ShaderView(grid)
    }

    /// A pressed key, by its code.
    fn key(&mut self, key: KeyCode, shift: bool, illuminate: bool) -> PanelAction {
        // The finishers page has no rows to step: Escape and Tab only.
        if self.tab == Tab::Shaders && self.finishers_shown {
            return match key {
                KeyCode::Escape | KeyCode::Tab => self.key_any(key, shift, illuminate, false),
                _ => PanelAction::None,
            };
        }
        let vertical = self.tab == Tab::Shaders;
        if self.tab == Tab::Tags && self.tag_key(key) {
            return PanelAction::None;
        }
        if self.tab == Tab::Cards && self.card_inspect {
            // Looking at a card closely: the arrows go to the previous or next tile,
            // Escape, Enter, Space and I put it back.
            match key {
                KeyCode::ArrowLeft | KeyCode::ArrowUp | KeyCode::PageUp => self.inspect_step(-1),
                KeyCode::ArrowRight | KeyCode::ArrowDown | KeyCode::PageDown => {
                    self.inspect_step(1);
                }
                KeyCode::Home => self.card = 0,
                KeyCode::End => self.card = self.cards_shown.saturating_sub(1),
                KeyCode::KeyV => self.cycle_card_look(shift),
                KeyCode::Escape
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Space
                | KeyCode::KeyI => self.card_inspect = false,
                _ => {}
            }
            return PanelAction::None;
        }
        if self.tab == Tab::Cards && self.cards_shown > 0 {
            // The binder: Left and Right step a tile, Up and Down a line, stopping at the
            // ends (a short last line takes Down to its last tile); Page Up and Page Down
            // a screen of lines, Home and End to the first and last tile.
            let columns = cards::COLUMNS;
            let page = columns * cards::LINES;
            let last = self.cards_shown - 1;
            let at = self.card.min(last);
            match key {
                KeyCode::ArrowLeft => self.card = at.saturating_sub(1),
                KeyCode::ArrowRight => self.card = (at + 1).min(last),
                KeyCode::ArrowUp => self.card = at.checked_sub(columns).unwrap_or(at),
                KeyCode::ArrowDown if at / columns < last / columns => {
                    self.card = (at + columns).min(last);
                }
                KeyCode::ArrowDown => {}
                KeyCode::PageUp => self.card = at.saturating_sub(page.min(at / columns * columns)),
                KeyCode::PageDown if at / columns < last / columns => {
                    self.card =
                        (at + page.min((last / columns - at / columns) * columns)).min(last);
                }
                KeyCode::PageDown => {}
                KeyCode::Home => self.card = 0,
                KeyCode::End => self.card = last,
                _ => return self.key_any(key, shift, illuminate, vertical),
            }
            return PanelAction::None;
        }
        if self.tab == Tab::Shaders && self.grid {
            // The wall: Left and Right step a tile (on into the next tier's), Up and Down
            // a line of tiles, to the same column or the nearest, past the headings,
            // stopping at the ends.
            let last = self.shader_count - 1;
            match key {
                KeyCode::ArrowLeft => self.shader = self.shader.min(last).saturating_sub(1),
                KeyCode::ArrowRight => self.shader = (self.shader + 1).min(last),
                KeyCode::ArrowUp | KeyCode::ArrowDown => {
                    self.shader = shaders::vertical(
                        &self.shader_lines,
                        self.shader.min(last),
                        key == KeyCode::ArrowDown,
                    );
                }
                _ => {}
            }
            if matches!(
                key,
                KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
            ) {
                return PanelAction::None;
            }
        }
        self.key_any(key, shift, illuminate, vertical)
    }

    /// A pressed key the tab's own grid did not take.
    fn key_any(
        &mut self,
        key: KeyCode,
        shift: bool,
        illuminate: bool,
        vertical: bool,
    ) -> PanelAction {
        match key {
            KeyCode::Escape => return PanelAction::Close,
            KeyCode::KeyV if self.tab == Tab::Shaders => return self.set_grid(!self.grid),
            // F turns the achievements board's page: General, Fun, Hobbies.
            KeyCode::KeyF if self.tab == Tab::Achievements => {
                self.set_achievement_page(self.achievement_page.next());
            }
            // K waits for a key to bind to the chosen toy.
            KeyCode::KeyK if self.tab == Tab::Toys => self.bind_wait = Some(0),
            KeyCode::KeyR if self.tab == Tab::Cards => return self.reveal(),
            KeyCode::Tab => self.step(if shift { -1 } else { 1 }, true),
            KeyCode::ArrowLeft if !vertical => self.step(-1, false),
            KeyCode::ArrowRight if !vertical => self.step(1, false),
            KeyCode::ArrowUp | KeyCode::ArrowDown if self.tab == Tab::Achievements => {
                self.achievement = achievements::vertical(
                    &self.achievement_slots,
                    self.achievement,
                    key == KeyCode::ArrowDown,
                );
            }
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.step(-1, false),
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.step(1, false),
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                return self.activate(illuminate);
            }
            // 1 to 6 switch the chosen shader's options.
            KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4
            | KeyCode::Digit5
            | KeyCode::Digit6
                if self.tab == Tab::Shaders =>
            {
                let index = match key {
                    KeyCode::Digit1 => 0,
                    KeyCode::Digit2 => 1,
                    KeyCode::Digit3 => 2,
                    KeyCode::Digit4 => 3,
                    KeyCode::Digit5 => 4,
                    _ => 5,
                };
                return self.switch_option(index);
            }
            _ => {}
        }
        PanelAction::None
    }

    /// Switch the chosen shader's option `index` (as the last frame showed them) the
    /// other way; a locked extra (listed without its id) stays as it is.
    fn switch_option(&self, index: usize) -> PanelAction {
        self.shader_options
            .get(index)
            .filter(|(_, option, _)| !option.is_empty())
            .map_or(PanelAction::None, |(skin, option, on)| {
                PanelAction::SkinOption {
                    skin,
                    option: option.clone(),
                    on: !on,
                }
            })
    }

    /// A pointer event; `illuminate` is whether the holocron is lit.
    pub(crate) fn handle_pointer(&mut self, event: InputEvent, illuminate: bool) -> PanelAction {
        self.pointer_at(event, illuminate, Instant::now())
    }

    /// A pointer event at `now`.
    fn pointer_at(&mut self, event: InputEvent, illuminate: bool, now: Instant) -> PanelAction {
        self.hover.moved();
        match event {
            InputEvent::PointerMove(at)
            | InputEvent::PointerPress { position: at, .. }
            | InputEvent::PointerRelease { position: at, .. }
            | InputEvent::PointerWheel { position: at, .. } => self.pointer = Some([at.x, at.y]),
            InputEvent::PointerLeave => self.pointer = None,
            InputEvent::Action(_) => {}
        }
        // The Loadouts card takes the pointer while it shows; the page under it does not.
        if self.loadouts.is_open() {
            let Some(event) = self.ui.pointer(event) else {
                return PanelAction::None;
            };
            let Some(token) = event.token else {
                return PanelAction::None;
            };
            let reply = self.loadouts.token(
                token,
                event.kind == UiEventKind::Activate,
                &self.loadout_slots,
            );
            return Self::loadout_reply(reply);
        }
        // The lone saber turns with a drag that starts on it, wherever it goes after.
        if matches!(self.tab, Tab::Shaders | Tab::SaberLab) {
            match event {
                InputEvent::PointerPress { position: at, .. }
                    if self.backstage != Backstage::None
                        && self
                            .ui
                            .rect_for(SABER_TOKEN)
                            .is_some_and(|rect| rect.contains(at))
                        // The target switch over the saber is pressed, not the saber, and
                        // so is the Loadouts button in the room's top right corner (hub
                        // report #245: the stage took its clicks).
                        && ![
                            TARGET_FIRST_TOKEN,
                            TARGET_SECOND_TOKEN,
                            BOTH_TOKEN,
                            loadouts::OPEN_TOKEN,
                        ]
                        .iter()
                        .any(|token| {
                            self.ui.rect_for(*token).is_some_and(|rect| rect.contains(at))
                        }) =>
                {
                    self.spin.grab([at.x, at.y]);
                    self.saber_press = Some([at.x, at.y]);
                    return PanelAction::None;
                }
                InputEvent::PointerMove(at) if self.spin.held() => {
                    self.spin.drag([at.x, at.y], self.scale);
                    return PanelAction::None;
                }
                InputEvent::PointerRelease { position: at, .. } if self.spin.held() => {
                    self.spin.release(self.epoch.elapsed().as_secs_f32());
                    // A click, not a drag, on the sabers chooses the one it lands on: two
                    // sabers lie one above the other; on a staff, the half nearer the
                    // click (where the stage drew each, as it turns), else the other one.
                    let clicked = self.saber_press.take().is_some_and(|[px, py]| {
                        (at.x - px).hypot(at.y - py) <= CLICK_SLOP * self.scale.max(0.1)
                    });
                    // With one saber there is no side to choose, and the Both box is left
                    // as it is (review of 22/10/2026: any click unticked it).
                    if clicked
                        && self.pair != crate::saber_skins::Pair::None
                        && let Some(rect) = self.ui.rect_for(SABER_TOKEN)
                    {
                        let second = match self.pair {
                            crate::saber_skins::Pair::TwoSabers => {
                                at.y > rect.y + rect.height * 0.5
                            }
                            crate::saber_skins::Pair::Staff => {
                                crate::saber_skins::staff_half_at([at.x, at.y])
                                    .unwrap_or(!self.second_target)
                            }
                            crate::saber_skins::Pair::None => false,
                        };
                        return self.set_target(second);
                    }
                    return PanelAction::None;
                }
                InputEvent::PointerLeave if self.spin.held() => {
                    self.saber_press = None;
                    self.spin.release(self.epoch.elapsed().as_secs_f32());
                    return PanelAction::None;
                }
                _ => {}
            }
        }
        if self.tab == Tab::SaberLab
            && let Some(action) = self
                .lab_edit_pointer(event)
                .or_else(|| self.lab_build_pointer(event))
                .or_else(|| self.lab_pointer(event))
        {
            return action;
        }
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::Forge
        {
            if delta.y != 0.0 && self.forge.ask.is_none() {
                self.forge.wheel(delta.y > 0.0, self.grid);
            }
            return PanelAction::None;
        }
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::History
        {
            // The wheel scrolls the list three rows a notch.
            if delta.y != 0.0 {
                self.history.scroll(if delta.y > 0.0 { -3 } else { 3 });
            }
            return PanelAction::None;
        }
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::Cards
        {
            if self.card_inspect {
                // The wheel goes to the previous (up) or next (down) card.
                if delta.y != 0.0 {
                    self.inspect_step(if delta.y > 0.0 { -1 } else { 1 });
                }
                return PanelAction::None;
            }
            self.scroll_cards(delta.y);
            return PanelAction::None;
        }
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::Tags
        {
            // The wheel scrolls the wall a line a notch.
            if delta.y != 0.0 {
                self.scroll_tags(delta.y > 0.0);
            }
            return PanelAction::None;
        }
        if let InputEvent::PointerWheel { delta, .. } = event
            && self.tab == Tab::Shaders
        {
            // The wheel scrolls the wall or the list a line a notch; the shader chosen
            // stays in view.
            if delta.y != 0.0 {
                self.scroll_shaders(delta.y > 0.0);
            }
            return PanelAction::None;
        }
        let Some(event) = self.ui.pointer(event) else {
            return PanelAction::None;
        };
        if event.kind == UiEventKind::Activate && event.token != Some(SEARCH_TOKEN) {
            self.leave_search();
        }
        // A click anywhere but on the key waiting lets go of it.
        if event.kind == UiEventKind::Activate
            && !matches!(event.token, Some(BIND_TOKEN | BIND_SECOND_TOKEN))
        {
            self.bind_wait = None;
        }
        let Some(token) = event.token else {
            return PanelAction::None;
        };
        if self.tab == Tab::History
            && let Some(action) = self.history_token(
                token,
                event.kind == UiEventKind::Activate,
                matches!(event.kind, UiEventKind::HoverEnter | UiEventKind::Hover),
            )
        {
            return action;
        }
        if self.tab == Tab::Forge && (forge::forge_token(token) || self.forge.ask.is_some()) {
            let activate = event.kind == UiEventKind::Activate;
            let resting = matches!(event.kind, UiEventKind::HoverEnter | UiEventKind::Hover);
            // A click on a shader tile sees a New one at once, as on Shaders.
            self.clicked_shader |= activate
                && (forge::TILE_BASE..forge::TILE_BASE + forge::TILES_MAX as u16).contains(&token);
            if !(activate || resting) {
                return PanelAction::None;
            }
            if activate && self.forge.ask.is_none() {
                match token {
                    forge::STAR_TOKEN => {
                        return self.forge_mark(sjk_identity::Mark::Favourite);
                    }
                    forge::PADLOCK_TOKEN => return self.forge_mark(sjk_identity::Mark::Locked),
                    forge::FAVOURITES_TOKEN => {
                        return self.forge.toggle_favourites(self.marks.kept);
                    }
                    _ => {}
                }
            }
            return match self.forge.token(token, activate, now) {
                Some(PanelAction::ShaderView(grid)) => self.set_grid(grid),
                Some(action) => action,
                None => PanelAction::None,
            };
        }
        if event.kind == UiEventKind::Activate
            && let Some(index) = token
                .checked_sub(SHADER_BASE)
                .map(usize::from)
                .filter(|index| *index < self.shader_count)
        {
            // A click chooses (and previews) a shader; Equip, Unequip or Enter wear it
            // (Creyon, 21/10/2026: a click that wore it was too easy to do by mistake).
            // A New one is seen by the click (its sticker goes).
            self.shader = index;
            self.clicked_shader = true;
            return PanelAction::None;
        }
        if event.kind == UiEventKind::Activate {
            let tiers = self.tier_chips();
            if let Some(index) = token
                .checked_sub(FILTER_TIER_BASE)
                .map(usize::from)
                .filter(|index| *index <= tiers.len())
            {
                self.shader_filter.tier = index.checked_sub(1).map(|index| tiers[index]);
                return self.filter_changed();
            }
            if token == FILTER_OWNED_TOKEN {
                return self.toggle_owned();
            }
            if token == FILTER_CHROMA_TOKEN {
                return self.toggle_chroma();
            }
            if token == FILTER_FAVOURITE_TOKEN {
                return self.toggle_favourites_only();
            }
            if let Some(sort) = token
                .checked_sub(SORT_BASE)
                .and_then(|index| ShaderSort::ALL.get(usize::from(index)))
            {
                return self.sort_by(*sort);
            }
            if let Some(tag) = token
                .checked_sub(TAG_BASE)
                .map(usize::from)
                .filter(|tag| *tag < self.tag_entries.len())
            {
                // A click chooses (and shows) a tag; Choose or Enter picks it for toy_tag.
                self.tag = tag;
                return PanelAction::None;
            }
            if self.tab == Tab::Banners {
                if token == BANNER_WEAR {
                    return self.wear_banner();
                }
                if let Some(index) = token
                    .checked_sub(BANNER_BASE)
                    .map(usize::from)
                    .filter(|index| *index < crate::profile_card::banner::ALL.len())
                {
                    self.banner = index;
                    return PanelAction::None;
                }
            }
            if let Some(finish) = token
                .checked_sub(TAG_FINISH_BASE)
                .map(usize::from)
                .filter(|finish| *finish < crate::cards::FINISHES.len())
            {
                return self.choose_tag_finish(finish);
            }
            if token == TAG_CHOOSE_TOKEN {
                return self.choose_tag();
            }
            if token == TAG_BREAK_TOKEN {
                return self.press_tag_tool(tags::Tool::Break);
            }
            if token == TAG_CRAFT_TOKEN {
                return self.press_tag_tool(tags::Tool::Craft);
            }
            if let Some(page) = token
                .checked_sub(ACHIEVEMENT_PAGE_BASE)
                .and_then(|index| crate::achievements::Page::ALL.get(usize::from(index)))
            {
                self.set_achievement_page(*page);
                return PanelAction::None;
            }
            if let Some(toy) = token
                .checked_sub(TOY_SWITCH_BASE)
                .map(usize::from)
                .filter(|toy| *toy < TOY_COUNT)
            {
                self.toy = toy;
                return self.activate(illuminate);
            }
            if matches!(token, BIND_TOKEN | BIND_SECOND_TOKEN) {
                let slot = usize::from(token == BIND_SECOND_TOKEN);
                self.bind_wait = (self.bind_wait != Some(slot)).then_some(slot);
                return PanelAction::None;
            }
            if let Some(show) = token
                .checked_sub(CARD_SHOW_BASE)
                .and_then(|index| crate::cards::binder::Show::ALL.get(usize::from(index)))
            {
                self.card_filter.show = *show;
                return self.card_filter_changed();
            }
            let faces = crate::cards::faces::current();
            match token {
                CARD_RARITY_TOKEN => self.card_filter.cycle_rarity(false),
                CARD_KIND_TOKEN => self.card_filter.cycle_kind(&faces, false),
                CARD_FINISH_TOKEN => self.card_filter.cycle_finish(false),
                _ => {}
            }
            if matches!(
                token,
                CARD_RARITY_TOKEN | CARD_KIND_TOKEN | CARD_FINISH_TOKEN
            ) {
                return self.card_filter_changed();
            }
            if token == SEARCH_TOKEN {
                return self.focus_search();
            }
            // Looking at a card closely: a click anywhere puts it back; a click on the
            // card up close or on a tile looks at it.
            if let Some(finish) = token
                .checked_sub(CARD_LOOK_BASE)
                .map(usize::from)
                .filter(|finish| *finish < crate::cards::FINISHES.len())
            {
                self.card_look = Some((self.card, finish));
                return PanelAction::None;
            }
            if token == CARD_INSPECT_TOKEN {
                self.card_inspect = false;
                return PanelAction::None;
            }
            if token == CARD_CLOSE_UP_TOKEN {
                return self.activate(illuminate);
            }
            if token == CARD_SHADER_TOKEN {
                return self.go_to_card_shader();
            }
            if self.tab == Tab::Cards && self.choose_token(token) {
                return self.activate(illuminate);
            }
        }
        let choose = matches!(
            event.kind,
            UiEventKind::HoverEnter | UiEventKind::Hover | UiEventKind::Activate
        );
        if event.kind != UiEventKind::Activate && !self.hover_chooses(token) {
            return PanelAction::None;
        }
        if choose && self.choose_token(token) {
            return PanelAction::None;
        }
        if event.kind != UiEventKind::Activate {
            return PanelAction::None;
        }
        if self.tab == Tab::SaberLab
            && let Some(action) = self
                .lab_edit_click(token)
                .or_else(|| self.lab_build_click(token))
                .or_else(|| self.lab_click(token))
        {
            return action;
        }
        if self.tab == Tab::Shaders
            && let Some(action) = self.finishers_click(token)
        {
            return action;
        }
        if let Some(style) = token
            .checked_sub(STYLE_BASE)
            .and_then(|index| crate::hud::nameplate::Style::ALL.get(usize::from(index)))
        {
            return PanelAction::NameplateStyle(*style);
        }
        if let Some(index) = token
            .checked_sub(OPTION_BASE)
            .map(usize::from)
            .filter(|index| *index < OPTION_TOKENS)
        {
            return self.switch_option(index);
        }
        match token {
            BACK_TOKEN => PanelAction::Close,
            LIST_VIEW_TOKEN => self.set_grid(false),
            GRID_VIEW_TOKEN => self.set_grid(true),
            WEAR_TOKEN | TOY_SWITCH_TOKEN | SHARE_TOKEN => self.activate(illuminate),
            TOY_SIGHT_TOKEN => self.toy_sight(),
            // One card shows at a time: the button sells what that toy's log holds.
            SELL_JUNK_TOKEN if self.toy == MINING_TOY => PanelAction::SellRubble,
            SELL_JUNK_TOKEN => PanelAction::SellJunk,
            loadouts::OPEN_TOKEN => {
                self.loadouts.open();
                PanelAction::None
            }
            token
                if (TOY_OPTION_TOKEN..TOY_OPTION_TOKEN + TOY_OPTION_TOKENS as u16)
                    .contains(&token) =>
            {
                self.toy_option(usize::from(token - TOY_OPTION_TOKEN))
            }
            TARGET_FIRST_TOKEN => self.set_target(false),
            TARGET_SECOND_TOKEN => self.set_target(true),
            BOTH_TOKEN => self.toggle_both(),
            FORGE_LINK_TOKEN => self.open_forge_link(),
            FAVOURITE_TOKEN => self.toggle_mark(sjk_identity::Mark::Favourite),
            LOCK_TOKEN => self.toggle_mark(sjk_identity::Mark::Locked),
            SEEN_ALL_TOKEN => self.see_all(),
            // A toy's row chooses it; its own switch runs it.
            TOY_TOKEN | BOX_TOKEN | ORDER_TOKEN | HAX_TOKEN | RAGDOLL_TOKEN | DRONE_TOKEN
            | BOOMBOX_TOKEN | MONITOR_TOKEN | BOX_HUNT_TOKEN | BANANA_TOKEN | FISHING_TOKEN => {
                self.toy = TOY_TOKENS.iter().position(|&row| row == token).unwrap_or(0);
                PanelAction::None
            }
            REVEAL_TOKEN => self.reveal(),
            token => match crate::profile_hub::collection_tab_of(token) {
                Some(Tab::Holocrons) => PanelAction::Hub(Tab::Holocrons),
                Some(tab) => {
                    self.tab = tab;
                    PanelAction::None
                }
                None => PanelAction::None,
            },
        }
    }

    /// The blade card up close's button: the Forge's Craft on its shader while it is one
    /// of the trade the player lacks (there its copy crafts it for less), else the Shaders
    /// tab on it; any filter there is cleared so it is in the list.
    fn go_to_card_shader(&mut self) -> PanelAction {
        let Some(skin) = self.card_blade else {
            return PanelAction::None;
        };
        if self.card_blade_craft {
            self.leave_search();
            self.forge_on(forge::Section::Craft, skin.id);
            return PanelAction::None;
        }
        self.show(Tab::Shaders);
        self.shader_filter = ShaderFilter::default();
        self.shader_focus = Some(skin.id);
        PanelAction::None
    }

    /// The Shaders tab's way to the Forge: Craft for a shader of the trade not held, Break
    /// down for one of the player's own copies, the shader chosen there.
    fn open_forge_link(&mut self) -> PanelAction {
        if let Some((section, id)) = self.forge_link {
            self.leave_search();
            self.forge_on(section, id);
            ui_cues_click();
        }
        PanelAction::None
    }

    /// Reveal the sealed cards, when the last frame offered it: the line under the button
    /// waits for the identity's answer.
    fn reveal(&mut self) -> PanelAction {
        if self.revealable == 0 {
            return PanelAction::None;
        }
        self.reveal_asked =
            Some(crate::player_identity::reveal_outcome().map_or(0, |outcome| outcome.serial));
        PanelAction::RevealCards
    }

    /// Whether resting the pointer on `token` chooses it: never a shader, which only a
    /// click chooses and previews (Creyon, 31/10/2026: going to Equip, the pointer
    /// crossed other blades and changed the choice; before, from 21/10, resting on a
    /// row's blade chose it); everything else always.
    fn hover_chooses(&self, token: u16) -> bool {
        token
            .checked_sub(SHADER_BASE)
            .map(usize::from)
            .is_none_or(|index| index >= self.shader_count)
    }

    /// Choose the thing `token` stands for; whether it is one.
    fn choose_token(&mut self, token: u16) -> bool {
        let within = |base: u16, count: usize| {
            token
                .checked_sub(base)
                .map(usize::from)
                .filter(|index| *index < count)
        };
        if let Some(index) = within(MEDAL_BASE, self.medals_shown) {
            self.medal = index;
        } else if let Some(index) = within(ACHIEVEMENT_BASE, WALL_MAX) {
            self.achievement = index;
        } else if let Some(index) = within(SHADER_BASE, self.shader_count) {
            self.shader = index;
        } else if let Some(index) = within(CARD_BASE, self.cards_shown) {
            self.card = index;
        } else {
            return false;
        }
        true
    }

    /// Choose the Cards tab's tile `index` (in the list the filter shows), for a world
    /// shot.
    #[cfg(test)]
    pub(crate) fn choose_card_for_shot(&mut self, index: usize) {
        self.card = index;
    }

    /// Look at the Cards tab's chosen tile closely (or put it back), for a world shot.
    #[cfg(test)]
    pub(crate) fn inspect_card_for_shot(&mut self, on: bool) {
        self.card_inspect = on;
    }

    /// Set the Cards tab's filter, for a world shot.
    #[cfg(test)]
    pub(crate) fn card_filter_for_shot(&mut self, filter: crate::cards::binder::Filter) {
        self.card_filter = filter;
        let _ = self.card_filter_changed();
    }

    /// Tilt card `key` (0 up close, 1 and up the grid's) as though the pointer had rested
    /// at `at` on it for a while, for a world shot.
    #[cfg(test)]
    pub(crate) fn tilt_card_for_shot(&mut self, key: usize, at: [f32; 2]) {
        self.tilt.rest(key, at, self.shot_seconds.unwrap_or(0.0));
    }

    /// Show the hover card of the tab's thing `index` (the toy for Toys; `None` for none),
    /// for a world shot.
    #[cfg(test)]
    pub(crate) fn hover_for_shot(&mut self, index: Option<usize>) {
        let base = match self.tab {
            Tab::Medals => MEDAL_BASE,
            Tab::Achievements => ACHIEVEMENT_BASE,
            Tab::Shaders => SHADER_BASE,
            Tab::Cards => CARD_BASE,
            // The Forge's shots name the token itself.
            Tab::Forge => 0,
            // A toy's row, by its place in the tab; past the last, the Toy Master's "?".
            _ => {
                self.hover.pin(index.and_then(|index| {
                    TOY_TOKENS
                        .get(index)
                        .copied()
                        .or((index == TOY_COUNT).then_some(TOY_MASTER_TOKEN))
                }));
                return;
            }
        };
        self.hover.pin(index.map(|index| base + index as u16));
    }

    /// Choose the Toys tab's toy `index` (0 Illuminate), for a world shot.
    #[cfg(test)]
    pub(crate) fn choose_toy_for_shot(&mut self, index: usize) {
        self.toy = index.min(TOY_COUNT - 1);
    }

    /// Hold the lone saber turned so, still, as a drag would (`None` lets go), for a
    /// world shot.
    #[cfg(test)]
    pub(crate) fn hold_saber_for_shot(&mut self, turn: Option<SaberTurn>) {
        match turn {
            Some(turn) => self.spin.hold(turn),
            None => self.spin.release(f32::NEG_INFINITY),
        }
    }

    /// Choose shader row `row` (0 the stock blade), for a world shot.
    /// Select a banner in an offline UI fixture.
    #[cfg(test)]
    pub(crate) fn choose_banner_for_shot(&mut self, index: usize) {
        self.banner = index.min(crate::profile_card::banner::ALL.len() - 1);
    }

    #[cfg(test)]
    pub(crate) fn choose_shader_for_shot(&mut self, row: usize) {
        self.shader = row.min(SHADER_ROWS - 1);
    }

    /// The token of the thing chosen on the tab on show, for the canvas's focus.
    fn focus_token(&self) -> u16 {
        match self.tab {
            Tab::Medals => MEDAL_BASE + self.medal as u16,
            Tab::Achievements => ACHIEVEMENT_BASE + self.achievement.min(WALL_MAX - 1) as u16,
            Tab::Shaders => SHADER_BASE + self.shader as u16,
            Tab::Toys => TOY_TOKENS[self.toy.min(TOY_COUNT - 1)],
            Tab::Tags => TAG_BASE + self.tag.min(self.tag_entries.len().saturating_sub(1)) as u16,
            Tab::Banners => BANNER_BASE + self.banner as u16,
            Tab::Cards if self.cards_shown > 0 => CARD_BASE + self.card as u16,
            Tab::Nameplates => SHARE_TOKEN,
            Tab::History => self.history_focus(),
            Tab::Forge => match self.forge.ask {
                Some(ask) => {
                    if matches!(
                        ask,
                        forge::Ask::Last { yes: true, .. } | forge::Ask::All { yes: true }
                    ) {
                        forge::ASK_YES_TOKEN
                    } else {
                        forge::ASK_NO_TOKEN
                    }
                }
                None => BACK_TOKEN,
            },
            Tab::SaberLab => self.lab_focus(),
            _ => BACK_TOKEN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sjk_identity::Unlock;

    fn sun() -> Unlock {
        Unlock {
            holo: 0,
            solar: 0,
            id: "saber_sun".into(),
            granted: 1_791_336_225,
            note: "Thanks for testing the blade".into(),
            count: 1,
            medal: None,
            card: None,
        }
    }

    pub(super) fn inputs<'a>(holdings: Holdings<'a>, setting: &'a str) -> Inputs<'a> {
        Inputs {
            enabled: true,
            snapshot: None,
            holdings,
            setting,
            standings: &[],
            illuminate: true,
            cardboard_box: false,
            order66: false,
            hax: false,
            ragdoll: false,
            drone: false,
            boombox: false,
            monitor: false,
            box_hunt: false,
            banana: false,
            fishing: false,
            mining: false,
            fishing_card: None,
            mining_card: None,
            name: "^1Sol^7Vulpes",
            stock: sjk_ui::Color::new(0.16, 0.48, 1.0, 1.0),
            grid: false,
            options: "",
            second: "",
            pair: crate::saber_skins::Pair::None,
            share: true,
            nameplate_style: crate::hud::nameplate::Style::Holo,
            toy_keys: &[],
            toys_unseen: 0,
            toy_options: 0,
            tag: "",
            tag_finish: "",
            finisher: "",
        }
    }

    /// Hub report #108 (Nib): the Toys tab counts every toy, all of them held.
    #[test]
    fn the_toys_tab_counts_every_toy() {
        let counts = view::counts(&inputs(Holdings::Known(&[]), ""));
        let toys = &counts[3];
        assert_eq!((toys.held, toys.total), (Some(TOY_COUNT), TOY_COUNT));
        assert_eq!(TOY_COUNT, TOY_TOKENS.len());
    }

    /// The Shaders tab's rows for a player who holds no Unique shader: the stock blade
    /// and every other blade skin.
    pub(super) fn rows_shown() -> usize {
        1 + Holdings::Known(&[]).listed_count()
    }

    /// The shader row of blade skin `id`.
    pub(super) fn row_of(id: &str) -> usize {
        (1..SHADER_ROWS)
            .find(|&row| shader_skin(row).is_some_and(|skin| skin.id == id))
            .unwrap()
    }

    /// The panel open on `tab` after one frame of `inputs`, so its things are known.
    pub(super) fn drawn(tab: Tab, inputs: &Inputs<'_>) -> Panel {
        let mut panel = Panel::new();
        panel.open(tab, true, true, ReturnTarget::MainMenu);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
        panel
    }

    /// Redraw the Shaders tab of `panel` with `inputs`.
    fn redraw(panel: &mut Panel, inputs: &Inputs<'_>) {
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(inputs, &fonts.font, [1920.0, 1080.0]);
        assert!(!panel.overflowed(), "the filter fits the canvas");
    }

    fn ids(panel: &Panel) -> Vec<&'static str> {
        panel.shader_list.iter().map(|skin| skin.id).collect()
    }

    /// A blade's number in the search lists that blade alone.
    #[test]
    fn a_number_searches_for_that_blade_alone() {
        let filter = ShaderFilter {
            query: "#2".to_owned(),
            ..ShaderFilter::default()
        };
        let found: Vec<_> = unlockables::blade_skins()
            .filter(|skin| filter.passes(skin, false, false))
            .map(|skin| skin.number)
            .collect();
        assert_eq!(found, [2]);
    }

    /// A tier's segment lists that tier's shaders alone, the rarest first as the wall
    /// stands; cycling comes round to All again.
    #[test]
    fn the_tier_filter_lists_one_tier_and_cycling_comes_round() {
        let held = [sun()];
        let inputs = inputs(Holdings::Known(&held), "");
        let mut panel = drawn(Tab::Shaders, &inputs);
        let all = ids(&panel);
        assert_eq!(panel.shader_count, rows_shown());
        panel.shader = 9;
        let _ = panel.cycle_tier(false);
        assert_eq!(panel.shader, 0, "back to the top");
        redraw(&mut panel, &inputs);
        assert_eq!(
            panel.shader_filter.tier,
            Some(unlockables::Rarity::Mythical)
        );
        assert_eq!(ids(&panel)[0], "saber_sun");
        assert_eq!(
            panel.shader_count,
            1 + ids(&panel).len(),
            "the stock blade stays first"
        );
        for _ in 0..5 {
            let _ = panel.cycle_tier(false);
        }
        redraw(&mut panel, &inputs);
        assert_eq!(ids(&panel), all, "All again after the last tier");
        let _ = panel.cycle_tier(true);
        redraw(&mut panel, &inputs);
        assert_eq!(panel.shader_filter.tier, Some(unlockables::Rarity::Common));
        assert_eq!(ids(&panel).len(), 43);
        // C lists the chromas only.
        panel.shader_filter = ShaderFilter::default();
        let _ = panel.toggle_chroma();
        redraw(&mut panel, &inputs);
        assert!(!ids(&panel).is_empty());
        assert!(panel.shader_list.iter().all(|skin| skin.chroma));
    }

    /// A hover card under a filter is the card of the row shown, not of the catalogue's
    /// row at that place; a click elsewhere or another tab lets go of the search.
    #[test]
    fn the_hover_card_follows_the_filter_and_the_search_lets_go() {
        let held = [sun()];
        let inputs = inputs(Holdings::Known(&held), "");
        let mut panel = drawn(Tab::Shaders, &inputs);
        panel.shader_filter.query = "kamino".to_owned();
        redraw(&mut panel, &inputs);
        let card = panel.card_of(SHADER_BASE + 1, &inputs).expect("a card");
        assert_eq!(card.title, "Kamino blade");
        assert!(
            panel.card_of(SHADER_BASE + 2, &inputs).is_none(),
            "past the list"
        );
        let _ = panel.focus_search();
        panel.show(Tab::Medals);
        assert!(!panel.search_focus);
        panel.show(Tab::Shaders);
        let _ = panel.focus_search();
        panel.leave_search();
        assert!(!panel.search_focus);
        assert_eq!(panel.shader_filter.query, "kamino", "what it holds stays");
    }

    /// Owned shows the shaders held; a search matches names and tier words.
    #[test]
    fn owned_and_search_narrow_the_list() {
        let held = [sun()];
        let inputs = inputs(Holdings::Known(&held), "");
        let mut panel = drawn(Tab::Shaders, &inputs);
        let _ = panel.toggle_owned();
        redraw(&mut panel, &inputs);
        assert_eq!(ids(&panel), ["saber_sun"]);
        let _ = panel.toggle_owned();
        panel.shader_filter.query = " KAMINO ".to_owned();
        redraw(&mut panel, &inputs);
        assert_eq!(ids(&panel), ["saber_tidal"]);
        panel.shader_filter.query = "creyon".to_owned();
        redraw(&mut panel, &inputs);
        assert_eq!(ids(&panel).len(), 151, "the saber smith is searched");
        panel.shader_filter.query = "mythical".to_owned();
        redraw(&mut panel, &inputs);
        assert_eq!(
            ids(&panel),
            [
                "saber_sun",
                "saber_void",
                "saber_comic",
                "saber_obsidian",
                "saber_abyssal_eye",
                "saber_carbonite",
                "saber_eclipse",
                "saber_event_horizon",
                "saber_exogorth",
                "saber_world_shift",
                "saber_exegol_rift",
                "saber_varykino_blossom",
            ]
        );
        panel.shader_filter.query = "nothing".to_owned();
        redraw(&mut panel, &inputs);
        assert!(ids(&panel).is_empty());
        assert_eq!(panel.shader_count, 1, "only the stock blade is left");
        assert!(panel.shader_filter.active());
        panel.shader_filter = ShaderFilter::default();
        assert!(!panel.shader_filter.active());
    }

    /// Sol's rule (25/10/2026): a Unique shader shows only to its holders. Not held, it is
    /// not listed, found by a search (its name, its tier's word or its number), counted in
    /// the headline, the Saber pill or the tab's tally, and has no chip; held, it is listed
    /// first (the rarest), counted, and its chip lists it. Taken back, its chip goes too.
    #[test]
    fn a_unique_shader_shows_only_to_its_holders() {
        // The shaders every player sees: all but the Unique ones.
        let total = unlockables::blade_skins()
            .filter(|skin| !skin.hidden_unless_held())
            .count();
        let only_sun = [sun()];
        let lacking = inputs(Holdings::Known(&only_sun), "");
        let mut panel = drawn(Tab::Shaders, &lacking);
        let runs =
            |panel: &Panel| -> Vec<String> { panel.ui.text_runs().map(str::to_owned).collect() };
        assert!(!ids(&panel).contains(&"saber_thousand_cranes"));
        assert_eq!(
            panel.shader_count,
            1 + total,
            "the stock blade and the others"
        );
        let seen = runs(&panel);
        assert!(seen.contains(&format!("1 of {total} owned")), "{seen:?}");
        assert!(seen.contains(&format!("1/{total}")), "{seen:?}");
        assert!(seen.iter().all(|run| !run.contains("Unique")), "{seen:?}");
        assert!(panel.ui.rect_for(FILTER_TIER_BASE + 6).is_none(), "no chip");
        let counts = view::counts(&lacking);
        assert_eq!((counts[2].held, counts[2].total), (Some(1), total));
        for query in ["thousand", "Cranes", "unique", "167", "#167"] {
            panel.shader_filter.query = query.to_owned();
            redraw(&mut panel, &lacking);
            assert!(ids(&panel).is_empty(), "{query}");
        }
        panel.shader_filter = ShaderFilter::default();
        for _ in 0..STRIP_TIERS_WITH_UNIQUE.len() + 1 {
            let _ = panel.cycle_tier(false);
            assert_ne!(panel.shader_filter.tier, Some(unlockables::Rarity::Unique));
        }

        let both = [
            sun(),
            Unlock {
                id: "saber_thousand_cranes".into(),
                ..sun()
            },
        ];
        let holding = inputs(Holdings::Known(&both), "");
        let mut panel = drawn(Tab::Shaders, &holding);
        assert_eq!(ids(&panel)[0], "saber_thousand_cranes", "the rarest first");
        assert_eq!(panel.shader_count, 2 + total, "and the one held");
        let seen = runs(&panel);
        assert!(
            seen.contains(&format!("2 of {} owned", total + 1)),
            "{seen:?}"
        );
        assert!(seen.contains(&format!("2/{}", total + 1)), "{seen:?}");
        let counts = view::counts(&holding);
        assert_eq!((counts[2].held, counts[2].total), (Some(2), total + 1));
        assert!(
            panel.ui.rect_for(FILTER_TIER_BASE + 6).is_some(),
            "its chip"
        );
        let card = panel.card_of(SHADER_BASE + 1, &holding).expect("its card");
        assert_eq!(card.title, "Thousand Cranes blade");
        assert_eq!(card.lines[0].words, unlockables::PERSONAL_THANKS);
        // T from All: the first segment, Unique's, lists it alone.
        let _ = panel.cycle_tier(false);
        assert_eq!(panel.shader_filter.tier, Some(unlockables::Rarity::Unique));
        redraw(&mut panel, &holding);
        assert_eq!(ids(&panel), ["saber_thousand_cranes"]);
        // Taken back: no chip, no filter on it, not listed.
        redraw(&mut panel, &lacking);
        assert_eq!(panel.shader_filter.tier, None);
        assert!(!ids(&panel).contains(&"saber_thousand_cranes"));
        assert!(panel.ui.rect_for(FILTER_TIER_BASE + 6).is_none());
    }

    /// R on Cards (or Reveal) asks the hub only when the last frame offered something to
    /// reveal, and then waits for the answer.
    #[test]
    fn reveal_asks_only_when_there_is_something_to_reveal() {
        let mut panel = drawn(Tab::Cards, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(panel.revealable, 0, "no identity, nothing to reveal");
        assert_eq!(panel.key(KeyCode::KeyR, false, true), PanelAction::None);
        assert_eq!(panel.reveal_asked, None);
        panel.revealable = 3;
        assert_eq!(
            panel.key(KeyCode::KeyR, false, true),
            PanelAction::RevealCards
        );
        assert_eq!(panel.reveal_asked, Some(0), "no reveal before this one");
        // Not on another tab.
        panel.tab = Tab::Medals;
        assert_eq!(panel.key(KeyCode::KeyR, false, true), PanelAction::None);
    }

    #[test]
    fn shaders_open_on_what_is_worn_and_enter_equips_and_unequips() {
        let owned = [sun()];
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), "saber_sun"));
        let sun_row = row_of("saber_sun");
        assert_eq!(panel.shader, sun_row, "the worn one is chosen");
        assert_eq!(
            panel.stage_wish().map(|wish| wish.skin),
            Some(PreviewSkin::Skin("saber_sun"))
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Wear("")
        );
        // The stock row wears the stock blade; the Sun row, once not worn, the Sun.
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Wear("saber_sun")
        );
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0);
        assert_eq!(
            panel.stage_wish().map(|wish| wish.skin),
            Some(PreviewSkin::Stock)
        );
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::None,
            "worn already"
        );
        assert_eq!(panel.key(KeyCode::Escape, false, true), PanelAction::Close);
    }

    #[test]
    fn a_locked_shader_is_previewed_but_never_equipped() {
        for holdings in [
            Holdings::Known(&[]),
            Holdings::IdentityOff,
            Holdings::Waiting,
        ] {
            let mut panel = drawn(Tab::Shaders, &inputs(holdings, ""));
            let _ = panel.key(KeyCode::ArrowDown, false, true);
            let id = shader_skin(1).unwrap().id;
            assert_eq!(
                panel.stage_wish().map(|wish| wish.skin),
                Some(PreviewSkin::Skin(id)),
                "{holdings:?}"
            );
            assert_eq!(panel.key(KeyCode::Enter, false, true), PanelAction::None);
            assert!(panel.ui.rect_for(WEAR_TOKEN).is_none(), "no Equip");
        }
    }

    #[test]
    fn the_toy_lights_and_puts_out_illuminate_and_only_the_kit_tabs_want_the_stage() {
        let mut panel = drawn(Tab::Toys, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Illuminate(false)
        );
        assert_eq!(
            panel.key(KeyCode::Space, false, false),
            PanelAction::Illuminate(true)
        );
        assert!(panel.stage_wish().is_some_and(|wish| wish.holocron));
        for (tab, wants) in [
            (Tab::Medals, false),
            (Tab::Achievements, false),
            (Tab::Shaders, true),
            (Tab::Toys, true),
            (Tab::Nameplates, true),
        ] {
            panel.show(tab);
            assert_eq!(panel.stage_wish().is_some(), wants, "{tab:?}");
        }
        // A tab of the Profile screen is not the page's.
        panel.show(Tab::Saber);
        assert_eq!(panel.tab(), Tab::Nameplates);
        let _ = panel.close();
        assert_eq!(panel.stage_wish(), None);
    }

    #[test]
    fn the_keys_stay_on_the_things_and_tab_comes_round() {
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&[]), ""));
        assert_eq!(panel.shader, 0, "the stock blade, worn");
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0, "the arrows stop at the ends");
        for _ in 0..rows_shown() + 2 {
            let _ = panel.key(KeyCode::ArrowDown, false, true);
        }
        assert_eq!(panel.shader, rows_shown() - 1);
        let _ = panel.key(KeyCode::Tab, false, true);
        assert_eq!(panel.shader, 0, "Tab comes round");
        let _ = panel.key(KeyCode::Tab, true, true);
        assert_eq!(panel.shader, rows_shown() - 1);
    }

    /// The Toys tab's switch and plinth light the holocron when it is out and put it out
    /// when it is lit, whichever state the page was told, and its words name the new
    /// command, not the Force wheel.
    #[test]
    fn the_toys_switch_lights_or_puts_out_by_the_state_it_is_told() {
        let click = |panel: &mut Panel, token: u16, lit: bool| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.handle_pointer(event, lit);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        for (token, lit, wanted) in [
            (TOY_SWITCH_TOKEN, false, true),
            (TOY_SWITCH_TOKEN, true, false),
            (TOY_SWITCH_BASE, false, true),
        ] {
            let mut shown = inputs(Holdings::Known(&[]), "");
            shown.illuminate = lit;
            let mut panel = drawn(Tab::Toys, &shown);
            assert_eq!(
                click(&mut panel, token, lit),
                PanelAction::Illuminate(wanted),
                "token {token}, lit {lit}"
            );
            let all = panel.ui.text_runs().collect::<Vec<_>>().join(
                "
",
            );
            assert!(all.contains("toy_illuminate"), "{all}");
            assert!(all.contains("Holocron lit"), "{all}");
            assert!(
                !all.contains("Force wheel") && !all.contains("force_illuminate"),
                "{all}"
            );
        }
    }

    /// The box's niche chooses it, then runs it; Enter and the switch get in or out by
    /// the state the page was told, and the stage holds no holocron for it.
    #[test]
    fn the_cardboard_box_is_chosen_then_put_on_or_taken_off() {
        let mut shown = inputs(Holdings::Known(&[]), "");
        let mut panel = drawn(Tab::Toys, &shown);
        let _ = panel.key(KeyCode::ArrowRight, false, false);
        assert_eq!(panel.toy, 1);
        assert_eq!(
            panel.key(KeyCode::Enter, false, false),
            PanelAction::CardboardBox(true)
        );
        assert!(panel.stage_wish().is_some_and(|wish| !wish.holocron));
        shown.cardboard_box = true;
        let mut panel = drawn(Tab::Toys, &shown);
        let _ = panel.key(KeyCode::ArrowRight, false, false);
        redraw(&mut panel, &shown);
        assert_eq!(
            panel.key(KeyCode::Enter, false, false),
            PanelAction::CardboardBox(false)
        );
        let all = panel.ui.text_runs().collect::<Vec<_>>().join("\n");
        assert!(
            all.contains("toy_box") && all.contains("In the box"),
            "{all}"
        );
        let _ = panel.key(KeyCode::ArrowLeft, false, false);
        assert_eq!(
            panel.key(KeyCode::Enter, false, true),
            PanelAction::Illuminate(false)
        );
    }

    /// The Fishing card's Sell junk button sells (its token is no nameplate style's or
    /// toy option's, which a click reads by range first).
    #[test]
    fn the_fishing_cards_sell_button_sells() {
        let log = crate::fishing::Log::parse(
            "boot 3
hilt 1
",
        );
        let card = crate::fishing_sell::card(&log, Some((8, 15)), false, None);
        let mut shown = inputs(Holdings::Known(&[]), "");
        shown.fishing_card = Some(&card);
        let mut panel = drawn(Tab::Toys, &shown);
        panel.toy = FISHING_TOY;
        redraw(&mut panel, &shown);
        let rect = panel.ui.rect_for(SELL_JUNK_TOKEN).expect("the Sell button");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let button = sjk_ui::PointerButton::Primary;
        let action = [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button,
            },
            InputEvent::PointerRelease {
                position: at,
                button,
            },
        ]
        .into_iter()
        .map(|event| panel.pointer_at(event, true, Instant::now()))
        .find(|action| *action != PanelAction::None)
        .unwrap_or(PanelAction::None);
        assert_eq!(action, PanelAction::SellJunk);
    }

    /// The Mining card's Sell rubble button, the same token on its own card.
    #[test]
    fn the_mining_cards_sell_button_sells_rubble() {
        let log = crate::mining::Log::parse(
            "rubble 3
fossil 1
",
        );
        let card = crate::mining_sell::card(&log, Some((6, 15)), false, None);
        let mut shown = inputs(Holdings::Known(&[]), "");
        shown.mining_card = Some(&card);
        let mut panel = drawn(Tab::Toys, &shown);
        panel.toy = MINING_TOY;
        redraw(&mut panel, &shown);
        let rect = panel.ui.rect_for(SELL_JUNK_TOKEN).expect("the Sell button");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let button = sjk_ui::PointerButton::Primary;
        let action = [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button,
            },
            InputEvent::PointerRelease {
                position: at,
                button,
            },
        ]
        .into_iter()
        .map(|event| panel.pointer_at(event, true, Instant::now()))
        .find(|action| *action != PanelAction::None)
        .unwrap_or(PanelAction::None);
        assert_eq!(action, PanelAction::SellRubble);
    }

    /// Order 66 is the third toy: chosen, then turned on or off by the state the page
    /// was told; its row chooses it, its row's own switch runs it.
    #[test]
    fn order_66_is_chosen_then_turned_on_or_off() {
        let mut shown = inputs(Holdings::Known(&[]), "");
        let mut panel = drawn(Tab::Toys, &shown);
        let _ = panel.key(KeyCode::ArrowRight, false, false);
        let _ = panel.key(KeyCode::ArrowRight, false, false);
        assert_eq!(panel.toy, 2);
        assert_eq!(
            panel.key(KeyCode::Enter, false, false),
            PanelAction::Order66(true)
        );
        assert!(panel.stage_wish().is_some_and(|wish| !wish.holocron));
        shown.order66 = true;
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let button = sjk_ui::PointerButton::Primary;
            [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button,
                },
            ]
            .into_iter()
            .map(|event| panel.pointer_at(event, true, Instant::now()))
            .find(|action| *action != PanelAction::None)
            .unwrap_or(PanelAction::None)
        };
        let mut panel = drawn(Tab::Toys, &shown);
        assert_eq!(click(&mut panel, ORDER_TOKEN), PanelAction::None, "chosen");
        assert_eq!(panel.toy, 2);
        redraw(&mut panel, &shown);
        assert_eq!(
            click(&mut panel, ORDER_TOKEN),
            PanelAction::None,
            "still chosen"
        );
        assert_eq!(
            click(&mut panel, TOY_SWITCH_BASE + 2),
            PanelAction::Order66(false)
        );
        // Another row's switch runs that toy, and chooses it.
        assert_eq!(
            click(&mut panel, TOY_SWITCH_BASE + 3),
            PanelAction::Hax(true)
        );
        assert_eq!(panel.toy, 3);
        let all = panel.ui.text_runs().collect::<Vec<_>>().join("\n");
        assert!(
            all.contains("toy_order66") && all.contains("In the order"),
            "{all}"
        );
        // The Hax acts only on players who have it on: its card has no "See" switch.
        redraw(&mut panel, &shown);
        assert!(panel.ui.rect_for(TOY_SIGHT_TOKEN).is_none());
        // Its card holds its trap's two switches (moved from Settings > Gameplay).
        assert_eq!(
            click(&mut panel, TOY_OPTION_TOKEN),
            PanelAction::ToyOption {
                cvar: crate::hax::TRAP_CVAR,
                on: true,
            }
        );
        shown.toy_options = toys::options_on(|cvar| cvar == crate::hax::AWAY_CVAR);
        redraw(&mut panel, &shown);
        assert_eq!(
            click(&mut panel, TOY_OPTION_TOKEN + 1),
            PanelAction::ToyOption {
                cvar: crate::hax::AWAY_CVAR,
                on: false,
            }
        );
        assert!(panel.ui.rect_for(TOY_OPTION_TOKEN + 2).is_none());
        shown.toy_options = 0;
        // The Monitor's hides the monitors, and shows them again.
        let _ = click(&mut panel, MONITOR_TOKEN);
        redraw(&mut panel, &shown);
        assert_eq!(
            click(&mut panel, TOY_SIGHT_TOKEN),
            PanelAction::ToySight {
                cvar: crate::monitor::SHOW_CVAR,
                seen: false,
            }
        );
        shown.toys_unseen = 1 << 7;
        redraw(&mut panel, &shown);
        assert_eq!(
            click(&mut panel, TOY_SIGHT_TOKEN),
            PanelAction::ToySight {
                cvar: crate::monitor::SHOW_CVAR,
                seen: true,
            }
        );
        assert_eq!(
            toys::unseen(|cvar| cvar != crate::toy_sight::DRONE),
            1 << 5,
            "the Drone's bit"
        );
    }

    /// The Nameplates tab's Style row offers every nameplate style, the one in use
    /// lit, and a click on another asks for it; each style's plate fits the page.
    #[test]
    fn the_style_row_chooses_the_nameplates_look() {
        use crate::hud::nameplate::Style;
        for style in Style::ALL {
            let shown = Inputs {
                nameplate_style: style,
                ..inputs(Holdings::Known(&[]), "")
            };
            let mut panel = drawn(Tab::Nameplates, &shown);
            assert!(!panel.ui.overflowed(), "{style:?}");
            for (index, other) in Style::ALL.into_iter().enumerate() {
                let rect = panel
                    .ui
                    .rect_for(STYLE_BASE + index as u16)
                    .expect("a style's segment");
                let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
                let button = sjk_ui::PointerButton::Primary;
                let action = [
                    InputEvent::PointerMove(at),
                    InputEvent::PointerPress {
                        position: at,
                        button,
                    },
                    InputEvent::PointerRelease {
                        position: at,
                        button,
                    },
                ]
                .into_iter()
                .map(|event| panel.pointer_at(event, true, Instant::now()))
                .find(|action| *action != PanelAction::None);
                assert_eq!(action, Some(PanelAction::NameplateStyle(other)));
            }
            let all = panel.ui.text_runs().collect::<Vec<_>>().join(
                "
",
            );
            assert!(all.contains("Capsule") && all.contains("Classic"), "{all}");
        }
    }

    /// The Nameplates tab's See and share nameplates switch, clicked or with Enter, flips the
    /// state the page was told, and says what off means.
    #[test]
    fn the_share_switch_flips_sharing() {
        let mut shown = inputs(Holdings::Known(&[]), "");
        let mut panel = drawn(Tab::Nameplates, &shown);
        let rect = panel.ui.rect_for(SHARE_TOKEN).expect("the switch");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let button = sjk_ui::PointerButton::Primary;
        let action = [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button,
            },
            InputEvent::PointerRelease {
                position: at,
                button,
            },
        ]
        .into_iter()
        .map(|event| panel.pointer_at(event, true, Instant::now()))
        .find(|action| *action != PanelAction::None);
        assert_eq!(action, Some(PanelAction::SeeAndShare(false)));
        shown.share = false;
        redraw(&mut panel, &shown);
        assert_eq!(
            panel.key(KeyCode::Enter, false, false),
            PanelAction::SeeAndShare(true)
        );
        let all = panel.ui.text_runs().collect::<Vec<_>>().join("\n");
        assert!(
            all.contains("See & share nameplates") && all.contains("you see no nameplates"),
            "{all}"
        );
    }

    #[test]
    fn a_click_chooses_and_never_wears() {
        let owned = [sun()];
        let click = |panel: &mut Panel, token: u16, now: Instant| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.pointer_at(event, true, now);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        let row = |id: &str| SHADER_BASE + row_of(id) as u16;
        let start = Instant::now();
        let later = |millis: u64| start + std::time::Duration::from_millis(millis);
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), ""));
        // A click chooses a shader, owned or not; Equip wears it.
        assert_eq!(
            click(&mut panel, row("saber_sun"), later(0)),
            PanelAction::None
        );
        assert_eq!(panel.shader, usize::from(row("saber_sun") - SHADER_BASE));
        // A locked one is only previewed.
        assert_eq!(
            click(&mut panel, row("saber_void"), later(3_000)),
            PanelAction::None
        );
        assert_eq!(
            click(&mut panel, row("saber_void"), later(3_200)),
            PanelAction::None
        );
        assert_eq!(panel.shader, usize::from(row("saber_void") - SHADER_BASE));
        // The worn one stays worn.
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), "saber_sun"));
        assert_eq!(
            click(&mut panel, row("saber_sun"), later(0)),
            PanelAction::None
        );
        assert_eq!(
            click(&mut panel, row("saber_sun"), later(200)),
            PanelAction::None
        );
        // The stock blade is chosen by a click too, not worn.
        assert_eq!(
            click(&mut panel, SHADER_BASE, later(1_000)),
            PanelAction::None
        );
    }

    /// Creyon, 31/10/2026: a New shader not seen yet stands out in gold, on the wall and in
    /// the list, besides its sticker; chosen, it wears the white outline instead.
    #[test]
    fn a_new_shader_is_outlined_in_gold() {
        let owned = [sun()];
        let shown = inputs(Holdings::Known(&owned), "");
        let gold = |panel: &Panel| {
            panel
                .ui
                .draw_list()
                .commands()
                .iter()
                .filter(|command| match command {
                    sjk_ui::DrawCommand::Border { color: paint, .. }
                    | sjk_ui::DrawCommand::RoundedRect { color: paint, .. } => {
                        use crate::menu::sjk::color;
                        *paint == color::alpha(color::GOLD_BRIGHT, 0.85)
                            || *paint == color::alpha(color::GOLD_BRIGHT, 0.9)
                    }
                    _ => false,
                })
                .count()
        };
        for grid in [true, false] {
            let mut panel = Panel::new();
            panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
            let _ = panel.set_grid(grid);
            redraw(&mut panel, &shown);
            let plain = gold(&panel);
            panel.preview_marks = Some(crate::shader_marks::Marks::of(
                None,
                false,
                &owned,
                Some(&crate::shader_marks::Seen::default()),
                false,
                [None, None],
            ));
            redraw(&mut panel, &shown);
            assert_eq!(gold(&panel), plain + 1, "grid {grid}: one New, outlined");
            assert!(panel.ui.text_runs().any(|run| run == "New"), "grid {grid}");
            panel.shader = row_of("saber_sun");
            redraw(&mut panel, &shown);
            assert_eq!(
                gold(&panel),
                plain,
                "grid {grid}: chosen, the white outline"
            );
        }
    }

    /// Creyon, 31/10/2026: only a click chooses (and previews) a shader; the pointer
    /// resting on another blade, on the way to Equip, leaves the choice alone.
    #[test]
    fn resting_on_a_shader_never_chooses_it() {
        let owned = [sun()];
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), ""));
        let index = row_of("saber_sun");
        let row = panel
            .ui
            .rect_for(SHADER_BASE + index as u16)
            .expect("the row's pointer area");
        let at = sjk_ui::Vec2::new(row.x + row.width * 0.5, row.y + row.height * 0.5);
        let now = Instant::now();
        assert_ne!(panel.shader, index);
        let _ = panel.pointer_at(InputEvent::PointerMove(at), true, now);
        let _ = panel.pointer_at(
            InputEvent::PointerMove(at),
            true,
            now + std::time::Duration::from_secs(2),
        );
        assert_ne!(panel.shader, index, "resting chose it");
        for event in [
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.pointer_at(event, true, now);
        }
        assert_eq!(panel.shader, index, "a click chooses it");
        assert!(panel.clicked_shader, "a New one is seen at the click");
        // The next frame takes the click: a New one chosen is seen then, not a second later.
        panel.view(Some("saber_sun"));
        assert!(!panel.clicked_shader);
    }

    /// Finishers (04/11/2026): the Shaders tab's Finishers pill shows them, a held one's
    /// Equip wears it and its Unequip takes it off, an unheld one has no button, and the
    /// Saber pill goes back to the shaders.
    #[test]
    fn the_finishers_pill_shows_them_and_equips_a_held_one() {
        use super::finishers::{FINISHER_WEAR_BASE, FINISHERS_KIND_TOKEN, SABER_KIND_TOKEN};
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.handle_pointer(event, true);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        let held = [Unlock {
            id: "finisher_disintegrate".into(),
            ..sun()
        }];
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        // Not held: the page shows it without a button.
        let none = inputs(Holdings::Known(&[]), "");
        let mut panel = drawn(Tab::Shaders, &none);
        assert_eq!(click(&mut panel, FINISHERS_KIND_TOKEN), PanelAction::None);
        panel.build(&none, &fonts.font, [1920.0, 1080.0]);
        assert!(panel.ui.rect_for(FINISHER_WEAR_BASE).is_none());
        assert!(!panel.overflowed());
        // Held: Equip wears it; worn, Unequip takes it off.
        let holding = inputs(Holdings::Known(&held), "");
        panel.build(&holding, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, FINISHER_WEAR_BASE),
            PanelAction::WearFinisher("finisher_disintegrate")
        );
        let wearing = Inputs {
            finisher: "finisher_disintegrate",
            ..inputs(Holdings::Known(&held), "")
        };
        panel.build(&wearing, &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, FINISHER_WEAR_BASE),
            PanelAction::WearFinisher("")
        );
        // The Saber pill brings the shaders back.
        assert_eq!(click(&mut panel, SABER_KIND_TOKEN), PanelAction::None);
        panel.build(&wearing, &fonts.font, [1920.0, 1080.0]);
        assert!(!panel.shader_list.is_empty());
        assert!(panel.ui.rect_for(FINISHER_WEAR_BASE).is_none());
    }

    #[test]
    fn a_click_chooses_acts_and_turns_the_tab() {
        let owned = [sun()];
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&owned), ""));
        let click = |panel: &mut Panel, token: u16| {
            let rect = panel.ui.rect_for(token).expect("a pointer area");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.handle_pointer(event, true);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        let sun_row = row_of("saber_sun") as u16;
        assert_eq!(click(&mut panel, SHADER_BASE + sun_row), PanelAction::None);
        assert_eq!(panel.shader, usize::from(sun_row));
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::Wear("saber_sun")
        );
        // With one saber of one blade there is no target switch.
        assert!(panel.ui.rect_for(TARGET_SECOND_TOKEN).is_none());
        // Two sabers on the stage, the first wearing the Sun and the second nothing of its
        // own. The switch's pills are hit over the sabers' room (the later region wins).
        panel.set_backstage(Backstage::World { head: None });
        let dual = |first, second| Inputs {
            second,
            pair: crate::saber_skins::Pair::TwoSabers,
            ..inputs(Holdings::Known(&owned), first)
        };
        panel.build(&dual("saber_sun", ""), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(click(&mut panel, TARGET_SECOND_TOKEN), PanelAction::None);
        assert!(panel.stage_wish().unwrap().on_second);
        // The second side's own look is chosen: the first's Sun, which it wears.
        panel.build(&dual("saber_sun", ""), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.shader, usize::from(sun_row));
        // A click on the stock row chooses it; Equip puts it on the second only.
        assert_eq!(click(&mut panel, SHADER_BASE), PanelAction::None);
        panel.build(&dual("saber_sun", ""), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::WearSecond(crate::looks::STOCK_SECOND)
        );
        // Back on the first, which wears the Sun: while the second has no choice of its own
        // it follows the first, so Unequip dresses both with the stock blade, as
        // `saberskin` does.
        assert_eq!(click(&mut panel, TARGET_FIRST_TOKEN), PanelAction::None);
        panel.build(&dual("saber_sun", ""), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(panel.shader, usize::from(sun_row));
        assert_eq!(click(&mut panel, WEAR_TOKEN), PanelAction::Wear(""));
        // With a second skin of its own, the first is dressed alone, and the second's
        // Unequip leaves it the stock blade (not the first's skin).
        panel.build(
            &dual("saber_sun", "saber_sun"),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(click(&mut panel, WEAR_TOKEN), PanelAction::Wear(""));
        assert_eq!(click(&mut panel, TARGET_SECOND_TOKEN), PanelAction::None);
        panel.build(
            &dual("saber_sun", "saber_sun"),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(panel.shader, usize::from(sun_row));
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::WearSecond(crate::looks::STOCK_SECOND)
        );
        // A press on a pill never takes hold of the sabers to turn them.
        assert!(!panel.spin.held());
        // The "Both" box: Equip dresses both sabers, the stage shows the shader on both,
        // and with both wearing it, Unequip leaves both the stock blade.
        assert_eq!(click(&mut panel, BOTH_TOKEN), PanelAction::None);
        assert!(panel.stage_wish().unwrap().on_both);
        assert!(!panel.spin.held());
        panel.build(&dual("", "stock"), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(click(&mut panel, SHADER_BASE + sun_row), PanelAction::None);
        panel.build(&dual("", "stock"), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::WearBoth {
                first: "saber_sun",
                second: ""
            }
        );
        panel.build(&dual("saber_sun", ""), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::WearBoth {
                first: "",
                second: ""
            }
        );
        // The stock row worn by both has nothing to do; worn by one, it dresses both.
        assert_eq!(click(&mut panel, SHADER_BASE), PanelAction::None);
        panel.build(&dual("", ""), &fonts.font, [1920.0, 1080.0]);
        assert!(panel.ui.rect_for(WEAR_TOKEN).is_none());
        panel.build(&dual("", "saber_sun"), &fonts.font, [1920.0, 1080.0]);
        assert_eq!(
            click(&mut panel, WEAR_TOKEN),
            PanelAction::WearBoth {
                first: "",
                second: ""
            }
        );
        // Holding one saber, a click on it leaves the box as it is (review of 22/10/2026).
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(click(&mut panel, SABER_TOKEN), PanelAction::None);
        panel.build(&dual("", "saber_sun"), &fonts.font, [1920.0, 1080.0]);
        assert!(panel.stage_wish().unwrap().on_both);
        // Choosing one side unticks the box.
        assert_eq!(click(&mut panel, TARGET_FIRST_TOKEN), PanelAction::None);
        assert!(!panel.stage_wish().unwrap().on_both);
        panel.set_backstage(Backstage::None);
        // Holding one saber again, the target goes back to it.
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert!(!panel.stage_wish().unwrap().on_second);
        assert_eq!(
            click(&mut panel, crate::profile_hub::TOKEN + 1),
            PanelAction::None
        );
        assert_eq!(panel.tab(), Tab::Achievements);
        panel.build(
            &inputs(Holdings::Known(&owned), ""),
            &fonts.font,
            [1920.0, 1080.0],
        );
        assert_eq!(click(&mut panel, BACK_TOKEN), PanelAction::Close);
    }

    /// The Shaders tab's card points to the Forge once the hub's economy is known: Break
    /// down for a shader of the trade held (a medal's never), Craft for one not held; a
    /// click shows the Forge on that section with the shader chosen.
    #[test]
    fn the_shader_card_points_to_the_forge() {
        let owned = [sjk_identity::Unlock {
            id: "saber_tidal".to_owned(),
            count: 3,
            ..sun()
        }];
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        let build = |panel: &mut Panel| {
            panel.build(
                &inputs(Holdings::Known(&owned), ""),
                &fonts.font,
                [1920.0, 1080.0],
            );
        };
        build(&mut panel);
        panel.choose_shader_for_shot(row_of("saber_tidal"));
        build(&mut panel);
        assert!(
            panel.ui.rect_for(FORGE_LINK_TOKEN).is_none(),
            "no economy yet"
        );
        panel.set_trade_known(true);
        build(&mut panel);
        assert!(panel.ui.rect_for(FORGE_LINK_TOKEN).is_some());
        let runs: Vec<_> = panel.ui.text_runs().collect();
        assert!(
            runs.iter()
                .any(|run| run.contains("Break down in the Forge")),
            "{runs:?}"
        );
        assert_eq!(panel.open_forge_link(), PanelAction::None);
        assert_eq!(panel.tab(), Tab::Forge);
        assert_eq!(panel.forge_section(), forge::Section::Break);
        assert_eq!(panel.forge.want.as_deref(), Some("saber_tidal"));
        // A locked shader of the trade points to Craft; a medal's blade nowhere.
        panel.show(Tab::Shaders);
        let locked = (1..rows_shown())
            .find(|&row| {
                shader_skin(row)
                    .is_some_and(|skin| skin.id != "saber_tidal" && crate::economy::in_trade(skin))
            })
            .unwrap();
        panel.choose_shader_for_shot(locked);
        build(&mut panel);
        let runs: Vec<_> = panel.ui.text_runs().collect();
        assert!(runs.iter().any(|run| run.contains("Craft in the Forge")));
        panel.choose_shader_for_shot(row_of("saber_glitch"));
        build(&mut panel);
        assert!(
            panel.ui.rect_for(FORGE_LINK_TOKEN).is_none(),
            "a medal's blade"
        );
    }

    #[test]
    fn the_shaders_list_the_rarest_first_after_the_stock_blade() {
        assert_eq!(shader_skin(0), None, "the stock blade first");
        let tiers: Vec<_> = (1..rows_shown())
            .map(|row| shader_skin(row).unwrap().tier)
            .collect();
        assert!(tiers.windows(2).all(|pair| pair[0] >= pair[1]), "{tiers:?}");
        assert_eq!(shader_skin(1).unwrap().id, "saber_sun");
        assert_eq!(shader_skin(rows_shown()), None);
        for grid in [false, true] {
            let shown = Inputs {
                grid,
                ..inputs(Holdings::Known(&[]), "")
            };
            let panel = drawn(Tab::Shaders, &shown);
            let names: Vec<_> = panel.ui.text_runs().collect();
            let at = |name: &str| names.iter().position(|run| *run == name).unwrap();
            assert!(at("Sun") < at("Void"), "{names:?}");
            let mythical = format!("{}  Mythical", unlockables::Rarity::Mythical.symbol());
            let legendary = format!("{}  Legendary", unlockables::Rarity::Legendary.symbol());
            // Mythical's heading over its shaders; Legendary's below, if the list reaches
            // that far.
            assert!(at(&mythical) < at("Sun"), "{names:?}");
            let legendary = names.iter().position(|run| *run == legendary);
            assert!(legendary.is_none_or(|l| at(&mythical) < l), "{names:?}");
        }
    }

    /// S orders the list by name, the ones held, the tier again (the blades' numbers'
    /// only while they show), the chosen shader kept; a click on a column's head orders
    /// it so too.
    #[test]
    fn s_and_the_heads_sort_the_list_keeping_the_chosen() {
        let held = [sun()];
        let inputs = inputs(Holdings::Known(&held), "");
        let mut panel = drawn(Tab::Shaders, &inputs);
        panel.shader = row_of("saber_void");
        assert_eq!(panel.key_sort(), ShaderSort::Name);
        redraw(&mut panel, &inputs);
        assert_eq!(
            panel.skin_at(panel.shader).map(|skin| skin.id),
            Some("saber_void")
        );
        let names: Vec<_> = panel.shader_list.iter().map(|skin| skin.name).collect();
        assert!(
            names
                .windows(2)
                .all(|pair| pair[0].to_lowercase() <= pair[1].to_lowercase())
        );
        assert_eq!(panel.key_sort(), ShaderSort::Yours);
        redraw(&mut panel, &inputs);
        assert_eq!(panel.shader_list[0].id, "saber_sun", "held first");
        assert_eq!(panel.key_sort(), ShaderSort::Tier, "no numbers shown");
        // The tier's head is a click away.
        let _ = panel.sort_by(ShaderSort::Name);
        redraw(&mut panel, &inputs);
        let head = SORT_BASE;
        let tier = ShaderSort::ALL
            .iter()
            .position(|&sort| sort == ShaderSort::Tier)
            .unwrap();
        let rect = panel
            .ui
            .rect_for(head + tier as u16)
            .expect("the tier's head");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        for event in [
            InputEvent::PointerMove(at),
            InputEvent::PointerPress {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
            InputEvent::PointerRelease {
                position: at,
                button: sjk_ui::PointerButton::Primary,
            },
        ] {
            let _ = panel.handle_pointer(event, true);
        }
        assert_eq!(panel.shader_sort, ShaderSort::Tier);
    }

    /// The panel on Shaders in the grid after one frame.
    fn grid(holdings: Holdings<'_>, setting: &str, viewport: [f32; 2]) -> Panel {
        let mut panel = Panel::new();
        panel.open(Tab::Shaders, true, true, ReturnTarget::MainMenu);
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = Inputs {
            grid: true,
            ..inputs(holdings, setting)
        };
        panel.build(&shown, &fonts.font, viewport);
        panel
    }

    #[test]
    fn v_and_the_switches_change_the_view_and_say_so() {
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&[]), ""));
        assert!(!panel.grid);
        assert_eq!(
            panel.key(KeyCode::KeyV, false, true),
            PanelAction::ShaderView(true)
        );
        assert!(panel.grid);
        assert_eq!(
            panel.key(KeyCode::KeyV, false, true),
            PanelAction::ShaderView(false)
        );
        // V is the Shaders tab's only.
        panel.show(Tab::Medals);
        assert_eq!(panel.key(KeyCode::KeyV, false, true), PanelAction::None);
        // The switches answer a click.
        let mut panel = drawn(Tab::Shaders, &inputs(Holdings::Known(&[]), ""));
        for (token, grid) in [(GRID_VIEW_TOKEN, true), (LIST_VIEW_TOKEN, false)] {
            let rect = panel.ui.rect_for(token).expect("a switch");
            let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.handle_pointer(event, true);
                if next != PanelAction::None {
                    action = next;
                }
            }
            assert_eq!(action, PanelAction::ShaderView(grid));
        }
        // The page follows the setting it is given each frame.
        let panel = grid(Holdings::Known(&[]), "", [1920.0, 1080.0]);
        assert!(panel.grid);
    }

    impl Panel {
        /// S on the list, as `handle_key` takes it: the next order, which it returns.
        fn key_sort(&mut self) -> ShaderSort {
            let _ = self.sort_by(self.shader_sort.next(self.shader_numbers));
            self.shader_sort
        }
    }

    #[test]
    fn the_arrows_move_in_two_directions_on_the_wall() {
        let mut panel = grid(Holdings::Known(&[]), "", [1920.0, 1080.0]);
        assert_eq!(panel.shader, 0);
        let _ = panel.key(KeyCode::ArrowLeft, false, true);
        assert_eq!(panel.shader, 0, "stops at the start");
        let _ = panel.key(KeyCode::ArrowRight, false, true);
        assert_eq!(panel.shader, 1, "on into the first tier's");
        let _ = panel.key(KeyCode::ArrowDown, false, true);
        assert_eq!(panel.shader, 1 + shaders::WALL_COLUMNS);
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 1);
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0, "the stock blade's line, over the headings");
        let _ = panel.key(KeyCode::ArrowUp, false, true);
        assert_eq!(panel.shader, 0, "stops at the first line");
        for _ in 0..rows_shown() {
            let _ = panel.key(KeyCode::ArrowDown, false, true);
        }
        let last = panel.shader_lines.last().copied();
        assert!(
            matches!(last, Some(shaders::Line::Things(start, end)) if (start..end).contains(&panel.shader)),
            "on the last line"
        );
        for _ in 0..rows_shown() {
            let _ = panel.key(KeyCode::ArrowRight, false, true);
        }
        assert_eq!(panel.shader, rows_shown() - 1, "stops at the last tile");
    }

    #[test]
    fn the_wheel_scrolls_the_wall_a_line_and_keeps_the_tile_shown() {
        let mut panel = grid(Holdings::Known(&[]), "", [1920.0, 1080.0]);
        let lines = panel.shader_lines.len();
        let wheel = |panel: &mut Panel, y: f32| {
            let _ = panel.handle_pointer(
                InputEvent::PointerWheel {
                    position: sjk_ui::Vec2::new(500.0, 500.0),
                    delta: sjk_ui::Vec2::new(0.0, y),
                },
                false,
            );
        };
        for _ in 0..lines + 2 {
            wheel(&mut panel, -1.0);
        }
        assert!(panel.shader_first > 0, "scrolled down");
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let shown = Inputs {
            grid: true,
            ..inputs(Holdings::Known(&[]), "")
        };
        panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
        let first = panel.shader_first;
        assert!(
            panel
                .ui
                .rect_for(SHADER_BASE + panel.shader as u16)
                .is_some(),
            "the chosen tile in view"
        );
        for _ in 0..lines + 2 {
            wheel(&mut panel, 1.0);
        }
        assert_eq!(panel.shader_first, 0);
        assert!(first > 0);
    }

    /// Hub report #201: the wheel scrolls on over the tiers' headings, as each frame
    /// draws in between, on the wall and the list, all shaders or the Owned ones.
    #[test]
    fn the_wheel_scrolls_past_the_headings_frame_after_frame() {
        let held: Vec<Unlock> = unlockables::blade_skins()
            .take(80)
            .map(|skin| Unlock {
                id: skin.id.into(),
                count: 1,
                ..sun()
            })
            .collect();
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        let notch = |panel: &mut Panel, y: f32| {
            let _ = panel.handle_pointer(
                InputEvent::PointerWheel {
                    position: sjk_ui::Vec2::new(500.0, 500.0),
                    delta: sjk_ui::Vec2::new(0.0, y),
                },
                false,
            );
        };
        for grid in [false, true] {
            for owned in [false, true] {
                let shown = Inputs {
                    grid,
                    ..inputs(Holdings::Known(&held), "")
                };
                let mut panel = drawn(Tab::Shaders, &shown);
                if owned {
                    let _ = panel.toggle_owned();
                }
                panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
                let lines = panel.shader_lines.len();
                let mut tops = vec![panel.shader_first];
                for _ in 0..lines {
                    notch(&mut panel, -1.0);
                    panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
                    tops.push(panel.shader_first);
                }
                let end = *tops.last().unwrap();
                // The wheel reached the end: the last shader is drawn.
                let last = SHADER_BASE + (panel.shader_count - 1) as u16;
                assert!(
                    panel.ui.rect_for(last).is_some(),
                    "grid {grid}, owned {owned}: {tops:?}"
                );
                // Each notch moved on a line until the end.
                let stop = tops.iter().position(|top| *top == end).unwrap();
                assert!(
                    tops[..=stop].windows(2).all(|pair| pair[1] == pair[0] + 1),
                    "grid {grid}, owned {owned}: {tops:?}"
                );
                for _ in 0..lines {
                    notch(&mut panel, 1.0);
                    panel.build(&shown, &fonts.font, [1920.0, 1080.0]);
                }
                assert_eq!(panel.shader_first, 0, "grid {grid}, owned {owned}: back up");
            }
        }
    }

    #[test]
    fn a_click_on_a_card_chooses_it() {
        let owned = [sun()];
        let mut panel = grid(Holdings::Known(&owned), "", [1920.0, 1080.0]);
        let token = SHADER_BASE + row_of("saber_sun") as u16;
        let rect = panel.ui.rect_for(token).expect("the Sun's card");
        let at = sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
        let start = Instant::now();
        let mut click = |millis: u64| {
            let now = start + std::time::Duration::from_millis(millis);
            let mut action = PanelAction::None;
            for event in [
                InputEvent::PointerMove(at),
                InputEvent::PointerPress {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
                InputEvent::PointerRelease {
                    position: at,
                    button: sjk_ui::PointerButton::Primary,
                },
            ] {
                let next = panel.pointer_at(event, true, now);
                if next != PanelAction::None {
                    action = next;
                }
            }
            action
        };
        assert_eq!(click(0), PanelAction::None);
    }
    /// Hub report #245: a click on the Loadouts button opens the card, as P does, on
    /// every tab that offers it.
    #[test]
    fn a_click_on_the_loadouts_button_opens_the_card() {
        // Every blade skin held: the wall is as full as it gets.
        let owned: Vec<Unlock> = unlockables::blade_skins()
            .map(|skin| Unlock {
                id: skin.id.into(),
                ..sun()
            })
            .collect();
        let fonts = crate::text::load_modern(1.0, None).expect("Inter");
        for tab in [Tab::Shaders, Tab::Tags, Tab::Nameplates] {
            for grid in [false, true] {
                for (size, stage) in [[1920.0, 1080.0], [1280.0, 720.0], [2560.0, 1440.0]]
                    .into_iter()
                    .flat_map(|size| {
                        [
                            Backstage::None,
                            Backstage::World { head: None },
                            Backstage::Preview { ready: true },
                        ]
                        .map(|stage| (size, stage))
                    })
                {
                    let shown = Inputs {
                        grid,
                        ..inputs(Holdings::Known(&owned), "")
                    };
                    let mut panel = Panel::new();
                    panel.open(tab, true, true, ReturnTarget::MainMenu);
                    // In the game the page always has the stage behind it.
                    panel.set_backstage(stage);
                    panel.build(&shown, &fonts.font, size);
                    let rect = panel.ui.rect_for(loadouts::OPEN_TOKEN).unwrap_or_else(|| {
                        panic!("{tab:?} grid {grid} {size:?} {stage:?}: no button area")
                    });
                    let at =
                        sjk_ui::Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5);
                    for event in [
                        InputEvent::PointerMove(at),
                        InputEvent::PointerPress {
                            position: at,
                            button: sjk_ui::PointerButton::Primary,
                        },
                        InputEvent::PointerRelease {
                            position: at,
                            button: sjk_ui::PointerButton::Primary,
                        },
                    ] {
                        let _ = panel.handle_pointer(event, true);
                    }
                    assert!(
                        panel.loadouts.is_open(),
                        "{tab:?} grid {grid} {size:?} {stage:?}: the click left the card shut"
                    );
                }
            }
        }
    }
}
