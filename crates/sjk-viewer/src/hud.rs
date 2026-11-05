//! Data-driven in-game HUD over authoritative snapshot values.

mod data_source;
pub(crate) mod duel_pairs;
use data_source::*;
pub(crate) mod crosshair;
pub(crate) mod enemy_info;
mod estimate;
pub(crate) mod family;
pub(crate) mod flag_status;
mod follow;
pub(crate) use follow::shown as follow_shown;
mod force_streams;
pub(crate) mod force_wheel;
mod hack_bar;
mod jetpack_fuel;
pub(crate) mod lagometer;
mod monitor_marks;
pub(crate) use monitor_marks::MonitorMarks;
pub(crate) mod icons;
pub(crate) mod identification;
mod info;
pub(crate) mod kill_feed;
pub(crate) mod movement;
pub(crate) mod movement_keys;
pub(crate) mod nameplate;
pub(crate) mod nameplate_math;
mod npc_class;
pub(crate) mod options;
pub(crate) mod placement;
pub(crate) mod player_card;
pub(crate) mod portrait;
mod radial;
pub(crate) mod rocket_lock;
mod selection;
mod sound_sonar;
pub(crate) mod targeting;
pub(crate) mod team_logo;
pub(crate) mod team_outline;
mod text_values;
pub(crate) mod tints;
pub(crate) mod toggle;
mod update;
pub(crate) mod vehicle;
pub(crate) mod vehicle_feed;
mod vitals_estimate;
mod vote;
mod widgets;

use super::{TextVertex, UiFont};
use crate::game_font::RetailFont;
use crate::{console::ViewerConsole, ui_renderer};
use sjk_client::pmove::MovementState;
use sjk_client::{
    ClientSession, HudDataSource as ClientHudData, legacy_hud_data, legacy_predicted_hud_data,
};
use sjk_protocol::{GameState, PlayerState};
use sjk_ui::{
    DrawList, Easing, HudDataSource, HudLayoutDocument, Insets, LayoutContext, LayoutEngine,
    LayoutKind, LayoutScratch, TextId, Theme, Tween, Vec2, Widget, WidgetId, WidgetTree,
};
use std::path::Path;

const WIDGET_LIMIT: usize = 64;
/// The HUD's draw commands: the 384 its other widgets had, and room for a full
/// kill feed (55), the strafe aura and the movement keys.
const DRAW_LIMIT: usize = 384
    + kill_feed::CAPACITY * kill_feed::ENTRY_COMMANDS
    + movement::AURA_COMMANDS
    + movement_keys::COMMANDS
    + sound_sonar::COMMANDS;
const DEFAULT_LAYOUT: &str = include_str!("../assets/hud/default.json");
/// The weapon name fades over this long at the end of its life.
const WEAPON_FADE_MS: u64 = 600;
/// The weapon name stays fully visible this long after a switch, then fades. Name and
/// fade together last as long as the weapon selection row (`WEAPON_SELECT_TIME`), which
/// replaces the name while it shows, so the name never appears after the row ends.
const WEAPON_HOLD_MS: u64 = crate::weapon_select::SHOW.as_millis() as u64 - WEAPON_FADE_MS;
/// Opacity of a transient that holds for `hold` ms and fades over `fade` ms.
fn transient_alpha(age_ms: u64, hold: u64, fade: u64) -> f32 {
    if age_ms <= hold {
        1.0
    } else {
        1.0 - (age_ms - hold).min(fade) as f32 / fade as f32
    }
}
const CLASSIC_LAYOUT: &str = include_str!("../assets/hud/classic.json");
const RADIAL_LAYOUT: &str = include_str!("../assets/hud/radial.json");

/// Independent visibility switches used by widget predicates and the HUD shader.
#[derive(Clone, Copy)]
pub(crate) struct HudVisibility {
    /// Some of the HUD's layout draws: its elements, or the crosshair alone when the HUD
    /// switch hides the rest ([`toggle`]).
    pub(crate) hud: bool,
    /// The HUD's elements other than the crosshair (status, timers, vote, kill feed, the
    /// layout's other widgets): off with `cg_draw2D 0`, or `cg_drawHud 0` while its
    /// [`toggle::Part::Elements`] switch is on.
    pub(crate) elements: bool,
    pub(crate) status: bool,
    /// The vehicle HUD, which `cg_drawStatus` governs too, but which stays when the
    /// vehicle takes the player's own status HUD (`hideRider`).
    pub(crate) vehicle: bool,
    pub(crate) weapon: bool,
    /// Weapon, inventory and Force selection rows, independently of status cvars.
    pub(crate) selection: bool,
    pub(crate) crosshair: bool,
    pub(crate) crosshair_names: bool,
    pub(crate) timer: bool,
    pub(crate) lagometer: bool,
    /// The third-person ground HUD stands in for health, shield, Force and
    /// stance this frame (`crate::ground_hud`), so those widgets hide.
    pub(crate) ground_hud: bool,
    /// The game-data menu HUD (`crate::menu_hud`) draws health, armor,
    /// Force and ammo this frame, so the built-in status and weapon widgets hide.
    pub(crate) menu_hud: bool,
}

