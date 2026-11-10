//! What each setting does, in the player's words, for the classic+ option
//! panels' detail box (see `docs/classic-plus.md`), as retail's `descText`
//! said what each `setup.menu` item did. Keyed by cvar; every row of the
//! catalogue has one. When a setting takes effect comes from its label's
//! "(restart)", "(reload)" or "(next map)", which the classic+ rows show as a mark.

/// Lines the detail box gives a description, and characters per line.
pub(super) const HELP_LINES: usize = 2;
pub(super) const HELP_LINE_CHARS: usize = 68;

/// When a changed setting takes effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Timing {
    Now,
    Restart,
    /// After a graphics reload (`graphics_reload.rs`), which the reload card offers.
    Reload,
    NextMap,
}

impl Timing {
    /// The detail box's words for it.
    pub(super) fn note(self) -> Option<&'static str> {
        match self {
            Self::Now => None,
            Self::Restart => Some("applies after a restart"),
            Self::Reload => Some("applies after a graphics reload"),
            Self::NextMap => Some("applies on the next map"),
        }
    }
}

/// `label` without its timing note, and the timing it gave.
pub(super) fn timing(label: &str) -> (&str, Timing) {
    for (suffix, timing) in [
        (" (restart)", Timing::Restart),
        (" (reload)", Timing::Reload),
        (" (next map)", Timing::NextMap),
    ] {
        if let Some(stem) = label.strip_suffix(suffix) {
            return (stem, timing);
        }
    }
    (label, Timing::Now)
}

/// What a classic+ row calls a setting: its label without the timing note
/// and without a trailing note in brackets ("Dynamic glow (0 off, 2 sabers)"
/// is "Dynamic glow"), which the retail label column has no room for; the
/// detail box shows the whole label and explains the values.
pub(super) fn row_label(label: &str) -> (&str, Timing) {
    let (label, timing) = timing(label);
    let short = match label.find(" (") {
        Some(at) if label.ends_with(')') && at > 0 => &label[..at],
        _ => label,
    };
    (short, timing)
}

/// Classic row names of settings whose label is longer than the label
/// column; the detail box still shows the whole label.
const SHORT_LABELS: &[(&str, &str)] = &[
    ("cg_drawFps", "FPS readout"),
    ("cg_forceModel", "Everyone as my model"),
    ("cg_fovViewmodel", "Weapon FOV"),
    ("cg_cameraFPS", "Damping style"),
    ("cg_thirdPersonRange", "Camera distance"),
    ("cg_thirdPersonAngle", "Camera angle"),
    ("cg_thirdPersonPitchOffset", "Camera pitch"),
    ("cg_thirdPersonVertOffset", "Camera height"),
    ("cg_thirdPersonHorzOffset", "Camera side"),
    ("cg_thirdPersonCameraDamp", "Camera damping"),
    ("cg_thirdPersonTargetDamp", "Target damping"),
    ("cg_wallGrabLock", "Lock wall-grab camera"),
    (crate::held_view::LOOK_CVAR, "Free look when held"),
    ("cg_predictLegacy", "Prediction"),
    ("cg_errorDecay", "Error smoothing"),
    (crate::ghost_replay::SHOWCASE_CVAR, "Showcase tools"),
    (crate::frame_target::aa::exposure::MIN_EV, "Max darken, EV"),
    (
        crate::frame_target::aa::exposure::MAX_EV,
        "Max brighten, EV",
    ),
    ("r_modelPixelLight", "Pixel model lighting"),
    ("r_ambientFillOcclusion", "Ambient fill shading"),
];

/// What a classic+ row calls setting `cvar` labelled `label`: its short name
/// if it has one, else [`row_label`].
pub(super) fn classic_label<'a>(cvar: &str, label: &'a str) -> (&'a str, Timing) {
    let (row, timing) = row_label(label);
    let short = SHORT_LABELS
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map_or(row, |(_, short)| *short);
    (short, timing)
}

