//! Integrated command console over cvars, binds, and persistence.

use crate::{
    AliasTable, BindTable, CommandBuffer, CommandFileResolver, CommandRegistry, CvarRegistry,
    NoCommandFiles, ShellError, load_config, tokenize,
};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[path = "shell_alias.rs"]
mod alias_commands;
#[path = "shell_builtins.rs"]
mod builtins;
#[path = "shell_completion.rs"]
mod completion;
#[path = "shell_cvar_commands.rs"]
mod cvar_commands;
#[path = "shell_dispatch.rs"]
mod dispatch;
#[path = "shell_file_log.rs"]
mod file_log;
#[path = "shell_schedule.rs"]
mod schedule;

pub use completion::{ArgumentSource, CompletionKey, MAP_EXTENSION};

const DEFAULT_LOG_CAPACITY: usize = 256;
/// Bytes of the `[HH:MM:SS] ` prefix of [`ConsoleLine::stamped_text`].
const STAMP_PREFIX_BYTES: usize = 11;
/// Echoed before typed input, as `Console_Key` prints `CONSOLE_PROMPT_CHAR`
/// straight before the line: `]cmd`.
pub(crate) const PROMPT: &str = "]";

/// Origin/severity of a console line.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsoleLineKind {
    /// A command entered by the user.
    Input,
    /// Normal command output.
    Output,
    /// Parsing, validation, or handler failure.
    Error,
    /// Application-originated diagnostic text.
    Log,
}

/// Where a command listed by [`Shell::command_help`] is handled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandSource {
    /// A command of the shell itself (`bind`, `set`, `exec`, ...).
    Shell,
    /// A command the application registered.
    Application,
    /// A command forwarded to an external processor, such as the game server.
    External,
}

/// One bounded console scrollback entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsoleLine {
    /// Monotonic shell-clock timestamp for expiring notify text.
    pub written_millis: u64,
    /// Timestamped display copy, prepared once when the line is appended: the
    /// local time it was written as `[HH:MM:SS] `, then [`Self::text`].
    pub stamped_text: String,
    /// Origin/severity used by frontends for presentation.
    pub kind: ConsoleLineKind,
    /// Console-formatted text.
    pub text: String,
    /// Whether the line may show among the transient notify lines while the
    /// console is closed. Stock `CL_ConsolePrint` (`cl_console.cpp`) clears this
    /// for text starting with `*` or `[skipnotify]`, which cgame uses for chat.
    pub notify: bool,
}

impl ConsoleLine {
    /// The local time the line was written, `HH:MM:SS`, as EternalJK draws it
    /// in the console's left column.
    pub fn clock(&self) -> &str {
        self.stamped_text.get(1..9).unwrap_or("")
    }
}

/// Complete portable shell state used by a client frontend.
pub struct Shell {
    schedule: schedule::Schedule,
    /// Typed configuration registry.
    pub cvars: CvarRegistry,
    /// Application/mod command handlers.
    pub commands: CommandRegistry,
    /// Physical-input command bindings.
    pub binds: BindTable,
    /// The player's `alias` command lines, kept in `aliases.cfg` beside the config.
    pub aliases: AliasTable,
    aliases_state: alias_commands::AliasFileState,
    config_path: Option<PathBuf>,
    config_load_failed: bool,
    config_saver: crate::config_saver::ConfigSaver,
    /// Tells the shell's own saves from another program's edit of the file.
    config_watch: crate::config_watch::ConfigWatch,
    /// The file was deleted while running: the stale state is not written back.
    config_removed: bool,
    file_log: file_log::FileLog,
    /// Sees every line as it is kept ([`Shell::set_line_tap`]).
    line_tap: Option<fn(&str)>,
    lines: VecDeque<ConsoleLine>,
    log_capacity: usize,
    /// Lines ever appended, so frontends can name a line across scrollback trimming.
    lines_written: u64,
    external_command_help: BTreeMap<String, String>,
    command_buffer: CommandBuffer,
}

