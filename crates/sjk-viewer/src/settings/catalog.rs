//! Declarative settings catalog shared by the settings screen.
//!

//! Every row here is backed by a consumer: a cvar that nothing reads yet is
//! not offered. Player identity
//! (name, model, sabers) lives in the Player menu, not here.

pub(super) const TABS: [&str; 9] = [
    "VIDEO",
    "AUDIO",
    "HUD",
    "CONTROLS",
    "GAME",
    "NETWORK",
    "HUD+",
    "TEXT",
    FIRST_SETUP_CAPTION,
];
/// The first-start tab's caption, also the classic Setup group's name.
pub(crate) const FIRST_SETUP_CAPTION: &str = "FIRST SETUP";
/// The first-start tab (`settings/quick.rs`); last, so the other tabs keep their numbers.
pub(super) const QUICK_TAB: usize = 8;
/// The tab the renderer settings ([`RENDERER_TABS`]) open from.
pub(super) const RENDERER_TAB: usize = 0;
/// Tabs of the renderer settings, SJK's own rendering cvars, which the
/// classic Graphics page's panels and the SJK UI's Graphics show.
pub(super) const RENDERER_TABS: [&str; 4] = ["IMAGE", "LIGHTING", "SHADOWS", "WEATHER"];

#[derive(Clone, Copy)]
pub(super) enum ValueKind {
    /// On/off; an integer cvar reads nonzero as on and is written 0 or 1.
    Bool,
    Integer {
        min: i64,
        max: i64,
        step: i64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
    },
    Choice(&'static [&'static str]),
    /// A [`ValueKind::Choice`] whose choices are known only while the game runs (the
    /// installed emotes); [`Setting::kind`] hands it on as one.
    Live(fn() -> &'static [&'static str]),
    Text,
    /// `r_resolution`: steps within the aspect group, Enter opens the list.
    Resolution,
    /// `r_fullscreen` with `r_exclusiveFullscreen`, as named modes.
    DisplayMode,
    /// The HUD in use (`cg_hudStyle`, `cg_hudFiles`, `cg_hudPack`): steps
    /// through the HUDs, Enter opens the HUD picker with its previews.
    HudPicker,
    /// The quick wheel's pages (`wheel.json`, not a cvar): Enter opens their
    /// editor ([`super::wheel_editor`]).
    WheelPages,
    /// The graphics preset ([`crate::graphics_quality`], not a cvar): steps
    /// or lists the presets, each setting the renderer's costly cvars.
    Quality,
    /// The SJK identity's key (not a cvar): Enter opens the Identity page, where the
    /// key's id and file stay hidden until shown.
    IdentityPage,
    /// Import from another client (not a cvar): Enter opens the Import page with
    /// the configs found, or the file dialog over it ([`crate::config_import`]).
    ImportPage,
    /// Open the installed PK3 pack browser.
    AssetBrowser,
    /// A chat kind's preview (not a cvar; its name is the console command that plays
    /// the same, `chat_sounds.rs`): Enter or a click plays the kind's chosen sound.
    SoundPreview,
    /// Open a content card: JoF content (`jof.downloads`, `crate::jof_content`) or the
    /// Sith cloud (`sith.downloads`, `crate::sith_cloud`); the row's cvar is the command.
    #[cfg_attr(not(feature = "mod-jof"), allow(dead_code))]
    ContentCard,
    /// Open the HUD edit mode over the game (`hud_edit`, [`crate::hud_edit`]); the
    /// row's cvar is the command.
    HudEdit,
    /// Run a console command at once (not a cvar): the row's cvar is the command.
    Run,
    /// `cl_gameplayFps`: Left/Right step through the presets (125, 142, 250,
    /// 333); Enter opens the card that explains it and takes any value
    /// ([`crate::gameplay_fps`]).
    GameplayFps,
}

/// The "SJK identity key" row's name in place of a cvar: the command that opens the
/// same page.
pub(crate) const IDENTITY_ROW: &str = "identity";

/// The "Import from a client" row's name in place of a cvar: the command that
/// opens the same page.
pub(crate) const IMPORT_ROW: &str = "firstsetup import";

/// The Gameplay FPS row, shared by Video and First setup.
pub(super) const GAMEPLAY_FPS_ROW: Setting = Setting {
    label: "Gameplay FPS",
    cvar: crate::gameplay_fps::CVAR,
    kind: ValueKind::GameplayFps,
};

#[derive(Clone, Copy)]
pub(super) struct Setting {
    pub(super) label: &'static str,
    pub(super) cvar: &'static str,
    pub(super) kind: ValueKind,
}

impl Setting {
    /// How the row edits its value: a [`ValueKind::Live`] row as the choice it is now.
    pub(super) fn kind(&self) -> ValueKind {
        match self.kind {
            ValueKind::Live(choices) => ValueKind::Choice(choices()),
            kind => kind,
        }
    }
}

