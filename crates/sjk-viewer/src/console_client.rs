//! Client command catalogue and bounded handoff to the existing viewer services.
use super::*;

impl ViewerConsole {
    /// Coalesce `scl` until the ordinary viewer service handoff.
    pub(crate) fn request_siege_class(&mut self) {
        self.client_commands.siege_class = true;
    }
}
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;

#[path = "console_hud_commands.rs"]
mod hud_commands;
#[path = "console_identity.rs"]
mod identity;
#[path = "console_info.rs"]
mod info;
#[path = "console_queries.rs"]
mod queries;
#[path = "console_remaps.rs"]
mod remaps;
#[path = "console_restarts.rs"]
mod restarts;
/// `vid_restart`'s one argument, for its Tab completion.
pub(super) use restarts::FORCE as VID_RESTART_FORCE;

/// Completion/help inventory for commands owned by viewer services.
pub(super) const COMMANDS: &[(&str, &str)] = &[
    (
        "remapShader",
        "Replace a local shader: remapShader <old> <new>",
    ),
    ("listRemaps", "List server and local shader replacements"),
    (
        "clearRemaps",
        "Clear server and local shader replacements until new ones arrive",
    ),
    (
        "blockRemaps",
        "Ignore server shader remaps on a map, the loaded one by default: blockRemaps [map]",
    ),
    (
        "unblockRemaps",
        "Allow server shader remaps on a blocked map again: unblockRemaps [map]",
    ),
    (
        "allowRemaps",
        "Apply server shader remaps on a map even with cg_serverRemaps 0, the loaded one by default: allowRemaps [map]",
    ),
    (
        "unallowRemaps",
        "Let a map's server shader remaps follow cg_serverRemaps again: unallowRemaps [map]",
    ),
    (
        "syncBlockRemaps",
        "SJK creator: ignore server shader remaps on a map for every SJK player, the loaded one by default: syncBlockRemaps [map]",
    ),
    (
        "syncUnblockRemaps",
        "SJK creator: take a map off the shared remap blocks: syncUnblockRemaps [map]",
    ),
    ("speedometer", "Configure supported speedometer flags"),
    ("strafehelper", "Configure supported airborne CGAZ flags"),
    ("play", "Play local sound files"),
    ("music", "Start intro and repeating music"),
    ("stopmusic", "Stop background music"),
    ("soundstop", "Stop sounds and music"),
    ("soundlist", "List registered sound assets"),
    ("soundinfo", "Describe audio output"),
    (
        "s_dynamic",
        "Select dynamic music state (untimed transitions)",
    ),
    (
        "pmonly",
        "Toggle private-only game chat for this map: pmonly [on|off]",
    ),
    (
        crate::chat::quiet::COMMAND,
        "Toggle quiet mode until the map changes (only your duel partner and whispers): quiet [on|off]",
    ),
    ("messagemode", "Compose global chat"),
    ("messagemode2", "Compose team chat"),
    (
        "messagemode3",
        "Compose crosshair-client chat when a target is available",
    ),
    (
        "messagemode4",
        "Compose last-attacker chat when tracking is available",
    ),
    (
        "messagemode5",
        "Compose SJK chat, which every SJK player shares through the SJK hub",
    ),
    (
        crate::chat::tabs::ADMIN_COMMAND,
        "Compose admin chat (amsay): read only by the server's admins, on JA+ and jaPRO servers",
    ),
    (
        crate::chat::tabs::JOF_COMMAND,
        "Compose in JoF's clan chat through the SJK hub, once joined (verified JoF and S players, staff)",
    ),
    (
        crate::chat::tabs::SITH_COMMAND,
        "Compose in the Sith clan's chat (S) through the SJK hub, once joined (verified JoF and S players, staff)",
    ),
    (crate::am_color::COMMAND, crate::am_color::HELP),
    (
        crate::chat_sounds::PREVIEW_COMMAND,
        "Play the sound chosen for a kind of chat message: chatsoundpreview <public|team|private|sjk>",
    ),
    (
        "reply",
        "Whisper to the last player who whispered to you: reply <message>, or bare to open the chat on it",
    ),
    (crate::player_trades::COMMAND, crate::player_trades::HELP),
    (
        crate::e2e_whispers::VERIFY_COMMAND,
        "Show the safety code of your encrypted whispers with a player, to compare with them: whisper_verify <player>",
    ),
    (
        crate::player_ids::CLIENT_LIST_COMMAND,
        crate::player_ids::CLIENT_LIST_HELP,
    ),
    (
        crate::player_ids::CLIENT_LIST_INFO_COMMAND,
        crate::player_ids::CLIENT_LIST_INFO_HELP,
    ),
    (
        crate::player_ids::SHOW_PLAYER_ID_COMMAND,
        crate::player_ids::SHOW_PLAYER_ID_HELP,
    ),
    ("toggleconsole", "Toggle the console"),
    (super::asset_browser::COMMAND, super::asset_browser::HELP),
    (
        "consolebrowser",
        "Search commands and cvars and edit cvar values (F3 in the console)",
    ),
    (super::debug_panel::COMMAND, super::debug_panel::HELP),
    (super::spawn_panel::COMMAND, super::spawn_panel::HELP),
    (super::model_viewer::COMMAND, super::model_viewer::HELP),
    (
        crate::medal_popup::rehearsal::COMMAND,
        crate::medal_popup::rehearsal::HELP,
    ),
    (
        crate::holocrons::rehearsal::COMMAND,
        crate::holocrons::rehearsal::HELP,
    ),
    (
        crate::cards::rehearsal::COMMAND,
        crate::cards::rehearsal::HELP,
    ),
    (
        crate::chat::find_blade::COMMAND,
        crate::chat::find_blade::HELP,
    ),
    (
        crate::unlock_toast::UPDATE_COMMAND,
        crate::unlock_toast::UPDATE_HELP,
    ),
    (
        crate::official::ANNOUNCEMENT_COMMAND,
        crate::official::ANNOUNCEMENT_HELP,
    ),
    (
        crate::official::REWARDS_COMMAND,
        crate::official::REWARDS_HELP,
    ),
    (
        crate::official::ANNOUNCE_COMMAND,
        crate::official::ANNOUNCE_HELP,
    ),
    (
        crate::official::DEBUG_ANNOUNCE_COMMAND,
        crate::official::DEBUG_ANNOUNCE_HELP,
    ),
    (
        crate::official::DEBUG_REWARD_COMMAND,
        crate::official::DEBUG_REWARD_HELP,
    ),
    (
        crate::quick_wheel::OPEN_COMMAND,
        crate::quick_wheel::OPEN_HELP,
    ),
    (
        crate::quick_wheel::RUN_COMMAND,
        crate::quick_wheel::RUN_HELP,
    ),
    (
        crate::quick_wheel::GIF_OPEN_COMMAND,
        crate::quick_wheel::GIF_OPEN_HELP,
    ),
    (
        crate::quick_wheel::GIF_RUN_COMMAND,
        crate::quick_wheel::GIF_RUN_HELP,
    ),
    (crate::weather::COMMAND, crate::weather::HELP),
    (super::changelog::COMMAND, super::changelog::HELP),
    (crate::saber_lab::COMMAND, crate::saber_lab::HELP),
    (super::credits::COMMAND, super::credits::HELP),
    (
        crate::console::PROFILE_COMMAND,
        crate::console::PROFILE_HELP,
    ),
    (
        crate::console::COLLECTION_COMMAND,
        crate::console::COLLECTION_HELP,
    ),
    (
        crate::console::ACHIEVEMENTS_COMMAND,
        crate::console::ACHIEVEMENTS_HELP,
    ),
    (crate::console::STAFF_COMMAND, crate::console::STAFF_HELP),
    (
        crate::console::UNLOCKABLES_COMMAND,
        crate::console::UNLOCKABLES_HELP,
    ),
    (
        crate::console::HOLOCRONS_COMMAND,
        crate::console::HOLOCRONS_HELP,
    ),
    (
        crate::console::SJK_CHAT_COMMAND,
        crate::console::SJK_CHAT_HELP,
    ),
    (
        crate::console::SJK_SAY_COMMAND,
        crate::console::SJK_SAY_HELP,
    ),
    (
        crate::console::SJK_GIF_COMMAND,
        crate::console::SJK_GIF_HELP,
    ),
    (crate::emotes::COMMAND, crate::emotes::HELP),
    (crate::boombox::TRACK_COMMAND, crate::boombox::HELP),
    (crate::propgun::PICK_COMMAND, crate::propgun::PICK_HELP),
    (
        crate::toy_switch_words::COMMAND,
        crate::toy_switch_words::HELP,
    ),
    (crate::block_command::COMMAND, crate::block_command::HELP),
    (crate::sound_mutes::COMMAND, crate::sound_mutes::HELP),
    (
        crate::sound_mutes::UNMUTE_COMMAND,
        crate::sound_mutes::UNMUTE_HELP,
    ),
    (
        crate::block_command::UNBLOCK_COMMAND,
        crate::block_command::UNBLOCK_HELP,
    ),
    (
        crate::saber_skin_command::COMMAND,
        crate::saber_skin_command::HELP,
    ),
    (crate::blade_load_all::COMMAND, crate::blade_load_all::HELP),
    (super::update_panel::COMMAND, super::update_panel::HELP),
    (
        crate::identity_command::COMMAND,
        crate::identity_command::HELP,
    ),
    (crate::avatar_command::COMMAND, crate::avatar_command::HELP),
    (crate::hud::nameplate::COMMAND, crate::hud::nameplate::HELP),
    ("togglemenu", "Toggle the in-game menu"),
    (
        crate::menu::first_setup::COMMAND,
        crate::menu::first_setup::HELP,
    ),
    (
        crate::graphics_quality::COMMAND,
        crate::graphics_quality::HELP,
    ),
    (
        crate::graphics_quality::RENDERER_COMMAND,
        crate::graphics_quality::RENDERER_HELP,
    ),
    (
        crate::graphics_quality::RECOMMEND_COMMAND,
        crate::graphics_quality::RECOMMEND_HELP,
    ),
    ("cmd", "Forward arguments as a reliable server command"),
    (
        "copyServerIP",
        "Copy the connected server IP and port to the clipboard",
    ),
    ("clientinfo", "Print client state and userinfo"),
    ("userinfo", "Print userinfo"),
    ("model", "Set player model and optional skin"),
    ("forcepowers", "Set the player force profile"),
    ("configstrings", "Print non-empty indexed configstrings"),
    ("listEmojis", "List the chat emojis (cg_chatBoxEmojis)"),
    ("mods", "List the client mods and whether each is on"),
    ("showip", "List local interface addresses"),
    ("fs_openedList", "Print mounted package names"),
    ("fs_referencedList", "Print pure-proof package references"),
    ("modelist", "List video modes"),
    ("minimize", "Minimize the window"),
    ("ping", "Query server info and round-trip time"),
    ("serverstatus", "Query server status"),
    ("globalservers", "Query master server addresses"),
    ("localservers", "Discover LAN servers"),
    ("addFavorite", "Add an address to browser favorites"),
    ("rcon", "Execute a remote console command"),
    ("afk", "Toggle the AFK name prefix"),
    ("freecam", "Toggle detached local flight: freecam [on|off]"),
    (
        "peek",
        "Watch a player briefly: peek [id|name] [seconds], peek off",
    ),
    (
        "fakenoclip",
        "Fly locally while the server sees you standing still (toggle)",
    ),
    (crate::ghost_replay::COMMAND, crate::ghost_replay::HELP),
    (crate::mimic::COMMAND, crate::mimic::HELP),
    ("colorname", "Apply selected name colors"),
    ("colorstring", "Configure outgoing chat colors"),
    ("vid_restart", restarts::VID_RESTART_HELP),
    (crate::fps_help::COMMAND, crate::fps_help::HELP),
    (crate::hud_edit::COMMAND, crate::hud_edit::HELP),
    (
        crate::hud::VOTE_PREVIEW_COMMAND,
        crate::hud::VOTE_PREVIEW_HELP,
    ),
    (crate::map_edits::SYNC_COMMAND, crate::map_edits::SYNC_HELP),
    (crate::emote_help::COMMAND, crate::emote_help::HELP),
    (crate::emote_picker::COMMAND, crate::emote_picker::HELP),
    (crate::news_panel::COMMAND, crate::news_panel::HELP),
    (
        crate::news_panel::NEWS_COMMAND,
        crate::news_panel::NEWS_HELP,
    ),
    (crate::gameplay_fps::COMMAND, crate::gameplay_fps::HELP),
    (crate::tickrate_help::COMMAND, crate::tickrate_help::HELP),
    (
        "snd_restart",
        "Recreate audio output and reload sound assets",
    ),
    ("in_restart", "Reset held input and pointer capture"),
    (crate::fs_restart::COMMAND, crate::fs_restart::HELP),
    (crate::rig_reload::COMMAND, crate::rig_reload::HELP),
    (
        crate::chat_gifs::hidden::UNHIDE_ALL,
        crate::chat_gifs::hidden::UNHIDE_ALL_HELP,
    ),
];