/// What a choice row's value is called where the value itself is a number or
/// a code: each `(cvar, value, name)`. A value not listed is shown as it is
/// written ([`super::classic_view::choice_label`]).
const CHOICE_NAMES: &[(&str, &str, &str)] = &[
    (crate::hud::clock::CVAR, "0", "Off"),
    (crate::hud::clock::CVAR, "1", "Time"),
    (crate::hud::clock::CVAR, "2", "Time and date"),
    (crate::hud::clock::CVAR, "3", "Date"),
    (
        crate::chat::incoming_color::CVAR,
        "0",
        crate::chat::incoming_color::LABELS[0],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "1",
        crate::chat::incoming_color::LABELS[1],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "2",
        crate::chat::incoming_color::LABELS[2],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "3",
        crate::chat::incoming_color::LABELS[3],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "4",
        crate::chat::incoming_color::LABELS[4],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "5",
        crate::chat::incoming_color::LABELS[5],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "6",
        crate::chat::incoming_color::LABELS[6],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "7",
        crate::chat::incoming_color::LABELS[7],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "8",
        crate::chat::incoming_color::LABELS[8],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "9",
        crate::chat::incoming_color::LABELS[9],
    ),
    (
        crate::chat::incoming_color::CVAR,
        "10",
        crate::chat::incoming_color::LABELS[10],
    ),
    (crate::fishing::CUE_CVAR, "0", "Off"),
    (crate::fishing::CUE_CVAR, "1", "Sound off"),
    (crate::fishing::CUE_CVAR, "2", "Always"),
    (crate::chat_sounds::MASTER_CVAR, "0", "Off"),
    (crate::chat_sounds::MASTER_CVAR, "1", "On"),
    (crate::chat_sounds::MASTER_CVAR, "2", "Distinct (EJK)"),
    (crate::camera::STYLE_CVAR, "ejk", "MP camera"),
    (crate::camera::STYLE_CVAR, "sjk", "SP camera"),
    (crate::player_trades::SIZE_CVAR, "0", "Small"),
    (crate::player_trades::SIZE_CVAR, "1", "Normal"),
    (crate::player_trades::SIZE_CVAR, "2", "Large"),
    (crate::holocron_popup::NOTIFICATIONS_CVAR, "0", "Off"),
    (
        crate::holocron_popup::NOTIFICATIONS_CVAR,
        "1",
        "After duels",
    ),
    (crate::holocron_popup::NOTIFICATIONS_CVAR, "2", "Always"),
    (crate::discord_presence::CVAR, "0", "Off"),
    (crate::discord_presence::CVAR, "1", "On"),
    (crate::discord_presence::CVAR, "2", "Server hidden"),
    #[cfg(feature = "mod-jof")]
    (sjk_mod_jof::destruction::CVAR, "-1", "Auto"),
    #[cfg(feature = "mod-jof")]
    (sjk_mod_jof::destruction::CVAR, "0", "Off"),
    #[cfg(feature = "mod-jof")]
    (sjk_mod_jof::destruction::CVAR, "1", "Grant contract"),
    (
        crate::actor_world_submission::force_powers::flame::CVAR,
        "-1",
        "Auto",
    ),
    (
        crate::actor_world_submission::force_powers::flame::CVAR,
        "0",
        "Off",
    ),
    (
        crate::actor_world_submission::force_powers::flame::CVAR,
        "1",
        "JA+",
    ),
    (crate::text::charset::CVAR, "auto", "Auto"),
    (crate::text::charset::CVAR, "western", "Western"),
    (crate::text::charset::CVAR, "cyrillic", "Cyrillic"),
    (crate::duel_visibility::HIDE_OTHERS_CVAR, "0", "Off"),
    (crate::duel_visibility::HIDE_OTHERS_CVAR, "1", "Auto"),
    (crate::duel_visibility::HIDE_OTHERS_CVAR, "2", "Always"),
    (crate::input::idrive::CVAR, "0", "Off"),
    (crate::input::idrive::CVAR, "1", "All keys"),
    (crate::input::idrive::CVAR, "2", "Jump, crouch"),
    (crate::input::mouse_repeat::CVAR, "0", "Off"),
    (crate::input::mouse_repeat::CVAR, "1", "Mouse 4 and 5"),
    (crate::input::mouse_repeat::CVAR, "2", "Mouse 4 only"),
    (crate::hud::player_card::CVAR, "0", "Off"),
    (crate::hud::player_card::CVAR, "1", "On look and key"),
    (crate::hud::player_card::CVAR, "2", "Key only"),
    (crate::hud::lagometer::CVAR, "0", "Off"),
    (crate::hud::lagometer::CVAR, "1", "Graph"),
    (crate::hud::lagometer::CVAR, "2", "Graph + speed"),
    (crate::hud::lagometer::CVAR, "3", "No backdrop"),
    ("cg_predictLegacy", "0", "Current"),
    ("cg_predictLegacy", "255", "As in 2026.1028.1"),
    ("cg_hitsounds", "0", "Off"),
    ("cg_hitsounds", "1", "Tick"),
    ("cg_hitsounds", "2", "Blip"),
    ("cg_hitsounds", "3", "Two notes"),
    ("cg_hitsounds", "4", "Knock"),
    ("cg_hitsounds", "5", "Saber: stock hit"),
    ("cg_hitsounds", "6", "Saber: mixed hits"),
    (crate::audio::SKIN_SOUNDS_CVAR, "0", "Off"),
    (crate::audio::SKIN_SOUNDS_CVAR, "1", "With normal"),
    (crate::audio::SKIN_SOUNDS_CVAR, "2", "Only skin"),
    (crate::hud::nameplate::STYLE_CVAR, "0", "Holo"),
    (crate::hud::nameplate::STYLE_CVAR, "1", "Capsule"),
    (crate::hud::nameplate::STYLE_CVAR, "2", "Classic"),
    ("cg_nameplateOutline", "0", "Off"),
    ("cg_nameplateOutline", "1", "Faint"),
    ("cg_nameplateOutline", "2", "Strong"),
    (crate::effect_aux::saber_rain_steam::CVAR, "0", "Off"),
    (crate::effect_aux::saber_rain_steam::CVAR, "1", "Everyone"),
    (crate::effect_aux::saber_rain_steam::CVAR, "2", "Only mine"),
    (crate::stock_blades::CVAR, "0", "Off"),
    (crate::stock_blades::CVAR, "1", "Others"),
    (crate::stock_blades::CVAR, "2", "All"),
    ("jof.contentSource", "auto", "Auto"),
    ("jof.contentSource", "mirror", "JoF Cloud"),
    ("jof.contentSource", "origin", "jofhub.eu"),
    ("cg_remaps", "0", "Never"),
    ("cg_remaps", "1", "Not players"),
    ("cg_remaps", "2", "Always"),
    ("cg_cameraFPS", "125.0", "Default"),
    ("cg_cameraFPS", "30.0", "Smooth (SP)"),
    ("cg_cameraFPS", "0.0", "BaseJKA"),
    (crate::frame_timings::DRAW_CVAR, "0", "Off"),
    (crate::frame_timings::DRAW_CVAR, "1", "GPU"),
    (crate::frame_timings::DRAW_CVAR, "2", "GPU + CPU"),
    (crate::brush_view::CVAR, "0", "Off"),
    (crate::brush_view::CVAR, "1", "Gameplay"),
    (crate::brush_view::CVAR, "2", "All"),
    (crate::entity_view::CVAR, "0", "Off"),
    (crate::entity_view::CVAR, "1", "Common"),
    (crate::entity_view::CVAR, "2", "All"),
    ("r_DynamicGlow", "0", "Off"),
    ("r_DynamicGlow", "1", "On"),
    ("r_DynamicGlow", "2", "Sabers"),
    ("r_dynamicGlowStyle", "0", "Original"),
    ("r_dynamicGlowStyle", "1", "Vulkan"),
    ("r_superSample", "1", "Off"),
    ("r_superSample", "2", "2x"),
    ("r_superSample", "3", "3x"),
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "0",
        "Full",
    ),
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "1",
        "Half",
    ),
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "2",
        "Quarter",
    ),
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "3",
        "Eighth",
    ),
    (crate::dust_motes::CVAR, "0.0", "Off"),
    (crate::dust_motes::CVAR, "0.5", "Light"),
    (crate::dust_motes::CVAR, "1.0", "Full"),
    ("r_liveLighting", "0", "Baked bounce"),
    ("r_liveLighting", "1", "Live, casters kept"),
    ("r_liveLighting", "2", "All live"),
    ("r_dayMinutes", "0.0", "Holds still"),
    ("r_dayMinutes", "10.0", "10 minutes"),
    ("r_dayMinutes", "30.0", "30 minutes"),
    ("r_dayMinutes", "60.0", "1 hour"),
    ("r_dayMinutes", "1440.0", "24 hours"),
    ("r_sunShadowResolution", "512", "Low"),
    ("r_sunShadowResolution", "1024", "Medium"),
    ("r_sunShadowResolution", "2048", "High"),
    ("r_sunShadowResolution", "4096", "Ultra"),
    ("r_sunShadowTaps", "4", "Hard"),
    ("r_sunShadowTaps", "8", "Soft"),
    ("r_sunShadowTaps", "16", "Softer"),
    ("r_sunShadowTaps", "24", "Very soft"),
    ("r_sunShadowTaps", "32", "Softest"),
    ("r_sunShadowDistance", "512", "Short"),
    ("r_sunShadowDistance", "1024", "Medium"),
    ("r_sunShadowDistance", "2048", "Long"),
    ("r_sunShadowDistance", "4096", "Very long"),
    ("r_sunShadowNear", "128", "Short"),
    ("r_sunShadowNear", "256", "Medium"),
    ("r_sunShadowNear", "512", "Long"),
    ("r_sunShadowNear", "1024", "Very long"),
    (crate::weather::DENSITY_CVAR, "0.5", "Light"),
    (crate::weather::DENSITY_CVAR, "1.0", "Original"),
    (crate::weather::DENSITY_CVAR, "2.0", "Double"),
    (crate::weather::DENSITY_CVAR, "3.0", "Triple"),
    (crate::weather::DENSITY_CVAR, "4.0", "Quadruple"),
    (crate::weather::PUDDLES_CVAR, "0.0", "None"),
    (crate::weather::PUDDLES_CVAR, "0.5", "A few"),
    (crate::weather::PUDDLES_CVAR, "1.0", "Some"),
    (crate::weather::PUDDLES_CVAR, "1.5", "Many"),
    (crate::weather::PUDDLES_CVAR, "2.0", "Flooded"),
    (crate::weather::RAIN_OPACITY_CVAR, "0.5", "Faint"),
    (crate::weather::RAIN_OPACITY_CVAR, "0.75", "Light"),
    (crate::weather::RAIN_OPACITY_CVAR, "1.0", "Normal"),
    (crate::weather::RAIN_OPACITY_CVAR, "1.5", "Heavy"),
    (crate::weather::RAIN_OPACITY_CVAR, "2.0", "Very heavy"),
    (crate::weather::HAZE_CVAR, "0.0", "Clear"),
    (crate::weather::HAZE_CVAR, "0.5", "Light"),
    (crate::weather::HAZE_CVAR, "1.0", "Normal"),
    (crate::weather::HAZE_CVAR, "1.5", "Thick"),
    (crate::weather::HAZE_CVAR, "2.0", "Very thick"),
    (crate::weather::FOG_DENSITY_CVAR, "0.5", "Thin"),
    (crate::weather::FOG_DENSITY_CVAR, "1.0", "Normal"),
    (crate::weather::FOG_DENSITY_CVAR, "1.5", "Thick"),
    (crate::weather::FOG_DENSITY_CVAR, "2.0", "Very thick"),
    (crate::weather::FOG_DENSITY_CVAR, "3.0", "Pea soup"),
    ("r_volumetrics", "0", "Off"),
    ("r_volumetrics", "1", "Low"),
    ("r_volumetrics", "2", "Medium"),
    ("r_volumetrics", "3", "High"),
    (crate::weather::QUALITY_CVAR, "0", "Low"),
    (crate::weather::QUALITY_CVAR, "1", "Medium"),
    (crate::weather::QUALITY_CVAR, "2", "High"),
    (crate::weather::QUALITY_CVAR, "3", "Ultra"),
    (crate::weather::FORCE_CVAR, "0", "The map's"),
    (crate::weather::FORCE_CVAR, "1", "Drizzle"),
    (crate::weather::FORCE_CVAR, "2", "Rain"),
    (crate::weather::FORCE_CVAR, "3", "Storm"),
    (crate::weather::FORCE_CVAR, "4", "Snow"),
    (crate::weather::FOG_CVAR, "0", "Never"),
    (crate::weather::FOG_CVAR, "1", "Map's fog"),
    (crate::weather::FOG_CVAR, "2", "Always"),
    (crate::weather::REFLECTIONS_CVAR, "0", "By quality"),
    (crate::weather::REFLECTIONS_CVAR, "1", "Sky only"),
    (crate::weather::REFLECTIONS_CVAR, "2", "Puddles"),
    (crate::weather::REFLECTIONS_CVAR, "3", "Everything wet"),
    ("r_drawfog", "0", "Off"),
    ("r_drawfog", "1", "Volume curve"),
    ("r_drawfog", "2", "Original"),
];

/// What choice `value` of setting `cvar` is called, when it has a name of its own.
pub(super) fn choice_name(cvar: &str, value: &str) -> Option<&'static str> {
    CHOICE_NAMES
        .iter()
        .find(|(name, choice, _)| {
            name.eq_ignore_ascii_case(cvar) && choice.eq_ignore_ascii_case(value.trim())
        })
        .map(|(_, _, shown)| *shown)
        .or_else(|| crate::away_emote::choice_name(cvar, value))
        .or_else(|| crate::chat_sounds::choice_name(cvar, value))
        .or_else(|| crate::text::system_font::choice_name(cvar, value))
        .or_else(|| crate::chat::tabs::choice_name(cvar, value))
}

/// What the detail column says, in gold, while the SJK renderer is on and a
/// renderer setting is focused ([`super::renderer_row`]).
pub(super) const ALPHA_WARNING: &str = "The Solar Renderer is in alpha: some maps can show \
    glitches (lighting, shadows, missing surfaces) or run slower. Report them from the SJK menu's \
    Report a bug; Vanilla is the safe choice.";

/// The settings whose description speaks of SJK's medals and holocrons, which the JoF
/// tools may rename ([`crate::terms`]); the others' holocrons are the game's pickups.
const SJK_TERMS: &[&str] = &[
    crate::unlock_toast::SOUND_CVAR,
    "cl_identity",
    crate::holocron_popup::NOTIFICATIONS_CVAR,
];

/// The description of setting `cvar`.
/// The plugin options' rows are described by their mods' tables
/// ([`super::plugin_rows`]).
pub(super) fn help(cvar: &str) -> Option<&'static str> {
    HELP.iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(cvar))
        .map(|&(name, text)| {
            if SJK_TERMS.contains(&name) {
                crate::terms::say_static(text)
            } else {
                text
            }
        })
        .or_else(|| super::plugin_rows::help(cvar))
}

/// `text` cut into at most [`HELP_LINES`] lines of [`HELP_LINE_CHARS`]
/// characters at spaces; the last line takes what is left.
pub(super) fn lines(text: &str) -> [&str; HELP_LINES] {
    let mut out = [""; HELP_LINES];
    let mut rest = text.trim();
    for (index, line) in out.iter_mut().enumerate() {
        if rest.chars().count() <= HELP_LINE_CHARS || index + 1 == HELP_LINES {
            *line = rest;
            break;
        }
        let limit = rest
            .char_indices()
            .nth(HELP_LINE_CHARS)
            .map_or(rest.len(), |(at, _)| at);
        let cut = rest[..limit].rfind(' ').unwrap_or(limit);
        *line = rest[..cut].trim_end();
        rest = rest[cut..].trim_start();
    }
    out
}