pub(crate) const RESOLUTIONS: &[&str] = &[
    "1280x720",
    "1600x900",
    "1920x1080",
    "2560x1440",
    "3840x2160",
];
pub(super) const VIDEO: &[Setting] = &[
    Setting {
        label: "Graphics preset (reload)",

        cvar: crate::graphics_quality::ROW_NAME,
        kind: ValueKind::Quality,
    },
    Setting {
        label: "Resolution",
        cvar: "r_resolution",
        kind: ValueKind::Resolution,
    },
    Setting {
        label: "High DPI (reload)",
        cvar: super::display::HIGH_DPI_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Display mode",
        cvar: "r_fullscreen",
        kind: ValueKind::DisplayMode,
    },
    Setting {
        label: "Vertical sync",
        cvar: "r_vsync",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "FPS cap (AUTO, 0 = off)",
        cvar: "com_maxfps",
        kind: ValueKind::Integer {
            min: -1,
            max: 2000,
            step: 25,
        },
    },
    Setting {
        label: "Menu FPS cap (0 = game)",
        cvar: crate::runtime_settings::MENU_CAP_CVAR,
        // A cap to lower the menus' rate: past 300 it would rarely lower anything, and the
        // usual 30 to 144 keep room on the slider.
        kind: ValueKind::Integer {
            min: 0,
            max: 300,
            step: 10,
        },
    },
    GAMEPLAY_FPS_ROW,
    Setting {
        label: "Ignore server caps",
        cvar: crate::server_caps::OVERRIDE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Detect refresh rate",
        cvar: crate::runtime_settings::MONITOR_CAP_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Field of view",
        cvar: "cg_fov",
        kind: ValueKind::Float {
            min: 70.0,
            max: 130.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Impact marks",
        cvar: "cg_marks",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Player shadows",
        cvar: "cg_shadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Brightness",
        cvar: "r_gamma",
        kind: ValueKind::Float {
            min: 0.5,
            max: 3.0,
            step: 0.1,
        },
    },
];
pub(super) const AUDIO: &[Setting] = &[
    Setting {
        label: "Effects volume",
        cvar: "s_volume",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Music volume",
        cvar: "s_musicVolume",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Voice volume",
        cvar: crate::audio::VOICE_VOLUME_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Map music sync",
        cvar: crate::audio::music::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Doppler",
        cvar: "s_doppler",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Blade skin sounds",
        cvar: crate::audio::SKIN_SOUNDS_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Jetpack ignition sound",
        cvar: crate::jetpack::ON_SOUND_CVAR,
        kind: ValueKind::Choice(&["1", "2"]),
    },
    Setting {
        label: "Jetpack hover sound",
        cvar: crate::jetpack::HOVER_SOUND_CVAR,
        kind: ValueKind::Choice(&["1", "2"]),
    },
    Setting {
        label: "Taunt anti-spam",
        cvar: "cg_tauntAntiSpam",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Footsteps",
        cvar: "cg_footsteps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hit sounds",
        cvar: "cg_hitsounds",
        kind: ValueKind::Choice(&["0", "1", "2", "3", "4", "5", "6"]),
    },
    Setting {
        label: "Muted sounds",
        cvar: crate::audio::muted_sounds::CVAR,
        kind: ValueKind::Text,
    },
    Setting {
        label: "Sound sonar",
        cvar: crate::sound_sonar::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "3D sound",
        cvar: crate::audio::spatial::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Rain and wind sounds",
        cvar: crate::weather::sound::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Footprints and dust",
        cvar: crate::footprints::PRINTS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Jump and landing steps",
        cvar: crate::footprints::JUMP_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ground by texture name",
        cvar: crate::footprints::GUESS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Mute in background",
        cvar: "snd_mute_losefocus",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Emote volume",
        cvar: crate::emotes::VOLUME_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: crate::emotes::MAX_VOLUME,
            step: 0.1,
        },
    },
    Setting {
        label: "Mute emotes in duels",
        cvar: crate::emotes::DUEL_MUTE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Attack ends emote",
        cvar: crate::emotes::ATTACK_STOP_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Typing keeps emote",
        cvar: crate::emotes::TYPING_KEEP_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Away emote",
        cvar: crate::away_emote::SJK_CVAR,
        kind: ValueKind::Live(crate::away_emote::sjk_choices),
    },
    Setting {
        label: "JA+ away emote",
        cvar: crate::away_emote::JAPLUS_CVAR,
        kind: ValueKind::Choice(&crate::away_emote::JAPLUS_EMOTES),
    },
    Setting {
        label: "Away when alt-tabbed",
        cvar: crate::away_emote::ALT_TAB_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Away after (seconds)",
        cvar: crate::away_emote::IDLE_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: 1800,
            step: 30,
        },
    },
    Setting {
        label: "Toy volume",
        cvar: crate::toy_sounds::VOLUME_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: crate::toy_sounds::MAX_VOLUME,
            step: 0.1,
        },
    },
    Setting {
        label: "Cardboard box sounds",
        cvar: crate::toy_sounds::Toy::Box.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Order 66 sounds",
        cvar: crate::toy_sounds::Toy::Order66.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hax sounds",
        cvar: crate::toy_sounds::Toy::Hax.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ragdoll sounds",
        cvar: crate::toy_sounds::Toy::Ragdoll.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Drone sounds",
        cvar: crate::toy_sounds::Toy::Drone.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Boombox music",
        cvar: crate::toy_sounds::Toy::Boombox.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Monitor sounds",
        cvar: crate::toy_sounds::Toy::Monitor.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Mystery box sounds",
        cvar: crate::toy_sounds::Toy::MysteryBox.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Banana peel sounds",
        cvar: crate::toy_sounds::Toy::Banana.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Tag spray sound",
        cvar: crate::toy_sounds::Toy::Tag.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Toy Master sounds",
        cvar: crate::toy_sounds::Toy::Master.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Fishing sounds",
        cvar: crate::toy_sounds::Toy::Fishing.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Fishing bite cue",
        cvar: crate::fishing::CUE_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Unlock sound",
        cvar: crate::unlock_toast::SOUND_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat sounds",
        cvar: crate::chat_sounds::MASTER_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Public chat",
        cvar: crate::chat_sounds::Kind::Public.switch_cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Public chat sound",
        cvar: crate::chat_sounds::Kind::Public.sound_cvar(),
        kind: ValueKind::Choice(&crate::chat_sounds::SOUND_IDS),
    },
    Setting {
        label: "Preview public chat",
        cvar: crate::chat_sounds::Kind::Public.preview_row(),
        kind: ValueKind::SoundPreview,
    },
    Setting {
        label: "Team chat",
        cvar: crate::chat_sounds::Kind::Team.switch_cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Team chat sound",
        cvar: crate::chat_sounds::Kind::Team.sound_cvar(),
        kind: ValueKind::Choice(&crate::chat_sounds::SOUND_IDS),
    },
    Setting {
        label: "Preview team chat",
        cvar: crate::chat_sounds::Kind::Team.preview_row(),
        kind: ValueKind::SoundPreview,
    },
    Setting {
        label: "Private messages",
        cvar: crate::chat_sounds::Kind::Private.switch_cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Private message sound",
        cvar: crate::chat_sounds::Kind::Private.sound_cvar(),
        kind: ValueKind::Choice(&crate::chat_sounds::SOUND_IDS),
    },
    Setting {
        label: "Preview private message",
        cvar: crate::chat_sounds::Kind::Private.preview_row(),
        kind: ValueKind::SoundPreview,
    },
    Setting {
        label: "SJK chat messages",
        cvar: crate::chat_sounds::Kind::Sjk.switch_cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK chat sound",
        cvar: crate::chat_sounds::Kind::Sjk.sound_cvar(),
        kind: ValueKind::Choice(&crate::chat_sounds::SOUND_IDS),
    },
    Setting {
        label: "Preview SJK chat",
        cvar: crate::chat_sounds::Kind::Sjk.preview_row(),
        kind: ValueKind::SoundPreview,
    },
    Setting {
        label: "Private message flash",
        cvar: crate::chat_sounds::FLASH_CVAR,
        kind: ValueKind::Bool,
    },
];
pub(super) const HUD_OPTIONS: &[Setting] = &[
    Setting {
        label: "Crosshair size",
        cvar: "cg_crosshairSize",
        kind: ValueKind::Float {
            min: 0.0,
            max: 96.0,
            step: 4.0,
        },
    },
    Setting {
        label: "Speedometer",
        cvar: "cg_speedometer",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Local time",
        cvar: crate::hud::clock::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2", "3"]),
    },
    Setting {
        label: "Local time: 12-hour",
        cvar: crate::hud::clock::HOURS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Movement keys",
        cvar: crate::hud::movement_keys::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Strafe lines",
        cvar: crate::hud::movement::LINES_ROW,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Strafe aura",
        cvar: crate::hud::movement::AURA_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Aura on the ground",
        cvar: crate::hud::movement::AURA_GROUND_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard style",
        cvar: crate::scoreboard::style::CVAR,
        kind: ValueKind::Choice(&crate::scoreboard::style::ScoreboardStyle::NAMES),
    },
    Setting {
        label: "Compact SJK scoreboard",
        cvar: crate::scoreboard::style::COMPACT_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard frame rate",
        cvar: crate::scoreboard::style::FPS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Away players apart",
        cvar: crate::scoreboard::AFK_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard client IDs",
        cvar: crate::scoreboard::style::IDS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK scoreboard IDs",
        cvar: crate::scoreboard::style::SJK_IDS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Rank by score - deaths",
        cvar: crate::scoreboard::style::DEATHS_RANK_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Clan emblems",
        cvar: crate::jof_tag::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Scoreboard head icons",
        cvar: "cg_drawScoreboardIcons",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Small scoreboard rows",
        cvar: "cg_smallScoreboard",
        kind: ValueKind::Bool,
    },
];