impl HudVisibility {
    pub(crate) fn from_console(console: Option<&ViewerConsole>) -> Self {
        let enabled = |name, fallback| {
            console
                .and_then(|console| console.bool_cvar(name))
                .unwrap_or(fallback)
        };
        // `cg_drawHud 0` hides the parts its switches name (`toggle`); `cg_draw2D 0` all.
        let draw_2d = enabled("cg_draw2D", true);
        let elements = draw_2d && !toggle::hides(console, toggle::Part::Elements);
        let aim = draw_2d && !toggle::hides(console, toggle::Part::Crosshair);
        Self {
            hud: elements || aim,
            elements,
            status: elements && enabled("cg_drawStatus", true),
            vehicle: elements && enabled("cg_drawStatus", true),
            weapon: elements && enabled("cg_drawWeapon", true),
            selection: elements,
            crosshair: aim
                && console
                    .and_then(|c| c.integer_cvar("cg_drawCrosshair"))
                    .unwrap_or(1)
                    != 0,
            crosshair_names: aim && enabled("cg_drawCrosshairNames", true),
            timer: elements && enabled("cg_drawTimer", true),
            lagometer: elements && lagometer::mode(console) != 0,
            ground_hud: false,
            menu_hud: false,
        }
    }

    /// Intermission hides every HUD layer.
    pub(crate) const HIDDEN: Self = Self {
        hud: false,
        elements: false,
        status: false,
        vehicle: false,
        weapon: false,
        selection: false,
        crosshair: false,
        crosshair_names: false,
        timer: false,
        lagometer: false,
        ground_hud: false,
        menu_hud: false,
    };
}

/// Whether a layout widget is the crosshair or the name under it, which show by their
/// own flags (`draw_crosshair`, `draw_crosshair_name`) while the HUD switch hides the
/// other elements ([`toggle::Part::Crosshair`]).
fn crosshair_widget(widget: &sjk_ui::HudWidget) -> bool {
    matches!(
        &widget.visibility,
        sjk_ui::VisibilityCondition::Flag { binding }
            if binding == "draw_crosshair" || binding == "draw_crosshair_name"
    )
}

/// Allocation-free-per-frame widget state for the local player's HUD.
pub(crate) struct HudOverlay {
    /// Map-lifetime textures and sampled icon presentation policy.
    pub(crate) icons: icons::Icons,
    pub(crate) tints: tints::State,
    /// World-projected labels, sharing the chat roster's retained names.
    pub(crate) identification: identification::State,
    /// The card shown beside a player looked at for a moment.
    pub(crate) card: player_card::State,
    /// MMO-style nameplates, drawn in the classic font; they replace the labels above.
    pub(crate) nameplate: nameplate::State,
    /// Who duels whom, for the nameplates' duel badge colours.
    pub(crate) duel_pairs: duel_pairs::DuelPairs,
    /// Team logos over allies' heads, under the labels and nameplates.
    pub(crate) team_logo: team_logo::State,
    /// Who the team outlines ring this frame.
    pub(crate) team_outline: team_outline::State,
    guides: movement::Guides,
    /// JoF EJK's movement keys (`cg_movementKeys`).
    pub(crate) movement_keys: movement_keys::State,
    family: family::Policy,
    pub(crate) targeting: targeting::State,
    pub(crate) enemy_info: enemy_info::State,
    /// The pilot's vehicle HUD and the crosshair it changes.
    pub(crate) vehicle: vehicle::State,
    /// The CTF flag icons and the team score plates.
    pub(crate) flag_status: flag_status::State,
    score_text: String,
    snapshot_text: String,

    inventory_bits: u32,
    selector: Option<sjk_client::selection::SelectionView>,
    /// The Force selector is the retail icon bar ([`force_wheel`]), not the list.
    force_wheel_bar: bool,
    /// The bar's map-lifetime pictures.
    force_wheel_icons: [Option<sjk_ui::TextureId>; force_wheel::ICONS],
    extension_icons_tried: u32,
    /// The holocron tiers' pictures in the UI atlas (`holocrons/icons.rs`), where they
    /// loaded.
    pub(crate) holocron_icons: [Option<sjk_ui::TextureId>; crate::holocrons::COUNT],
    /// The classic crosshair's map-lifetime pictures.
    pub(crate) crosshair_pictures: [Option<sjk_ui::TextureId>; crosshair::PICTURES],
    /// The last layout drew the crosshair picture, so `hud.wgsl` skips its own.
    pub(crate) crosshair_picture_drawn: bool,
    /// The rocket launcher's lock-on around its target.
    pub(crate) rocket_lock: rocket_lock::State,
    /// The binoculars' heading numbers, while their view shows (`scope::binoculars`).
    pub(crate) binoculars: crate::scope::binoculars::Heading,
    /// JA+ merc mode's flamethrower, latched across snapshots.
    flamethrower: sjk_client::force_wheel::FlamethrowerOverride,
    /// The selector shows Force Lightning as the flamethrower.
    flamethrower_shown: bool,

