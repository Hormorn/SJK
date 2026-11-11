//! MMO-style nameplates over players (and, optionally, NPCs).
//!
//! Players choose how a plate looks (`cg_nameplateStyle`, [`Style`], 31/10/2026): the
//! holo frame, the capsule or the classic plate, the three designs SJK has had. All
//! three share what a plate says; only its shape differs.
//!
//! - **Holo frame** (the default, design C, Sol's pick of 28/10/2026): a hexagon holding
//!   the player's picture (their SJK hub picture, else their initial on their colour, or
//!   for a player without SJK their model's head icon) over the left end of a plate with
//!   a cut corner, which holds their name. The frame's colour tells an SJK player (holo
//!   blue) from one without SJK (grey), a team or a duel; a sharer's plate has a gold
//!   corner. Closer, a sharer's health, shield and Force bars fade in under the name (a
//!   dotted line without them); far away the plate fades and a small hexagon and the
//!   name stay over a hairline.
//! - **Capsule** ([`capsule`], design E of 28/10/2026): a pill holding the picture and
//!   the name, with a sharer's bars on a tray tucked under it.
//! - **Classic** ([`classic`], the plates before 28/10/2026): the name over a framed plate
//!   of rounded bars, gold-edged for a sharer, grey bars for a player who does not share.
//!
//! No weapon since 28/10/2026. The Force powers a player has on stand over the plate in
//! square cells. A dark name (black `^0`) is edged in a light halo so it reads on the
//! see-through plate ([`crate::text::CodePalette::Haloed`]).
//! Names come from the chat roster and are drawn in the classic HUD font with
//! their colour codes; the layout maths is in [`super::nameplate_math`]. A sharer's
//! health and shield are estimated between the values they share, once per snapshot,
//! in [`super::vitals_estimate`], each as a range whose uncertainty the bar shows as a
//! grey haze; their Force is the last value they shared, not estimated (17/10/2026). The plain
//! TaystJK names stay in [`super::identification`] (`cg_drawPlayerNames`); a
//! nameplate replaces them.
use super::estimate::Range;
use super::force_streams;
use super::identification::{Camera, info_number, unoccluded};
use super::nameplate_math::{self as math, Holo, HoloSpec, Rows};
use super::vitals_estimate;
use crate::{TextVertex, UiFont, chat::ChatOverlay, console::ViewerConsole};
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use sjk_client::TeamInfoTable;
use sjk_game_jka::force_powers::{FP_DRAIN, FP_LIGHTNING};
use sjk_protocol::{GameState, Snapshot};
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_ui::{
    Color, DrawCommand, DrawList, FontWeight, Gradient, Rect, TextAlign, TextId, TextOverflow,
    TextureId,
};

pub(crate) mod capsule;
pub(crate) mod classic;

/// Text ids below this are player slots; above it, `NPC_TEXT + class_t`.
const NPC_TEXT: u32 = 1024;
/// Text id of the "?" over a bar the estimate cannot fill.
const UNKNOWN_TEXT: u32 = 1000;
/// Text id of the mode's name, shown for a moment when the mode changes.
const MODE_TEXT: u32 = 1001;
/// Text ids from this one up are the initials on the plates' stand-ins, by plate.
const INITIAL_TEXT: u32 = 2000;
/// How long the mode's name shows, and the last part of that over which it fades.
const MODE_SHOWN: i64 = 1_500;
const MODE_FADE: i64 = 400;
/// How long the player last aimed at keeps their bars in [`Mode::Target`].
const FOCUS_LINGER: i64 = 3_000;
/// `cg_nameplateBars` 3: bars only on the player aimed at (never a duel opponent's,
/// [`Duel`]).
const BARS_TARGET: i64 = 3;

/// Console command that cycles the nameplate modes, or sets one (`nameplates target`).
pub(crate) const COMMAND: &str = "nameplates";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Cycle the nameplates: off, names only, bars on your target, bars on everyone (or name one)";

/// The nameplate modes the `nameplates` command (V by default) cycles through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// No nameplates (`cg_nameplate 0`).
    Off,
    /// Names only (`cg_nameplateBars 0`).
    Names,
    /// Names, and bars on the player aimed at (`cg_nameplateBars 3`).
    Target,
    /// Names and bars on everyone (`cg_nameplateBars 2`).
    All,
}

impl Mode {
    /// The mode `cg_nameplate` and `cg_nameplateBars` make; bars on allies only (1),
    /// which the cycle skips, counts as between names and target.
    pub(crate) fn of(enabled: bool, bars: i64) -> Self {
        match (enabled, bars) {
            (false, _) => Self::Off,
            (true, 2) => Self::All,
            (true, BARS_TARGET) => Self::Target,
            (true, 1) => Self::Names,
            _ => Self::Names,
        }
    }

    /// The next mode V gives: off, names, target, everyone, and round again.
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Off => Self::Names,
            Self::Names => Self::Target,
            Self::Target => Self::All,
            Self::All => Self::Off,
        }
    }

    /// `cg_nameplate` and, when on, `cg_nameplateBars` for this mode.
    pub(crate) fn settings(self) -> (bool, Option<i64>) {
        match self {
            Self::Off => (false, None),
            Self::Names => (true, Some(0)),
            Self::Target => (true, Some(BARS_TARGET)),
            Self::All => (true, Some(2)),
        }
    }

    /// A mode named by the `nameplates` command's word.
    pub(crate) fn named(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "off" | "0" => Some(Self::Off),
            "names" | "name" | "1" => Some(Self::Names),
            "target" | "2" => Some(Self::Target),
            "all" | "everyone" | "3" => Some(Self::All),
            _ => None,
        }
    }

    /// What the screen says when the mode is chosen.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Off => "Nameplates: off",
            Self::Names => "Nameplates: names only",
            Self::Target => "Nameplates: names, bars on your target",
            Self::All => "Nameplates: names and bars on everyone",
        }
    }
}

/// The nameplates' look (`cg_nameplateStyle`, 31/10/2026): the three designs SJK has
/// had, for players to choose from. Out-of-range values read as the holo frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Style {
    /// The holo frame (design C, 28/10/2026), the default.
    #[default]
    Holo,
    /// The capsule (design E, 28/10/2026): a pill with the picture, a tray of bars.
    Capsule,
    /// The plates before 28/10/2026: the name over a framed plate of bars.
    Classic,
}

impl Style {
    /// Every style, in `cg_nameplateStyle`'s order.
    pub(crate) const ALL: [Self; 3] = [Self::Holo, Self::Capsule, Self::Classic];

    /// The style `cg_nameplateStyle` names.
    pub(crate) fn of(value: i64) -> Self {
        match value {
            1 => Self::Capsule,
            2 => Self::Classic,
            _ => Self::Holo,
        }
    }

    /// Its `cg_nameplateStyle` value.
    pub(crate) fn value(self) -> i64 {
        match self {
            Self::Holo => 0,
            Self::Capsule => 1,
            Self::Classic => 2,
        }
    }

    /// Its name in Settings and the Collection.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Holo => "Holo",
            Self::Capsule => "Capsule",
            Self::Classic => "Classic",
        }
    }
}

/// A health or shield range this wide (shares of a full bar) is too unsure to show:
/// the bar dims and a "?" stands over it until something narrows it.
const UNSURE_WIDTH: f32 = 0.6;
/// A shield whose high bound is under this share is known to be empty.
const EMPTY_SHARE: f32 = 0.005;
/// `ET_PLAYER` and `ET_NPC` entity types, and `EF_DEAD`.
const ET_PLAYER: u8 = 1;
const ET_NPC: u8 = 13;
const EF_DEAD: u32 = 2;
/// The first team game type.
const GT_TEAM: i32 = 6;
/// `GT_DUEL`, `GT_POWERDUEL` and `TEAM_SPECTATOR`.
const GT_DUEL: i32 = 3;
const GT_POWERDUEL: i32 = 4;
const TEAM_SPECTATOR: i32 = 3;
/// Tags drawn at most: every client slot, plus a bounded number of NPCs.
const MAX_PLAYER_TAGS: usize = 32;
const MAX_NPC_TAGS: usize = 16;
const MAX_TAGS: usize = MAX_PLAYER_TAGS + MAX_NPC_TAGS;
/// Entity numbers are ten bits on the wire.
const ENTITY_SLOTS: usize = 1024;
/// A plate unseen this long fades in again instead of resuming.
const STALE_MILLIS: i64 = 250;
/// Opacity of a plate behind a wall when `cg_nameplateWalls` is on.
const WALL_OPACITY: f32 = 0.35;
/// Allowed overshoot of the screen (in half-screens) before a plate is dropped.
const SCREEN_MARGIN: f32 = 1.15;
/// Conservative vertical extent of a nameplate above its bottom, in unscaled design pixels.
/// Keep a moved plate clear of a team logo until the whole plate has passed it.
const PLATE_COLLISION_HEIGHT: f32 = 100.0;
/// Smallest size, as a share of full, a plate at the end of its range shrinks to.
const MIN_SCALE: f32 = 0.6;
/// Opacity of a name too far for a plate.
const FAR_NAME_OPACITY: f32 = 0.85;
/// Full health and armour, for teammates (`tinfo` sends points, not a share).
const FULL_POINTS: f32 = 100.0;
/// Power icons shown over a name at most.
const MAX_ICONS: usize = 4;
/// Force powers whose icon shows while they are on, dark side first, then the
/// light side and neutral ones, as `forcePowers_t` indices. Push, pull, jump and
/// the saber powers are left out: they last a moment and would only flicker.
const ICON_POWERS: [u8; 11] = [7, 6, 13, 8, 9, 10, 2, 0, 12, 5, 14];
/// The colour of players who follow no team: no team tints their frame.
const NEUTRAL_ACCENT: Color = Color::new(0.78, 0.82, 0.9, 0.9);
/// The holo frame of an SJK player out of team games: its hexagon and its edge.
pub(crate) const HOLO_HEX: Color = Color::new(110.0 / 255.0, 210.0 / 255.0, 1.0, 0.85);
pub(crate) const HOLO_EDGE: Color = Color::new(110.0 / 255.0, 210.0 / 255.0, 1.0, 0.75);
/// The grey frame of a player without SJK, and their far plate's hairline.
const GREY_HEX: Color = Color::new(170.0 / 255.0, 178.0 / 255.0, 190.0 / 255.0, 0.7);
const GREY_EDGE: Color = Color::new(170.0 / 255.0, 178.0 / 255.0, 190.0 / 255.0, 0.6);
const GREY_HAIRLINE: Color = Color::new(110.0 / 255.0, 118.0 / 255.0, 130.0 / 255.0, 0.8);
/// The dotted line under the name of a plate without bars.
const DOTTED: Color = Color::new(160.0 / 255.0, 170.0 / 255.0, 185.0 / 255.0, 0.4);
/// The gold corner of a player who shares their bars (`cg_nameplate`, See and share
/// nameplates), the verified badge's gold.
pub(crate) const SHARED_GOLD: Color = Color::new(1.0, 209.0 / 255.0, 64.0 / 255.0, 1.0);
/// The plate's opacity (Sol, 28/10/2026: more see-through than the design's 0.78), the
/// scan lines' over it and the bars' tracks', all in the frame's colour.
pub(crate) const PLATE_ALPHA: f32 = 0.55;
pub(crate) const SCAN_ALPHA: f32 = 0.05;
pub(crate) const TRACK_ALPHA: f32 = 0.12;
/// The edge of NPC plates, a paler gold.
const NPC_ACCENT: Color = Color::new(0.95, 0.8, 0.35, 0.7);
/// The grey haze over a bar's uncertain stretch, at its thickest.
const HAZE: Color = Color::new(0.8, 0.82, 0.86, 0.6);
/// A range narrower than this share of the bar is drawn sharp.
const HAZE_MIN_WIDTH: f32 = 0.02;
/// The power icons' square cells' backdrop (Sol, 28/10/2026: squares, not round
/// chips), and their edge in the frame's colour.
const CHIP_FILL: Color = Color::new(0.03, 0.04, 0.06, 0.55);
const CHIP_EDGE_ALPHA: f32 = 0.5;
/// A cell's corners, and the room round its icon, in the design's pixels.
const CHIP_CORNER: f32 = 2.0;
const CHIP_INSET: f32 = 1.0;
/// The picture's place while a head icon is being cut.
const BLANK_PICTURE: Color = Color::new(0.16, 0.18, 0.22, 0.9);
/// Milliseconds before a head icon that could not be cut is tried again.
const HEAD_RETRY: i64 = 2_000;
/// The duel badge's least opacity: a far plate's name dims, the badge stays clear.
const DUEL_BADGE_OPACITY: f32 = 0.9;
/// Milliseconds between reads of which players are verified.
const VERIFIED_EVERY: i64 = 1_000;
/// Milliseconds between reads of what the other players share: their values come
/// about four times a second.
const SHARED_EVERY: i64 = 100;

/// What one slot's player shares (`PROTOCOL.md`, "Vitals"), their claim matching the
/// name the game shows there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SharedSlot {
    /// They see everyone's estimated bars.
    pub(crate) estimates: bool,
    /// Their values, while they have some (alive in a match).
    pub(crate) vitals: Option<sjk_identity::Vitals>,
}

/// The picture of an SJK player's plate, from the hub's players on the server (read
/// once a second, [`crate::player_identity::slot_pictures`]): their key id and their
/// picture's version as the hub sends them, and their stand-in, the initial of their
/// hub name on their colour ([`crate::profile_card::stand_in_colour`]), shown while
/// the picture loads and when they have none.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SlotPicture {
    key_id: [u8; 16],
    version: Option<[u8; 16]>,
    initial: Initial,
    colour: Color,
}

impl SlotPicture {
    /// The picture of the player with key `key_id`, whose picture is at `version`
    /// (empty for none) and whose hub name is `name`; `None` for a key id the hub would
    /// not send.
    pub(crate) fn new(key_id: &str, version: &str, name: &str) -> Option<Self> {
        if !sjk_identity::avatar::valid_key_id(key_id) {
            return None;
        }
        let key: [u8; 16] = key_id.as_bytes().try_into().ok()?;
        let version = sjk_identity::avatar::valid_version(version)
            .then(|| version.as_bytes().try_into().ok())
            .flatten();
        Some(Self {
            key_id: key,
            version,
            initial: Initial::new(crate::profile_card::initial(name)),
            colour: crate::profile_card::stand_in_colour(key_id, name),
        })
    }

    /// The hub picture to ask the cache for, if they have one.
    fn wanted(&self) -> crate::avatars::Wanted<'_> {
        fn text(bytes: &[u8; 16]) -> Option<&str> {
            std::str::from_utf8(bytes).ok()
        }
        match (text(&self.key_id), self.version.as_ref().and_then(text)) {
            (Some(key_id), Some(version)) => crate::avatars::Wanted::Hub(key_id, version),
            _ => crate::avatars::Wanted::Nothing,
        }
    }
}