/// Queue and worker replies exist only for explicitly requested commands.
#[derive(Default)]
pub(super) struct Commands {
    /// Coalesced server request; never forwarded back to the server.
    siege_class: bool,
    last_demo: Option<String>,
    /// Bounded main-thread service requests.
    pub pending: VecDeque<Vec<String>>,
    /// Bounded worker reply channels.
    pub jobs: Vec<Receiver<Vec<String>>>,
    /// Consecutive frames beyond the connected-server timeout.
    pub timeout_frames: u8,
    /// Name-edit cooldown shared with the userinfo callback.
    pub name_clock: identity::NameClock,
    /// `fs_restart`'s mount in flight.
    pub fs_restart: crate::fs_restart::Task,
    /// `rig_reload`'s mount in flight.
    pub rig_reload: crate::fs_restart::Task,
}

/// JoF content's and the Sith cloud's commands (`jof.downloads`, `sith.downloads`),
/// in a build with JoF tools.
fn jof_commands() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    #[cfg(feature = "mod-jof")]
    {
        crate::jof_content::COMMANDS
            .iter()
            .chain(crate::sith_cloud::COMMANDS)
            .chain(crate::model_sync::COMMANDS)
    }
    #[cfg(not(feature = "mod-jof"))]
    {
        [].iter()
    }
}

