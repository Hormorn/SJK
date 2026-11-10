//! Physical-input to console-command bindings.

use crate::CommandError;
use crate::command::split_commands;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{Display, Formatter};

/// Maximum number of two-key bindings, matching JoF EJK.
pub const MAX_KEY_COMBOS: usize = 256;

/// Case-insensitive stable key binding table.
#[derive(Clone, Debug, Default)]
pub struct BindTable {
    bindings: BTreeMap<String, Binding>,
    // Table order decides which chord wins when several held chords match, as
    // JoF EJK's fixed combo array does. Removing one swaps the last into its slot.
    combos: Vec<ComboBinding>,
    /// Stamp of the bindings ([`Self::revision`]); not part of equality.
    revision: u64,
}

impl PartialEq for BindTable {
    fn eq(&self, other: &Self) -> bool {
        self.bindings == other.bindings && self.combos == other.combos
    }
}

impl Eq for BindTable {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Binding {
    key: String,
    command: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ComboBinding {
    first: String,
    second: String,
    binding: Binding,
}

/// Held keys and active chords for one input stream.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BindState {
    held: BTreeSet<String>,
    active: BTreeMap<String, String>,
}

impl BindState {
    /// Forget all held keys and active chords, for example after focus is lost.
    pub fn clear(&mut self) {
        self.held.clear();
        self.active.clear();
    }
}

impl BindTable {
    /// Construct an empty binding table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the canonical saved spelling of a key or two-key chord.
    ///
    /// A chord is written as `K+L`. The last plus separates its keys so the
    /// plus key itself can be the first key in `++K`, as it is in JoF EJK.
    pub fn canonical_key(key: &str) -> Result<String, BindError> {
        Ok(parse_key(key)?.canonical().to_owned())
    }

    /// Bind a physical key or two-key chord, or clear it with an empty script.
    pub fn bind(&mut self, key: &str, command: impl Into<String>) -> Result<(), BindError> {
        let key = parse_key(key)?;
        let command = command.into();
        if command.is_empty() {
            self.remove(&key);
            return Ok(());
        }
        match key {
            Key::Single {
                normalized,
                canonical,
            } => {
                let binding = Binding {
                    key: canonical,
                    command,
                };
                if self.bindings.get(&normalized) != Some(&binding) {
                    self.bindings.insert(normalized, binding);
                    self.revision = crate::revision::next();
                }
            }
            Key::Combo {
                first,
                second,
                canonical,
            } => {
                if let Some(combo) = self
                    .combos
                    .iter_mut()
                    .find(|combo| combo.first == first && combo.second == second)
                {
                    let binding = Binding {
                        key: canonical,
                        command,
                    };
                    if combo.binding != binding {
                        combo.binding = binding;
                        self.revision = crate::revision::next();
                    }
                } else {
                    if self.combos.len() >= MAX_KEY_COMBOS {
                        return Err(BindError::TooManyCombos);
                    }
                    self.combos.push(ComboBinding {
                        first,
                        second,
                        binding: Binding {
                            key: canonical,
                            command,
                        },
                    });
                    self.revision = crate::revision::next();
                }
            }
        }
        Ok(())
    }

    /// Remove a binding case-insensitively and independently of chord order.
    pub fn unbind(&mut self, key: &str) -> bool {
        let Ok(key) = parse_key(key) else {
            return false;
        };
        self.remove(&key)
    }

    fn remove(&mut self, key: &Key) -> bool {
        let removed = match key {
            Key::Single { normalized, .. } => self.bindings.remove(normalized).is_some(),
            Key::Combo { first, second, .. } => self
                .combos
                .iter()
                .position(|combo| combo.first == *first && combo.second == *second)
                .map(|index| self.combos.swap_remove(index))
                .is_some(),
        };
        if removed {
            self.revision = crate::revision::next();
        }
        removed
    }

    /// Remove every binding.
    pub fn clear(&mut self) {
        self.revision = crate::revision::next();
        self.bindings.clear();
        self.combos.clear();
    }