/// Each slot's plate picture ([`SlotPicture`]); `None` for a player the hub does not
/// know there.
pub(crate) type SlotPictures = [Option<SlotPicture>; MAX_PLAYER_TAGS];

/// A stand-in's initial, kept without an allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Initial {
    bytes: [u8; 4],
    len: u8,
}

impl Initial {
    fn new(initial: char) -> Self {
        let mut bytes = [0; 4];
        let len = initial.encode_utf8(&mut bytes).len() as u8;
        Self { bytes, len }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

/// What a plate shows as its picture.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Picture {
    /// None: an NPC's plate.
    #[default]
    Nothing,
    /// A picture in the atlas: the player's hub picture, or their model's head icon
    /// cut round ([`crate::avatars::insert_local`]).
    Texture(TextureId),
    /// The plate's initial on this colour: an SJK player without a picture (or before
    /// it loads), or a player without SJK whose model has no head icon.
    StandIn(Color),
    /// An empty disc, while a head icon is being cut.
    Blank,
}

/// A player without SJK's head icon, cut round for their plate: the key it is kept
/// under (from their `model`), when it was last asked for, and, once it could not be
/// had, the stand-in their plate shows instead.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Head {
    key: u64,
    asked: Option<i64>,
    missing: Option<(Initial, Color)>,
}

/// The key a model's head icon is kept under in the pictures' cache (FNV-1a of its
/// `model` value): players wearing one model share one picture.
fn head_key(model: &[u8]) -> u64 {
    model.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The nameplates' look ([`Style`]).
pub(crate) const STYLE_CVAR: &str = "cg_nameplateStyle";

/// How solid the plates are drawn (`cg_nameplateAlpha`, 04/11/2026, from #ideas): the
/// whole plate, in every style, times its own distance and wall fade.
pub(crate) const OPACITY_CVAR: &str = "cg_nameplateAlpha";

/// The lowest `cg_nameplateAlpha` honoured: `cg_nameplate 0` is how plates go away.
const MIN_OPACITY: f32 = 0.1;

/// Register the nameplate settings.
pub(super) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    cvars.register(CvarDefinition::new(
        "cg_nameplate",
        // Off by default (17/10/2026): it shares the player's own values too.
        false,
        CvarFlags::ARCHIVE,
        "See and share nameplates: MMO-style nameplates over players (they replace cg_drawPlayerNames), and your own health, shield and Force for the other SJK players of your server",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateRange",
        3000_i64,
        CvarFlags::ARCHIVE,
        "Distance in units out to which nameplates show",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateNear",
        1000_i64,
        CvarFlags::ARCHIVE,
        "Distance in units inside which a nameplate shows its bars",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateScale",
        0.5_f64,
        CvarFlags::ARCHIVE,
        "Nameplate text size",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateOffset",
        0.0_f64,
        CvarFlags::ARCHIVE,
        "Vertical offset of nameplates above players, in game units",
    ))?;
    cvars.register(CvarDefinition::new(
        OPACITY_CVAR,
        1.0_f64,
        CvarFlags::ARCHIVE,
        "Nameplate opacity, from 0.1 (faint) to 1 (solid); the distance and wall fades still apply",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateBars",
        2_i64,
        CvarFlags::ARCHIVE,
        "Nameplate bars: 0 none, 1 allies only, 2 everyone, 3 the player you aim at; never a duel opponent's",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateIcons",
        true,
        CvarFlags::ARCHIVE,
        "Icons of the Force powers a player has on, over the nameplate when close",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateDuel",
        true,
        CvarFlags::ARCHIVE,
        "Crossed-sabers badge before the name of a player in a private duel",
    ))?;
    cvars.register(CvarDefinition::new(
        STYLE_CVAR,
        0_i64,
        CvarFlags::ARCHIVE,
        "Nameplate look: 0 the holo frame, 1 the capsule, 2 the classic plate",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateOutline",
        1_i64,
        CvarFlags::ARCHIVE,
        "How names are set off from the nameplate: 0 a shadow, 1 a faint halo on dark letters, 2 a strong halo and a dark outline on the rest",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateWalls",
        false,
        CvarFlags::ARCHIVE,
        "Show nameplates dimmed through walls",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateNpcs",
        false,
        CvarFlags::ARCHIVE,
        "NPC names; health requires active Sense 3 unless the mapper enables it",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateSelf",
        false,
        CvarFlags::ARCHIVE,
        "Your own nameplate over your head in third person, with your real bars",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateHideDuelers",
        false,
        CvarFlags::ARCHIVE,
        "Hide the whole nameplate of players the server marks as private duellers",
    ))?;
    cvars.register(CvarDefinition::new(
        "cg_nameplateDebug",
        false,
        CvarFlags::NONE,
        "Log what the server sends about other players every two seconds",
    ))?;
    Ok(())
}

/// See and share nameplates is one setting, `cg_nameplate` (17/10/2026): seeing other
/// players' nameplates shares the player's own health, shield and Force, and turning
/// the nameplates off stops both. It replaces `cg_nameplateShare` and the hold that kept
/// it on while the plates showed bars (Sol's rule, 15/10/2026: seeing bars means
/// sharing).
pub(crate) fn see_and_share(console: &ViewerConsole) -> bool {
    console.bool_cvar("cg_nameplate") == Some(true)
}

/// Settings, sampled once per frame.
#[derive(Clone, Copy)]
struct Settings {
    enabled: bool,
    range: f32,
    near: f32,
    scale: f32,
    /// Vertical offset above the player's head, in game units.
    offset: f32,
    /// The plates' opacity (`cg_nameplateAlpha`), [`MIN_OPACITY`] to 1.
    opacity: f32,
    bars: i64,
    walls: bool,
    npcs: bool,
    icons: bool,
    /// The duel badge before a duellist's name (`cg_nameplateDuel`).
    duel_badge: bool,
    /// How the names are set off from the plate (`cg_nameplateOutline`).
    outline: crate::text::CodePalette,
    /// The plates' look (`cg_nameplateStyle`).
    style: Style,
    friends: bool,
    /// The local player's own plate, in third person.
    own: bool,
    /// Suppress plates of server-marked private-duel participants.
    hide_duelers: bool,
    debug: bool,
    /// The local player's drain and lightning levels from its Force profile.
    drain_level: Option<u8>,
    lightning_level: Option<u8>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            range: 3000.0,
            near: 1000.0,
            scale: 0.5,
            offset: 0.0,
            opacity: 1.0,
            bars: 2,
            walls: false,
            npcs: false,
            icons: true,
            duel_badge: true,
            outline: crate::text::CodePalette::Haloed,
            style: Style::Holo,
            friends: true,
            own: false,
            hide_duelers: false,
            debug: false,
            drain_level: None,
            lightning_level: None,
        }
    }
}

/// One visible plate, collected in `update` and drawn in `append`.
#[derive(Clone, Copy)]
struct Entry {
    /// Player slot, or NPC entity number.
    number: u16,
    /// `class_t` for an NPC, zero for a player.
    npc_class: u8,
    /// Screen position of the point above the head.
    point: [f32; 2],
    distance: f32,
    /// Size multiplier from distance.
    scale: f32,
    /// Smoothed opacity, distance fade included.
    alpha: f32,
    /// How much of the plate shows, 0 (far: name only) to 1.
    detail: f32,
    /// How close the player is, 0 (far) to 1: the same ramp, whether or not there are bars.
    proximity: f32,
    /// Force powers to show icons for (`forcePowers_t` indices), `power_count` of them.
    powers: [u8; MAX_ICONS],
    power_count: u8,
    /// Shares of a full bar.
    health: Option<Range>,
    shield: Option<Range>,
    force: Option<Range>,
    accent: Color,
    names_allowed: bool,
    /// The mark after the name: the gold badge of a player the SJK hub's operator vouches
    /// for, or their trust mark (`ui_renderer::mark_texture`).
    mark: Option<sjk_ui::TextureId>,
    /// The icon of another client they play on, small after the mark
    /// (`player_identity::slot_clients`): none for SJK.
    client: Option<sjk_ui::TextureId>,
    /// They share their bars, of their own will or because they see everyone's: a
    /// gold contour.
    shared: bool,
    /// A player who does not share: their bars are drawn grey and empty.
    withheld: bool,
    /// In a private duel (a player entity's `bolt1`, its `duelInProgress`): the duel
    /// badge before the name.
    duelling: bool,
    /// The duel's badge colour ([`super::duel_pairs`]): both duellists of one duel
    /// share it.
    duel_colour: u8,
    /// The picture in the hexagon ([`State::place_pictures`]), and the initial a
    /// stand-in shows.
    picture: Picture,
    initial: Initial,
    /// The SJK hub knows them on this server: a player without SJK has a grey frame.
    sjk: bool,
}

impl Entry {
    fn rows(&self) -> Rows {
        Rows {
            health: self.health.is_some(),
            shield: self.shield.is_some(),
            force: self.force.is_some(),
        }
    }
}

/// Smoothed opacity of one entity's plate.
#[derive(Clone, Copy, Default)]
struct Fade {
    alpha: f32,
    seen: i64,
}

/// Fixed-capacity plate state: at most 32 clients and 16 NPCs, no owned name strings.
pub(crate) struct State {
    /// Shapes and text IDs submitted through the normal HUD renderer.
    pub(crate) list: DrawList,
    settings: Settings,
    entries: Vec<Entry>,
    fades: Box<[Fade; ENTITY_SLOTS]>,
    last_update: i64,
    last_debug: i64,
    streams: force_streams::Tracker,
    vitals: vitals_estimate::Estimator,
    /// The health, shield and Force colours of the HUD in use, which the bars take.
    colors: BarColors,
    /// Each slot's mark after its name (`player_identity::slot_marks`), and when they were
    /// last read.
    marks: crate::player_identity::SlotMarks,
    marks_read: Option<i64>,
    /// Each slot's client icon (`player_identity::slot_clients`), read with the marks.
    clients: crate::player_identity::SlotMarks,
    /// What each slot's player shares (`PROTOCOL.md`, "Vitals"), and when it was last
    /// read.
    shared: [Option<SharedSlot>; MAX_PLAYER_TAGS],
    shared_read: Option<i64>,
    /// The hub has the player's own sharing: without it, no shared or estimated bars.
    sharing_live: bool,
    /// The local player is in a duel ([`Duel::of`]): their own values are not sent.
    duelling: bool,
    /// The server time of the shared values each slot's estimates were last
    /// corrected with: each value corrects them once.
    anchored: [Option<i32>; MAX_PLAYER_TAGS],
    /// Where the local player stands, while their own plate may show (third person).
    own_origin: Option<Vec3>,
    /// The player the crosshair was last on, and when.
    focus: Option<(u16, i64)>,
    /// The mode just chosen and when, to show its name for a moment.
    announced: Option<(Mode, i64)>,
    /// Each slot's team logo range this frame ([`super::team_logo`]), to avoid covering it.
    logo_bounds: [Option<(f32, f32)>; MAX_PLAYER_TAGS],
    /// Each slot's duel colour this frame ([`super::duel_pairs`]).
    duel_colours: [Option<u8>; MAX_PLAYER_TAGS],
    /// Each slot's SJK picture, read with the marks ([`Self::refresh_marks`]).
    pictures: SlotPictures,
    /// Each slot's head icon, for a player without SJK, and the one to cut this frame
    /// ([`Self::head_to_cut`]): its slot and key.
    heads: [Head; MAX_PLAYER_TAGS],
    head_wanted: Option<(u16, u64)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            list: DrawList::new(MAX_TAGS * 40 + 8),
            settings: Settings::default(),
            entries: Vec::with_capacity(MAX_TAGS),
            fades: Box::new([Fade::default(); ENTITY_SLOTS]),
            last_update: 0,
            last_debug: 0,
            streams: force_streams::Tracker::default(),
            vitals: vitals_estimate::Estimator::default(),
            colors: BarColors::default(),
            marks: [None; 32],
            marks_read: None,
            clients: [None; 32],
            shared: [None; MAX_PLAYER_TAGS],
            shared_read: None,
            sharing_live: false,
            duelling: false,
            anchored: [None; MAX_PLAYER_TAGS],
            own_origin: None,
            focus: None,
            announced: None,
            logo_bounds: [None; MAX_PLAYER_TAGS],
            duel_colours: [None; MAX_PLAYER_TAGS],
            pictures: [None; MAX_PLAYER_TAGS],
            heads: [Head::default(); MAX_PLAYER_TAGS],
            head_wanted: None,
        }
    }
}

impl State {
    /// Sample settings once, outside text emission.
    pub(crate) fn sample(&mut self, console: Option<&ViewerConsole>) {
        let flag =
            |name: &str, default: bool| console.and_then(|c| c.bool_cvar(name)).unwrap_or(default);
        let number = |name: &str, default: f32| {
            console
                .and_then(|c| c.integer_cvar(name))
                .map_or(default, |value| value as f32)
        };
        let profile = console.and_then(ViewerConsole::own_force_allocation);
        self.settings = Settings {
            enabled: flag("cg_nameplate", false),
            range: number("cg_nameplaterange", 3000.0).clamp(500.0, 10_000.0),
            near: number("cg_nameplatenear", 1000.0).clamp(0.0, 10_000.0),
            scale: crate::cgame_options::scalar(console, "cg_nameplatescale", 0.5).clamp(0.1, 3.0),
            offset: crate::cgame_options::scalar(console, "cg_nameplateoffset", 0.0)
                .clamp(-100.0, 40.0),
            opacity: crate::cgame_options::scalar(console, "cg_nameplatealpha", 1.0)
                .clamp(MIN_OPACITY, 1.0),
            bars: console
                .and_then(|c| c.integer_cvar("cg_nameplatebars"))
                .unwrap_or(2),
            walls: flag("cg_nameplatewalls", false),
            npcs: flag("cg_nameplatenpcs", false),
            icons: flag("cg_nameplateicons", true),
            duel_badge: flag("cg_nameplateduel", true),
            outline: crate::text::CodePalette::nameplate(
                console
                    .and_then(|c| c.integer_cvar("cg_nameplateoutline"))
                    .unwrap_or(1),
            )
            .white_outlined(crate::text::CodePalette::white_outline_on(console)),
            style: Style::of(
                console
                    .and_then(|c| c.integer_cvar("cg_nameplatestyle"))
                    .unwrap_or(0),
            ),
            friends: flag("cg_drawfriend", true),
            own: flag("cg_nameplateself", false),
            hide_duelers: flag("cg_nameplatehideduelers", false),
            debug: flag("cg_nameplatedebug", false),
            drain_level: profile.as_ref().map(|profile| profile.levels[FP_DRAIN]),
            lightning_level: profile.as_ref().map(|profile| profile.levels[FP_LIGHTNING]),
        };
    }

    /// Whether nameplates replace the plain overhead names.
    pub(crate) fn enabled(&self) -> bool {
        self.settings.enabled
    }