/// Whether `name` is one of the client's own commands. Checked before the mods,
/// whose `<id>.` prefix a client command may share (`jof.downloads`).
pub(crate) fn known(name: &str) -> bool {
    all_commands().any(|(known, _)| known.eq_ignore_ascii_case(name))
}

/// Every client command with its help: the one list that registration,
/// queueing and [`known`] share, so a command can't be registered yet never run.
fn all_commands() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    COMMANDS
        .iter()
        .chain(crate::cosmetics::command::COMMANDS)
        .chain(crate::saber_colour_command::COMMANDS)
        .chain(crate::saber_hilt_command::COMMANDS)
        .chain(crate::map_edits::editor::COMMANDS)
        .chain(crate::mystery_box::COMMANDS)
        .chain(crate::toy_master::COMMANDS)
        .chain(jof_commands())
}

/// Register command metadata and only the settings consumed by these services.
pub(super) fn register(shell: &mut Shell, commands: &Commands) -> Result<(), Box<dyn Error>> {
    for &(name, help) in all_commands() {
        if !shell.commands.contains(name) && shell.cvars.get(name).is_none() {
            shell.commands.register(name, help, |_| {
                Err(sjk_shell::CommandError::Handler(
                    "Viewer command dispatcher required".into(),
                ))
            })?;
        }
    }
    queries::register(&mut shell.cvars)?;
    restarts::register(&mut shell.cvars)?;
    identity::register(&mut shell.cvars, &commands.name_clock)?;
    for (name, value, flags, help) in [
        (
            "cg_chatBeep",
            true,
            CvarFlags::ARCHIVE,
            "Play global chat notifications",
        ),
        (
            "cg_teamChatBeep",
            true,
            CvarFlags::ARCHIVE,
            "Play team chat notifications",
        ),
        (
            "s_allowDynamicMusic",
            true,
            CvarFlags::ARCHIVE,
            "Enable dynamic music selection",
        ),
        (
            "s_initsound",
            true,
            CvarFlags::ARCHIVE,
            "Open audio output on startup/restart",
        ),
        (
            "s_show",
            false,
            CvarFlags::NONE,
            "Print started sounds (diagnostic)",
        ),
        (
            "cg_duelMusic",
            true,
            CvarFlags::ARCHIVE,
            "Play private duel music",
        ),
        (
            crate::ghost_replay::SHOWCASE_CVAR,
            false,
            CvarFlags::ARCHIVE,
            "Showcase tools, SJK staff only",
        ),
    ] {
        shell
            .cvars
            .register(CvarDefinition::new(name, value, flags, help))?;
    }
    shell.cvars.register(CvarDefinition::new(
        "s_separation",
        0.5,
        CvarFlags::ARCHIVE,
        "Stereo separation",
    ))?;
    Ok(())
}

/// The file of `firstsetup import <path>` as typed: the tokenizer reads a backslash
/// inside quotes as an escape, which would eat a quoted Windows path's separators
/// (`"C:\Users\..."`, as Explorer's Copy as path gives it). One pair of surrounding
/// quotes is dropped and the rest kept as it is.
fn raw_import_path(command: &str, tokens: &[String]) -> Option<String> {
    if !(tokens.len() > 2
        && tokens[0].eq_ignore_ascii_case(crate::menu::first_setup::COMMAND)
        && tokens[1].eq_ignore_ascii_case(crate::menu::first_setup::IMPORT))
    {
        return None;
    }
    let (_, rest) = command.trim_start().split_once(char::is_whitespace)?;
    let (_, raw) = rest.trim_start().split_once(char::is_whitespace)?;
    let raw = raw.trim();
    let raw = raw
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(raw);
    Some(raw.to_owned())
}

impl Commands {
    /// Retain a recognized service command, preserving rcon's and
    /// `firstsetup import`'s raw argument text.
    pub fn queue(
        &mut self,
        command: &str,
        tokens: &[String],
    ) -> Option<Result<Vec<String>, String>> {
        if !known(&tokens[0]) {
            return None;
        }
        if self.pending.len() == 64 {
            return Some(Err("Client command queue is full".into()));
        }
        if tokens[0].eq_ignore_ascii_case("rcon") {
            let raw = command
                .split_once(char::is_whitespace)
                .map_or("", |(_, args)| args);
            self.pending
                .push_back(vec![tokens[0].clone(), raw.to_owned()]);
        } else if let Some(path) = raw_import_path(command, tokens) {
            self.pending
                .push_back(vec![tokens[0].clone(), tokens[1].clone(), path]);
        } else {
            self.pending.push_back(tokens.to_vec());
        }
        Some(Ok(Vec::new()))
    }
}

impl ViewerConsole {
    /// Last successfully opened console demo, retained after playback stops.
    pub(crate) fn last_console_demo(&self) -> Option<&str> {
        self.client_commands.last_demo.as_deref()
    }

    /// Remember a demo after its header and gamestate have loaded successfully.
    pub(crate) fn remember_console_demo(&mut self, name: &str) {
        self.client_commands.last_demo = Some(name.to_owned());
    }
}