    /// A stamp of the bindings: it changes with every bind, unbind or clear, so
    /// an equal stamp means the same bindings. Stamps are unique across tables;
    /// every new table starts empty at 0.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Return a key or chord's command script case-insensitively.
    pub fn get(&self, key: &str) -> Option<&str> {
        match parse_key(key).ok()? {
            Key::Single { normalized, .. } => self
                .bindings
                .get(&normalized)
                .map(|binding| binding.command.as_str()),
            Key::Combo { first, second, .. } => self
                .combos
                .iter()
                .find(|combo| combo.first == first && combo.second == second)
                .map(|combo| combo.binding.command.as_str()),
        }
    }

    /// Iterate single bindings in normalized order, then chords in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.bindings
            .values()
            .chain(self.combos.iter().map(|combo| &combo.binding))
            .map(|binding| (binding.key.as_str(), binding.command.as_str()))
    }

    /// Resolve one key event, without tracking or activating two-key chords.
    pub fn commands_for_event(&self, key: &str, pressed: bool) -> Result<Vec<String>, BindError> {
        commands_for_script(self.get(key), pressed)
    }

    /// Resolve one physical-key event, including held two-key chords.
    ///
    /// The first key keeps its ordinary press. Pressing the second key runs the
    /// first matching chord instead of that key's ordinary bind. A `+command`
    /// chord receives its `-command` when the triggering second key is released.
    pub fn commands_for_chord_event(
        &self,
        state: &mut BindState,
        key: &str,
        pressed: bool,
    ) -> Result<Vec<String>, BindError> {
        let normalized = normalize_single_key(key)?;
        if pressed {
            state.held.insert(normalized.clone());
            state.active.remove(&normalized);
            if let Some(combo) = self.combos.iter().find(|combo| {
                (combo.first == normalized && state.held.contains(&combo.second))
                    || (combo.second == normalized && state.held.contains(&combo.first))
            }) {
                state.active.insert(normalized, combo.binding.key.clone());
                return commands_for_script(Some(&combo.binding.command), true);
            }
            return commands_for_script(
                self.bindings
                    .get(&normalized)
                    .map(|binding| binding.command.as_str()),
                true,
            );
        }

        // A release can arrive after a menu or text field took the press. It is
        // not one of this binding stream's keys and must not synthesize a
        // `-command` or disturb a chord.
        if !state.held.remove(&normalized) {
            return Ok(Vec::new());
        }
        if let Some(combo) = state.active.remove(&normalized)
            && let Some(command) = self.get(&combo)
        {
            return commands_for_script(Some(command), false);
        }
        commands_for_script(
            self.bindings
                .get(&normalized)
                .map(|binding| binding.command.as_str()),
            false,
        )
    }

    /// The commands an automatic repeat of a held key runs: its bind's (or its active
    /// chord's) down-only commands, such as `weapnext`. Stock `CL_ParseBinding` runs
    /// those again on every repeated press, so a held key keeps cycling the weapons; a
    /// `+command` stays held and is not pressed again. Nothing for a key this binding
    /// stream did not press.
    pub fn commands_for_repeat(
        &self,
        state: &BindState,
        key: &str,
    ) -> Result<Vec<String>, BindError> {
        let normalized = normalize_single_key(key)?;
        if !state.held.contains(&normalized) {
            return Ok(Vec::new());
        }
        let script = match state.active.get(&normalized) {
            Some(combo) => self.get(combo),
            None => self
                .bindings
                .get(&normalized)
                .map(|binding| binding.command.as_str()),
        };
        Ok(commands_for_script(script, true)?
            .into_iter()
            .filter(|command| !command.trim_start().starts_with(['+', '-']))
            .collect())
    }
}

fn commands_for_script(command: Option<&str>, pressed: bool) -> Result<Vec<String>, BindError> {
    let Some(command) = command else {
        return Ok(Vec::new());
    };
    let commands = split_commands(command).map_err(BindError::Command)?;
    if pressed {
        return Ok(commands);
    }
    Ok(commands
        .into_iter()
        .filter_map(|command| {
            let command = command.trim();
            command.strip_prefix('+').map(|rest| format!("-{rest}"))
        })
        .collect())
}

#[derive(Debug)]
enum Key {
    Single {
        normalized: String,
        canonical: String,
    },
    Combo {
        first: String,
        second: String,
        canonical: String,
    },
}