pub(super) const HUD: &[Setting] = &[
    Setting {
        label: "HUD",
        cvar: "cg_drawHud",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD off: elements",
        cvar: crate::hud::toggle::Part::Elements.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD off: crosshair",
        cvar: crate::hud::toggle::Part::Crosshair.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD off: names",
        cvar: crate::hud::toggle::Part::Names.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD off: chat",
        cvar: crate::hud::toggle::Part::Chat.cvar(),
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HUD scale",
        cvar: "cg_hudScale",
        kind: ValueKind::Float {
            min: 0.5,
            max: 1.5,
            step: 0.05,
        },
    },
    Setting {
        label: "Move HUD elements",
        cvar: crate::hud_edit::COMMAND,
        kind: ValueKind::HudEdit,
    },
    Setting {
        label: "Radial HUD spacing",
        cvar: crate::hud::RadialPlacement::SPREAD_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 250.0,
            step: 10.0,
        },
    },
    Setting {
        label: "Radial HUD height",
        cvar: crate::hud::RadialPlacement::HEIGHT_CVAR,
        kind: ValueKind::Float {
            min: -20.0,
            max: 40.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Status (health / armour / force)",
        cvar: "cg_drawStatus",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Weapon bar",
        cvar: "cg_drawWeapon",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Inventory list",
        cvar: "cg_drawInventory",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Crosshair (0 off)",
        cvar: "cg_crosshair",
        kind: ValueKind::Integer {
            min: 0,
            max: 10,
            step: 1,
        },
    },
    Setting {
        label: "Crosshair names",
        cvar: "cg_drawCrosshairNames",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Damage hitmarker",
        cvar: "cg_hitMarker",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Green on shield hits",
        cvar: crate::damage_feedback::SHIELD_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Overhead names",
        cvar: "cg_drawPlayerNames",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Name size",
        cvar: "cg_drawPlayerNamesScale",
        kind: ValueKind::Float {
            min: 0.2,
            max: 1.5,
            step: 0.1,
        },
    },
    Setting {
        label: "Team logos",
        cvar: "cg_drawFriend",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Player card",
        cvar: crate::hud::player_card::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Card delay (s)",
        cvar: "cg_playerCardDelay",
        kind: ValueKind::Float {
            min: 0.5,
            max: 5.0,
            step: 0.5,
        },
    },
    Setting {
        label: "Card outline",
        cvar: "cg_playerCardOutline",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Team outlines",
        cvar: crate::hud::team_outline::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "See & share nameplates",
        cvar: "cg_nameplate",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Nameplate style",
        cvar: crate::hud::nameplate::STYLE_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Nameplate range",
        cvar: "cg_nameplateRange",
        kind: ValueKind::Integer {
            min: 500,
            max: 10000,
            step: 250,
        },
    },
    Setting {
        label: "Bars distance",
        cvar: "cg_nameplateNear",
        kind: ValueKind::Integer {
            min: 0,
            max: 5000,
            step: 100,
        },
    },
    Setting {
        label: "Nameplate size",
        cvar: "cg_nameplateScale",
        kind: ValueKind::Float {
            min: 0.2,
            max: 1.5,
            step: 0.1,
        },
    },
    Setting {
        label: "Nameplate opacity",
        cvar: crate::hud::nameplate::OPACITY_CVAR,
        kind: ValueKind::Float {
            min: 0.1,
            max: 1.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Nameplate bars",
        cvar: "cg_nameplateBars",
        kind: ValueKind::Integer {
            min: 0,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Power icons",
        cvar: "cg_nameplateIcons",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Duel badge",
        cvar: "cg_nameplateDuel",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Name outline",
        cvar: "cg_nameplateOutline",
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "My own nameplate",
        cvar: "cg_nameplateSelf",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hide duellers",
        cvar: "cg_nameplateHideDuelers",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Plates through walls",
        cvar: "cg_nameplateWalls",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "NPC nameplates",
        cvar: "cg_nameplateNpcs",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Others' world pings",
        cvar: crate::world_ping::SHOW_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "World ping sound",
        cvar: crate::world_ping::SOUND_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "FPS / frame-time readout",
        cvar: "cg_drawFps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Match timer",
        cvar: "cg_drawTimer",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Upper right clock",
        cvar: "cg_drawTimerUpperRight",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Team scores",
        cvar: "cg_drawTeamScores",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK team score boxes",
        cvar: "cg_teamScoresBoxes",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Team score box size",
        cvar: "cg_teamScoresScale",
        kind: ValueKind::Float {
            min: 0.5,
            max: 2.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Flag status (capture the flag)",
        cvar: "cg_drawFlagStatus",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Version and date",
        cvar: crate::version_overlay::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Lagometer",
        cvar: crate::hud::lagometer::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2", "3"]),
    },
    Setting {
        label: "Lagometer scale",
        cvar: "cg_lagometerScale",
        kind: ValueKind::Float {
            min: 0.25,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Kill feed",
        cvar: "cg_killfeed",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat",
        cvar: "cg_drawChat",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat bubble while away",
        cvar: "cl_unfocusedChatbox",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Save game chat logs",
        cvar: crate::console::chat_log::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Game PM-only chat",
        cvar: crate::chat::pm_only::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Quiet mode",
        cvar: crate::chat::quiet::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat text size",
        cvar: "cg_chatBoxFontSize",
        kind: ValueKind::Float {
            min: crate::chat::options::CHAT_SIZE.0,
            max: crate::chat::options::CHAT_SIZE.1,
            step: 0.05,
        },
    },
    Setting {
        label: "Chat font",
        cvar: crate::text::system_font::CHAT_CVAR,
        kind: ValueKind::Live(crate::text::system_font::choices),
    },
    Setting {
        label: "Chat emojis",
        cvar: crate::chat::emoji::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Remember chat channel",
        cvar: crate::chat::reply::REMEMBER_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Tabs hide other chat",
        cvar: crate::chat::tabs::FILTER_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Team chat button",
        cvar: crate::chat::tabs::TEAM_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "SJK chat button",
        cvar: crate::chat::tabs::SJK_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "JoF chat button",
        cvar: crate::chat::tabs::JOF_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "S chat button",
        cvar: crate::chat::tabs::SITH_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Admin chat button",
        cvar: crate::chat::tabs::ADMIN_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "PM chat button",
        cvar: crate::chat::tabs::PM_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat line spacing",
        cvar: crate::chat::options::LINE_SPACING_CVAR,
        kind: ValueKind::Float {
            min: crate::chat::options::LINE_SPACING.0,
            max: crate::chat::options::LINE_SPACING.1,
            step: 1.0,
        },
    },
    Setting {
        label: "Chat letter spacing",
        cvar: "cg_chatBoxLetterSpacing",
        kind: ValueKind::Float {
            min: -2.0,
            max: 8.0,
            step: 0.5,
        },
    },
    Setting {
        label: "Chat backdrop (%)",
        cvar: crate::chat::options::BACKDROP_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: 100,
            step: 10,
        },
    },
    Setting {
        label: "Incoming chat colour",
        cvar: crate::chat::incoming_color::CVAR,
        kind: ValueKind::Choice(crate::chat::incoming_color::CHOICES),
    },
    Setting {
        label: "Chat text outline",
        cvar: crate::chat::options::OUTLINE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "White edge on dark text",
        cvar: crate::text::DARK_OUTLINE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Chat card size",
        cvar: crate::chat::options::CARD_SCALE_CVAR,
        kind: ValueKind::Float {
            min: crate::chat::options::CARD_SCALE.0 as f64,
            max: crate::chat::options::CARD_SCALE.1 as f64,
            step: 0.1,
        },
    },
    Setting {
        label: "HUD look (Enter: pick)",
        cvar: crate::menu_hud::STYLE_CVAR,
        kind: ValueKind::HudPicker,
    },
    Setting {
        label: "Game HUD files",
        cvar: crate::menu_hud::FILES_CVAR,
        kind: ValueKind::Text,
    },
    Setting {
        label: "Classic HUD font",
        cvar: "cg_classicHudFont",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Classic game fonts",
        cvar: crate::game_font::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ground HUD (third person)",
        cvar: crate::ground_hud::CVAR,
        kind: ValueKind::Bool,
    },
];
pub(super) const CONTROLS: &[Setting] = &[
    Setting {
        label: "Mouse sensitivity",
        cvar: "sensitivity",
        kind: ValueKind::Float {
            min: 0.1,
            max: 20.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Invert mouse",
        cvar: "m_invert",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Always run",
        cvar: "cl_run",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Last key wins",
        cvar: crate::input::idrive::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Last key wins delay (ms)",
        cvar: crate::input::idrive::DELAY_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: crate::input::idrive::MAX_DELAY_MILLIS as i64,
            step: 5,
        },
    },
    Setting {
        label: "Hold push/pull repeats",
        cvar: crate::input::force_repeat::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hold mouse 4/5 repeats",
        cvar: crate::input::mouse_repeat::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Move with console open",
        cvar: crate::console_move::CVAR,
        kind: ValueKind::Bool,
    },
];
/// The Hax toy's trap switches, which live on the Collection's Toys tab (the Hax's
/// card) since 03/11/2026 (Lex's idea): no category shows them, and a search finds them
/// under [`HAX_PLACE`], where they still switch.
pub(super) const HAX_TOY: &[Setting] = &[
    Setting {
        label: "Hax trap",
        cvar: crate::hax::TRAP_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hax trap while away",
        cvar: crate::hax::AWAY_CVAR,
        kind: ValueKind::Bool,
    },
];

/// Where [`HAX_TOY`]'s switches are, as a search's heading names it.
pub(super) const HAX_PLACE: &str = "Collection > Toys > Hax";

pub(super) const GAME: &[Setting] = &[
    Setting {
        label: "Asset browser",
        cvar: "assetbrowser",
        kind: ValueKind::AssetBrowser,
    },
    Setting {
        label: "Simple pickup icons",
        cvar: "cg_simpleItems",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show everyone as my model",
        cvar: "cg_forceModel",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Respect model metadata",
        cvar: sjk_client::player_model_eligibility::METADATA_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Player model blacklist",
        cvar: sjk_client::player_model_eligibility::BLACKLIST_CVAR,
        kind: ValueKind::Text,
    },
    Setting {
        label: "Saber trail",
        cvar: "cg_saberTrail",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Reduce bright flashes",
        cvar: crate::effect_flash::REDUCE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Saber clash flare",
        cvar: crate::saber_clash_flare::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Clash flare size",
        cvar: crate::saber_clash_flare::SIZE_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 2.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Blade core thickness",
        cvar: crate::saber_core::CVAR,
        kind: ValueKind::Float {
            min: crate::saber_core::MIN as f64,
            max: crate::saber_core::MAX as f64,
            step: 0.1,
        },
    },
    Setting {
        label: "Saber hit sparks",
        cvar: crate::impacts::FLESH_SPARKS_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: crate::impacts::FLESH_SPARKS_ALL as i64,
            step: 1,
        },
    },
    Setting {
        label: "Team saber colours",
        cvar: crate::team_sabers::TEAM_COLORS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "CTF team saber colours",
        cvar: crate::team_sabers::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "JA+ black sabers",
        cvar: crate::saber_rgb::black::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Blade Lab designs",
        cvar: crate::blade_lab_designs::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show monitors",
        cvar: crate::monitor::SHOW_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show banana peels",
        cvar: crate::banana::SHOW_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show others' holocrons",
        cvar: crate::toy_sight::ILLUMINATE,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show others' boxes",
        cvar: crate::toy_sight::CARDBOARD_BOX,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show others' ragdolls",
        cvar: crate::toy_sight::RAGDOLL,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show others' drones",
        cvar: crate::toy_sight::DRONE,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Show others' boomboxes",
        cvar: crate::toy_sight::BOOMBOX,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Boombox range",
        cvar: crate::boombox::RANGE_CVAR,
        kind: ValueKind::Integer {
            min: crate::boombox::MIN_RANGE,
            max: crate::boombox::MAX_RANGE,
            step: 32,
        },
    },
    Setting {
        label: "Normal blades in duels",
        cvar: crate::duel_blades::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Stock blades (faster)",
        cvar: crate::stock_blades::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Saber rain steam",
        cvar: crate::effect_aux::saber_rain_steam::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Force Speed trail",
        cvar: "cg_speedTrail",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Grip victim effects",
        cvar: crate::actor_world_submission::force_powers::victim::CVAR,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF Destruction support",
        cvar: sjk_mod_jof::destruction::CVAR,
        kind: ValueKind::Choice(&["-1", "0", "1"]),
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF dialogue",
        cvar: sjk_mod_jof::dialogue::CVAR,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF EJK server opt-in",
        cvar: sjk_mod_jof::EJK_OPT_IN,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF ability sounds",
        cvar: sjk_mod_jof::ABILITY_SOUNDS,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF names (Honors, Echoes)",
        cvar: sjk_mod_jof::HONOR_NAMES,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Flamethrower effects",
        cvar: crate::actor_world_submission::force_powers::flame::CVAR,
        kind: ValueKind::Choice(&["-1", "0", "1"]),
    },
    Setting {
        label: "Holstered saber hilts",
        cvar: crate::holstered_sabers::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Holster server support",
        cvar: crate::holstered_sabers::SUPPORT_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Staff handoff point",
        cvar: crate::holstered_sabers::SWAP_CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Staff handoff sounds",
        cvar: "cg_holsteredStaffSound",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shield hit sphere",
        cvar: "cg_shieldSphere",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shield hit brightness",
        cvar: "cg_shieldBrightness",
        kind: ValueKind::Integer {
            min: 1,
            max: 12,
            step: 1,
        },
    },
    Setting {
        label: "Duel figures (JoF)",
        cvar: crate::duel_ghost::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hide others in duel",
        cvar: crate::duel_visibility::HIDE_OTHERS_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Hide corpses in duel",
        cvar: crate::duel_visibility::CORPSES_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Hide pickups in duel",
        cvar: crate::duel_visibility::PICKUPS_CVAR,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "Push away kiss attempts (JA+)",
        cvar: sjk_mod_japlus::anti_kiss::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Force Seeing aura",
        cvar: "cg_auraShell",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Force animation effects",
        cvar: crate::actor_world_submission::force_powers::policy::ANIMATION_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Empower glow (-1 server)",
        cvar: crate::actor_world_submission::force_powers::policy::EMPOWER_CVAR,
        kind: ValueKind::Integer {
            min: -1,
            max: crate::actor_world_submission::force_powers::policy::MAX_SCOPE,
            step: 1,
        },
    },
    Setting {
        label: "Protect+Absorb combo",
        cvar: "cg_spProtAbsColor",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK map edits",
        cvar: crate::map_edits::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Sync map edits",
        cvar: crate::map_edits::SYNC_COMMAND,
        kind: ValueKind::Run,
    },
    Setting {
        label: "Server remaps",
        cvar: crate::remap_blocked_maps::SERVER_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK remap blocks",
        cvar: crate::remap_blocked_maps::SHARED_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Remaps on this map",
        cvar: crate::remap_blocked_maps::MAP_CVAR,
        kind: ValueKind::Choice(&crate::remap_blocked_maps::Choice::NAMES),
    },
    Setting {
        // SJK defaults to 2 (EternalJK); 1 excludes player-texture configstring remaps.
        label: "Shader remaps",
        cvar: "cg_remaps",
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Camera style",
        cvar: crate::camera::STYLE_CVAR,
        kind: ValueKind::Choice(&crate::camera::Style::NAMES),
    },
    Setting {
        label: "First-person weapon",
        cvar: "cg_drawGun",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "First-person weapon FOV",
        cvar: "cg_fovViewmodel",
        kind: ValueKind::Float {
            min: 0.0,
            max: 120.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Camera damping style",
        cvar: "cg_cameraFPS",
        // `cg_cameraFPS` is a float so console values such as 60.5 still work;
        // choices use its canonical text representation.
        kind: ValueKind::Choice(&["125.0", "30.0", "0.0"]),
    },
    Setting {
        label: "Third-person distance",
        cvar: "cg_thirdPersonRange",
        kind: ValueKind::Float {
            min: 40.0,
            max: 500.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Third-person angle",
        cvar: "cg_thirdPersonAngle",
        kind: ValueKind::Float {
            min: -180.0,
            max: 180.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Third-person pitch",
        cvar: "cg_thirdPersonPitchOffset",
        kind: ValueKind::Float {
            min: -129.0,
            max: 50.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Third-person height",
        cvar: "cg_thirdPersonVertOffset",
        kind: ValueKind::Float {
            min: -58.0,
            max: 80.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Third-person horizontal offset",
        cvar: "cg_thirdPersonHorzOffset",
        kind: ValueKind::Float {
            min: -64.0,
            max: 64.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Third-person camera damping",
        cvar: "cg_thirdPersonCameraDamp",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Third-person target damping",
        cvar: "cg_thirdPersonTargetDamp",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Lock wall-grab camera",
        cvar: "cg_wallGrabLock",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Free look when held",
        cvar: crate::held_view::LOOK_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Prediction",
        cvar: "cg_predictLegacy",
        kind: ValueKind::Choice(&["0", "255"]),
    },
    Setting {
        label: "Prediction error smoothing (ms)",
        cvar: "cg_errorDecay",
        kind: ValueKind::Float {
            min: 0.0,
            max: 500.0,
            step: 25.0,
        },
    },
    Setting {
        label: "Rarity effects",
        cvar: crate::rarity_fx::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Menu style",
        cvar: crate::menu::style::CVAR,
        kind: ValueKind::Choice(&crate::menu::style::MenuStyle::NAMES),
    },
    Setting {
        label: "Support SJK",
        cvar: crate::support_links::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Slider peek",
        cvar: super::slider_peek::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Menu pictures (saves video memory)",
        cvar: crate::menu_pictures::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Quick wheel pages",
        cvar: crate::quick_wheel::pages::FILE,
        kind: ValueKind::WheelPages,
    },
    Setting {
        label: "Quick wheel sounds",
        cvar: crate::quick_wheel::SOUNDS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Trade window size",
        cvar: crate::player_trades::SIZE_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    // The client mods (`docs/mods.md`): on, each loading on its servers only.
    Setting {
        label: "JA+ tools",
        cvar: "mod_japlus",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "jaPRO tools",
        cvar: "mod_japro",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Predict all styles",
        cvar: sjk_mod_japro::movement_styles::PREDICT_ANYWAY_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "JoF tools",
        cvar: "mod_jof",
        kind: ValueKind::Bool,
    },
    // JA+ and jaPRO plugin options (`docs/mods.md`): each a bit of `cp_pluginDisable`,
    // named after the feature (`settings/plugin_rows.rs`, the mods' tables).
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "See others in duels (JA+)",
        cvar: "japlus.plugin 1",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "Black sabers (JA+)",
        cvar: "japlus.plugin 3",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "New Force effects (JA+)",
        cvar: "japlus.plugin 5",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "Staff on the back (JA+)",
        cvar: "japlus.plugin 9",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japlus")]
    Setting {
        label: "Holstered sabers (JA+)",
        cvar: "japlus.plugin 15",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Black sabers (jaPRO)",
        cvar: "japro.plugin 0",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Force jumps (jaPRO)",
        cvar: "japro.plugin 1",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Respawn where you fell (jaPRO)",
        cvar: "japro.plugin 5",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Checkpoints on screen (jaPRO)",
        cvar: "japro.plugin 6",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Checkpoints in chat (jaPRO)",
        cvar: "japro.plugin 7",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Damage numbers (jaPRO)",
        cvar: "japro.plugin 8",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Shots from the centre (jaPRO)",
        cvar: "japro.plugin 9",
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-japro")]
    Setting {
        label: "Checkpoints in console (jaPRO)",
        cvar: "japro.plugin 10",
        kind: ValueKind::Bool,
    },
    // JoF content (`docs/jof-content.md`): its card, and where it downloads from.
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF content",
        cvar: crate::jof_content::COMMAND,
        kind: ValueKind::ContentCard,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF content auto-update",
        cvar: crate::jof_content::AUTO,
        kind: ValueKind::Bool,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "JoF content host",
        cvar: crate::jof_content::SOURCE,
        kind: ValueKind::Choice(&crate::jof_content::SOURCES),
    },
    // The Sith cloud (`docs/sith-cloud.md`): the S community's server, on the same card.
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "Sith cloud",
        cvar: crate::sith_cloud::COMMAND,
        kind: ValueKind::ContentCard,
    },
    #[cfg(feature = "mod-jof")]
    Setting {
        label: "Sith cloud auto-update",
        cvar: crate::sith_cloud::AUTO,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Invisible brushes",
        cvar: crate::brush_view::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Map entities",
        cvar: crate::entity_view::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    // Experimental features, off by default.
    Setting {
        label: "Showcase tools (SJK staff)",
        cvar: crate::ghost_replay::SHOWCASE_CVAR,
        kind: ValueKind::Bool,
    },
];
pub(super) const NETWORK: &[Setting] = &[
    Setting {
        label: "Master server",
        cvar: "cl_master",
        kind: ValueKind::Text,
    },
    Setting {
        label: "Second master server",
        cvar: "cl_master2",
        kind: ValueKind::Text,
    },
    Setting {
        label: "Check for updates",
        cvar: "cl_autoUpdate",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK identity",
        cvar: "cl_identity",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Share my model list",
        cvar: crate::installed_models::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK hub",
        cvar: "cl_hubUrl",
        kind: ValueKind::Text,
    },
    Setting {
        label: "SJK chat",
        cvar: "cl_sjkChat",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK chat in game feed",
        cvar: crate::sjk_chat_frame::FEED_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "SJK chat in console",
        cvar: crate::sjk_chat_frame::CONSOLE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Holocron notifications",
        cvar: crate::holocron_popup::NOTIFICATIONS_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "SJK chat GIFs",
        cvar: crate::chat_gifs::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Play chat video clips (Windows)",
        cvar: crate::chat_gifs::VIDEO_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "JoF clan chat",
        cvar: crate::chat::tabs::JOF_JOIN_CVAR,
        kind: ValueKind::Choice(&["-1", "0", "1"]),
    },
    Setting {
        label: "S clan chat",
        cvar: crate::chat::tabs::SITH_JOIN_CVAR,
        kind: ValueKind::Choice(&["-1", "0", "1"]),
    },
    Setting {
        label: "Other clients' icons",
        cvar: crate::chat::client_icons::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "GIPHY key",
        cvar: crate::chat_gifs::search::KEY_CVAR,
        kind: ValueKind::Text,
    },
    Setting {
        label: "Encrypted whispers",
        cvar: crate::e2e_whispers::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Follow SJK staff blocks",
        cvar: crate::map_blocks::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ignore mutes sounds",
        cvar: crate::sound_mutes::IGNORE_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Send crash reports",
        cvar: crate::crash_report::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Share settings usage",
        cvar: crate::settings_usage::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Discord status",
        cvar: crate::discord_presence::CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "SJK identity key",
        cvar: IDENTITY_ROW,
        kind: ValueKind::IdentityPage,
    },
    Setting {
        label: "Rate (bytes/s)",
        cvar: "rate",
        kind: ValueKind::Integer {
            min: 1000,
            max: 100000,
            step: 1000,
        },
    },
    Setting {
        label: "Snapshots per second",
        cvar: "snaps",
        kind: ValueKind::Integer {
            min: 10,
            max: 125,
            step: 5,
        },
    },
];
/// Text size and spacing, and the console's style and feed. Menu rows keep their layout.
pub(super) const TEXT: &[Setting] = &[
    Setting {
        label: "Console style",
        cvar: crate::console::console_options::STYLE_CVAR,
        kind: ValueKind::Choice(&crate::console::console_options::ConsoleStyle::NAMES),
    },
    Setting {
        label: "Menu text size",
        cvar: crate::text::style::SCALE_CVAR,
        kind: ValueKind::Float {
            min: 0.8,
            max: 1.4,
            step: 0.05,
        },
    },
    Setting {
        label: "Letter spacing (menus, console)",
        cvar: crate::text::style::TRACKING_CVAR,
        kind: ValueKind::Float {
            min: -0.05,
            max: 0.15,
            step: 0.01,
        },
    },
    Setting {
        label: "Console font",
        cvar: crate::text::system_font::CONSOLE_CVAR,
        kind: ValueKind::Live(crate::text::system_font::choices),
    },
    Setting {
        label: "Console text size",
        cvar: "con_scale",
        kind: ValueKind::Float {
            min: 0.5,
            max: 2.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Console feed (top left)",
        cvar: crate::console::console_options::DRAW_NOTIFY_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Copy console colours",
        cvar: crate::console::console_options::COPY_COLORS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Swap Shift console size",
        cvar: crate::console::console_options::SHIFT_HEIGHT_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Console over chat",
        cvar: crate::console::console_options::OVER_CHAT_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Other clients' text",
        cvar: crate::text::charset::CVAR,
        kind: ValueKind::Choice(&crate::text::charset::NAMES),
    },
];

// Renderer settings. "(reload)" marks cvars the renderer reads only when its graphics
// start (their registrations print a notice on change and the graphics reload card
// offers to apply them, `graphics_reload.rs`); "(next map)" those read
// when a map loads. The rest apply immediately. Ranges follow each consumer's clamp.

pub(super) const RENDER_IMAGE: &[Setting] = &[
    Setting {
        label: "HDR scene (reload)",
        cvar: "r_sceneHdr",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "HDR exposure",
        cvar: "r_hdrExposure",
        kind: ValueKind::Float {
            min: 0.25,
            max: 4.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Eye adaptation",
        cvar: crate::frame_target::aa::exposure::ENABLED,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Adaptation: max darken, EV (HDR)",
        cvar: crate::frame_target::aa::exposure::MIN_EV,
        kind: ValueKind::Float {
            min: -2.0,
            max: 0.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Adaptation: max brighten, EV",
        cvar: crate::frame_target::aa::exposure::MAX_EV,
        kind: ValueKind::Float {
            min: 0.0,
            max: 2.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Filmic tone curve",
        cvar: "r_toneCurve",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Bloom",
        cvar: "r_sceneBloom",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Dynamic glow",
        cvar: "r_DynamicGlow",
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "No glow on sabers",
        cvar: crate::frame_target::aa::glow::NO_SABERS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Glow style",
        cvar: "r_dynamicGlowStyle",
        kind: ValueKind::Choice(&["0", "1"]),
    },
    Setting {
        label: "FXAA (reload)",
        cvar: "r_fxaa",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Sharpening (%)",
        cvar: crate::frame_target::aa::color::SHARPEN_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: crate::frame_target::aa::color::SHARPEN_MAX,
            step: 5,
        },
    },
    Setting {
        label: "Supersampling, 1 off (reload)",
        cvar: "r_superSample",
        kind: ValueKind::Integer {
            min: 1,
            max: 3,
            step: 1,
        },
    },
    Setting {
        label: "Render scale (%) (reload)",
        cvar: crate::frame_target::scale::CVAR,
        kind: ValueKind::Integer {
            min: crate::frame_target::scale::MIN_PERCENT as i64,
            max: crate::frame_target::scale::NATIVE as i64,
            step: 5,
        },
    },
    Setting {
        label: "Texture detail, 0 full (next map)",
        cvar: crate::world_materials::filtering::mips::PICMIP_CVAR,
        kind: ValueKind::Integer {
            min: 0,
            max: crate::world_materials::filtering::mips::PICMIP_MAX as i64,
            step: 1,
        },
    },
    Setting {
        label: "Texture compression (next map)",
        cvar: crate::world_materials::filtering::mips::compress::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Soft particles",
        cvar: "r_softParticles",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Light flares",
        cvar: "r_flares",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Ambient occlusion",
        cvar: "r_ssao",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Sunbeam dust (0 off)",
        cvar: crate::dust_motes::CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Per-pixel model lighting",
        cvar: "r_modelPixelLight",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Toon models",
        cvar: crate::dynamic_lights::TOON_MODELS_CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Reflection probes (reload)",
        cvar: "r_cubeMapping",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Floor mirrors",
        cvar: "r_floorReflections",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Normal maps (reload)",
        cvar: "r_normalMapping",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Specular maps (reload)",
        cvar: "r_specularMapping",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Parallax mapping (reload)",
        cvar: "r_parallaxMapping",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Parallax depth (0 flat)",
        cvar: "r_parallaxStrength",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Emission maps (reload)",
        cvar: "r_emissiveMaps",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Emission strength (0 off)",
        cvar: "r_emissionStrength",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Emission glow halo",
        cvar: "r_emissiveGlow",
        kind: ValueKind::Bool,
    },
];

pub(super) const RENDER_LIGHTING: &[Setting] = &[
    Setting {
        label: "Sun and sky (reload)",
        cvar: "r_dayNight",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Live lighting 0-2 (next map)",
        cvar: "r_liveLighting",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    Setting {
        label: "Time of day (hour)",
        cvar: "r_dayHour",
        kind: ValueKind::Float {
            min: 0.0,
            max: 24.0,
            step: 0.5,
        },
    },
    Setting {
        label: "Sun follows PC clock",
        cvar: "r_dayClock",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Day length, min (0 holds)",
        cvar: "r_dayMinutes",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1440.0,
            step: 5.0,
        },
    },
    Setting {
        label: "Sunlight brightness",
        cvar: "r_dayBrightness",
        kind: ValueKind::Float {
            min: 0.1,
            max: 10.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Ambient fill",
        cvar: "r_ambientFill",
        kind: ValueKind::Float {
            min: 0.0,
            max: 0.2,
            step: 0.005,
        },
    },
    Setting {
        label: "Ambient fill corner shading",
        cvar: "r_ambientFillOcclusion",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
    Setting {
        label: "Indirect light boost",
        cvar: "r_indirectBoost",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.1,
        },
    },
    Setting {
        label: "Emission-map lights (next map)",
        cvar: "r_emissiveLights",
        kind: ValueKind::Float {
            min: 0.0,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Glowing lamp light (next map)",
        cvar: crate::world_materials::emission::lamp_gain::CVAR,
        kind: ValueKind::Float {
            min: 0.0,
            max: crate::world_materials::emission::lamp_gain::MAX as f64,
            step: 0.5,
        },
    },
    Setting {
        label: "Light shafts (reload)",
        cvar: "r_volumetrics",
        kind: ValueKind::Choice(&["0", "1", "2", "3"]),
    },
    Setting {
        label: "Light shaft clarity",
        cvar: "r_volumetricClarity",
        kind: ValueKind::Float {
            min: 0.0,
            max: 1.0,
            step: 0.05,
        },
    },
];

pub(super) const RENDER_SHADOWS: &[Setting] = &[
    Setting {
        label: "World sun shadows (reload)",
        cvar: "r_worldSunShadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Character sun shadows (reload)",
        cvar: "r_actorSunShadows",
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Shadow resolution (reload)",
        cvar: "r_sunShadowResolution",
        kind: ValueKind::Integer {
            min: 512,
            max: 4096,
            step: 512,
        },
    },
    Setting {
        label: "Sharp shadow distance (reload)",
        cvar: "r_sunShadowDistance",
        kind: ValueKind::Integer {
            min: 128,
            max: 4096,
            step: 128,
        },
    },
    Setting {
        label: "Close shadow distance (reload)",
        cvar: "r_sunShadowNear",
        kind: ValueKind::Integer {
            min: 64,
            max: 1024,
            step: 64,
        },
    },
    Setting {
        label: "Shadow filter taps (reload)",
        cvar: "r_sunShadowTaps",
        kind: ValueKind::Integer {
            min: 4,
            max: 32,
            step: 4,
        },
    },
    Setting {
        label: "Close shadow slits (units)",
        cvar: "r_sunShadowGapClose",
        kind: ValueKind::Float {
            min: 0.0,
            max: 64.0,
            step: 1.0,
        },
    },
    Setting {
        label: "Contact shadows",
        cvar: "r_contactShadows",
        kind: ValueKind::Bool,
    },
];

/// The weather: rain, snow and fog (`weather.rs`) and the clouds.
pub(super) const RENDER_WEATHER: &[Setting] = &[
    Setting {
        label: "Weather (rain, snow, mist)",
        cvar: crate::weather::CVAR,
        kind: ValueKind::Bool,
    },
    Setting {
        label: "Weather density (1 original)",
        cvar: crate::weather::DENSITY_CVAR,
        kind: ValueKind::Float {
            min: 0.25,
            max: 4.0,
            step: 0.25,
        },
    },
    Setting {
        label: "Weather quality",
        cvar: crate::weather::QUALITY_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2", "3"]),
    },
    Setting {
        label: "Force weather",
        cvar: crate::weather::FORCE_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2", "3", "4"]),
    },
    Setting {
        label: "Ground fog",
        cvar: crate::weather::FOG_CVAR,
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Map fog",
        cvar: "r_drawfog",
        kind: ValueKind::Choice(&["0", "1", "2"]),
    },
    Setting {
        label: "Volumetric clouds",
        cvar: crate::weather::CLOUDS_CVAR,
        kind: ValueKind::Bool,
    },
];