    /// Take the bars' colours from the HUD in use (its health, armour and Force
    /// meters); `None` for one it names none for (the game-data HUD draws
    /// pictures), which gets the retail colour.
    pub(crate) fn set_hud_colors(
        &mut self,
        health: Option<Color>,
        shield: Option<Color>,
        force: Option<Color>,
    ) {
        self.colors = BarColors {
            health: health.unwrap_or(math::HEALTH_COLOR),
            shield: shield.unwrap_or(math::SHIELD_COLOR),
            force: force.unwrap_or(math::FORCE_COLOR),
        };
    }

    /// Read each slot's mark, client icon and SJK picture (`read` asks the identity
    /// service) at most once a second, so the names it compares are not rebuilt every
    /// frame.
    pub(crate) fn refresh_marks(
        &mut self,
        now: i64,
        read: impl FnOnce() -> (
            crate::player_identity::SlotMarks,
            crate::player_identity::SlotMarks,
            SlotPictures,
        ),
    ) {
        if self
            .marks_read
            .is_some_and(|last| (0..VERIFIED_EVERY).contains(&(now - last)))
        {
            return;
        }
        self.marks_read = Some(now);
        (self.marks, self.clients, self.pictures) = read();
    }

    /// Slot `number`'s mark.
    fn mark(&self, number: impl Into<usize>) -> Option<sjk_ui::TextureId> {
        self.marks.get(number.into()).copied().flatten()
    }

    /// Slot `number`'s client icon.
    fn client(&self, number: impl Into<usize>) -> Option<sjk_ui::TextureId> {
        self.clients.get(number.into()).copied().flatten()
    }

    /// Read what the other SJK players share and whether the hub has the player's own
    /// sharing (`read` asks the identity service) at most every [`SHARED_EVERY`] ms,
    /// and correct the estimates with each new value.
    pub(crate) fn refresh_shared(
        &mut self,
        now: i64,
        read: impl FnOnce() -> ([Option<SharedSlot>; MAX_PLAYER_TAGS], bool),
    ) {
        if self
            .shared_read
            .is_some_and(|last| (0..SHARED_EVERY).contains(&(now - last)))
        {
            return;
        }
        self.shared_read = Some(now);
        (self.shared, self.sharing_live) = read();
        for (slot, shared) in self.shared.iter().enumerate() {
            let Some(values) = shared.and_then(|shared| shared.vitals) else {
                continue;
            };
            if self.anchored[slot] == Some(values.t) {
                continue;
            }
            self.anchored[slot] = Some(values.t);
            let slot = slot as u16;
            self.vitals
                .anchor(slot, values.t, values.health as f32, values.armor as f32);
        }
    }

    /// The Force player `number` last shared, as a share of a full pool: what their
    /// nameplate's Force bar shows (`None` while they share nothing).
    fn shared_force(&self, number: u16) -> Option<Range> {
        let values = self
            .shared
            .get(usize::from(number))
            .copied()
            .flatten()?
            .vitals?;
        Some(Range::exact(
            (values.force as f32 / values.max_force.max(1) as f32).clamp(0.0, 1.0),
        ))
    }

    /// What the player shares this frame: their own health, shield and Force while
    /// alive in a match, whenever their nameplates are on ([`see_and_share`]).
    pub(crate) fn sharing(&self, snapshot: Option<&Snapshot>) -> sjk_identity::Sharing {
        let settings = &self.settings;
        let share = settings.enabled;
        // Nothing in a duel, whoever reads it (Milamber's rule, 18/10/2026).
        let vitals = snapshot
            .filter(|snapshot| {
                share
                    && !self.duelling
                    && snapshot.player.health() > 0
                    && !snapshot.player.is_spectator()
            })
            .map(|snapshot| sjk_identity::Vitals {
                t: snapshot.server_time,
                health: snapshot.player.health(),
                armor: snapshot.player.armor(),
                force: i32::from(snapshot.player.force_power()),
                max_health: snapshot.player.max_health().max(1),
                // The protocol does not carry `fd.forcePowerMax`; a full pool is 100.
                max_force: 100,
            });
        sjk_identity::Sharing {
            share,
            // `cg_nameplateEstimate` is gone (17/10/2026): nobody's bars are estimated
            // without their sharing, so this client never says it sees them.
            estimates: false,
            vitals,
        }
    }

    /// The player the crosshair is on at `now`, if any, for [`Mode::Target`]: they keep
    /// their bars for a few seconds after the crosshair leaves them.
    pub(crate) fn set_focus(&mut self, target: Option<u16>, now: i64) {
        if let Some(client) = target {
            self.focus = Some((client, now));
        }
    }

    /// Show `mode`'s name on screen for a moment.
    pub(crate) fn announce(&mut self, mode: Mode) {
        self.announced = Some((mode, self.last_update));
    }

    /// Where the local player stands, for their own plate; `None` hides it (first
    /// person, where it would sit inside the camera).
    pub(crate) fn set_own_origin(&mut self, origin: Option<[f32; 3]>) {
        self.own_origin = origin.map(Vec3::from_array);
    }

    /// Feed one accepted snapshot to the estimates. Every snapshot is observed
    /// once, in order, whether or not plates are shown, so no event is missed.
    pub(crate) fn observe_snapshot(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
    ) {
        let mode = info_number(game.config_string(0), "g_gametype");
        self.duelling = Duel::of(snapshot, game, mode).active();
        // The events first (drain names its victims), then the shots, which the
        // health takes.
        self.vitals.read(snapshot, game);
        self.observe_streams(snapshot, game, mode, bsp, scratch);
        self.vitals.apply(snapshot, game, mode, &self.streams);
    }

    /// Rebuild this snapshot's drain, lightning and grip shots from where every
    /// living player stands and looks ([`force_streams`]); `bsp` gives the walls that
    /// stop them.
    fn observe_streams(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        mode: i32,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
    ) {
        use force_streams::Body;
        let mut bodies = [Body::default(); 33];
        let mut count = 0;
        for entity in &snapshot.entities {
            let number = entity.number();
            if entity.entity_type() != ET_PLAYER
                || number >= 32
                || entity.e_flags() & EF_DEAD != 0
                || count == bodies.len()
            {
                continue;
            }
            let (mins, maxs) = Body::unpack_box(entity.solid());
            bodies[count] = Body {
                number,
                origin: entity.trajectory_base(),
                view: entity.angular_trajectory_base(),
                mins,
                maxs,
                active: entity.force_powers_active(),
                weapon: entity.weapon(),
                team: info_number(game.config_string(1131 + usize::from(number)), "t"),
                duelling: entity.bolt1(),
                electrified: entity.emplaced_owner(),
                drain_level: None,
                lightning_level: None,
                estimated: true,
            };
            count += 1;
        }
        let player = &snapshot.player;
        if player.health() > 0 && !player.is_spectator() && count < bodies.len() {
            let (mins, maxs) = Body::standing_box();
            bodies[count] = Body {
                number: player.client_num(),
                origin: player.origin(),
                view: player.view_angles(),
                mins,
                maxs,
                active: player.force_powers_active(),
                weapon: player.weapon(),
                team: i32::from(player.team()),
                duelling: player.duel_in_progress(),
                electrified: player.electrify_time(),
                drain_level: self.settings.drain_level,
                lightning_level: self.settings.lightning_level,
                estimated: false,
            };
            count += 1;
        }
        let rules = force_streams::Rules {
            teams: mode >= GT_TEAM,
        };
        let own = (player.health() > 0 && !player.is_spectator()).then(|| force_streams::Own {
            number: player.client_num(),
            pool: i32::from(player.force_power()),
            health: player.health(),
            max_health: player.max_health(),
        });
        self.streams.observe(
            snapshot.server_time,
            &bodies[..count],
            rules,
            self.vitals.drained(),
            own,
            force_streams::world_sight(bsp, scratch),
        );
        if let Some(run) = self.streams.take_finished() {
            log_drain(&run, game);
        }
    }