impl Key {
    fn canonical(&self) -> &str {
        match self {
            Self::Single { canonical, .. } | Self::Combo { canonical, .. } => canonical,
        }
    }
}

fn parse_key(key: &str) -> Result<Key, BindError> {
    if let Some(canonical) = crate::key_names::canonical_key(key) {
        return Ok(Key::Single {
            normalized: canonical.to_lowercase(),
            canonical: canonical.to_owned(),
        });
    }
    let (left, right) = key
        .rsplit_once('+')
        .ok_or_else(|| BindError::InvalidKey(key.to_owned()))?;
    let left = crate::key_names::canonical_key(left)
        .ok_or_else(|| BindError::InvalidKey(key.to_owned()))?;
    let right = crate::key_names::canonical_key(right)
        .ok_or_else(|| BindError::InvalidKey(key.to_owned()))?;
    let mut first = (left.to_lowercase(), left);
    let mut second = (right.to_lowercase(), right);
    if first.0 == second.0 {
        return Err(BindError::InvalidKey(key.to_owned()));
    }
    if first.0 > second.0 {
        std::mem::swap(&mut first, &mut second);
    }
    Ok(Key::Combo {
        canonical: format!("{}+{}", first.1, second.1),
        first: first.0,
        second: second.0,
    })
}

fn normalize_single_key(key: &str) -> Result<String, BindError> {
    crate::key_names::canonical_key(key)
        .map(str::to_lowercase)
        .ok_or_else(|| BindError::InvalidKey(key.to_owned()))
}

/// Failure while creating or resolving a key binding.
#[derive(Debug)]
pub enum BindError {
    /// The key name is empty or contains unsupported characters.
    InvalidKey(String),
    /// A binding was given no command.
    EmptyCommand,
    /// The JoF-compatible two-key binding table is full.
    TooManyCombos,
    /// A bound command script could not be parsed.
    Command(CommandError),
}

impl Display for BindError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKey(key) => write!(formatter, "invalid bind key {key:?}"),
            Self::EmptyCommand => formatter.write_str("bind command cannot be empty"),
            Self::TooManyCombos => {
                write!(
                    formatter,
                    "at most {MAX_KEY_COMBOS} key combos can be bound"
                )
            }
            Self::Command(error) => Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for BindError {}

#[cfg(test)]
mod tests {
    use super::{BindError, BindState, BindTable, MAX_KEY_COMBOS};

    #[test]
    fn layout_characters_bind_and_resolve() {
        let mut binds = BindTable::new();
        binds.bind("é", "+attack").unwrap();
        binds.bind("^", "+use").unwrap();
        binds.bind("<", "+speed").unwrap();
        binds.bind("Ù", "+back").unwrap();
        assert_eq!(binds.get("é"), Some("+attack"));
        assert_eq!(binds.get("É"), Some("+attack"));
        assert_eq!(binds.get("^"), Some("+use"));
        assert_eq!(binds.get("<"), Some("+speed"));
        assert_eq!(binds.get("ù"), Some("+back"));
    }

    #[test]
    fn saved_names_keep_their_meaning() {
        let mut binds = BindTable::new();
        binds.bind("W", "+forward").unwrap();
        binds.bind("SEMICOLON", "+left").unwrap();
        assert_eq!(binds.get("w"), Some("+forward"));
        assert_eq!(binds.get(";"), Some("+left"));
        assert!(binds.bind("éé", "+attack").is_err());
        assert!(binds.bind("\u{7f}", "+attack").is_err());
    }

    #[test]
    fn combo_names_are_order_independent_and_allow_the_plus_key() {
        let mut binds = BindTable::new();
        binds.bind("l+K", "kill").unwrap();
        binds.bind("++K", "say plus").unwrap();
        assert_eq!(binds.get("k+l"), Some("kill"));
        assert_eq!(binds.get("L+k"), Some("kill"));
        assert_eq!(binds.get("++k"), Some("say plus"));
        assert!(binds.bind("K+K", "no").is_err());
        assert!(binds.bind("K+", "no").is_err());
    }

