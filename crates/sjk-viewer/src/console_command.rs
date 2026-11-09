//! Console command buffering and application-action handoff.

use super::*;

impl ViewerConsole {
    pub(crate) fn mod_chat_commands(&self) -> &[&'static str] {
        self.mods.chat_commands()
    }

    pub(crate) fn drain_input(&mut self) -> std::vec::Drain<'_, String> {
        self.pending_input.drain(..)
    }
    pub(crate) fn input_millis(&self) -> u64 {
        self.shell.command_clock_millis()
    }
    pub(crate) fn advance_command_frame(&mut self) {
        self.shell.advance_command_frame();
    }
    pub(crate) fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.pending_quit)
    }
    /// Whether the Profile page's Change name was pressed since the last call.
    pub(crate) fn take_name_edit(&mut self) -> bool {
        std::mem::take(&mut self.pending_name_edit)
    }
    /// Attach the mounted search path used after the writable config directory.
    pub(crate) fn attach_script_vfs(&mut self, vfs: Arc<sjk_vfs::VirtualFileSystem>) {
        self.script_vfs = Some(vfs);
    }
    /// `fs_restart`'s mount in flight, kept with the console across map changes.
    pub(crate) fn fs_restart_task(&mut self) -> &mut crate::fs_restart::Task {
        &mut self.client_commands.fs_restart
    }
    /// `rig_reload`'s mount in flight.
    pub(crate) fn rig_reload_task(&mut self) -> &mut crate::fs_restart::Task {
        &mut self.client_commands.rig_reload
    }

    /// Queue and immediately execute typed console input.
    pub(crate) fn execute_console_line(
        &mut self,
        input: &str,
        session: Option<&mut ClientSession>,
    ) {
        // A chat message typed here stays out of crash reports and logs, as
        // received chat does (`crash_report::CHAT_COMMANDS` and the mods').
        let queued = if crate::crash_report::typed_chat(input, self.mods.chat_commands()) {
            crate::crash_report::privately(|| self.shell.queue_line(input))
        } else {
            self.shell.queue_line(input)
        };
        if let Err(error) = queued {
            self.shell.push_log(format!("^1{error}"));
            return;
        }
        self.execute_buffered_frame(session, |_, _| false);
    }

    /// Execute the frame's queued cfg/bind commands in OpenJK buffer order.
    pub(crate) fn execute_buffered_frame(
        &mut self,
        mut session: Option<&mut ClientSession>,
        mut application_command: impl FnMut(&str, &[String]) -> bool,
    ) {
        // A switch flipped since the last frame adds or removes its mod's commands.
        if self.mods.sync(&mut self.shell) {
            self.reset_unloaded_dialogue();
            self.userinfo_dirty
                .store(true, std::sync::atomic::Ordering::Release);
            self.refresh_server_help();
        }
        // The JoF folder joins or leaves the search paths with its switch.
        #[cfg(feature = "mod-jof")]
        crate::jof_content::follow(self);
        if !self.shell.has_buffered_commands() {
            return;
        }
        let mut resolver =
            console_scripting::Resolver::new(&self.config_directory, self.script_vfs.as_deref());
        let chat_mask = self.integer_cvar("cl_colorstring").unwrap_or(0) as u16 & 1023;
        let chat_random = self
            .integer_cvar("cl_colorstringrandom")
            .unwrap_or(2)
            .clamp(1, 1000) as u32;
        let pending_connection = &mut self.pending_connection;
        let pending_demo = &mut self.pending_demo;
        let pending_screenshot = &mut self.pending_screenshot;
        let pending_viewpos = &mut self.pending_viewpos;
        let pending_mark = &mut self.pending_mark;
        let pending_quit = &mut self.pending_quit;
        let exit_command = &self.qcommon.exit_command;
        let pending_input = &mut self.pending_input;
        let pending_chat = &mut self.pending_chat;
        let client_commands = &mut self.client_commands;
        let mods = &self.mods;
        let director = &mut self.director;
        let userinfo_names: Vec<_> = self
            .shell
            .cvars
            .iter()
            .filter(|v| {
                v.flags.contains(CvarFlags::USER_INFO) && !v.flags.contains(CvarFlags::USER_CREATED)
            })
            .map(|v| v.name.clone())
            .collect();
        let result = self.shell.execute_buffered_frame_with_cvars(
            &mut resolver,
            |command, tokens, local, cvars| {
                if tokens[0].eq_ignore_ascii_case("sets") {
                    return Some(Err("sets: no local serverinfo owner/listen server".into()));
                }
                if let Some(result) = crate::cosmetics::typed_colour::apply(tokens, cvars) {
                    return Some(result);
                }
                if tokens[0].eq_ignore_ascii_case("setu")
                    && tokens.len() > 2
                    && !userinfo_names
                        .iter()
                        .any(|name| name.eq_ignore_ascii_case(&tokens[1]))
                {
                    return Some(Err(
                        "setu: custom userinfo keys lack a protocol adapter".into()
                    ));
                }
                // OpenJK common.cpp:1210 (Com_Quit_f); exit is a client alias.
                if tokens.first().is_some_and(|n| {
                    n.eq_ignore_ascii_case("quit") || n.eq_ignore_ascii_case("exit")
                }) {
                    if tokens[0].eq_ignore_ascii_case("exit") && !exit_command.enabled() {
                        return Some(Err("exit disabled by cl_exitCommand; use quit".into()));
                    }
                    *pending_quit = true;
                    return Some(Ok(Vec::new()));
                }
                match console_connect::parse(tokens) {
                    Ok(Some(action)) => {
                        *pending_connection = Some(action);
                        return Some(Ok(Vec::new()));
                    }
                    Ok(None) => {}
                    Err(error) => return Some(Err(error.to_string())),
                }
                match crate::screenshot::parse_request(tokens) {
                    Ok(Some(request)) => {
                        *pending_screenshot = Some(request);
                        return Some(Ok(Vec::new()));
                    }
                    Ok(None) => {}
                    Err(error) => return Some(Err(error)),
                }
                if tokens
                    .first()
                    .is_some_and(|name| name.eq_ignore_ascii_case("viewpos"))
                {
                    *pending_viewpos = true;
                    return Some(Ok(Vec::new()));
                }
                // `perfmark <text>`: a timestamped line in the client log, here so a
                // script's markers keep their order against its `wait`s.
                if tokens
                    .first()
                    .is_some_and(|name| name.eq_ignore_ascii_case("perfmark"))
                {
                    let text = tokens[1..].join(" ");
                    crate::log::progress(format_args!("perf-mark {text}"));
                    crate::frame_timings::note(format_args!("mark {text}"));
                    return Some(Ok(vec![format!("perf-mark {text}")]));
                }
                // `mark <note>`: `viewpos`, kept with the note for whoever reads the marks.
                if tokens
                    .first()
                    .is_some_and(|name| name.eq_ignore_ascii_case("mark"))
                {
                    *pending_mark = Some(tokens[1..].join(" "));
                    return Some(Ok(Vec::new()));
                }
                match console_demo::parse(tokens) {
                    Ok(Some(action)) => {
                        *pending_demo = Some(action);
                        return Some(Ok(Vec::new()));
                    }
                    Ok(None) => {}
                    Err(error) => return Some(Err(error)),
                }
                if let Some(result) = director.command(tokens) {
                    return Some(result);
                }
                if application_command(command, tokens) {
                    return Some(Ok(Vec::new()));
                }
                // A mod's command runs with the client's frame state, as the
                // client commands do; one whose mod is off answers here. The
                // client's own commands and the mods' settings come first, even
                // under a mod's prefix.
                let mod_answer =
                    if console_client::known(&tokens[0]) || cvars.get(&tokens[0]).is_some() {
                        None
                    } else {
                        mods.accepts(&tokens[0])
                    };
                match mod_answer {
                    Some(Ok(())) if client_commands.pending.len() == 64 => {
                        return Some(Err("Client command queue is full".into()));
                    }
                    Some(Ok(())) => {
                        client_commands.pending.push_back(tokens.to_vec());
                        return Some(Ok(Vec::new()));
                    }
                    Some(Err(error)) => return Some(Err(error)),
                    None => {}
                }
                if let Some(result) = client_commands.queue(command, tokens) {
                    return Some(result);
                }
                if crate::input::GameplayInput::recognizes(command) {
                    if pending_input.len() == 64 {
                        return Some(Err("Input command queue is full".into()));
                    }
                    pending_input.push(command.to_owned());
                    return Some(Ok(Vec::new()));
                }
                if local {
                    return None;
                }
                Some(match forward_payload(command, tokens, session.is_some()) {
                    ForwardAction::Ignore => Ok(Vec::new()),
                    ForwardAction::Unknown => Err(format!("Unknown command \"{}\"", tokens[0])),
                    ForwardAction::Reliable(payload) => {
                        // JA+ style `tell <name>` is resolved against the roster
                        // first; chat is then coloured and queued like stock.
                        let resolved;
                        let payload = match session.as_deref() {
                            Some(active) if tokens[0].eq_ignore_ascii_case("tell") => {
                                let mut roster = sjk_client::ChatRoster::default();
                                roster.update(active.game_state());
                                match console_tell::resolve(tokens, &roster) {
                                    console_tell::Tell::Unchanged => payload,
                                    console_tell::Tell::Send(command) => {
                                        resolved = command;
                                        &resolved
                                    }
                                    console_tell::Tell::Refused(lines) => {
                                        return Some(Ok(lines));
                                    }
                                }
                            }
                            _ => payload,
                        };
                        if ["say", "say_team", "tell", "amsay"]
                            .iter()
                            .any(|name| tokens[0].eq_ignore_ascii_case(name))
                            || mods
                                .chat_commands()
                                .iter()
                                .any(|name| tokens[0].eq_ignore_ascii_case(name))
                        {
                            if pending_chat.len() >= 64 {
                                return Some(Err("Chat command queue is full".into()));
                            }
                            // Keep the frame's colour policy while expanding the
                            // message later against the presented player state.
                            pending_chat.push_back((payload.to_owned(), chat_mask, chat_random));
                            return Some(Ok(Vec::new()));
                        }
                        let Some(active) = session.as_deref_mut() else {
                            return Some(Err(format!("Unknown command \"{}\"", tokens[0])));
                        };
                        active
                            .send_reliable_command_from(
                                payload.as_bytes(),
                                sjk_client::ReliableCommandOrigin::Input,
                            )
                            .map(|()| Vec::new())
                            .map_err(|error| format!("Could not forward {}: {error}", tokens[0]))
                    }
                })
            },
        );
        if let Err(error) = result {
            crate::log::progress(format_args!("console command: {error}"));
        }
        self.persist();
    }

    /// `mods`: each client mod with its switch and whether it is on.
    pub(crate) fn mods_listing(&self) -> Vec<String> {
        let list = self.mods.listing();
        if list.is_empty() {
            return vec!["This build carries no client mods.".into()];
        }
        list.into_iter()
            .map(|(id, title, about, state)| {
                format!(
                    "^5{title}^7 ({}, {id}.*) {state}^7: {about}",
                    crate::mods::switch(id)
                )
            })
            .collect()
    }

    /// Consume one local connection action after console input releases its borrows.
    pub(crate) fn take_connection_action(&mut self) -> Option<ConnectionAction> {
        self.pending_connection.take()
    }

    /// Answer `viewpos` or `mark` with `pose`, the rendered camera's: logged, put on the
    /// clipboard, and for a mark appended with its note and `place` (the map) to the marks
    /// file, so a report of "this looks wrong here" needs no copying of numbers.
    pub(crate) fn answer_pose(&mut self, pose: &str, place: &str) {
        let viewpos = std::mem::take(&mut self.pending_viewpos);
        let Some(line) = self
            .pending_mark
            .take()
            .map(|note| format!("{pose} | {place} | {note}"))
            .or_else(|| viewpos.then(|| pose.to_owned()))
        else {
            return;
        };
        if line.len() > pose.len() {
            use std::io::Write;
            let kept = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.marks)
                .and_then(|mut file| writeln!(file, "{line}"));
            crate::log::progress(format_args!("mark: {line}"));
            self.push_log(match kept {
                Ok(()) => format!("marked: {line}"),
                Err(error) => format!("^1mark not kept in {}: {error}", self.marks.display()),
            });
        } else {
            self.push_log(line.clone());
        }
        // A test run leaves the desktop's clipboard alone.

        clipboard::copy(&line);
        self.copied = line;
    }

    /// Whether a pose was asked for since the last answer.
    pub(crate) fn wants_pose(&self) -> bool {
        self.pending_viewpos || self.pending_mark.is_some()
    }

    /// Consume one renderer screenshot request.
    pub(crate) fn take_screenshot_request(&mut self) -> Option<crate::screenshot::Request> {
        self.pending_screenshot.take()
    }

    /// Consume one local demo action after console input releases its borrows.
    pub(crate) fn take_demo_action(&mut self) -> Option<DemoAction> {
        self.pending_demo.take()
    }

    pub(crate) fn close_for_connection(&mut self) {
        self.set_open(false);
    }

    pub(crate) fn clear_server_info(&mut self) {
        self.reset_dialogue();
        self.movement_policy_log = None;
        // Off a server no mod is loaded.
        self.mods.set_server(None);
        self.server_profile = None;
        self.server_cap = None;
        self.server_family = None;
        self.input_health.reset();
        self.mods.sync(&mut self.shell);
        self.refresh_server_help();
        // Off a server the profile goes out as written, with nothing pending.
        self.force_profile.reset();
        if let Ok(mut status) = self.server_status.write() {
            *status = "Not connected".to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_mod_command_waits_for_its_server() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("japlus.guntele", None);
        assert!(console.client_commands.pending.is_empty());
        assert!(
            console
                .shell
                .lines()
                .any(|line| line.text.contains("JA+ servers only"))
        );
        console.mods.set_server(Some(sjk_mod::Server {
            kind: sjk_mod::ServerKind::JaPlus,
            address: "127.0.0.1:29070".into(),
            info: String::new(),
        }));
        console.execute_console_line("japlus.guntele 100", None);
        assert_eq!(
            console.client_commands.pending.back().map(Vec::as_slice),
            Some(&["japlus.guntele".to_owned(), "100".to_owned()][..])
        );
        assert!(console.mods_listing()[0].contains("^2loaded"));
    }

    /// Hub report #218: what a quick wheel choice queues (`queue_command`, as a
    /// bind's script) keeps its arguments, for a mod's command and for one the
    /// server is sent.
    #[test]
    fn a_queued_command_keeps_its_arguments() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.mods.set_server(Some(sjk_mod::Server {
            kind: sjk_mod::ServerKind::JaPlus,
            address: "127.0.0.1:29070".into(),
            info: String::new(),
        }));
        console.queue_command("japlus.guntele 300 45").unwrap();
        console.execute_buffered_frame(None, |_, _| false);
        assert_eq!(
            console.client_commands.pending.back().map(Vec::as_slice),
            Some(
                &[
                    "japlus.guntele".to_owned(),
                    "300".to_owned(),
                    "45".to_owned()
                ][..]
            )
        );
        console.queue_command("japlus.guntelewall 300").unwrap();
        console.execute_buffered_frame(None, |_, _| false);
        assert_eq!(
            console.client_commands.pending.back().map(Vec::as_slice),
            Some(&["japlus.guntelewall".to_owned(), "300".to_owned()][..])
        );
        let tokens = ["amtele".to_owned(), "3".to_owned()];
        assert_eq!(
            super::forward_payload("amtele 3", &tokens, true),
            super::ForwardAction::Reliable("amtele 3")
        );
    }

    /// `jof.downloads` is the client's own command under the JoF tools' prefix:
    /// it runs off a JoF server, where the mod's commands do not.
    #[cfg(feature = "mod-jof")]
    #[test]
    fn a_client_command_under_a_mod_prefix_runs_anywhere() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("jof.downloads", None);
        assert_eq!(
            console.client_commands.pending.back().map(Vec::as_slice),
            Some(&["jof.downloads".to_owned()][..])
        );
        assert!(
            !console
                .shell
                .lines()
                .any(|line| line.text.contains("Unknown command"))
        );
        // The mod's own commands still wait for a JoF server.
        console.execute_console_line("jof.commands", None);
        assert_eq!(console.client_commands.pending.len(), 1);
    }

    /// A mod's setting is set from the console like any cvar, off its servers too.
    #[cfg(feature = "mod-jof")]
    #[test]
    fn a_mod_setting_is_set_from_the_console() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("jof.e2eWhispers 1", None);
        assert_eq!(console.bool_cvar("jof.e2eWhispers"), Some(true));
        assert!(
            !console
                .shell
                .lines()
                .any(|line| line.text.contains("Unknown command"))
        );
    }

    #[test]
    fn perfmark_answers_in_the_console() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line("perfmark lamps off", None);
        assert!(
            console
                .shell
                .lines()
                .any(|line| line.text == "perf-mark lamps off"),
        );
    }
    #[test]
    fn buffered_colours_observe_prior_changes_and_never_overwrite_later_names() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.set_cvar("color1", "4oldhat");
        console.execute_console_line("color1 3newhat; color1 2", None);
        assert_eq!(console.text_value("color1"), Some("2newhat"));
        console.execute_console_line("color1 5; color1 1lasthat", None);
        assert_eq!(console.text_value("color1"), Some("1lasthat"));
        console.execute_console_line("color1 6; color1 3; color1", None);
        assert_eq!(console.text_value("color1"), Some("3lasthat"));
        assert!(
            console
                .shell
                .lines()
                .any(|line| line.text.contains("color1 = \"3lasthat\" (default"))
        );
    }

    #[test]
    fn vstr_and_wait_keep_the_same_order_and_cvar_notifications() {
        let directory = tempfile::tempdir().unwrap();
        let mut console =
            crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        console.execute_console_line(
            "set recolour \"color1 3newhat; color1 2\"; vstr recolour; wait; color1 6",
            None,
        );
        assert_eq!(console.text_value("color1"), Some("2newhat"));
        console.advance_command_frame();
        console.execute_buffered_frame(None, |_, _| false);
        assert_eq!(console.text_value("color1"), Some("6newhat"));
        console
            .userinfo_dirty
            .store(false, std::sync::atomic::Ordering::Release);
        console.execute_console_line("seta color1 4; setu color2 3cape; setu color2 5", None);
        assert_eq!(console.text_value("color1"), Some("4newhat"));
        assert_eq!(console.text_value("color2"), Some("5cape"));
        assert!(
            console
                .userinfo_dirty
                .load(std::sync::atomic::Ordering::Acquire)
        );
        assert!(
            console
                .shell
                .cvars
                .get("color1")
                .unwrap()
                .flags
                .contains(sjk_shell::CvarFlags::ARCHIVE)
        );
        assert!(
            console
                .shell
                .cvars
                .get("color2")
                .unwrap()
                .flags
                .contains(sjk_shell::CvarFlags::USER_INFO)
        );
    }
}