    /// Drop every collected plate and drawn shape.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.list.clear();
        self.head_wanted = None;
    }

    /// Smooth one entity's opacity towards `target`.
    fn fade(&mut self, number: u16, target: f32, now: i64, step: f32) -> f32 {
        let fade = &mut self.fades[usize::from(number) % ENTITY_SLOTS];
        if now - fade.seen > STALE_MILLIS {
            fade.alpha = 0.0;
        }
        fade.seen = now;
        fade.alpha += (target - fade.alpha).clamp(-step, step);
        fade.alpha
    }

    /// The team logos' vertical ranges this frame, by client slot.
    pub(crate) fn set_logo_bounds(&mut self, bounds: [Option<(f32, f32)>; MAX_PLAYER_TAGS]) {
        self.logo_bounds = bounds;
    }

    /// This frame's duel colours, by client slot ([`super::duel_pairs`]).
    pub(crate) fn set_duel_colours(&mut self, colours: [Option<u8>; MAX_PLAYER_TAGS]) {
        self.duel_colours = colours;
    }

    /// Client slot `number`'s duel colour, the neutral one when its pair is unknown.
    fn duel_colour(&self, number: u16) -> u8 {
        self.duel_colours
            .get(usize::from(number))
            .copied()
            .flatten()
            .unwrap_or(super::duel_pairs::UNKNOWN)
    }

    /// Collect visible players using fixed scratch and the actual rendered camera.
    pub(crate) fn update(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        world: &sjk_runtime::World,
        team_info: &TeamInfoTable,
        now: i64,
        camera: Camera,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
        predicted_force_powers: Option<u32>,
        hidden: bool,
    ) {
        self.clear();
        let npc_health =
            sjk_client::LegacyNpcHealthView::new(&snapshot.player, predicted_force_powers);
        let step = (now - self.last_update).clamp(0, 100) as f32 / math::FADE_MILLIS;
        self.last_update = now;
        let settings = self.settings;
        if !settings.enabled {
            return;
        }
        let mode = info_number(game.config_string(0), "g_gametype");
        let local = snapshot.player.client_num();
        if settings.debug && now - self.last_debug >= 2_000 {
            self.last_debug = now;
            self.log_players(snapshot, game, team_info);
        }
        if hidden {
            return;
        }
        let standing = Standing {
            mode,
            restrictions: info_number(game.config_string(0), "restricts"),
            local_team: snapshot.player.team() as i32,
            local_duel: info_number(game.config_string(1131 + usize::from(local)), "dt"),
            master: snapshot
                .entities
                .iter()
                .any(|e| e.number() < 32 && e.is_jedi_master()),
            local_master: snapshot.player.is_jedi_master(),
        };
        // `cg_nameplateBars 3`: the player aimed at lately.
        let focus = self
            .focus
            .filter(|(_, seen)| (0..=FOCUS_LINGER).contains(&(now - seen)))
            .map(|(client, _)| client);
        let duel = Duel::of(snapshot, game, mode);
        let mut npcs = 0;
        for entity in &snapshot.entities {
            let number = entity.number();
            let player = entity.entity_type() == ET_PLAYER && number < 32 && number != local;
            let npc = !player
                && settings.npcs
                && entity.entity_type() == ET_NPC
                && usize::from(number) < ENTITY_SLOTS
                && super::npc_class::name(entity.npc_class()).is_some()
                && entity.health() > 0
                && npcs < MAX_NPC_TAGS;
            if !(player || npc)
                // Player `bolt1` is the private-duel marker used by EJK #12.
                // NPCs reuse the field, so it is meaningful here only for players.
                || (settings.hide_duelers && player && entity.bolt1())
                || entity.e_flags() & EF_DEAD != 0
                || entity.client_bitflag(local)
                || entity.powerups() & (1 << 11) != 0
            {
                continue;
            }
            let (accent, names_allowed, ally) = if player {
                let Some(who) =
                    identify(game, &settings, &standing, number, entity.is_jedi_master())
                else {
                    continue;
                };
                (who.accent, who.names_allowed, who.ally)
            } else {
                (NPC_ACCENT, true, false)
            };
            let presented = |id: u16| {
                world
                    .entity(sjk_runtime::EntityId::new(u64::from(id) + 1))
                    .map(|e| e.sample(now).translation)
            };
            // A pilot hidden in his vehicle (EF_NODRAW) has no presented body:
            // his plate hangs over the vehicle instead.
            let vehicle = match entity.vehicle_entity_num() {
                0 => None,
                id => snapshot
                    .entities
                    .binary_search_by_key(&id, |state| state.number())
                    .ok()
                    .and_then(|at| {
                        presented(id).map(|origin| (origin, snapshot.entities[at].solid()))
                    }),
            };
            // A limp player's plate hangs over their ragdoll, not their real body.
            let ragdoll = u8::try_from(number)
                .ok()
                .filter(|_| player)
                .and_then(crate::ragdoll::shown_top)
                .map(|(pelvis, head)| math::Anchor {
                    origin: pelvis.to_array(),
                    head,
                });
            let Some(placed) = ragdoll.or_else(|| {
                math::anchor(
                    presented(number).map(|origin| (origin, entity.solid())),
                    vehicle.filter(|_| player),
                )
            }) else {
                continue;
            };
            let origin = Vec3::from_array(placed.origin);
            let distance = origin.distance(camera.eye);
            if distance >= settings.range {
                continue;
            }
            let anchor = origin + Vec3::Z * (placed.head + math::HEAD_CLEARANCE + settings.offset);
            let Some(point) = camera.project_within(anchor, SCREEN_MARGIN) else {
                continue;
            };
            let trace = bsp.trace_box_with(
                scratch,
                camera.eye.to_array(),
                origin.to_array(),
                Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1 | 0x0200_0000,
            );
            let visibility = if unoccluded(trace.fraction, trace.start_solid, trace.all_solid) {
                1.0
            } else if settings.walls {
                WALL_OPACITY
            } else {
                0.0
            };
            let alpha = self.fade(
                number,
                math::distance_fade(distance, settings.range) * visibility,
                now,
                step,
            );
            if alpha < 0.02 {
                continue;
            }
            let bars = names_allowed
                && match settings.bars {
                    2 => true,
                    1 => ally,
                    BARS_TARGET => player && focus == Some(number),
                    _ => false,
                };
            let shares = player
                && self
                    .shared
                    .get(usize::from(number))
                    .is_some_and(Option::is_some);
            // A player who does not share, on SJK or not, has grey bars for everyone:
            // no estimate, not even the server's own for a teammate (Sol's rule,
            // 16/10/2026 for SJK players, 17/10/2026 for everyone).
            // A duel opponent's health, shield and Force never show, whatever the
            // settings (Milamber's rule, 18/10/2026); their name, weapon and icons stay.
            let stats = bars && !(player && duel.opponent(game, number));
            let withheld = stats && player && !shares;
            // Estimates show only on players who share, corrected by what they share,
            // and only while the hub has the player's own sharing.
            let estimated = player
                && !withheld
                && self.sharing_live
                && self
                    .shared
                    .get(usize::from(number))
                    .copied()
                    .flatten()
                    .is_some_and(|shared| shared.vitals.is_some());
            let (health, shield) = if withheld {
                (Some(Range::exact(0.0)), Some(Range::exact(0.0)))
            } else if stats && npc_health.permits(entity) {
                let predicted = estimated.then_some(&self.vitals);
                bar_values(entity, number, ally && player, team_info, predicted)
            } else {
                (None, None)
            };
            // Force is the last value the player shared, never estimated (17/10/2026).
            let force = if withheld {
                Some(Range::exact(0.0))
            } else if stats && estimated {
                self.shared_force(number)
            } else {
                None
            };
            let rows = Rows {
                health: health.is_some(),
                shield: shield.is_some(),
                force: force.is_some(),
            };
            // Names only means names: the icons come with the bars.
            let (powers, power_count) = if settings.icons && player && bars {
                icon_powers(entity.force_powers_active())
            } else {
                ([0; MAX_ICONS], 0)
            };
            if npc {
                npcs += 1;
            }
            self.entries.push(Entry {
                number,
                npc_class: if npc { entity.npc_class() } else { 0 },
                point,
                distance,
                scale: math::distance_scale(distance, settings.range, MIN_SCALE),
                alpha,
                detail: if rows.any() {
                    math::detail(distance, settings.near)
                } else {
                    0.0
                },
                proximity: math::detail(distance, settings.near),
                powers,
                power_count,
                health,
                shield,
                force,
                accent,
                names_allowed,
                mark: if player { self.mark(number) } else { None },
                client: if player { self.client(number) } else { None },
                shared: shares,
                withheld,
                duelling: player && entity.bolt1(),
                duel_colour: self.duel_colour(number),
                picture: Picture::Nothing,
                initial: Initial::default(),
                sjk: false,
            });
            if self.entries.len() == MAX_TAGS {
                break;
            }
        }
        self.hidden_pilots(
            snapshot, game, world, &standing, now, camera, bsp, scratch, step,
        );
        if settings.own {
            self.own_plate(snapshot, mode, now, camera, step);
        }
        self.place_pictures(game, now);
        // Far plates first, so a near plate covers a far one.
        self.entries
            .sort_unstable_by(|a, b| b.distance.total_cmp(&a.distance));
    }

    /// Each plate's picture, all asked of the pictures' cache under one lock: an SJK
    /// player's hub picture, else their stand-in; a player without SJK, their model's
    /// head icon cut round (the first one missing is cut this frame,
    /// [`Self::head_to_cut`]), else the initial of their name; an NPC, none.
    fn place_pictures(&mut self, game: &GameState, now: i64) {
        use crate::avatars::Wanted;
        self.head_wanted = None;
        let mut wanted = [Wanted::Nothing; MAX_TAGS];
        for (entry, wanted) in self.entries.iter().zip(wanted.iter_mut()) {
            let slot = usize::from(entry.number);
            if entry.npc_class != 0 || slot >= MAX_PLAYER_TAGS {
                continue;
            }
            if let Some(picture) = &self.pictures[slot] {
                *wanted = picture.wanted();
                continue;
            }
            let model = game
                .config_string(1131 + slot)
                .and_then(|info| sjk_client::LegacyClientInfo::new(info).bytes("model"))
                .unwrap_or_default();
            let key = head_key(model);
            let head = &mut self.heads[slot];
            if head.key != key {
                *head = Head {
                    key,
                    ..Head::default()
                };
            }
            if head.missing.is_none() {
                *wanted = Wanted::Local(key);
            }
        }
        let textures = crate::avatars::textures(&wanted);
        for (entry, texture) in self.entries.iter_mut().zip(textures) {
            let slot = usize::from(entry.number);
            if entry.npc_class != 0 || slot >= MAX_PLAYER_TAGS {
                continue;
            }
            entry.sjk = self.pictures[slot].is_some();
            (entry.picture, entry.initial) = match (&self.pictures[slot], texture) {
                (_, Some(texture)) => (Picture::Texture(texture), Initial::default()),
                (Some(picture), None) => (Picture::StandIn(picture.colour), picture.initial),
                (None, None) => {
                    let head = &mut self.heads[slot];
                    match head.missing {
                        Some((initial, colour)) => (Picture::StandIn(colour), initial),
                        None => {
                            let due = head
                                .asked
                                .is_none_or(|asked| !(0..HEAD_RETRY).contains(&(now - asked)));
                            if due && self.head_wanted.is_none() {
                                head.asked = Some(now);
                                self.head_wanted = Some((entry.number, head.key));
                            }
                            (Picture::Blank, Initial::default())
                        }
                    }
                }
            };
        }
    }

    /// The head icon to cut this frame, at most one: the slot of a player without SJK
    /// whose model's icon is not in the pictures' cache, and the key to keep it under
    /// ([`crate::avatars::insert_local`]). The caller, which has the game data, cuts it,
    /// or says it could not with [`Self::head_missing`].
    pub(crate) fn head_to_cut(&self) -> Option<(u16, u64)> {
        self.head_wanted
    }

    /// The head icon under `key` of slot `slot`'s model could not be had (no icon, or
    /// a model this PC does not have): their plate shows the initial of their name on a
    /// colour of their own until their model changes.
    pub(crate) fn head_missing(&mut self, slot: u16, key: u64, game: &GameState) {
        let Some(head) = self.heads.get_mut(usize::from(slot)) else {
            return;
        };
        if head.key != key {
            return;
        }
        let name = crate::player_identity::shown_name(game, usize::from(slot)).unwrap_or_default();
        head.missing = Some((
            Initial::new(crate::profile_card::initial(&name)),
            crate::profile_card::stand_in_colour("", &name),
        ));
    }

    /// The plates of pilots sealed in a vehicle (`hideRider`). `Ghost` gives such a
    /// pilot `SVF_NOCLIENT`, so no snapshot carries his entity; stock cgame finds him
    /// through the vehicle's `entityState_t::owner` (`CG_DrawCrosshair`'s target
    /// name), and the plate hangs over the vehicle. Only the name and team are
    /// known of him: no bars, icons or weapon.
    #[allow(clippy::too_many_arguments)]
    fn hidden_pilots(
        &mut self,
        snapshot: &Snapshot,
        game: &GameState,
        world: &sjk_runtime::World,
        standing: &Standing,
        now: i64,
        camera: Camera,
        bsp: &Bsp,
        scratch: &mut TraceScratch,
        step: f32,
    ) {
        let settings = self.settings;
        let local = snapshot.player.client_num();
        for vehicle in &snapshot.entities {
            let Some(number) = math::hidden_pilot(
                vehicle.entity_type(),
                vehicle.npc_class(),
                vehicle.owner(),
                local,
                |pilot| {
                    snapshot
                        .entities
                        .binary_search_by_key(&pilot, |state| state.number())
                        .is_ok()
                },
            ) else {
                continue;
            };
            if self.entries.len() >= MAX_TAGS {
                break;
            }
            let Some(who) = identify(game, &settings, standing, number, false) else {
                continue;
            };
            let Some(placed) = world
                .entity(sjk_runtime::EntityId::new(u64::from(vehicle.number()) + 1))
                .and_then(|presented| {
                    math::anchor(
                        None,
                        Some((presented.sample(now).translation, vehicle.solid())),
                    )
                })
            else {
                continue;
            };
            let origin = Vec3::from_array(placed.origin);
            let distance = origin.distance(camera.eye);
            if distance >= settings.range {
                continue;
            }
            let Some(point) = camera.project_within(
                origin + Vec3::Z * (placed.head + math::HEAD_CLEARANCE + settings.offset),
                SCREEN_MARGIN,
            ) else {
                continue;
            };
            let trace = bsp.trace_box_with(
                scratch,
                camera.eye.to_array(),
                origin.to_array(),
                Aabb::new([0.0; 3], [0.0; 3]).unwrap(),
                1 | 0x0200_0000,
            );
            let visibility = if unoccluded(trace.fraction, trace.start_solid, trace.all_solid) {
                1.0
            } else if settings.walls {
                WALL_OPACITY
            } else {
                0.0
            };
            let alpha = self.fade(
                number,
                math::distance_fade(distance, settings.range) * visibility,
                now,
                step,
            );
            if alpha < 0.02 {
                continue;
            }
            self.entries.push(Entry {
                number,
                npc_class: 0,
                point,
                distance,
                scale: math::distance_scale(distance, settings.range, MIN_SCALE),
                alpha,
                detail: 0.0,
                proximity: math::detail(distance, settings.near),
                powers: [0; MAX_ICONS],
                power_count: 0,
                health: None,
                shield: None,
                force: None,
                accent: who.accent,
                names_allowed: who.names_allowed,
                mark: self.mark(number),
                client: self.client(number),
                shared: false,
                withheld: false,
                // A pilot cannot duel: the server ends a duel on boarding.
                duelling: false,
                duel_colour: 0,
                picture: Picture::Nothing,
                initial: Initial::default(),
                sjk: false,
            });
        }
    }

    /// `cg_nameplateSelf`: the local player's own plate over their head in third
    /// person, with the real health, shield and Force the server sends them.
    fn own_plate(&mut self, snapshot: &Snapshot, mode: i32, now: i64, camera: Camera, step: f32) {
        let settings = self.settings;
        let player = &snapshot.player;
        let Some(origin) = self.own_origin else {
            return;
        };
        if player.health() <= 0
            || player.is_spectator()
            || (settings.hide_duelers && player.duel_in_progress())
            || self.entries.len() >= MAX_TAGS
        {
            return;
        }
        let local = player.client_num();
        // `CROUCH_VIEWHEIGHT` is 12, the standing one 36: the box top drops to 16. Limp,
        // the plate hangs over the ragdoll.
        let (origin, head) = u8::try_from(local)
            .ok()
            .and_then(crate::ragdoll::shown_top)
            .unwrap_or((
                origin,
                if player.view_height() < 24 {
                    16.0
                } else {
                    math::head_height(0)
                },
            ));
        let distance = origin.distance(camera.eye);
        if distance >= settings.range {
            return;
        }
        let Some(point) = camera.project_within(
            origin + Vec3::Z * (head + math::HEAD_CLEARANCE + settings.offset),
            SCREEN_MARGIN,
        ) else {
            return;
        };
        let alpha = self.fade(
            local,
            math::distance_fade(distance, settings.range),
            now,
            step,
        );
        if alpha < 0.02 {
            return;
        }
        let bars = settings.bars != 0;
        let full = player.max_health().max(1) as f32;
        let exact = |value: i32, full: f32| Range::exact(value.max(0) as f32).share_over(full);
        let shield = Some(exact(player.armor(), full));
        let entry = Entry {
            number: local,
            npc_class: 0,
            point,
            distance,
            scale: math::distance_scale(distance, settings.range, MIN_SCALE),
            alpha,
            detail: if bars {
                math::detail(distance, settings.near)
            } else {
                0.0
            },
            proximity: math::detail(distance, settings.near),
            powers: [0; MAX_ICONS],
            power_count: 0,
            health: bars.then(|| exact(player.health(), full)),
            shield: shield.filter(|_| bars),
            force: bars.then(|| {
                exact(
                    i32::from(player.force_power()),
                    sjk_game_jka::force_powers::FORCE_POWER_MAX as f32,
                )
            }),
            accent: team_accent(mode, player.team() as i32),
            names_allowed: true,
            mark: self.mark(local),
            client: self.client(local),
            shared: self.sharing_live,
            withheld: false,
            duelling: player.duel_in_progress(),
            duel_colour: self.duel_colour(local),
            picture: Picture::Nothing,
            initial: Initial::default(),
            sjk: false,
        };
        let (powers, power_count) = if settings.icons {
            icon_powers(player.force_powers_active())
        } else {
            ([0; MAX_ICONS], 0)
        };
        self.entries.push(Entry {
            powers,
            power_count,
            ..entry
        });
    }

    /// `cg_nameplateDebug`: what the server sends about each other player, to
    /// settle whether enemy health reaches the client.
    fn log_players(&self, snapshot: &Snapshot, game: &GameState, team_info: &TeamInfoTable) {
        let force_keys: Vec<String> = game
            .config_string(0)
            .map(|info| {
                String::from_utf8_lossy(info)
                    .trim_start_matches('\\')
                    .split('\\')
                    .collect::<Vec<_>>()
                    .chunks(2)
                    .filter(|pair| {
                        let key = pair[0].to_ascii_lowercase();
                        key.contains("force") || key.contains("regen")
                    })
                    .map(|pair| pair.join("="))
                    .collect()
            })
            .unwrap_or_default();
        eprintln!(
            "nameplate: server info force keys {force_keys:?}, pain events {:?}",
            self.vitals.pain_report(),
        );
        let local = snapshot.player.client_num();
        for entity in &snapshot.entities {
            let number = entity.number();
            if entity.entity_type() != ET_PLAYER || number >= 32 || number == local {
                continue;
            }
            let team = team_info
                .entries()
                .iter()
                .find(|row| u16::from(row.client_num) == number)
                .map(|row| (row.health, row.armor));
            eprintln!(
                "nameplate: client {number} health {} max {} hp~{} armor~{} powers {:#x} fp~{} weapon {} style {} tinfo {team:?}",
                entity.health(),
                entity.max_health(),
                shown(self.vitals.health(number)),
                shown(self.vitals.armor(number)),
                entity.force_powers_active(),
                shown(self.shared_force(number).map(|r| r.map(|v| v * 100.0))),
                entity.weapon(),
                entity.fire_flag(),
            );
        }
    }

    /// Text id of an entry's label.
    fn text_id(entry: &Entry) -> TextId {
        if entry.npc_class == 0 {
            TextId(u32::from(entry.number))
        } else {
            TextId(NPC_TEXT + u32::from(entry.npc_class))
        }
    }

    /// Lay the collected plates out as shapes and text commands, in the chosen style.
    fn build<'a>(
        &mut self,
        label: &dyn Fn(TextId) -> &'a str,
        icons: &[Option<TextureId>],
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        self.list.clear();
        let unit = crate::ui_scale::height_scale(viewport[1]);
        for index in 0..self.entries.len() {
            let entry = self.entries[index];
            if !entry.names_allowed {
                continue;
            }
            let u = unit * entry.scale;
            let [x, y] = entry.point;
            // Over a team logo (drawn by `super::team_logo` at the same anchor).
            let logo_bounds = if entry.npc_class == 0 {
                self.logo_bounds
                    .get(usize::from(entry.number))
                    .copied()
                    .flatten()
            } else {
                None
            };
            // First honour the user's projected position exactly as with no logo.
            // A plate grows upward from this bottom, so its collision range continues
            // below the logo by the plate's height. Only positions in that range are
            // an exception: there the plate goes to its usual safe spot above the logo.
            let desired_bottom = y - 2.0 * u;
            let bottom = match logo_bounds {
                Some((top, logo_bottom))
                    if desired_bottom > top
                        && desired_bottom
                            < logo_bottom + PLATE_COLLISION_HEIGHT * self.settings.scale * u =>
                {
                    top - 2.0 * u
                }
                _ => desired_bottom,
            };
            let place = Place {
                index,
                id: Self::text_id(&entry),
                x,
                bottom,
                size: 36.0 * self.settings.scale * u,
                u,
                unit,
            };
            let opacity = entry.alpha * self.settings.opacity;
            let _ = self.list.push(DrawCommand::PushOpacity(opacity));
            match self.settings.style {
                Style::Holo => self.holo(&entry, place, label, icons, font, viewport),
                Style::Capsule => self.capsule(&entry, place, label, icons, font, viewport),
                Style::Classic => self.classic(&entry, place, label, icons, font, viewport),
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        self.mode_label(label, font, unit, viewport);
    }

    /// One holo frame plate (design C).
    fn holo<'a>(
        &mut self,
        entry: &Entry,
        place: Place,
        label: &dyn Fn(TextId) -> &'a str,
        icons: &[Option<TextureId>],
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let Place {
            index,
            id,
            x,
            bottom,
            size,
            unit,
            ..
        } = place;
        let scale = size / font.height.max(1.0);
        // The duel badge stands on the picture's corner.
        let duel_badge = entry.duelling && self.settings.duel_badge;
        // A sharer's bars: a player who does not share has a dotted line under
        // their name instead (28/10/2026, no grey bars), and a duel shows none.
        let rows = if entry.withheld || duel_badge {
            Rows::default()
        } else {
            entry.rows()
        };
        let holo = Holo::new(&HoloSpec {
            centre_x: x,
            bottom,
            size,
            name_width: crate::text::visible_text_width(font, label(id), scale),
            picture: entry.picture != Picture::Nothing,
            mark: entry.mark.is_some(),
            client: entry.client.is_some(),
            duel: duel_badge,
            rows,
            detail: entry.detail,
            proximity: entry.proximity,
            least_bar: 2.0 * unit,
            viewport,
        });
        let frame = Frame::of(entry, duel_badge);
        let opacity = name_opacity(entry);
        // The plate fades out with distance: far away the hexagon and the name stay,
        // over a hairline.
        if entry.proximity > 0.01 {
            let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
            self.plate(entry, &holo, &frame, rows, duel_badge);
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        if entry.proximity < 0.99 {
            let _ = self
                .list
                .push(DrawCommand::PushOpacity(1.0 - entry.proximity));
            let _ = self.list.push(DrawCommand::SolidRect {
                rect: holo.hairline,
                color: frame.hairline,
            });
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        self.picture(entry, &holo, &frame, index, font, opacity);
        self.name_line(
            entry,
            id,
            [holo.name.x, holo.middle],
            holo.name.width,
            size,
            font,
        );
        self.badges(entry, [holo.mark, holo.client], holo.badge, opacity);
        if entry.power_count > 0 && entry.proximity > 0.01 {
            self.power_row(entry, icons, &holo, &frame, viewport);
        }
    }

    /// A plate's name, left-aligned from `x` with its capitals centred on `middle`, at
    /// most `width` wide: white for a player, a pale gold for an NPC.
    fn name_line(
        &mut self,
        entry: &Entry,
        id: TextId,
        [x, middle]: [f32; 2],
        width: f32,
        size: f32,
        font: &UiFont,
    ) {
        let scale = size / font.height.max(1.0);
        let _ = self.list.push(DrawCommand::Text {
            rect: Rect::new(
                x,
                middle - font.capital_middle(scale),
                width + 0.5,
                size * 1.15,
            ),
            text: id,
            size,
            color: name_tint(entry),
            align: TextAlign::Start,
            overflow: TextOverflow::Ellipsis,
            weight: FontWeight::Regular,
            letter_spacing: 0.0,
        });
    }

    /// The mark after the name and the client icon after it in `[mark, client]`, and
    /// the duel badge in `badge`, at the name's `opacity` (the badge never under
    /// [`DUEL_BADGE_OPACITY`]). The client icon keeps its own colours.
    fn badges(
        &mut self,
        entry: &Entry,
        [mark, client]: [Option<Rect>; 2],
        badge: Option<Rect>,
        opacity: f32,
    ) {
        for (rect, texture) in [(mark, entry.mark), (client, entry.client)] {
            if let (Some(rect), Some(texture)) = (rect, texture) {
                let _ = self.list.push(DrawCommand::TexturedQuad {
                    rect,
                    texture,
                    color: Color::new(1.0, 1.0, 1.0, opacity),
                });
            }
        }
        if let Some(rect) = badge {
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect,
                texture: crate::ui_renderer::duel_texture(usize::from(entry.duel_colour)),
                color: Color::new(1.0, 1.0, 1.0, opacity.max(DUEL_BADGE_OPACITY)),
            });
        }
    }

    /// The yellow "?" over a bar too unsure to show, sized to it so the marks of two
    /// unsure bars do not meet. `k` is the plate's pixel.
    fn unknown_mark(&mut self, rect: Rect, k: f32) {
        let size = rect.height * 1.3 + 2.5 * k;
        let _ = self.list.push(DrawCommand::Text {
            rect: Rect::new(
                rect.x,
                rect.y + (rect.height - size * 1.15) * 0.5,
                rect.width,
                size * 1.15,
            ),
            text: TextId(UNKNOWN_TEXT),
            size,
            color: math::UNKNOWN_MARK,
            align: TextAlign::Center,
            overflow: TextOverflow::Ellipsis,
            weight: FontWeight::Semibold,
            letter_spacing: 0.0,
        });
    }

    /// A filled convex polygon, the first [`sjk_ui::POLYGON_POINTS`] of `points`.
    fn polygon(
        &mut self,
        points: &[[f32; 2]],
        color: Color,
        texture: Option<(TextureId, [[f32; 2]; 2])>,
    ) {
        let mut corners = [[0.0; 2]; sjk_ui::POLYGON_POINTS];
        let count = points.len().min(corners.len());
        corners[..count].copy_from_slice(&points[..count]);
        let _ = self.list.push(DrawCommand::Polygon {
            points: corners,
            count: count as u8,
            color,
            texture,
        });
    }

    /// The plate: its fill (the frame's colour made dark) under scan lines, its edge,
    /// the gold corner of a player who shares, and the bars, or while they have not
    /// faded in, the dotted line under the name.
    fn plate(&mut self, entry: &Entry, holo: &Holo, frame: &Frame, rows: Rows, duel: bool) {
        let k = holo.k;
        let outline = holo.outline();
        self.polygon(&outline, math::plate_fill(frame.edge, PLATE_ALPHA), None);
        let plate = holo.plate;
        if frame.scan {
            let colour = Color::new(frame.edge.r, frame.edge.g, frame.edge.b, SCAN_ALPHA);
            let step = (math::SCAN * k).max(2.0);
            let mut y = plate.bottom() - 1.5;
            while y > plate.y + 1.0 {
                let cut = (holo.cut - (y - plate.y)).max(0.0);
                let _ = self.list.push(DrawCommand::Line {
                    from: [plate.x + 1.0, y],
                    to: [plate.right() - 1.0 - cut, y],
                    width: 1.0,
                    color: colour,
                });
                y -= step;
            }
        }
        let width = (math::EDGE * k).max(1.0);
        let half = width * 0.5;
        let [top_left, cut_start, cut_end, bottom_right, bottom_left] = outline;
        for (from, to) in [
            (
                [top_left[0], top_left[1] + half],
                [cut_start[0], cut_start[1] + half],
            ),
            (
                [cut_start[0], cut_start[1] + half],
                [cut_end[0] - half, cut_end[1]],
            ),
            (
                [cut_end[0] - half, cut_end[1]],
                [bottom_right[0] - half, bottom_right[1]],
            ),
            (
                [bottom_right[0], bottom_right[1] - half],
                [bottom_left[0], bottom_left[1] - half],
            ),
            (
                [bottom_left[0] + half, bottom_left[1] - half],
                [top_left[0] + half, top_left[1] + width],
            ),
        ] {
            let _ = self.list.push(DrawCommand::Line {
                from,
                to,
                width,
                color: frame.edge,
            });
        }
        if entry.shared && !duel {
            let [start, corner, end] = holo.gold();
            for (from, to) in [(start, corner), (corner, end)] {
                let _ = self.list.push(DrawCommand::Line {
                    from,
                    to,
                    width: (math::GOLD_WIDTH * k).max(1.0),
                    color: SHARED_GOLD,
                });
            }
        }
        let detail = if rows.any() { entry.detail } else { 0.0 };
        if detail > 0.01 {
            let _ = self.list.push(DrawCommand::PushOpacity(detail));
            let colors = self.colors;
            let track = Color::new(frame.edge.r, frame.edge.g, frame.edge.b, TRACK_ALPHA);
            for (bar, range, color, meter) in [
                (holo.bars.shield, entry.shield, colors.shield, Meter::Shield),
                (holo.bars.health, entry.health, colors.health, Meter::Health),
                (holo.bars.force, entry.force, colors.force, Meter::Force),
            ] {
                if let (Some(rect), Some(range)) = (bar, range) {
                    self.meter(rect, range, color, meter, k, track);
                }
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
        if detail < 0.99 {
            let _ = self.list.push(DrawCommand::PushOpacity(1.0 - detail));
            let line = holo.dotted;
            for (start, end) in math::dashes(line.x, line.width, math::DOT * k, math::DOT_GAP * k) {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(start, line.y, end - start, line.height),
                    color: frame.dotted,
                });
            }
            let _ = self.list.push(DrawCommand::PopOpacity);
        }
    }

    /// The hexagon at the plate's left end, at `opacity` (the name's): the frame's
    /// colour round the player's picture or head icon, or their initial on their
    /// colour.
    fn picture(
        &mut self,
        entry: &Entry,
        holo: &Holo,
        frame: &Frame,
        index: usize,
        font: &UiFont,
        opacity: f32,
    ) {
        let Some(hex) = holo.hex else {
            return;
        };
        let faded = |colour: Color| Color::new(colour.r, colour.g, colour.b, colour.a * opacity);
        self.polygon(&math::hexagon(hex), faded(frame.hex), None);
        // The frame round the picture: two design pixels near, one far.
        let inner = math::inset(hex, (1.0 + entry.proximity) * holo.k);
        let shape = math::hexagon(inner);
        match entry.picture {
            Picture::Nothing => {}
            Picture::Texture(texture) => {
                self.polygon(
                    &shape,
                    Color::new(1.0, 1.0, 1.0, opacity),
                    Some((texture, math::HEX_UV)),
                );
            }
            Picture::StandIn(colour) => {
                self.polygon(&shape, faded(Color { a: 1.0, ..colour }), None);
                let size = inner.height * 0.55;
                let scale = size / font.height.max(1.0);
                let _ = self.list.push(DrawCommand::Text {
                    rect: Rect::new(
                        inner.x,
                        inner.y + inner.height * 0.5 - font.capital_middle(scale),
                        inner.width,
                        size * 1.15,
                    ),
                    text: TextId(INITIAL_TEXT + index as u32),
                    size,
                    color: Color::new(1.0, 1.0, 1.0, opacity),
                    align: TextAlign::Center,
                    overflow: TextOverflow::Clip,
                    weight: FontWeight::Semibold,
                    letter_spacing: 0.0,
                });
            }
            Picture::Blank => self.polygon(&shape, faded(BLANK_PICTURE), None),
        }
    }

    /// The name of the mode just chosen, near the top of the screen for a moment.
    fn mode_label<'a>(
        &mut self,
        label: &dyn Fn(TextId) -> &'a str,
        font: &UiFont,
        unit: f32,
        viewport: [f32; 2],
    ) {
        let Some((_, at)) = self.announced else {
            return;
        };
        let age = self.last_update - at;
        if !(0..MODE_SHOWN).contains(&age) {
            return;
        }
        let alpha = ((MODE_SHOWN - age) as f32 / MODE_FADE as f32).min(1.0);
        let size = 28.0 * unit;
        let text = crate::text::visible_text_width(
            font,
            label(TextId(MODE_TEXT)),
            size / font.height.max(1.0),
        );
        let width = (text + size * 1.6).min(viewport[0]);
        let rect = Rect::new(
            (viewport[0] - width) * 0.5,
            viewport[1] * 0.18,
            width,
            size * 1.6,
        );
        let _ = self.list.push(DrawCommand::PushOpacity(alpha));
        let _ = self.list.push(DrawCommand::RoundedRect {
            rect,
            radius: rect.height * 0.5,
            color: Color::new(0.03, 0.04, 0.06, 0.7),
        });
        let _ = self.list.push(DrawCommand::Text {
            rect: Rect::new(rect.x, rect.y + size * 0.2, rect.width, size * 1.2),
            text: TextId(MODE_TEXT),
            size,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            align: TextAlign::Center,
            overflow: TextOverflow::Ellipsis,
            weight: FontWeight::Semibold,
            letter_spacing: 0.0,
        });
        let _ = self.list.push(DrawCommand::PopOpacity);
    }

    /// The row of power icons over the plate's right end, each on a square cell edged
    /// in the frame's colour.
    fn power_row(
        &mut self,
        entry: &Entry,
        icons: &[Option<TextureId>],
        holo: &Holo,
        frame: &Frame,
        viewport: [f32; 2],
    ) {
        let shown = entry.powers[..usize::from(entry.power_count)]
            .iter()
            .filter(|power| icons.get(usize::from(**power)).copied().flatten().is_some())
            .count();
        if shown == 0 {
            return;
        }
        let (mut chip, step) = holo.chips(shown, viewport);
        let inset = (CHIP_INSET * holo.k).max(1.0);
        let edge = Color::new(frame.edge.r, frame.edge.g, frame.edge.b, CHIP_EDGE_ALPHA);
        let _ = self.list.push(DrawCommand::PushOpacity(entry.proximity));
        for power in &entry.powers[..usize::from(entry.power_count)] {
            let Some(texture) = icons.get(usize::from(*power)).copied().flatten() else {
                continue;
            };
            let radius = CHIP_CORNER * holo.k;
            let _ = self.list.push(DrawCommand::RoundedRect {
                rect: chip,
                radius,
                color: CHIP_FILL,
            });
            let _ = self.list.push(DrawCommand::Border {
                rect: chip,
                radius,
                width: holo.k.max(1.0),
                color: edge,
            });
            let _ = self.list.push(DrawCommand::TexturedQuad {
                rect: math::inset(chip, inset),
                texture,
                color: Color::new(1.0, 1.0, 1.0, 1.0),
            });
            chip.x += step;
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
    }

    /// One bar: shield and health in segments, Force whole. Each piece shows the fill
    /// to the guess and the track past it, the haze over the uncertain stretch, and over
    /// a full bar (overheal, overshield) a second layer in a deeper shade from the left.
    /// An empty shield is a broken grey bar; health or shield too unsure to show dims
    /// under a yellow "?". `k` is the design's pixel.
    fn meter(
        &mut self,
        rect: Rect,
        range: Range,
        color: Color,
        meter: Meter,
        k: f32,
        track: Color,
    ) {
        if meter == Meter::Shield && range.high < EMPTY_SHARE {
            for (start, end) in math::dashes(
                rect.x,
                rect.width,
                math::EMPTY_DASH * k,
                math::EMPTY_GAP * k,
            ) {
                let _ = self.list.push(DrawCommand::SolidRect {
                    rect: Rect::new(start, rect.y, end - start, rect.height),
                    color: math::EMPTY_SHIELD,
                });
            }
            return;
        }
        let unsure = meter != Meter::Force && range.width() >= UNSURE_WIDTH;
        let _ = self
            .list
            .push(DrawCommand::PushOpacity(if unsure { 0.35 } else { 1.0 }));
        let (on, off) = if meter == Meter::Force {
            (rect.width, 0.0)
        } else {
            (math::SEGMENT * k, math::SEGMENT_GAP * k)
        };
        let base = range.clamp(0.0, 1.0);
        // The second layer is an inner band, so the full bar shows round it.
        let over = (range.high > 1.0).then(|| {
            (
                math::overflow_band(rect),
                range.map(|share| (share - 1.0).clamp(0.0, 1.0)),
            )
        });
        let at = |share: f32| rect.x + rect.width * share;
        for (start, end) in math::segments(rect.x, rect.width, on, off) {
            let piece = |from: f32, to: f32, band: Rect| {
                let (from, to) = (from.max(start), to.min(end));
                (to - from > 0.01).then(|| Rect::new(from, band.y, to - from, band.height))
            };
            let filled = at(base.best);
            if let Some(rect) = piece(rect.x, filled, rect) {
                let _ = self.list.push(DrawCommand::SolidRect { rect, color });
            }
            if let Some(rect) = piece(filled, rect.right(), rect) {
                let _ = self
                    .list
                    .push(DrawCommand::SolidRect { rect, color: track });
            }
            self.haze(rect, base, (start, end));
            if let Some((band, layer)) = over {
                if let Some(rect) = piece(band.x, at(layer.best), band) {
                    let _ = self.list.push(DrawCommand::SolidRect {
                        rect,
                        color: math::saturated(color),
                    });
                }
                self.haze(band, layer, (start, end));
            }
        }
        let _ = self.list.push(DrawCommand::PopOpacity);
        if unsure {
            self.unknown_mark(rect, k);
        }
    }

    /// The grey haze over the uncertain stretch of a bar, within its piece from `start`
    /// to `end` (pixels): thickest at the guess, fading out towards either bound, so the
    /// bar's edge looks as blurred as the estimate is loose.
    fn haze(&mut self, rect: Rect, range: Range, (start, end): (f32, f32)) {
        if range.width() < HAZE_MIN_WIDTH {
            return;
        }
        let at = |share: f32| rect.x + rect.width * share;
        let clear = Color::new(HAZE.r, HAZE.g, HAZE.b, 0.0);
        for (from, to, low, high) in [
            (range.low, range.best, 0.0, HAZE.a),
            (range.best, range.high, HAZE.a, 0.0),
        ] {
            let (left, right) = (at(from), at(to));
            let (shown_left, shown_right) = (left.max(start), right.min(end));
            if shown_right - shown_left < 0.5 {
                continue;
            }
            let alpha = |x: f32| low + (high - low) * (x - left) / (right - left).max(1e-6);
            let _ = self.list.push(DrawCommand::GradientRect {
                rect: Rect::new(shown_left, rect.y, shown_right - shown_left, rect.height),
                radius: 0.0,
                gradient: Gradient {
                    start: Color {
                        a: alpha(shown_left),
                        ..clear
                    },
                    end: Color {
                        a: alpha(shown_right),
                        ..clear
                    },
                    vertical: false,
                },
            });
        }
    }

    /// The plates' stand-in initials, by plate, copied out so the text they are drawn
    /// with can be read while the plates are laid out.
    fn initials(&self) -> [Initial; MAX_TAGS] {
        let mut initials = [Initial::default(); MAX_TAGS];
        for (initial, entry) in initials.iter_mut().zip(&self.entries) {
            *initial = entry.initial;
        }
        initials
    }

    /// Build the plates from the existing roster text at submission, never
    /// allocating another name store.
    pub(crate) fn append(
        &mut self,
        chat: &ChatOverlay,
        icons: &[Option<TextureId>],
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        let mode = self.announced.map(|(mode, _)| mode);
        let initials = self.initials();
        let text = |id: TextId| {
            if id.0 == MODE_TEXT {
                mode.map_or("", Mode::label)
            } else if let Some(initial) = initial(&initials, id) {
                initial
            } else {
                label(chat, id)
            }
        };
        self.build(&text, icons, font, viewport);
        self.append_text(text, vertices, font, viewport);
    }

    /// Append the laid out plates' text: a black name keeps its colour, edged in a
    /// light halo to read on the see-through plate.
    fn append_text<'a>(
        &self,
        text: impl Fn(TextId) -> &'a str,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        crate::ui_renderer::append_text_commands_where(
            &self.list,
            text,
            |_, _| true,
            vertices,
            font,
            viewport,
            crate::text::TextStyle::NEUTRAL,
            self.settings.outline,
        );
    }
}