    #[test]
    fn a_held_key_repeats_only_its_down_only_commands() {
        let mut binds = BindTable::new();
        let mut state = BindState::default();
        binds.bind("Q", "weapnext").unwrap();
        binds.bind("E", "+attack; weapprev").unwrap();
        binds.bind("R", "+use").unwrap();
        binds.bind("R+T", "weapnext; +speed").unwrap();

        // A key the bindings never saw pressed repeats nothing.
        assert!(binds.commands_for_repeat(&state, "Q").unwrap().is_empty());

        binds
            .commands_for_chord_event(&mut state, "Q", true)
            .unwrap();
        assert_eq!(
            binds.commands_for_repeat(&state, "Q").unwrap(),
            ["weapnext"]
        );
        binds
            .commands_for_chord_event(&mut state, "E", true)
            .unwrap();
        assert_eq!(
            binds.commands_for_repeat(&state, "E").unwrap(),
            ["weapprev"]
        );
        binds
            .commands_for_chord_event(&mut state, "R", true)
            .unwrap();
        assert!(binds.commands_for_repeat(&state, "R").unwrap().is_empty());
        // The chord's second key repeats the chord's down-only part.
        binds
            .commands_for_chord_event(&mut state, "T", true)
            .unwrap();
        assert_eq!(
            binds.commands_for_repeat(&state, "T").unwrap(),
            ["weapnext"]
        );

        binds
            .commands_for_chord_event(&mut state, "Q", false)
            .unwrap();
        assert!(binds.commands_for_repeat(&state, "Q").unwrap().is_empty());
    }

    #[test]
    fn second_key_runs_combo_and_owns_its_release() {
        let mut binds = BindTable::new();
        let mut state = BindState::default();
        binds.bind("K", "+use").unwrap();
        binds.bind("L", "+attack").unwrap();
        binds.bind("K+L", "+speed; say combo").unwrap();

        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "K", true)
                .unwrap(),
            ["+use"]
        );
        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "L", true)
                .unwrap(),
            ["+speed", "say combo"]
        );
        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "K", false)
                .unwrap(),
            ["-use"]
        );
        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "L", false)
                .unwrap(),
            ["-speed"]
        );
    }

    #[test]
    fn combo_works_in_either_press_order_and_state_can_be_cleared() {
        let mut binds = BindTable::new();
        let mut state = BindState::default();
        binds.bind("K+L", "kill").unwrap();
        assert!(
            binds
                .commands_for_chord_event(&mut state, "L", true)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "K", true)
                .unwrap(),
            ["kill"]
        );
        state.clear();
        assert!(
            binds
                .commands_for_chord_event(&mut state, "K", true)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_release_without_a_bound_press_runs_nothing() {
        let mut binds = BindTable::new();
        let mut state = BindState::default();
        binds.bind("SHIFT", "+speed").unwrap();
        binds.bind("SHIFT+U", "reply").unwrap();

        assert!(
            binds
                .commands_for_chord_event(&mut state, "SHIFT", false)
                .unwrap()
                .is_empty()
        );
        assert!(
            binds
                .commands_for_chord_event(&mut state, "U", false)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn combo_table_has_the_jof_limit() {
        let mut binds = BindTable::new();
        let mut names = Vec::new();
        for function in 1..=12 {
            for character in 'a'..='z' {
                names.push(format!("F{function}+{character}"));
            }
        }
        for name in names.iter().take(MAX_KEY_COMBOS) {
            binds.bind(name, "kill").unwrap();
        }
        binds.bind(&names[0], "say replacement").unwrap();
        assert!(matches!(
            binds.bind(&names[MAX_KEY_COMBOS], "kill"),
            Err(BindError::TooManyCombos)
        ));
    }

    #[test]
    fn removing_a_combo_moves_the_last_into_its_jof_table_slot() {
        let mut binds = BindTable::new();
        let mut state = BindState::default();
        binds.bind("A+X", "removed").unwrap();
        binds.bind("B+K", "older").unwrap();
        binds.bind("C+K", "last").unwrap();
        assert!(binds.unbind("X+A"));

        assert!(
            binds
                .commands_for_chord_event(&mut state, "B", true)
                .unwrap()
                .is_empty()
        );
        assert!(
            binds
                .commands_for_chord_event(&mut state, "C", true)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            binds
                .commands_for_chord_event(&mut state, "K", true)
                .unwrap(),
            ["last"]
        );
    }
}