    values: Option<ClientHudData>,
    health: String,
    armor: String,
    force: String,
    weapon: String,
    ammo: String,
    health_value: String,
    armor_value: String,
    force_value: String,
    weapon_value: String,
    ammo_value: String,
    style_value: String,
    /// HUD time of the last weapon change; drives the weapon-name fade.
    weapon_shown_ms: Option<u64>,
    /// Retail's weapon selection row (`crate::weapon_select`) and its names.
    pub(crate) weapon_select: crate::weapon_select::State,
    weapon_alpha: f32,
    vote_active: bool,
    team_vote_active: bool,
    vote_heading: String,
    vote_text: String,
    team_vote_heading: String,
    team_vote_text: String,
    vote_keys: String,
    yes_keys: String,
    no_keys: String,
    /// The kill feed at the top right.
    pub(crate) kill_feed: kill_feed::Feed,
    crosshair_name: String,
    speed: options::Speed,
    crosshair_alpha: f32,
    crosshair_teammate: bool,
    match_timer: String,
    /// Whether the match clock shows in the upper right this frame (`cg_drawTimer`,
    /// `cg_drawUpperRight`, a clock to show), as JoF EJK's `CG_DrawUpperRight` puts it.
    upper_right_timer: bool,
    warmup_text: String,
    /// The followed player's name (`CG_DrawFollow`), empty when not following.
    followed_name: String,
    interrupted: bool,
    /// The share of the player's hack still to go (`hack_bar`), `None` while it hacks
    /// nothing.
    hack_left: Option<f32>,
    /// The jetpack fuel the bar at the right edge shows (`jetpack_fuel`), `None` while
    /// it is hidden.
    jetpack_fuel: Option<i32>,
    /// The Hax toy's hack still to go (`hax.rs`), drawn as a siege hack is.
    toy_hack_left: Option<f32>,
    /// The Monitor toy's charge ring and hit marker (`monitor.rs`).
    monitor_marks: MonitorMarks,
    /// The sound sonar's arcs this frame (`crate::sound_sonar`).
    sound_sonar: crate::sound_sonar::Markers,
    lagometer: sjk_client::LagometerSamples,
    /// `cg_lagometer` 2 and 3's numbers and speed history ([`lagometer`]).
    lagometer_readout: lagometer::Readout,
    default_document: HudLayoutDocument,
    classic_document: HudLayoutDocument,
    override_document: Option<HudLayoutDocument>,
    tree: WidgetTree,
    scratch: LayoutScratch,
    draw_list: DrawList,
    theme: Theme,
    ratios: [Tween; 3],
    displayed_ratios: [f32; 3],
    ammo_ratio: Tween,
    displayed_ammo: f32,
    /// The radial look as bundled, and as drawn: placed by the player's spacing and
    /// height ([`Self::set_radial_placement`]).
    radial_base: HudLayoutDocument,
    radial_document: HudLayoutDocument,
    radial_placement: RadialPlacement,
    /// Where the player moved the elements without cvars of their own
    /// ([`placement::OFFSETS_CVAR`]).
    offsets: placement::Offsets,
    /// The box each element drew in the last layout, for the HUD edit mode.
    pub(crate) element_bounds: placement::Bounds,
}

/// Where the player wants the radial look's status meters (Lex's idea, 29/10/2026: too
/// close to the character): `spread` logical pixels farther out on each side
/// (`cg_hudRadialSpread`), `height` per cent of the screen higher (`cg_hudRadialHeight`;
/// negative is lower).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RadialPlacement {
    pub(crate) spread: f32,
    pub(crate) height: f32,
}

impl RadialPlacement {
    pub(crate) const SPREAD_CVAR: &'static str = "cg_hudRadialSpread";
    pub(crate) const HEIGHT_CVAR: &'static str = "cg_hudRadialHeight";
    pub(crate) const SPREAD_MAX: f32 = 250.0;
    pub(crate) const HEIGHT_RANGE: (f32, f32) = (-20.0, 40.0);

    /// The placement the console's cvars ask for, within its limits.
    pub(crate) fn read(console: Option<&crate::console::ViewerConsole>) -> Self {
        let number = |name: &str| {
            console
                .and_then(|console| console.float_cvar(name))
                .unwrap_or(0.0) as f32
        };
        let (low, high) = Self::HEIGHT_RANGE;
        Self {
            spread: number(Self::SPREAD_CVAR).clamp(0.0, Self::SPREAD_MAX),
            height: number(Self::HEIGHT_CVAR).clamp(low, high),
        }
    }
}

/// The radial look's status meters, the pills, arcs and numbers round the crosshair:
/// what [`RadialPlacement`] moves. The crosshair, its name, the timer and the rest stay.
const RADIAL_STATUS: [&str; 23] = [
    "left_pill",
    "right_pill",
    "health_arc",
    "armor_arc",
    "force_arc",
    "ammo_arc",
    "style_arc",
    "health_value",
    "armor_value",
    "ammo_value",
    "force_value",
    "style_value",
    "weapon_value",
    "vehicle_arc",
    "vehicle_value",
    "vehicle_speed_arc",
    "vehicle_ammo_arc",
    "vehicle_speed_value",
    "vehicle_ammo_value",
    "vehicle_turbo_arc_charging",
    "vehicle_turbo_arc_ready",
    "vehicle_turbo_arc_burning",
    "vehicle_turbo_value",
];