/// One line in the log for each drain of the local player: what its pool and health
/// measured, and whom it was put on, so a report says what the plates were told.
fn log_drain(run: &force_streams::Run, game: &GameState) {
    use std::fmt::Write;
    let mut victims = String::new();
    for (slot, taken) in run.taken.iter().enumerate() {
        if *taken > 0.0 {
            let name = game
                .config_string(1131 + slot)
                .and_then(|info| sjk_client::LegacyClientInfo::new(info).text("n"))
                .unwrap_or("?");
            let _ = write!(
                victims,
                "{}{} {:.0}",
                if victims.is_empty() { "" } else { ", " },
                crate::text::Plain(name),
                taken
            );
        }
    }
    let healed = run.healed.map_or_else(
        || "health at its maximum".to_owned(),
        |healed| format!("healed {healed:.0}"),
    );
    let [reach, event, front] = run.found;
    crate::log::progress(format_args!(
        "nameplate drain: {:.2} s, {:.0} shots paid ({:.0} Force), {healed}; taken: {} \
         (found in reach {reach}, by the drained event {event}, in front {front}); \
         pace {:.0} ms a shot",
        run.millis as f32 / 1_000.0,
        run.shots,
        run.shots * 5.0,
        if victims.is_empty() {
            "nothing"
        } else {
            &victims
        },
        run.pace,
    ));
}