impl Shell {
    /// Compose shell state around application-defined cvars and default binds.
    pub fn new(cvars: CvarRegistry, binds: BindTable) -> Self {
        Self {
            schedule: schedule::Schedule::default(),
            cvars,
            commands: CommandRegistry::new(),
            binds,
            aliases: AliasTable::new(),
            aliases_state: Default::default(),
            config_path: None,
            config_load_failed: false,
            config_saver: Default::default(),
            config_watch: Default::default(),
            config_removed: false,
            file_log: file_log::FileLog::default(),
            line_tap: None,
            lines: VecDeque::with_capacity(DEFAULT_LOG_CAPACITY),
            log_capacity: DEFAULT_LOG_CAPACITY,
            lines_written: 0,
            external_command_help: BTreeMap::new(),
            command_buffer: CommandBuffer::new(),
        }
    }

    /// Select the explicit application-owned persistence file.
    pub fn set_config_path(&mut self, path: impl Into<PathBuf>) {
        self.config_path = Some(path.into());
    }

    /// Return the selected persistence file, if any.
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Restore archived cvars and the complete bind table.
    pub fn load(&mut self) -> Result<(), ShellError> {
        self.load_aliases_once();
        let path = self
            .config_path
            .as_deref()
            .ok_or(ShellError::NoConfigPath)?;
        let result = load_config(path, &mut self.cvars, &mut self.binds);
        self.config_load_failed = result.is_err();
        // Watched even after a failed load, so a file fixed while running loads.
        let bytes = std::fs::read(path).ok();
        self.config_watch.loaded(path, bytes.as_deref());
        self.config_removed = false;
        result?;
        // The file now holds exactly this state: nothing to save yet.
        self.config_saver.mark_saved(self.config_revision());
        Ok(())
    }

    fn config_revision(&self) -> crate::config_saver::Revision {
        (self.cvars.archive_revision(), self.binds.revision())
    }

    /// Whether archived cvars or binds changed since the config was loaded or saved.
    pub fn config_dirty(&self) -> bool {
        self.config_saver.is_dirty(self.config_revision())
    }

    /// Persist archived cvars and the complete bind table now, on this thread,
    /// after any background save still running. For exits and for code about to
    /// read the file back.
    pub fn save(&mut self) -> Result<(), ShellError> {
        // A failed earlier write is moot once this one lands.
        let _ = self.config_saver.wait_idle();
        self.save_aliases_if_changed()?;
        self.follow_external_config(None);
        if self.config_load_failed {
            return Err(ShellError::ConfigLoadFailed);
        }
        if self.config_removed {
            return Ok(());
        }
        let path = self
            .config_path
            .as_deref()
            .ok_or(ShellError::NoConfigPath)?;
        let current = self.config_revision();
        let contents = crate::config::config_text(&self.cvars, &self.binds)?;
        self.config_watch.writing(&contents);
        crate::config::write_config_text(path, &contents)?;
        self.config_saver.mark_saved(current);
        Ok(())
    }

    /// Save only if something saved changed, now, on this thread: [`Self::save`]
    /// when [`Self::config_dirty`].
    pub fn save_if_dirty(&mut self) -> Result<(), ShellError> {
        self.follow_external_config(None);
        if self.config_dirty() {
            self.save()
        } else {
            self.config_saver.wait_idle()?;
            Ok(())
        }
    }

    /// Once per frame: save archived cvars and binds once they changed and then
    /// held still for [`crate::CONFIG_SAVE_DELAY`]. The text is built here and
    /// written on a background thread; an unchanged frame costs two comparisons.
    /// Returns an error an earlier background write met, or why this one could not
    /// be queued (each reported once).
    pub fn autosave(&mut self, now: Instant) -> Result<(), ShellError> {
        if let Some(error) = self.config_saver.take_error() {
            return Err(error.into());
        }
        self.save_aliases_if_changed()?;
        self.follow_external_config(Some(now));
        let current = self.config_revision();
        if !self.config_saver.due(current, now) {
            return Ok(());
        }
        let Some(path) = self.config_path.clone() else {
            return Ok(());
        };
        if self.config_load_failed {
            // Once per change, as each save used to report it.
            self.config_saver.mark_saved(current);
            return Err(ShellError::ConfigLoadFailed);
        }
        if self.config_removed {
            self.config_saver.mark_saved(current);
            return Ok(());
        }
        let contents = crate::config::config_text(&self.cvars, &self.binds)?;
        self.config_watch.writing(&contents);
        self.config_saver.queue(current, path, contents)?;
        Ok(())
    }