impl crate::GpuState {
    /// `sjkemote <id>`: play an emote the SJK players on this server see; alone it
    /// lists the installed ones.
    fn emote_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let Some(id) = args.first().map(|id| id.to_ascii_lowercase()) else {
            if crate::emotes::lock().catalogue.is_none()
                && let Some(vfs) = self.vfs.as_deref()
            {
                crate::emotes::lock().catalogue = Some(crate::emotes::load(vfs));
            }
            let state = crate::emotes::lock();
            return Ok(crate::emotes::listing(
                state.catalogue.as_deref().unwrap_or_default(),
            ));
        };
        if id == crate::emote_help::WORD {
            return self.emote_help_command();
        }
        if !crate::emotes::valid_id(&id) {
            return Err("An emote id is 1 to 32 of a to z, 0 to 9 and _".to_owned());
        }
        let Some(session) = self.live_session.as_ref() else {
            return Err("Join a server to emote".to_owned());
        };
        // The player's own emote starts here at once; the hub's echo of it is not
        // started again (`update_emotes`).
        let own = u8::try_from(session.latest_snapshot().player.client_num()).ok();
        if id == crate::emotes::STOP && args.len() > 1 {
            return self.emote_stop_others(own, &args[1..].join(" "));
        }
        if id == crate::emotes::MUTE || id == crate::emotes::UNMUTE {
            return self.emote_mute(own, id == crate::emotes::MUTE, &args[1..].join(" "));
        }
        if id == crate::emotes::STOP {
            if let Some(slot) = own {
                crate::emotes::lock().active.stop(slot);
            }
            // The others stop it too; without the identity there is nobody to tell.
            let _ = crate::player_identity::emote(id, None);
            return Ok(Vec::new());
        }
        // The vehicle plays its rider's animations; an emote would fight them for the
        // whole ride (hub report #247).
        if session.latest_snapshot().player.vehicle_entity_num() != 0 {
            return Err("Get off the vehicle to emote".to_owned());
        }
        if id == crate::emotes::SYNC {
            let Some(slot) = own else {
                return Err("Join a server to emote".to_owned());
            };
            return self.emote_sync(slot, args.get(1).map(String::as_str));
        }
        let mut state = crate::emotes::lock();
        if state.catalogue.is_none()
            && let Some(vfs) = self.vfs.as_deref()
        {
            state.catalogue = Some(crate::emotes::load(vfs));
        }
        let def = state
            .catalogue
            .as_ref()
            .and_then(|catalogue| catalogue.iter().find(|def| def.id == id))
            .cloned();
        if let Some(slot) = own {
            let origin = self
                .live_world
                .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))
                .map(|entity| entity.current().transform.translation);
            state
                .active
                .start(slot, &id, def.as_ref(), origin, std::time::Instant::now());
        }
        drop(state);
        if def.is_some_and(|def| def.local) {
            return Ok(Vec::new());
        }
        if !crate::player_identity::emote(id, None) {
            return Err(
                "Played here only: other SJK players see emotes with the SJK identity (cl_identity 1)"
                    .to_owned(),
            );
        }
        Ok(Vec::new())
    }

    /// The other player `word` names: [`crate::emotes::TARGET`] (under the crosshair), a
    /// slot number, or part of the name the game shows (a whole name first); with
    /// that name, without colour codes.
    pub(crate) fn emote_player(&self, own: Option<u8>, word: &str) -> Result<(u8, String), String> {
        let Some(session) = self.live_session.as_ref() else {
            return Err("Join a server to emote".to_owned());
        };
        let game_state = session.game_state();
        let shown = |slot: u8| crate::player_identity::shown_name(game_state, usize::from(slot));
        let players: Vec<(u8, String)> = (0..64u8)
            .filter(|slot| Some(*slot) != own)
            .filter_map(|slot| Some((slot, shown(slot)?)))
            .collect();
        let found = if word.eq_ignore_ascii_case(crate::emotes::TARGET) {
            let now_ms = session.latest_snapshot().server_time;
            let aimed = self
                .crosshair_scan
                .chat_client(now_ms)
                .and_then(|slot| u8::try_from(slot).ok());
            players
                .iter()
                .find(|(slot, _)| Some(*slot) == aimed)
                .ok_or_else(|| "Nobody is under your crosshair".to_owned())?
        } else if let Ok(wanted) = word.parse::<u8>() {
            players
                .iter()
                .find(|(slot, _)| *slot == wanted)
                .ok_or_else(|| format!("Nobody is in slot {wanted}"))?
        } else {
            let words = crate::emotes::plain_name(word);
            let matching: Vec<&(u8, String)> = players
                .iter()
                .filter(|(_, name)| crate::emotes::plain_name(name).contains(&words))
                .collect();
            match matching[..] {
                [] => return Err(format!("Nobody here is called {word}")),
                [one] => one,
                _ => matching
                    .iter()
                    .find(|(_, name)| crate::emotes::plain_name(name) == words)
                    .copied()
                    .ok_or_else(|| {
                        let names: Vec<String> = matching
                            .iter()
                            .map(|(_, name)| crate::emotes::plain_name_shown(name))
                            .collect();
                        format!("{word} matches {}: say more of the name", names.join(", "))
                    })?,
            }
        };
        Ok((found.0, crate::emotes::plain_name_shown(&found.1)))
    }

    /// `sjkemote stop all|<player>`: end the other players' emotes (everyone's but
    /// the player's own, or one player's) on this screen only, once: their next one
    /// plays.
    fn emote_stop_others(&mut self, own: Option<u8>, word: &str) -> Result<Vec<String>, String> {
        if word.eq_ignore_ascii_case(crate::emotes::ALL) {
            let mut state = crate::emotes::lock();
            let slots: Vec<u8> = state
                .active
                .playing()
                .map(|(slot, _)| slot)
                .filter(|slot| Some(*slot) != own)
                .collect();
            for slot in &slots {
                state.active.stop(*slot);
            }
            return Ok(vec![match slots.len() {
                0 => "Nobody else is playing an emote".to_owned(),
                1 => "Stopped 1 emote here".to_owned(),
                count => format!("Stopped {count} emotes here"),
            }]);
        }
        let (slot, name) = self.emote_player(own, word)?;
        let mut state = crate::emotes::lock();
        if state.active.get(slot).is_none() {
            return Err(format!("{name} is not playing an emote"));
        }
        state.active.stop(slot);
        Ok(vec![format!("Stopped {name}'s emote here")])
    }

    /// `sjkemote mute <player>` hides that player's emotes on this screen, by their
    /// identity key, until `sjkemote unmute <player|all>` or the game closes; `mute`
    /// alone lists the muted players.
    fn emote_mute(
        &mut self,
        own: Option<u8>,
        mute: bool,
        word: &str,
    ) -> Result<Vec<String>, String> {
        if word.is_empty() {
            if !mute {
                return Err("sjkemote unmute <player|target|all>".to_owned());
            }
            let state = crate::emotes::lock();
            return Ok(if state.muted.is_empty() {
                vec!["Nobody's emotes are muted".to_owned()]
            } else {
                std::iter::once("Emotes muted:".to_owned())
                    .chain(state.muted.values().map(|name| format!("  {name}")))
                    .collect()
            });
        }
        if !mute && word.eq_ignore_ascii_case(crate::emotes::ALL) {
            let count = std::mem::take(&mut crate::emotes::lock().muted).len();
            return Ok(vec![format!("Unmuted {count} player(s)")]);
        }
        let (slot, name) = self.emote_player(own, word)?;
        let shown = self
            .live_session
            .as_ref()
            .and_then(|session| {
                crate::player_identity::shown_name(session.game_state(), usize::from(slot))
            })
            .unwrap_or_default();
        let key = crate::player_identity::hub_mark(slot, &shown).map(|mark| mark.key_id);
        let mut state = crate::emotes::lock();
        if !mute {
            let removed = key.and_then(|key| state.muted.remove(&key)).or_else(|| {
                // Gone from the hub's list since: by the name kept when muted.
                let key = state
                    .muted
                    .iter()
                    .find(|(_, kept)| {
                        crate::emotes::plain_name(kept) == crate::emotes::plain_name(&name)
                    })
                    .map(|(key, _)| key.clone())?;
                state.muted.remove(&key)
            });
            return match removed {
                Some(_) => Ok(vec![format!("{name}'s emotes play again")]),
                None => Err(format!("{name}'s emotes are not muted")),
            };
        }
        let Some(key) = key else {
            return Err(format!("{name} is not an SJK player: they send no emotes"));
        };
        state.muted.insert(key, name.clone());
        state.active.stop(slot);
        Ok(vec![format!(
            "{name}'s emotes are muted here (sjkemote unmute {name})"
        )])
    }

    /// `sjkemote sync [player]`: the player in `own` joins, in step, the emote of
    /// `target` (a slot number or part of a name), else of the player under the
    /// crosshair, else of the nearest one within [`crate::emotes::SYNC_RANGE`].
    fn emote_sync(&mut self, own: u8, target: Option<&str>) -> Result<Vec<String>, String> {
        let Some(session) = self.live_session.as_ref() else {
            return Err("Join a server to emote".to_owned());
        };
        let game_state = session.game_state();
        let now_ms = session.latest_snapshot().server_time;
        let position = |slot: u8| {
            self.live_world
                .entity(sjk_runtime::EntityId::new(u64::from(slot) + 1))
                .map(|entity| entity.current().transform.translation)
        };
        let seen = position(own);
        let here = seen.unwrap_or_default();
        let mut state = crate::emotes::lock();
        let emoting: Vec<(u8, String)> = state
            .active
            .playing()
            .filter(|(slot, _)| *slot != own)
            .map(|(slot, active)| (slot, active.emote.clone()))
            .collect();
        let shown = |slot: u8| {
            crate::player_identity::shown_name(game_state, usize::from(slot)).unwrap_or_default()
        };
        let leader = match target {
            Some(target) => {
                let wanted = target.parse::<u8>().ok();
                let words = crate::emotes::plain_name(target);
                emoting
                    .iter()
                    .find(|(slot, _)| {
                        wanted == Some(*slot)
                            || (wanted.is_none()
                                && crate::emotes::plain_name(&shown(*slot)).contains(&words))
                    })
                    .ok_or_else(|| format!("{target} is not playing an emote"))?
            }
            None => {
                let aimed = self
                    .crosshair_scan
                    .chat_client(now_ms)
                    .and_then(|slot| u8::try_from(slot).ok());
                let distance = |slot: u8| {
                    position(slot).map_or(f32::INFINITY, |at| {
                        at.iter()
                            .zip(here)
                            .map(|(a, b)| (a - b) * (a - b))
                            .sum::<f32>()
                            .sqrt()
                    })
                };
                emoting
                    .iter()
                    .find(|(slot, _)| Some(*slot) == aimed)
                    .or_else(|| {
                        emoting
                            .iter()
                            .filter(|(slot, _)| distance(*slot) <= crate::emotes::SYNC_RANGE)
                            .min_by(|a, b| distance(a.0).total_cmp(&distance(b.0)))
                    })
                    .ok_or_else(|| "Nobody near you is playing an emote".to_owned())?
            }
        };
        let (leader, emote) = leader.clone();
        if state.catalogue.is_none()
            && let Some(vfs) = self.vfs.as_deref()
        {
            state.catalogue = Some(crate::emotes::load(vfs));
        }
        let def = state
            .catalogue
            .as_ref()
            .and_then(|catalogue| catalogue.iter().find(|def| def.id == emote))
            .cloned();
        state.active.sync(
            own,
            leader,
            &emote,
            def.as_ref(),
            seen,
            std::time::Instant::now(),
        );
        drop(state);
        let name = crate::emotes::plain_name_shown(&shown(leader));
        if def.as_ref().is_some_and(|def| def.local) {
            return Ok(vec![format!("In step with {name} (played here only)")]);
        }
        if !crate::player_identity::emote(emote, Some(leader)) {
            return Err(
                "Played here only: other SJK players see emotes with the SJK identity (cl_identity 1)"
                    .to_owned(),
            );
        }
        Ok(vec![format!("In step with {name}")])
    }

    /// Open stock targeted chat using authoritative damage and retained crosshair state;
    /// while following a player, `messagemode3` whispers to them ([`whisper_slot`]).
    pub(crate) fn targeted_chat(&mut self, attacker: bool) {
        if let Some(session) = &self.live_session {
            let snapshot = session.latest_snapshot();
            let slot = if !attacker {
                whisper_slot(
                    &snapshot.player,
                    self.crosshair_scan.chat_client(snapshot.server_time),
                )
            } else {
                self.damage_feedback
                    .has_attacker()
                    .then(|| u16::try_from(snapshot.player.persistent[6]).ok())
                    .flatten()
            };
            self.chat.update_roster(
                self.resident
                    .session
                    .as_ref()
                    .unwrap_or(session)
                    .game_state(),
            );
            if self.chat.whisper_to(slot) {
                self.gameplay_input.release_keys();
                if let Some(console) = &mut self.console {
                    console.set_open(false);
                }
                self.sync_cursor_policy();
            }
        }
    }
    /// `reply [message]` (JoF EternalJK's): whisper `message` to the last player who
    /// whispered to us, or open the composer on that whisper (`chat/reply.rs`).
    fn reply_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let Some(session) = &self.live_session else {
            return Err("reply: not in a game".into());
        };
        self.chat.update_roster(
            self.resident
                .session
                .as_ref()
                .unwrap_or(session)
                .game_state(),
        );
        if args.is_empty() {
            self.chat
                .open_reply()
                .map_err(|why| why.line().to_owned())?;
            self.gameplay_input.release_keys();
            if let Some(console) = &mut self.console {
                console.set_open(false);
            }
            self.sync_cursor_policy();
            return Ok(Vec::new());
        }
        let target = self
            .chat
            .reply_target()
            .map_err(|why| why.line().to_owned())?;
        let Some(command) = sjk_client::chat_command_full(
            sjk_client::ChatDestination::Player(target.slot()),
            &args.join(" "),
        ) else {
            return Ok(Vec::new());
        };
        self.send_chat_command(&command, None);
        Ok(Vec::new())
    }

    /// Consume explicit service requests and nonblocking query replies each frame.
    pub(crate) fn run_client_commands(&mut self, audio: &mut Option<crate::GameAudio>) {
        while let Some((command, mask, randomness)) = self
            .console
            .as_mut()
            .and_then(|c| c.pending_chat.pop_front())
        {
            self.send_chat_command(&command, Some((mask, randomness)));
        }
        if self.live_presentation_ready()
            && self
                .console
                .as_mut()
                .is_some_and(|c| std::mem::take(&mut c.client_commands.siege_class))
        {
            self.open_siege_classes();
        }
        if let (Some(audio), Some(console)) = (audio.as_mut(), self.console.as_mut()) {
            audio.print_sound_starts(console);
        }
        if let (Some(session), Some(console)) = (
            self.resident
                .session
                .as_mut()
                .or(self.live_session.as_mut()),
            &mut self.console,
        ) {
            while let Some(text) = session.pop_server_print() {
                console.push_log(text);
            }
            let limit = console.float_cvar("cl_timeout").unwrap_or(200.0).max(0.0);
            let timed_out = session
                .packet_silence()
                .is_some_and(|age| age.as_secs_f64() > limit);
            console.client_commands.timeout_frames = if timed_out {
                console.client_commands.timeout_frames.saturating_add(1)
            } else {
                0
            };
            if console.client_commands.timeout_frames > 5 {
                self.session_disconnected("Server connection timed out".into());
            }
        }
        loop {
            let command = self
                .console
                .as_mut()
                .and_then(|console| console.client_commands.pending.pop_front());
            let Some(tokens) = command else { break };
            let result = self.client_command(&tokens, audio);
            if let Some(console) = &mut self.console {
                match result {
                    Ok(lines) => {
                        for line in lines {
                            console.push_log(line);
                        }
                    }
                    Err(error) => console.push_log(format!("^1{error}")),
                }
            }
        }
        if let Some(console) = &mut self.console {
            let mut index = 0;
            while index < console.client_commands.jobs.len() {
                match console.client_commands.jobs[index].try_recv() {
                    Ok(lines) => {
                        console.client_commands.jobs.swap_remove(index);
                        for line in lines {
                            console.push_log(line);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => index += 1,
                    Err(_) => {
                        console.client_commands.jobs.swap_remove(index);
                        console.push_log("^1Query worker stopped");
                    }
                }
            }
        }
        self.poll_fs_restart();
        self.poll_rig_reload();
    }

    fn client_command(
        &mut self,
        tokens: &[String],
        audio: &mut Option<crate::GameAudio>,
    ) -> Result<Vec<String>, String> {
        if let Some(result) = self.run_mod_command(tokens) {
            return result;
        }
        let name = tokens[0].to_ascii_lowercase();
        let args = &tokens[1..];
        match name.as_str() {
            "remapshader" | "listremaps" | "clearremaps" => {
                return self.remap_command(&name, args);
            }
            "blockremaps" | "unblockremaps" | "allowremaps" | "unallowremaps" => {
                return self.remap_block_command(&name, args);
            }
            "syncblockremaps" | "syncunblockremaps" => {
                return self.remap_shared_command(&name, args);
            }
            name if crate::map_edits::editor::is_command(name) => {
                return self.map_edit_command(name, args);
            }
            name if crate::mystery_box::is_command(name) => {
                return self.mystery_box_command(name, args);
            }
            name if crate::toy_master::is_command(name) => {
                return self.toy_master_command(name, args);
            }
            "speedometer" | "strafehelper" => {
                return hud_commands::execute(
                    self.console.as_mut().ok_or("Console unavailable")?,
                    &name,
                    args,
                );
            }
            "play" | "music" | "stopmusic" | "soundstop" | "soundlist" | "soundinfo"
            | "s_dynamic" => {
                let Some(audio) = audio else {
                    if name == "soundinfo" {
                        return Ok(vec!["Audio device closed/unavailable".into()]);
                    }
                    return Err("Audio is not initialized".into());
                };
                audio.sync_gains(self.console.as_ref());
                return audio.sound_command(&name, args, self.vfs.as_deref());
            }
            "messagemode" | "messagemode2" => {
                self.apply_input_action(Some(crate::input::InputAction::MessageMode(
                    name == "messagemode2",
                )));
            }
            "messagemode3" | "messagemode4" => {
                self.targeted_chat(name == "messagemode4");
            }
            "messagemode5" => {
                self.apply_input_action(Some(crate::input::InputAction::SjkMessageMode));
            }
            crate::chat::tabs::ADMIN_COMMAND => {
                self.apply_input_action(Some(crate::input::InputAction::AdminMessageMode));
            }
            crate::chat::tabs::JOF_COMMAND | crate::chat::tabs::SITH_COMMAND => {
                self.apply_input_action(crate::input::chat_action(&name));
            }
            crate::am_color::COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return crate::am_color::run(console, args);
            }
            crate::chat_sounds::PREVIEW_COMMAND => return crate::chat_sounds::command(args),
            "pmonly" => {
                return crate::chat::pm_only::command(
                    self.console.as_mut().ok_or("Console unavailable")?,
                    args,
                );
            }
            crate::chat::quiet::COMMAND => {
                return crate::chat::quiet::command(
                    self.console.as_mut().ok_or("Console unavailable")?,
                    args,
                );
            }
            "reply" => return self.reply_command(args),
            crate::e2e_whispers::VERIFY_COMMAND => return self.whisper_verify_command(args),
            crate::player_trades::COMMAND => return self.trade_command(args),
            crate::player_ids::CLIENT_LIST_COMMAND | crate::player_ids::SHOW_PLAYER_ID_COMMAND => {
                let session = self
                    .resident
                    .session
                    .as_ref()
                    .or(self.live_session.as_ref())
                    .ok_or("clientlist: not in a game")?;
                return Ok(crate::player_ids::client_list_lines(session.game_state()));
            }
            crate::player_ids::CLIENT_LIST_INFO_COMMAND => {
                let session = self
                    .resident
                    .session
                    .as_ref()
                    .or(self.live_session.as_ref())
                    .ok_or("clientlistinfo: not in a game")?;
                return Ok(crate::player_ids::client_list_info_lines(
                    session.game_state(),
                    session.latest_snapshot(),
                ));
            }
            "toggleconsole" => {
                if let Some(console) = &mut self.console {
                    console.set_open(!console.is_open());
                }
                self.sync_cursor_policy();
            }
            "consolebrowser" => {
                if let Some(console) = &mut self.console {
                    console.open_browser();
                }
                self.sync_cursor_policy();
            }
            #[cfg(feature = "mod-jof")]
            crate::jof_content::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.open_jof_content(&self.game_data);
                }
                self.sync_cursor_policy();
            }
            #[cfg(feature = "mod-jof")]
            crate::model_sync::GET | crate::model_sync::SHARE | crate::model_sync::ROLLBACK => {
                let Some(console) = self.console.as_ref() else {
                    return Ok(Vec::new());
                };
                let worn = crate::player_identity::own_model_folder();
                return crate::model_sync::command(
                    &name,
                    args,
                    console,
                    &self.game_data,
                    self.vfs.as_deref(),
                    worn,
                );
            }
            #[cfg(feature = "mod-jof")]
            crate::sith_cloud::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.open_sith_cloud(&self.game_data);
                }
                self.sync_cursor_policy();
            }
            super::asset_browser::COMMAND => {
                if let Some(console) = &mut self.console {
                    console
                        .open_asset_browser(&self.game_data)
                        .map_err(|error| error.to_string())?;
                }
                self.sync_cursor_policy();
            }
            super::debug_panel::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_debug_panel();
                }
                self.sync_cursor_policy();
            }
            super::spawn_panel::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_spawn_panel(self.vfs.as_deref());
                }
                self.sync_cursor_policy();
            }
            super::model_viewer::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_model_viewer();
                }
                self.sync_cursor_policy();
            }
            super::changelog::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_changelog();
                }
                self.sync_cursor_policy();
            }
            super::credits::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_credits();
                }
                self.sync_cursor_policy();
            }
            super::update_panel::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_update_panel();
                }
                self.sync_cursor_policy();
            }
            // The SJK UI shows them as the Profile and Collection screens' tabs.
            crate::console::PROFILE_COMMAND if self.in_game_menu.is_sjk() => {
                self.toggle_profile_hub(crate::profile_hub::Tab::Profile);
            }
            crate::console::PROFILE_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_profile_panel();
                }
                self.sync_cursor_policy();
            }
            crate::console::COLLECTION_COMMAND if self.in_game_menu.is_sjk() => {
                let tab = self
                    .console
                    .as_ref()
                    .map_or(crate::profile_hub::Tab::Medals, |console| {
                        console.collection_tab()
                    });
                self.toggle_profile_hub(tab);
            }
            crate::console::COLLECTION_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_collection_panel(None);
                }
                self.sync_cursor_policy();
            }
            crate::emotes::COMMAND => return self.emote_command(args),
            crate::boombox::TRACK_COMMAND => return self.boombox_command(args),
            crate::propgun::PICK_COMMAND => return self.propgun_command(args),
            crate::toy_switch_words::COMMAND => return Ok(crate::toy_switch_words::help_lines()),
            crate::block_command::COMMAND => return self.block_command(args),
            crate::block_command::UNBLOCK_COMMAND => return self.unblock_command(args),
            crate::sound_mutes::COMMAND => return self.mute_sounds_command(args),
            crate::sound_mutes::UNMUTE_COMMAND => return self.unmute_sounds_command(args),
            crate::medal_popup::rehearsal::COMMAND => return self.debug_medal_command(args),
            crate::holocrons::rehearsal::COMMAND => return self.debug_holocron_command(args),
            crate::cards::rehearsal::COMMAND => return self.debug_open_command(args),
            crate::chat::find_blade::COMMAND => return self.debug_find_command(args),
            crate::unlock_toast::UPDATE_COMMAND => return self.debug_update_command(args),
            crate::official::ANNOUNCEMENT_COMMAND => return self.announcement_command(args),
            crate::official::REWARDS_COMMAND => return self.rewards_command(),
            crate::official::ANNOUNCE_COMMAND => return self.announce_command(),
            crate::official::DEBUG_ANNOUNCE_COMMAND => return self.debug_announce_command(args),
            crate::official::DEBUG_REWARD_COMMAND => return self.debug_reward_command(args),
            crate::saber_skin_command::COMMAND => return self.saber_skin_command(args),
            crate::blade_load_all::COMMAND => return Ok(crate::blade_load_all::command()),
            crate::saber_lab::COMMAND => return Ok(self.saber_lab_command(args)),
            crate::console::SJK_CHAT_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_sjk_chat_panel();
                }
                self.sync_cursor_policy();
            }
            crate::console::STAFF_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.staff_command(&args.join(" "));
                }
                self.sync_cursor_policy();
            }
            crate::console::UNLOCKABLES_COMMAND if self.in_game_menu.is_sjk() => {
                self.toggle_profile_hub(crate::profile_hub::Tab::Shaders);
            }
            crate::console::UNLOCKABLES_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_collection_panel(Some(crate::profile_hub::Tab::Shaders));
                }
                self.sync_cursor_policy();
            }
            crate::console::HOLOCRONS_COMMAND if self.in_game_menu.is_sjk() => {
                self.toggle_profile_hub(crate::profile_hub::Tab::Holocrons);
            }
            crate::console::HOLOCRONS_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_holocrons_panel();
                }
                self.sync_cursor_policy();
            }
            crate::console::ACHIEVEMENTS_COMMAND if self.in_game_menu.is_sjk() => {
                self.toggle_profile_hub(crate::profile_hub::Tab::Achievements);
            }
            crate::console::ACHIEVEMENTS_COMMAND => {
                if let Some(console) = &mut self.console {
                    console.toggle_collection_panel(Some(crate::profile_hub::Tab::Achievements));
                }
                self.sync_cursor_policy();
            }
            crate::identity_command::COMMAND if args.is_empty() => {
                if let Some(console) = &mut self.console {
                    console.toggle_identity_panel();
                }
                self.sync_cursor_policy();
            }
            crate::identity_command::COMMAND => return self.identity_command(args),
            crate::avatar_command::COMMAND => return self.avatar_command(args),
            crate::weather::COMMAND => return self.weather_command(args),
            crate::quick_wheel::OPEN_COMMAND => return self.open_quick_wheel(args),
            crate::quick_wheel::RUN_COMMAND | crate::quick_wheel::GIF_RUN_COMMAND => {
                return self.release_quick_wheel();
            }
            crate::quick_wheel::GIF_OPEN_COMMAND => return self.open_gif_wheel(),
            crate::console::SJK_SAY_COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return console.sjk_say_command(args);
            }
            crate::console::SJK_GIF_COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return console.sjk_gif_command(args);
            }
            crate::hud::nameplate::COMMAND => return self.nameplate_command(args),
            crate::graphics_quality::COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return crate::graphics_quality::command(console, args);
            }
            crate::graphics_quality::RENDERER_COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return crate::graphics_quality::renderer_command(console, args);
            }
            crate::graphics_quality::RECOMMEND_COMMAND => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return crate::graphics_quality::recommend_command(console);
            }
            crate::menu::first_setup::COMMAND
                if args.first().is_some_and(|arg| {
                    arg.eq_ignore_ascii_case(crate::menu::first_setup::IMPORT)
                }) =>
            {
                // A path with spaces may come unquoted: its words are joined again.
                let path = args[1..].join(" ");
                if let Some(console) = &mut self.console {
                    console.open_config_import(
                        (!path.is_empty()).then(|| std::path::Path::new(&path)),
                    );
                }
                self.sync_cursor_policy();
            }
            crate::menu::first_setup::COMMAND => {
                if let Some(console) = &mut self.console {
                    console.set_open(false);
                }
                let in_game = self.live_session.is_some();
                if let (Some(menu), Some(console)) = (&mut self.client_menu, &self.console) {
                    let target = if in_game {
                        crate::player_menu::ReturnTarget::InGame
                    } else {
                        crate::player_menu::ReturnTarget::MainMenu
                    };
                    menu.open_first_setup(console, target);
                    self.game_menu = false;
                }
                self.sync_cursor_policy();
            }
            "togglemenu" => {
                if self.game_menu {
                    self.game_menu = false;
                    self.sync_cursor_policy();
                } else if self.live_session.is_some() {
                    self.release_pointer();
                    self.game_menu = true;
                    self.game_menu_page = crate::ingame_menu::Page::Main;
                    self.game_menu_row = 0;
                } else {
                    return Err("No game session; the client menu is already available".into());
                }
            }
            "minimize" => {
                self.window
                    .as_ref()
                    .ok_or("No window to minimize")?
                    .set_minimized(true);
            }
            "fakenoclip" => return self.fake_noclip_command(),
            crate::ghost_replay::COMMAND | crate::mimic::COMMAND if !self.showcase_enabled() => {
                return Err(crate::ghost_replay::showcase_off(&name));
            }
            crate::ghost_replay::COMMAND => return self.ghost_command(args),
            crate::mimic::COMMAND => return self.mimic_command(args),
            crate::fps_help::COMMAND => return self.fps_help_command(),
            crate::hud_edit::COMMAND => return self.hud_edit_command(args),
            crate::hud::VOTE_PREVIEW_COMMAND => {
                return self.hud.preview_vote_command(args).map(|line| vec![line]);
            }
            crate::map_edits::SYNC_COMMAND => return self.map_edits_sync_command(),
            crate::emote_help::COMMAND => return self.emote_help_command(),
            crate::emote_picker::COMMAND => return self.emote_picker_command(),
            crate::news_panel::COMMAND => {
                self.preview_whats_new();
                return Ok(Vec::new());
            }
            crate::news_panel::NEWS_COMMAND => return self.news_command(),
            crate::gameplay_fps::COMMAND => return self.gameplay_fps_command(args),
            crate::tickrate_help::COMMAND => return self.tickrate_help_command(),
            crate::fs_restart::COMMAND => return self.fs_restart_command(),
            crate::rig_reload::COMMAND => return self.rig_reload_command(),
            crate::chat_gifs::hidden::UNHIDE_ALL => {
                let count = crate::chat_gifs::hidden::clear();
                return Ok(vec![crate::chat_gifs::hidden::unhide_all_reply(count)]);
            }
            "freecam" => return self.free_camera_command(args),
            "peek" => return self.peek_command(args),
            "cosmetics" => return self.cosmetics_command(args),
            crate::saber_colour_command::COMMAND => return self.saber_colour_command(args),
            crate::saber_hilt_command::COMMAND => return self.saber_hilt_command(args),
            "vid_restart" | "snd_restart" | "in_restart" | "modelist" => {
                return self.restart_command(&name, args, audio);
            }
            "ping" | "serverstatus" | "globalservers" | "localservers" | "rcon" | "showip" => {
                let server = self.live_session.as_ref().map(|session| session.server());
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                queries::start(console, &name, args, server)?;
            }
            "addfavorite" => {
                let address = match args {
                    [address] => queries::address(address, 29070)?,
                    [] => self
                        .live_session
                        .as_ref()
                        .ok_or("Not connected to a server.")?
                        .server(),
                    _ => return Err("usage: addFavorite [address]".into()),
                };
                let menu = self
                    .client_menu
                    .as_mut()
                    .ok_or("Server browser unavailable")?;
                menu.add_favorite(address)?;
                return Ok(vec![format!("Added favorite {address}")]);
            }
            _ => {
                let console = self.console.as_mut().ok_or("Console unavailable")?;
                return console.info_command(&name, args, self.live_session.as_mut());
            }
        }
        Ok(Vec::new())
    }
}