/// The Force powers of `active` that get an icon, in display order, at most
/// [`MAX_ICONS`] of them.
fn icon_powers(active: u32) -> ([u8; MAX_ICONS], u8) {
    let mut powers = [0; MAX_ICONS];
    let mut count = 0;
    for power in ICON_POWERS {
        if active & (1 << power) != 0 && usize::from(count) < MAX_ICONS {
            powers[usize::from(count)] = power;
            count += 1;
        }
    }
    (powers, count)
}

/// What the viewer's own standing decides about every other player's plate.
struct Standing {
    mode: i32,
    restrictions: i32,
    local_team: i32,
    local_duel: i32,
    master: bool,
    local_master: bool,
}

/// How a player's plate is drawn for the viewer.
struct Identity {
    accent: Color,
    names_allowed: bool,
    ally: bool,
}

/// The local player's duel, if any: nobody sees a duel opponent's health, shield and
/// Force, and the duellist sends nobody theirs (Milamber's rule, 18/10/2026), whatever
/// either side's settings. Read from the snapshot and the game state, every frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Duel {
    /// Not duelling (a spectator of a duel game type included).
    #[default]
    None,
    /// A private duel (`duelInProgress`) against this client.
    Private(u16),
    /// `GT_DUEL`: every other player in the arena.
    Arena,
    /// `GT_POWERDUEL`, on this duel side (`dt` in `CS_PLAYERS`): the other side.
    Sides(i32),
}

impl Duel {
    pub(crate) fn of(snapshot: &Snapshot, game: &GameState, mode: i32) -> Self {
        let player = &snapshot.player;
        if player.duel_in_progress() {
            return Self::Private(player.duel_index());
        }
        if player.is_spectator() || player.team() as i32 == TEAM_SPECTATOR {
            return Self::None;
        }
        match mode {
            GT_DUEL => Self::Arena,
            GT_POWERDUEL => Self::Sides(info_number(
                game.config_string(1131 + usize::from(player.client_num())),
                "dt",
            )),
            _ => Self::None,
        }
    }

    pub(crate) fn active(self) -> bool {
        self != Self::None
    }

    /// Whether player `number` (not a spectator: those have no plate) is an opponent.
    pub(crate) fn opponent(self, game: &GameState, number: u16) -> bool {
        match self {
            Self::None => false,
            Self::Private(opponent) => opponent == number,
            Self::Arena => true,
            Self::Sides(side) => {
                info_number(game.config_string(1131 + usize::from(number)), "dt") != side
            }
        }
    }
}

/// Player `number`'s plate identity from his `CS_PLAYERS` string; `None` where
/// he has none, is a spectator, or the server's restrictions hide him.
fn identify(
    game: &GameState,
    settings: &Settings,
    standing: &Standing,
    number: u16,
    jedi_master: bool,
) -> Option<Identity> {
    let info = game.config_string(1131 + usize::from(number));
    let team = info_number(info, "t");
    if info.is_none() || team == 3 {
        return None;
    }
    // An ally with a team logo keeps his plate where a server's restrictions hide
    // names, as stock keeps the ally marker.
    let logo = settings.friends
        && super::team_logo::ally_logo(
            standing.mode,
            standing.local_team,
            team,
            standing.local_duel,
            info_number(info, "dt"),
            standing.master,
            standing.local_master,
            jedi_master,
        )
        .is_some();
    if !logo && standing.restrictions & 64 != 0 {
        return None;
    }
    Some(Identity {
        accent: team_accent(standing.mode, team),
        names_allowed: standing.restrictions & 64 == 0,
        ally: standing.mode >= GT_TEAM && (team == 1 || team == 2) && team == standing.local_team,
    })
}

/// Health and shield shares of `entity` (up to 2: a second bar's worth over the
/// maximum): a teammate's from the team overlay (points out of a full 100), anyone's
/// from the entity state when the server sends health there, else the `predicted`
/// estimate.
fn bar_values(
    entity: &sjk_protocol::EntityState,
    number: u16,
    teammate: bool,
    team_info: &TeamInfoTable,
    predicted: Option<&vitals_estimate::Estimator>,
) -> (Option<Range>, Option<Range>) {
    if teammate
        && let Some(row) = team_info
            .entries()
            .iter()
            .find(|row| u16::from(row.client_num) == number)
    {
        let share = |points: i32| Range::exact(points as f32).share_over(FULL_POINTS);
        return (Some(share(row.health)), Some(share(row.armor)));
    }
    let maximum = entity.max_health() as f32;
    if maximum > 0.0 {
        return (
            Some(Range::exact(entity.health() as f32).share(maximum)),
            None,
        );
    }
    let Some(vitals) = predicted else {
        return (None, None);
    };
    let full = vitals.full();
    (
        vitals.health(number).map(|range| range.share_over(full)),
        vitals.armor(number).map(|range| range.share_over(full)),
    )
}

/// A range in points for the debug log.
fn shown(range: Option<Range>) -> String {
    range.map_or_else(
        || "-".to_owned(),
        |range| format!("{:.0}[{:.0}..{:.0}]", range.best, range.low, range.high),
    )
}

/// Frame colour for a player on `team` in game type `mode`: red or blue in
/// team games, neutral otherwise.
fn team_accent(mode: i32, team: i32) -> Color {
    match (mode >= GT_TEAM, team) {
        (true, 1) => Color::new(1.0, 0.3, 0.25, 0.95),
        (true, 2) => Color::new(0.3, 0.6, 1.0, 0.95),
        _ => NEUTRAL_ACCENT,
    }
}

/// The bars' colours.
#[derive(Clone, Copy, Debug)]
struct BarColors {
    health: Color,
    shield: Color,
    force: Color,
}

impl Default for BarColors {
    fn default() -> Self {
        Self {
            health: math::HEALTH_COLOR,
            shield: math::SHIELD_COLOR,
            force: math::FORCE_COLOR,
        }
    }
}

/// The colours of a plate's frame.
#[derive(Clone, Copy, Debug)]
struct Frame {
    /// The hexagon round the picture.
    hex: Color,
    /// The plate's edge, whose colour also tints its fill, scan lines, bar tracks and
    /// power icon cells.
    edge: Color,
    /// The dotted line under the name of a plate without bars, and the hairline under
    /// a far plate's name.
    dotted: Color,
    hairline: Color,
    /// Scan lines over the plate: not on a grey frame, an NPC's or in a duel.
    scan: bool,
}

impl Frame {
    /// The frame of `entry`'s plate (design C): a duel's pair's colour, a team's, an
    /// NPC's pale gold, grey for a player without SJK, else the holo blue.
    fn of(entry: &Entry, duel: bool) -> Self {
        let with = |colour: Color, alpha: f32| Color::new(colour.r, colour.g, colour.b, alpha);
        if duel {
            let colours = crate::ui_renderer::duel_badge::COLOURS;
            let [r, g, b] = colours[usize::from(entry.duel_colour) % colours.len()];
            let pair = Color::new(r, g, b, 1.0);
            return Self {
                hex: pair,
                edge: pair,
                dotted: with(pair, 0.45),
                hairline: with(pair, 0.6),
                scan: false,
            };
        }
        let (hex, edge, scan) = if entry.npc_class != 0 {
            (NPC_ACCENT, NPC_ACCENT, false)
        } else if entry.accent != NEUTRAL_ACCENT {
            (with(entry.accent, 1.0), entry.accent, true)
        } else if !entry.sjk {
            (GREY_HEX, GREY_EDGE, false)
        } else {
            (HOLO_HEX, HOLO_EDGE, true)
        };
        Self {
            hex,
            edge,
            dotted: DOTTED,
            hairline: if entry.sjk || entry.npc_class != 0 {
                with(edge, 0.6)
            } else {
                GREY_HAIRLINE
            },
            scan,
        }
    }
}

/// Where one plate stands this frame, worked out once for every [`Style`].
#[derive(Clone, Copy, Debug)]
struct Place {
    /// The plate's place in the frame's entries (its stand-in initial's text), and its
    /// name's text.
    index: usize,
    id: TextId,
    /// The point over the head the plate is centred on, and the plate's bottom (over a
    /// team logo).
    x: f32,
    bottom: f32,
    /// The name's text size; the HUD's pixel at the plate's distance, and unscaled.
    size: f32,
    u: f32,
    unit: f32,
}

/// How opaque a plate's name is: dimmer far away.
fn name_opacity(entry: &Entry) -> f32 {
    FAR_NAME_OPACITY + (1.0 - FAR_NAME_OPACITY) * entry.proximity
}