/// Which side of the crosshair a status widget is on: -1 left, 1 right, 0 under it. An
/// arc is centred on the crosshair, so its side is its sweep's; anything else, its
/// offset's. The vehicle's arcs and numbers stay together under it.
fn radial_side(widget: &sjk_ui::HudWidget) -> f32 {
    if widget.id.starts_with("vehicle_") {
        return 0.0;
    }
    let x = match widget.style.arc {
        Some(arc) => (arc.start_degrees + arc.sweep_degrees * 0.5)
            .to_radians()
            .cos(),
        None => widget.offset.x,
    };
    if x.abs() < 1e-3 { 0.0 } else { x.signum() }
}

/// `base` with its status meters placed as `placement` says.
fn place_radial(base: &HudLayoutDocument, placement: RadialPlacement) -> HudLayoutDocument {
    let mut document = base.clone();
    for widget in &mut document.widgets {
        if RADIAL_STATUS.contains(&widget.id.as_str()) {
            widget.offset.x += radial_side(widget) * placement.spread;
            widget.offset_fraction.y -= placement.height / 100.0;
        }
    }
    document
}

/// Which bundled layout document the status HUD is drawn from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HudLook {
    /// Under the game-data HUD: the default layout (or a `hud.json` override),
    /// whose crosshair, team rows, votes, kill feed, timer and lagometer stay
    /// while the game HUD draws the status, and whose status widgets stand in
    /// when its files give none; the classic layout with `cg_classicHudFont`.
    Game,
    /// SJK's classic layout.
    Classic,
    /// SJK's radial layout: arcs around the crosshair.
    Radial,
}

/// Pixel-space geometry consumed by the existing fullscreen HUD renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HudLayout {
    pub(crate) health_bar: [f32; 4],
    pub(crate) armor_bar: [f32; 4],
    pub(crate) force_bar: [f32; 4],
}

impl HudOverlay {
    /// The Force power pictures, by `forcePowers_t` index, which the nameplates reuse.
    pub(crate) fn power_icons(&self) -> [Option<sjk_ui::TextureId>; force_wheel::ICONS] {
        self.force_wheel_icons
    }

    /// Whether the Force selector shows Lightning as JA+ merc mode's flamethrower.
    pub(crate) fn flamethrower_shown(&self) -> bool {
        self.flamethrower_shown
    }

    /// How much larger the procedural crosshair is drawn while riding a vehicle.
    pub(crate) fn vehicle_crosshair_factor(&self) -> f32 {
        self.vehicle.crosshair_factor(self.targeting.policy.look)
    }

