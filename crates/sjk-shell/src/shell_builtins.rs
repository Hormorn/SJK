use super::*;
use crate::{CvarDefinition, CvarError, CvarFlags, CvarValue, ShellError, save_config};
use std::path::Component;

impl Shell {
    pub(super) fn set_command(
        &mut self,
        arguments: &[String],
        dynamic_flags: CvarFlags,
    ) -> Result<Vec<String>, ShellError> {
        let [name, values @ ..] = arguments else {
            return Err(ShellError::Usage("set[a] <name> <value>"));
        };
        if values.is_empty() {
            return self.describe_cvar(name);
        }
        let value = values.join(" ");
        if self.cvars.get(name).is_none() {
            self.cvars.register(CvarDefinition::new(
                name,
                value.clone(),
                dynamic_flags | CvarFlags::USER_CREATED,
                "User-created cvar",
            ))?;
        } else {
            self.cvars.set_text(name, &value)?;
            self.cvars.add_flags(name, dynamic_flags)?;
        }
        self.describe_cvar(name)
    }

    pub(super) fn describe_cvar(&self, name: &str) -> Result<Vec<String>, ShellError> {
        let cvar = self
            .cvars
            .get(name)
            .ok_or_else(|| CvarError::Unknown(name.to_owned()))?;
        Ok(vec![format!(
            "{} = \"{}\" (default \"{}\")",
            cvar.name,
            cvar.shown_value(),
            cvar.shown_default()
        )])
    }

