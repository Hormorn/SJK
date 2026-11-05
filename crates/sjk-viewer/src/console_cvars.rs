//! Daily-use cvar definitions kept separate from console interaction logic.

use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// `cl_maxpackets` default: one packet per user command (125 a second), as
/// JoF EJK. Stock's 30 and EternalJK's 63 batch two to four commands a packet.
pub(crate) const DEFAULT_MAX_PACKETS: i64 = 125;

/// `rate` default: 90000 bytes a second, so the 40 snapshots of a busy server are
/// not delayed by the client's own rate; servers cap it with `sv_maxRate`. Stock's
/// 25000 left about 600 bytes a snapshot at 40 a second.
pub(crate) const DEFAULT_RATE: i64 = 90_000;
/// The stock default every profile saved, moved once to [`DEFAULT_RATE`].
pub(crate) const OLD_DEFAULT_RATE: i64 = 25_000;
/// Marks that move as done.
pub(crate) const RATE_VERSION_CVAR: &str = "rateDefaultVersion";

/// `name`'s value as a number, whole or not, for the one-time moves of saved values.
pub(crate) fn saved_number(cvars: &CvarRegistry, name: &str) -> Option<f64> {
    match cvars.get(name).map(|cvar| &cvar.value) {
        Some(sjk_shell::CvarValue::Float(value)) => Some(*value),
        Some(sjk_shell::CvarValue::Integer(value)) => Some(*value as f64),
        _ => None,
    }
}

/// Change-callback cache for integer values; frame reads never look up a name.
pub(super) struct IntegerSetting(Arc<AtomicI64>);

impl IntegerSetting {
    /// Bind before config loading so archived values apply.
    pub(super) fn bind(
        cvars: &mut CvarRegistry,
        name: &str,
        default: i64,
    ) -> Result<Self, sjk_shell::CvarError> {
        let value = Arc::new(AtomicI64::new(default));
        let changed = Arc::clone(&value);
        // An on/off cvar (`cg_serverRemaps`, `cg_mapEdits`) is 0 or 1 here.
        cvars.on_change(name, move |change| match change.current {
            sjk_shell::CvarValue::Integer(value) => changed.store(value, Ordering::Relaxed),
            sjk_shell::CvarValue::Bool(on) => changed.store(i64::from(on), Ordering::Relaxed),
            _ => {}
        })?;
        Ok(Self(value))
    }

    /// Read the nonzero state without allocation or locking.
    pub(super) fn enabled(&self) -> bool {
        self.value() != 0
    }

    /// Read the integer value without allocation or locking.
    pub(super) fn value(&self) -> i64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Change counter for a cvar that frame code reads again only after it changes.
pub(super) struct RevisionSetting(Arc<AtomicU64>);

impl RevisionSetting {
    /// Bind before config loading so an archived value counts as a change.
    pub(super) fn bind(cvars: &mut CvarRegistry, name: &str) -> Result<Self, sjk_shell::CvarError> {
        let revision = Arc::new(AtomicU64::new(0));
        let changed = Arc::clone(&revision);
        cvars.on_change(name, move |_| {
            changed.fetch_add(1, Ordering::Relaxed);
        })?;
        Ok(Self(revision))
    }

