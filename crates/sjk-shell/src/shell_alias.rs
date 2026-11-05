//! The `alias` commands, their expansion and `aliases.cfg` ([`crate::alias`]).
use super::*;
use crate::{ALIAS_FILE_NAME, MAX_EXEC_DEPTH};

/// Whether `aliases.cfg` was read, and which table revision it holds.
#[derive(Debug, Default)]
pub(super) struct AliasFileState {
    loaded: bool,
    /// Reading it failed: the table is not written over the unread file.
    load_failed: bool,
    saved_revision: u64,
}

impl Shell {
    fn aliases_path(&self) -> Option<PathBuf> {
        Some(self.config_path.as_deref()?.with_file_name(ALIAS_FILE_NAME))
    }

    /// Read `aliases.cfg` the first time the config loads. A reload of a config
    /// edited outside SJK leaves the aliases as they are.
    pub(super) fn load_aliases_once(&mut self) {
        if self.aliases_state.loaded {
            return;
        }
        self.aliases_state.loaded = true;
        let Some(path) = self.aliases_path() else {
            return;
        };
        let text = match std::fs::read(&path) {
            Ok(bytes) => crate::decode_config_text(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => {
                self.aliases_state.load_failed = true;
                self.push_log(format!("^1couldn't read {}: {error}", path.display()));
                return;
            }
        };
        let (table, problems) = AliasTable::from_file_text(&text);
        for problem in problems {
            self.push_log(format!("^3{problem}"));
        }
        self.aliases = table;
        self.aliases_state.saved_revision = self.aliases.revision();
    }

    /// Write `aliases.cfg` when an alias changed since it was read or written.
    pub(super) fn save_aliases_if_changed(&mut self) -> Result<(), ShellError> {
        let revision = self.aliases.revision();
        if revision == self.aliases_state.saved_revision || self.aliases_state.load_failed {
            return Ok(());
        }
        let Some(path) = self.aliases_path() else {
            return Ok(());
        };
        // Once per change, whether or not the write lands.
        self.aliases_state.saved_revision = revision;
        crate::config::write_config_text(&path, &self.aliases.file_text())?;
        Ok(())
    }

    /// Run alias `name`'s `command` in its place in the buffer, one level deeper
    /// than the line that named it, so an alias that names itself stops.
    pub(super) fn expand_alias(
        &mut self,
        name: &str,
        command: &str,
        exec_depth: u8,
    ) -> Result<Vec<String>, ShellError> {
        if exec_depth >= MAX_EXEC_DEPTH {
            return Err(ShellError::Application(format!(
                "alias {name}: more than {MAX_EXEC_DEPTH} aliases or scripts inside each other, \
                 stopped (does it run itself?)"
            )));
        }
        self.command_buffer
            .insert_after_current(command, exec_depth + 1)?;
        Ok(Vec::new())
    }

    /// `alias` lists, `alias <name>` shows one, `alias <name> <commands>` defines.
    pub(super) fn alias_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        match arguments {
            [] => Ok(self.alias_list(&[])),
            [name] => match self.aliases.get(name) {
                Some(command) => Ok(vec![format!(
                    "{name} = \"{}\"",
                    crate::secret::input_line(command, &self.cvars)
                )]),
                None => Err(ShellError::Application(format!("no alias {name}"))),
            },
            [name, commands @ ..] => {
                if self.recognizes_local_command(name) {
                    return Err(ShellError::Application(format!(
                        "{name} is already a command or a setting; choose another alias name"
                    )));
                }
                self.aliases
                    .define(name, &commands.join(" "))
                    .map_err(|error| ShellError::Application(error.to_string()))?;
                Ok(Vec::new())
            }
        }
    }