    pub(super) fn reset_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let [name] = arguments else {
            return Err(ShellError::Usage("reset <name>"));
        };
        self.cvars.reset(name)?;
        self.describe_cvar(name)
    }

    pub(super) fn toggle_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let [name, values @ ..] = arguments else {
            return Err(ShellError::Usage("toggle <name> [value1 value2 ...]"));
        };
        // OpenJK cvar.cpp:1005-1036: string cycling, first value on no match. A typed
        // cvar reads back as its own text (`cg_fov` 80 as "80.0"), which OpenJK's
        // string cvars do not, so a number matches a value of the same number too;
        // otherwise `toggle cg_fov 80 100` stayed on 80 (08/11/2026).
        if !values.is_empty() {
            if values.len() < 2 {
                return Err(ShellError::Usage("toggle needs at least two values"));
            }
            let current = self
                .cvars
                .get(name)
                .map(|v| v.value.as_text())
                .unwrap_or_default();
            let next = values
                .iter()
                .position(|value| same_value(value, &current))
                .map_or(0, |index| (index + 1) % values.len());
            return self.set_command(&[name.clone(), values[next].clone()], CvarFlags::NONE);
        }
        if self.cvars.get(name).is_none() {
            return self.set_command(&[name.clone(), "1".into()], CvarFlags::NONE);
        }
        let current = self
            .cvars
            .get(name)
            .ok_or_else(|| CvarError::Unknown(name.clone()))?
            .value
            .clone();
        let next = match current {
            CvarValue::Bool(value) => CvarValue::Bool(!value),
            CvarValue::Integer(value) => CvarValue::Integer(i64::from(value == 0)),
            CvarValue::Float(value) => CvarValue::Float(f64::from(value == 0.0)),
            CvarValue::Text(value) => {
                CvarValue::Text(i32::from(value.parse::<f32>().unwrap_or(0.0) == 0.0).to_string())
            }
        };
        self.cvars.set_value(name, next)?;
        self.describe_cvar(name)
    }

    pub(super) fn bind_command(&mut self, arguments: &[String]) -> Result<Vec<String>, ShellError> {
        let Some(key) = arguments.first() else {
            return Err(ShellError::Usage("bind <key> [command]"));
        };
        let key = BindTable::canonical_key(key)?;
        if arguments.len() == 1 {
            return Ok(vec![self.binds.get(&key).map_or_else(
                || format!("{} is not bound", crate::key_names::display_key(&key)),
                |command| {
                    format!(
                        "{} = \"{}\"",
                        crate::key_names::display_key(&key),
                        crate::secret::input_line(command, &self.cvars)
                    )
                },
            )]);
        }
        let command = arguments[1..].join(" ");
        if command.is_empty() {
            self.binds.unbind(&key);
            return Ok(Vec::new());
        }
        self.binds.bind(&key, command.clone())?;
        Ok(vec![format!(
            "{} = \"{}\"",
            crate::key_names::display_key(&key),
            crate::secret::input_line(&command, &self.cvars)
        )])
    }

    /// SJK's `togglebind <key> <cvar> [value1 value2 ...]` (Blep's idea, 08/11/2026): binds the
    /// key to `toggle` that cvar, so each press flips it between 0 and 1, or steps through
    /// the values given. The cvar must exist, so a typo is told at once rather than making a
    /// new cvar on the first press; `bind <key> "toggle <name>"` still takes any name.
    pub(super) fn togglebind_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let [key, name, values @ ..] = arguments else {
            return Err(ShellError::Usage(
                "togglebind <key> <cvar> [value1 value2 ...]",
            ));
        };
        if values.len() == 1 {
            return Err(ShellError::Usage("toggle needs at least two values"));
        }
        let Some(cvar) = self.cvars.get(name) else {
            return Err(CvarError::Unknown(name.clone()).into());
        };
        let mut command = format!("toggle {}", cvar.name);
        for value in values {
            if value.contains(|c: char| c.is_whitespace() || c == ';') {
                command.push_str(&format!(" \"{value}\""));
            } else {
                command.push(' ');
                command.push_str(value);
            }
        }
        self.bind_command(&[key.clone(), command])
    }

    pub(super) fn unbind_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let [key] = arguments else {
            return Err(ShellError::Usage("unbind <key>"));
        };
        let key = BindTable::canonical_key(key)?;
        self.binds.unbind(&key);
        Ok(vec![format!(
            "{} unbound",
            crate::key_names::display_key(&key)
        )])
    }

    pub(super) fn write_config_command(
        &mut self,
        arguments: &[String],
    ) -> Result<Vec<String>, ShellError> {
        let path = match arguments {
            [] => self.config_path.clone().ok_or(ShellError::NoConfigPath)?,
            [name] => self.config_file_path(name)?,
            _ => return Err(ShellError::Usage("writeconfig [name]")),
        };
        if self.config_load_failed {
            return Err(ShellError::ConfigLoadFailed);
        }
        // The profile's own file goes through the saver, after its queued writes.
        if self.config_path.as_deref() == Some(path.as_path()) {
            // Asked for by name: written even after the file was deleted.
            self.follow_external_config(None);
            self.config_removed = false;
            self.save()?;
        } else {
            save_config(&path, &self.cvars, &self.binds)?;
        }
        Ok(vec![format!("wrote {}", path.display())])
    }

    /// Where `writeconfig <name>` writes, as OpenJK's `Com_WriteConfig_f`
    /// (`common.cpp`) does: `.cfg` added when the name has no extension, no other
    /// extension, and the file in the profile's folder, beside `config.cfg`, where
    /// `exec <name>` reads it. A bare name once went to the process's working folder
    /// (the home folder on Linux) without `.cfg`, so `exec` could not find it.
    fn config_file_path(&self, name: &str) -> Result<PathBuf, ShellError> {
        let file = super::default_cfg_extension(name);
        let relative = Path::new(&file);
        if !relative
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("cfg"))
        {
            return Err(ShellError::Usage(
                "writeconfig <name>: only the .cfg extension is supported",
            ));
        }
        if relative.is_absolute()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        {
            return Err(ShellError::Usage(
                "writeconfig <name>: the file stays in the config folder",
            ));
        }
        let folder = self
            .config_path
            .as_deref()
            .and_then(Path::parent)
            .ok_or(ShellError::NoConfigPath)?;
        Ok(folder.join(relative))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindTable, CvarRegistry};

    #[test]
    fn writeconfig_names_land_beside_config_cfg_as_exec_reads_them() {
        let mut shell = Shell::new(CvarRegistry::new(), BindTable::new());
        assert!(matches!(
            shell.config_file_path("sjkdata"),
            Err(ShellError::NoConfigPath)
        ));
        let folder = Path::new("profile");
        shell.set_config_path(folder.join("config.cfg"));
        assert_eq!(
            shell.config_file_path("sjkdata").unwrap(),
            folder.join("sjkdata.cfg")
        );
        assert_eq!(
            shell.config_file_path("binds/duel.CFG").unwrap(),
            folder.join("binds/duel.CFG")
        );
        for refused in ["notes.txt", "../outside", "/etc/passwd"] {
            assert!(
                matches!(shell.config_file_path(refused), Err(ShellError::Usage(_))),
                "{refused}"
            );
        }
    }
}

/// Whether `toggle`'s `value` names the cvar's `current` text: the same text, or the
/// same number however written ("80" and "80.0").
fn same_value(value: &str, current: &str) -> bool {
    value == current
        || matches!(
            (value.trim().parse::<f64>(), current.trim().parse::<f64>()),
            (Ok(value), Ok(current)) if value == current
        )
}