const HELP: &[(&str, &str)] = &[
    #[cfg(feature = "mod-jof")]
    (
        sjk_mod_jof::destruction::CVAR,
        "Auto follows the server flag; Off hides Destruction. Grant contract skips the flag. The server must still grant the power.",
    ),
    #[cfg(feature = "mod-jof")]
    (
        sjk_mod_jof::dialogue::CVAR,
        "Show server conversations and their installed voice audio. Off cancels the current conversation; quests remain server-owned.",
    ),
    #[cfg(feature = "mod-jof")]
    (
        sjk_mod_jof::EJK_OPT_IN,
        "Advertise JoF EJK extension support on JoF servers with JoF tools on. Enables server confirmations; see Client mods for scope.",
    ),
    #[cfg(feature = "mod-jof")]
    (
        sjk_mod_jof::ABILITY_SOUNDS,
        "Play installed Stasis and Repulse sounds when the server confirms firing. Duplicate cues within 250 ms are suppressed.",
    ),
    #[cfg(feature = "mod-jof")]
    (
        sjk_mod_jof::HONOR_NAMES,
        "Call medals Honors and holocrons Echoes in every menu and message while JoF tools are on, on every server. On by default.",
    ),
    (
        sjk_client::player_model_eligibility::METADATA_CVAR,
        "Respect installed NPC-only model declarations and pack lists. Off ignores those rules; your blacklist still applies.",
    ),
    (
        sjk_client::player_model_eligibility::BLACKLIST_CVAR,
        "Model names separated by spaces, commas or semicolons. Players use Kyle instead; NPCs are unaffected.",
    ),
    (
        "cg_drawInventory",
        "Permanent list of carried items, off by default. Cycling items still shows the inventory selector.",
    ),
    (
        crate::actor_world_submission::force_powers::flame::CVAR,
        "Flame visuals, pose and sound: Auto detects JA+ servers; Off disables them; JA+ forces support. Needs installed flame assets.",
    ),
    // VIDEO
    (
        crate::graphics_quality::ROW_NAME,
        "Vanilla: the original look. Vanilla Plus: SJK's image on the maps' lighting. Solar Renderer: live lighting, Low to Ultra.",
    ),
    (
        crate::graphics_quality::RECOMMEND_COMMAND,
        "Picks the preset this PC's graphics card runs smoothly: its colour in the preset list is green.",
    ),
    (
        "r_resolution",
        "The window's size, or the screen mode in exclusive fullscreen. Enter opens the list of sizes.",
    ),
    (
        super::display::HIGH_DPI_CVAR,
        "On: sizes and the world in the screen's pixels, sharpest on Retina. \
         Off: in points, as EternalJK on a Mac, much faster.",
    ),
    (
        "r_fullscreen",
        "Windowed, borderless (fills the desktop) or exclusive fullscreen (sets the monitor's mode).",
    ),
    (
        "r_vsync",
        "Waits for the monitor's refresh before showing a frame: no tearing, a little more input delay.",
    ),
    (
        "com_maxfps",
        "Most frames drawn a second; your movement is Gameplay FPS. AUTO is 125, or the monitor's rate with Detect refresh rate; 0 is off.",
    ),
    (
        crate::runtime_settings::MENU_CAP_CVAR,
        "Most frames a second in the menus while no game or demo runs, never above the FPS cap. 0 uses the FPS cap.",
    ),
    (
        crate::server_caps::OVERRIDE_CVAR,
        "On, Gameplay FPS applies on servers SJK knows to cap commands (JoF's: 125) too. Inputs may not go through; Heimdall may kick you.",
    ),
    (
        crate::server_caps::FRESH_STAMPS_CVAR,
        "On, commands carry this frame's server time on every server, not the previous frame's as stock. Heimdall (JoF) may kick you.",
    ),
    (
        crate::gameplay_fps::CVAR,
        "Auto moves you at your FPS cap; 125, 142, 250 or 333 keep that movement at any FPS. Enter explains it and takes any value.",
    ),
    (
        crate::runtime_settings::MONITOR_CAP_CVAR,
        "AUTO on the FPS cap follows the monitor's refresh rate. Off, AUTO caps at 125 and the monitor is not read.",
    ),
    (
        "cg_fov",
        "How wide you see, in degrees across the screen: higher shows more at the sides.",
    ),
    (
        "cg_marks",
        "Scorch and blaster marks left on walls and floors by shots and explosions.",
    ),
    ("cg_shadows", "Players' shadows on the ground under them."),
    (
        "cg_drawGun",
        "Shows your weapon in first person; off hides it for a clear view.",
    ),
    (
        "cg_fovViewmodel",
        "Width of the first-person weapon view. 80 is EJK; 0 uses the world FOV like the original game.",
    ),
    (
        "cg_drawFps",
        "Shows frames per second and frame time in a corner of the screen.",
    ),
    (
        crate::frame_timings::DRAW_CVAR,
        "Panel at the top right: the milliseconds each part of the frame takes on the graphics card (GPU), and on the processor (CPU) too.",
    ),
    (
        crate::frame_timings::LOG_CVAR,
        "Writes the frame timings to logs/timings.log every second, with your graphics settings and each change you make to them.",
    ),
    (
        "r_gamma",
        "Brightness curve of the whole display, HUD included. 1 is neutral; higher lifts dark areas.",
    ),
    // AUDIO
    (
        "s_volume",
        "Loudness of every sound but the music and the voices.",
    ),
    ("s_musicVolume", "Loudness of the music."),
    (
        crate::audio::VOICE_VOLUME_CVAR,
        "Loudness of voices (pain, taunts, grunts, voice commands), which ignore the effects volume, as in the original game.",
    ),
    (
        crate::jetpack::ON_SOUND_CVAR,
        "Jetpack ignition: 1 default, 2 alternate, as in JoF EJK. Shutdown uses one fixed sound.",
    ),
    (
        crate::jetpack::HOVER_SOUND_CVAR,
        "Jetpack hover: 1 default, 2 alternate, as in JoF EJK. The flaming fire loop is independent.",
    ),
    (
        "s_doppler",
        "Moving looping sounds, such as passing ships, change pitch as they come and go.",
    ),
    (
        crate::actor_world_submission::force_powers::policy::ANIMATION_CVAR,
        "Rage, Heal, Quick Heal and Grip-victim effects. Plugin settings can also disable them.",
    ),
    (
        crate::actor_world_submission::force_powers::policy::EMPOWER_CVAR,
        "Empower glow: -1 follows the server; 0 full body, 1 arms, 2 hands, 3 off. Ordinary Push stays full body.",
    ),
    (
        "cg_tauntAntiSpam",
        "Limits repeated player model voices. Taunts wait until the previous line ends; other grunts have separate short holds.",
    ),
    (
        crate::audio::music::CVAR,
        "Resume map music by level time on join or after duels. Off restarts it. Dynamic music keeps its own timing.",
    ),
    (
        crate::chat::pm_only::CVAR,
        "Shows only server-marked private game chat for this map. Console history and SJK hub chat stay available.",
    ),
    (
        crate::chat::quiet::CVAR,
        "Hides all chat but your duel partner's and whispers, and their sounds, until the map changes. Also `quiet`.",
    ),
    (
        crate::audio::SKIN_SOUNDS_CVAR,
        "A blade skin's own ignition, swings and hum: off (normal saber sounds), over the normal ones, or instead of them.",
    ),
    ("cg_footsteps", "Footstep sounds, yours and other players'."),
    (
        crate::audio::muted_sounds::CVAR,
        "Words, such as ysalamiri hacking: no sound whose file name holds one plays. mutesounds <player> mutes a player.",
    ),
    (
        "cg_hitsounds",
        "A sound each time you hit a player (another for a teammate), as in EJK. Saber: only changes saber hit sounds.",
    ),
    (
        crate::sound_sonar::CVAR,
        "For deaf and hard-of-hearing players: arcs round the crosshair point to the sounds played. Needs sound output (not s_initsound 0).",
    ),
    (
        crate::audio::spatial::CVAR,
        "Sounds all round your head, duller behind walls and shut doors, round corners and through doorways; rooms ring with their own echo.",
    ),
    (
        crate::audio::spatial::HEADPHONES_CVAR,
        "On: 3D sound through a measured dummy head, sounds in front, behind, above or below. Off: panned left and right, for speakers.",
    ),
    (
        crate::audio::spatial::REVERB_CVAR,
        "How loud rooms ring with 3D sound: 1 as the walls of the room make it, 0 no echo, 2 twice as loud.",
    ),
    (
        crate::weather::sound::CVAR,
        "Hear the rain and the wind with the weather, as loud as they fall and blow, quieter under a roof. The game's own outdoor sounds.",
    ),
    (
        crate::footprints::PRINTS_CVAR,
        "Steps on sand, snow, mud and dirt kick up dust and leave footprints for ten seconds, as in the original game.",
    ),
    (
        crate::footprints::JUMP_CVAR,
        "A jump and a forward landing step with both feet: the ground's sound, dust and prints. Not in the original game.",
    ),
    (
        crate::footprints::GUESS_CVAR,
        "Untagged ground (much original sand and snow) sounds and marks as its texture name says. Off: stone, as originally.",
    ),
    (
        "snd_mute_losefocus",
        "Silences the game while its window is alt-tabbed out or minimised.",
    ),
    (
        crate::emotes::VOLUME_CVAR,
        "Loudness of emote sounds, apart from the game volume: 1 as recorded, 2 twice as loud.",
    ),
    (
        crate::emotes::DUEL_MUTE_CVAR,
        "On: emotes go silent while you duel (private or Duel server), even ones playing; one still playing when the duel ends is heard again.",
    ),
    (
        crate::toy_sounds::VOLUME_CVAR,
        "Loudness of every toy's sounds: 1 as made, 2 twice as loud, 0 silent. Each toy can also be silenced below.",
    ),
    (
        crate::toy_sounds::Toy::Box.cvar(),
        "Off: cardboard box sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Order66.cvar(),
        "Off: Order 66 sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Hax.cvar(),
        "Off: Hax sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Ragdoll.cvar(),
        "Off: Ragdoll sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Drone.cvar(),
        "Off: Drone sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Boombox.cvar(),
        "Off: no boombox music plays on this PC, yours or other players'. Their stereo is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Monitor.cvar(),
        "Off: Monitor sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::MysteryBox.cvar(),
        "Off: mystery boxes open silently on this PC, yours and other players'.",
    ),
    (
        crate::toy_sounds::Toy::Banana.cvar(),
        "Off: Banana peel sounds are not played on this PC, yours or other players'. The toy still works and is still seen.",
    ),
    (
        crate::toy_sounds::Toy::Tag.cvar(),
        "Off: tags go up without the spray can's sound on this PC, yours and other players'.",
    ),
    (
        crate::toy_sounds::Toy::Master.cvar(),
        "Off: the toys the Toy Master hides on maps play their shows in silence on this PC.",
    ),
    (
        crate::text::system_font::CHAT_CVAR,
        "A font from this PC for the chat. What it lacks (Cyrillic, CJK, emoji) comes from SJK's fonts.",
    ),
    (
        crate::text::system_font::CONSOLE_CVAR,
        "A font from this PC for the console, kept to its even grid. What it lacks comes from SJK's fonts.",
    ),
    (
        crate::toy_sounds::Toy::Fishing.cvar(),
        "Off: your casts, bites and catches are silent. The toy still works.",
    ),
    (
        crate::toy_sounds::Toy::WheelOfDoom.cvar(),
        "Off: every WHEEL OF DOOM spins silently on this PC, yours and other players', quacks included.",
    ),
    (
        crate::toy_sounds::Toy::Quests.cvar(),
        "Off: no chime as you are given a quest, named in one or finish one. Quests still work.",
    ),
    (
        crate::fishing::CUE_CVAR,
        "A \"!\" over the float and \"Bite! Press Use!\" when a fish bites. Sound off: only while the fishing sounds can't be heard.",
    ),
    (
        crate::toy_sounds::Toy::Mining.cvar(),
        "Off: your swings, blows and finds are silent. The toy still works.",
    ),
    (
        crate::away_emote::SJK_CVAR,
        "A looping emote you play while away (alt-tabbed out or idle), silent. Any emote you play then goes quiet too, for everyone.",
    ),
    (
        crate::away_emote::JAPLUS_CVAR,
        "On a JA+ server, the JA+ emote you play while away instead (sit, kneel...). Coming back stands you up.",
    ),
    (
        crate::away_emote::ALT_TAB_CVAR,
        "On: alt-tabbing out or minimising counts as away and starts the away emote. Your emote goes quiet either way.",
    ),
    (
        crate::away_emote::IDLE_CVAR,
        "How long without a key or the mouse before you count as away; 0 only alt-tabbing does.",
    ),
    (
        crate::emotes::ATTACK_STOP_CVAR,
        "On: attacking or alt attacking ends your emote, for everyone, so your real swing shows. Off: you keep dancing.",
    ),
    (
        crate::emotes::TYPING_KEEP_CVAR,
        "On: while a player types (chat, console or menu open), their emote stays, even if pushed; moving after ends it.",
    ),
    (
        crate::unlock_toast::SOUND_CVAR,
        "Plays the secret-area chime with each unlock card: an achievement, or a new medal or holocron in a match.",
    ),
    // HUD
    (
        "cg_drawHud",
        "Shows the HUD; off hides it, for screenshots and recordings. The four rows under it choose what off hides.",
    ),
    (
        crate::hud::toggle::Part::Elements.cvar(),
        "With the HUD off: health, shield, Force, ammo, timers, the kill feed and the rest of the HUD hide. Off: they stay.",
    ),
    (
        crate::hud::toggle::Part::Crosshair.cvar(),
        "With the HUD off: the crosshair and the name under it hide. Off: they stay.",
    ),
    (
        crate::hud::toggle::Part::Names.cvar(),
        "With the HUD off: nameplates, names over players and team logos hide. Off: they stay.",
    ),
    (
        crate::hud::toggle::Part::Chat.cvar(),
        "With the HUD off: the chat box hides too. Off (the default): the chat stays.",
    ),
    ("cg_hudScale", "Size of the HUD: higher is larger."),
    (
        crate::hud_edit::COMMAND,
        "Drag the HUD's elements over the game with the mouse, the kill feed too; right-click one to put it back.",
    ),
    (
        crate::hud::RadialPlacement::SPREAD_CVAR,
        "SJK radial HUD only: moves the health and armour meters left and the Force and ammo meters right, farther from your character.",
    ),
    (
        crate::hud::RadialPlacement::HEIGHT_CVAR,
        "SJK radial HUD only: moves its meters up (higher numbers) or down (below zero) on the screen.",
    ),
    ("cg_drawStatus", "Health, armor and Force on the HUD."),
    (
        "cg_drawWeapon",
        "The weapon bar shown while you change weapons.",
    ),
    (
        "cg_crosshair",
        "Crosshair picture: 0 hides it, 1 to 9 pick a picture, 10 is a white dot.",
    ),
    (
        "cg_drawCrosshairNames",
        "Shows the name of the player under your crosshair.",
    ),
    (
        "cg_drawPlayerNames",
        "Plain names above players: 0 off, 1 names, 2 with a health strip. Nameplates replace it.",
    ),
    (
        "cg_drawPlayerNamesScale",
        "Size of the plain names above players.",
    ),
    (
        crate::hud::player_card::CVAR,
        "A card with a player's model, sabers and SJK profile: on a steady look or the Inspect player key, or Key only: that key alone.",
    ),
    (
        "cg_playerCardDelay",
        "With Player card On look and key: seconds you keep looking at a player, steady, before it shows.",
    ),
    (
        "cg_playerCardOutline",
        "Outline the player whose card is up, in the card's colour.",
    ),
    (
        crate::hud::team_outline::CVAR,
        "In team games, a red outline around enemies and a green one around teammates, only where you can see them.",
    ),
    (
        "cg_drawFriend",
        "Your team's logo over teammates' heads (everyone's while spectating), your partner's in power duel.",
    ),
    (
        "cg_nameplate",
        "Nameplates over players, and your health, shield and Force for SJK players on your server. Off: neither.",
    ),
    (
        crate::hud::nameplate::STYLE_CVAR,
        "Holo: your picture in a hexagon by a plate. Capsule: a pill, the bars under it. Classic: the name over a plate of bars.",
    ),
    (
        "cg_nameplateRange",
        "How far away, in game units, a nameplate still shows. It fades near the end.",
    ),
    (
        "cg_nameplateNear",
        "Inside this distance the bars fade in under the name; beyond it just the name.",
    ),
    ("cg_nameplateScale", "Size of the nameplate text."),
    (
        "cg_nameplateAlpha",
        "How solid nameplates are, from 0.1 (faint) to 1 (solid, the default). The distance and wall fades still apply.",
    ),
    (
        "cg_nameplateHideDuelers",
        "Hide names, bars and icons on players the server marks as private duellers, including your own plate. Off by default.",
    ),
    (
        "cg_nameplateBars",
        "Bars under the name: 0 none, 1 allies, 2 everyone, 3 your target. Never on a duel opponent. V cycles modes.",
    ),
    (
        "cg_nameplateSelf",
        "Your own nameplate over your head in third person, with your real bars.",
    ),
    (
        "cg_nameplateIcons",
        "Icons of the Force powers a player has on, over the name when close.",
    ),
    (
        "cg_nameplateOutline",
        "Edges names so they read on any background. Faint: dark letters get a soft halo. Strong: a dark outline around every name too.",
    ),
    (
        "cg_nameplateDuel",
        "Crossed sabers before the name of a player who is in a duel.",
    ),
    (
        "cg_nameplateWalls",
        "Shows nameplates of players behind walls, dimmed. Off hides them.",
    ),
    (
        "cg_nameplateNpcs",
        "NPC names, such as Stormtrooper. Health requires active level 3 Sense unless the mapper enables it.",
    ),
    (
        crate::world_ping::SHOW_CVAR,
        "Shows other SJK players' pings (+ping, middle mouse): your team's in team games, everyone's otherwise. Off hides them.",
    ),
    (
        crate::world_ping::SOUND_CVAR,
        "A short sound when a world ping appears, yours or another player's.",
    ),
    ("cg_drawTimer", "Shows the match time."),
    (
        "cg_drawTimerUpperRight",
        "Puts the match time in the upper right under the frame rate, as JoF EJK does, instead of in the HUD layout.",
    ),
    (
        "cg_drawTeamScores",
        "Team games: the red and blue scores at the top centre of the HUD, in every HUD style.",
    ),
    (
        "cg_teamScoresScale",
        "Size of the SJK team score boxes: 1 is the default, 2 twice as large.",
    ),
    (
        "cg_teamScoresBoxes",
        "Team scores in SJK boxes, in CTF with each flag's carrier or return time. Off: the classic plates and flag icons.",
    ),
    (
        "cg_drawFlagStatus",
        "Capture the flag: where each flag is, in the team boxes or as icons at the left, and a note while you carry one.",
    ),
    (
        crate::version_overlay::CVAR,
        "Shows SJK's version, build date and commit at the top, so screenshots say which build.",
    ),
    (
        crate::hud::lagometer::CVAR,
        "Network delay graph. Graph + speed adds the average ping and interpolation and a speed graph (JoF EJK); No backdrop, bare.",
    ),
    (
        "cg_lagometerScale",
        "Size of the lagometer: 1 normal, 2 double. Keeps its bottom-right corner fixed.",
    ),
    (
        "cg_killfeed",
        "The last kills, killer, weapon icon, victim, at the top right or wherever Move HUD elements put it.",
    ),
    ("cg_drawChat", "Shows chat messages over the game."),
    (
        "cl_unfocusedChatbox",
        "Shows your automatic chat bubble while unfocused or minimized. Open chat, console and menus still show their normal bubble.",
    ),
    (
        crate::status_balloons::CVAR,
        "The chat bubble over SJK players shows why: dots while typing, a pause sign in a menu or the console, a moon while away.",
    ),
    (
        crate::console::chat_log::CVAR,
        "Log all game-server chat, including team/PMs before filters, in this profile's chatlogs/chat.log. Off by default.",
    ),
    (
        "cg_chatBoxFontSize",
        "Size of chat messages and their rows; 1 is EternalJK's size, higher is larger and wraps sooner.",
    ),
    (
        crate::chat::emoji::CVAR,
        "Shows emoji pictures in place of names like :poop: in new chat messages.",
    ),
    (
        crate::chat::tabs::FILTER_CVAR,
        "On, a chat button shows only its own messages while you type; off, every chat stays in view, labelled.",
    ),
    (
        crate::chat::tabs::TEAM_CVAR,
        "Shows Team beside All when you type. Not in Tab keeps the button but Tab skips it; the team key still opens it.",
    ),
    (
        crate::chat::tabs::SJK_CVAR,
        "Shows SJK beside All when you type, for the SJK chat. Not in Tab keeps the button but Tab skips it.",
    ),
    (
        crate::chat::tabs::ADMIN_CVAR,
        "Shows Admin on JA+ and jaPRO servers: only the server's admins read it (amsay). Not in Tab: Tab skips it.",
    ),
    (
        crate::chat::tabs::JOF_CVAR,
        "Shows JoF when you type, once you joined JoF's clan chat. Not in Tab keeps the button but Tab skips it.",
    ),
    (
        crate::chat::tabs::SITH_CVAR,
        "Shows S when you type, once you joined the Sith clan's chat. Not in Tab keeps the button but Tab skips it.",
    ),
    (
        crate::chat::tabs::JOF_JOIN_CVAR,
        "JoF's clan chat, for verified JoF players and staff only. Auto joins it if it is yours; Joined or Left choose.",
    ),
    (
        crate::chat::tabs::SITH_JOIN_CVAR,
        "The Sith clan's chat (S), for verified S players and staff only. Auto joins it if it is yours.",
    ),
    (
        crate::chat::tabs::PM_CVAR,
        "Shows PM when you type: it replies to players who whispered you; left click next, right click back.",
    ),
    (
        crate::chat::reply::REMEMBER_CVAR,
        "On, the chat key opens on your last channel: All, Team, SJK or a whisper. Off (the default), it always opens on All.",
    ),
    (
        crate::chat::options::LINE_SPACING_CVAR,
        "Space between chat messages; lower packs them closer. A long message's own lines stay together.",
    ),
    (
        "cg_chatBoxLetterSpacing",
        "Adds space between the letters of chat messages; below zero packs them closer.",
    ),
    (
        crate::chat::options::BACKDROP_CVAR,
        "A dark band behind each chat line, so it reads over bright maps: how dark, in per cent. 0 shows none.",
    ),
    (
        crate::chat::incoming_color::CVAR,
        "Colour public All chat text. Names, team/admin chat, whispers and SJK chat keep their colours. 0 keeps originals.",
    ),
    (
        crate::chat::options::OUTLINE_CVAR,
        "Edges chat letters and what you type in a dark outline instead of a shadow, so they read over bright maps.",
    ),
    (
        crate::text::DARK_OUTLINE_CVAR,
        "Black and other dark letters in names and chat get a white outline instead of a soft glow.",
    ),
    (
        crate::chat::options::CARD_SCALE_CVAR,
        "Size of the player card a chat name shows, over the chat's own size: 0.5 half, 2 double.",
    ),
    (
        crate::menu_hud::STYLE_CVAR,
        "Which HUD shows health, armor, Force and ammo: the game's, an installed HUD pack's, or SJK's.",
    ),
    (
        crate::menu_hud::FILES_CVAR,
        "The game HUD's menu list (ui/jahud.txt); 1 is the text-only HUD, 3 and 4 EternalJK's HUDs.",
    ),
    (
        "cg_classicHudFont",
        "Draws the HUD's numbers and text in the original game's HUD font.",
    ),
    (
        crate::game_font::CVAR,
        "Draws the menus and console in the original game's fonts instead of SJK's.",
    ),
    (
        crate::ground_hud::CVAR,
        "In third person, shows health, armor and Force on the ground around your character.",
    ),
    // CONTROLS
    (
        "sensitivity",
        "How far the view turns for a given mouse movement.",
    ),
    (
        "m_invert",
        "Moving the mouse forward looks down instead of up.",
    ),
    ("cl_run", "Run by default; the Walk key walks."),
    (
        crate::input::idrive::CVAR,
        "Opposite movement keys move toward the one pressed last instead of cancelling: all of them, or jump and crouch only.",
    ),
    (
        crate::input::idrive::DELAY_CVAR,
        "Stand still this long after pressing the opposite key before it takes over; some servers punish instant flips.",
    ),
    (
        crate::input::force_repeat::CVAR,
        "Keep a push or pull key held to use the power again the moment its cooldown ends, as in JoF EJK.",
    ),
    (
        crate::input::mouse_repeat::CVAR,
        "Hold mouse 4 or 5 to run its bind again 20 times a second, like a held key (JoF EJK, there at the frame rate).",
    ),
    (
        crate::console_move::CVAR,
        "Movement keys held as the console opens keep moving you, with no talk balloon; keys pressed while it is open still type.",
    ),
    // GAME
    (
        "cg_simpleItems",
        "Shows pickups as flat icons instead of 3D models, easier to spot from afar.",
    ),
    (
        "cg_forceModel",
        "Draws every player with your own model and skin.",
    ),
    (
        "cg_saberTrail",
        "The light trail behind a swinging saber blade.",
    ),
    (
        crate::effect_flash::REDUCE_CVAR,
        "For players sensitive to flashing light: no saber clash flare and no screen flash from explosions or DEMP2 shocks.",
    ),
    (
        crate::saber_clash_flare::CVAR,
        "The burst of light drawn over the screen where two sabers meet, as in the original game.",
    ),
    (
        crate::saber_clash_flare::SIZE_CVAR,
        "Size of the saber clash flare: 1 as in the original game, 0 to 2.",
    ),
    (
        crate::saber_core::CVAR,
        "How thick the white core down a lit blade is drawn: 1 as in the original game, 0.5 to 2. The glow around it stays the same.",
    ),
    (
        crate::impacts::FLESH_SPARKS_CVAR,
        "Red sparks when a saber hits a body: 7 all, as in the original game, down to 0 none (JoF EJK).",
    ),
    (
        crate::team_sabers::TEAM_COLORS_CVAR,
        "Team games: sabers in the team's colour, shaders worn over it. Off changes only the other team's colour and RGB or black.",
    ),
    (
        crate::team_sabers::CVAR,
        "In capture the flag, every red or blue player's sabers in their team's colour, with no blade shader. Off by default.",
    ),
    (
        crate::saber_rgb::black::CVAR,
        "Show black blades on JA+/jaPRO; off uses orange. Plugin feature 3 must also be allowed.",
    ),
    (
        crate::blade_lab_designs::CVAR,
        "Draw the Blade Lab skins (Serpent, Comic and the rest) as Creyon's Blade Lab drew them; off uses their skin files.",
    ),
    (
        crate::hax::TRAP_CVAR,
        "The Hax toy: a player who hacks you springs a trap; after two seconds you both blow up, as in Order 66.",
    ),
    (
        crate::hax::AWAY_CVAR,
        "With the Hax trap on: players may hack you while you are away, and blow up with you. Others can see it is set.",
    ),
    (
        crate::monitor::SHOW_CVAR,
        "Off: no Monitor toy monitor is drawn, yours or other players', nor their sounds heard. You can still be hit while the toy is on.",
    ),
    (
        crate::banana::SHOW_CVAR,
        "Off: no Banana peel toy peel is drawn, yours or other players', nor their sounds heard. You can still slip while the toy is on.",
    ),
    (
        crate::tags::SHOW_CVAR,
        "Off: other players' tags are not drawn on this PC, nor their sprays heard. Yours still is.",
    ),
    (
        crate::tags::GIF_SHOW_CVAR,
        "Off by default. On: GIF tags (a GIPHY GIF on a blank tag) play on walls, yours and others'.",
    ),
    (
        crate::toy_sight::ILLUMINATE,
        "Off: other players' Illuminate holocrons are not drawn on this PC. Yours still is.",
    ),
    (
        crate::toy_sight::CARDBOARD_BOX,
        "Off: other players' cardboard boxes are not drawn on this PC, and you cannot lift them.",
    ),
    (
        crate::toy_sight::RAGDOLL,
        "Off: other players' Ragdoll falls are not played on this PC; you see their bodies as the game shows them.",
    ),
    (
        crate::toy_sight::DRONE,
        "Off: other players' drones are not drawn or heard on this PC. You are still told who peeks at you.",
    ),
    (
        crate::toy_sight::BOOMBOX,
        "Off: other players' boomboxes are not drawn or heard on this PC.",
    ),
    (
        crate::boombox::RANGE_CVAR,
        "How far your boombox carries, in units (a player is 64 tall). It fades out toward the edge.",
    ),
    (
        crate::duel_blades::CVAR,
        "In a duel you play or follow, and on Duel servers, every blade is a normal one in its own colour.",
    ),
    (
        crate::stock_blades::CVAR,
        "For weak PCs: blades drawn stock in their colour, no shaders or effects. Others: not yours. All: yours too.",
    ),
    (
        crate::effect_aux::saber_rain_steam::CVAR,
        "Steam and sizzle in rain on exposed blades: everyone's, or only your own.",
    ),
    (
        "cg_speedTrail",
        "Afterimages behind players using Force Speed.",
    ),
    (
        crate::actor_world_submission::force_powers::victim::CVAR,
        "Red neck puffs on choking players. On by default; your own stay hidden in first person and while zoomed.",
    ),
    (
        crate::holstered_sabers::CVAR,
        "Show stowed hilts on the hips; a single staff goes on the back when JA+ plugin 9 is enabled. Hidden, dead and mounted players show none.",
    ),
    (
        crate::holstered_sabers::SUPPORT_CVAR,
        "0: off. 1: detect supported servers. 2: force the JA+ ownership and plugin-bit contract; use only when your server supports it.",
    ),
    (
        crate::holstered_sabers::SWAP_CVAR,
        "Where the hand takes the staff during a JA+ reach (default 0.45). Requires installed reach animations.",
    ),
    (
        "cg_holsteredStaffSound",
        "Delay staff ignition until the hand reaches its hilt, sound shutdown at the put-away reach, and suppress its late duplicate.",
    ),
    (
        "cg_auraShell",
        "The glowing shell Force Sight shows around other players.",
    ),
    (
        "cg_spProtAbsColor",
        "Cyan when Protect and Absorb run together. Plugin settings can force the separate green/blue shells.",
    ),
    (
        "cg_shieldSphere",
        "Off: a shield hit flashes on the body, like single player. On: multiplayer's sphere around the player.",
    ),
    (
        "cg_hitMarker",
        "Red direction indicator when you take damage. Off by default; camera kick and sounds stay the same.",
    ),
    (
        crate::damage_feedback::SHIELD_CVAR,
        "With the damage hitmarker on, it shows green instead of red when the hit took your shield.",
    ),
    (
        "cg_shieldBrightness",
        "How bright the shield flash on the body is: 1 is the stock look, 2 the default, 12 the strongest. Not used by the sphere.",
    ),
    (
        crate::duel_ghost::CVAR,
        "On: players in a duel show as see-through blue figures, as on JoF EJK. Off: dark under a blue bubble, as in the base game.",
    ),
    (
        crate::duel_visibility::HIDE_OTHERS_CVAR,
        "In a duel, hides all but you and your opponent: bodies, sabers, shots, sounds, drones. Auto: JA+, jaPRO. Always: all. Off: dark.",
    ),
    (
        crate::duel_visibility::CORPSES_CVAR,
        "Hide dead bodies while you are in a private duel. Bodies remain visible outside duels. On by default.",
    ),
    (
        crate::duel_visibility::PICKUPS_CVAR,
        "Hide health, shield, weapons, ammo and holocrons while you are in a private duel. On by default.",
    ),
    #[cfg(feature = "mod-japlus")]
    (
        sjk_mod_japlus::anti_kiss::CVAR,
        "On JA+ servers, with JA+ tools on, use Force Push once when another player puts you in the amkiss victim animation. Off by default.",
    ),
    (
        crate::map_edits::CVAR,
        "Shows the SJK team's fixes to maps: props removed, models added and textures swapped. Only what you see changes. On by default.",
    ),
    (
        crate::map_edits::SYNC_COMMAND,
        "Asks the SJK hub for its latest map fixes now. They also come by themselves within seconds on a server.",
    ),
    (
        crate::remap_blocked_maps::SERVER_CVAR,
        "Lets servers swap the map's textures. Off by default; the map's own swaps and SJK's map fixes always apply.",
    ),
    (
        crate::remap_blocked_maps::SHARED_CVAR,
        "Ignores server texture swaps on the maps the SJK team lists. Your choice for this map comes first. On by default.",
    ),
    (
        crate::remap_blocked_maps::MAP_CVAR,
        "This map only: default follows Server remaps, on lets the server swap its textures anyway, off never.",
    ),
    (
        "cg_remaps",
        "With Server remaps on, which server swaps apply: never, all but player models, or always.",
    ),
    (
        crate::camera::STYLE_CVAR,
        "MP camera: the third-person camera stays locked behind you, as in JoF EJK. SP camera: it trails you a little as you move and turn.",
    ),
    (
        "cg_cameraFPS",
        "With SP camera, use EJK damping timing: Default 125, Smooth (SP) 30, or BaseJKA's original 50 ms steps.",
    ),
    (
        "cg_thirdPersonRange",
        "Distance of the third-person camera behind your character, from 40 to 500 units.",
    ),
    (
        "cg_thirdPersonAngle",
        "Turns the third-person camera around your character: 0 is directly behind.",
    ),
    (
        "cg_thirdPersonPitchOffset",
        "Raises or lowers the direction the third-person camera looks, in degrees.",
    ),
    (
        "cg_thirdPersonVertOffset",
        "Raises or lowers the third-person camera relative to your character.",
    ),
    (
        "cg_thirdPersonHorzOffset",
        "Moves the third-person camera sideways: negative left, positive right.",
    ),
    (
        "cg_thirdPersonCameraDamp",
        "How loosely the third-person camera follows you: lower is smoother, 0 holds it still, 1 sticks to you.",
    ),
    (
        "cg_thirdPersonTargetDamp",
        "How loosely the third-person camera turns where you aim: lower is smoother, 1 is instant.",
    ),
    (
        "cg_wallGrabLock",
        "Off: stock locks; JA+ lets you look and aim the kick-off. On locks JA+ too, preventing aim during the hold.",
    ),
    (
        "cg_predictLegacy",
        "Current follows the server. As in 2026.1028.1 undoes the prediction changes made since; some moves get corrected.",
    ),
    (
        crate::held_view::LOOK_CVAR,
        "On lets the mouse look around while a kick, knockdown, kiss or grapple holds your view; the camera may shake.",
    ),
    (
        "cg_errorDecay",
        "Smooths the view over this many milliseconds when the server corrects you; 0 snaps.",
    ),
    (
        "mod_japlus",
        "On JA+ servers: the japlus. commands (admin teleports, auto login, plugin options) and JA+ command completion.",
    ),
    (
        "mod_japro",
        "On jaPRO servers: shows the movement styles SJK does not predict yet (CPM, QW...) as the server has them; japro.styles lists them.",
    ),
    #[cfg(feature = "mod-japro")]
    (
        sjk_mod_japro::movement_styles::PREDICT_ANYWAY_CVAR,
        "Predict every jaPRO movement style as JKA, as before: smoother to aim, but styles SJK does not model snap you back.",
    ),
    (
        "mod_jof",
        "On JoF servers: jof.commands and completion of their own commands (gun admin commands, jetpack, emotes).",
    ),
    (
        "jof.downloads",
        "JoF's maps, skins, hilts and packs, as JoF's launcher gets them. Off until you turn it on; Enter opens the card.",
    ),
    (
        "sith.downloads",
        "The S community's maps, skins and extras, into the game's base folder. Off until you turn it on; Enter opens the card.",
    ),
    (
        "jof.contentAuto",
        "Off by default. With JoF content on, checks at start and every hour and fetches what changed in the groups marked Auto.",
    ),
    (
        "sith.cloudAuto",
        "Off by default. With the Sith cloud on, checks at start and every hour and fetches what changed in the groups marked Auto.",
    ),
    (
        "jof.contentSource",
        "Where JoF content downloads from: JoF Cloud, then jofhub.eu when a file is missing there (Auto), or only one of them.",
    ),
    (
        crate::ghost_replay::SHOWCASE_CVAR,
        "Showcase tools, for SJK staff.",
    ),
    (
        crate::brush_view::CVAR,
        "Shows invisible brushes (clip, trigger...) and movers (doors, lifts) see-through, as Radiant does: those of gameplay, or all.",
    ),
    (
        crate::entity_view::CVAR,
        "Marks the map's entities (lights, spawns, origins, items...) through walls with their names. A world note can pick them.",
    ),
    (
        crate::rarity_fx::CVAR,
        "Animates rarity: Rare frames breathe, Legendary ones catch a sheen, Mythical ones a running spark. Off keeps the tier frames still.",
    ),
    (
        crate::menu::style::CVAR,
        "The SJK UI, SJK's own menus over the live map; or classic menus after the original game's.",
    ),
    (
        crate::launcher_notice::CVAR,
        "Without the SJK Launcher on this computer, the SJK UI's main page says it is out, with a link to it. Off hides it, as its x does.",
    ),
    (
        super::slider_peek::CVAR,
        "In the game, hold a slider in the SJK UI a moment and the rest of Settings fades away, showing the game as you drag. Off keeps it.",
    ),
    (
        crate::menu_pictures::CVAR,
        "Map pictures behind the menus and drawn saber shaders, not live ones: saves video memory. On by itself on small cards.",
    ),
    (
        crate::quick_wheel::pages::FILE,
        "The quick wheel's pages (+wheel) and the choices on each, kept in wheel.json. Enter edits them.",
    ),
    (
        crate::quick_wheel::SOUNDS_CVAR,
        "The game's menu sounds as the quick wheel changes page, moves to another choice and runs one.",
    ),
    // NETWORK
    (
        "cl_master",
        "The master server the server browser asks for its list of servers.",
    ),
    (
        "cl_master2",
        "A second master server, asked at the same time; servers either one knows are listed. Empty asks only the first.",
    ),
    (
        "cl_autoUpdate",
        "Looks for a newer SJK release at start and every 30 minutes while SJK runs, and says so with a card. Install it from the Update page.",
    ),
    (
        "ui_hideFirstSetup",
        "Ticked, First setup no longer opens when SJK starts. Settings and the firstsetup command still open it.",
    ),
    (
        "cl_identity",
        "Makes an identity key and tells the SJK hub where you play and if you are active (badge, holocrons). Off sends nothing.",
    ),
    (
        crate::installed_models::CVAR,
        "Tells the SJK hub which player models you have, so SJK players' cards say if they see yours. Off makes it forget.",
    ),
    (
        "cl_hubUrl",
        "The SJK hub's address (https://...). Empty means no hub: nothing is sent.",
    ),
    (
        crate::crash_report::CVAR,
        "After a crash, the next start sends the SJK team a report: system, map, the error, recent console lines (no chat). Off sends none.",
    ),
    (
        crate::discord_presence::CVAR,
        "Shows on your Discord profile what you do in SJK: menus, server, map, mode, players. Server hides its name.",
    ),
    (
        crate::settings_usage::CVAR,
        "Tells the SJK team anonymously what you change and search for here, to make Settings clearer. No key or name sent.",
    ),
    (
        "cl_sjkChat",
        "The chat every SJK player shares through the SJK hub, in menus and games. Off hides it.",
    ),
    (
        crate::sjk_chat_frame::FEED_CVAR,
        "Shows live SJK chat messages in the in-game general chat feed. Off still leaves SJK chat in the ESC menu and on its own page.",
    ),
    (
        crate::sjk_chat_frame::CONSOLE_CVAR,
        "Keeps live SJK chat messages in console scrollback. Off does not hide SJK chat anywhere else.",
    ),
    (
        crate::chat_sounds::MASTER_CVAR,
        "Every chat sound. Distinct (EJK) gives team chat and private messages EJK's own sounds when theirs is Default.",
    ),
    (
        "cg_chatBeep",
        "A sound for public chat on game servers. Private messages have their own switch below.",
    ),
    (
        "cl_chatSoundPublic",
        "The sound public chat plays; Default is JKA's chat sound. Choosing one plays it.",
    ),
    (
        "chatsoundpreview public",
        "Plays the sound public chat plays now, even with its switch off.",
    ),
    ("cg_teamChatBeep", "A sound for team chat on game servers."),
    (
        "cl_chatSoundTeam",
        "The sound team chat plays; Default is JKA's chat sound (EJK's team beep with Distinct).",
    ),
    (
        "chatsoundpreview team",
        "Plays the sound team chat plays now, even with its switch off.",
    ),
    (
        crate::chat_sounds::Kind::Private.switch_cvar(),
        "A sound for private messages: a server's tells (JA+, JoF and others) and encrypted whispers.",
    ),
    (
        "cl_chatSoundPrivate",
        "The sound private messages play; Default is JKA's chat sound for a tell, the SJK whisper sound for an encrypted whisper.",
    ),
    (
        "chatsoundpreview private",
        "Plays the sound private messages play now, even with their switch off.",
    ),
    (
        crate::sjk_chat_frame::SOUND_CVAR,
        "A sound for each new SJK chat message, yours included, in menus and games.",
    ),
    (
        "cl_chatSoundSjk",
        "The sound SJK chat messages play; Default is SJK's own variant of JKA's chat sound.",
    ),
    (
        "chatsoundpreview sjk",
        "Plays the sound SJK chat messages play now, even with their switch off.",
    ),
    (
        crate::chat_sounds::FLASH_CVAR,
        "Flashes the game's taskbar button when a private message arrives while you are away from the window.",
    ),
    (
        crate::player_trades::SIZE_CVAR,
        "How big the trade window is in the middle of the screen. Small leaves the most of the match in view.",
    ),
    (
        crate::holocron_popup::NOTIFICATIONS_CVAR,
        "A card for each holocron found in play; After duels holds it in a private duel until the end. The holocron waits in ESC.",
    ),
    (
        crate::chat::client_icons::CVAR,
        "SJK chat messages sent from JoF EternalJK start with its icon. Off, every message starts with SJK's emblem.",
    ),
    (
        crate::chat_gifs::CVAR,
        "GIPHY links in SJK chat show their GIF. Your PC loads it from GIPHY, which sees your address. Off keeps links as text.",
    ),
    (
        crate::chat_gifs::VIDEO_CVAR,
        "Off by default. MP4 clips from the internet are read by Windows' decoder, outside SJK's safe Rust: a crafted clip could exploit it.",
    ),
    (
        crate::chat_gifs::search::KEY_CVAR,
        "Your own GIPHY key, which GIF search needs: free at developers.giphy.com. The GIF picker's Search shows how.",
    ),
    (
        crate::e2e_whispers::CVAR,
        "Whispers to SJK players go end-to-end encrypted through the SJK hub: only you two can read them. Off sends them through the server.",
    ),
    (
        crate::sound_mutes::IGNORE_CVAR,
        "A JA+ or jaPRO ignore <player> also mutes their sounds until the map changes; ignore again hears them.",
    ),
    (
        crate::map_blocks::CVAR,
        "Players SJK staff block for everyone show as a silent BLOCKED Kyle until the map changes. Off shows them as usual.",
    ),
    (
        super::catalog::IDENTITY_ROW,
        "Your SJK key: switch it on, back it up, copy its id. Enter opens it; the key stays hidden until Show.",
    ),
    (
        super::catalog::IMPORT_ROW,
        "Bring name, model, FOV, FPS and keys from Base, EJK, TaystJK or any .cfg. Enter shows the configs found.",
    ),
    (
        "assetbrowser",
        "Enable or disable installed PK3 packs for the next client start. Retail base packs stay enabled; files stay in place.",
    ),
    (
        "rate",
        "Most data per second the server may send you; raise it on a fast connection.",
    ),
    (
        "snaps",
        "World updates per second asked of the server; it sends at most its own rate.",
    ),
    // HUD+
    ("cg_crosshairSize", "Size of the crosshair."),
    ("cg_speedometer", "Shows how fast you move."),
    (
        crate::hud::clock::CVAR,
        "Shows your PC's local time, the time and date, or the date on the HUD. Move it in the HUD editor or with cg_clockX, Y and Size.",
    ),
    (
        crate::hud::clock::HOURS_CVAR,
        "Counts the HUD clock's hours from 1 to 12 with am and pm, instead of 0 to 23.",
    ),
    (
        crate::hud::movement_keys::CVAR,
        "JoF EJK's movement keys on screen, each lit while you hold it. Place them with cg_movementKeysX, Y and Size.",
    ),
    (
        crate::hud::movement::LINES_ROW,
        "JoF EJK's strafe helper: a line to the best heading for each strafe key. Keep the crosshair on the green one to gain speed.",
    ),
    (
        crate::hud::movement::AURA_CVAR,
        "A glow over the crosshair while you jump or fall: the speed each heading gains. Turn until its top tick sits on the white edge.",
    ),
    (
        crate::hud::movement::AURA_GROUND_CVAR,
        "Shows the strafe aura while you run on the ground too, not only in the air.",
    ),
    (
        crate::scoreboard::style::CVAR,
        "Auto matches the menus: SJK's own with the SJK UI, else the classic one after the original game's.",
    ),
    (
        crate::scoreboard::style::COMPACT_CVAR,
        "Thinner rows on SJK's scoreboard, so every player fits in one column, in a board as wide as its names, centred.",
    ),
    (
        crate::scoreboard::style::FPS_CVAR,
        "Shows your frame rate on SJK's scoreboard, left of the match clock.",
    ),
    (
        crate::scoreboard::AFK_CVAR,
        "Players whose view has not moved for 3 minutes, or with [AFK] in their name, show faint below the others.",
    ),
    (
        crate::scoreboard::style::IDS_CVAR,
        "Shows each player's client number on the scoreboard.",
    ),
    (
        crate::scoreboard::style::SJK_IDS_CVAR,
        "Shows each player's client number on SJK's scoreboard, small before the name.",
    ),
    (
        crate::scoreboard::style::DEATHS_RANK_CVAR,
        "Lists players by score minus deaths where deaths show, fewer deaths first on a tie. Off keeps the server's order.",
    ),
    (
        crate::jof_tag::CVAR,
        "Shows a clan's emblem (JoF, JoFA, JAWA, {{S}}) left of a tagged player's name in the chat, on the scoreboard and on player cards.",
    ),
    (
        "cg_drawScoreboardIcons",
        "Shows each player's head icon on the scoreboard.",
    ),
    (
        "cg_smallScoreboard",
        "Smaller scoreboard rows, so more players fit.",
    ),
    // TEXT
    (
        crate::console::console_options::STYLE_CVAR,
        "The console's look: the SJK UI's with its menus, or classic, after EternalJK's.",
    ),
    (crate::text::style::SCALE_CVAR, "Size of the menu text."),
    (
        crate::text::style::TRACKING_CVAR,
        "Extra space between letters in the menus and the console.",
    ),
    ("con_scale", "Size of the console text."),
    (
        crate::console::console_options::CHAT_ONLY_CVAR,
        "The open console lists only chat, so a conversation is easy to follow. Every line is kept: switch it off to see them all.",
    ),
    (
        crate::console::console_options::DRAW_NOTIFY_CVAR,
        "Shows the newest console lines at the top left while playing. Chat keeps its own box.",
    ),
    (
        crate::console::console_options::COPY_COLORS_CVAR,
        "Keeps ^0-^9 colour codes in text copied from console scrollback.",
    ),
    (
        crate::console::console_options::SHIFT_HEIGHT_CVAR,
        "Shift+console opens at Console height, Shift+Escape a quarter screen. Off: as in EternalJK.",
    ),
    (
        crate::console::console_options::OVER_CHAT_CVAR,
        "The console key opens the console over the chat; your message waits. Off: the chat takes the key.",
    ),
    (
        crate::text::charset::CVAR,
        "Chat and names of EternalJK and retail players: Auto spots Russian, Cyrillic always reads it as Russian, Western never.",
    ),
    // IMAGE
    (
        "r_sceneHdr",
        "Renders the scene in high dynamic range: brighter lights, smoother bright areas.",
    ),
    (
        "r_hdrExposure",
        "How bright the scene is drawn; eye adaptation works around this value.",
    ),
    (
        crate::frame_target::aa::exposure::ENABLED,
        "Brightness follows what you look at, as eyes adapt between dark rooms and bright sky.",
    ),
    (
        crate::frame_target::aa::exposure::MIN_EV,
        "How far eye adaptation may darken a bright view, in exposure steps (HDR only).",
    ),
    (
        crate::frame_target::aa::exposure::MAX_EV,
        "How far eye adaptation may brighten a dark view, in exposure steps.",
    ),
    (
        "r_toneCurve",
        "A filmic curve that softens the brightest parts of the picture.",
    ),
    (
        "r_sceneBloom",
        "Bright lights bleed a soft glow around them. The HUD and menus never bloom.",
    ),
    (
        "r_DynamicGlow",
        "The glow around sabers and glowing surfaces: on everything that glows, or on sabers only.",
    ),
    (
        crate::frame_target::aa::glow::NO_SABERS_CVAR,
        "Saber blades and trails get no glow halo, so each saber's details show; the rest still glows.",
    ),
    (
        "r_dynamicGlowStyle",
        "How the glow is blurred: like the original renderer, or like EternalJK's Vulkan one.",
    ),
    ("r_fxaa", "Smooths jagged edges with a light filter."),
    (
        crate::frame_target::aa::color::SHARPEN_CVAR,
        "Sharpens the world against the blur of edge smoothing and textures, without halos.          0 is off. Menus and HUD are not touched.",
    ),
    (
        "r_superSample",
        "Draws the scene 2 or 3 times larger each way and shrinks it: very sharp, very costly.",
    ),
    (
        crate::frame_target::scale::CVAR,
        "Draws the world at this % of the window and stretches it, for weak graphics cards. \
         Menus and HUD stay sharp. Off with supersampling.",
    ),
    (
        crate::world_materials::filtering::mips::PICMIP_CVAR,
        "Each step halves large textures: a quarter of their video memory. For graphics cards of 4 GB or less.",
    ),
    (
        crate::world_materials::filtering::mips::MAX_SIZE_CVAR,
        "Halves larger textures down to this size: a quarter of their video memory each time. 0 keeps every size.",
    ),
    (
        crate::world_materials::filtering::mips::compress::CVAR,
        "Keeps map and model textures compressed: a quarter or less of their video memory, a little blockier. Experimental.",
    ),
    (
        "r_flares",
        "Halos the maps put around some lamps, which fade as walls hide them. EternalJK has them off.",
    ),
    (
        "r_softParticles",
        "Smoke and dust fade where they meet walls instead of cutting into them.",
    ),
    (
        "r_ssao",
        "Darkens corners and creases of the map's walls and floors a little.",
    ),
    (
        crate::dust_motes::CVAR,
        "Dust drifting in sunbeams; needs light shafts.",
    ),
    (
        crate::weather::CVAR,
        "The rain, snow and mist a map asks for. It stays under open sky: roofs keep it out.",
    ),
    (
        crate::weather::DENSITY_CVAR,
        "How many weather particles: 1 as many as the original game, 2 twice as many.",
    ),
    (
        crate::weather::QUALITY_CVAR,
        "Medium adds splashes, 3D fog and wet ground; High far rain, spray and water on walls; \
         Ultra puddles and reflections.",
    ),
    (
        crate::weather::FORCE_CVAR,
        "Weather on every map with sky: the map's own, or drizzle, rain, a storm or snow everywhere.",
    ),
    (
        crate::weather::FOG_CVAR,
        "Fog lying on the ground outdoors: never, where the map has fog, or on every map \
         with sky. Works with rain and forced weather.",
    ),
    (
        "r_drawfog",
        "The map's own fog: pits and misty rooms, and its haze. Off hides it all; Original          draws it as the game does.",
    ),
    (
        crate::weather::CLOUDS_CVAR,
        "Drifting 3D clouds over open sky, lit by the map's sun; darker and thicker in a storm.",
    ),
    (
        crate::weather::LIGHTNING_CVAR,
        "Storms flash now and then, lighting the rain, fog and wet ground; thunder follows with Rain and wind sounds on.",
    ),
    (
        crate::weather::REFLECTIONS_CVAR,
        "What wet streets, walls and puddles mirror: the sky's colour, or the buildings and \
         lights round them (Ultra: everything).",
    ),
    (
        crate::weather::PUDDLES_CVAR,
        "How much water stands on flat ground in the rain at Ultra quality: 0 none, 1 some, \
         2 flooded streets.",
    ),
    (
        crate::weather::RAIN_OPACITY_CVAR,
        "How clearly the falling rain shows: lower for a clearer view in a fight, higher for a \
         heavier downpour.",
    ),
    (
        crate::weather::HAZE_CVAR,
        "How much falling rain and snow grey the distance, and the spray a downpour raises over \
         the ground. 0 keeps the air clear.",
    ),
    (
        crate::weather::FOG_DENSITY_CVAR,
        "How thick the fog lying on the ground is, where Ground fog shows it.",
    ),
    (
        crate::weather::SNOW_COVER_CVAR,
        "Falling snow settles on the ground, roofs and ledges under open sky, deeper in drifts \
         and against walls.",
    ),
    (
        "r_modelPixelLight",
        "Lights models at every pixel rather than once each, so they match the light around them.",
    ),
    (
        crate::dynamic_lights::TOON_MODELS_CVAR,
        "Shows the cartoon shading players chose for their own model: soft light bands and a bright rim. Off, every model stays stock.",
    ),
    (
        crate::toon_choice::CVAR,
        "Your model in cartoon-style shading, as SJK players who show toon models see it. Also on the Profile's Character tab.",
    ),
    (
        "r_cubeMapping",
        "Captures each map's surroundings so metal and shiny surfaces reflect their room.",
    ),
    (
        "r_floorReflections",
        "Mirror-like reflections on polished floors.",
    ),
    (
        "r_normalMapping",
        "Surface relief from a material pack's normal maps: bricks and stone catch the light.",
    ),
    (
        "r_specularMapping",
        "Shine from a material pack's specular maps: metal and wet stone glint.",
    ),
    (
        "r_parallaxMapping",
        "Gives stone, tiles and sand real-looking depth from their height maps.",
    ),
    (
        "r_parallaxStrength",
        "How deep parallax looks: 0 flat, 1 the full depth of the maps; 0.1 by default.",
    ),
    (
        "r_emissiveMaps",
        "Emission maps, which make lamps, screens and signs glow on their own.",
    ),
    (
        "r_emissionStrength",
        "Brightness of emission maps: 1 as made, 0 off.",
    ),
    (
        "r_emissiveGlow",
        "A glow halo around emission-mapped surfaces; needs dynamic glow.",
    ),
    // LIGHTING
    (
        "r_dayNight",
        "The Solar Renderer: a live sun, sky and lamps light the map in place of its baked light.",
    ),
    (
        crate::world_materials::shadows::settings::BOUNCE_CVAR,
        "With real-time lighting, light bounces off walls and floors into the shade. Off saves a pass.",
    ),
    (
        crate::world_materials::shadows::settings::LAMP_SHADOWS_CVAR,
        "With real-time lighting, the nearest lamps cast shadows of walls and players.",
    ),
    (
        crate::world_materials::shadows::day::live::SKY_CVAR,
        "Without real-time lighting, the sky still shows the hour: dawn, dusk, and darker nights.",
    ),
    (
        "r_liveLighting",
        "How much light is computed live: 0 keeps the map's baked bounce light, 2 computes it all.",
    ),
    (
        "r_dayHour",
        "The hour of the sun, 0 to 24, with real-time lighting or the time-of-day sky.",
    ),
    (
        "r_dayClock",
        "Day and night follow your PC's clock; without real-time lighting the nights are darker.",
    ),
    (
        "r_dayMinutes",
        "How long a whole day lasts in real time, or the hour holds still.",
    ),
    (
        "r_dayBrightness",
        "Brightness of the live lighting, relative to the sun.",
    ),
    (
        "r_ambientFill",
        "A faint light in very dark places so they stay readable; 0 is off.",
    ),
    (
        "r_ambientFillOcclusion",
        "How much that fill darkens in corners: 1 as designed, 0 not at all.",
    ),
    (
        "r_indirectBoost",
        "Brightness of bounced and sky light; direct light stays as it is. 1 as designed.",
    ),
    (
        crate::world_materials::material_maps::lights::CVAR,
        "Lets emission-mapped surfaces light their surroundings in live lighting; 0 is off.",
    ),
    (
        crate::world_materials::emission::lamp_gain::CVAR,
        "How far lamps that only glow, and flares, cast their colour in live lighting; 0 is off.",
    ),
    (
        "r_volumetrics",
        "Light shafts through the air; each level up is finer and costs more.",
    ),
    (
        "r_volumetricClarity",
        "1 keeps sunlit air clear and shows only the shafts; 0 adds an even haze.",
    ),
    // SHADOWS
    (
        "r_worldSunShadows",
        "The sun casts shadows from walls and buildings.",
    ),
    (
        "r_actorSunShadows",
        "Players and NPCs cast sun shadows too.",
    ),
    (
        "r_sunShadowResolution",
        "Detail of the sun's shadows: higher is sharper and takes more video memory.",
    ),
    (
        "r_sunShadowDistance",
        "How far from you sun shadows stay sharp; a coarser map covers the rest of the level.",
    ),
    (
        "r_sunShadowNear",
        "Range of the finest shadows, close around you.",
    ),
    (
        "r_sunShadowTaps",
        "How soft a shadow's edge is: softer takes more samples and costs more.",
    ),
    (
        "r_sunShadowGapClose",
        "Fills light leaking through slits narrower than this many units; 0 is off.",
    ),
    (
        "r_contactShadows",
        "Small shadows where things touch the ground, toward the sun. Still being tuned.",
    ),
];

