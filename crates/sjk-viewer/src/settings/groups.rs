//! The classic Setup page's own groups. Retail's `setup.menu` and the settings
//! screen's tabs cut the settings differently, so classic+ regroups them by
//! what they are about (`docs/classic-plus.md`): gameplay options, the
//! menus and console, the HUD, the scoreboard. Each group lists its rows'
//! cvars; the settings come from the catalogue, so a group shows the same
//! rows the tabs do.

use super::catalog::*;
use std::sync::OnceLock;

/// A classic Setup group gathering rows of several settings tabs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Group {
    /// Gameplay: pickups, models, trails, remaps, camera and prediction.
    GameOptions,
    /// The menus' style, colours and fonts, and the console's look.
    Interface,
    /// The status HUD, crosshair and the readouts over the game.
    Hud,
    /// The scoreboard's style and columns.
    Scoreboard,
    /// The first-start settings ([`super::quick`]); gathers rows the other groups
    /// and tabs already hold, so it is not one of [`Group::ALL`].
    Quick,
    /// The renderer and its quality, then the renderer's four tabs (image,
    /// lighting, shadows, weather), as one group under their names, for the SJK
    /// UI's Graphics; the classic pages show each tab on its own, so it is not
    /// one of [`Group::ALL`].
    Graphics,
    /// The SJK UI's Display (Advanced): Video's rows but the renderer's, which
    /// Graphics leads with, under sub-headings. Not one of [`Group::ALL`]: the
    /// classic pages show the Video tab.
    Display,
    /// The SJK UI's Network (Advanced): the Network tab's rows under
    /// sub-headings (SJK's services, the connection), the SJK chat's left to
    /// Chat. Not one of [`Group::ALL`]: the classic pages show the tab.
    Network,
    /// Advanced's own regroupings of the classic groups and tabs, SJK UI only.
    Advanced(Advanced),
    /// Simple mode's categories ([`super::mode`]): the settings that matter
    /// most, a few to a category. SJK UI only.
    Simple(Simple),
}

/// Simple mode's categories, in the rail's order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Simple {
    Display,
    Graphics,
    Sound,
    Controls,
    Gameplay,
    Hud,
    General,
}

/// Advanced's categories that regroup the classic groups' rows by subject
/// (`docs/sjk-ui.md`, Settings): every sound in Sound, the chats in Chat, the
/// packs, mods and tools in Mods, so Gameplay, Interface and HUD keep what is
/// theirs (HUD with the scoreboard's rows).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Advanced {
    Sound,
    Gameplay,
    Interface,
    Hud,
    Chat,
    Mods,
}

impl Advanced {
    pub(crate) const ALL: [Self; 6] = [
        Self::Sound,
        Self::Gameplay,
        Self::Interface,
        Self::Hud,
        Self::Chat,
        Self::Mods,
    ];
}

/// The rows Chat gathers: the in-game chat box's (HUD's in the classic
/// groups, the code page of other clients' text Interface's) and the SJK chat's
/// (Network's).
const CHAT_ROWS: &[&str] = &[
    "cg_drawChat",
    "cl_unfocusedChatbox",
    crate::status_balloons::CVAR,
    crate::console::chat_log::CVAR,
    crate::chat::pm_only::CVAR,
    crate::chat::quiet::CVAR,
    "cg_chatBoxFontSize",
    crate::text::system_font::CHAT_CVAR,
    crate::chat::emoji::CVAR,
    crate::chat::reply::REMEMBER_CVAR,
    crate::chat::tabs::FILTER_CVAR,
    crate::chat::tabs::TEAM_CVAR,
    crate::chat::tabs::SJK_CVAR,
    crate::chat::tabs::JOF_CVAR,
    crate::chat::tabs::SITH_CVAR,
    crate::chat::tabs::ADMIN_CVAR,
    crate::chat::tabs::PM_CVAR,
    crate::chat::options::LINE_SPACING_CVAR,
    "cg_chatBoxLetterSpacing",
    crate::chat::options::BACKDROP_CVAR,
    crate::chat::options::OUTLINE_CVAR,
    crate::text::DARK_OUTLINE_CVAR,
    crate::chat::incoming_color::CVAR,
    crate::chat::options::CARD_SCALE_CVAR,
    crate::text::charset::CVAR,
    "cl_sjkChat",
    crate::sjk_chat_frame::FEED_CVAR,
    crate::sjk_chat_frame::CONSOLE_CVAR,
    crate::chat::tabs::JOF_JOIN_CVAR,
    crate::chat::tabs::SITH_JOIN_CVAR,
    crate::chat_gifs::CVAR,
    crate::chat_gifs::VIDEO_CVAR,
    crate::chat::client_icons::CVAR,
    crate::chat_gifs::search::KEY_CVAR,
    crate::e2e_whispers::CVAR,
    crate::map_blocks::CVAR,
    crate::sound_mutes::IGNORE_CVAR,
];

/// The first row of the JA+ plugin options, under their heading.
const JAPLUS_PLUGIN_ROW: &str = "japlus.plugin 1";
/// The first row of the jaPRO plugin options, under their heading.
const JAPRO_PLUGIN_ROW: &str = "japro.plugin 0";