    pub(super) fn unalias_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let [name] = arguments else {
            return Err(ShellError::Usage("unalias <name>"));
        };
        if self.aliases.remove(name) {
            Ok(Vec::new())
        } else {
            Err(ShellError::Application(format!("no alias {name}")))
        }
    }

    /// `aliaslist [<part of a name>]`: each alias and its commands, passwords hidden.
    pub(super) fn alias_list(&self, arguments: &[String]) -> Vec<String> {
        let lines: Vec<String> = self
            .aliases
            .iter()
            .filter(|(name, _)| list_matches(name, arguments))
            .map(|(name, command)| {
                format!(
                    "{name:<16} {}",
                    crate::secret::input_line(command, &self.cvars)
                )
            })
            .collect();
        if lines.is_empty() {
            vec!["no aliases; make one with alias <name> <commands>".to_owned()]
        } else {
            lines
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CvarDefinition, CvarFlags, CvarRegistry};

    fn shell() -> Shell {
        let mut cvars = CvarRegistry::new();
        cvars
            .register(CvarDefinition::new(
                "cg_fov",
                "80",
                CvarFlags::ARCHIVE,
                "FOV",
            ))
            .unwrap();
        Shell::new(cvars, BindTable::new())
    }

    /// Runs `line` and every command it queues, collecting what reaches the
    /// application, as the viewer's interceptor sees it.
    fn run(shell: &mut Shell, line: &str) -> (Vec<String>, Result<Vec<String>, ShellError>) {
        let mut seen = Vec::new();
        shell.queue_line(line).unwrap();
        let result = shell.execute_buffered_frame_with(&mut NoCommandFiles, |text, _, local| {
            if local {
                None
            } else {
                seen.push(text.to_owned());
                Some(Ok(Vec::new()))
            }
        });
        (seen, result)
    }

    /// Runs `line`, which must succeed.
    fn ok(shell: &mut Shell, line: &str) {
        run(shell, line).1.unwrap();
    }

    #[test]
    fn an_alias_runs_its_commands_where_it_stood() {
        let mut shell = shell();
        ok(&mut shell, "alias zoom \"cg_fov 40; say zoomed\"");
        let (seen, result) = run(&mut shell, "say before; ZOOM; say after");
        result.unwrap();
        assert_eq!(seen, ["say before", "say zoomed", "say after"]);
        assert_eq!(shell.cvars.get("cg_fov").unwrap().value.as_text(), "40");
    }

    #[test]
    fn aliases_call_aliases_and_plus_minus_pairs_work() {
        let mut shell = shell();
        ok(&mut shell, "alias inner \"say inner\"");
        ok(&mut shell, "alias outer \"inner; say outer\"");
        ok(&mut shell, "alias +zoom \"cg_fov 30\"");
        ok(&mut shell, "alias -zoom \"cg_fov 80\"");
        let (seen, _) = run(&mut shell, "outer");
        assert_eq!(seen, ["say inner", "say outer"]);
        ok(&mut shell, "+zoom");
        assert_eq!(shell.cvars.get("cg_fov").unwrap().value.as_text(), "30");
        ok(&mut shell, "-zoom");
        assert_eq!(shell.cvars.get("cg_fov").unwrap().value.as_text(), "80");
    }

    #[test]
    fn an_alias_that_runs_itself_stops_with_a_message() {
        let mut shell = shell();
        ok(&mut shell, "alias loop \"say again; loop\"");
        let (seen, result) = run(&mut shell, "loop");
        let error = result.unwrap_err().to_string();
        assert!(error.contains("alias loop"), "{error}");
        assert_eq!(seen.len(), usize::from(MAX_EXEC_DEPTH));
        assert!(!shell.has_buffered_commands());
    }

    #[test]
    fn commands_and_settings_cannot_be_aliased() {
        let mut shell = shell();
        assert!(run(&mut shell, "alias bind kill").1.is_err());
        assert!(run(&mut shell, "alias CG_FOV kill").1.is_err());
        assert!(shell.aliases.is_empty());
    }

    #[test]
    fn show_list_and_remove() {
        let mut shell = shell();
        ok(&mut shell, "alias hi \"say hello\"");
        assert_eq!(
            run(&mut shell, "alias hi").1.unwrap(),
            ["hi = \"say hello\""]
        );
        let listed = run(&mut shell, "aliaslist").1.unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].starts_with("hi"), "{listed:?}");
        run(&mut shell, "unalias HI").1.unwrap();
        assert!(run(&mut shell, "unalias hi").1.is_err());
        // Without the alias the name goes on to the application (the server).
        assert_eq!(run(&mut shell, "hi").0, ["hi"]);
        ok(&mut shell, "alias a kill; alias b kill");
        assert_eq!(shell.aliases.len(), 2);
        ok(&mut shell, "unaliasall");
        assert!(shell.aliases.is_empty());
    }

    #[test]
    fn aliases_are_kept_beside_the_config() {
        let directory = std::env::temp_dir().join(format!(
            "sjk-shell-alias-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let config = directory.join("config.cfg");

        let mut shell = shell();
        shell.set_config_path(&config);
        shell.load().unwrap();
        ok(&mut shell, "alias hi \"say \\\"hello\\\"; wave\"");
        shell.save().unwrap();
        let saved = std::fs::read_to_string(directory.join(ALIAS_FILE_NAME)).unwrap();
        assert!(saved.contains("alias \"hi\""), "{saved}");
        // config.cfg holds no alias line, so builds without aliases still load it.
        let config_text = std::fs::read_to_string(&config).unwrap();
        assert!(!config_text.contains("alias"), "{config_text}");

        let mut restored = self::shell();
        restored.set_config_path(&config);
        restored.load().unwrap();
        assert_eq!(restored.aliases.get("hi"), Some("say \"hello\"; wave"));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