    /// Read the change count without allocation or locking.
    pub(super) fn value(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

pub(super) fn register_daily_cvars(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    crate::frame_target::aa::register(cvars)?;
    crate::frame_target::scale::register(cvars)?;
    crate::world_materials::filtering::register(cvars)?;
    crate::world_materials::material_maps::register(cvars)?;
    crate::world_materials::emission::lamp_gain::register(cvars)?;
    crate::scene_views::register_floor_reflections(cvars)?;
    crate::assets::search_paths::register(cvars)?;
    #[cfg(feature = "mod-jof")]
    crate::jof_content::register(cvars)?;
    #[cfg(feature = "mod-jof")]
    crate::sith_cloud::register(cvars)?;
    super::client_options::register(cvars)?;
    super::ui_options::register(cvars)?;
    crate::hud::icons::register(cvars)?;
    crate::text::style::register(cvars)?;
    crate::text::charset::register(cvars)?;
    crate::menu_hud::register(cvars)?;
    crate::chat_sounds::register(cvars)?;
    let archive = CvarFlags::ARCHIVE;
    let definitions = [
        CvarDefinition::new(
            crate::remap_blocked_maps::SERVER_CVAR,
            false,
            archive,
            "Let servers swap the map's textures (shader remaps); off by default, cg_remaps filters them when on",
        ),
        CvarDefinition::new(
            crate::map_edits::CVAR,
            true,
            archive,
            "Draw SJK's map edits: props hidden, models placed and textures swapped by the SJK team (on by default)",
        ),
        CvarDefinition::new(
            "cg_remaps",
            2_i64,
            archive,
            "Server shader remaps while cg_serverRemaps is on: 0 off, 1 skip player textures, 2 all (default, as EternalJK)",
        ),
        CvarDefinition::new(
            crate::fps_help::CVAR,
            true,
            archive,
            "Offer Vanilla graphics when a map's SJK lighting keeps the FPS low (help_fps shows it)",
        ),
        CvarDefinition::new(
            crate::graphics_quality::RESTORE_CVAR,
            "",
            archive,
            "The SJK renderer settings put aside while the renderer is Vanilla, restored when the SJK renderer is turned on again",
        ),
        CvarDefinition::new(
            crate::remap_blocked_maps::CVAR,
            "",
            archive,
            "Maps whose server shader remaps are ignored, as with cg_remaps 0: mp/ffa4 mp/duel6",
        ),
        CvarDefinition::new(
            crate::remap_blocked_maps::ALLOWED_CVAR,
            "",
            archive,
            "Maps whose server shader remaps apply even with cg_serverRemaps 0: mp/ffa4 mp/duel6",
        ),
        CvarDefinition::new(
            crate::remap_blocked_maps::MAP_CVAR,
            crate::remap_blocked_maps::Choice::NAMES[0],
            CvarFlags::NONE,
            "Server shader remaps on the loaded map: default (as cg_serverRemaps), on or off; kept in cg_remapsAllowedMaps and cg_remapsBlockedMaps",
        ),
        CvarDefinition::new(
            "cg_forceEnemyModel",
            "none",
            archive,
            "Enemy model with cg_forceModel",
        ),
        CvarDefinition::new(
            "cg_forceAllyModel",
            "none",
            archive,
            "Ally model with cg_forceModel",
        ),
        CvarDefinition::new(
            "cg_forceOwnSaber",
            "none",
            CvarFlags::NONE,
            "Local presentation hilt override(s)",
        ),
        CvarDefinition::new(
            "cg_scorePlums",
            1_i64,
            archive,
            "Server-authored floating numbers",
        ),
        CvarDefinition::new(
            "cg_killMessage",
            1_i64,
            archive,
            "Personal kill announcement mode",
        ),
        CvarDefinition::new(
            "cg_duelSounds",
            1_i64,
            archive,
            "Duel: 0 off, 1 both, 2 sound, 3 text",
        ),
        CvarDefinition::new(
            "cg_scoreDeaths",
            1_i64,
            archive,
            "Scoreboard deaths: 0 off, 1 server, 2 server/local fallback, 3 local",
        ),
        CvarDefinition::new(
            "cg_hitsounds",
            0_i64,
            archive,
            "A sound when your attack hits a player: 0 off, 1-4 four sounds (a pack's hitsound*.wav, else SJK's); 5-6 saber hit variants",
        ),
        CvarDefinition::new(
            "cg_oldPainSounds",
            false,
            archive,
            "Local pain from health loss",
        ),
        CvarDefinition::new(
            "cg_killSounds",
            2_i64,
            archive,
            "Kill cue: 0 off, 1 normal, 2 midair",
        ),
        CvarDefinition::new(
            crate::audio::music::CVAR,
            true,
            archive,
            "Resume file-based map music using level time; dynamic music keeps its own transitions",
        ),
        CvarDefinition::new(
            "cg_jumpSounds",
            0_i64,
            archive,
            "Jump voices: 0 off, 1 all, 2 others, 3 self",
        ),
        CvarDefinition::new(
            "cg_rollSounds",
            1_i64,
            archive,
            "Roll voices: 0 off, 1 all, 2 others, 3 self",
        ),
        CvarDefinition::new(
            "cg_tauntAntiSpam",
            true,
            archive,
            "Limit repeated model voices; taunts wait for the previous sample to finish",
        ),
        CvarDefinition::new(
            "cl_allowDownload",
            1_i64,
            archive,
            "Allow verified UDP pak downloads",
        ),
        CvarDefinition::new("developer", 0_i64, CvarFlags::NONE, "Debug server commands"),
        CvarDefinition::new(
            "pmove_fixed",
            0_i64,
            CvarFlags::READ_ONLY,
            "Server fixed-step movement",
        ),
        CvarDefinition::new(
            "pmove_msec",
            8_i64,
            CvarFlags::READ_ONLY,
            "Server movement step (8..33 ms)",
        ),
        CvarDefinition::new(
            "pmove_float",
            0_i64,
            CvarFlags::READ_ONLY,
            "Server unsnapped velocity",
        ),
        CvarDefinition::new(
            "g_stepSlideFix",
            0_i64,
            CvarFlags::READ_ONLY,
            "Server step-slide correction",
        ),
        CvarDefinition::new("r_resolution", "1280x720", archive, "Window resolution"),
        // Stock r_fullscreen: fullscreen on/off (Alt+Enter toggles it). Which
        // kind of fullscreen is SJK's choice; see settings/display.rs.
        CvarDefinition::new("r_fullscreen", false, archive, "Fullscreen (0/1)"),
        CvarDefinition::new(
            crate::settings::EXCLUSIVE_CVAR,
            false,
            archive,
            "Fullscreen kind: 0 borderless at the desktop size, 1 exclusive video mode at r_resolution",
        ),
        CvarDefinition::new(
            "r_swapInterval",
            false,
            archive,
            "Synchronize presentation (0/1)",
        ),
        CvarDefinition::new(
            crate::menu::style::CVAR,
            crate::menu::style::MenuStyle::DEFAULT_NAME,
            archive,
            "Menu layout: sjk (the SJK UI) or classic (after the original Jedi Academy menus)",
        ),
        CvarDefinition::new(
            "r_gamma",
            1.0_f64,
            archive,
            "Display gamma (0.5–3.0, neutral 1); applies immediately, including HUD",
        ),
        CvarDefinition::new("cg_fov", 90.0_f64, archive, "Horizontal field of view"),
        CvarDefinition::new(
            "cg_fovAspectAdjust",
            true,
            archive,
            "Widen the field of view for widescreen displays",
        ),
        CvarDefinition::new(
            "cl_showTimeDelta",
            // Default off: this was raised to 1 during the step-74 netcode investigation and
            // never lowered, so every connected session spammed the owner's console with a
            // `nettiming:` line every five seconds. Set it to 1 to bring the windows back.
            0_i64,
            CvarFlags::NONE,
            "Log aggregate snapshot and prediction timing every five seconds while connected",
        ),
        CvarDefinition::new(
            "cg_smoothClients",
            0_i64,
            archive,
            "Extrapolate remote players and NPCs from snapshot velocity",
        ),
        CvarDefinition::new(
            crate::duel_visibility::HIDE_OTHERS_CVAR,
            1_i64,
            archive,
            "Hide players outside your private duel: 0 never, 1 where the server expects it, 2 always (their drones: unless 0)",
        ),
        CvarDefinition::new(
            crate::duel_visibility::CORPSES_CVAR,
            1_i64,
            archive,
            "Hide dead bodies only while you are in a private duel",
        ),
        CvarDefinition::new(
            crate::duel_visibility::PICKUPS_CVAR,
            1_i64,
            archive,
            "Hide pickups only while you are in a private duel",
        ),
        CvarDefinition::new(
            "cg_wallGrabLock",
            false,
            archive,
            "Lock wall-grab look even when the server allows free look; off follows the server profile",
        ),
        CvarDefinition::new(
            "cg_predictLegacy",
            0_i64,
            archive,
            "Prediction changes since 2026.1028.1 turned back off, one bit each (255 all): 1 saber files, 2 saber-lock facing, 4 respawn jump, 8 trigger box, 16 gesture, 32 held, 64 other players, 128 history gap",
        ),
        // EternalJK's: write prediction misses to the log (`cg_predict.c:1534`).
        CvarDefinition::new(
            "cg_showMiss",
            0_i64,
            CvarFlags::NONE,
            "1 writes each large prediction miss to the console and the log (at most 4 a second)",
        ),
        CvarDefinition::new(
            "cg_errorDecay",
            100.0_f64,
            archive,
            "Milliseconds over which a prediction miss is smoothed out of the view; 0 snaps",
        ),
        CvarDefinition::new(
            "cg_hudScale",
            0.7_f64,
            archive,
            "Size multiplier for the HUD (0.25 to 2)",
        ),
        CvarDefinition::new(
            crate::hud::RadialPlacement::SPREAD_CVAR,
            0.0_f64,
            archive,
            "SJK radial HUD: how much farther from the crosshair its meters sit on each side, in HUD pixels (0 to 250)",
        ),
        CvarDefinition::new(
            crate::hud::RadialPlacement::HEIGHT_CVAR,
            0.0_f64,
            archive,
            "SJK radial HUD: how much higher its meters sit, in per cent of the screen (-20 lower to 40 higher)",
        ),
        CvarDefinition::new(
            crate::hud::placement::OFFSETS_CVAR,
            "",
            archive,
            "Where hud_edit moved the HUD's elements: id:x,y pairs in 480-line units, right and down positive (vitals:0,-20 timer:40,0)",
        ),
        CvarDefinition::new(
            "cg_thirdPersonRange",
            80.0,
            archive,
            "Third-person camera distance behind the player",
        ),
        CvarDefinition::new(
            "cg_thirdPersonVertOffset",
            16.0,
            archive,
            "Third-person camera height above the player",
        ),
        CvarDefinition::new(
            "cg_thirdPersonHorzOffset",
            0.0,
            archive,
            "Third-person camera sideways offset",
        ),
        CvarDefinition::new(
            "cg_thirdPersonAngle",
            0.0,
            archive,
            "Third-person camera yaw offset in degrees",
        ),
        CvarDefinition::new(
            "cg_thirdPersonPitchOffset",
            0.0,
            archive,
            "Third-person camera pitch offset in degrees",
        ),
        CvarDefinition::new(
            "cg_thirdPersonCameraDamp",
            0.3_f64,
            archive,
            "Third-person camera easing per cg_cameraFPS frame (per 50 ms below 15); 1 or more snaps to the ideal position",
        ),
        CvarDefinition::new(
            "cg_thirdPersonTargetDamp",
            0.5_f64,
            archive,
            "Third-person look-target easing per cg_cameraFPS frame (per 50 ms below 15); 1 or more snaps",
        ),
        CvarDefinition::new(
            "cg_cameraFPS",
            // A float: the camera reads it with `float_cvar`, so an integer value
            // was never seen and `cg_cameraFPS 0` did nothing.
            125.0_f64,
            archive,
            "Third-person camera easing as EternalJK: damping per frame at this rate, \
             independent of the real frame rate; below 15 uses the original per-50 ms easing",
        ),
        CvarDefinition::new(
            crate::camera::STYLE_CVAR,
            crate::camera::Style::DEFAULT_NAME,
            archive,
            "Third-person camera: ejk stays locked behind you with no damping, as JoF EJK \
             with its strafe helper on; sjk eases after you with the damping cvars",
        ),
        CvarDefinition::new(
            "com_maxfps",
            -1_i64,
            archive | CvarFlags::OMIT_DEFAULT,
            "Maximum rendered frames per second; -1 (AUTO) is 125, or the monitor's refresh \
             rate with com_maxfpsMonitor 1; 0 is uncapped",
        ),
        CvarDefinition::new("cg_drawFPS", false, archive, "Display FPS and frame time"),
        CvarDefinition::new(
            "cg_classicHudFont",
            false,
            archive,
            "Use the retail bitmap font for the in-game status HUD only",
        ),
        CvarDefinition::new(
            crate::game_font::CVAR,
            true,
            archive,
            "Draw text with the game's own fonts where retail did (menus, chat, HUD text, scoreboard, console), else Inter",
        ),
        CvarDefinition::new(
            crate::brush_view::CVAR,
            0_i64,
            archive,
            "Invisible brushes (clip, trigger, ladder...) and movers (door, lift...) drawn see-through: 0 off, 1 gameplay, 2 all",
        ),
        CvarDefinition::new(
            crate::brush_view::TYPES_CVAR,
            "",
            archive,
            "Brush kinds r_showBrushes shows instead of its set: clip playerclip npcclip shotclip              solid trigger teleport push hurt ladder nodrop slick cushion door lift button train endless weather botclip caulk nodraw breakable travel",
        ),
        CvarDefinition::new(
            crate::entity_view::CVAR,
            0_i64,
            archive,
            "Map entities (lights, spawns, origins, items...) marked through walls, pickable with a world note: 0 off, 1 common, 2 all",
        ),
        CvarDefinition::new(
            crate::entity_view::TYPES_CVAR,
            "",
            archive,
            "Entity kinds r_showEntities shows instead of its set: light spawn origin item target sound effect misc other",
        ),
        CvarDefinition::new(
            crate::ground_hud::CVAR,
            false,
            archive,
            "Third-person ground HUD: health, shield, Force and stance around the feet",
        ),
        // Retail names and defaults: OpenJK codemp client/snd_dma.cpp S_Init
        // (s_volume "0.5", s_musicvolume "0.25", s_volumeVoice "1.0",
        // s_khz "44"); the retail sound menu labels s_volume "Effects Volume".
        CvarDefinition::new("s_volume", 0.5_f64, archive, "Effects volume"),
        CvarDefinition::new("s_musicVolume", 0.25_f64, archive, "Music volume"),
        CvarDefinition::new(
            crate::audio::VOICE_VOLUME_CVAR,
            1.0_f64,
            archive,
            "Voice volume: pain, death, taunts, grunts and voice commands, in place of s_volume",
        ),
        CvarDefinition::new(
            "s_khz",
            44_i64,
            archive,
            "Retail sample-rate preset (11/22/44); SJK always mixes at 44.1 kHz",
        ),
        CvarDefinition::new("s_doppler", true, archive, "Legacy looping-sound Doppler"),
        CvarDefinition::new(
            crate::audio::SKIN_SOUNDS_CVAR,
            1_i64,
            archive,
            "Blade skin sounds: 0 stock saber sounds only, 1 over the stock ones, 2 in their place",
        ),
        CvarDefinition::new(
            "cg_jetpackOnSound",
            1_i64,
            archive,
            "Jetpack ignition sound: 1 default, 2 alternate",
        ),
        CvarDefinition::new(
            "cg_jetpackHoverSound",
            1_i64,
            archive,
            "Jetpack hover sound: 1 default, 2 alternate",
        ),
        CvarDefinition::new("cg_footsteps", true, archive, "Play footstep sounds"),
        CvarDefinition::new(
            crate::sound_sonar::CVAR,
            false,
            archive,
            "Sound sonar: arcs round the crosshair point toward the sounds you hear",
        ),
        CvarDefinition::new(
            crate::audio::spatial::CVAR,
            false,
            archive,
            "3D sound: heard from your head, muffled behind walls and doors, rooms ring",
        ),
        CvarDefinition::new(
            crate::team_sabers::TEAM_COLORS_CVAR,
            true,
            archive,
            "Team games: red and blue players' sabers in their team's colour (0: only the other team's or RGB)",
        ),
        CvarDefinition::new(
            crate::team_sabers::CVAR,
            false,
            archive,
            "CTF and CTY: draw every red or blue blade in the team's colour",
        ),
        CvarDefinition::new(
            crate::footprints::PRINTS_CVAR,
            true,
            archive,
            "Steps on sand, snow, mud and dirt kick up dust and leave footprints",
        ),
        CvarDefinition::new(
            crate::footprints::JUMP_CVAR,
            true,
            archive,
            "Jumps and forward landings step with both feet (SJK)",
        ),
        CvarDefinition::new(
            crate::footprints::GUESS_CVAR,
            true,
            archive,
            "Ground a map gives no material takes it from its texture name (SJK)",
        ),
        CvarDefinition::new(
            crate::emotes::VOLUME_CVAR,
            crate::emotes::DEFAULT_VOLUME,
            archive,
            "Loudness of emote sounds, 1 as recorded, apart from the effects volume (SJK)",
        ),
        CvarDefinition::new(
            crate::emotes::VOLUME_VERSION_CVAR,
            0_i64,
            archive,
            "Internal migration marker for the emote volume apart from s_volume",
        ),
        CvarDefinition::new(
            crate::emotes::DUEL_MUTE_CVAR,
            crate::emotes::DEFAULT_DUEL_MUTE,
            archive,
            "Emote sounds are silent while you are in a duel (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::VOLUME_CVAR,
            crate::toy_sounds::DEFAULT_VOLUME,
            archive,
            "Loudness of the toys' sounds, times the effects volume (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Box.cvar(),
            true,
            archive,
            "Play cardboard box sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Order66.cvar(),
            true,
            archive,
            "Play Order 66 sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Hax.cvar(),
            true,
            archive,
            "Play Hax sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Ragdoll.cvar(),
            true,
            archive,
            "Play Ragdoll sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Drone.cvar(),
            true,
            archive,
            "Play Drone sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Boombox.cvar(),
            true,
            archive,
            "Play other players' boombox music and your own (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Monitor.cvar(),
            true,
            archive,
            "Play Monitor sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::MysteryBox.cvar(),
            true,
            archive,
            "Play mystery box sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Banana.cvar(),
            true,
            archive,
            "Play Banana peel sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Tag.cvar(),
            true,
            archive,
            "Play the Tag toy's spray sound (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::PropGun.cvar(),
            true,
            archive,
            "Play Prop gun sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Master.cvar(),
            true,
            archive,
            "Play the sounds of the Toy Master's hidden toys (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Fishing.cvar(),
            true,
            archive,
            "Play the Fishing toy's sounds (SJK)",
        ),
        CvarDefinition::new(
            crate::toy_sounds::Toy::Quests.cvar(),
            true,
            archive,
            "Play the chimes as you are given a quest or finish one (SJK)",
        ),
        CvarDefinition::new(
            crate::fishing::CUE_CVAR,
            crate::fishing::CUE_DEFAULT,
            archive,
            "Show a fish's bite on screen: 0 never, 1 when the fishing sounds can't be heard, 2 always (SJK)",
        ),
        CvarDefinition::new(
            crate::held_view::LOOK_CVAR,
            crate::held_view::DEFAULT_LOOK,
            archive,
            "Let the mouse turn a view the server holds (JA+ kicks and kisses, grapples), which shakes it (SJK)",
        ),
        // SJK's: the JA+ servers known to leave the view free through knockdowns keep
        // it free unless overridden (`held_view::FREE_KNOCKDOWN_VIEW`).
        CvarDefinition::new(
            crate::held_view::KNOCKDOWN_CVAR,
            -1_i64,
            archive,
            "JA+ knockdowns and get-ups: -1 hold the view or not as SJK knows the server (JoF's leave it free), 0 the server holds it (EternalJK), 1 the view stays free (SJK)",
        ),
        CvarDefinition::new(
            crate::emotes::ATTACK_STOP_CVAR,
            crate::emotes::DEFAULT_ATTACK_STOP,
            archive,
            "An attack or alt attack ends your own emote (SJK)",
        ),
        CvarDefinition::new(
            crate::emotes::TYPING_KEEP_CVAR,
            crate::emotes::DEFAULT_TYPING_KEEP,
            archive,
            "A player typing keeps their emote until they move or attack after (SJK)",
        ),
        CvarDefinition::new(
            crate::away_emote::SJK_CVAR,
            crate::away_emote::NONE,
            archive,
            "Looping SJK emote played, quiet, while you are away: an emote id or none (SJK)",
        ),
        CvarDefinition::new(
            crate::away_emote::JAPLUS_CVAR,
            crate::away_emote::NONE,
            archive,
            "JA+ emote played while you are away on a JA+ server: amsit, amkneel... or none (SJK)",
        ),
        CvarDefinition::new(
            crate::away_emote::ALT_TAB_CVAR,
            true,
            archive,
            "An alt-tabbed out or minimised window counts as away for emotes (SJK)",
        ),
        CvarDefinition::new(
            crate::away_emote::IDLE_CVAR,
            crate::away_emote::DEFAULT_IDLE,
            archive,
            "Seconds without input before you count as away for emotes; 0 never (SJK)",
        ),
        CvarDefinition::new(
            crate::unlock_toast::SOUND_CVAR,
            true,
            archive,
            "Play the secret-area sound with the unlock cards",
        ),
        CvarDefinition::new(
            crate::player_trades::SIZE_CVAR,
            1_i64,
            archive,
            "Size of the trade window: 0 small, 1 normal (default), 2 large",
        ),
        CvarDefinition::new(
            crate::holocron_popup::NOTIFICATIONS_CVAR,
            crate::holocron_popup::NOTIFICATIONS_DEFAULT,
            archive,
            "Holocron unlock cards during play: 0 off, 1 on, held during a private duel until it ends (default), 2 always, duels included (the game menu still shows the holocron)",
        ),
        // Stock defaults and units: `sensitivity` 5 scaling `m_yaw`/`m_pitch`
        // degrees per mouse count (`cl_main.cpp:2774,2802-2803`, applied in
        // `cl_input.cpp:1161-1180`). Matching them is what lets a player carry
        // a sensitivity value over from JKA and keep their aim.
        CvarDefinition::new("sensitivity", 5.0_f64, archive, "Mouse sensitivity"),
        CvarDefinition::new(
            "m_yaw",
            0.022_f64,
            archive,
            "Degrees turned per horizontal mouse count",
        ),
        CvarDefinition::new(
            "m_pitch",
            0.022_f64,
            archive,
            "Degrees turned per vertical mouse count",
        ),
        CvarDefinition::new("m_invert", false, archive, "Invert mouse pitch"),
        CvarDefinition::new("in_raw", true, archive, "Saved raw-input preference"),
        CvarDefinition::new(
            "cl_run",
            true,
            archive,
            "Run by default; the walk key walks",
        ),
        // JoF EJK's `cl_idrive` (`cl_input.cpp`, default 0) and SJK's
        // `cl_idriveDelay` ([`crate::input::idrive`]).
        CvarDefinition::new(
            crate::input::idrive::CVAR,
            0_i64,
            archive,
            "Last-pressed movement key wins: 0 off, 1 all directions, 2 jump/crouch only",
        ),
        // A held push or pull key, as JoF EJK's key repeats do ([`crate::input::force_repeat`]).
        CvarDefinition::new(
            crate::input::force_repeat::CVAR,
            true,
            archive,
            "Holding a push or pull key uses it again as soon as its cooldown is over",
        ),
        // EternalJK's `in_mouserepeat` (`sdl_input.cpp`), at a fixed rate
        // ([`crate::input::mouse_repeat`]).
        CvarDefinition::new(
            crate::input::mouse_repeat::CVAR,
            0_i64,
            archive,
            "Held mouse 4 and 5 press their bind again 20 times a second: 0 off, 1 both, 2 mouse 4 only",
        ),
        CvarDefinition::new(
            crate::input::idrive::DELAY_CVAR,
            0_i64,
            archive,
            "cl_idrive: milliseconds a reversal stays neutral before the new key wins (0-1000)",
        ),
        // Retail defaults: `codemp/cgame/cg_xcvar.h` cg_marks 1, cg_shadows 1,
        // cg_drawGun 1.
        CvarDefinition::new(
            "cg_saberContact",
            true,
            archive,
            "Saber contact marks and sparks",
        ),
        CvarDefinition::new("cg_marks", true, archive, "Leave impact marks on the world"),
        CvarDefinition::new("cg_shadows", true, archive, "Draw player shadows"),
        // EternalJK's `cg_dismember` (`cg_xcvar.h`) is off by default; SJK shows
        // every cut limb.
        CvarDefinition::new(
            crate::dismember::CVAR,
            2_i64,
            archive,
            "Show cut-off limbs: 0 none, 1 no heads or waists, 2 all",
        ),
        CvarDefinition::new(
            crate::dismember::SERVER_CVAR,
            0_i64,
            archive,
            "Chance (0-100) that games you host cut off limbs",
        ),
        // The reference's `g_forceRegenTime` default (`g_xcvar.h`), and EternalJK's for
        // its duels.
        CvarDefinition::new(
            crate::connection::local_server::FORCE_REGEN_CVARS[0],
            200_i64,
            archive,
            "Milliseconds a point of Force takes to come back in games you host",
        ),
        CvarDefinition::new(
            crate::connection::local_server::FORCE_REGEN_CVARS[1],
            200_i64,
            archive,
            "Milliseconds a point of Force takes to come back in private duels in games you host",
        ),
        CvarDefinition::new(
            crate::connection::local_server::FORCE_REGEN_CVARS[2],
            200_i64,
            archive,
            "Milliseconds a point of Force takes to come back in full-force duels in games you host",
        ),
        CvarDefinition::new("cg_drawGun", true, archive, "Draw the first-person weapon"),
        CvarDefinition::new(
            "cg_debugMissiles",
            0_i64,
            CvarFlags::NONE,
            "Log missile trails once a second (diagnostics)",
        ),
        CvarDefinition::new(
            "fx_debug",
            0_i64,
            CvarFlags::NONE,
            "Log each effect as it plays, with its shaders and sizes (diagnostics)",
        ),
        CvarDefinition::new(
            crate::emote_end_debug::CVAR,
            false,
            CvarFlags::NONE,
            "Say what came just before a JA+ emote let go of you (diagnostics)",
        ),
        CvarDefinition::new(
            crate::debug_collectibles::CVAR,
            false,
            archive,
            "SJK staff: show every collectible as held, on this PC only, to try them (the hub and other players see what you really hold)",
        ),
        CvarDefinition::new(
            "cg_drawCrosshair",
            1_i64,
            archive,
            "Crosshair picture 0-10: 0 hides it, 1-8 gfx/2d/crosshairb-i, 9 a, 10 j, a white dot of cg_crosshairSize pixels",
        ),
        // Stock defaults from `codemp/cgame/cg_xcvar.h:60,72,102`.
        CvarDefinition::new(
            "cg_drawCrosshairNames",
            true,
            archive,
            "Display the name under the crosshair",
        ),
        CvarDefinition::new("cg_drawTimer", true, archive, "Display elapsed match time"),
        CvarDefinition::new(
            "cg_drawTimerUpperRight",
            false,
            archive,
            "Show the match time in the upper right under the frame rate, as JoF EJK does, not in the HUD layout",
        ),
        CvarDefinition::new(
            crate::version_overlay::CVAR,
            true,
            archive,
            "Draw SJK's version, build date and commit at the top of the screen",
        ),
        // JoF EJK's `flipkick` run (`cg_xcvar.h`), counted in user commands.
        CvarDefinition::new(
            crate::input::flip_kick::DURATION_CVAR,
            50_i64,
            archive,
            "flipkick: user commands the run of jump taps lasts",
        ),
        CvarDefinition::new(
            crate::input::flip_kick::FIRST_JUMP_CVAR,
            0_i64,
            archive,
            "flipkick: user commands the first jump stays held",
        ),
        CvarDefinition::new(
            crate::input::flip_kick::SECOND_JUMP_CVAR,
            0_i64,
            archive,
            "flipkick: user command at which the second jump starts",
        ),
        // JoF EJK's modes (`cg_draw.c` `CG_DrawLagometer`, `CG_DrawSpeedGraph`;
        // [`crate::hud::lagometer`]).
        CvarDefinition::new(
            crate::hud::lagometer::CVAR,
            0_i64,
            archive,
            "Lagometer: 0 off, 1 graph, 2 graph with average ping, interpolation and a speed graph, 3 the same without the backdrop",
        ),
        CvarDefinition::new(
            "cg_saberTrail",
            1_i64,
            archive,
            "Saber motion trails; mode 2 currently uses the normal trail",
        ),
        // EternalJK's (JoF EJK `cg_xcvar.h`): the clash flare and its size.
        CvarDefinition::new(
            crate::saber_clash_flare::CVAR,
            true,
            archive,
            "Saber clash flare: the burst of light where sabers meet",
        ),
        CvarDefinition::new(
            crate::saber_clash_flare::SIZE_CVAR,
            1.0_f64,
            archive,
            "Saber clash flare size, 0 to 2",
        ),
        CvarDefinition::new(
            crate::saber_core::CVAR,
            1.0_f64,
            archive,
            "Blade core thickness: the white line down a lit blade, 0.5 to 2 (1 as in the original game)",
        ),
        // EternalJK's (JoF EJK `cg_xcvar.h`, `cg_event.c` `EV_SABER_HIT`): the red sparks
        // of a saber hit on a body ([`crate::impacts::flesh_sparks`]).
        CvarDefinition::new(
            crate::impacts::FLESH_SPARKS_CVAR,
            i64::from(crate::impacts::FLESH_SPARKS_ALL),
            archive,
            "Red sparks of a saber hit on a body: 0 none to 7 all (default, as stock), as EternalJK",
        ),
        CvarDefinition::new(
            crate::effect_flash::REDUCE_CVAR,
            false,
            archive,
            "Reduce bright flashes: no saber clash flare and no screen flash from explosions",
        ),
        CvarDefinition::new(
            crate::blade_lab_designs::CVAR,
            true,
            archive,
            "Draw the blade skins from Creyon's Blade Lab as the lab drew them",
        ),
        CvarDefinition::new(
            crate::toy_saved::ORDER66_CVAR,
            false,
            archive,
            "The Order 66 toy is on (toy_order66 switches it; it stays on until turned off)",
        ),
        CvarDefinition::new(
            crate::hax::TRAP_CVAR,
            false,
            archive,
            "Hax trap: a player who hacks you springs it, and you both blow up as in Order 66",
        ),
        CvarDefinition::new(
            crate::hax::AWAY_CVAR,
            false,
            archive,
            "With the Hax trap on, players may hack you while you are away (it shows on your look)",
        ),
        CvarDefinition::new(
            crate::toy_saved::HAX_CVAR,
            false,
            archive,
            "The Hax toy is on (toy_hax switches it; it stays on until turned off)",
        ),
        CvarDefinition::new(
            crate::toy_saved::DRONE_CVAR,
            false,
            archive,
            "The Drone toy is on: other players may peek at you, their drone following you (toy_drone switches it)",
        ),
        CvarDefinition::new(
            crate::toy_saved::MONITOR_CVAR,
            false,
            archive,
            "The Monitor toy is on: you may throw monitors (monitor_throw, or +monitor_throw held to charge) and be hit by them (toy_monitor switches it)",
        ),
        CvarDefinition::new(
            crate::toy_saved::BOX_HUNT_CVAR,
            false,
            archive,
            "Box hunt is on: you may become the mark (one every 10 minutes), and killing the mark wins you a free mystery box roll (toy_boxhunt switches it)",
        ),
        CvarDefinition::new(
            crate::toy_saved::QUESTS_CVAR,
            false,
            archive,
            "Quests are on: you may be given a quest, such as hugging or kissing two other players within a minute, for an Echo, or be named in one (toy_quests switches it)",
        ),
        CvarDefinition::new(
            crate::toy_saved::BANANA_CVAR,
            false,
            archive,
            "The Banana peel toy is on: you may drop banana peels (banana_drop) and slip on the ones you walk onto (toy_banana switches it)",
        ),
        CvarDefinition::new(
            crate::toy_saved::PROPGUN_CVAR,
            false,
            archive,
            "The Prop gun toy is on: you hold the chosen prop gun instead of your weapon, attack fires it, and others' bolts knock you down (toy_propgun switches it)",
        ),
        // Other players' toys seen (Collection > Toys).
        CvarDefinition::new(
            crate::toy_sight::CVARS[0].0,
            true,
            archive,
            crate::toy_sight::CVARS[0].1,
        ),
        CvarDefinition::new(
            crate::toy_sight::CVARS[1].0,
            true,
            archive,
            crate::toy_sight::CVARS[1].1,
        ),
        CvarDefinition::new(
            crate::toy_sight::CVARS[2].0,
            true,
            archive,
            crate::toy_sight::CVARS[2].1,
        ),
        CvarDefinition::new(
            crate::toy_sight::CVARS[3].0,
            true,
            archive,
            crate::toy_sight::CVARS[3].1,
        ),
        CvarDefinition::new(
            crate::toy_sight::CVARS[4].0,
            true,
            archive,
            crate::toy_sight::CVARS[4].1,
        ),
        CvarDefinition::new(
            crate::monitor::SHOW_CVAR,
            true,
            archive,
            "Monitors are seen (0: no Monitor toy monitor is drawn, nor its sounds in the world heard, on this PC; SJK)",
        ),
        CvarDefinition::new(
            crate::propgun::SHOW_CVAR,
            true,
            archive,
            "Other players' prop guns are seen (0: their Prop gun toy guns, bolts and sounds are not drawn or heard on this PC; SJK)",
        ),
        CvarDefinition::new(
            crate::propgun::GUN_CVAR,
            "",
            archive,
            "The gun the Prop gun toy holds: the name of a propguns/<name>.gun file (propgun lists them; SJK)",
        ),
        CvarDefinition::new(
            crate::banana::SHOW_CVAR,
            true,
            archive,
            "Banana peels are seen (0: no Banana peel toy peel is drawn, nor its sounds in the world heard, on this PC; SJK)",
        ),
        CvarDefinition::new(
            crate::tags::CVAR,
            crate::tags::DEFAULT,
            archive,
            "The tag toy_tag sticks, by id (tag_gg), or a credits card you hold (sjk_alpha_001); one you do not hold is not stuck (the Collection's Tags tab chooses it; SJK)",
        ),
        CvarDefinition::new(
            crate::tags::FINISH_CVAR,
            "normal",
            archive,
            "The finish toy_tag sticks your tag in: normal, holo or solar; one you hold no copy in sticks normal (the Collection's Tags tab chooses it; SJK)",
        ),
        CvarDefinition::new(
            crate::tags::SHOW_CVAR,
            true,
            archive,
            "Other players' tags are seen (0: no tag but your own is drawn on this PC; SJK)",
        ),
        CvarDefinition::new(
            crate::boombox::RANGE_CVAR,
            crate::boombox::DEFAULT_RANGE,
            archive,
            "How far your boombox is heard, in units (128 to 1000; SJK)",
        ),
        CvarDefinition::new(
            crate::duel_blades::CVAR,
            false,
            archive,
            "Draw every blade stock, in its own colour, in a duel you play or follow and on Duel or Power Duel servers; 0 keeps blade skins",
        ),
        CvarDefinition::new(
            crate::stock_blades::CVAR,
            0_i64,
            archive,
            "Draw blades stock, in their own colour, to save GPU time: 0 off, 1 other players' blades, 2 every blade, yours too",
        ),
        CvarDefinition::new(
            crate::effect_aux::saber_rain_steam::CVAR,
            1_i64,
            archive,
            "Saber steam in rain: 0 off, 1 everyone, 2 only me",
        ),
        // Stock default "1" (`codemp/cgame/cg_xcvar.h`).
        CvarDefinition::new(
            crate::holstered_sabers::CVAR,
            true,
            archive,
            "Show holstered saber hilts",
        ),
        CvarDefinition::new(
            crate::holstered_sabers::SUPPORT_CVAR,
            1_i64,
            archive,
            "Holster server contract: 0 off, 1 auto, 2 force JA+",
        ),
        CvarDefinition::new(
            crate::holstered_sabers::SWAP_CVAR,
            0.45_f64,
            archive,
            "Staff reach fraction where the hilt changes hands",
        ),
        CvarDefinition::new(
            "cg_holsteredStaffSound",
            true,
            archive,
            "Synchronize JA+ staff ignition and shutdown with its reach",
        ),
        CvarDefinition::new(
            "cg_holsteredSaberPos",
            "0 3.5 0",
            archive,
            "Hip hilt offset along bolt right, forward, up",
        ),
        CvarDefinition::new(
            "cg_holsteredSaberAng1",
            "0 0 270",
            archive,
            "Primary hip hilt pitch yaw roll",
        ),
        CvarDefinition::new(
            "cg_holsteredSaberAng2",
            "0 0 270",
            archive,
            "Secondary hip hilt pitch yaw roll",
        ),
        CvarDefinition::new(
            "cg_holsteredStaffPos",
            "0 0 0",
            archive,
            "Back hilt offset along bolt forward right up",
        ),
        CvarDefinition::new(
            "cg_holsteredStaffAng",
            "0 0 150",
            archive,
            "Back hilt pitch yaw roll",
        ),
        CvarDefinition::new(
            "cg_speedTrail",
            1_i64,
            archive,
            "Draw Force Speed afterimages behind players",
        ),
        CvarDefinition::new(
            "cg_auraShell",
            1_i64,
            archive,
            "Display the Force Seeing aura shell on other players",
        ),
        // JoF EJK's `cg_spprotabscolor` (cg_xcvar.h), same default.
        CvarDefinition::new(
            crate::actor_world_submission::force_powers::policy::ANIMATION_CVAR,
            true,
            archive,
            "Rage, Heal, Quick Heal and Grip-victim animation effects",
        ),
        CvarDefinition::new(
            crate::actor_world_submission::force_powers::policy::EMPOWER_CVAR,
            -1_i64,
            archive,
            "Empower glow: -1 server, 0 full body, 1 arms, 2 hands, 3 off; ordinary Push stays full body",
        ),
        CvarDefinition::new(
            "cg_spProtAbsColor",
            true,
            archive,
            "Protect and Absorb together show one cyan shell, as in single player",
        ),
        CvarDefinition::new(
            crate::unlockables::SABER_SKIN_CVAR,
            "",
            archive,
            "The blade skin your sabers wear, by unlock id (saber_sun: the Sun blade), shown once your SJK profile holds it; empty for the stock blade",
        ),
        CvarDefinition::new(
            crate::looks::SECOND_CVAR,
            "",
            archive,
            "The blade skin your second saber wears, by unlock id; empty for the same as the first (cg_saberSkin), stock for the stock blade",
        ),
        CvarDefinition::new(
            crate::finishers::CVAR,
            "",
            archive,
            "The finisher your saber kills play, by unlock id (finisher_disintegrate), worn once your SJK profile holds it; empty for none",
        ),
        CvarDefinition::new(
            crate::saber_sound_choice::CVAR,
            "",
            archive,
            "The saber sounds you hear and other SJK players hear for your sabers: empty follows your saber shader, stock the stock sounds, or a shader you own by unlock id (saber_sun: the Sun blade's)",
        ),
        CvarDefinition::new(
            crate::saber_skin_options::CVAR,
            "",
            archive,
            "The blade skins' parts you switched off, skin.option words (saber_sun.haze: the Sun blade without its heat haze); every SJK player sees your blade so",
        ),
        CvarDefinition::new(
            crate::quick_wheel::SOUNDS_CVAR,
            true,
            archive,
            "Quick wheel sounds: changing page, moving to another choice and running one (0 silent)",
        ),
        CvarDefinition::new(
            "cg_shieldSphere",
            0_i64,
            archive,
            "Show a shield hit as multiplayer's sphere (1) instead of a shell on the body (0)",
        ),
        CvarDefinition::new(
            "cg_hitMarker",
            false,
            archive,
            "Show the red damage-direction indicator near the crosshair (0 off, 1 on)",
        ),
        CvarDefinition::new(
            crate::damage_feedback::SHIELD_CVAR,
            false,
            archive,
            "The damage indicator shows green, not red, when the hit took your shield (armour)",
        ),
        CvarDefinition::new(
            "cg_shieldBrightness",
            2_i64,
            archive,
            "How bright a shield hit shows on the body, 1 (stock) to 12, default 2; ignored by the sphere",
        ),
        CvarDefinition::new(
            crate::duel_ghost::CVAR,
            true,
            archive,
            "Players in a duel show as see-through blue figures, as on JoF EJK (1), or dark under the blue bubble (0)",
        ),
        CvarDefinition::new(
            "cg_shieldBrightnessDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the two-pass shield flash default",
        ),
        CvarDefinition::new(
            crate::news_panel::SEEN_CVAR,
            "",
            archive,
            "The SJK version whose update recap (the News panel) you closed; empty shows the current one once",
        ),
        CvarDefinition::new(
            crate::news_panel::RESET_CVAR,
            "",
            archive,
            "The SJK version whose update put the settings it changed back to their default (the News panel's Settings to review), so they are reset once",
        ),
        CvarDefinition::new(
            crate::settings::slider_peek::CVAR,
            true,
            archive,
            "Over a match, holding a Settings slider for a moment fades the rest of the screen away so its change shows on the game (0 keeps the screen)",
        ),
        CvarDefinition::new(
            crate::support_links::CVAR,
            true,
            archive,
            "Show Support SJK at the top left of the SJK UI's main page: links to SJK's GitHub Sponsors and Ko-fi pages (0 hides it, as its x does)",
        ),
        CvarDefinition::new(
            crate::alpha_notice_card::CVAR,
            0,
            archive,
            "The alpha notice's version you accepted (the card at the first start that says SJK is an alpha with no warranty); 0 shows it again",
        ),
        CvarDefinition::new(
            "ui_hideFirstSetup",
            false,
            archive,
            "Don't open First setup when the client starts (the firstsetup command still opens it)",
        ),
        CvarDefinition::new(
            "cl_autoUpdate",
            true,
            archive,
            "Look for a newer SJK release at start and every 30 minutes while running (the Update page installs it)",
        ),
        CvarDefinition::new(
            "cl_identity",
            true,
            archive,
            "Keep an identity key and tell the SJK hub which game server you are on, so other SJK players see your badge (0 sends nothing)",
        ),
        CvarDefinition::new(
            crate::crash_report::CVAR,
            true,
            archive,
            "Send a report to the SJK team when SJK crashes (build, system, graphics card, map, server, the error and the last console lines without chat), at the next start through the SJK hub (0 keeps it on this PC only)",
        ),
        CvarDefinition::new(
            crate::settings_usage::CVAR,
            true,
            archive,
            "Tell the SJK team anonymously which settings you change and what you search for in Settings (no key, name or server is sent; 0 sends nothing)",
        ),
        CvarDefinition::new(
            crate::discord_presence::CVAR,
            1,
            archive,
            "Show what you are doing in SJK on your Discord profile, through the Discord app on this PC (0 off, 1 on, 2 without the server's name)",
        ),
        CvarDefinition::new(
            crate::settings::mode::CVAR,
            crate::settings::mode::SettingsMode::Simple.name(),
            archive,
            "Settings shows the settings that matter most (simple) or every setting (advanced)",
        ),
        CvarDefinition::new(
            "cl_hubUrl",
            crate::player_identity::DEFAULT_HUB_URL,
            archive,
            "Address of the SJK hub (https://...); empty means no hub",
        ),
        CvarDefinition::new(
            "cl_sjkChat",
            true,
            archive,
            "Show the SJK chat, which every SJK player shares through the SJK hub, and read it (0 hides it and stops reading)",
        ),
        CvarDefinition::new(
            crate::chat::tabs::JOF_JOIN_CVAR,
            -1,
            archive,
            "Read JoF's clan chat (verified JoF and S players, staff): -1 when it is yours, 1 joined, 0 left",
        ),
        CvarDefinition::new(
            crate::chat::tabs::SITH_JOIN_CVAR,
            -1,
            archive,
            "Read the Sith clan's chat, S (verified JoF and S players, staff): -1 when it is yours, 1 joined, 0 left",
        ),
        CvarDefinition::new(
            crate::sjk_chat_frame::FEED_CVAR,
            true,
            archive,
            "Show live SJK chat messages in the in-game general chat feed (the ESC menu dock and SJK chat page stay visible)",
        ),
        CvarDefinition::new(
            crate::sjk_chat_frame::CONSOLE_CVAR,
            true,
            archive,
            "Keep live SJK chat messages in console scrollback",
        ),
        CvarDefinition::new(
            crate::installed_models::CVAR,
            true,
            archive,
            "Tell the SJK hub which player models you have, so other SJK players' cards say whether they see the model you wear (0 makes the hub forget your list)",
        ),
        CvarDefinition::new(
            crate::world_ping::SHOW_CVAR,
            true,
            archive,
            "Show other SJK players' world pings (+ping): your team's in team games, everyone's otherwise; 0 hides them",
        ),
        CvarDefinition::new(
            crate::world_ping::SOUND_CVAR,
            true,
            archive,
            "Play a short sound when a world ping appears, yours included",
        ),
        CvarDefinition::new(
            crate::sjk_chat_frame::SOUND_CVAR,
            true,
            archive,
            "Play a sound when an SJK chat message arrives, yours included (cg_chatSounds 0 silences it too)",
        ),
        CvarDefinition::new(
            crate::chat::client_icons::CVAR,
            true,
            archive,
            "Start an SJK chat message sent from another known client (JoF EternalJK) with its icon instead of SJK's emblem",
        ),
        CvarDefinition::new(
            crate::chat_gifs::CVAR,
            true,
            archive,
            "Show the GIF of a GIPHY link in SJK chat, fetched from GIPHY by this PC, which GIPHY sees (0 keeps links as text and fetches nothing)",
        ),
        CvarDefinition::new(
            crate::chat_gifs::VIDEO_CVAR,
            false,
            archive,
            "Play silent MP4 clips from trusted SJK chat sources with Windows' built-in decoder (off by default: Windows code, not SJK's Rust, reads these internet files)",
        ),
        CvarDefinition::new(
            crate::e2e_whispers::CVAR,
            true,
            archive,
            "Whisper other SJK players end-to-end encrypted through the SJK hub: only the two of you can read it (0 sends every whisper through the game server)",
        ),
        CvarDefinition::new(
            crate::sound_mutes::IGNORE_CVAR,
            false,
            archive,
            "A JA+ or jaPRO ignore <player> you send also mutes that player's sounds until the map changes; ignore again hears them",
        ),
        CvarDefinition::new(
            crate::audio::muted_sounds::CVAR,
            "",
            archive,
            "Words: every sound whose file path holds one is never played (s_muteSounds \"ysalamiri hacking\")",
        ),
        CvarDefinition::new(
            crate::map_blocks::CVAR,
            true,
            archive,
            "Draw players SJK staff blocked for everyone on this server as blocked until the map changes (0 shows them as themselves; your own blocks still apply)",
        ),
        CvarDefinition::new(
            crate::chat_gifs::search::KEY_CVAR,
            "",
            archive,
            "Your own GIPHY API key for the SJK chat's GIF search (empty uses SJK's, shared by every SJK player)",
        ),
        CvarDefinition::new(
            "cl_updateAs",
            "",
            CvarFlags::NONE,
            "Pretend to be this release version when checking for updates (testing a local build)",
        ),
        // JoF EJK's: 1 shows your own movement from the server, 2 also its angles.
        CvarDefinition::new(
            "cg_noPredict",
            0_i64,
            CvarFlags::NONE,
            "Skip movement prediction: 1 shows the server's position, 2 its angles too",
        ),
        CvarDefinition::new(
            "cg_freeCamera",
            0_i64,
            CvarFlags::NONE,
            "Detached camera flight with a stationary server body; use /freecam",
        ),
        CvarDefinition::new(
            // JoF EJK fake noclip: fly locally with a stationary server body.
            "cg_fakeNoclip",
            0_i64,
            CvarFlags::NONE,
            "Fly client-side while the server sees you standing still (use /fakenoclip)",
        ),
        // JoF EJK's: 0 hides all hats and capes, 1 shows everyone's, 2 only yours.
        CvarDefinition::new(
            crate::cosmetics::VISIBILITY_CVAR,
            1_i64,
            archive,
            "Draw worn hats and capes: 0 none, 1 everyone, 2 only yours",
        ),
        // Clamped 15..1000 like JoF EJK and EternalJK (`CL_ReadyToSendPacket`).
        CvarDefinition::new(
            "cl_maxpackets",
            DEFAULT_MAX_PACKETS,
            archive,
            "Most move packets sent a second (15 to 1000); user commands are made at your gameplay FPS (gameplay_fps) and wait for the next packet",
        ),
        // SJK's: the servers known to cap commands keep their cap unless lifted
        // (`server_caps.rs`).
        CvarDefinition::new(
            crate::server_caps::OVERRIDE_CVAR,
            0_i64,
            archive,
            "1 ignores the servers SJK knows to cap commands (JoF's at 125 a second): gameplay_fps applies as set there too",
        ),
        // SJK's: the frame rate whose movement the player wants, apart from the
        // frames drawn, or 0 to follow the FPS cap (`command_rate`, `gameplay_fps.rs`).
        CvarDefinition::new(
            crate::gameplay_fps::CVAR,
            sjk_client::command_rate::AUTO_GAMEPLAY_FPS,
            archive,
            "Gameplay FPS: 0 (Auto) moves you at your FPS cap; 125, 142, 250, 333 or 20 to 1000 keep that movement whatever FPS you draw; a pmove_fixed server's step wins; help_gameplay_fps explains it",
        ),
        // Stock default "1", clamped 0..5 (`codemp/client/cl_input.cpp:1539`).
        CvarDefinition::new(
            "cl_packetdup",
            1_i64,
            archive,
            "Repeat the previous N packets' usercmds in every packet (0 to 5) so lost packets drop no move; servers SJK knows to take fewer get fewer (JoF's: none)",
        ),
        // SJK's: the servers known to take fewer repeated commands keep their cap
        // unless lifted (`server_caps.rs`).
        CvarDefinition::new(
            crate::server_caps::PACKET_DUP_OVERRIDE_CVAR,
            0_i64,
            archive,
            "1 ignores the servers SJK knows to take fewer repeated packets (JoF's: none, Heimdall kicks for them): cl_packetdup applies as set there too",
        ),
        CvarDefinition::new(
            "cl_timeNudge",
            0_i64,
            archive,
            "Live presentation latency in milliseconds (-30 to 30); positive adds buffer",
        ),
        CvarDefinition::new("cg_drawHud", true, archive, "Display the in-game HUD"),
        CvarDefinition::new("cg_draw2D", true, archive, "Display 2D game overlays"),
        CvarDefinition::new(
            crate::hud::toggle::Part::Elements.cvar(),
            crate::hud::toggle::Part::Elements.hidden_by_default(),
            archive,
            "cg_drawHud 0 hides the HUD's elements (health, ammo, timers, kill feed)",
        ),
        CvarDefinition::new(
            crate::hud::toggle::Part::Crosshair.cvar(),
            crate::hud::toggle::Part::Crosshair.hidden_by_default(),
            archive,
            "cg_drawHud 0 hides the crosshair and the name under it",
        ),
        CvarDefinition::new(
            crate::hud::toggle::Part::Names.cvar(),
            crate::hud::toggle::Part::Names.hidden_by_default(),
            archive,
            "cg_drawHud 0 hides nameplates, names over players and team logos",
        ),
        CvarDefinition::new(
            crate::hud::toggle::Part::Chat.cvar(),
            crate::hud::toggle::Part::Chat.hidden_by_default(),
            archive,
            "cg_drawHud 0 hides the chat box",
        ),
        CvarDefinition::new(
            "cg_simpleItems",
            false,
            archive,
            "Draw pickup icons instead of models",
        ),
        CvarDefinition::new(
            sjk_client::player_model_eligibility::BLACKLIST_CVAR,
            "",
            archive,
            "Player model names replaced with Kyle; NPCs are unaffected",
        ),
        CvarDefinition::new(
            sjk_client::player_model_eligibility::METADATA_CVAR,
            true,
            archive,
            "Respect installed player-model eligibility metadata",
        ),
        CvarDefinition::new(
            "cg_forceModel",
            false,
            archive,
            "Use your model for other players",
        ),
        CvarDefinition::new(
            "cg_crosshairSize",
            24.0_f64,
            archive,
            "Crosshair size in 640x480 units",
        ),
        CvarDefinition::new(
            "cg_speedometer",
            0_i64,
            archive,
            "Speed bits: 1 on, 256 kph, 512 mph, 32768 XYZ",
        ),
        CvarDefinition::new(
            "cl_consoleKeys",
            // `cl_main.cpp:2826` ships "~ ` 0x7e 0x60 0xb2"; 0xb0 (the shifted
            // German key) is added so shift+that key opens the console there.
            "~ ` 0x7e 0x60 0xb2 0xb0",
            archive,
            "Characters that toggle the console",
        ),
        CvarDefinition::new(
            "cg_drawStatus",
            true,
            archive,
            "Display health, armor, and force status",
        ),
        CvarDefinition::new(
            "cg_drawWeapon",
            true,
            archive,
            "Display current weapon and ammunition",
        ),
        CvarDefinition::new(
            "cg_drawScores",
            true,
            archive,
            "Display the bound scoreboard",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::CVAR,
            crate::scoreboard::style::ScoreboardStyle::DEFAULT_NAME,
            archive,
            "Scoreboard layout: auto (sjk with the SJK UI's menus, else classic), sjk, or classic (after the retail scoreboard)",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::COMPACT_CVAR,
            true,
            archive,
            "SJK scoreboard: thin rows, so every player fits in one column, as wide as the names and centred",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::FPS_CVAR,
            true,
            archive,
            "SJK scoreboard: show the frame rate on the header line, left of the clock",
        ),
        CvarDefinition::new(
            crate::scoreboard::AFK_CVAR,
            true,
            archive,
            "Scoreboard: players seen still for 3 minutes (or wearing [AFK]) drawn faint below the others",
        ),
        CvarDefinition::new(
            "cg_smallScoreboard",
            false,
            archive,
            "Classic scoreboard: always use the small rows",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::IDS_CVAR,
            true,
            archive,
            "Classic scoreboard: show each player's client ID",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::SJK_IDS_CVAR,
            false,
            archive,
            "SJK scoreboard: show each player's client number before the name",
        ),
        CvarDefinition::new(
            crate::scoreboard::style::DEATHS_RANK_CVAR,
            false,
            archive,
            "Scoreboard: rank players by score minus deaths where deaths are shown, fewer deaths first on a tie",
        ),
        CvarDefinition::new(
            crate::jof_tag::CVAR,
            true,
            archive,
            "Show a known clan's emblem left of names carrying its tag (JoF, JoFA, JAWA, {{S}})",
        ),
        CvarDefinition::new(
            "cg_drawScoreboardIcons",
            true,
            archive,
            "Classic scoreboard: show each player's head icon",
        ),
        CvarDefinition::new(
            "cg_drawScoreboardPlayerCount",
            1_i64,
            archive,
            "Classic scoreboard header: 0 off, 1 host name and counts, 2 counts",
        ),
        CvarDefinition::new(
            "cg_drawChat",
            true,
            archive,
            "Display incoming chat and prints",
        ),
        CvarDefinition::new(
            "cl_sensitivityScaleVersion",
            0_i64,
            archive,
            "Internal migration marker for the move to stock sensitivity units",
        ),
        CvarDefinition::new(
            RATE_VERSION_CVAR,
            0_i64,
            archive,
            "Internal migration marker for the 90000 rate default",
        ),
        CvarDefinition::new(
            "com_maxfpsDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the AUTO com_maxfps default",
        ),
        CvarDefinition::new(
            "cl_bindDefaultsVersion",
            0_i64,
            archive,
            "Internal stock-bind migration version",
        ),
        CvarDefinition::new(
            "cl_consoleKeyDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the scan-code console key default",
        ),
        CvarDefinition::new(
            "cg_scoreboardStyleDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the auto scoreboard style default",
        ),
        CvarDefinition::new(
            "cl_wheelBindVersion",
            0_i64,
            archive,
            "Internal migration marker for Q's quick wheel bind opening the last page",
        ),
        CvarDefinition::new(
            "ui_menuStyleDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the SJK UI menu style default",
        ),
        CvarDefinition::new(
            "cg_cameraStyleDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the ejk camera style default",
        ),
        CvarDefinition::new(
            "cg_killfeedDefaultVersion",
            0_i64,
            archive,
            "Internal migration marker for the kill feed being on by default",
        ),
        CvarDefinition::new(
            crate::chat::text_sizes::VERSION_CVAR,
            0_i64,
            archive,
            "Internal migration marker for chat and centre-print sizes on EternalJK's scale",
        ),
    ];
    for definition in definitions {
        cvars.register(definition)?;
    }
    cvars.register_alias("r_vsync", "r_swapInterval")?;
    cvars.register_alias(
        crate::away_emote::JAPLUS_OLD_CVAR,
        crate::away_emote::JAPLUS_CVAR,
    )?;
    crate::cgame_options::register(cvars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_old_away_emote_name_sets_the_japlus_one() {
        let mut cvars = CvarRegistry::new();
        register_daily_cvars(&mut cvars).unwrap();
        cvars
            .set_text(crate::away_emote::JAPLUS_OLD_CVAR, "amsit")
            .unwrap();
        let current = cvars.get(crate::away_emote::JAPLUS_CVAR).unwrap();
        assert_eq!(current.value.as_text(), "amsit");
        assert_eq!(current.name, crate::away_emote::JAPLUS_CVAR);
    }

    #[test]
    fn an_integer_setting_follows_an_on_off_cvar() {
        // cg_serverRemaps 1 answered "1" yet remaps stayed off: the binding
        // only took integer values.
        let mut cvars = CvarRegistry::new();
        cvars
            .register(CvarDefinition::new("on_off", false, CvarFlags::ARCHIVE, ""))
            .unwrap();
        let setting = IntegerSetting::bind(&mut cvars, "on_off", 0).unwrap();
        cvars.set_text("on_off", "1").unwrap();
        assert_eq!(setting.value(), 1);
        cvars.set_text("on_off", "0").unwrap();
        assert_eq!(setting.value(), 0);
        cvars
            .register(CvarDefinition::new("level", 2_i64, CvarFlags::ARCHIVE, ""))
            .unwrap();
        let level = IntegerSetting::bind(&mut cvars, "level", 2).unwrap();
        cvars.set_text("level", "1").unwrap();
        assert_eq!(level.value(), 1);
    }
}