/// The rows Mods gathers from the classic Game options.
const MODS_ROWS: &[&str] = &[
    "assetbrowser",
    "mod_japlus",
    #[cfg(feature = "mod-japlus")]
    sjk_mod_japlus::anti_kiss::CVAR,
    "mod_japro",
    #[cfg(feature = "mod-japro")]
    sjk_mod_japro::movement_styles::PREDICT_ANYWAY_CVAR,
    "mod_jof",
    #[cfg(feature = "mod-jof")]
    sjk_mod_jof::destruction::CVAR,
    #[cfg(feature = "mod-jof")]
    sjk_mod_jof::dialogue::CVAR,
    #[cfg(feature = "mod-jof")]
    sjk_mod_jof::EJK_OPT_IN,
    #[cfg(feature = "mod-jof")]
    sjk_mod_jof::ABILITY_SOUNDS,
    #[cfg(feature = "mod-jof")]
    sjk_mod_jof::HONOR_NAMES,
    #[cfg(feature = "mod-japlus")]
    "japlus.plugin 1",
    #[cfg(feature = "mod-japlus")]
    "japlus.plugin 3",
    #[cfg(feature = "mod-japlus")]
    "japlus.plugin 5",
    #[cfg(feature = "mod-japlus")]
    "japlus.plugin 9",
    #[cfg(feature = "mod-japlus")]
    "japlus.plugin 15",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 0",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 1",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 5",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 6",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 7",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 8",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 9",
    #[cfg(feature = "mod-japro")]
    "japro.plugin 10",
    "jof.downloads",
    "jof.contentAuto",
    "jof.contentSource",
    "sith.downloads",
    "sith.cloudAuto",
    crate::brush_view::CVAR,
    crate::entity_view::CVAR,
    crate::ghost_replay::SHOWCASE_CVAR,
];

/// The sounds Sound gathers from Network and Interface after the Sound tab's.
const NOTIFICATION_SOUNDS: &[&str] = &[
    crate::world_ping::SOUND_CVAR,
    crate::quick_wheel::SOUNDS_CVAR,
];

/// The chat sounds' rows (`chat_sounds.rs`), which Sound shows last under their
/// heading: the master switch, then each kind's switch, sound and preview, then the
/// private messages' taskbar flash.
const CHAT_SOUND_ROWS: &[&str] = {
    use crate::chat_sounds::Kind;
    &[
        crate::chat_sounds::MASTER_CVAR,
        Kind::Public.switch_cvar(),
        Kind::Public.sound_cvar(),
        Kind::Public.preview_row(),
        Kind::Team.switch_cvar(),
        Kind::Team.sound_cvar(),
        Kind::Team.preview_row(),
        Kind::Private.switch_cvar(),
        Kind::Private.sound_cvar(),
        Kind::Private.preview_row(),
        Kind::Sjk.switch_cvar(),
        Kind::Sjk.sound_cvar(),
        Kind::Sjk.preview_row(),
        crate::chat_sounds::FLASH_CVAR,
    ]
};

/// The quick wheel's rows of the classic Interface, which Advanced's Quick
/// wheel category and Sound hold.
const WHEEL_ROWS: &[&str] = &[
    crate::quick_wheel::pages::FILE,
    crate::quick_wheel::SOUNDS_CVAR,
];

impl Simple {
    pub(crate) const ALL: [Self; 7] = [
        Self::Display,
        Self::Graphics,
        Self::Sound,
        Self::Controls,
        Self::Gameplay,
        Self::Hud,
        Self::General,
    ];
}

/// The rows Graphics leads with, before the renderer's tabs: the graphics
/// preset and the two effects the original game had (impact marks, player
/// shadows).
const GRAPHICS_LEAD: &[&str] = &[
    crate::graphics_quality::ROW_NAME,
    crate::graphics_quality::RECOMMEND_COMMAND,
    "cg_marks",
    "cg_shadows",
];

/// Headings inside the renderer tabs on the Graphics page, each over the row it names
/// (the tabs' own names head their first rows).
const GRAPHICS_SUBHEADINGS: &[(&str, &str)] = &[
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "Textures",
    ),
    ("r_softParticles", "Effects"),
    ("r_normalMapping", "Materials"),
    ("r_volumetrics", "Light shafts"),
    (
        crate::world_materials::shadows::day::live::SKY_CVAR,
        "Time of day",
    ),
    (crate::weather::REFLECTIONS_CVAR, "Rain looks"),
];

/// The renderer tabs [`Group::Graphics`] gathers after [`GRAPHICS_LEAD`], each
/// under its heading.
fn graphics_tabs() -> [(&'static str, &'static [Setting]); 4] {
    [
        ("Image", RENDER_IMAGE),
        ("Solar Renderer lighting", RENDER_LIGHTING),
        ("Shadows", RENDER_SHADOWS),
        ("Weather", RENDER_WEATHER),
    ]
}

/// The heading over [`GRAPHICS_LEAD`].
const GRAPHICS_LEAD_HEADING: &str = "Quality";

/// The catalogue's row of setting `cvar`, on any tab, the renderer's included.
fn find(cvar: &str) -> Option<Setting> {
    (0..TABS.len())
        .flat_map(super::settings)
        .chain(
            [
                RENDER_IMAGE,
                RENDER_LIGHTING,
                RENDER_SHADOWS,
                RENDER_WEATHER,
            ]
            .into_iter()
            .flatten(),
        )
        .find(|setting| setting.cvar.eq_ignore_ascii_case(cvar))
        .copied()
}

impl Group {
    /// The classic Setup page's groups.
    #[cfg(test)]
    pub(crate) const ALL: [Self; 4] = [
        Self::GameOptions,
        Self::Interface,
        Self::Hud,
        Self::Scoreboard,
    ];

    /// How many groups there are, Simple's and Advanced's included.
    const COUNT: usize = 8 + Simple::ALL.len() + Advanced::ALL.len();

    /// The group's name, as the one tab of the tabbed screen showing it.
    const CAPTIONS: [&'static str; Self::COUNT] = [
        "GAME OPTIONS",
        "INTERFACE",
        "HUD",
        "SCOREBOARD",
        super::catalog::FIRST_SETUP_CAPTION,
        "GRAPHICS",
        "DISPLAY",
        "NETWORK",
        "DISPLAY",
        "GRAPHICS",
        "SOUND",
        "CONTROLS",
        "GAMEPLAY",
        "HUD",
        "GENERAL",
        "SOUND",
        "GAMEPLAY",
        "INTERFACE",
        "HUD",
        "CHAT",
        "MODS",
    ];