/// A plate's name's colour: white for a player, a pale gold for an NPC.
fn name_tint(entry: &Entry) -> Color {
    let opacity = name_opacity(entry);
    if entry.npc_class == 0 {
        Color::new(1.0, 1.0, 1.0, opacity)
    } else {
        Color::new(1.0, 0.95, 0.75, opacity)
    }
}

/// Which bar [`State::meter`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Meter {
    Shield,
    Health,
    Force,
}

/// The text of stand-in initial `id` ([`INITIAL_TEXT`]), if it is one.
fn initial(initials: &[Initial; MAX_TAGS], id: TextId) -> Option<&str> {
    let index = id.0.checked_sub(INITIAL_TEXT)?;
    initials.get(index as usize).map(Initial::as_str)
}

/// Text of a plate: a roster name, an NPC class name, or the "?" over a bar.
fn label(chat: &ChatOverlay, id: TextId) -> &str {
    if id.0 == UNKNOWN_TEXT {
        return "?";
    }
    match id.0.checked_sub(NPC_TEXT) {
        // A blocked player's plate reads BLOCKED (`muted_players.rs`).
        None => crate::muted_players::label(id.0, chat.player_label(id.0 as u16)),
        Some(class) => super::npc_class::name(class as u8).unwrap_or(""),
    }
}

/// A plate for the off-screen snapshots (`menu_snapshot.rs`), its bars as shares of
/// a full one: `[low, guess, high]`.
#[cfg(test)]
pub(crate) struct PreviewPlate {
    pub(crate) slot: u16,
    pub(crate) point: [f32; 2],
    pub(crate) distance: f32,
    pub(crate) health: Option<[f32; 3]>,
    pub(crate) shield: Option<[f32; 3]>,
    pub(crate) force: Option<[f32; 3]>,
    /// The Force powers on (`forcePowersActive` bits), shown over the plate.
    pub(crate) powers: u32,
    pub(crate) verified: bool,
    /// Another client's icon, small after the mark.
    pub(crate) client: Option<TextureId>,
    /// In a private duel: the duel badge before the name.
    pub(crate) duelling: bool,
    /// The frame's colour: red or blue team, else neutral.
    pub(crate) team: Option<bool>,
    /// The gold contour of a player who shares their bars.
    pub(crate) shared: bool,
    /// A player who does not share: a dotted line, no bars.
    pub(crate) withheld: bool,
    /// Their picture in the atlas, else their initial on this colour, else none.
    pub(crate) picture: Option<TextureId>,
    pub(crate) stand_in: Option<(char, Color)>,
    /// On SJK: else the frame is grey.
    pub(crate) sjk: bool,
}

#[cfg(test)]
impl State {
    /// Lay out `plates` as [`State::append`] does, naming slot `n` `names[n]`.
    pub(crate) fn preview(
        &mut self,
        plates: &[PreviewPlate],
        names: &[&str],
        power_icons: &[Option<TextureId>],
        font: &UiFont,
        vertices: &mut Vec<TextVertex>,
        viewport: [f32; 2],
    ) {
        let range = |share: [f32; 3]| Range::new(share[0], share[1], share[2]);
        self.entries.clear();
        for plate in plates {
            let proximity = math::detail(plate.distance, self.settings.near);
            self.entries.push(Entry {
                number: plate.slot,
                npc_class: 0,
                point: plate.point,
                distance: plate.distance,
                scale: math::distance_scale(plate.distance, self.settings.range, MIN_SCALE),
                alpha: 1.0,
                detail: proximity,
                proximity,
                powers: icon_powers(plate.powers).0,
                power_count: icon_powers(plate.powers).1,
                health: plate.health.map(range),
                shield: plate.shield.map(range),
                force: plate.force.map(range),
                accent: match plate.team {
                    Some(red) => team_accent(GT_TEAM, if red { 1 } else { 2 }),
                    None => NEUTRAL_ACCENT,
                },
                names_allowed: true,
                mark: plate
                    .verified
                    .then_some(crate::ui_renderer::VERIFIED_TEXTURE),
                client: plate.client,
                shared: plate.shared,
                withheld: plate.withheld,
                duelling: plate.duelling,
                duel_colour: 0,
                picture: match (plate.picture, plate.stand_in) {
                    (Some(texture), _) => Picture::Texture(texture),
                    (None, Some((_, colour))) => Picture::StandIn(colour),
                    (None, None) => Picture::Nothing,
                },
                initial: plate
                    .stand_in
                    .map_or_else(Initial::default, |(initial, _)| Initial::new(initial)),
                sjk: plate.sjk,
            });
        }
        let initials = self.initials();
        let label = |id: TextId| {
            if id.0 == UNKNOWN_TEXT {
                "?"
            } else if let Some(initial) = initial(&initials, id) {
                initial
            } else {
                names.get(id.0 as usize).copied().unwrap_or("")
            }
        };
        self.build(&label, power_icons, font, viewport);
        self.append_text(label, vertices, font, viewport);
    }

    /// Draw the previews in `style`.
    pub(crate) fn set_style(&mut self, style: Style) {
        self.settings.style = style;
    }
}

impl crate::GpuState {
    /// `nameplates [off|names|target|all]`: the next mode, or the one named; it is
    /// shown on screen for a moment.
    pub(crate) fn nameplate_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let console = self
            .console
            .as_mut()
            .ok_or_else(|| "no console".to_owned())?;
        let current = Mode::of(
            console.bool_cvar("cg_nameplate").unwrap_or(false),
            console.integer_cvar("cg_nameplateBars").unwrap_or(2),
        );
        let mode = match args.first() {
            None => current.next(),
            Some(word) => Mode::named(word)
                .ok_or_else(|| "usage: nameplates [off | names | target | all]".to_owned())?,
        };
        let (enabled, bars) = mode.settings();
        console.set_cvar("cg_nameplate", if enabled { "1" } else { "0" });
        if let Some(bars) = bars {
            console.set_cvar("cg_nameplateBars", &bars.to_string());
        }
        self.hud.nameplate.announce(mode);
        Ok(vec![mode.label().to_owned()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sharer's Force bar is the Force they last shared, as a share of their pool,
    /// with no estimate; a player who shares nothing has none.
    #[test]
    fn the_force_bar_is_the_shared_value() {
        let mut state = State::default();
        let mut shared = [None; MAX_PLAYER_TAGS];
        shared[3] = Some(SharedSlot {
            estimates: false,
            vitals: Some(sjk_identity::Vitals {
                t: 1_000,
                health: 80,
                armor: 20,
                force: 40,
                max_health: 100,
                max_force: 100,
            }),
        });
        state.refresh_shared(0, || (shared, true));
        assert_eq!(state.shared_force(3), Some(Range::exact(0.4)));
        assert_eq!(state.shared_force(4), None);
    }

    /// See and share nameplates is one switch, off by default: nameplates on share,
    /// whatever bars they show (names only too); off shares nothing.
    #[test]
    fn nameplates_on_share_and_off_does_not() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut state = State::default();
        // Off by default: a new player sees no nameplates and shares nothing.
        assert!(!see_and_share(&console));
        state.sample(Some(&console));
        assert!(!state.sharing(None).share);
        for (enabled, bars, shares) in [("1", "2", true), ("1", "0", true), ("0", "2", false)] {
            console.set_cvar("cg_nameplate", enabled);
            console.set_cvar("cg_nameplateBars", bars);
            state.sample(Some(&console));
            assert_eq!(state.sharing(None).share, shares, "{enabled} {bars}");
            assert_eq!(see_and_share(&console), shares, "{enabled} {bars}");
        }
    }

    /// `cg_nameplateAlpha` reads 0.1 to 1 (solid by default), and every style draws its
    /// whole plate at that opacity times the plate's own fade.
    #[test]
    fn the_opacity_setting_fades_every_style() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let mut state = State::default();
        state.sample(Some(&console));
        assert_eq!(state.settings.opacity, 1.0);
        for (value, read) in [("0.5", 0.5), ("0", MIN_OPACITY), ("3", 1.0)] {
            console.set_cvar(OPACITY_CVAR, value);
            state.sample(Some(&console));
            assert!((state.settings.opacity - read).abs() < 1e-6, "{value}");
        }
        let font = crate::text::load_classic().expect("the HUD font");
        for style in Style::ALL {
            let mut state = State::default();
            state.set_style(style);
            state.settings.opacity = 0.5;
            state.preview(
                &[plate()],
                &["Hello"],
                &[],
                &font.font,
                &mut Vec::new(),
                [800.0, 600.0],
            );
            let first = state
                .list
                .commands()
                .iter()
                .find_map(|command| match command {
                    DrawCommand::PushOpacity(value) => Some(*value),
                    _ => None,
                });
            assert_eq!(first, Some(0.5), "{style:?}");
        }
    }

    #[test]
    fn v_cycles_off_names_target_everyone_and_round() {
        let mut mode = Mode::Off;
        let mut seen = Vec::new();
        for _ in 0..4 {
            mode = mode.next();
            seen.push(mode);
        }
        assert_eq!(seen, [Mode::Names, Mode::Target, Mode::All, Mode::Off]);
        // Each mode's settings read back as that mode.
        for mode in seen {
            let (enabled, bars) = mode.settings();
            assert_eq!(Mode::of(enabled, bars.unwrap_or(2)), mode);
        }
        // Bars on allies only sits between names and target.
        assert_eq!(Mode::of(true, 1).next(), Mode::Target);
        assert_eq!(Mode::named("TARGET"), Some(Mode::Target));
        assert_eq!(Mode::named("everyone"), Some(Mode::All));
        assert_eq!(Mode::named("x"), None);
    }

    #[test]
    fn the_mode_name_shows_for_a_moment_then_goes() {
        let mut state = State {
            last_update: 10_000,
            ..State::default()
        };
        state.announce(Mode::Target);
        let font = crate::text::load_modern(1.0, None).unwrap().font;
        let label = |id: TextId| {
            if id.0 == MODE_TEXT {
                Mode::Target.label()
            } else {
                ""
            }
        };
        let texts = |state: &mut State| {
            state.list.clear();
            state.mode_label(&label, &font, 1.0, [1920.0, 1080.0]);
            state
                .list
                .commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count()
        };
        assert_eq!(texts(&mut state), 1);
        state.last_update = 10_000 + MODE_SHOWN;
        assert_eq!(texts(&mut state), 0);
    }

    #[test]
    fn icons_list_the_continuous_powers_dark_side_first() {
        let active = (1 << 9) | (1 << 7) | (1 << 2);
        assert_eq!(icon_powers(active), ([7, 9, 2, 0], 3));
    }