/// `PMF_FOLLOW` (`bg_public.h`): the player state is the followed player's.
const PMF_FOLLOW: u16 = 4096;

/// Who `messagemode3` whispers to: while following a player (spectating, `PMF_FOLLOW`),
/// the player followed, as EternalJK's `Con_MessageMode3_f` sends to
/// `cl.snap.ps.clientNum` (`cl_console.cpp:121`); else the player under the crosshair
/// (`crosshair`). Kushina's report, 22/10/2026: the bind did nothing while spectating.
fn whisper_slot(player: &sjk_protocol::PlayerState, crosshair: Option<u16>) -> Option<u16> {
    if player.movement_flags() & PMF_FOLLOW != 0 {
        Some(player.client_num())
    } else {
        crosshair
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_whisper_bind_goes_to_the_followed_player_while_spectating() {
        let mut player = sjk_protocol::PlayerState::default();
        player.set_client_num(5);
        assert_eq!(super::whisper_slot(&player, Some(2)), Some(2));
        assert_eq!(super::whisper_slot(&player, None), None);
        player.set_movement_flags(super::PMF_FOLLOW);
        assert_eq!(super::whisper_slot(&player, None), Some(5));
        assert_eq!(super::whisper_slot(&player, Some(2)), Some(5));
    }
    use super::*;

    fn import_path(command: &str) -> Option<String> {
        raw_import_path(command, &sjk_shell::tokenize(command).unwrap())
    }

    #[test]
    fn an_import_path_keeps_its_backslashes_quoted_or_not() {
        let quoted = r#"firstsetup import "C:\Users\turin\Downloads\turin.cfg""#;
        assert_eq!(
            import_path(quoted).as_deref(),
            Some(r"C:\Users\turin\Downloads\turin.cfg")
        );
        let spaced = r"firstsetup IMPORT  C:\My Games\jampconfig.cfg ";
        assert_eq!(
            import_path(spaced).as_deref(),
            Some(r"C:\My Games\jampconfig.cfg")
        );
        assert_eq!(import_path("firstsetup import"), None);
        assert_eq!(import_path("firstsetup"), None);
    }

    #[test]
    fn every_registered_command_reaches_the_client() {
        // `mapedit` was registered but never queued, so it answered "Viewer
        // command dispatcher required".
        for &(name, _) in all_commands() {
            let mut commands = Commands::default();
            let tokens = vec![name.to_owned(), "1".to_owned()];
            assert!(
                matches!(commands.queue(name, &tokens), Some(Ok(_))),
                "{name} is not queued"
            );
            assert_eq!(commands.pending.len(), 1, "{name}");
        }
        let mut commands = Commands::default();
        let tokens = vec!["MapEdit_Panel".to_owned()];
        assert!(commands.queue("MapEdit_Panel", &tokens).is_some());
    }
}