    /// Take in a config file another program changed while the game ran
    /// (checked every [`crate::config_watch::CHECK_INTERVAL`] at `now`, or at once
    /// with `None`) instead of writing the values from before over it: an edited
    /// file is loaded as at start, and after a deletion nothing more is saved this
    /// run, so the next start begins on the defaults the player asked for.
    fn follow_external_config(&mut self, now: Option<Instant>) {
        use crate::config_watch::External;
        let Some(path) = self.config_path.clone() else {
            return;
        };
        // The disk is compared with this shell's own writes once they landed.
        if now.is_none() {
            self.config_saver.wait_writes();
        } else if self.config_saver.writing() {
            return;
        }
        match self.config_watch.check(&path, now) {
            None => {}
            Some(External::Edited) => match self.load() {
                Ok(()) => self.push_log(format!(
                    "^3{} was changed outside SJK: its settings are loaded",
                    path.display()
                )),
                Err(error) => self.push_log(format!(
                    "^1{} was changed outside SJK and could not be loaded: {error}",
                    path.display()
                )),
            },
            Some(External::Removed) => {
                self.config_removed = true;
                self.config_saver.mark_saved(self.config_revision());
                self.push_log(format!(
                    "^3{} was deleted while SJK ran: it is not written again, so SJK starts on its defaults next time (settings changed before then are not kept)",
                    path.display()
                ));
            }
        }
    }

    /// Iterate bounded scrollback from oldest to newest.
    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &ConsoleLine> {
        self.lines.iter()
    }

    /// Number of lines ever appended to scrollback. Lines are numbered from zero in
    /// the order they were appended, so the newest line in [`Self::lines`] is number
    /// `lines_written() - 1`. Numbers are never reused, even after trimming or
    /// clearing, so a frontend can keep one (for a text selection, say) and later
    /// find the line again or learn that it is gone.
    pub fn lines_written(&self) -> u64 {
        self.lines_written
    }

    /// The scrollback line numbered `number` (see [`Self::lines_written`]), if it
    /// is still kept.
    pub fn line(&self, number: u64) -> Option<&ConsoleLine> {
        let first = self.lines_written - self.lines.len() as u64;
        usize::try_from(number.checked_sub(first)?)
            .ok()
            .and_then(|index| self.lines.get(index))
    }

    /// Remove every scrollback line.
    pub fn clear_lines(&mut self) {
        self.lines.clear();
    }

    /// Append application-originated text to bounded scrollback.
    pub fn push_log(&mut self, text: impl Into<String>) {
        self.push_line(ConsoleLineKind::Log, text.into());
    }

    /// Hand every line the scrollback keeps to `tap` as well, as it is kept (a
    /// frontend's crash trail); `cl_noprint` silences it too.
    pub fn set_line_tap(&mut self, tap: fn(&str)) {
        self.line_tap = Some(tap);
    }

    /// Append application-originated text to scrollback only, never to the
    /// notify lines: the stock `*` print prefix cgame uses to echo chat.
    pub fn push_log_quiet(&mut self, text: impl Into<String>) {
        self.push_line_with(ConsoleLineKind::Log, text.into(), false);
    }

    /// Replace completion/help metadata for commands forwarded to an external
    /// command processor. These names remain unknown to local dispatch.
    pub fn replace_external_command_help(
        &mut self,
        commands: impl IntoIterator<Item = (String, String)>,
    ) {
        self.external_command_help.clear();
        self.external_command_help.extend(
            commands
                .into_iter()
                .map(|(name, description)| (name.to_ascii_lowercase(), description)),
        );
    }

    /// Every command `cmdlist` lists, with its description and where it runs, in
    /// no particular order.
    pub fn command_help(&self) -> impl Iterator<Item = (&str, &str, CommandSource)> {
        builtin_commands()
            .map(|(name, description)| (name, description, CommandSource::Shell))
            .chain(
                self.commands
                    .iter()
                    .map(|(name, description)| (name, description, CommandSource::Application)),
            )
            .chain(
                self.external_command_help
                    .iter()
                    .map(|(name, description)| {
                        (name.as_str(), description.as_str(), CommandSource::External)
                    }),
            )
    }

