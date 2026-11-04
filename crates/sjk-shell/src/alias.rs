//! Console aliases: `alias <name> <commands>` gives a command line a name of its
//! own, as Quake II's `alias` does (`Cmd_Alias_f`, `cmd.c`); the stock Jedi Academy
//! engine has none. Running the name runs its commands where it stood in the
//! buffer, from the console, a bind, an `exec` script or the quick wheel.
//!
//! The table is the player's: [`AliasTable::file_text`] is kept in `aliases.cfg`
//! beside the config, not in `config.cfg`, so a build without aliases still loads
//! that config (its loader stops on a line it does not know).

use crate::tokenize;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// Longest alias name, in bytes (Quake II's `MAX_ALIAS_NAME` is 32).
pub const MAX_ALIAS_NAME_BYTES: usize = 32;
/// Longest command line an alias holds, in bytes.
pub const MAX_ALIAS_COMMAND_BYTES: usize = 1024;
/// Most aliases one profile keeps.
pub const MAX_ALIASES: usize = 512;
/// The file beside the config that keeps the aliases.
pub const ALIAS_FILE_NAME: &str = "aliases.cfg";

/// Named command lines, looked up without regard to ASCII case.
#[derive(Debug, Default)]
pub struct AliasTable {
    /// Lower-case name to the name as typed and its command line.
    entries: BTreeMap<String, (String, String)>,
    revision: u64,
}

impl AliasTable {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The command line `name` stands for.
    pub fn get(&self, name: &str) -> Option<&str> {
        if self.entries.is_empty() {
            return None;
        }
        self.entries
            .get(&name.to_ascii_lowercase())
            .map(|(_, command)| command.as_str())
    }

    /// Name `command` as `name`, replacing an alias of that name.
    pub fn define(&mut self, name: &str, command: &str) -> Result<(), AliasError> {
        check_name(name)?;
        if command.len() > MAX_ALIAS_COMMAND_BYTES {
            return Err(AliasError::CommandTooLong);
        }
        let key = name.to_ascii_lowercase();
        if !self.entries.contains_key(&key) && self.entries.len() >= MAX_ALIASES {
            return Err(AliasError::TooMany);
        }
        let entry = (name.to_owned(), command.to_owned());
        if self.entries.get(&key) != Some(&entry) {
            self.entries.insert(key, entry);
            self.revision += 1;
        }
        Ok(())
    }

    /// Forget the alias `name`; whether there was one.
    pub fn remove(&mut self, name: &str) -> bool {
        let removed = self.entries.remove(&name.to_ascii_lowercase()).is_some();
        if removed {
            self.revision += 1;
        }
        removed
    }

    /// Forget every alias; how many there were.
    pub fn clear(&mut self) -> usize {
        let count = self.entries.len();
        if count > 0 {
            self.entries.clear();
            self.revision += 1;
        }
        count
    }

    /// Every alias as typed with its command line, in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .values()
            .map(|(name, command)| (name.as_str(), command.as_str()))
    }

    /// How many aliases there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Changes each time the table does, for saving.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The text of `aliases.cfg`: one `alias "<name>" "<commands>"` line each.
    pub fn file_text(&self) -> String {
        let mut text = String::from(
            "// SJK aliases (alias, unalias in the console). Edit while SJK is closed.\n",
        );
        for (name, command) in self.iter() {
            text.push_str("alias \"");
            push_escaped(&mut text, name);
            text.push_str("\" \"");
            push_escaped(&mut text, command);
            text.push_str("\"\n");
        }
        text
    }

    /// Read `aliases.cfg`'s text: the aliases, and a message for each line that
    /// could not be read (the rest still load).
    pub fn from_file_text(text: &str) -> (Self, Vec<String>) {
        let mut table = Self::new();
        let mut problems = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let result = match tokenize(line) {
                Ok(tokens) => match tokens.as_slice() {
                    [command, name, rest @ ..] if command.eq_ignore_ascii_case("alias") => table
                        .define(name, &rest.join(" "))
                        .map_err(|e| e.to_string()),
                    _ => Err("expected `alias name commands`".to_owned()),
                },
                Err(error) => Err(error.to_string()),
            };
            if let Err(message) = result {
                problems.push(format!("{ALIAS_FILE_NAME} line {}: {message}", index + 1));
            }
        }
        table.revision = 0;
        (table, problems)
    }
}