    pub(crate) fn new() -> Self {
        let override_document = crate::platform::user_config_file()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("hud.json")))
            .and_then(|path| load_override(&path));
        Self {
            icons: icons::Icons::default(),
            weapon_select: crate::weapon_select::State::new(),
            tints: tints::State::default(),
            guides: movement::Guides::default(),
            movement_keys: movement_keys::State::default(),
            family: family::Policy::default(),
            targeting: targeting::State::default(),
            enemy_info: enemy_info::State::default(),
            vehicle: vehicle::State::default(),
            identification: identification::State::default(),
            card: player_card::State::default(),
            nameplate: nameplate::State::default(),
            duel_pairs: duel_pairs::DuelPairs::default(),
            team_logo: team_logo::State::default(),
            team_outline: team_outline::State::default(),
            flag_status: flag_status::State::default(),
            score_text: String::with_capacity(80),
            snapshot_text: String::with_capacity(96),

            inventory_bits: 0,
            values: None,
            health: String::with_capacity(24),
            armor: String::with_capacity(24),
            force: String::with_capacity(24),
            weapon: String::with_capacity(40),
            ammo: String::with_capacity(24),
            health_value: String::with_capacity(12),
            armor_value: String::with_capacity(12),
            force_value: String::with_capacity(12),
            weapon_value: String::with_capacity(32),
            ammo_value: String::with_capacity(12),
            style_value: String::with_capacity(12),
            weapon_shown_ms: None,
            weapon_alpha: 0.0,
            selector: None,
            force_wheel_bar: false,
            force_wheel_icons: [None; force_wheel::ICONS],
            extension_icons_tried: 0,
            holocron_icons: [None; crate::holocrons::COUNT],
            crosshair_pictures: [None; crosshair::PICTURES],
            rocket_lock: rocket_lock::State::default(),
            binoculars: Default::default(),
            crosshair_picture_drawn: false,
            flamethrower: Default::default(),
            flamethrower_shown: false,

            vote_active: false,
            team_vote_active: false,
            vote_heading: String::with_capacity(64),
            vote_text: String::with_capacity(160),
            team_vote_heading: String::with_capacity(64),
            team_vote_text: String::with_capacity(160),
            vote_keys: String::with_capacity(96),
            yes_keys: String::with_capacity(48),
            no_keys: String::with_capacity(48),
            kill_feed: kill_feed::Feed::default(),
            crosshair_name: String::with_capacity(64),
            speed: options::Speed::default(),
            crosshair_alpha: 0.0,
            crosshair_teammate: false,
            match_timer: String::with_capacity(16),
            upper_right_timer: false,
            warmup_text: String::with_capacity(48),
            followed_name: String::with_capacity(64),
            interrupted: false,
            hack_left: None,
            jetpack_fuel: None,
            toy_hack_left: None,
            monitor_marks: MonitorMarks::default(),
            sound_sonar: Default::default(),
            lagometer: sjk_client::LagometerSamples::new(),
            lagometer_readout: lagometer::Readout::default(),
            default_document: HudLayoutDocument::from_json(DEFAULT_LAYOUT)
                .expect("bundled default HUD document is valid"),
            classic_document: HudLayoutDocument::from_json(CLASSIC_LAYOUT)
                .expect("bundled classic HUD document is valid"),
            override_document,
            tree: WidgetTree::new(WIDGET_LIMIT),
            scratch: LayoutScratch::new(WIDGET_LIMIT),
            draw_list: DrawList::new(DRAW_LIMIT),
            theme: Theme::default(),
            ratios: [
                Tween::settled(1.0),
                Tween::settled(0.0),
                Tween::settled(1.0),
            ],
            displayed_ratios: [1.0, 0.0, 1.0],
            ammo_ratio: Tween::settled(0.0),
            displayed_ammo: 0.0,
            radial_base: HudLayoutDocument::from_json(RADIAL_LAYOUT)
                .expect("bundled radial HUD document is valid"),
            radial_document: HudLayoutDocument::from_json(RADIAL_LAYOUT)
                .expect("bundled radial HUD document is valid"),
            radial_placement: RadialPlacement::default(),
            offsets: placement::Offsets::default(),
            element_bounds: placement::Bounds::default(),
        }
    }

    /// Place the elements the player moved ([`placement`]).
    pub(crate) fn set_offsets(&mut self, offsets: placement::Offsets) {
        self.offsets = offsets;
    }

    /// Place the radial look's status meters as the player asks (laid out again only
    /// when it changes).
    pub(crate) fn set_radial_placement(&mut self, placement: RadialPlacement) {
        if placement != self.radial_placement {
            self.radial_placement = placement;
            self.radial_document = place_radial(&self.radial_base, placement);
        }
    }

    /// The share of a Hax toy's hack still to go, while the player hacks or is hacked.
    pub(crate) fn set_toy_hack(&mut self, left: Option<f32>) {
        self.toy_hack_left = left;
    }

    /// The Monitor toy's marks on the crosshair this frame.
    pub(crate) fn set_monitor_marks(&mut self, marks: MonitorMarks) {
        self.monitor_marks = marks;
    }

    /// The sound sonar's arcs this frame.
    pub(crate) fn set_sound_sonar(&mut self, markers: crate::sound_sonar::Markers) {
        self.sound_sonar = markers;
    }

    /// Build, lay out and retain the HUD widget draw list. `user_scale` is
    /// the player's `cg_hudScale` on top of the resolution-derived scale.
    /// The match clock to show in the upper right under the frame rate, as JoF EJK's
    /// `CG_DrawTimer` does, when it shows ([`Self::layout`] decides each frame).
    pub(crate) fn upper_right_timer(&self) -> Option<&str> {
        self.upper_right_timer.then_some(self.match_timer.as_str())
    }

    pub(crate) fn layout(
        &mut self,
        font: &UiFont,
        look: HudLook,
        viewport: [f32; 2],
        user_scale: f32,
        visibility: HudVisibility,
        time_ms: u64,
    ) -> HudLayout {
        self.upper_right_timer = visibility.timer
            && self.family.upper
            && self.family.timer_upper_right
            && !self.match_timer.is_empty();
        self.displayed_ratios = self.ratios.map(|tween| tween.sample(time_ms));
        self.displayed_ammo = self.ammo_ratio.sample(time_ms);
        self.weapon_alpha = self.weapon_shown_ms.map_or(0.0, |shown| {
            transient_alpha(
                time_ms.saturating_sub(shown),
                WEAPON_HOLD_MS,
                WEAPON_FADE_MS,
            )
        });
        let document = match look {
            HudLook::Radial => &self.radial_document,
            HudLook::Classic => &self.classic_document,
            HudLook::Game if font.is_modern() => self
                .override_document
                .as_ref()
                .unwrap_or(&self.default_document),
            HudLook::Game => &self.classic_document,
        };
        // The nameplates' bars wear this HUD's health, armour and Force colours.
        let meter = |binding: &str| {
            document
                .widgets
                .iter()
                .find(|widget| widget.binding.as_deref() == Some(binding))
                .and_then(|widget| widget.style.foreground)
        };
        let (health, armor, force) = (
            meter("health_ratio"),
            meter("armor_ratio"),
            meter("force_ratio"),
        );
        self.nameplate.set_hud_colors(health, armor, force);
        let data = WidgetData {
            visibility,
            ratios: self.displayed_ratios,
            ammo_ratio: self.displayed_ammo,
            health: &self.health,
            armor: &self.armor,
            force: &self.force,
            weapon: &self.weapon,
            ammo: &self.ammo,
            health_value: &self.health_value,
            armor_value: &self.armor_value,
            force_value: &self.force_value,
            weapon_value: &self.weapon_value,
            ammo_value: &self.ammo_value,
            saber_style: self.values.and_then(|values| values.saber_style),
            weapon_alpha: self.weapon_alpha,
            weapon_row: self.weapon_select.shown.is_some(),
            vote_active: self.vote_active,
            team_vote_active: self.team_vote_active,
            crosshair_name: !self.crosshair_name.is_empty(),
            timer: !self.match_timer.is_empty(),
            warmup: !self.warmup_text.is_empty(),
            interrupted: self.interrupted,
            // SJK's own looks draw the vehicle as their layouts' hull, speed and
            // ammunition meters; the game HUD look keeps the vector strip, where the
            // game HUD does not draw it.
            vehicle: self.vehicle.meters().filter(|_| look != HudLook::Game),
            has_ammo: self.values.is_some_and(|values| values.ammo.is_some()),
        };
        let mut visible = [false; WIDGET_LIMIT];
        for (index, widget) in document.widgets.iter().take(WIDGET_LIMIT).enumerate() {
            // The crosshair's widgets answer to their own flags; the others to the
            // HUD's elements, which the HUD switch may hide without the crosshair.
            visible[index] = (visibility.elements || crosshair_widget(widget))
                && visibility.hud
                && widget.visibility.evaluate(&data)
                && self.family.visible(widget.binding.as_deref());
        }
        let dpi_scale =
            crate::ui_scale::height_scale(viewport[1]).max(2.0 / 3.0) * user_scale.clamp(0.25, 2.0);
        let hero_scale = crate::ui_scale::height_scale(viewport[1]) * user_scale.clamp(0.25, 2.0);
        // Layout runs in logical pixels: the screen's size is its physical size over the scale.
        let logical_screen = [viewport[0] / dpi_scale, viewport[1] / dpi_scale];
        // Each widget's movable element ([`placement`]), and how far the player moved
        // it, in logical pixels.
        let radial = look == HudLook::Radial;
        let mut elements = [None; WIDGET_LIMIT];
        self.tree.clear();
        for (index, widget) in document.widgets.iter().take(WIDGET_LIMIT).enumerate() {
            elements[index] = placement::widget_element(&widget.id, radial);
            let moved = elements[index].map_or([0.0; 2], |element| {
                self.offsets
                    .pixels(element, viewport)
                    .map(|pixels| pixels / dpi_scale)
            });
            let _ = self.tree.add(Widget {
                id: WidgetId(index as u32),
                parent: None,
                layout: LayoutKind::Anchored {
                    anchor: widget.anchor,
                    offset: Vec2::new(
                        widget.offset.x + widget.offset_fraction.x * logical_screen[0] + moved[0],
                        widget.offset.y + widget.offset_fraction.y * logical_screen[1] + moved[1],
                    ),
                },
                size: widget.size,
                visible: visible[index],
                focusable: false,
                scrollable: false,
                opacity: 1.0,
                z: widget.layer,
            });
        }
        let rectangles = LayoutEngine.layout(
            &self.tree,
            LayoutContext {
                viewport_physical: Vec2::new(viewport[0], viewport[1]),
                dpi_scale,
                safe_area: Insets::all(0.0),
            },
            &mut self.scratch,
        );
        self.draw_list.clear();
        self.tints.emit(&mut self.draw_list, viewport);
        self.rocket_lock.emit(&mut self.draw_list);
        self.binoculars.emit(&mut self.draw_list, viewport);
        // A pilot's crosshair is doubled, and is the vehicle's own picture when its
        // `.veh` names one that loaded.
        let crosshair_look = self.vehicle.crosshair_look(self.targeting.policy.look);
        self.crosshair_picture_drawn = visibility.crosshair
            && match self.vehicle.crosshair_picture() {
                Some(texture) => crosshair::emit_picture(
                    &mut self.draw_list,
                    texture,
                    crosshair_look,
                    self.targeting.center(viewport),
                    self.targeting.color,
                    viewport,
                ),
                None => crosshair::emit(
                    &mut self.draw_list,
                    &self.crosshair_pictures,
                    crosshair_look,
                    self.targeting.center(viewport),
                    self.targeting.color,
                    viewport,
                ),
            };
        // Drawn by `CG_DrawCrosshair` after the crosshair, so only with it.
        if visibility.crosshair {
            hack_bar::emit(
                &mut self.draw_list,
                viewport,
                self.hack_left.or(self.toy_hack_left),
                crosshair::rect(crosshair_look, self.targeting.center(viewport), viewport),
                self.icons.cell(icons::assets::HACKER),
            );
            // The crosshair's middle is a fraction of the screen: in pixels here.
            let [x, y] = self.targeting.center(viewport);
            monitor_marks::emit(
                &mut self.draw_list,
                viewport,
                [x * viewport[0], y * viewport[1]],
                self.monitor_marks,
            );
        }
        if visibility.elements {
            let [x, y] = self.targeting.center(viewport);
            sound_sonar::emit(
                &mut self.draw_list,
                viewport,
                [x * viewport[0], y * viewport[1]],
                user_scale,
                &self.sound_sonar,
            );
        }
        let mut output = HudLayout::default();
        let low_health = self.values.is_some_and(|value| value.health <= 25);
        let low_ammo = self
            .values
            .is_some_and(|value| value.ammo.is_some_and(|ammo| ammo <= 5));
        let pulse = Tween::pulse(0.68, 1.0, time_ms, self.theme.motion.slow);
        // Bottom of what stands at the top right, which the kill feed goes under.
        let mut top_right = 0.0_f32;
        self.element_bounds.clear();
        for (index, (widget, rect)) in document.widgets.iter().zip(rectangles).enumerate() {
            if !visible[index] {
                continue;
            }
            let start = self.draw_list.len();
            let element = elements[index].or_else(|| {
                (widget.binding.as_deref() == Some("lagometer"))
                    .then_some(placement::Element::Lagometer)
            });
            widgets::emit(
                &mut self.draw_list,
                self.theme,
                widget,
                *rect,
                &widgets::EmitContext {
                    data: &data,
                    family: self.family,
                    targeting: self.targeting.policy,
                    viewport,
                    dpi_scale,
                    hero_scale,
                    low_health,
                    low_ammo,
                    pulse,
                    crosshair_alpha: self.crosshair_alpha,
                    crosshair_teammate: self.crosshair_teammate,
                    lagometer: &self.lagometer,
                    lagometer_readout: &self.lagometer_readout,
                },
                &mut output,
            );
            if let Some(element) = element {
                self.element_bounds
                    .add(element, self.draw_list.bounds_from(start));
            }
        }
        if visibility.elements {
            use placement::Element;
            let start = self.draw_list.len();
            jetpack_fuel::emit(&mut self.draw_list, viewport, user_scale, self.jetpack_fuel);
            self.place_from(Element::Jetpack, start, viewport);
            if visibility.vehicle && look == HudLook::Game {
                let start = self.draw_list.len();
                self.vehicle
                    .emit(&mut self.draw_list, self.theme, viewport, user_scale);
                self.place_from(Element::Vehicle, start, viewport);
            }
            let start = self.draw_list.len();
            self.speed.emit(&mut self.draw_list, self.theme, viewport);
            self.place_from(Element::Speedometer, start, viewport);
            let start = self.draw_list.len();
            self.movement_keys.emit(&mut self.draw_list, viewport);
            self.place_from(Element::MovementKeys, start, viewport);
            let start = self.draw_list.len();
            follow::emit(
                &mut self.draw_list,
                &self.followed_name,
                viewport,
                dpi_scale,
            );
            self.place_from(Element::Follow, start, viewport);
            let start = self.draw_list.len();
            let selector = self.selector.filter(|_| visibility.selection);
            match selector {
                Some(view) if !view.inventory && self.force_wheel_bar => force_wheel::emit(
                    &mut self.draw_list,
                    view,
                    &self.force_wheel_icons,
                    self.flamethrower_shown,
                    viewport,
                    user_scale,
                ),
                view => selection::emit(
                    &mut self.draw_list,
                    view,
                    self.flamethrower_shown,
                    self.theme,
                    viewport,
                    user_scale,
                ),
            }
            self.place_from(Element::Selection, start, viewport);
            top_right = top_right.max(self.emit_family(viewport));
            self.icons.flags_in_boxes = self.flag_status.flags_in_boxes(visibility);
            top_right = top_right.max(self.icons.emit(
                &mut self.draw_list,
                viewport,
                visibility,
                self.family.upper,
                self.weapon_select.shown.is_some(),
                self.weapon_alpha,
            ));
            let options = self.kill_feed.options();
            if options.fps || self.upper_right_timer {
                top_right = top_right.max(crate::console_overlay::upper_right_bottom(
                    viewport[1],
                    options.fps,
                    self.upper_right_timer,
                ));
            }
            let start = self.draw_list.len();
            self.kill_feed.emit(
                &mut self.draw_list,
                kill_feed::Area::below(viewport, hero_scale, dpi_scale, top_right, options),
                kill_feed::Art {
                    icons: &self.icons,
                    holocrons: &self.force_wheel_icons,
                },
                font,
                self.theme,
            );
            self.place_from(Element::KillFeed, start, viewport);
            let start = self.draw_list.len();
            self.flag_status
                .emit(&mut self.draw_list, viewport, &self.icons, visibility);
            self.place_from(Element::Flags, start, viewport);
            self.guides.emit(&mut self.draw_list, viewport, time_ms);
        }
        // CG_DrawWeaponSelect does not follow cg_drawHud: only the row's own
        // conditions (`GpuState::sample_weapon_select`) decide.
        if let Some(shown) = &self.weapon_select.shown {
            crate::weapon_select::emit_icons(&mut self.draw_list, &self.icons, shown, viewport);
        }
        output
    }

    /// Move what `element` drew from command `start` on where the player put it, when
    /// it keeps its place in [`placement::OFFSETS_CVAR`] (the others read their own
    /// cvars as they draw), and keep its box for the edit mode.
    fn place_from(&mut self, element: placement::Element, start: usize, viewport: [f32; 2]) {
        if element.binding() == placement::Binding::Free {
            let [dx, dy] = self.offsets.pixels(element, viewport);
            self.draw_list.translate_from(start, dx, dy);
        }
        self.element_bounds
            .add(element, self.draw_list.bounds_from(start));
    }

    /// Settle the status values and meters on a fixed state, for the layout snapshots
    /// and the HUD picker's pictures, with the weapon name showing as right after a
    /// weapon change.
    pub(crate) fn preview_values(&mut self, values: ClientHudData, ratios: [f32; 4]) {
        self.format_values(values);
        self.weapon_shown_ms = Some(0);
        self.ratios = [ratios[0], ratios[1], ratios[2]].map(Tween::settled);
        self.ammo_ratio = Tween::settled(ratios[3]);
    }

    /// Show the speedometer reading `text` with its right edge at `x` (640 units), as
    /// `cg_speedometer` does, for snapshots.
    #[cfg(test)]
    pub(crate) fn preview_speedometer(&mut self, text: &str, x: f32) {
        self.speed.preview(text, x);
    }

    pub(crate) fn displayed_ratios(&self) -> [f32; 3] {
        self.displayed_ratios
    }

    /// Append HUD text: what retail drew with a game font goes to that font when
    /// `ui_gameFont` has it loaded ([`text_values::retail_font`]), the rest to
    /// `vertices` with `font`.
    pub(crate) fn append(
        &self,
        fonts: &mut crate::game_font::GameFonts,
        vertices: &mut Vec<TextVertex>,
        font: &UiFont,
        viewport: [f32; 2],
    ) {
        fonts.append_routed(
            &self.draw_list,
            |id| self.resolve_text(id),
            |id, _| text_values::retail_font(id),
            |_| crate::text::CodePalette::Game,
            (vertices, font),
            viewport,
            crate::text::TextStyle::NEUTRAL,
            None,
        );
    }

    pub(crate) fn draw_list(&self) -> &DrawList {
        &self.draw_list
    }
}