    fn index(self) -> usize {
        match self {
            Self::GameOptions => 0,
            Self::Interface => 1,
            Self::Hud => 2,
            Self::Scoreboard => 3,
            Self::Quick => 4,
            Self::Graphics => 5,
            Self::Display => 6,
            Self::Network => 7,
            Self::Simple(simple) => 8 + simple as usize,
            Self::Advanced(advanced) => 8 + Simple::ALL.len() + advanced as usize,
        }
    }

    /// The cvars of the group's rows, in order: [`Self::cvars`], or for one of
    /// Advanced's, the classic groups' rows it takes.
    fn cvar_list(self) -> Vec<&'static str> {
        let without = |group: Self, left: &[&str]| -> Vec<&'static str> {
            group
                .cvars()
                .iter()
                .copied()
                .filter(|cvar| !left.contains(cvar))
                .collect()
        };
        match self {
            Self::Advanced(Advanced::Gameplay) => without(Self::GameOptions, MODS_ROWS),
            Self::Advanced(Advanced::Interface) => without(Self::Interface, WHEEL_ROWS)
                .into_iter()
                .filter(|cvar| !CHAT_ROWS.contains(cvar))
                .collect(),
            Self::Advanced(Advanced::Hud) => {
                let mut cvars: Vec<&str> = without(Self::Hud, CHAT_ROWS)
                    .into_iter()
                    .filter(|cvar| !NOTIFICATION_SOUNDS.contains(cvar))
                    .collect();
                cvars.extend_from_slice(Self::Scoreboard.cvars());
                cvars
            }
            Self::Advanced(Advanced::Sound) => {
                let mut cvars: Vec<&str> = AUDIO
                    .iter()
                    .map(|setting| setting.cvar)
                    .filter(|cvar| !CHAT_SOUND_ROWS.contains(cvar))
                    .collect();
                cvars.extend_from_slice(NOTIFICATION_SOUNDS);
                cvars.extend_from_slice(CHAT_SOUND_ROWS);
                cvars
            }
            _ => self.cvars().to_vec(),
        }
    }

    /// The tab list of a screen showing the group: its name alone.
    pub(super) fn tabs(self) -> &'static [&'static str] {
        std::slice::from_ref(&Self::CAPTIONS[self.index()])
    }

    /// The cvars of the group's rows, in order.
    fn cvars(self) -> &'static [&'static str] {
        match self {
            Self::GameOptions => &[
                "assetbrowser",
                "cg_simpleItems",
                "cg_forceModel",
                sjk_client::player_model_eligibility::METADATA_CVAR,
                sjk_client::player_model_eligibility::BLACKLIST_CVAR,
                "cg_saberTrail",
                crate::effect_flash::REDUCE_CVAR,
                crate::saber_clash_flare::CVAR,
                crate::saber_clash_flare::SIZE_CVAR,
                crate::saber_core::CVAR,
                crate::impacts::FLESH_SPARKS_CVAR,
                crate::team_sabers::TEAM_COLORS_CVAR,
                crate::team_sabers::CVAR,
                crate::saber_rgb::black::CVAR,
                crate::blade_lab_designs::CVAR,
                crate::duel_blades::CVAR,
                crate::stock_blades::CVAR,
                crate::effect_aux::saber_rain_steam::CVAR,
                "cg_speedTrail",
                crate::actor_world_submission::force_powers::victim::CVAR,
                crate::actor_world_submission::force_powers::flame::CVAR,
                crate::holstered_sabers::CVAR,
                crate::holstered_sabers::SUPPORT_CVAR,
                crate::holstered_sabers::SWAP_CVAR,
                "cg_holsteredStaffSound",
                "cg_auraShell",
                crate::actor_world_submission::force_powers::policy::ANIMATION_CVAR,
                crate::actor_world_submission::force_powers::policy::EMPOWER_CVAR,
                "cg_spProtAbsColor",
                "cg_shieldSphere",
                "cg_shieldBrightness",
                crate::duel_ghost::CVAR,
                crate::duel_visibility::HIDE_OTHERS_CVAR,
                crate::duel_visibility::CORPSES_CVAR,
                crate::duel_visibility::PICKUPS_CVAR,
                crate::map_edits::CVAR,
                crate::map_edits::SYNC_COMMAND,
                crate::remap_blocked_maps::SERVER_CVAR,
                crate::remap_blocked_maps::SHARED_CVAR,
                crate::remap_blocked_maps::MAP_CVAR,
                "cg_remaps",
                crate::monitor::SHOW_CVAR,
                crate::banana::SHOW_CVAR,
                crate::tags::SHOW_CVAR,
                crate::tags::GIF_SHOW_CVAR,
                crate::toy_sight::ILLUMINATE,
                crate::toy_sight::CARDBOARD_BOX,
                crate::toy_sight::RAGDOLL,
                crate::toy_sight::DRONE,
                crate::toy_sight::BOOMBOX,
                crate::boombox::RANGE_CVAR,
                crate::camera::STYLE_CVAR,
                "cg_drawGun",
                "cg_fovViewmodel",
                "cg_cameraFPS",
                "cg_thirdPersonRange",
                "cg_thirdPersonAngle",
                "cg_thirdPersonPitchOffset",
                "cg_thirdPersonVertOffset",
                "cg_thirdPersonHorzOffset",
                "cg_thirdPersonCameraDamp",
                "cg_thirdPersonTargetDamp",
                "cg_wallGrabLock",
                crate::held_view::LOOK_CVAR,
                "cg_predictLegacy",
                "cg_errorDecay",
                "mod_japlus",
                #[cfg(feature = "mod-japlus")]
                sjk_mod_japlus::anti_kiss::CVAR,
                "mod_japro",
                #[cfg(feature = "mod-japro")]
                sjk_mod_japro::movement_styles::PREDICT_ANYWAY_CVAR,
                "mod_jof",
                #[cfg(feature = "mod-jof")]
                sjk_mod_jof::destruction::CVAR,
                #[cfg(feature = "mod-jof")]
                sjk_mod_jof::dialogue::CVAR,
                #[cfg(feature = "mod-jof")]
                sjk_mod_jof::EJK_OPT_IN,
                #[cfg(feature = "mod-jof")]
                sjk_mod_jof::ABILITY_SOUNDS,
                #[cfg(feature = "mod-jof")]
                sjk_mod_jof::HONOR_NAMES,
                "jof.downloads",
                "jof.contentAuto",
                "jof.contentSource",
                "sith.downloads",
                "sith.cloudAuto",
                #[cfg(feature = "mod-japlus")]
                "japlus.plugin 1",
                #[cfg(feature = "mod-japlus")]
                "japlus.plugin 3",
                #[cfg(feature = "mod-japlus")]
                "japlus.plugin 5",
                #[cfg(feature = "mod-japlus")]
                "japlus.plugin 9",
                #[cfg(feature = "mod-japlus")]
                "japlus.plugin 15",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 0",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 1",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 5",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 6",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 7",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 8",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 9",
                #[cfg(feature = "mod-japro")]
                "japro.plugin 10",
                crate::brush_view::CVAR,
                crate::entity_view::CVAR,
                crate::ghost_replay::SHOWCASE_CVAR,
            ],
            Self::Interface => &[
                crate::menu::style::CVAR,
                crate::launcher_notice::CVAR,
                crate::menu_pictures::CVAR,
                super::slider_peek::CVAR,
                crate::rarity_fx::CVAR,
                crate::game_font::CVAR,
                crate::text::style::SCALE_CVAR,
                crate::text::style::TRACKING_CVAR,
                crate::console::console_options::STYLE_CVAR,
                crate::text::system_font::CONSOLE_CVAR,
                "con_scale",
                crate::console::console_options::DRAW_NOTIFY_CVAR,
                crate::console::console_options::CHAT_ONLY_CVAR,
                crate::console::console_options::COPY_COLORS_CVAR,
                crate::console::console_options::SHIFT_HEIGHT_CVAR,
                crate::console::console_options::OVER_CHAT_CVAR,
                crate::text::charset::CVAR,
                crate::quick_wheel::pages::FILE,
                crate::quick_wheel::SOUNDS_CVAR,
                crate::player_trades::SIZE_CVAR,
            ],
            Self::Hud => &[
                "cg_drawHud",
                crate::hud::toggle::ELEMENTS_CVAR,
                crate::hud::toggle::CROSSHAIR_CVAR,
                crate::hud::toggle::NAMES_CVAR,
                crate::hud::toggle::CHAT_CVAR,
                crate::menu_hud::STYLE_CVAR,
                crate::menu_hud::FILES_CVAR,
                "cg_hudScale",
                crate::hud_edit::COMMAND,
                crate::hud::RadialPlacement::SPREAD_CVAR,
                crate::hud::RadialPlacement::HEIGHT_CVAR,
                "cg_classicHudFont",
                "cg_drawStatus",
                "cg_drawWeapon",
                "cg_drawInventory",
                "cg_crosshair",
                "cg_crosshairSize",
                "cg_hitMarker",
                crate::damage_feedback::SHIELD_CVAR,
                "cg_drawCrosshairNames",
                "cg_drawPlayerNames",
                "cg_drawPlayerNamesScale",
                "cg_drawFriend",
                crate::hud::player_card::CVAR,
                "cg_playerCardDelay",
                "cg_playerCardOutline",
                crate::hud::team_outline::CVAR,
                "cg_nameplate",
                crate::hud::nameplate::STYLE_CVAR,
                "cg_nameplateRange",
                "cg_nameplateNear",
                "cg_nameplateScale",
                "cg_nameplateOffset",
                crate::hud::nameplate::OPACITY_CVAR,
                "cg_nameplateBars",
                "cg_nameplateSelf",
                "cg_nameplateHideDuelers",
                "cg_nameplateIcons",
                "cg_nameplateDuel",
                "cg_nameplateOutline",
                "cg_nameplateWalls",
                "cg_nameplateNpcs",
                crate::world_ping::SHOW_CVAR,
                crate::world_ping::SOUND_CVAR,
                "cg_drawFps",
                crate::frame_timings::DRAW_CVAR,
                crate::frame_timings::LOG_CVAR,
                "cg_drawTimer",
                "cg_drawTimerUpperRight",
                "cg_drawTeamScores",
                "cg_teamScoresBoxes",
                "cg_teamScoresScale",
                "cg_drawFlagStatus",
                crate::version_overlay::CVAR,
                "cg_speedometer",
                crate::hud::clock::CVAR,
                crate::hud::clock::HOURS_CVAR,
                crate::hud::movement_keys::CVAR,
                crate::hud::movement::LINES_ROW,
                crate::hud::movement::AURA_CVAR,
                crate::hud::movement::AURA_GROUND_CVAR,
                crate::hud::lagometer::CVAR,
                "cg_lagometerScale",
                "cg_killfeed",
                crate::ground_hud::CVAR,
                "cg_drawChat",
                "cl_unfocusedChatbox",
                crate::status_balloons::CVAR,
                crate::chat::pm_only::CVAR,
                crate::console::chat_log::CVAR,
                crate::chat::quiet::CVAR,
                "cg_chatBoxFontSize",
                crate::text::system_font::CHAT_CVAR,
                crate::chat::emoji::CVAR,
                crate::chat::reply::REMEMBER_CVAR,
                crate::chat::tabs::FILTER_CVAR,
                crate::chat::tabs::TEAM_CVAR,
                crate::chat::tabs::SJK_CVAR,
                crate::chat::tabs::JOF_CVAR,
                crate::chat::tabs::SITH_CVAR,
                crate::chat::tabs::ADMIN_CVAR,
                crate::chat::tabs::PM_CVAR,
                crate::chat::options::LINE_SPACING_CVAR,
                "cg_chatBoxLetterSpacing",
                crate::chat::options::BACKDROP_CVAR,
                crate::chat::options::OUTLINE_CVAR,
                crate::text::DARK_OUTLINE_CVAR,
                crate::chat::incoming_color::CVAR,
                crate::chat::options::CARD_SCALE_CVAR,
            ],
            Self::Scoreboard => &[
                crate::scoreboard::style::CVAR,
                crate::scoreboard::style::COMPACT_CVAR,
                crate::scoreboard::style::FPS_CVAR,
                crate::scoreboard::AFK_CVAR,
                crate::scoreboard::style::IDS_CVAR,
                crate::scoreboard::style::SJK_IDS_CVAR,
                crate::scoreboard::style::DEATHS_RANK_CVAR,
                crate::jof_tag::CVAR,
                "cg_drawScoreboardIcons",
                "cg_smallScoreboard",
            ],
            Self::Display => &[
                "r_resolution",
                super::display::HIGH_DPI_CVAR,
                "r_fullscreen",
                "r_vsync",
                "com_maxfps",
                crate::runtime_settings::MENU_CAP_CVAR,
                crate::gameplay_fps::CVAR,
                crate::server_caps::OVERRIDE_CVAR,
                crate::server_caps::FRESH_STAMPS_CVAR,
                crate::runtime_settings::MONITOR_CAP_CVAR,
                "cg_fov",
                "r_gamma",
            ],
            Self::Network => &[
                "cl_autoUpdate",
                "cl_identity",
                super::catalog::IDENTITY_ROW,
                crate::installed_models::CVAR,
                "cl_hubUrl",
                crate::holocron_popup::NOTIFICATIONS_CVAR,
                crate::crash_report::CVAR,
                crate::settings_usage::CVAR,
                crate::discord_presence::CVAR,
                "rate",
                "snaps",
                "cl_master",
                "cl_master2",
            ],
            Self::Simple(Simple::Display) => &[
                "r_resolution",
                crate::frame_target::scale::CVAR,
                super::display::HIGH_DPI_CVAR,
                "r_fullscreen",
                "r_vsync",
                "com_maxfps",
                "cg_fov",
                "r_gamma",
            ],
            Self::Simple(Simple::Graphics) => &[
                crate::graphics_quality::ROW_NAME,
                crate::graphics_quality::RECOMMEND_COMMAND,
                crate::weather::CVAR,
                "r_DynamicGlow",
                "cg_marks",
                "cg_shadows",
            ],
            Self::Simple(Simple::Sound) => &[
                "s_volume",
                "s_musicVolume",
                crate::audio::VOICE_VOLUME_CVAR,
                "snd_mute_losefocus",
            ],
            Self::Simple(Simple::Controls) => &["sensitivity", "m_invert", "cl_run"],
            Self::Simple(Simple::Gameplay) => &[
                crate::camera::STYLE_CVAR,
                crate::gameplay_fps::CVAR,
                "cg_drawGun",
                "cg_saberTrail",
                crate::effect_flash::REDUCE_CVAR,
                crate::duel_blades::CVAR,
                "cg_forceModel",
                "cg_auraShell",
                crate::away_emote::SJK_CVAR,
                crate::away_emote::JAPLUS_CVAR,
            ],
            Self::Simple(Simple::Hud) => &[
                crate::menu_hud::STYLE_CVAR,
                "cg_hudScale",
                "cg_crosshair",
                "cg_crosshairSize",
                "cg_hitMarker",
                "cg_nameplate",
                "cg_drawFriend",
                crate::hud::team_outline::CVAR,
                "cg_drawFps",
                "cg_speedometer",
                "cg_killfeed",
                "cg_drawChat",
                "cg_chatBoxFontSize",
            ],
            Self::Simple(Simple::General) => &[
                crate::menu::style::CVAR,
                crate::text::style::SCALE_CVAR,
                crate::player_trades::SIZE_CVAR,
                super::catalog::IMPORT_ROW,
                "jof.downloads",
                "jof.contentAuto",
                "sith.downloads",
                "sith.cloudAuto",
                "cl_autoUpdate",
                "cl_sjkChat",
                crate::holocron_popup::NOTIFICATIONS_CVAR,
                crate::crash_report::CVAR,
                crate::settings_usage::CVAR,
                crate::discord_presence::CVAR,
                super::catalog::IDENTITY_ROW,
            ],
            Self::Advanced(Advanced::Chat) => CHAT_ROWS,
            Self::Advanced(Advanced::Mods) => MODS_ROWS,
            // Their rows are taken from the classic groups ([`Self::cvar_list`]).
            Self::Advanced(_) | Self::Quick | Self::Graphics => &[],
        }
    }

    /// The sub-headings that divide the group's rows: each heading stands
    /// before the row of its cvar.
    fn headings(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::GameOptions => &[
                ("assetbrowser", "Asset packs"),
                ("cg_simpleItems", "Items and models"),
                ("cg_saberTrail", "Effects"),
                (crate::monitor::SHOW_CVAR, "Toys"),
                (crate::camera::STYLE_CVAR, "Camera and prediction"),
                (JAPLUS_PLUGIN_ROW, "JA+ plugin options"),
                (JAPRO_PLUGIN_ROW, "jaPRO plugin options"),
                (crate::brush_view::CVAR, "Map tools"),
            ],
            Self::Interface => &[
                (crate::menu::style::CVAR, "Menus"),
                (crate::console::console_options::STYLE_CVAR, "Console"),
                (crate::quick_wheel::pages::FILE, "Quick wheel"),
            ],
            Self::Hud => &[
                ("cg_drawHud", "Layout"),
                ("cg_crosshair", "Crosshair"),
                ("cg_drawCrosshairNames", "Names and cards"),
                ("cg_nameplate", "Nameplates"),
                (crate::world_ping::SHOW_CVAR, "World pings"),
                ("cg_drawFps", "Readouts"),
                ("cg_drawChat", "Chat"),
            ],
            Self::Display => &[
                ("r_resolution", "Screen"),
                ("com_maxfps", "Frame rate"),
                ("cg_fov", "View"),
            ],
            Self::Network => &[("cl_autoUpdate", "SJK online"), ("rate", "Connection")],
            Self::Advanced(Advanced::Sound) => &[
                ("s_volume", "Game"),
                (crate::emotes::VOLUME_CVAR, "Emotes"),
                (crate::toy_sounds::VOLUME_CVAR, "Toys"),
                (crate::unlock_toast::SOUND_CVAR, "Notifications"),
                (crate::chat_sounds::MASTER_CVAR, "Chat sounds"),
            ],
            Self::Advanced(Advanced::Gameplay) => &[
                ("cg_simpleItems", "Items and models"),
                ("cg_saberTrail", "Effects"),
                (crate::monitor::SHOW_CVAR, "Toys"),
                (crate::camera::STYLE_CVAR, "Camera and prediction"),
            ],
            Self::Advanced(Advanced::Interface) => &[
                (crate::menu::style::CVAR, "Menus"),
                (crate::console::console_options::STYLE_CVAR, "Console"),
            ],
            Self::Advanced(Advanced::Hud) => &[
                ("cg_drawHud", "Layout"),
                ("cg_crosshair", "Crosshair"),
                ("cg_drawCrosshairNames", "Names and cards"),
                ("cg_nameplate", "Nameplates"),
                (crate::world_ping::SHOW_CVAR, "World pings"),
                ("cg_drawFps", "Readouts"),
                (crate::scoreboard::style::CVAR, "Scoreboard"),
            ],
            Self::Advanced(Advanced::Chat) => {
                &[("cg_drawChat", "Chat box"), ("cl_sjkChat", "SJK chat")]
            }
            Self::Advanced(Advanced::Mods) => &[
                ("assetbrowser", "Packs"),
                ("mod_japlus", "Server mods"),
                (JAPLUS_PLUGIN_ROW, "JA+ plugin options"),
                (JAPRO_PLUGIN_ROW, "jaPRO plugin options"),
                ("jof.downloads", "JoF content"),
                ("sith.downloads", "Sith cloud"),
                (crate::brush_view::CVAR, "Map tools"),
                (crate::ghost_replay::SHOWCASE_CVAR, "Experimental"),
            ],
            Self::Simple(Simple::Graphics) => &[
                (crate::graphics_quality::ROW_NAME, "Preset"),
                (crate::weather::CVAR, "Effects"),
            ],
            Self::Simple(Simple::Hud) => &[
                (crate::menu_hud::STYLE_CVAR, "HUD"),
                ("cg_crosshair", "Crosshair"),
                ("cg_nameplate", "Over the game"),
            ],
            Self::Simple(Simple::General) => &[
                (crate::menu::style::CVAR, "Menus"),
                (super::catalog::IMPORT_ROW, "From another client"),
                ("jof.downloads", "JoF content"),
                ("sith.downloads", "Sith cloud"),
                ("cl_autoUpdate", "SJK online"),
            ],
            // Its headings are its tabs' names ([`graphics_tabs`]).
            Self::Scoreboard | Self::Graphics | Self::Simple(_) => &[],
            Self::Quick => &[
                (super::catalog::IMPORT_ROW, "From another client"),
                (crate::graphics_quality::ROW_NAME, "Graphics"),
                (crate::menu::style::CVAR, "Styles"),
                ("r_resolution", "Display"),
                ("sensitivity", "Aim"),
                ("s_volume", "Sound"),
                (crate::menu_hud::STYLE_CVAR, "HUD"),
                ("cg_nameplate", "Nameplates"),
                ("cg_auraShell", "Force shells"),
                ("cl_autoUpdate", "SJK"),
            ],
        }
    }

    /// The lines a classic panel shows for the group: its rows, under their
    /// sub-headings.
    pub(super) fn lines(self) -> Vec<super::Line> {
        if self == Self::Graphics {
            let mut lines = Vec::with_capacity(self.rows().len() + 5 + GRAPHICS_SUBHEADINGS.len());
            lines.push(super::Line::Heading(GRAPHICS_LEAD_HEADING));
            lines.extend((0..GRAPHICS_LEAD.len()).map(super::Line::Row));
            let mut row = GRAPHICS_LEAD.len();
            for (heading, tab) in graphics_tabs() {
                lines.push(super::Line::Heading(heading));
                for setting in tab {
                    if let Some((_, sub)) = GRAPHICS_SUBHEADINGS
                        .iter()
                        .find(|(cvar, _)| cvar.eq_ignore_ascii_case(setting.cvar))
                    {
                        lines.push(super::Line::Heading(sub));
                    }
                    lines.push(super::Line::Row(row));
                    row += 1;
                }
            }
            return lines;
        }
        let rows = self.rows();
        let mut lines = Vec::with_capacity(rows.len() + self.headings().len());
        for (row, setting) in rows.iter().enumerate() {
            if let Some((_, heading)) = self
                .headings()
                .iter()
                .find(|(cvar, _)| cvar.eq_ignore_ascii_case(setting.cvar))
            {
                lines.push(super::Line::Heading(heading));
            }
            lines.push(super::Line::Row(row));
        }
        lines
    }

    /// The group's settings, looked up in the catalogue once.
    pub(super) fn rows(self) -> &'static [Setting] {
        if self == Self::Quick {
            return super::quick::rows();
        }
        static ROWS: [OnceLock<Vec<Setting>>; Group::COUNT] =
            [const { OnceLock::new() }; Group::COUNT];
        ROWS[self.index()].get_or_init(|| {
            if self == Self::Graphics {
                return GRAPHICS_LEAD
                    .iter()
                    .filter_map(|cvar| find(cvar))
                    .chain(
                        graphics_tabs()
                            .iter()
                            .flat_map(|(_, tab)| tab.iter().copied()),
                    )
                    .collect();
            }
            self.cvar_list()
                .iter()
                .filter_map(|cvar| find(cvar))
                .collect()
        })
    }

    /// Every group of rows the SJK UI's Settings shows, Simple's and the
    /// Advanced ones that are not one of [`Group::ALL`].
    #[cfg(test)]
    pub(super) fn sjk_only() -> impl Iterator<Item = Self> {
        [Self::Graphics, Self::Display, Self::Network]
            .into_iter()
            .chain(Simple::ALL.map(Self::Simple))
            .chain(Advanced::ALL.map(Self::Advanced))
    }

    /// The groups of rows Advanced's rail shows, First setup's aside.
    #[cfg(test)]
    pub(super) fn advanced_rail() -> [Self; 9] {
        [
            Self::Display,
            Self::Graphics,
            Self::Advanced(Advanced::Sound),
            Self::Advanced(Advanced::Gameplay),
            Self::Advanced(Advanced::Interface),
            Self::Advanced(Advanced::Hud),
            Self::Advanced(Advanced::Chat),
            Self::Advanced(Advanced::Mods),
            Self::Network,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_cvar_is_a_setting() {
        for group in Group::ALL
            .into_iter()
            .chain(Group::sjk_only())
            .filter(|group| *group != Group::Graphics)
        {
            assert_eq!(group.rows().len(), group.cvar_list().len(), "{group:?}");
        }
    }

    #[test]
    fn display_and_network_hold_their_tabs_rows_but_the_renderers() {
        let tab = |caption: &str| {
            super::super::settings(TABS.iter().position(|tab| *tab == caption).unwrap())
        };
        let shown = |group: Group| -> Vec<&str> {
            group.rows().iter().map(|setting| setting.cvar).collect()
        };
        let display = shown(Group::Display);
        for setting in tab("VIDEO") {
            let in_graphics = GRAPHICS_LEAD.contains(&setting.cvar);
            assert_eq!(
                display.contains(&setting.cvar),
                !in_graphics,
                "{}",
                setting.cvar
            );
        }
        // Network: the tab's rows, but the SJK chat's (Chat) and its sound (Sound).
        let network = shown(Group::Network);
        let mut expected: Vec<&str> = tab("NETWORK")
            .iter()
            .map(|setting| setting.cvar)
            .filter(|cvar| !CHAT_ROWS.contains(cvar) && !NOTIFICATION_SOUNDS.contains(cvar))
            .collect();
        let mut found = network.clone();
        expected.sort_unstable();
        found.sort_unstable();
        assert_eq!(found, expected, "Network holds the tab's rows, once each");
    }

    /// Every setting is on exactly one of Advanced's categories (First setup's
    /// rows are its others'): the groups of rows, the Controls tab, and the
    /// quick wheel's pages, which its own category edits.
    #[test]
    fn advanced_shows_every_setting_once() {
        let mut shown: Vec<&str> = Group::advanced_rail()
            .iter()
            .flat_map(|group| group.rows())
            .map(|setting| setting.cvar)
            .collect();
        let controls =
            super::super::settings(TABS.iter().position(|tab| *tab == "CONTROLS").unwrap());
        shown.extend(controls.iter().map(|setting| setting.cvar));
        shown.push(crate::quick_wheel::pages::FILE);
        let every = (0..TABS.len())
            .filter(|tab| *tab != QUICK_TAB)
            .flat_map(super::super::settings)
            .chain(
                [
                    RENDER_IMAGE,
                    RENDER_LIGHTING,
                    RENDER_SHADOWS,
                    RENDER_WEATHER,
                ]
                .into_iter()
                .flatten(),
            );
        for setting in every {
            let count = shown.iter().filter(|cvar| **cvar == setting.cvar).count();
            assert_eq!(count, 1, "{} is shown {count} times", setting.cvar);
        }
        // Only First setup's own rows are First setup's alone.
        for setting in Group::Quick.rows() {
            let own = [
                super::super::catalog::IMPORT_ROW,
                super::super::quick::HIDE_CVAR,
            ];
            assert!(
                own.contains(&setting.cvar) || shown.contains(&setting.cvar),
                "{} is only in First setup",
                setting.cvar
            );
        }
    }

    #[test]
    fn simple_mode_keeps_its_categories_short() {
        for simple in Simple::ALL {
            let rows = Group::Simple(simple).rows();
            // General holds the two content collections' automatic updates too.
            assert!(
                (3..=15).contains(&rows.len()),
                "{simple:?}: {} rows",
                rows.len()
            );
        }
        // The preset first: the choice that matters most.
        assert_eq!(
            Group::Simple(Simple::Graphics).rows()[0].cvar,
            crate::graphics_quality::ROW_NAME
        );
    }

    #[test]
    fn chat_text_size_is_offered_in_every_settings_mode() {
        let has_size = |group: Group| {
            group
                .rows()
                .iter()
                .any(|setting| setting.cvar.eq_ignore_ascii_case("cg_chatBoxFontSize"))
        };
        assert!(has_size(Group::Hud), "classic+ HUD");
        assert!(has_size(Group::Advanced(Advanced::Chat)), "SJK Advanced");
        assert!(has_size(Group::Simple(Simple::Hud)), "SJK Simple");
    }

    #[test]
    fn console_text_size_stays_in_the_full_settings_modes() {
        let has_size = |group: Group| {
            group
                .rows()
                .iter()
                .any(|setting| setting.cvar.eq_ignore_ascii_case("con_scale"))
        };
        assert!(has_size(Group::Interface), "classic+ Interface");
        assert!(
            has_size(Group::Advanced(Advanced::Interface)),
            "SJK Advanced"
        );
        assert!(!has_size(Group::Simple(Simple::General)), "SJK Simple");
    }

    #[test]
    fn console_colour_copy_stays_in_the_full_interface_settings() {
        let has_switch = |group: Group| {
            group.rows().iter().any(|setting| {
                setting
                    .cvar
                    .eq_ignore_ascii_case(crate::console::console_options::COPY_COLORS_CVAR)
            })
        };
        assert!(has_switch(Group::Interface), "classic+ Interface");
        assert!(
            has_switch(Group::Advanced(Advanced::Interface)),
            "SJK Advanced"
        );
        assert!(!has_switch(Group::Simple(Simple::General)), "SJK Simple");
    }

    #[test]
    fn ejk_camera_rows_stay_together_in_classic_and_advanced_gameplay() {
        let expected = [
            crate::camera::STYLE_CVAR,
            "cg_drawGun",
            "cg_fovViewmodel",
            "cg_cameraFPS",
            "cg_thirdPersonRange",
            "cg_thirdPersonAngle",
            "cg_thirdPersonPitchOffset",
            "cg_thirdPersonVertOffset",
            "cg_thirdPersonHorzOffset",
            "cg_thirdPersonCameraDamp",
            "cg_thirdPersonTargetDamp",
            "cg_wallGrabLock",
            crate::held_view::LOOK_CVAR,
            "cg_predictLegacy",
            "cg_errorDecay",
        ];
        for group in [Group::GameOptions, Group::Advanced(Advanced::Gameplay)] {
            let rows: Vec<&str> = group.rows().iter().map(|setting| setting.cvar).collect();
            assert!(
                rows.windows(expected.len())
                    .any(|window| window == expected)
            );
        }

        let float_range = |cvar| match find(cvar).unwrap().kind() {
            ValueKind::Float { min, max, step } => (min, max, step),
            _ => panic!("{cvar} is not a float slider"),
        };
        assert_eq!(float_range("cg_fovViewmodel"), (0.0, 120.0, 5.0));
        assert_eq!(float_range("cg_thirdPersonRange"), (40.0, 500.0, 5.0));
        assert_eq!(float_range("cg_thirdPersonAngle"), (-180.0, 180.0, 5.0));
        assert_eq!(
            float_range("cg_thirdPersonPitchOffset"),
            (-129.0, 50.0, 1.0)
        );
        assert_eq!(float_range("cg_thirdPersonVertOffset"), (-58.0, 80.0, 1.0));
        assert_eq!(float_range("cg_thirdPersonHorzOffset"), (-64.0, 64.0, 1.0));
    }

    #[test]
    fn every_heading_names_a_row_of_its_group_and_the_first_row_is_under_one() {
        for group in Group::ALL
            .into_iter()
            .chain([Group::Quick])
            .chain(Group::sjk_only().filter(|group| *group != Group::Graphics))
        {
            let rows = group.rows();
            for (cvar, heading) in group.headings() {
                assert!(
                    rows.iter()
                        .any(|setting| setting.cvar.eq_ignore_ascii_case(cvar)),
                    "{group:?}: {heading} names {cvar}, not one of its rows"
                );
            }
            let lines = group.lines();
            assert_eq!(
                lines.len(),
                rows.len() + group.headings().len(),
                "{group:?}"
            );
            if !group.headings().is_empty() {
                assert!(
                    matches!(lines[0], super::super::Line::Heading(_)),
                    "{group:?}"
                );
            }
        }
    }

    #[test]
    fn graphics_leads_with_the_preset_then_holds_every_renderer_row_once_under_its_tabs_name() {
        let rows = Group::Graphics.rows();
        let renderer: Vec<&str> = GRAPHICS_LEAD
            .iter()
            .copied()
            .chain(
                graphics_tabs()
                    .iter()
                    .flat_map(|(_, tab)| tab.iter().map(|setting| setting.cvar)),
            )
            .collect();
        assert_eq!(
            rows.iter().map(|setting| setting.cvar).collect::<Vec<_>>(),
            renderer
        );
        let lines = Group::Graphics.lines();
        let headings: Vec<&str> = lines
            .iter()
            .filter_map(|line| match line {
                super::super::Line::Heading(heading) => Some(*heading),
                super::super::Line::Row(_) => None,
            })
            .collect();
        assert_eq!(
            headings,
            [
                "Quality",
                "Image",
                "Textures",
                "Effects",
                "Materials",
                "Solar Renderer lighting",
                "Light shafts",
                "Time of day",
                "Shadows",
                "Weather",
                "Rain looks",
            ]
        );
        assert_eq!(lines.len(), rows.len() + 5 + GRAPHICS_SUBHEADINGS.len());
        // Every row once, in order.
        let shown: Vec<usize> = lines
            .iter()
            .filter_map(|line| match line {
                super::super::Line::Row(row) => Some(*row),
                super::super::Line::Heading(_) => None,
            })
            .collect();
        assert_eq!(shown, (0..rows.len()).collect::<Vec<_>>());
    }

    #[test]
    fn the_groups_hold_the_game_hud_and_text_tabs_once_each() {
        let grouped: Vec<&str> = Group::ALL
            .iter()
            .flat_map(|group| group.rows())
            .map(|setting| setting.cvar)
            .collect();
        for caption in ["GAME", "HUD", "HUD+", "TEXT"] {
            let tab = TABS.iter().position(|tab| *tab == caption).unwrap();
            for setting in super::super::settings(tab) {
                let count = grouped.iter().filter(|cvar| **cvar == setting.cvar).count();
                assert_eq!(count, 1, "{} ({caption})", setting.cvar);
            }
        }
    }
}