fn check_name(name: &str) -> Result<(), AliasError> {
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | ';'))
    {
        return Err(AliasError::InvalidName(name.to_owned()));
    }
    if name.len() > MAX_ALIAS_NAME_BYTES {
        return Err(AliasError::NameTooLong);
    }
    Ok(())
}

fn push_escaped(text: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '\\' => text.push_str("\\\\"),
            '"' => text.push_str("\\\""),
            '\n' => text.push_str("\\n"),
            '\r' => text.push_str("\\r"),
            '\t' => text.push_str("\\t"),
            other => text.push(other),
        }
    }
}

/// Why an alias could not be defined.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AliasError {
    /// The name is empty or holds a space, a quote, a `;` or a control character.
    InvalidName(String),
    /// The name is longer than [`MAX_ALIAS_NAME_BYTES`].
    NameTooLong,
    /// The commands are longer than [`MAX_ALIAS_COMMAND_BYTES`].
    CommandTooLong,
    /// [`MAX_ALIASES`] aliases already exist.
    TooMany,
}

impl Display for AliasError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidName(name) => write!(
                formatter,
                "{name:?} cannot be an alias name: no spaces, quotes or ;"
            ),
            Self::NameTooLong => write!(
                formatter,
                "alias names are at most {MAX_ALIAS_NAME_BYTES} characters"
            ),
            Self::CommandTooLong => write!(
                formatter,
                "an alias holds at most {MAX_ALIAS_COMMAND_BYTES} characters of commands"
            ),
            Self::TooMany => write!(formatter, "at most {MAX_ALIASES} aliases"),
        }
    }
}

impl std::error::Error for AliasError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_ignore_case_and_keep_their_spelling() {
        let mut table = AliasTable::new();
        table.define("Duel", "say ready; +use").unwrap();
        assert_eq!(table.get("duel"), Some("say ready; +use"));
        assert_eq!(table.get("DUEL"), Some("say ready; +use"));
        assert_eq!(
            table.iter().collect::<Vec<_>>(),
            [("Duel", "say ready; +use")]
        );
        assert!(table.remove("dUEL"));
        assert!(table.get("duel").is_none());
        assert!(!table.remove("duel"));
    }

    #[test]
    fn bad_names_and_sizes_are_refused() {
        let mut table = AliasTable::new();
        for name in ["", "two words", "a;b", "q\"t"] {
            assert!(matches!(
                table.define(name, "kill"),
                Err(AliasError::InvalidName(_))
            ));
        }
        assert_eq!(
            table.define(&"n".repeat(MAX_ALIAS_NAME_BYTES + 1), "kill"),
            Err(AliasError::NameTooLong)
        );
        assert_eq!(
            table.define("long", &"x".repeat(MAX_ALIAS_COMMAND_BYTES + 1)),
            Err(AliasError::CommandTooLong)
        );
        assert!(table.define("+zoom", "+attack").is_ok());
        table.clear();
        for index in 0..MAX_ALIASES {
            table.define(&format!("a{index}"), "kill").unwrap();
        }
        assert_eq!(table.define("onemore", "kill"), Err(AliasError::TooMany));
        // Redefining one that exists is still allowed at the limit.
        table.define("a0", "say hi").unwrap();
    }

    #[test]
    fn the_file_round_trips_quotes_and_semicolons() {
        let mut table = AliasTable::new();
        table.define("greet", "say \"hi there\"; wave").unwrap();
        table.define("+zoom", "cg_fov 40").unwrap();
        let text = table.file_text();
        let (restored, problems) = AliasTable::from_file_text(&text);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(restored.get("greet"), Some("say \"hi there\"; wave"));
        assert_eq!(restored.get("+zoom"), Some("cg_fov 40"));
        assert_eq!(restored.revision(), 0);
    }

    #[test]
    fn unreadable_lines_are_reported_and_skipped() {
        let (table, problems) =
            AliasTable::from_file_text("alias ok \"kill\"\nbind x kill\nalias \"a b\" kill\n");
        assert_eq!(table.get("ok"), Some("kill"));
        assert_eq!(table.len(), 1);
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems[0].contains("line 2"), "{problems:?}");
    }

    #[test]
    fn the_revision_moves_only_on_a_change() {
        let mut table = AliasTable::new();
        table.define("a", "kill").unwrap();
        let revision = table.revision();
        table.define("a", "kill").unwrap();
        assert_eq!(table.revision(), revision);
        table.define("a", "say hi").unwrap();
        assert_ne!(table.revision(), revision);
        let revision = table.revision();
        assert_eq!(table.clear(), 1);
        assert_ne!(table.revision(), revision);
    }
}