#[cfg(test)]
mod tests {
    use super::{CLASSIC_LAYOUT, DEFAULT_LAYOUT};
    use sjk_ui::{Anchor, Dimension, HudLayoutDocument};

    /// Both layouts put the crosshair name's line top where stock's is:
    /// `CG_DrawCrosshairNames` draws it at y = 170 in the 480-line virtual
    /// screen, 157.5 px above the centre in the HUD's 1080-line frame.
    #[test]
    fn crosshair_name_line_top_matches_stock() {
        for layout in [DEFAULT_LAYOUT, CLASSIC_LAYOUT] {
            let document = HudLayoutDocument::from_json(layout).unwrap();
            let widget = document
                .widgets
                .iter()
                .find(|widget| widget.id == "crosshair_name")
                .unwrap();
            assert_eq!(widget.anchor, Anchor::Center);
            let Dimension::Px(height) = widget.size.height else {
                panic!("crosshair name height is not in px");
            };
            let top = 540.0 - height * 0.5 + widget.offset.y;
            assert!((top - 170.0 * 1_080.0 / 480.0).abs() < 1e-3, "{top}");
        }
    }

    /// The radial look's spacing moves the left meters left and the right ones right,
    /// its height moves all of them, and the rest of the HUD stays where it was.
    #[test]
    fn radial_placement_moves_only_the_status_meters() {
        let base = HudLayoutDocument::from_json(super::RADIAL_LAYOUT).unwrap();
        let placed = super::place_radial(
            &base,
            super::RadialPlacement {
                spread: 100.0,
                height: 10.0,
            },
        );
        let find = |document: &HudLayoutDocument, id: &str| {
            document
                .widgets
                .iter()
                .find(|widget| widget.id == id)
                .unwrap()
                .clone()
        };
        for (id, side) in [
            ("health_arc", -1.0),
            ("left_pill", -1.0),
            ("health_value", -1.0),
            ("force_arc", 1.0),
            ("ammo_value", 1.0),
            ("weapon_value", 0.0),
        ] {
            let (before, after) = (find(&base, id), find(&placed, id));
            assert_eq!(after.offset.x - before.offset.x, side * 100.0, "{id}");
            assert!((before.offset_fraction.y - after.offset_fraction.y - 0.1).abs() < 1e-6);
        }
        for id in ["crosshair", "match_timer", "lagometer"] {
            assert_eq!(find(&base, id), find(&placed, id), "{id}");
        }
        assert_eq!(
            super::place_radial(&base, super::RadialPlacement::default()),
            base
        );
        for id in super::RADIAL_STATUS {
            assert!(base.widgets.iter().any(|widget| widget.id == id), "{id}");
        }
    }
}