    /// Return whether the first token is handled by this shell rather than an
    /// external command processor.
    pub fn recognizes_local_command(&self, name: &str) -> bool {
        builtin_commands().any(|(builtin, _)| builtin.eq_ignore_ascii_case(name))
            || self.commands.contains(name)
            || self.cvars.get(name).is_some()
    }

    /// Return the unique case-insensitive command-name completion, including
    /// external commands advertised by the active application. Ambiguous
    /// prefixes return `None`; the scan does not allocate.
    pub fn unique_command_completion(&self, prefix: &str) -> Option<&str> {
        let mut found = None;
        for name in builtin_commands()
            .map(|(name, _)| name)
            .chain(self.commands.iter().map(|(name, _)| name))
            .chain(self.cvars.iter().map(|cvar| cvar.name.as_str()))
            .chain(self.external_command_help.keys().map(String::as_str))
            .chain(self.aliases.iter().map(|(name, _)| name))
            .filter(|name| ascii_starts_with_ignore_case(name, prefix))
        {
            if found.is_some_and(|current: &str| !current.eq_ignore_ascii_case(name)) {
                return None;
            }
            found = Some(name);
        }
        found
    }

    /// Queue one possibly-semicolon-separated line behind pending commands.
    pub fn queue_line(&mut self, input: &str) -> Result<(), ShellError> {
        let input = input.trim();
        if input.is_empty() {
            return Ok(());
        }
        let shown = crate::secret::input_line(input, &self.cvars);
        self.push_line(ConsoleLineKind::Input, format!("{PROMPT}{shown}"));
        self.command_buffer.append(input)?;
        Ok(())
    }

    /// Append command text without echoing it as typed console input.
    ///
    /// Frontends use this for physical-key bindings and generated scripts.
    pub fn queue_script(&mut self, input: &str) -> Result<(), ShellError> {
        self.command_buffer.append(input)?;
        Ok(())
    }

    /// Return whether a command or `wait` delay remains buffered.
    pub fn has_buffered_commands(&self) -> bool {
        !self.command_buffer.is_idle() || !self.schedule.pending.is_empty()
    }

    /// Execute one frame of buffered commands with application interception.
    ///
    /// The interceptor runs before built-in dispatch. Returning `Some` marks
    /// the command as application-owned; `None` continues local shell dispatch.
    /// Errors are logged and do not abort later commands, matching cfg behavior.
    pub fn execute_buffered_frame_with<R, F>(
        &mut self,
        resolver: &mut R,
        mut intercept: F,
    ) -> Result<Vec<String>, ShellError>
    where
        R: CommandFileResolver,
        F: FnMut(&str, &[String], bool) -> Option<Result<Vec<String>, String>>,
    {
        self.execute_buffered_frame_with_cvars(resolver, |text, tokens, local, _| {
            intercept(text, tokens, local)
        })
    }