#[cfg(test)]
mod tests {
    use super::super::catalog::*;
    use super::*;

    fn every_row() -> impl Iterator<Item = &'static Setting> {
        let general = (0..TABS.len()).flat_map(super::super::settings);
        let renderer = [
            RENDER_IMAGE,
            RENDER_LIGHTING,
            RENDER_SHADOWS,
            RENDER_WEATHER,
            // Rows only a search shows (they live on the Collection's Hax card).
            HAX_TOY,
        ]
        .into_iter()
        .flatten();
        general.chain(renderer)
    }

    #[test]
    fn every_setting_says_what_it_does_in_two_lines() {
        for setting in every_row() {
            let text =
                help(setting.cvar).unwrap_or_else(|| panic!("{} has no description", setting.cvar));
            let lines = lines(text);
            assert!(
                lines
                    .iter()
                    .all(|line| line.chars().count() <= HELP_LINE_CHARS),
                "{}: {lines:?}",
                setting.cvar
            );
            assert!(
                text.is_ascii(),
                "{}: the menu font draws bytes",
                setting.cvar
            );
        }
        // And nothing describes a setting no row shows.
        for (cvar, _) in HELP {
            assert!(
                every_row().any(|setting| setting.cvar.eq_ignore_ascii_case(cvar)),
                "{cvar} is described but not on any page"
            );
        }
    }

    /// Longest row label, in characters, that fits the in-game pop-up's
    /// label column.
    const ROW_LABEL_CHARS: usize = 23;

    #[test]
    fn every_row_label_fits_the_label_column() {
        for setting in every_row() {
            let (label, _) = classic_label(setting.cvar, setting.label);
            assert!(
                label.chars().count() <= ROW_LABEL_CHARS,
                "{}: {label}",
                setting.cvar
            );
        }
        for (cvar, _) in SHORT_LABELS {
            assert!(
                every_row().any(|setting| setting.cvar.eq_ignore_ascii_case(cvar)),
                "{cvar} has a short label but no row"
            );
        }
        // A short name keeps the timing of the full label.
        assert_eq!(
            classic_label("cg_errorDecay", "Prediction error smoothing (restart)"),
            ("Error smoothing", Timing::Restart)
        );
    }

    #[test]
    fn lines_break_at_spaces() {
        let text = "Brightness curve of the whole display, HUD included. 1 is neutral; higher lifts dark areas.";
        let [first, second] = lines(text);
        assert!(first.len() <= HELP_LINE_CHARS && !first.ends_with(' '));
        assert_eq!(format!("{first} {second}"), text);
        assert_eq!(lines("Short."), ["Short.", ""]);
    }

    #[test]
    fn labels_lose_their_timing_note() {
        assert_eq!(timing("HDR scene (reload)"), ("HDR scene", Timing::Reload));
        assert_eq!(
            timing("Live lighting 0-2 (next map)"),
            ("Live lighting 0-2", Timing::NextMap)
        );
        assert_eq!(timing("Bloom"), ("Bloom", Timing::Now));
        assert_eq!(
            timing("Dynamic glow (0 off, 2 sabers)"),
            ("Dynamic glow (0 off, 2 sabers)", Timing::Now)
        );
        // Rows also drop a note in brackets.
        assert_eq!(
            row_label("Dynamic glow (0 off, 2 sabers)"),
            ("Dynamic glow", Timing::Now)
        );
        assert_eq!(
            row_label("Light shafts (reload)"),
            ("Light shafts", Timing::Reload)
        );
        assert_eq!(
            row_label("Adaptation: max darken, EV (HDR)"),
            ("Adaptation: max darken, EV", Timing::Now)
        );
        assert_eq!(row_label("FPS cap (AUTO, 0 = off)").0, "FPS cap");
        assert_eq!(
            row_label("Supersampling, 1 off (reload)").0,
            "Supersampling, 1 off"
        );
    }

    #[test]
    fn camera_damping_styles_have_their_ejk_names() {
        assert_eq!(choice_name("cg_cameraFPS", "125.0"), Some("Default"));
        assert_eq!(choice_name("cg_cameraFPS", "30.0"), Some("Smooth (SP)"));
        assert_eq!(choice_name("cg_cameraFPS", "0.0"), Some("BaseJKA"));
    }
}