    #[test]
    fn instant_powers_get_no_icon_and_the_row_is_capped() {
        // Jump, push, pull and the saber powers are skipped.
        let instant = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 16) | (1 << 17);
        assert_eq!(icon_powers(instant).1, 0);
        let many = u32::MAX & !instant;
        let (powers, count) = icon_powers(many);
        assert_eq!(usize::from(count), MAX_ICONS);
        assert_eq!(powers, [7, 6, 13, 8]);
    }

    /// A plate laid out as [`State::append`] does, for a player in slot 0 named `name`.
    fn laid_out(plate: PreviewPlate, name: &str) -> (State, crate::text::FontAtlas) {
        // The HUD font the plates use in game hangs its glyphs from the line top by
        // its own baseline, low in the line box.
        let font = crate::text::load_classic().expect("the HUD font");
        let mut state = State::default();
        state.preview(
            &[plate],
            &[name],
            &[],
            &font.font,
            &mut Vec::new(),
            [800.0, 600.0],
        );
        (state, font)
    }

    /// `plate`, named Hello, laid out in `style`.
    fn laid_out_in(style: Style, plate: PreviewPlate) -> State {
        let font = crate::text::load_classic().expect("the HUD font");
        let mut state = State::default();
        state.set_style(style);
        state.preview(
            &[plate],
            &["Hello"],
            &[],
            &font.font,
            &mut Vec::new(),
            [800.0, 600.0],
        );
        state
    }

    /// The shapes of a laid out plate drawn in `colour`.
    fn painted(state: &State, colour: Color) -> usize {
        state
            .list
            .commands()
            .iter()
            .filter(|command| match command {
                DrawCommand::SolidRect { color, .. } | DrawCommand::RoundedRect { color, .. } => {
                    *color == colour
                }
                _ => false,
            })
            .count()
    }

    /// `cg_nameplateStyle` names the three looks, and any other value is the holo frame.
    #[test]
    fn the_style_setting_names_the_three_looks() {
        for style in Style::ALL {
            assert_eq!(Style::of(style.value()), style);
        }
        assert_eq!(Style::of(7), Style::Holo);
        assert_eq!(Style::of(-1), Style::Holo);
        assert_eq!(Style::default(), Style::Holo);
    }

    /// Every style shows a sharer's bars; a player who does not share has grey bars on
    /// the classic plate only, and the capsule a tray only for a sharer.
    #[test]
    fn each_style_draws_a_sharers_bars_and_only_the_classic_greys_the_rest() {
        let sharer = || PreviewPlate {
            health: Some([0.8, 0.8, 0.8]),
            shield: Some([0.5, 0.5, 0.5]),
            force: Some([0.6, 0.6, 0.6]),
            duelling: false,
            shared: true,
            ..plate()
        };
        let withheld = || PreviewPlate {
            health: Some([0.0, 0.0, 0.0]),
            shield: Some([0.0, 0.0, 0.0]),
            force: Some([0.0, 0.0, 0.0]),
            shared: false,
            withheld: true,
            ..sharer()
        };
        for style in Style::ALL {
            let state = laid_out_in(style, sharer());
            assert!(painted(&state, math::HEALTH_COLOR) > 0, "{style:?}");
            let grey = painted(&laid_out_in(style, withheld()), classic::WITHHELD_BAR);
            assert_eq!(grey > 0, style == Style::Classic, "{style:?}");
            let holo = !polygons(&state).is_empty();
            assert_eq!(holo, style == Style::Holo, "{style:?}");
        }
        let tray = |state: &State| {
            state
                .list
                .commands()
                .iter()
                .any(|command| matches!(command, DrawCommand::PushRoundedClip { .. }))
        };
        assert!(tray(&laid_out_in(Style::Capsule, sharer())));
        assert!(!tray(&laid_out_in(Style::Capsule, withheld())));
    }

    fn plate() -> PreviewPlate {
        PreviewPlate {
            slot: 0,
            point: [400.0, 300.0],
            distance: 200.0,
            health: None,
            shield: None,
            force: None,
            powers: 0,
            verified: true,
            client: None,
            duelling: true,
            team: None,
            shared: false,
            withheld: false,
            picture: None,
            stand_in: Some(('H', Color::new(0.2, 0.5, 0.45, 1.0))),
            sjk: true,
        }
    }

    fn quad(state: &State, texture: TextureId) -> Option<Rect> {
        state
            .list
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::TexturedQuad {
                    rect, texture: t, ..
                } if *t == texture => Some(*rect),
                _ => None,
            })
    }

    /// The polygons of a laid out plate: the hexagons and the plate, with their colours.
    fn polygons(state: &State) -> Vec<(Vec<[f32; 2]>, Color)> {
        state
            .list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Polygon {
                    points,
                    count,
                    color,
                    ..
                } => Some((points[..usize::from(*count)].to_vec(), *color)),
                _ => None,
            })
            .collect()
    }

    fn bounds(points: &[[f32; 2]]) -> Rect {
        let (mut low, mut high) = ([f32::MAX; 2], [f32::MIN; 2]);
        for [x, y] in points {
            low = [low[0].min(*x), low[1].min(*y)];
            high = [high[0].max(*x), high[1].max(*y)];
        }
        Rect::new(low[0], low[1], high[0] - low[0], high[1] - low[1])
    }

    /// The plate with its cut corner comes first, then the hexagon over its left end
    /// holding the stand-in's initial; the name follows with its seal level with its
    /// capitals, and the duel badge sits on the hexagon's corner.
    #[test]
    fn the_hexagon_holds_the_picture_and_the_seal_follows_the_name() {
        let (state, font) = laid_out(plate(), "Hello");
        let font = &font.font;
        let commands = state.list.commands();
        let (line, size) = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    rect, size, text, ..
                } if *text == TextId(0) => Some((*rect, *size)),
                _ => None,
            })
            .expect("the name");
        let shapes = polygons(&state);
        let (plate, hex) = (bounds(&shapes[0].0), bounds(&shapes[1].0));
        assert_eq!(shapes[0].0.len(), 5, "the plate's cut corner");
        assert_eq!(shapes[1].0.len(), 6, "a hexagon");
        assert!(
            hex.x < plate.x && hex.right() > plate.x,
            "{hex:?} {plate:?}"
        );
        let capital = font.glyph(crate::text::TextFace::Regular, b'H');
        let scale = size / font.height;
        let middle = line.y + (capital.offset_y + capital.height * 0.5) * scale;
        let badge = quad(&state, crate::ui_renderer::VERIFIED_TEXTURE).expect("the badge");
        assert!((badge.y + badge.height * 0.5 - middle).abs() < 0.01);
        let width = crate::text::visible_text_width(font, "Hello", scale);
        assert!(badge.x >= line.x + width && badge.right() <= plate.right());
        assert!(line.x > hex.right());
        let duel = quad(&state, crate::ui_renderer::duel_texture(0)).expect("the duel badge");
        assert!(duel.right() > hex.right() && duel.x < hex.right());
        let initial = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { rect, text, .. } if *text == TextId(INITIAL_TEXT) => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("the initial");
        assert!(initial.x >= hex.x && initial.right() <= hex.right() + 0.01);
    }

    /// A sharer's bars stand in their plate under a gold corner; a player who does not
    /// share (and a duellist) has a dotted line under the name instead: no bars.
    #[test]
    fn only_a_sharers_bars_show() {
        let bars = || PreviewPlate {
            health: Some([0.8, 0.8, 0.8]),
            shield: Some([0.0, 0.0, 0.0]),
            force: Some([0.5, 0.5, 0.5]),
            duelling: false,
            shared: true,
            ..plate()
        };
        let health = |state: &State| {
            state
                .list
                .commands()
                .iter()
                .filter(|command| {
                    matches!(command, DrawCommand::SolidRect { color, .. }
                        if *color == math::HEALTH_COLOR)
                })
                .count()
        };
        let gold = |state: &State| {
            state.list.commands().iter().any(|command| {
                matches!(command, DrawCommand::Line { color, .. } if *color == SHARED_GOLD)
            })
        };
        let (sharer, _) = laid_out(bars(), "Hello");
        assert!(health(&sharer) > 3, "health in segments");
        assert!(gold(&sharer));
        let (withheld, _) = laid_out(
            PreviewPlate {
                withheld: true,
                shared: false,
                ..bars()
            },
            "Hello",
        );
        assert_eq!(health(&withheld), 0);
        assert!(!gold(&withheld));
        let (duel, _) = laid_out(
            PreviewPlate {
                duelling: true,
                ..bars()
            },
            "Hello",
        );
        assert_eq!(health(&duel), 0);
    }

    /// A player without SJK wears a grey frame, an SJK player the holo blue, a team its
    /// colour.
    #[test]
    fn the_frame_tells_sjk_players_and_teams() {
        let hex = |plate: PreviewPlate| polygons(&laid_out(plate, "Hello").0)[1].1;
        let without = PreviewPlate {
            sjk: false,
            duelling: false,
            ..plate()
        };
        assert_eq!(hex(without), GREY_HEX);
        let with = PreviewPlate {
            duelling: false,
            ..plate()
        };
        assert_eq!(hex(with), HOLO_HEX);
        let blue = hex(PreviewPlate {
            duelling: false,
            team: Some(false),
            ..plate()
        });
        assert!(blue.b > blue.r && blue.a == 1.0, "{blue:?}");
    }

    /// A model's head icon is kept under one key for every player wearing it, and the
    /// initial of a stand-in is drawn from a few bytes, whatever the letter.
    #[test]
    fn head_keys_follow_the_model_and_initials_keep_their_letter() {
        assert_eq!(head_key(b"kyle/default"), head_key(b"kyle/default"));
        assert_ne!(head_key(b"kyle/default"), head_key(b"kyle/red"));
        assert_eq!(Initial::new('E').as_str(), "E");
        assert_eq!(Initial::new('\u{c9}').as_str(), "\u{c9}");
        let initials = [Initial::new('Q'); MAX_TAGS];
        assert_eq!(initial(&initials, TextId(INITIAL_TEXT + 3)), Some("Q"));
        assert_eq!(initial(&initials, TextId(5)), None);
        let picture =
            SlotPicture::new("0123456789abcdef", "fedcba9876543210", "^1Sol").expect("valid");
        assert_eq!(picture.initial.as_str(), "S");
        assert_eq!(
            picture.wanted(),
            crate::avatars::Wanted::Hub("0123456789abcdef", "fedcba9876543210")
        );
        let none = SlotPicture::new("0123456789abcdef", "", "Sol").expect("valid");
        assert_eq!(none.wanted(), crate::avatars::Wanted::Nothing);
        assert_eq!(SlotPicture::new("nope", "", "Sol"), None);
    }

    mod drain {
        use super::*;
        use sjk_protocol::{EntityState, LEGACY_ENTITY_FIELDS, PlayerState};

        pub(super) fn snapshot(time: i32, draining: bool, ahead: f32, pool: u32) -> Snapshot {
            let mut player = PlayerState::zero();
            player.set_client_num(0);
            player.stats[0] = 100;
            player.stats[8] = 100;
            player.set_raw_field(18, pool);
            player.set_origin([0.0, 0.0, 24.0]);
            player.set_view_angles([0.0, 0.0, 0.0]);
            if draining {
                player.set_raw_field(82, 1 << FP_DRAIN);
            }
            let mut victim = EntityState::zero(3, &LEGACY_ENTITY_FIELDS);
            victim.set_raw_field(8, u32::from(ET_PLAYER));
            for (field, value) in [2, 1, 4].into_iter().zip([ahead, 0.0, 24.0]) {
                victim.set_raw_field(field, value.to_bits());
            }
            Snapshot {
                message_sequence: 0,
                reliable_acknowledge: 0,
                server_commands: Vec::new(),
                server_time: time,
                delta_from: None,
                flags: 0,
                area_mask: Vec::new(),
                player,
                vehicle_player: None,
                entities: vec![victim],
                consumed_bits: 0,
            }
        }
    }

    /// Who a duel's opponents are: a private duel's one, the other player in a Duel
    /// arena, the other side of a Power Duel; nobody for a spectator or outside duels.
    #[test]
    fn duel_opponents_by_kind() {
        let mut snapshot = drain::snapshot(1_000, false, 200.0, 100);
        let mut game = GameState::empty_local(0);
        assert_eq!(Duel::of(&snapshot, &game, 0), Duel::None);
        snapshot.player.set_raw_field(44, 3);
        snapshot.player.set_raw_field(119, 1);
        let private = Duel::of(&snapshot, &game, 0);
        assert_eq!(private, Duel::Private(3));
        assert!(private.opponent(&game, 3) && !private.opponent(&game, 4));
        snapshot.player.set_raw_field(119, 0);
        assert_eq!(Duel::of(&snapshot, &game, GT_DUEL), Duel::Arena);
        game.replace_config_string(1131, br"\n\Me\t\0\ds\m\dt\1".to_vec())
            .unwrap();
        game.replace_config_string(1134, br"\n\Lone\t\0\ds\f\dt\2".to_vec())
            .unwrap();
        game.replace_config_string(1135, br"\n\Pair\t\0\ds\m\dt\1".to_vec())
            .unwrap();
        let sides = Duel::of(&snapshot, &game, GT_POWERDUEL);
        assert_eq!(sides, Duel::Sides(1));
        assert!(sides.opponent(&game, 3) && !sides.opponent(&game, 4));
    }

    /// In a duel the opponent's health, shield and Force never show, though they share
    /// and the bars are on, and the duellist's own values are not sent; after the duel
    /// both come back.
    #[test]
    fn a_duel_hides_the_opponents_bars_and_sends_nothing() {
        use sjk_runtime::{EntityId, EntityKind, MotionSample, Transform, World, WorldId};

        let mut snapshot = drain::snapshot(1_000, false, 200.0, 100);
        let mut game = GameState::empty_local(0);
        game.replace_config_string(1134, br"\n\Opponent\t\0".to_vec())
            .unwrap();
        let mut world = World::new(WorldId::new(1));
        world.upsert(
            EntityId::new(4),
            EntityKind::Actor,
            MotionSample {
                time_millis: 1_000,
                transform: Transform {
                    translation: [200.0, 0.0, 24.0],
                    ..Transform::IDENTITY
                },
            },
        );
        let camera = Camera {
            eye: Vec3::ZERO,
            target: Vec3::X,
            up: Vec3::Z,
            fov: 90.0,
            viewport: [1920.0, 1080.0],
        };
        let bsp = Bsp::empty([-4096.0; 3], [4096.0; 3]);
        let mut scratch = TraceScratch::default();
        let mut state = State::default();
        let mut shared = [None; MAX_PLAYER_TAGS];
        shared[3] = Some(SharedSlot {
            estimates: false,
            vitals: Some(sjk_identity::Vitals {
                t: 1_000,
                health: 80,
                armor: 20,
                force: 40,
                max_health: 100,
                max_force: 100,
            }),
        });
        state.refresh_shared(0, || (shared, true));
        let mut now = 1_000;
        let mut plate = |state: &mut State, snapshot: &Snapshot| {
            state.observe_snapshot(snapshot, &game, &bsp, &mut scratch);
            state.update(
                snapshot,
                &game,
                &world,
                &TeamInfoTable::default(),
                now,
                camera,
                &bsp,
                &mut scratch,
                None,
                false,
            );
            now += 100;
            let entry = *state
                .entries
                .iter()
                .find(|entry| entry.number == 3)
                .unwrap();
            (entry.health, entry.shield, entry.force)
        };
        let (health, shield, force) = plate(&mut state, &snapshot);
        assert!(health.is_some() && shield.is_some(), "a sharer's bars show");
        assert_eq!(force, Some(Range::exact(0.4)));
        assert!(state.sharing(Some(&snapshot)).vitals.is_some());
        snapshot.player.set_raw_field(44, 3);
        snapshot.player.set_raw_field(119, 1);
        for bars in [2, BARS_TARGET] {
            state.settings.bars = bars;
            assert_eq!(plate(&mut state, &snapshot), (None, None, None), "{bars}");
        }
        let sharing = state.sharing(Some(&snapshot));
        assert!(sharing.share && sharing.vitals.is_none());
        state.settings.bars = 2;
        snapshot.player.set_raw_field(119, 0);
        assert!(plate(&mut state, &snapshot).0.is_some());
        assert!(state.sharing(Some(&snapshot)).vitals.is_some());
    }

    #[test]
    fn private_duel_filter_removes_whole_plates_and_restores_them() {
        use sjk_runtime::{EntityId, EntityKind, MotionSample, Transform, World, WorldId};

        let mut snapshot = drain::snapshot(1_000, false, 200.0, 100);
        snapshot.entities[0].set_raw_field(91, 1); // Player bolt1: private duel.
        let mut game = GameState::empty_local(0);
        // Power Duel side alone must never hide a plate.
        game.replace_config_string(1134, br"\n\Player\t\0\dt\1".to_vec())
            .unwrap();
        let mut world = World::new(WorldId::new(1));
        world.upsert(
            EntityId::new(4),
            EntityKind::Actor,
            MotionSample {
                time_millis: 1_000,
                transform: Transform {
                    translation: [200.0, 0.0, 24.0],
                    ..Transform::IDENTITY
                },
            },
        );
        let camera = Camera {
            eye: Vec3::ZERO,
            target: Vec3::X,
            up: Vec3::Z,
            fov: 90.0,
            viewport: [1920.0, 1080.0],
        };
        let bsp = Bsp::empty([-4096.0; 3], [4096.0; 3]);
        let mut scratch = TraceScratch::default();
        let mut state = State::default();
        state.settings.own = true;
        state.set_own_origin(Some([200.0, 30.0, 24.0]));
        snapshot.player.set_raw_field(119, 1); // Local private duel.
        let mut now = 1_000;
        let mut collect = |state: &mut State, snapshot: &Snapshot| {
            state.update(
                snapshot,
                &game,
                &world,
                &TeamInfoTable::default(),
                now,
                camera,
                &bsp,
                &mut scratch,
                None,
                false,
            );
            now += 100;
            state
                .entries
                .iter()
                .map(|entry| entry.number)
                .collect::<Vec<_>>()
        };
        assert!(!state.settings.hide_duelers);
        assert_eq!(collect(&mut state, &snapshot).len(), 2);
        state.settings.hide_duelers = true;
        assert!(collect(&mut state, &snapshot).is_empty());
        state.settings.hide_duelers = false;
        assert_eq!(collect(&mut state, &snapshot).len(), 2);
        state.settings.hide_duelers = true;
        snapshot.entities[0].set_raw_field(91, 0);
        snapshot.player.set_raw_field(119, 0);
        assert_eq!(collect(&mut state, &snapshot).len(), 2);
        // NPC bolt1 has other uses: an NPC with the bit still gets its plate.
        state.settings.npcs = true;
        snapshot.entities[0].set_raw_field(8, u32::from(ET_NPC));
        snapshot.entities[0].set_raw_field(88, 18); // Jedi class.
        snapshot.entities[0].set_raw_field(69, 100);
        snapshot.entities[0].set_raw_field(91, 1);
        assert_eq!(collect(&mut state, &snapshot).len(), 2);
    }
}
