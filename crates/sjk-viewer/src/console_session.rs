//! Session-facing initialization and compatibility status for the console.

use super::*;
use sjk_client::{
    ForceAllocation, ForceLegalizeRules, ForceProfileNegotiator, LegacyTeamChoice, ProfileApplied,
    force_rank_reply, force_rules_from_serverinfo, legalize_force_powers,
};
use sjk_protocol::GameState;

impl ViewerConsole {
    /// Cached retail `r_drawfog` mode: no allocation or lock on frame reads.
    pub(crate) fn draw_fog_mode(&self) -> crate::fog_volumes::Mode {
        crate::fog_volumes::Mode::from_cvar(self.draw_fog.value())
    }

    /// Build the stock multiplayer cvars/binds and restore the user config.
    /// Whether the profile had no config when the console started: the player's
    /// first start with it.
    pub(crate) fn new_profile(&self) -> bool {
        self.new_profile
    }

    pub(crate) fn new(config_path: PathBuf) -> Result<Self, Box<dyn Error>> {
        // First: it masks the mods' secrets before any cvar or line exists.
        let mods = crate::mods::Mods::new();
        let mut cvars = CvarRegistry::new();
        let user = CvarFlags::ARCHIVE | CvarFlags::USER_INFO;
        for definition in [
            CvarDefinition::new(
                "r_drawfog",
                2_i64,
                CvarFlags::ARCHIVE,
                "Fog: 0 off, 1 volume, 2 global EXP2",
            ),
            CvarDefinition::new("name", "Padawan", user, "Multiplayer player name"),
            CvarDefinition::new(
                crate::saber_rgb::black::CVAR,
                true,
                CvarFlags::ARCHIVE,
                "Show JA+/jaPRO black sabers when plugin feature 3 is allowed",
            ),
            CvarDefinition::new(
                "model",
                "kyle/default",
                user,
                "Multiplayer player model and skin",
            ),
            CvarDefinition::new(
                "rate",
                console_cvars::DEFAULT_RATE,
                user,
                "Maximum connection data rate (bytes a second); servers cap it with sv_maxRate",
            ),
            CvarDefinition::new("snaps", 120_i64, user, "Requested server snapshot rate"),
            // Not USER_INFO: sent only where the server profile sends the key.
            CvarDefinition::new(
                sjk_client::PLUGIN_IDENTITY_KEY,
                "",
                CvarFlags::ARCHIVE,
                "SJK staff only: the client plugin name sent to JA+ and jaPRO servers (what admins' amstatus shows); empty, or a key that is not staff, sends SJK's default for the server",
            ),
            CvarDefinition::new(
                "forcepowers",
                "7-1-032330000000001333",
                user,
                "Force rank, side, and power allocation",
            ),
            // Text, as in the legacy game: JoF EJK appends the worn hat
            // (`color1`) and cape (`color2`) to the colour (`4santahat`).
            CvarDefinition::new(
                "color1",
                "4",
                user,
                "Primary saber colour index, then the worn hat",
            ),
            CvarDefinition::new(
                "color2",
                "4",
                user,
                "Secondary saber colour index, then the worn cape",
            ),
            CvarDefinition::new(
                "cp_sbRGB1",
                0_i64,
                user,
                "Primary RGB blade tint (r | g<<8 | b<<16)",
            ),
            CvarDefinition::new(
                "cp_sbRGB2",
                0_i64,
                user,
                "Secondary RGB blade tint (r | g<<8 | b<<16)",
            ),
            // JA+/TaystJK plugin features to switch off; applied per server
            // profile (`CompatProfile::userinfo_for`).
            CvarDefinition::new(
                "cp_pluginDisable",
                i64::from(sjk_client::PLUGIN_DISABLE_DEFAULT),
                user,
                "JA+/jaPRO plugin mask; jaPRO bit 19 selects only-bhop when the server allows it",
            ),
            CvarDefinition::new("handicap", 100_i64, user, "Starting health percentage"),
            CvarDefinition::new("sex", "male", user, "Player voice sex token"),
            CvarDefinition::new("cg_predictItems", true, user, "Predict local item pickups"),
            CvarDefinition::new("saber1", "single_1", user, "Primary saber definition"),
            CvarDefinition::new("saber2", "none", user, "Secondary saber definition"),
            CvarDefinition::new("char_color_red", 255_i64, user, "Player tint red channel"),
            CvarDefinition::new(
                "char_color_green",
                255_i64,
                user,
                "Player tint green channel",
            ),
            CvarDefinition::new("char_color_blue", 255_i64, user, "Player tint blue channel"),
            CvarDefinition::new(
                "password",
                "",
                CvarFlags::ARCHIVE | CvarFlags::USER_INFO,
                "Password used only when joining a locked server",
            ),
            CvarDefinition::new(
                "fs_gameData",
                "",
                CvarFlags::ARCHIVE,
                "Jedi Academy GameData directory",
            ),
            CvarDefinition::new(
                "cl_master",
                "master.jkhub.org:29060",
                CvarFlags::ARCHIVE,
                "Legacy protocol-26 master server",
            ),
            CvarDefinition::new(
                "cl_master2",
                "master.ouned.de:29060",
                CvarFlags::ARCHIVE,
                "Second protocol-26 master server, asked with cl_master and its list merged",
            ),
            CvarDefinition::new(
                "ui_rankChange",
                0_i64,
                CvarFlags::NONE,
                "Force rank most recently supplied by the server",
            ),
            CvarDefinition::new(
                "ui_myteam",
                0_i64,
                CvarFlags::NONE,
                "Team selected by the server's Force-profile flow",
            ),
        ] {
            cvars.register(definition)?;
        }
        register_daily_cvars(&mut cvars)?;
        let post_color = crate::frame_target::aa::color::Settings::bind(&mut cvars)?;
        let dynamic_light_settings = crate::dynamic_lights::Settings::bind(&mut cvars)?;
        let soft_particles = crate::particle_draw::settings::Settings::bind(&mut cvars)?;
        let dust_motes = crate::dust_motes::Settings::bind(&mut cvars)?;
        let weather = crate::weather::Settings::bind(&mut cvars)?;
        let exposure = crate::frame_target::aa::exposure::Settings::bind(&mut cvars)?;
        let ssao = crate::world_materials::ssao::settings::Settings::bind(&mut cvars)?;
        crate::world_materials::shadows::settings::register(&mut cvars)?;
        let window_options = window_options::Options::register(&mut cvars)?;
        let chat_log = chat_log::register(&mut cvars)?;
        let qcommon = qcommon::Settings::register(&mut cvars)?;
        crate::hud::targeting::register(&mut cvars)?;
        crate::input::settings::register(&mut cvars)?;
        console_options::register(&mut cvars)?;
        let geometry_controls = crate::shared_geometry::environment::Cvars::bind(&mut cvars)?;
        let show_timedelta = crate::net_timing::CvarSetting::bind(&mut cvars)?;
        let time_nudge = crate::presentation_clock::CvarSetting::bind(&mut cvars)?;
        let smooth_clients =
            console_cvars::IntegerSetting::bind(&mut cvars, "cg_smoothClients", 0)?;
        let hide_corpses_in_duel = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::duel_visibility::CORPSES_CVAR,
            1,
        )?;
        let hide_pickups_in_duel = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::duel_visibility::PICKUPS_CVAR,
            1,
        )?;
        let duel_hide_others = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::duel_visibility::HIDE_OTHERS_CVAR,
            1,
        )?;
        let plugin_disable = console_cvars::IntegerSetting::bind(
            &mut cvars,
            "cp_pluginDisable",
            i64::from(sjk_client::PLUGIN_DISABLE_DEFAULT),
        )?;
        let remaps = console_cvars::IntegerSetting::bind(&mut cvars, "cg_remaps", 2)?;
        let map_edits = console_cvars::IntegerSetting::bind(&mut cvars, crate::map_edits::CVAR, 1)?;
        let server_remaps = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::remap_blocked_maps::SERVER_CVAR,
            0,
        )?;
        let remap_blocked_maps =
            console_cvars::RevisionSetting::bind(&mut cvars, crate::remap_blocked_maps::CVAR)?;
        let remap_allowed_maps = console_cvars::RevisionSetting::bind(
            &mut cvars,
            crate::remap_blocked_maps::ALLOWED_CVAR,
        )?;
        let remap_map_choice =
            console_cvars::RevisionSetting::bind(&mut cvars, crate::remap_blocked_maps::MAP_CVAR)?;
        let remap_shared_maps = console_cvars::RevisionSetting::bind(
            &mut cvars,
            crate::remap_blocked_maps::SHARED_LIST_CVAR,
        )?;
        let remap_shared = console_cvars::RevisionSetting::bind(
            &mut cvars,
            crate::remap_blocked_maps::SHARED_CVAR,
        )?;
        let draw_fog = console_cvars::IntegerSetting::bind(&mut cvars, "r_drawfog", 2)?;
        let packet_dup = console_cvars::IntegerSetting::bind(&mut cvars, "cl_packetdup", 1)?;
        let max_packets = console_cvars::IntegerSetting::bind(
            &mut cvars,
            "cl_maxpackets",
            console_cvars::DEFAULT_MAX_PACKETS,
        )?;
        let gameplay_fps = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::gameplay_fps::CVAR,
            sjk_client::command_rate::AUTO_GAMEPLAY_FPS,
        )?;
        let caps_lifted =
            console_cvars::IntegerSetting::bind(&mut cvars, crate::server_caps::OVERRIDE_CVAR, 0)?;
        let fresh_stamps = console_cvars::IntegerSetting::bind(
            &mut cvars,
            crate::server_caps::FRESH_STAMPS_CVAR,
            0,
        )?;
        let pmove_fixed = console_cvars::IntegerSetting::bind(&mut cvars, "pmove_fixed", 0)?;
        let pmove_msec = console_cvars::IntegerSetting::bind(&mut cvars, "pmove_msec", 8)?;
        let socket = super::socket::State::bind(&mut cvars)?;
        let userinfo_dirty = Arc::new(AtomicBool::new(true));
        let overlay_dirty = Arc::clone(&userinfo_dirty);
        // Nameplate bars for teammates read the server's team overlay data (`tinfo`).
        for name in [
            "cg_nameplate",
            "cg_nameplateBars",
            sjk_client::PLUGIN_IDENTITY_KEY,
        ] {
            let dirty = Arc::clone(&overlay_dirty);
            cvars.on_change(name, move |_| dirty.store(true, Ordering::Release))?;
        }
        let userinfo_names = cvars
            .iter()
            .filter(|cvar| cvar.flags.contains(CvarFlags::USER_INFO))
            .map(|cvar| cvar.name.clone())
            .collect::<Vec<_>>();
        for name in userinfo_names {
            let dirty = Arc::clone(&userinfo_dirty);
            cvars.on_change(&name, move |_| dirty.store(true, Ordering::Release))?;
        }
        // Any change of `forcepowers` (the console, a config, the menu) must
        // make the server read it: see `note_forcepowers_change`.
        let forcepowers_changed = Arc::new(AtomicBool::new(false));
        let changed = Arc::clone(&forcepowers_changed);
        cvars.on_change("forcepowers", move |_| {
            changed.store(true, Ordering::Release)
        })?;
        let mut shell = Shell::new(cvars, keybind_editor::default_bindings());
        crate::input::settings::register_commands(&mut shell)?;
        director::register(&mut shell)?;
        super::socket::register_command(&mut shell, &socket)?;
        let client_commands = console_client::Commands::default();
        console_client::register(&mut shell, &client_commands)?;
        mods.register(&mut shell)?;
        for name in mods.userinfo_settings() {
            let dirty = Arc::clone(&userinfo_dirty);
            shell
                .cvars
                .on_change(&name, move |_| dirty.store(true, Ordering::Release))?;
        }
        shell.set_command_frame(0);
        let server_status = Arc::new(RwLock::new("Not connected".to_owned()));
        for command in ["serverinfo", "status"] {
            let status = Arc::clone(&server_status);
            shell.commands.register(
                command,
                "Show the detected server compatibility profile",
                move |_| {
                    Ok(vec![status.read().map_or_else(
                        |_| "Server status unavailable".to_owned(),
                        |value| value.clone(),
                    )])
                },
            )?;
        }
        for (command, help) in [
            ("quit", "Quit the client"),
            ("exit", "Quit the client (alias of quit)"),
            ("connect", "Connect to host[:port]"),
            (
                "devmap",
                "devmap <map>: play a local map with cheats enabled",
            ),
            ("disconnect", "Leave the current server"),
            ("reconnect", "Reconnect to the last server"),
            ("record", "Begin recording a protocol-26 demo"),
            ("stoprecord", "Finish the active demo recording"),
            ("demo", "Play a demo from the demos directory"),
            ("playdemo", "Play a demo (demo alias)"),
            ("demo_restart", "Replay the last console-opened demo"),
            (
                "deletedemo",
                "Delete a demo from the writable demo directory",
            ),
            ("screenshot", "Write the current frame as PNG"),
            ("screenshotJPEG", "Write the current frame as JPEG"),
            (
                "viewpos",
                "Print the camera origin, yaw and pitch, and copy them to the clipboard",
            ),
            (
                "mark",
                "mark <note>: viewpos, kept with the note in marks.txt beside the configuration",
            ),
            (
                "perfmark",
                "perfmark <text>: a timestamped marker in the client log, to split timing                  runs (SJK_GPU_PHASES, SJK_FRAME_BUDGET) into steps",
            ),
        ] {
            shell.commands.register(command, help, |_| Ok(Vec::new()))?;
        }
        let config_directory = config_path
            .parent()
            .ok_or("console config path has no parent")?
            .to_owned();
        // A profile that has no config yet starts on the current defaults, which
        // no migration below should touch (sensitivity's rescale turned a new
        // profile's 5 into 13.022).
        let new_profile = !config_path.exists();
        let emote_volume_saved = std::fs::read_to_string(&config_path)
            .is_ok_and(|config| saves(&config, crate::emotes::VOLUME_CVAR));
        shell.set_config_path(config_path);
        shell.set_log_path(config_directory.join("qconsole.log"));
        shell.set_line_tap(crate::crash_report::trail);
        crate::crash_report::set_directory(&config_directory);
        if let Err(error) = shell.load() {
            shell.push_log(format!("^1Could not load config: {error}"));
            // Also to stderr, which is what the launcher captures. A failed load
            // now blocks every save, so settings silently stop persisting — and
            // a console-only warning is invisible to anyone reading the log
            // rather than opening the console.
            crate::log::progress(format_args!(
                "could not load config: {error}; settings will NOT be saved until it loads cleanly"
            ));
        }
        window_options::discard_retired(&mut shell.cvars);
        // Sensitivity used to be counts-to-radians at 0.001; it is now stock's
        // degrees-per-count through `m_yaw`/`m_pitch`. Rescale a value saved
        // under the old units once, so an existing profile keeps the aim it
        // had instead of silently turning 2.6 times slower.
        if matches!(
            shell
                .cvars
                .get("cl_sensitivityScaleVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            if let Some(saved) = shell
                .cvars
                .get("sensitivity")
                .filter(|_| !new_profile)
                .and_then(|cvar| match cvar.value {
                    CvarValue::Float(value) => Some(value),
                    CvarValue::Integer(value) => Some(value as f64),
                    _ => None,
                })
            {
                let rescaled = saved * 0.001_f64.to_degrees() / 0.022;
                let _ = shell
                    .cvars
                    .set_text("sensitivity", &format!("{rescaled:.3}"));
            }
            let _ = shell.cvars.set_text("cl_sensitivityScaleVersion", "1");
        }
        if matches!(
            shell
                .cvars
                .get("cl_bindDefaultsVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            keybind_editor::migrate_missing_defaults(&mut shell.binds);
            let _ = shell.cvars.set_text("cl_bindDefaultsVersion", "1");
        }
        if matches!(
            shell
                .cvars
                .get("cl_bindDefaultsVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(1 | 2))
        ) {
            let repair_global_u = matches!(
                shell
                    .cvars
                    .get("cl_bindDefaultsVersion")
                    .map(|cvar| &cvar.value),
                Some(CvarValue::Integer(2))
            );
            keybind_editor::migrate_chat_default(&mut shell.binds, repair_global_u);
            let _ = shell.cvars.set_text("cl_bindDefaultsVersion", "3");
        }
        // rate defaulted to stock's 25000, which chokes the 40 snapshots of a busy
        // server, and every profile saved it. Move a saved 25000 once to the new
        // default; a 25000 chosen after this stays.
        if matches!(
            shell
                .cvars
                .get(console_cvars::RATE_VERSION_CVAR)
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            if matches!(
                shell.cvars.get("rate").map(|cvar| &cvar.value),
                Some(CvarValue::Integer(console_cvars::OLD_DEFAULT_RATE))
            ) {
                let _ = shell.cvars.reset("rate");
            }
            let _ = shell.cvars.set_text(console_cvars::RATE_VERSION_CVAR, "1");
        }
        // com_maxfps defaulted to 1000 and every archived cvar was saved, so each
        // existing profile carries that old default. Move it once to the new
        // default, AUTO (-1); a 1000 chosen after this stays.
        if matches!(
            shell
                .cvars
                .get("com_maxfpsDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            if matches!(
                shell.cvars.get("com_maxfps").map(|cvar| &cvar.value),
                Some(CvarValue::Integer(1000))
            ) {
                let _ = shell.cvars.reset("com_maxfps");
            }
            let _ = shell.cvars.set_text("com_maxfpsDefaultVersion", "1");
        }
        // cl_consoleUseScanCode defaulted to 0, which left layouts whose key under
        // Escape types neither `~` nor `` ` `` (Hungarian `0`) without a console
        // key, and every profile saved that 0. Move it once to EternalJK's 1.
        if matches!(
            shell
                .cvars
                .get("cl_consoleKeyDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            let _ = shell.cvars.reset("cl_consoleUseScanCode");
            let _ = shell.cvars.set_text("cl_consoleKeyDefaultVersion", "1");
        }
        // cg_scoreboardStyle defaulted to classic and every profile saved it, so
        // the new default `auto` would reach none. Move a saved classic once to
        // auto: it draws the same classic board unless the menus are the SJK UI,
        // whose own board is the point; a classic chosen after this stays.
        if matches!(
            shell
                .cvars
                .get("cg_scoreboardStyleDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            let style = crate::scoreboard::style::CVAR;
            if shell
                .cvars
                .get(style)
                .is_some_and(|cvar| cvar.value.as_text().trim().eq_ignore_ascii_case("classic"))
            {
                let _ = shell.cvars.reset(style);
            }
            let _ = shell
                .cvars
                .set_text("cg_scoreboardStyleDefaultVersion", "1");
        }
        // Q opened the quick wheel on General (`+wheel general`), and every
        // profile saved that default; it now opens on the page used last (a
        // bare `+wheel`). Move Q's old default once; a page bound on purpose
        // after this, or on another key, stays.
        if matches!(
            shell
                .cvars
                .get("cl_wheelBindVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            if shell
                .binds
                .get("q")
                .is_some_and(|command| command.trim().eq_ignore_ascii_case("+wheel general"))
            {
                let _ = shell.binds.bind("q", "+wheel");
            }
            let _ = shell.cvars.set_text("cl_wheelBindVersion", "1");
        }
        // ui_menuStyle defaulted to classic and every profile saved it, so the
        // SJK UI, the new default, would reach none. Move a saved classic once
        // to the default; a classic chosen after this stays.
        if matches!(
            shell
                .cvars
                .get("ui_menuStyleDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            let style = crate::menu::style::CVAR;
            if shell.cvars.get(style).is_some_and(|cvar| {
                cvar.value
                    .as_text()
                    .trim()
                    .eq_ignore_ascii_case(crate::menu::style::MenuStyle::OLD_DEFAULT_NAME)
            }) {
                let _ = shell.cvars.reset(style);
            }
            let _ = shell.cvars.set_text("ui_menuStyleDefaultVersion", "1");
        }
        // con_style defaulted to classic and every profile saved it, so the new
        // default `auto` would reach none. Move a saved classic once to auto: it
        // draws the same classic console unless the menus are the SJK UI, whose
        // console design is the point; a classic chosen after this stays.
        let version = super::console_options::STYLE_VERSION_CVAR;
        if matches!(
            shell.cvars.get(version).map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            let style = super::console_options::STYLE_CVAR;
            if shell
                .cvars
                .get(style)
                .is_some_and(|cvar| cvar.value.as_text().trim().eq_ignore_ascii_case("classic"))
            {
                let _ = shell.cvars.reset(style);
            }
            let _ = shell.cvars.set_text(version, "1");
        }
        // cg_cameraStyle defaulted to sjk and every profile saved it, so the
        // locked ejk camera, the new default, would reach none. Move a saved sjk
        // once to the default; an sjk chosen after this stays.
        if matches!(
            shell
                .cvars
                .get("cg_cameraStyleDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            let style = crate::camera::STYLE_CVAR;
            if shell.cvars.get(style).is_some_and(|cvar| {
                cvar.value
                    .as_text()
                    .trim()
                    .eq_ignore_ascii_case(crate::camera::Style::OLD_DEFAULT_NAME)
            }) {
                let _ = shell.cvars.reset(style);
            }
            let _ = shell.cvars.set_text("cg_cameraStyleDefaultVersion", "1");
        }
        // cg_killfeed was off by default (a one-line obituary) and every profile
        // saved that 0, so the kill feed, on by default, would reach none. Move a
        // saved 0 once to the default; a 0 chosen after this stays.
        if matches!(
            shell
                .cvars
                .get("cg_killfeedDefaultVersion")
                .map(|cvar| &cvar.value),
            Some(CvarValue::Integer(0))
        ) {
            if matches!(
                shell.cvars.get("cg_killfeed").map(|cvar| &cvar.value),
                Some(CvarValue::Integer(0))
            ) {
                let _ = shell.cvars.reset("cg_killfeed");
            }
            let _ = shell.cvars.set_text("cg_killfeedDefaultVersion", "1");
        }
        retire_modern_ui(&mut shell.cvars);
        crate::chat::text_sizes::migrate(&mut shell.cvars, new_profile);
        // Older profiles archived an earlier default (four passes, then one).
        // Move those values once to two; other strengths, and choices made
        // after migration, remain deliberate.
        let shield_version = match shell
            .cvars
            .get("cg_shieldBrightnessDefaultVersion")
            .map(|cvar| &cvar.value)
        {
            Some(CvarValue::Integer(version)) => *version,
            _ => 2,
        };
        if shield_version < 2 {
            let old_defaults: &[i64] = if shield_version == 0 { &[1, 4] } else { &[1] };
            if matches!(
                shell
                    .cvars
                    .get("cg_shieldBrightness")
                    .map(|cvar| &cvar.value),
                Some(CvarValue::Integer(value)) if old_defaults.contains(value)
            ) {
                let _ = shell.cvars.reset("cg_shieldBrightness");
            }
            let _ = shell
                .cvars
                .set_text("cg_shieldBrightnessDefaultVersion", "2");
        }
        retire_force_illuminate(&mut shell);
        hold_monitor_throw(&mut shell);
        emote_volume_on_its_own(&mut shell.cvars, emote_volume_saved);
        shell.push_log("^5SJK console ready. ^7Type cmdlist for commands.");
        mods.migrate_defaults(&mut shell);
        Ok(Self {
            shell,
            new_profile,
            open: false,
            shift: false,
            held_keys: Default::default(),
            bind_state: Default::default(),
            force_repeat: Default::default(),
            mouse_repeat: Default::default(),
            input: String::with_capacity(INPUT_LIMIT),
            history: Vec::new(),
            history_index: None,
            scroll_offset: 0,
            server_status,
            socket,
            presentation: ConsolePresentation::new(),
            browser: super::browser::Browser::new(),
            asset_browser: Default::default(),
            #[cfg(feature = "mod-jof")]
            jof_content: Default::default(),
            #[cfg(feature = "mod-jof")]
            sith_cloud: crate::jof_content::card::Card::new(&crate::sith_cloud::WORDS),
            debug_panel: super::debug_panel::Panel::new(&config_directory),
            model_viewer: Default::default(),
            changelog: super::changelog::Panel::new(),
            credits: super::credits::Panel::new(),
            update_panel: super::update_panel::Panel::new(),
            identity_panel: super::identity_panel::Panel::new(),
            profile_panel: super::profile_panel::Panel::new(),
            staff_panel: super::staff_panel::Panel::new(),
            #[cfg(feature = "mod-jof")]
            dialogue: super::dialogue_panel::Panel::new(),
            collection_panel: super::collection_panel::Panel::new(),
            holocrons_panel: super::holocrons_panel::Panel::new(),
            profile_hub_return: crate::player_menu::ReturnTarget::MainMenu,
            holocrons_last: false,
            illuminate_lit: false,
            pending_illuminate: None,
            cardboard_box_on: false,
            pending_cardboard_box: None,
            order66_on: false,
            pending_order66: None,
            hax_on: false,
            pending_hax: None,
            ragdoll_on: false,
            pending_ragdoll: None,
            drone_on: false,
            pending_drone: None,
            boombox_on: false,
            pending_boombox: None,
            fishing_on: false,
            pending_fishing: None,
            mining_on: false,
            pending_mining: None,
            fishing_card: Default::default(),
            pending_fishing_sell: false,
            wheel_of_doom_on: false,
            pending_wheel_of_doom: false,
            mining_card: Default::default(),
            pending_mining_sell: false,
            monitor_on: false,
            pending_monitor: None,
            box_hunt_on: false,
            pending_box_hunt: None,
            banana_on: false,
            pending_banana: None,
            pending_settings_search: None,
            sjk_chat_panel: super::sjk_chat_panel::Panel::new(),
            config_import: super::config_import_panel::Panel::new(),
            userinfo_dirty,
            forcepowers_changed,
            show_timedelta,
            time_nudge,
            smooth_clients,
            hide_corpses_in_duel,
            hide_pickups_in_duel,
            duel_hide_others,
            plugin_disable,
            draw_fog,
            remaps,
            server_remaps,
            map_edits,
            remap_blocked_maps,
            remap_allowed_maps,
            remap_map_choice,
            remap_shared_maps,
            remap_shared,
            packet_dup,
            max_packets,
            gameplay_fps,
            frame_cap: crate::runtime_settings::STOCK_FPS_CAP,
            server_cap: None,
            server_family: None,
            caps_lifted,
            fresh_stamps,
            input_health: Default::default(),
            pmove_fixed,
            pmove_msec,
            post_color,
            dynamic_light_settings,
            soft_particles,
            dust_motes,
            weather,
            exposure,
            wheel_pages: crate::quick_wheel::pages::WheelPages::load(&config_directory),
            ssao,
            geometry_controls,
            director: director::Director::default(),
            pending_connection: None,
            pending_demo: None,
            pending_screenshot: None,
            pending_viewpos: false,
            pending_mark: None,
            marks: config_directory.join("marks.txt"),
            copied: String::new(),
            pending_quit: false,
            pending_name_edit: false,
            pending_input: Vec::with_capacity(64),
            pending_chat: std::collections::VecDeque::with_capacity(16),
            whisper_hover: Default::default(),

            client_commands,
            mods,
            server_profile: None,
            script_vfs: None,
            config_directory,
            force_profile: ForceProfileNegotiator::default(),
            saber_change: sjk_client::SaberChangeNotifier::default(),
            movement_policy_log: None,
            window_options,
            chat_log,
            qcommon,
            control: false,
            edit: super::line_edit::LineEdit::default(),
            dead_key: Default::default(),
            selection: super::selection::Selection::new(),
            classic: super::classic::State::default(),
            open_height: None,
            overstrike: false,
        })
    }

    /// Flush an effective profile change through OpenJK's reliable userinfo
    /// command, then send what the Force profile exchange has due.
    pub(crate) fn flush_userinfo(&mut self, session: &mut ClientSession, now: Instant) {
        self.flush_dialogue(session);
        // A change the server ignored (too many in its window) goes again.
        if self.userinfo_dirty.load(Ordering::Acquire) || session.userinfo_refused() {
            self.send_userinfo(session, now);
        }
        self.note_forcepowers_change();
        // Every frame: a rejoin retry falls due seconds after the userinfo
        // that preceded it went out, when nothing is left to flush.
        self.poll_force_rejoin(session, now);
        self.poll_saber_change(session, now);
    }

    fn send_userinfo(&mut self, session: &mut ClientSession, now: Instant) {
        let result = self
            .userinfo()
            .map_err(|error| error.to_string())
            .and_then(|userinfo| {
                session
                    .update_userinfo_with_mod_fields(
                        &userinfo,
                        Some(
                            self.bool_cvar("cg_nameplate").unwrap_or(false)
                                && self.integer_cvar("cg_nameplateBars").unwrap_or(0) > 0,
                        ),
                        &self.mods.userinfo_fields(&self.shell),
                        now,
                    )
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(sjk_client::UserinfoUpdateStatus::RateLimited) => {}
            Ok(sjk_client::UserinfoUpdateStatus::Sent)
            | Ok(sjk_client::UserinfoUpdateStatus::Unchanged) => {
                self.userinfo_dirty.store(false, Ordering::Release);
            }
            Err(error) => {
                self.userinfo_dirty.store(false, Ordering::Release);
                self.shell
                    .push_log(format!("^1Could not update userinfo: {error}"));
            }
        }
    }

    /// Send the queued `forcechanged` reply and bounded `team` retries.
    fn poll_force_rejoin(&mut self, session: &mut ClientSession, now: Instant) {
        let settled = !self.userinfo_dirty.load(Ordering::Acquire);
        let spectator = session.latest_snapshot().player.is_spectator();
        let output = self.force_profile.poll(now, settled, spectator);
        for command in &output.commands {
            if let Err(error) = session.send_reliable_command(command) {
                self.shell
                    .push_log(format!("^1Could not send Force-profile reply: {error}"));
            }
        }
        if let Some(notice) = output.notice {
            self.shell.push_log(notice);
        }
    }

    /// Send the sabers to a JA+ or jaPRO-lineage server on the next frame even if
    /// they did not change (the `saber` command, #132).
    pub(crate) fn request_saber_command(&mut self) {
        self.saber_change.request();
    }

    /// Tell a JA+ or jaPRO-lineage server about a saber changed since the last
    /// frame (menu or console), so it applies without a respawn.
    fn poll_saber_change(&mut self, session: &mut ClientSession, now: Instant) {
        let settled = !self.userinfo_dirty.load(Ordering::Acquire);
        let saber1 = self.text_value("saber1").unwrap_or_default().to_owned();
        let saber2 = self.text_value("saber2").unwrap_or_default().to_owned();
        let Some(command) =
            self.saber_change
                .poll(session.compat_profile(), &saber1, &saber2, settled, now)
        else {
            return;
        };
        match session.send_reliable_command(&command) {
            Ok(()) => self
                .shell
                .push_log(format!("^5Saber: ^7{}", String::from_utf8_lossy(&command))),
            Err(error) => self
                .shell
                .push_log(format!("^1Could not send saber change: {error}")),
        }
    }

    /// Record the team the player chose so an `nfr` reply returns them there.
    pub(crate) fn note_team_choice(&mut self, choice: LegacyTeamChoice) {
        self.force_profile.note_team_choice(choice);
    }

    /// Report the `forcepowers` value sent while joining a server when it is
    /// not the player's own (later userinfo is fitted the same way).
    pub(crate) fn note_server_forcepowers(&mut self, negotiated: &str) {
        let preferred = self.text_value("forcepowers").unwrap_or_default();
        if negotiated != preferred {
            self.shell.push_log(format!(
                "^5Force profile adjusted to server limits: ^7{negotiated}"
            ));
        }
    }

    /// The Force profile the server plays the local player with: the profile
    /// sent (the player's own, fitted to the server's rules with its disabled
    /// powers kept) as the server's rules then leave it (those powers dropped),
    /// or as written off a server.
    pub(crate) fn own_force_allocation(&self) -> Option<ForceAllocation> {
        let preferred = self.text_value("forcepowers")?;
        match self.force_profile.server_rules() {
            Some(rules) => {
                let sent = self.force_profile.sent_forcepowers(preferred);
                Some(legalize_force_powers(&sent, rules).allocation)
            }
            None => ForceAllocation::parse(preferred).ok(),
        }
    }

    /// The Force rules of the server the client plays on (`None` off one, or
    /// in a demo), `g_forcePowerDisable` included.
    pub(crate) fn server_force_rules(&self) -> Option<ForceLegalizeRules> {
        self.force_profile.server_rules()
    }

    /// Play as on a server with `rules`, for tests.
    #[cfg(test)]
    pub(crate) fn set_server_force_rules(&mut self, rules: ForceLegalizeRules) {
        self.force_profile.set_server_rules(Some(rules));
    }

    /// The server said whether its saber skills come free (`EV_SET_FREE_SABER`,
    /// stock cgame's `ui_freeSaber`): adopt it, and when the profile sent for
    /// the player's own changes with it, send that and have the server read it.
    pub(crate) fn note_free_saber(&mut self, free_saber: bool) {
        if !self.force_profile.free_saber_news(free_saber) {
            // Nothing new (or off a server): no profile work per snapshot.
            self.force_profile.note_free_saber(free_saber);
            return;
        }
        let preferred = self.text_value("forcepowers").map(str::to_owned);
        let before = preferred
            .as_deref()
            .map(|preferred| self.force_profile.sent_forcepowers(preferred));
        self.force_profile.note_free_saber(free_saber);
        let after = preferred
            .as_deref()
            .map(|preferred| self.force_profile.sent_forcepowers(preferred));
        if before != after {
            self.userinfo_dirty.store(true, Ordering::Release);
            self.force_profile.profile_applied();
        }
    }

    /// Make `value` the player's Force profile; on a server, have it read it
    /// (`forcechanged` once the userinfo carrying it is out): at once while
    /// spectating, at the next respawn in play.
    pub(crate) fn apply_forcepowers(&mut self, value: &str) {
        if self.set_cvar("forcepowers", value) {
            // Handled here, even when the value is unchanged (Apply again).
            self.forcepowers_changed.store(false, Ordering::Release);
            self.force_profile.profile_applied();
            // The `forcechanged` goes out after the userinfo flush, which an
            // unchanged value would not start.
            self.userinfo_dirty.store(true, Ordering::Release);
        }
    }

    /// A `forcepowers` change made while on a server, by the console, a
    /// config or anything but the Force page's Apply: have the server read it
    /// too (`forcechanged`, after the userinfo that carries it), as Apply
    /// does. A server only re-reads a changed profile when told to
    /// (`Cmd_ForceChanged_f`), so without this the profile stays the one it
    /// has until a menu Apply or a rejoin. The log says what happened, and why
    /// nothing was sent when that is so.
    fn note_forcepowers_change(&mut self) {
        if !self.forcepowers_changed.swap(false, Ordering::AcqRel) {
            return;
        }
        let value = self
            .text_value("forcepowers")
            .unwrap_or_default()
            .to_owned();
        let line = match self.force_profile.profile_applied() {
            ProfileApplied::Queued => format!(
                "^5Force profile: ^7{value} sent; the server reads it at once while you \
                 spectate and at your next respawn in play"
            ),
            ProfileApplied::AlreadyQueued => format!(
                "^5Force profile: ^7{value} sent; a Force-profile reply is already waiting \
                 and makes the server read it"
            ),
            ProfileApplied::NotOnServer => format!(
                "^3Force profile: ^7{value} not announced to the server: its Force rules \
                 are not known yet; the join sends the profile"
            ),
        };
        self.shell.push_log(line);
    }

    /// Refresh `serverinfo` and profile-specific completion from the active session.
    pub(crate) fn set_server_info(&mut self, session: &ClientSession) {
        self.reset_dialogue();
        // A change from before this connection is sent by its join.
        self.forcepowers_changed.store(false, Ordering::Release);
        // Each connection starts with the base userinfo payload, without teamoverlay.
        self.userinfo_dirty.store(true, Ordering::Release);
        self.update_server_info(
            session.game_state(),
            session.compat_profile(),
            session.server(),
        );
        self.force_profile
            .refresh_server_rules(session.game_state());
    }

    /// Refresh demo serverinfo through the same status and completion path as live play.
    pub(crate) fn set_demo_server_info(&mut self, game: &GameState) {
        self.reset_dialogue();
        self.update_server_info(
            game,
            &sjk_client::CompatProfile::from_game_state(game),
            "demo",
        );
    }

    /// Mirror server-owned movement cvars without resetting profile negotiation.
    pub(crate) fn refresh_movement_cvars(&mut self, game: &GameState) {
        // CS_SYSTEMINFO is server-owned (cg_xcvar.h:157-159); never serialize
        // these renderer/client cvars into userinfo or mutate wire commands.
        let mut policy = [0; 4];
        for (slot, (index, name, default)) in [
            (1, "pmove_fixed", 0),
            (1, "pmove_msec", 8),
            (1, "pmove_float", 0),
            (0, "g_stepSlideFix", 0),
        ]
        .into_iter()
        .enumerate()
        {
            let mut value = game
                .config_string(index)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|text| sjk_protocol::InfoString::parse(text).ok())
                .and_then(|info| info.get_i32(name))
                .unwrap_or(default);
            if name == "pmove_msec" {
                value = value.clamp(8, 33);
            }
            let _ = self.shell.cvars.restore_text(name, &value.to_string());
            policy[slot] = value;
        }
        if self.movement_policy_log.replace(policy) != Some(policy) {
            crate::log::progress(format_args!(
                "movement policy: pmove_fixed={} pmove_msec={} pmove_float={} g_stepSlideFix={} ",
                policy[0], policy[1], policy[2], policy[3],
            ));
        }
    }

    /// Complete the server's commands: the profile's own, then those of the mods
    /// that are on.
    pub(crate) fn refresh_server_help(&mut self) {
        let mut help: Vec<_> = self
            .server_profile
            .iter()
            .flat_map(compat_console_commands)
            .map(|command| (command.name.to_owned(), command.description.to_owned()))
            .collect();
        help.extend(self.mods.server_help());
        self.shell.replace_external_command_help(help);
    }

    fn update_server_info(
        &mut self,
        game: &GameState,
        profile: &sjk_client::CompatProfile,
        server: impl std::fmt::Display,
    ) {
        self.refresh_movement_cvars(game);
        let raw = String::from_utf8_lossy(game.config_string(0).unwrap_or_default());
        let server_info = sjk_mod::Server {
            kind: crate::mods::server_kind(profile),
            address: server.to_string(),
            info: raw.clone().into_owned(),
        };
        self.server_cap = crate::server_caps::find(&server_info);
        self.server_family = server_info.family();
        if let Some(cap) = self.server_cap {
            self.shell.push_log(format!(
                "gameplay FPS: {} servers take {} commands a second at most{}",
                cap.name,
                cap.per_second(),
                if self.server_caps_lifted() {
                    ", but gameplay_fps_nocap 1 lifts the cap"
                } else {
                    " (gameplay_fps_nocap 1 lifts it)"
                }
            ));
        }
        self.input_health.reset();
        // The same server's free-saber word survives its map changes.
        self.force_profile.begin_server(&server_info.address);
        self.saber_change.reset();
        let value = format!("profile={profile} server={server} CS_SERVERINFO={raw}");
        if let Ok(mut status) = self.server_status.write() {
            *status = value.clone();
        }
        self.mods.set_server(Some(server_info));
        self.server_profile = Some(profile.clone());
        // The mods made for this server load now; the others' commands go.
        self.mods.sync(&mut self.shell);
        self.refresh_server_help();
        self.shell.push_log(value);
    }

    /// Apply `CG_NewForceRank_f`'s two UI cvars and answer a profile request.
    ///
    /// When the server asks for the Force menu (`open_profile`), the stored
    /// profile is legalized against `CS_SERVERINFO` and the stock
    /// `forcechanged` reply is queued through [`ForceProfileNegotiator`].
    pub(crate) fn apply_force_rank(
        &mut self,
        update: ForceRankUpdate,
        game_state: &GameState,
        now: Instant,
    ) {
        let _ = self
            .shell
            .cvars
            .set_text("ui_rankChange", &update.rank.to_string());
        let _ = self
            .shell
            .cvars
            .set_text("ui_myteam", &update.team.to_string());
        self.shell.push_log(format!(
            "^5Force profile: ^7rank {} team {}{}",
            update.rank,
            update.team,
            if update.open_profile {
                " — server requested profile selection"
            } else {
                ""
            }
        ));
        if !update.open_profile {
            return;
        }
        // The notice's rank and team (the side `g_forceBasedTeams` holds the
        // player to) become the rules the sent profile is fitted to.
        let rules = force_rules_from_serverinfo(game_state, update.rank, update.team);
        self.force_profile.set_server_rules(Some(rules));
        // With the server's own free-saber word, when it has given it.
        let rules = self.force_profile.server_rules().unwrap_or(rules);
        let current = self
            .text_value("forcepowers")
            .unwrap_or_default()
            .to_owned();
        let reply = force_rank_reply(
            &current,
            self.force_profile
                .sent_rules()
                .unwrap_or_else(|| rules.for_sent_profile()),
            self.force_profile.rejoin_team(),
        );
        // The reply waits for this userinfo, carrying the fitted profile.
        self.userinfo_dirty.store(true, Ordering::Release);
        if reply.changed {
            self.shell.push_log(format!(
                "^5Force profile adjusted to server limits: ^7{}",
                reply.forcepowers
            ));
        }
        self.force_profile.queue(&reply, now);
    }

    /// Route `spc` to the modern shell without opening a legacy UI menu.
    /// Despite its cgame handler's name the command only marks the client as
    /// a spectator (`ui_myteam 3`); it precedes the Force-profile `nfr`.
    pub(crate) fn request_siege_profile(&mut self) {
        let _ = self.shell.cvars.set_text("ui_myteam", "3");
        self.shell
            .push_log("^5Server moved you to spectator mode pending the Force profile check");
    }
}

/// The modern menus, scoreboard, console and HUD are retired: a saved `modern`
/// (or the `0` that also named it) goes back to the setting's default, the SJK
/// UI and the styles that follow it, and the settings only they read are
/// dropped from the profile instead of being kept as user cvars.
fn retire_modern_ui(cvars: &mut CvarRegistry) {
    for style in [
        crate::menu::style::CVAR,
        crate::scoreboard::style::CVAR,
        super::console_options::STYLE_CVAR,
        crate::menu_hud::STYLE_CVAR,
    ] {
        if cvars.get(style).is_some_and(|cvar| {
            let value = cvar.value.as_text();
            value.trim().eq_ignore_ascii_case("modern") || value.trim() == "0"
        }) {
            let _ = cvars.reset(style);
        }
    }
    for retired in [
        "ui_accent",
        "con_lineSpacing",
        "con_maxLines",
        "con_datetime",
        // Darkened the backdrop of the old form screens, removed on 10/10/2026.
        "ui_menuContrast",
        // The Shaders tab's rack or grid (0 the rack), replaced on 25/10/2026 by
        // `ui_shaderList`, whose 0 is the new wall of tiles, so every player starts on it.
        "ui_shaderView",
        // The weapon icon on nameplates, removed on 28/10/2026 (Sol: it told nothing).
        "cg_nameplateWeapon",
    ] {
        let _ = cvars.unset(retired);
    }
}

/// Illuminate was a Force wheel entry (`force_illuminate`, on the wheel while
/// `cg_illuminate` was 1) and is a toy now (`toy_illuminate`, always available, lit
/// for the client's run). A saved `cg_illuminate`, whichever way it was set, is
/// dropped from the profile: neither value means anything now, so the holocron is
/// never lit nor unusable because of it. Binds saved with the old command are
/// rewritten to the new one, so the key-binding editor shows them (the old name
/// would run it anyway). Done on every load; it changes nothing once it has run.
fn retire_force_illuminate(shell: &mut sjk_shell::Shell) {
    let _ = shell.cvars.unset(crate::illuminate::OLD_CVAR);
    let renamed: Vec<(String, String)> = shell
        .binds
        .iter()
        .filter_map(|(key, command)| Some((key.to_owned(), renamed_illuminate_command(command)?)))
        .collect();
    for (key, command) in renamed {
        let _ = shell.binds.bind(&key, command);
    }
}

/// `command` with its `force_illuminate` parts named `toy_illuminate`, or `None`
/// when it has none.
fn renamed_illuminate_command(command: &str) -> Option<String> {
    let old = crate::illuminate::OLD_COMMAND;
    let mut changed = false;
    let parts: Vec<String> = command
        .split(';')
        .map(|part| {
            let trimmed = part.trim_start();
            let name = trimmed.split_ascii_whitespace().next().unwrap_or("");
            if name.eq_ignore_ascii_case(old) {
                changed = true;
                let indent = &part[..part.len() - trimmed.len()];
                format!(
                    "{indent}{}{}",
                    crate::illuminate::COMMAND,
                    &trimmed[name.len()..]
                )
            } else {
                part.to_owned()
            }
        })
        .collect();
    changed.then(|| parts.join(";"))
}

/// The Monitor's throw charges while held since 26/10/2026 (`+monitor_throw`): a key
/// bound to the plain `monitor_throw` alone (as the key bindings and the Collection
/// bound it before) is bound to the held one, which throws on the release. A bind that
/// runs it among other commands stays, still throwing at once.
fn hold_monitor_throw(shell: &mut sjk_shell::Shell) {
    let plain: Vec<String> = shell
        .binds
        .iter()
        .filter(|(_, command)| {
            command
                .trim()
                .eq_ignore_ascii_case(crate::monitor::THROW_COMMAND)
        })
        .map(|(key, _)| key.to_owned())
        .collect();
    for key in plain {
        let _ = shell.binds.bind(&key, crate::monitor::CHARGE_COMMAND);
    }
}

/// `s_emoteVolume` was times `s_volume` until 27/10/2026 (#163) and every profile saved
/// it: a saved value moves once to what the player heard, itself times the saved
/// `s_volume` (the old default 2 at the default 0.5 is the new default 1). `saved`:
/// the config holds an `s_emoteVolume` line; a profile without one (a new one, or one
/// saved before the emote volume) keeps the new default and only takes the marker.
fn emote_volume_on_its_own(cvars: &mut sjk_shell::CvarRegistry, saved: bool) {
    let version = crate::emotes::VOLUME_VERSION_CVAR;
    if !matches!(
        cvars.get(version).map(|cvar| &cvar.value),
        Some(CvarValue::Integer(0))
    ) {
        return;
    }
    let number = crate::console::saved_number;
    if saved
        && let (Some(emotes), Some(effects)) = (
            number(cvars, crate::emotes::VOLUME_CVAR),
            number(cvars, "s_volume"),
        )
    {
        let heard = (emotes * effects.clamp(0.0, 1.0)).clamp(0.0, crate::emotes::MAX_VOLUME);
        let _ = cvars.set_text(crate::emotes::VOLUME_CVAR, &format!("{heard:.2}"));
    }
    let _ = cvars.set_text(version, "1");
}

/// Whether `config` sets cvar `name` (`seta name "value"`, any case).
fn saves(config: &str, name: &str) -> bool {
    config.lines().any(|line| {
        let mut words = line.split_whitespace();
        words.next().is_some_and(|command| {
            command.eq_ignore_ascii_case("seta") || command.eq_ignore_ascii_case("set")
        }) && words
            .next()
            .is_some_and(|cvar| cvar.eq_ignore_ascii_case(name))
    })
}

#[cfg(test)]
mod tests {
    use crate::console::ViewerConsole;

    /// Hub report #163: a saved emote volume moves once to what was heard (times the
    /// saved effects volume); a new profile keeps the default 1, and a value set
    /// afterwards stays.
    #[test]
    fn a_saved_emote_volume_moves_once_to_what_was_heard() {
        let directory = tempfile::tempdir().unwrap();
        let fresh = ViewerConsole::new(directory.path().join("new.cfg")).unwrap();
        assert_eq!(fresh.float_cvar(crate::emotes::VOLUME_CVAR), Some(1.0));
        drop(fresh);
        for (emotes, effects, after) in [("2", "0.5", 1.0), ("3", "0.2", 0.6), ("4", "1", 3.0)] {
            let path = directory.path().join(format!("old-{emotes}-{effects}.cfg"));
            std::fs::write(
                &path,
                format!("seta s_emoteVolume \"{emotes}\"\nseta s_volume \"{effects}\"\n"),
            )
            .unwrap();
            let mut console = ViewerConsole::new(path.clone()).unwrap();
            let volume = console.float_cvar(crate::emotes::VOLUME_CVAR).unwrap();
            assert!(
                (volume - after).abs() < 1e-9,
                "{emotes} at {effects}: {volume}"
            );
            assert!(console.set_cvar(crate::emotes::VOLUME_CVAR, "2"));
            drop(console);
            let console = ViewerConsole::new(path).unwrap();
            assert_eq!(console.float_cvar(crate::emotes::VOLUME_CVAR), Some(2.0));
        }
        // An old profile that never saved the emote volume keeps the new default, not
        // the default times its s_volume.
        let path = directory.path().join("old-without.cfg");
        std::fs::write(
            &path,
            "seta s_volume \"0.5\"\nseta s_emoteVolumeVersion \"0\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(path).unwrap();
        assert_eq!(console.float_cvar(crate::emotes::VOLUME_CVAR), Some(1.0));
        assert!(!super::saves(
            "seta s_emoteVolumeVersion \"0\"",
            "s_emoteVolume"
        ));
        assert!(super::saves("SETA S_EMOTEVOLUME \"2\"", "s_emoteVolume"));
    }

    #[cfg(feature = "mod-jof")]
    #[test]
    fn mod_switch_and_server_opt_in_mark_userinfo_dirty_without_global_key() {
        use std::sync::atomic::Ordering;
        let dir = tempfile::tempdir().unwrap();
        let mut console = super::ViewerConsole::new(dir.path().join("config.cfg")).unwrap();
        for (name, value) in [("mod_jof", "0"), (sjk_mod_jof::EJK_OPT_IN, "0")] {
            console.userinfo_dirty.store(false, Ordering::Release);
            console.set_cvar(name, value);
            assert!(console.userinfo_dirty.load(Ordering::Acquire), "{name}");
        }
        assert!(console.shell.cvars.get("jofejk").is_none());
    }

    /// Settings' plugin options and the `japlus.plugin`/`japro.plugin` commands write
    /// `cp_pluginDisable`; like every userinfo cvar, a change goes to the server.
    #[test]
    fn plugin_mask_changes_mark_userinfo_dirty() {
        use std::sync::atomic::Ordering;
        let dir = tempfile::tempdir().unwrap();
        let mut console = super::ViewerConsole::new(dir.path().join("config.cfg")).unwrap();
        console.userinfo_dirty.store(false, Ordering::Release);
        console.set_cvar("cp_pluginDisable", "1544");
        assert!(console.userinfo_dirty.load(Ordering::Acquire));
        console.userinfo_dirty.store(false, Ordering::Release);
        console.try_set_cvar("cp_pluginDisable", "1536").unwrap();
        assert!(console.userinfo_dirty.load(Ordering::Acquire));
        assert_eq!(console.userinfo().unwrap().plugin_disable, Some(1_536));
    }

    /// A new profile keeps the default 5; a profile saved before the stock units
    /// is rescaled once, and only once.
    #[test]
    fn sensitivity_rescales_old_profiles_only() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        {
            let console = ViewerConsole::new(path.clone()).unwrap();
            assert_eq!(console.float_cvar("sensitivity"), Some(5.0));
        }
        let old = directory.path().join("old.cfg");
        std::fs::write(&old, "seta sensitivity \"5\"\n").unwrap();
        {
            let console = ViewerConsole::new(old.clone()).unwrap();
            let rescaled = console.float_cvar("sensitivity").unwrap();
            assert!((rescaled - 13.022).abs() < 1e-9, "{rescaled}");
        }
        let console = ViewerConsole::new(old).unwrap();
        let kept = console.float_cvar("sensitivity").unwrap();
        assert!((kept - 13.022).abs() < 1e-9, "{kept}");
    }

    #[test]
    fn shield_flash_defaults_to_two_passes_and_migrates_old_defaults_only_once() {
        let directory = tempfile::tempdir().unwrap();
        let fresh = ViewerConsole::new(directory.path().join("new.cfg")).unwrap();
        assert_eq!(fresh.integer_cvar("cg_shieldBrightness"), Some(2));
        for (version, saved, expected) in [
            (0, 1, 2),
            (0, 3, 3),
            (0, 4, 2),
            (0, 12, 12),
            (1, 1, 2),
            (1, 4, 4),
            (1, 6, 6),
        ] {
            let path = directory.path().join(format!("old-{version}-{saved}.cfg"));
            std::fs::write(
                &path,
                format!(
                    "seta cg_shieldBrightness {saved}
                     seta cg_shieldBrightnessDefaultVersion {version}
"
                ),
            )
            .unwrap();
            let mut console = ViewerConsole::new(path.clone()).unwrap();
            assert_eq!(console.integer_cvar("cg_shieldBrightness"), Some(expected));
            assert!(console.set_cvar("cg_shieldBrightness", "1"));
            drop(console);
            let console = ViewerConsole::new(path).unwrap();
            assert_eq!(console.integer_cvar("cg_shieldBrightness"), Some(1));
        }
    }

    /// Leaving the player screen's Force page on a server, by another page or by
    /// closing the screen, has the server read the profile even when nothing
    /// changed; staying on it sends nothing.
    #[test]
    fn leaving_the_force_page_on_a_server_sends_forcechanged() {
        use crate::player_menu::{PlayerMenu, ReturnTarget};
        use sjk_client::ForceLegalizeRules;
        use std::time::Instant;
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let own = "7-1-330000000330000333";
        assert!(console.set_cvar("forcepowers", own));
        console.note_forcepowers_change();
        console
            .force_profile
            .set_server_rules(Some(ForceLegalizeRules::default()));
        let now = Instant::now();
        let poll = |console: &mut ViewerConsole| console.force_profile.poll(now, true, false);
        assert!(poll(&mut console).commands.is_empty());
        let mut menu = PlayerMenu::new();
        menu.open(&console, ReturnTarget::InGame);
        menu.show_page(2);
        menu.settle_force_page(true, &mut console);
        menu.settle_force_page(true, &mut console);
        assert!(poll(&mut console).commands.is_empty(), "still on the page");
        menu.show_page(0);
        menu.settle_force_page(true, &mut console);
        assert_eq!(poll(&mut console).commands, vec![b"forcechanged".to_vec()]);
        assert_eq!(console.text_value("forcepowers"), Some(own));
        menu.show_page(2);
        menu.settle_force_page(true, &mut console);
        menu.settle_force_page(false, &mut console);
        assert_eq!(poll(&mut console).commands, vec![b"forcechanged".to_vec()]);
        menu.settle_force_page(false, &mut console);
        assert!(poll(&mut console).commands.is_empty(), "closed already");
    }

    /// A held key bound to push or pull repeats it until the key is let go of,
    /// whoever takes the release, and not with `cl_forceRepeat 0`; other powers'
    /// keys, and a press among other commands that are not push or pull, do not.
    #[test]
    fn a_held_push_or_pull_key_repeats_until_let_go() {
        use winit::keyboard::KeyCode;
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("bind mouse4 force_throw", None);
        console.execute_console_line("bind f \"forceselect 4; force_pull\"", None);
        console.execute_console_line("bind g force_protect", None);
        console.queue_bound_script("MOUSE4", true);
        assert_eq!(console.force_repeat_command(), Some(5));
        let f = crate::input::keys::KeyName::Fixed("f");
        console.queue_bound_key(KeyCode::KeyF, Some(f), true);
        assert_eq!(console.force_repeat_command(), Some(6), "the last pressed");
        console.queue_bound_script("MOUSE4", false);
        assert_eq!(console.force_repeat_command(), Some(6));
        console.queue_bound_key(KeyCode::KeyF, None, false);
        assert_eq!(
            console.force_repeat_command(),
            None,
            "release taken elsewhere"
        );
        console.queue_bound_script("g", true);
        assert_eq!(console.force_repeat_command(), None, "protect toggles");
        console.queue_bound_script("MOUSE4", true);
        console.clear_force_repeat();
        assert_eq!(console.force_repeat_command(), None, "window lost");
        assert!(console.set_cvar(crate::input::force_repeat::CVAR, "0"));
        console.queue_bound_script("MOUSE4", true);
        assert_eq!(console.force_repeat_command(), None, "turned off");
    }

    /// A held mouse 4 runs its bind again 20 times a second with `in_mouseRepeat 1`,
    /// on the console's clock, until let go; mouse 5 too unless it is 2, and a `+`
    /// bind's repeated press changes nothing.
    #[test]
    fn a_held_mouse_4_or_5_presses_its_bind_again_twenty_times_a_second() {
        use crate::input::mouse_repeat::CVAR;
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("bind mouse4 echo four", None);
        console.execute_console_line("bind mouse5 echo five", None);
        console.execute_console_line("bind mouse3 echo three", None);
        // Frames 4 ms apart for `millis`, the commands they ran by name.
        let run = |console: &mut ViewerConsole, start: u64, millis: u64| {
            let mut seen = Vec::new();
            let mut now = start;
            while now < start + millis {
                now += 4;
                console.shell.set_command_clock_millis(now);
                console.repeat_held_mouse_buttons();
                console.execute_buffered_frame(None, |command, _| {
                    seen.push(command.to_owned());
                    true
                });
            }
            seen
        };
        let count = |seen: &[String], command: &str| seen.iter().filter(|c| *c == command).count();
        console.shell.set_command_clock_millis(0);
        console.queue_bound_script("MOUSE4", true);
        console.queue_bound_script("MOUSE5", true);
        console.queue_bound_script("MOUSE3", true);
        let seen = run(&mut console, 0, 1_000);
        assert_eq!(
            count(&seen, "echo four"),
            1,
            "off by default: the press alone"
        );
        assert_eq!(count(&seen, "echo five"), 1);
        assert!(console.set_cvar(CVAR, "1"));
        let seen = run(&mut console, 1_000, 1_000);
        assert_eq!(count(&seen, "echo four"), 20);
        assert_eq!(count(&seen, "echo five"), 20);
        assert_eq!(count(&seen, "echo three"), 0, "only mouse 4 and 5");
        assert!(console.set_cvar(CVAR, "2"));
        let seen = run(&mut console, 2_000, 1_000);
        assert_eq!(
            (count(&seen, "echo four"), count(&seen, "echo five")),
            (20, 0)
        );
        // Let go, wherever the release went: no more presses.
        console.force_repeat_button_up("MOUSE4");
        assert!(run(&mut console, 3_000, 1_000).is_empty());
        // A `+` bind held runs again too; the button ignores a held key's press
        // (`input::state`, `a_held_keys_repeated_press_changes_nothing`).
        console.execute_console_line("bind mouse4 +attack", None);
        console.queue_bound_script("MOUSE4", true);
        let seen = run(&mut console, 4_000, 1_000);
        assert_eq!(
            seen.iter().filter(|c| c.starts_with("+attack")).count(),
            21,
            "the press and 20 repeats: {seen:?}"
        );
        console.clear_force_repeat();
        assert!(run(&mut console, 5_000, 1_000).is_empty(), "window lost");
    }

    #[test]
    fn second_held_key_queues_the_chord_instead_of_its_own_bind() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("bind k echo first", None);
        console.execute_console_line("bind l echo ordinary", None);
        console.execute_console_line("bind k+l kill", None);

        console.queue_bound_script("K", true);
        console.queue_bound_script("L", true);
        let mut seen = Vec::new();
        console.execute_buffered_frame(None, |command, _| {
            seen.push(command.to_owned());
            true
        });
        assert_eq!(seen, ["echo first", "kill"]);
    }

    /// The nameplate's own powers are what the server grants the profile that is
    /// sent, not what it would grant the player's own: a profile over the server's
    /// points only because of disabled powers has other powers trimmed in the
    /// sent one.
    #[test]
    fn own_force_allocation_follows_the_profile_sent() {
        use sjk_client::{ForceLegalizeRules, legalize_force_powers};
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let preferred = "7-1-211012023123002100";
        assert!(console.set_cvar("forcepowers", preferred));
        let rules = ForceLegalizeRules {
            max_rank: 4,
            disabled_mask: 65,
            ..ForceLegalizeRules::default()
        };
        console.force_profile.set_server_rules(Some(rules));
        let own = console.own_force_allocation().unwrap();
        let sent = console.force_profile.sent_forcepowers(preferred);
        assert_eq!(
            own,
            legalize_force_powers(&sent, rules).allocation,
            "the server grants the sent profile"
        );
        let naive = legalize_force_powers(preferred, rules).allocation;
        assert_ne!(own, naive, "the player's own profile would be granted more");
    }

    /// An `EV_SET_FREE_SABER` that changes the profile sent sends it again with
    /// one `forcechanged`; the same word again, or one that changes nothing
    /// sent, sends nothing.
    #[test]
    fn a_changing_free_saber_event_resends_the_profile_once() {
        use sjk_client::ForceLegalizeRules;
        use std::sync::atomic::Ordering;
        use std::time::Instant;
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert!(console.set_cvar("forcepowers", "7-1-331330000330001133"));
        console
            .force_profile
            .set_server_rules(Some(ForceLegalizeRules::default()));
        let now = Instant::now();
        let poll = |console: &mut ViewerConsole| console.force_profile.poll(now, true, false);
        console.userinfo_dirty.store(false, Ordering::Release);
        console.note_free_saber(true);
        assert!(console.userinfo_dirty.load(Ordering::Acquire));
        assert_eq!(poll(&mut console).commands, vec![b"forcechanged".to_vec()]);
        console.userinfo_dirty.store(false, Ordering::Release);
        console.note_free_saber(true);
        assert!(!console.userinfo_dirty.load(Ordering::Acquire));
        assert!(poll(&mut console).commands.is_empty(), "repeated");
        // The stock profile fits either way: nothing to send.
        assert!(console.set_cvar("forcepowers", "7-1-032330000000001333"));
        console.userinfo_dirty.store(false, Ordering::Release);
        poll(&mut console);
        console.note_free_saber(false);
        assert!(!console.userinfo_dirty.load(Ordering::Acquire));
        assert!(poll(&mut console).commands.is_empty(), "unchanged profile");
    }

    /// A `forcepowers` typed in the console, or set by a config, on a server
    /// has the server read it (`forcechanged`); the Force page's Apply does
    /// not send it twice, and off a server the join sends the profile.
    #[test]
    fn a_console_forcepowers_change_on_a_server_is_announced() {
        use sjk_client::{ForceLegalizeRules, ProfileApplied};
        use std::time::Instant;
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let now = Instant::now();
        let poll = |console: &mut ViewerConsole| console.force_profile.poll(now, true, false);
        let logged = |console: &ViewerConsole, needle: &str| {
            console.shell.lines().any(|line| line.text.contains(needle))
        };
        // Off a server nothing is queued, and the log says why.
        let _ = console
            .shell
            .execute_line("forcepowers 7-1-032330000000001000");
        console.note_forcepowers_change();
        assert!(poll(&mut console).commands.is_empty());
        assert!(logged(&console, "not announced to the server"));
        // On one, the change queues `forcechanged`, sent once.
        console
            .force_profile
            .set_server_rules(Some(ForceLegalizeRules::default()));
        let _ = console
            .shell
            .execute_line("forcepowers 7-1-032330000000001333");
        console.note_forcepowers_change();
        assert_eq!(poll(&mut console).commands, vec![b"forcechanged".to_vec()]);
        assert!(logged(&console, "the server reads it"));
        console.note_forcepowers_change();
        assert!(
            poll(&mut console).commands.is_empty(),
            "nothing new to send"
        );
        // While a reply is already waiting, the log says it covers the change.
        console
            .force_profile
            .set_server_rules(Some(ForceLegalizeRules::default()));
        assert_eq!(
            console.force_profile.profile_applied(),
            ProfileApplied::Queued
        );
        let _ = console
            .shell
            .execute_line("forcepowers 7-1-032330000000001000");
        console.note_forcepowers_change();
        assert!(logged(&console, "already waiting"));
        assert_eq!(poll(&mut console).commands.len(), 1);
        // The Force page's Apply handles its own change once.
        console.apply_forcepowers("7-2-011110300000000300");
        console.note_forcepowers_change();
        assert_eq!(poll(&mut console).commands, vec![b"forcechanged".to_vec()]);
        assert!(poll(&mut console).commands.is_empty());
    }

    /// Q's quick wheel opens on the page used last: a new profile binds the
    /// bare `+wheel`, Q's old saved default moves to it once, and a page bound
    /// on purpose later (or on another key) stays.
    #[test]
    fn q_opens_the_quick_wheel_on_the_page_used_last() {
        let directory = tempfile::tempdir().unwrap();
        let fresh = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(fresh.shell.binds.get("q"), Some("+wheel"));
        let old = directory.path().join("old.cfg");
        std::fs::write(
            &old,
            "bind q \"+wheel general\"\nbind e \"+wheel general\"\nbind r \"+wheel weather\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(old).unwrap();
        assert_eq!(console.shell.binds.get("q"), Some("+wheel"));
        assert_eq!(console.shell.binds.get("e"), Some("+wheel general"));
        assert_eq!(console.shell.binds.get("r"), Some("+wheel weather"));
        // A profile saved after the move (the marker set) keeps a General
        // page chosen on purpose.
        let chosen = directory.path().join("chosen.cfg");
        std::fs::write(
            &chosen,
            "seta cl_wheelBindVersion \"1\"\nbind q \"+wheel general\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(chosen).unwrap();
        assert_eq!(console.shell.binds.get("q"), Some("+wheel general"));
    }

    #[test]
    fn a_key_bound_to_the_monitor_throw_holds_it_to_charge() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("monitor.cfg");
        std::fs::write(
            &path,
            "bind t \"monitor_throw\"
bind y \"say hi; monitor_throw\"
bind u \"+monitor_throw\"
",
        )
        .unwrap();
        let console = ViewerConsole::new(path).unwrap();
        assert_eq!(console.shell.binds.get("t"), Some("+monitor_throw"));
        assert_eq!(console.shell.binds.get("y"), Some("say hi; monitor_throw"));
        assert_eq!(console.shell.binds.get("u"), Some("+monitor_throw"));
    }

    /// Illuminate left the Force wheel: a saved `cg_illuminate` (the old on-the-wheel
    /// switch, 0 or 1) is dropped whichever it was, binds with the old command are
    /// renamed, and a bind to anything else stays.
    #[test]
    fn saved_force_illuminate_settings_and_binds_become_the_toy() {
        let directory = tempfile::tempdir().unwrap();
        for value in ["0", "1"] {
            let path = directory.path().join(format!("old{value}.cfg"));
            std::fs::write(
                &path,
                format!(
                    "seta cg_illuminate \"{value}\"
bind x \"force_illuminate\"
                     bind y \"say hi; FORCE_ILLUMINATE\"
bind z \"weapnext\"
"
                ),
            )
            .unwrap();
            let console = ViewerConsole::new(path).unwrap();
            assert!(
                console.shell.cvars.get("cg_illuminate").is_none(),
                "cg_illuminate {value} is dropped, not kept as a user cvar"
            );
            assert_eq!(console.shell.binds.get("x"), Some("toy_illuminate"));
            assert_eq!(console.shell.binds.get("y"), Some("say hi; toy_illuminate"));
            assert_eq!(console.shell.binds.get("z"), Some("weapnext"));
        }
        // A new profile has neither, and the toy's bind stays as bound.
        let fresh = ViewerConsole::new(directory.path().join("fresh.cfg")).unwrap();
        assert!(fresh.shell.cvars.get("cg_illuminate").is_none());
        assert_eq!(
            super::renamed_illuminate_command("toy_illuminate"),
            None,
            "nothing to rename"
        );
        assert_eq!(
            super::renamed_illuminate_command("force_illuminate_x"),
            None,
            "only the whole command"
        );
    }

    /// A profile saved with the retired modern styles starts on the defaults,
    /// and the settings only they read are gone; other saved styles stay.
    #[test]
    fn saved_modern_styles_go_back_to_the_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.cfg");
        std::fs::write(
            &path,
            "seta ui_menuStyle \"modern\"\nseta cg_scoreboardStyle \"0\"\n\
             seta con_style \"Modern\"\nseta cg_hudStyle \"modern\"\n\
             seta ui_accent \"blue\"\nseta con_lineSpacing \"1.2\"\n\
             seta con_maxLines \"40\"\nseta con_datetime \"1\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(path).unwrap();
        assert_eq!(console.text_value("ui_menuStyle"), Some("sjk"));
        assert_eq!(console.text_value("cg_scoreboardStyle"), Some("auto"));
        assert_eq!(console.text_value("con_style"), Some("auto"));
        assert_eq!(console.text_value("cg_hudStyle"), Some("game"));
        for retired in [
            "ui_accent",
            "con_lineSpacing",
            "con_maxLines",
            "con_datetime",
        ] {
            assert!(console.shell.cvars.get(retired).is_none(), "{retired}");
        }
        // The other looks stay as chosen.
        let chosen = directory.path().join("chosen.cfg");
        std::fs::write(
            &chosen,
            "seta ui_menuStyleDefaultVersion \"1\"\nseta ui_menuStyle \"classic\"\n\
             seta cg_scoreboardStyleDefaultVersion \"1\"\nseta cg_scoreboardStyle \"classic\"\n\
             seta con_styleDefaultVersion \"1\"\nseta con_style \"sjk\"\n\
             seta cg_hudStyle \"radial\"\n",
        )
        .unwrap();
        let console = ViewerConsole::new(chosen).unwrap();
        assert_eq!(console.text_value("ui_menuStyle"), Some("classic"));
        assert_eq!(console.text_value("cg_scoreboardStyle"), Some("classic"));
        assert_eq!(console.text_value("con_style"), Some("sjk"));
        assert_eq!(console.text_value("cg_hudStyle"), Some("radial"));
    }
}