    /// Execute buffered commands with access to the current cvar registry.
    ///
    /// The application interceptor runs immediately before each command, so
    /// compatibility rewrites observe earlier writes in the same script and can
    /// publish their result before later commands, `vstr` expansion or `wait` boundaries.
    /// Returning `None` continues ordinary built-in dispatch.
    pub fn execute_buffered_frame_with_cvars<R, F>(
        &mut self,
        resolver: &mut R,
        mut intercept: F,
    ) -> Result<Vec<String>, ShellError>
    where
        R: CommandFileResolver,
        F: FnMut(&str, &[String], bool, &mut CvarRegistry) -> Option<Result<Vec<String>, String>>,
    {
        self.resume_scheduled()?;
        if self.command_buffer.is_idle() {
            return Ok(Vec::new());
        }
        let mut output = Vec::new();
        let mut first_error = None;
        loop {
            // OpenJK checks `cmd_wait` at the head of Cbuf_Execute's loop and
            // decrements it before returning (`cmd.cpp:189-195`). Thus the
            // frame which executed `wait` consumes the first wait count.
            if self.command_buffer.consume_wait() {
                break;
            }
            let Some(queued) = self.command_buffer.pop_front() else {
                break;
            };
            let result = match tokenize(&queued.text) {
                Ok(tokens) => {
                    let Some((name, arguments)) = tokens.split_first() else {
                        continue;
                    };
                    let local = self.recognizes_local_command(name);
                    // An alias runs before the application sees the name: it is
                    // no command of the shell's or the application's (`alias`
                    // refuses those), so it would be forwarded to the server.
                    if !local && let Some(command) = self.aliases.get(name) {
                        let command = command.to_owned();
                        self.expand_alias(name, &command, queued.exec_depth)
                    } else if let Some(result) =
                        intercept(&queued.text, &tokens, local, &mut self.cvars)
                    {
                        result.map_err(ShellError::Application)
                    } else {
                        self.execute_buffered_one(name, arguments, queued.exec_depth, resolver)
                    }
                }
                Err(error) => Err(error.into()),
            };
            match result {
                Ok(lines) => {
                    for line in lines {
                        // Returned as kept: scrubbed of secrets (`secret.rs`), once.
                        let line = self.scrubbed(line);
                        self.keep_line(ConsoleLineKind::Output, line.clone(), true);
                        output.push(line);
                    }
                }
                Err(error) => {
                    self.push_line(ConsoleLineKind::Error, error.to_string());
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        first_error.map_or(Ok(output), Err)
    }

    /// Execute one buffered frame without an application interceptor.
    pub fn execute_buffered_frame<R: CommandFileResolver>(
        &mut self,
        resolver: &mut R,
    ) -> Result<Vec<String>, ShellError> {
        self.execute_buffered_frame_with(resolver, |_, _, _| None)
    }

    /// Execute one possibly-semicolon-separated console line immediately.
    pub fn execute_line(&mut self, input: &str) -> Result<Vec<String>, ShellError> {
        self.queue_line(input)?;
        self.execute_buffered_frame(&mut NoCommandFiles)
    }

    fn push_line(&mut self, kind: ConsoleLineKind, text: String) {
        self.push_line_with(kind, text, true);
    }

    fn push_line_with(&mut self, kind: ConsoleLineKind, text: String, notify: bool) {
        let text = self.scrubbed(text);
        self.keep_line(kind, text, notify);
    }

    /// The last of the secret layers (`secret.rs`): `text` without the secret
    /// cvars' values, whichever path printed it; moved, not copied, when it holds
    /// none.
    fn scrubbed(&self, text: String) -> String {
        match self.cvars.scrub(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(scrubbed) => scrubbed,
        }
    }

    /// Keep an already [`Self::scrubbed`] line.
    fn keep_line(&mut self, kind: ConsoleLineKind, text: String, notify: bool) {
        // OpenJK cl_console.cpp:610-612: suppress console printing at its sink.
        if self
            .cvars
            .get("cl_noprint")
            .is_some_and(|v| v.value.as_text() != "0")
        {
            return;
        }
        if self.lines.len() == self.log_capacity {
            self.lines.pop_front();
        }
        if let Some(tap) = self.line_tap {
            tap(&text);
        }
        let written_millis = self.command_clock_millis();
        // Local time, as `Com_RealTime` stamps console text; frontends that draw
        // the stamp on its own read it back from bytes 1..9.
        let mut stamped_text = String::with_capacity(text.len() + STAMP_PREFIX_BYTES);
        stamped_text.push('[');
        crate::local_time::LocalTime::now().push_clock(&mut stamped_text);
        stamped_text.push_str("] ");
        stamped_text.push_str(&text);
        let mode = self
            .cvars
            .get("logfile")
            .and_then(|cvar| match cvar.value {
                crate::CvarValue::Integer(value) => Some(value),
                _ => None,
            })
            .unwrap_or(0);
        let timestamps = !self
            .cvars
            .get("com_timestamps")
            .is_some_and(|cvar| cvar.value == crate::CvarValue::Integer(0));
        self.file_log
            .write(mode, if timestamps { &stamped_text } else { &text });
        self.lines.push_back(ConsoleLine {
            kind,
            notify,
            text,
            written_millis,
            stamped_text,
        });
        self.lines_written += 1;
    }
}

fn builtin_commands() -> impl Iterator<Item = (&'static str, &'static str)> {
    [
        (
            "alias",
            "alias <name> <commands>: run the commands by typing the name",
        ),
        ("aliaslist", "List aliases"),
        ("bind", "Set or inspect a key binding"),
        ("bindlist", "List key bindings"),
        ("clear", "Clear console scrollback"),
        ("cmdlist", "List commands"),
        ("cvarlist", "List configuration variables"),
        ("echo", "Print text"),
        ("print", "Describe a cvar"),
        ("path", "List filesystem search roots"),
        ("dir", "List a virtual directory"),
        ("fdir", "List files matching a wildcard"),
        ("touchFile", "Read a file through the search path"),
        ("which", "Locate the winning file source"),
        ("condump", "Write console scrollback to a text file"),
        ("exec", "Execute a configuration script"),
        ("reset", "Restore a cvar default"),
        ("set", "Set or create a session cvar"),
        ("seta", "Set or create an archived cvar"),
        ("toggle", "Toggle a boolean/numeric cvar"),
        ("togglebind", "Bind a key to toggle a cvar"),
        ("unalias", "Remove an alias"),
        ("unaliasall", "Remove every alias"),
        ("unbind", "Remove a key binding"),
        ("unbindall", "Remove all key bindings"),
        ("vstr", "Execute a cvar value as commands"),
        ("wait", "Pause buffered commands for frames"),
        ("writeconfig", "Persist archived cvars and binds"),
        (
            "write",
            "Persist archived cvars and binds (writeconfig alias)",
        ),
    ]
    .into_iter()
    .chain(cvar_commands::COMMANDS.iter().copied())
    .chain(schedule::COMMANDS.iter().copied())
}

fn list_matches(name: &str, arguments: &[String]) -> bool {
    arguments.first().is_none_or(|pattern| {
        name.to_ascii_lowercase()
            .contains(&pattern.to_ascii_lowercase())
    })
}

fn default_cfg_extension(requested: &str) -> String {
    if Path::new(requested).extension().is_some() {
        requested.to_owned()
    } else {
        format!("{requested}.cfg")
    }
}

fn ascii_starts_with_ignore_case(value: &str, prefix: &str) -> bool {
    value.len() >= prefix.len()
        && value
            .bytes()
            .zip(prefix.bytes())
            .all(|(left, right)| left.eq_ignore_ascii_case(&right))
}

#[cfg(test)]
mod notify_tests {
    use super::*;

    #[test]
    fn quiet_log_lines_skip_notify_but_stay_in_scrollback() {
        let mut shell = Shell::new(CvarRegistry::new(), BindTable::new());
        shell.push_log("print");
        shell.push_log_quiet("chat");
        let lines: Vec<_> = shell
            .lines()
            .map(|line| (line.text.as_str(), line.notify))
            .collect();
        assert_eq!(lines, [("print", true), ("chat", false)]);
    }
}

#[cfg(test)]
mod key_display_tests {
    use super::*;

    #[test]
    fn bind_output_shows_uppercase_names_but_stores_the_canonical_one() {
        let mut shell = Shell::new(CvarRegistry::new(), BindTable::new());
        assert_eq!(
            shell.execute_line("bind W +forward").unwrap(),
            ["W = \"+forward\""]
        );
        assert_eq!(shell.execute_line("bind w").unwrap(), ["W = \"+forward\""]);
        let list = shell.execute_line("bindlist").unwrap();
        assert!(list.iter().any(|line| line.starts_with("W ")), "{list:?}");
        // Configs keep the saved spelling.
        assert_eq!(shell.binds.iter().collect::<Vec<_>>(), [("w", "+forward")]);
        assert_eq!(shell.execute_line("unbind w").unwrap(), ["W unbound"]);
    }

    #[test]
    fn bind_commands_accept_list_query_and_remove_two_key_chords() {
        let mut shell = Shell::new(CvarRegistry::new(), BindTable::new());
        assert_eq!(
            shell.execute_line("bind l+k kill").unwrap(),
            ["K+L = \"kill\""]
        );
        assert_eq!(shell.execute_line("bind K+L").unwrap(), ["K+L = \"kill\""]);
        let list = shell.execute_line("bindlist").unwrap();
        assert!(list.iter().any(|line| line.starts_with("K+L ")), "{list:?}");
        assert_eq!(shell.execute_line("unbind L+K").unwrap(), ["K+L unbound"]);
        assert_eq!(shell.binds.get("K+L"), None);
    }

    #[test]
    fn togglebind_binds_a_key_to_toggle_a_known_cvar() {
        let mut cvars = CvarRegistry::new();
        for (name, default) in [("cg_drawFPS", "0"), ("cg_fov", "80")] {
            cvars
                .register(crate::CvarDefinition::new(
                    name,
                    default,
                    crate::CvarFlags::NONE,
                    "test",
                ))
                .unwrap();
        }
        let mut shell = Shell::new(cvars, BindTable::new());
        assert_eq!(
            shell.execute_line("togglebind f5 cg_drawfps").unwrap(),
            ["F5 = \"toggle cg_drawFPS\""]
        );
        let command = shell.binds.get("F5").unwrap().to_owned();
        shell.execute_line(&command).unwrap();
        assert_eq!(shell.cvars.get("cg_drawFPS").unwrap().value.as_text(), "1");
        shell.execute_line(&command).unwrap();
        assert_eq!(shell.cvars.get("cg_drawFPS").unwrap().value.as_text(), "0");

        shell
            .execute_line("togglebind F6 cg_fov 80 100 120")
            .unwrap();
        let command = shell.binds.get("F6").unwrap().to_owned();
        assert_eq!(command, "toggle cg_fov 80 100 120");
        for expected in ["100", "120", "80"] {
            shell.execute_line(&command).unwrap();
            assert_eq!(shell.cvars.get("cg_fov").unwrap().value.as_text(), expected);
        }

        assert!(matches!(
            shell.execute_line("togglebind F7 cg_drawFSP"),
            Err(ShellError::Cvar(crate::CvarError::Unknown(_)))
        ));
        assert!(matches!(
            shell.execute_line("togglebind F7 cg_fov 90"),
            Err(ShellError::Usage(_))
        ));
        assert!(matches!(
            shell.execute_line("togglebind F7"),
            Err(ShellError::Usage(_))
        ));
        assert_eq!(shell.binds.get("F7"), None);
    }
}

#[cfg(test)]
mod secret_tests {
    use super::*;
    use crate::{CvarDefinition, CvarFlags};
    use std::sync::Mutex;

    static TAPPED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    fn tap(line: &str) {
        TAPPED.lock().unwrap().push(line.to_owned());
    }

    /// Each path gets its own password, so the scrub (which knows only the
    /// cvars' values) cannot hide a gap in the echo or the listings.
    const SECRETS: [&str; 5] = [
        "rconsecret",
        "bindsecret",
        "keysecret",
        "cvarsecret",
        "delaysecret",
    ];

    #[test]
    fn a_typed_password_reaches_the_cvar_but_no_printed_line() {
        crate::secret::add(crate::secret::TEST_MOD);
        let mut cvars = CvarRegistry::new();
        cvars
            .register(CvarDefinition::new(
                "rconPassword",
                "",
                CvarFlags::NONE,
                "Remote console password",
            ))
            .unwrap();
        let mut shell = Shell::new(cvars, BindTable::new());
        shell.set_line_tap(tap);
        let mut printed = Vec::new();
        for line in [
            "rconpassword rconsecret",
            "seta rconPassword rconsecret",
            "rconPassword",
            "print rconPassword",
            "cvarlist rcon",
            "cvar_modified",
            "bind F5 \"modlogin bindsecret\"",
            "bind F5",
            "bind P modlogin keysecret",
            "bind P",
            "bindlist",
            "seta autologin \"modlogin cvarsecret\"",
            "autologin",
            "print autologin",
            "cvarlist auto",
            "cvar_modified",
            "strSub echo $rconPassword$",
            "toggle rconPassword rconsecret other",
            "toggle rconPassword rconsecret other",
            // Refused (delay takes one argument), but echoed first.
            "delay 500 modlogin delaysecret",
        ] {
            if let Ok(lines) = shell.execute_line(line) {
                printed.extend(lines);
            }
        }
        // What a mod or a frontend prints by itself is scrubbed too.
        shell.push_log("userinfo: \\rconPassword\\rconsecret");
        assert_eq!(
            shell.cvars.get("rconPassword").unwrap().value.as_text(),
            "rconsecret"
        );
        assert_eq!(
            shell.cvars.get("autologin").unwrap().value.as_text(),
            "modlogin cvarsecret"
        );
        assert_eq!(shell.binds.get("f5"), Some("modlogin bindsecret"));
        assert_eq!(shell.binds.get("p"), Some("modlogin keysecret"));
        printed.extend(shell.lines().map(|line| line.text.clone()));
        printed.extend(TAPPED.lock().unwrap().iter().cloned());
        assert!(
            printed
                .iter()
                .any(|line| line.contains(crate::secret::MASK))
        );
        for line in &printed {
            for secret in SECRETS {
                assert!(!line.contains(secret), "{line}");
            }
        }
    }
}

#[cfg(test)]
mod external_config_tests {
    use super::*;
    use crate::{CvarDefinition, CvarFlags};

    fn shell_on(path: &Path) -> Shell {
        let mut cvars = CvarRegistry::new();
        for (name, value) in [("name", "Padawan"), ("cg_fov", "90")] {
            cvars
                .register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, ""))
                .unwrap();
        }
        let mut shell = Shell::new(cvars, BindTable::new());
        shell.set_config_path(path);
        shell.load().unwrap();
        shell
    }

    fn saved(path: &Path, name: &str) -> Option<String> {
        let text = std::fs::read_to_string(path).ok()?;
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("seta {name} \"")))
            .map(|value| value.trim_end_matches('"').to_owned())
    }

    /// A config edited while the game runs is read, not overwritten with the
    /// values from before at the next save or at exit.
    #[test]
    fn a_file_edited_while_running_is_loaded_and_kept() {
        let directory = crate::config_watch::TestFolder::new();
        let path = directory.path().join("config.cfg");
        std::fs::write(&path, "seta name \"Old\"\nseta cg_fov \"90\"\n").unwrap();
        let mut shell = shell_on(&path);
        std::fs::write(&path, "seta name \"New\"\nseta cg_fov \"110\"\n").unwrap();
        // Something else archived changes in the meantime, as a game does.
        shell.execute_line("seta sjk_other 1").unwrap();
        shell.save_if_dirty().unwrap();
        assert_eq!(shell.cvars.get("name").unwrap().value.as_text(), "New");
        assert_eq!(saved(&path, "name").as_deref(), Some("New"));
        assert_eq!(saved(&path, "cg_fov").as_deref(), Some("110"));
        assert!(saved(&path, "sjk_other").is_none(), "the edit came last");
        // Changes after the edit save as before.
        shell.execute_line("seta cg_fov 100").unwrap();
        shell.save().unwrap();
        assert_eq!(saved(&path, "cg_fov").as_deref(), Some("100"));
        assert_eq!(saved(&path, "name").as_deref(), Some("New"));
    }

    /// Deleting the config while the game runs leaves it deleted, so the next
    /// start is on the defaults; `writeconfig` still writes it on request.
    #[test]
    fn a_file_deleted_while_running_is_not_written_back() {
        let directory = crate::config_watch::TestFolder::new();
        let path = directory.path().join("config.cfg");
        std::fs::write(&path, "seta name \"Old\"\n").unwrap();
        let mut shell = shell_on(&path);
        std::fs::remove_file(&path).unwrap();
        shell.execute_line("seta cg_fov 120").unwrap();
        shell.save_if_dirty().unwrap();
        shell.save().unwrap();
        assert!(!path.exists());
        assert!(
            shell.lines().any(|line| line.text.contains("deleted")),
            "the console says why"
        );
        shell.execute_line("writeconfig").unwrap();
        assert_eq!(saved(&path, "cg_fov").as_deref(), Some("120"));
    }

    /// The shell's own background saves are not taken for someone else's.
    #[test]
    fn its_own_saves_are_not_external() {
        let directory = crate::config_watch::TestFolder::new();
        let path = directory.path().join("config.cfg");
        let mut shell = shell_on(&path);
        let start = Instant::now();
        shell.execute_line("seta name Mine").unwrap();
        shell.autosave(start).unwrap();
        shell.autosave(start + crate::CONFIG_SAVE_DELAY).unwrap();
        shell.save_if_dirty().unwrap();
        shell.execute_line("seta cg_fov 105").unwrap();
        shell.save().unwrap();
        assert_eq!(saved(&path, "cg_fov").as_deref(), Some("105"));
        assert!(!shell.lines().any(|line| line.text.contains("outside SJK")));
    }
}
