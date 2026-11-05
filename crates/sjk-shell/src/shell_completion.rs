//! Tab completion of the command or cvar name being typed (`Field_CompleteCommand`,
//! `qcommon/common.cpp`): a unique name completes with a trailing space; several
//! complete to their longest shared prefix and are listed in the scrollback, with
//! descriptions while the listing is short. The file argument of `exec` and the
//! other commands in [`FILE_ARGUMENTS`] completes the same way from the files the
//! caller lists (`Cmd_CompleteCfgName`, `qcommon/cmd.cpp`), and the map argument of
//! `devmap` from the maps it lists (`SV_CompleteMapName`, `server/sv_ccmds.cpp`).
//! The caller adds commands of its own the same way, each with an
//! [`ArgumentSource`] (`sjkemote` and its emote ids, in the viewer).

use super::{ConsoleLineKind, Shell, ascii_starts_with_ignore_case, builtin_commands};

/// A command whose first argument Tab completes from names the caller lists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArgumentSource {
    /// The command, any case.
    pub command: &'static str,
    /// What the caller is asked to list: a file extension, [`MAP_EXTENSION`] for
    /// maps, or a name of the caller's own.
    pub list: &'static str,
    /// The label of a listing row, at most six characters (`File`, `Map`).
    pub label: &'static str,
}

impl ArgumentSource {
    const fn file(command: &'static str, extension: &'static str) -> Self {
        Self {
            command,
            list: extension,
            label: "File",
        }
    }
}

/// Commands whose first argument names a file, with the extension of the files it
/// completes from, as the completion functions `Cmd_Init`, `Com_Init` and
/// `SV_AddOperatorCommands` register. [`MAP_EXTENSION`] asks for maps.
const FILE_ARGUMENTS: &[ArgumentSource] = &[
    ArgumentSource::file("exec", "cfg"),
    ArgumentSource::file("execq", "cfg"),
    ArgumentSource::file("writeconfig", "cfg"),
    ArgumentSource::file("write", "cfg"),
    ArgumentSource {
        command: "devmap",
        list: MAP_EXTENSION,
        label: "Map",
    },
];

/// The extension a map argument asks the caller's file list for. The caller answers
/// with map names as `devmap` takes them: the path under `maps/` without the
/// extension (`mp/ffa3`).
pub const MAP_EXTENSION: &str = "bsp";

/// Longest cvar value shown in a completion listing, as `TRUNCATE_LENGTH`.
const LISTED_VALUE_CHARS: usize = 64;
/// Most matches listed with their descriptions. Longer listings drop them and end with
/// the match count, so one listing fills as little of the bounded scrollback as it can.
const DESCRIBED_MATCHES: usize = 16;

/// The key that asked for a completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionKey {
    /// Tab: candidates are listed whenever more than one remains.
    Tab,
    /// Enter with `cl_allowEnterCompletion`: the line runs next, so candidates are
    /// listed only when the completed text is not itself a full name.
    Enter,
}

/// One completion candidate.
struct Candidate<'a> {
    name: &'a str,
    description: &'a str,
    /// Current value of a cvar; `None` for a command.
    value: Option<String>,
}

impl<'a> Candidate<'a> {
    fn command(name: &'a str, description: &'a str) -> Self {
        Self {
            name,
            description,
            value: None,
        }
    }
}

impl Shell {
    /// Complete the name at the end of `line`: the first word after its last unquoted
    /// `;`, without a leading `/` or `\`. Such a slash is kept on the line's first
    /// command, which the console strips when the line is entered, and dropped from a
    /// later one, which nothing strips. Returns the completed line, or `None` when
    /// nothing matches or the line already continues past the name.
    pub fn complete_line(&mut self, line: &str, key: CompletionKey) -> Option<String> {
        self.complete_line_with_files(line, key, |_| Vec::new())
    }

    /// [`Self::complete_line`], and on Tab also the file argument of `exec` and the
    /// other commands in `FILE_ARGUMENTS`: `files` lists the names, with their
    /// extension, of the files of the given extension the command can open, highest
    /// priority first. Enter leaves arguments alone, as `Field_CompleteCommand` does.
    pub fn complete_line_with_files(
        &mut self,
        line: &str,
        key: CompletionKey,
        files: impl FnOnce(&str) -> Vec<String>,
    ) -> Option<String> {
        self.complete_line_with_arguments(line, key, &[], files)
    }

    /// [`Self::complete_line_with_files`], with the first argument of the caller's
    /// own commands in `sources` completing too: `lists` is asked for the names of
    /// the source's `list`, as for a file extension.
    pub fn complete_line_with_arguments(
        &mut self,
        line: &str,
        key: CompletionKey,
        sources: &[ArgumentSource],
        lists: impl FnOnce(&str) -> Vec<String>,
    ) -> Option<String> {
        let (replaced, name, listing) = match typed_argument(line, sources) {
            Some(argument) if key == CompletionKey::Tab => {
                let names = lists(argument.source.list);
                let typed = &line[argument.start..];
                let (name, listing) = if argument.source.list == MAP_EXTENSION {
                    map_completion(typed, &names, argument.source.label)?
                } else {
                    word_completion(typed, &names, argument.source.label)?
                };
                (argument.start, name, listing)
            }
            Some(_) => return None,
            None => {
                let typed = typed_name(line)?;
                let (name, listing) = self.completion(&line[typed.name..], key)?;
                (typed.replaced, name, listing)
            }
        };
        let completed = format!("{}{name}", &line[..replaced]);
        if !listing.is_empty() {
            self.push_line(
                ConsoleLineKind::Input,
                format!("{}{completed}", super::PROMPT),
            );
            for text in listing {
                self.push_line(ConsoleLineKind::Output, text);
            }
        }
        Some(completed)
    }

    /// The unique name `line` would complete to, for a prompt hint; does not allocate.
    pub fn completion_hint(&self, line: &str) -> Option<&str> {
        self.unique_command_completion(&line[typed_name(line)?.name..])
    }

    /// The completed name and, when it is not unique, the lines listing the candidates.
    fn completion(&self, prefix: &str, key: CompletionKey) -> Option<(String, Vec<String>)> {
        let candidates = self.candidates(prefix);
        if let [only] = candidates.as_slice() {
            return Some((format!("{} ", only.name), Vec::new()));
        }
        let shared = shared_prefix(candidates.iter().map(|candidate| candidate.name))?;
        let full_name = candidates
            .iter()
            .any(|candidate| candidate.name.len() == shared.len());
        if key == CompletionKey::Enter && full_name {
            return Some((shared.to_owned(), Vec::new()));
        }
        let described = candidates.len() <= DESCRIBED_MATCHES;
        let mut listing = Vec::with_capacity(candidates.len() * 2 + 1);
        for candidate in &candidates {
            listing.push(listed(candidate));
            if described && !candidate.description.is_empty() {
                listing.push(format!("^2      {}", candidate.description));
            }
        }
        if !described {
            listing.push(format!(
                "^3{} matches; type more of the name to narrow them",
                candidates.len()
            ));
        }
        Some((shared.to_owned(), listing))
    }

    /// Commands, then cvars, whose names start with `prefix`, each sorted; a name that
    /// is both a command and a cvar is listed once, as the command.
    fn candidates(&self, prefix: &str) -> Vec<Candidate<'_>> {
        let mut commands = builtin_commands()
            .map(|(name, description)| Candidate::command(name, description))
            .chain(
                self.commands
                    .iter()
                    .map(|(name, description)| Candidate::command(name, description)),
            )
            .chain(
                self.external_command_help
                    .iter()
                    .map(|(name, description)| Candidate::command(name, description)),
            )
            .chain(
                self.aliases
                    .iter()
                    .map(|(name, _)| Candidate::command(name, "alias")),
            )
            .filter(|candidate| ascii_starts_with_ignore_case(candidate.name, prefix))
            .collect::<Vec<_>>();
        commands.sort_by_cached_key(|candidate| candidate.name.to_ascii_lowercase());
        commands.dedup_by(|later, earlier| later.name.eq_ignore_ascii_case(earlier.name));
        let mut cvars = self
            .cvars
            .iter()
            .filter(|cvar| ascii_starts_with_ignore_case(&cvar.name, prefix))
            .filter(|cvar| {
                !commands
                    .iter()
                    .any(|command| command.name.eq_ignore_ascii_case(&cvar.name))
            })
            .map(|cvar| Candidate {
                name: &cvar.name,
                description: &cvar.description,
                value: Some(listed_value(cvar.shown_value())),
            })
            .collect::<Vec<_>>();
        cvars.sort_by_cached_key(|candidate| candidate.name.to_ascii_lowercase());
        commands.append(&mut cvars);
        commands
    }
}

/// Byte offsets in a line of the name being typed.
struct TypedName {
    /// Start of the name itself.
    name: usize,
    /// Start of the text its completion replaces: the name, or the slash before it on a
    /// command after a `;`.
    replaced: usize,
}

/// The name being typed: after the last `;` that [`split_commands`](crate::split_commands)
/// splits on, leading whitespace and one `/` or `\`. `None` when that name is empty, an
/// argument follows it or a quote is still open.
fn typed_name(line: &str) -> Option<TypedName> {
    // The splitter's own view of the line; normalizing only rewrites the first
    // command's argument, so the name at the end is a suffix of both spellings.
    let normalized = crate::key_names::normalize_stock_backslash(line);
    let segment = last_command(&normalized)?;
    let command = normalized[segment..].trim_start_matches(|c: char| c.is_ascii_whitespace());
    let name = command.strip_prefix(['/', '\\']).unwrap_or(command);
    let continues = name
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte == b'"');
    if name.is_empty() || continues {
        return None;
    }
    let start = line.len() - name.len();
    Some(TypedName {
        name: start,
        replaced: if segment == 0 {
            start
        } else {
            line.len() - command.len()
        },
    })
}

/// Start of the last command on `line`: after its last `;` that
/// [`split_commands`](crate::split_commands) splits on. `None` while a quote is open.
fn last_command(line: &str) -> Option<usize> {
    let mut segment = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in line.bytes().enumerate() {
        if escaped {
            escaped = false;
        } else if byte == b'\\' && quoted {
            escaped = true;
        } else if byte == b'"' {
            quoted = !quoted;
        } else if byte == b';' && !quoted {
            segment = index + 1;
        }
    }
    (!quoted && !escaped).then_some(segment)
}

/// The argument being typed to a command in [`FILE_ARGUMENTS`] or the caller's
/// sources.
struct TypedArgument {
    /// Byte offset in the line of the argument; it runs to the end of the line.
    start: usize,
    /// What it completes from.
    source: ArgumentSource,
}

/// The first argument of the last command on `line` when [`FILE_ARGUMENTS`] or
/// `sources` name that command and nothing follows the argument. The argument may
/// still be empty.
fn typed_argument(line: &str, sources: &[ArgumentSource]) -> Option<TypedArgument> {
    let command = line[last_command(line)?..].trim_start_matches(|c: char| c.is_ascii_whitespace());
    let command = command.strip_prefix(['/', '\\']).unwrap_or(command);
    let (name, rest) = command.split_once(|c: char| c.is_ascii_whitespace())?;
    let argument = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    if argument
        .bytes()
        .any(|byte| byte.is_ascii_whitespace() || byte == b'"')
    {
        return None;
    }
    let &source = FILE_ARGUMENTS
        .iter()
        .chain(sources)
        .find(|source| source.command.eq_ignore_ascii_case(name))?;
    Some(TypedArgument {
        start: line.len() - argument.len(),
        source,
    })
}

/// The completed name and, when it is not unique, the lines listing the
/// candidates under `label`, as `PrintFileMatches` (`qcommon/common.cpp`) prints
/// files. A name met twice, in any letter case, is listed once, as first met. A
/// unique name replaces the typed text; several only extend it, keeping the letter
/// case typed so far.
fn word_completion(prefix: &str, names: &[String], label: &str) -> Option<(String, Vec<String>)> {
    let mut candidates = names
        .iter()
        .filter(|name| ascii_starts_with_ignore_case(name, prefix))
        .collect::<Vec<_>>();
    // A stable sort keeps the first met of the names equal without case before
    // the others, so dedup keeps it.
    candidates.sort_by_cached_key(|name| name.to_ascii_lowercase());
    candidates.dedup_by(|later, earlier| later.eq_ignore_ascii_case(earlier));
    if let [only] = candidates.as_slice() {
        return Some((format!("{only} "), Vec::new()));
    }
    let shared = shared_prefix(candidates.iter().map(|name| name.as_str()))?;
    // Every candidate starts with `prefix` without case, so its bytes end on a
    // boundary of `shared` too.
    let completed = format!("{prefix}{}", &shared[prefix.len()..]);
    let listing = candidates
        .iter()
        .map(|name| format!("^9{label:<6}^7{name}"))
        .collect();
    Some((completed, listing))
}

/// The completed map name and, when it is not unique, the lines listing the
/// candidates. A map matches when its name starts with `prefix`, or, for a prefix
/// without a `/`, when the name after its folder does: `siege_ho` finds
/// `mp/siege_hoth`. A unique map replaces the typed text; several extend it to their
/// shared start when that still holds what was typed, and are listed under `label`.
fn map_completion(prefix: &str, names: &[String], label: &str) -> Option<(String, Vec<String>)> {
    fn base(name: &str) -> &str {
        name.rsplit_once('/').map_or(name, |(_, base)| base)
    }
    let by_base = !prefix.contains('/');
    let mut candidates = names
        .iter()
        .filter(|name| {
            ascii_starts_with_ignore_case(name, prefix)
                || (by_base && ascii_starts_with_ignore_case(base(name), prefix))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_cached_key(|name| name.to_ascii_lowercase());
    candidates.dedup_by(|later, earlier| later.eq_ignore_ascii_case(earlier));
    if let [only] = candidates.as_slice() {
        return Some((format!("{only} "), Vec::new()));
    }
    let shared = shared_prefix(candidates.iter().map(|name| name.as_str()))?;
    let completed = if ascii_starts_with_ignore_case(shared, prefix) {
        format!("{prefix}{}", &shared[prefix.len()..])
    } else if ascii_starts_with_ignore_case(base(shared), prefix) {
        // Every match is by its name after the folder, and all are in one folder.
        shared.to_owned()
    } else {
        prefix.to_owned()
    };
    let listing = candidates
        .iter()
        .map(|name| format!("^9{label:<6}^7{name}"))
        .collect();
    Some((completed, listing))
}

/// Longest case-insensitive prefix shared by every name, spelled as the first.
fn shared_prefix<'a>(mut names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let first = names.next()?;
    let mut length = first.len();
    for name in names {
        length = first
            .bytes()
            .zip(name.bytes())
            .take(length)
            .take_while(|(left, right)| left.eq_ignore_ascii_case(right))
            .count();
    }
    while !first.is_char_boundary(length) {
        length -= 1;
    }
    Some(&first[..length])
}

/// One candidate's listing line, coloured as `PrintMatches` and
/// `PrintCvarMatches` (`qcommon/common.cpp`) print it: a grey `Cmd` or `Cvar`
/// label, the white name and, for a cvar, its value in grey quotes.
fn listed(candidate: &Candidate<'_>) -> String {
    match &candidate.value {
        None => format!("^9Cmd   ^7{}", candidate.name),
        Some(value) => format!("^9Cvar  ^7{} = ^9\"^7{value}^9\"^7", candidate.name),
    }
}

fn listed_value(value: String) -> String {
    match value.char_indices().nth(LISTED_VALUE_CHARS) {
        Some((end, _)) => format!("{}...", &value[..end]),
        None => value,
    }
}

#[cfg(test)]
mod tests {
    use crate::{BindTable, CommandRegistry, CvarDefinition, CvarFlags, CvarRegistry, Shell};

    use super::CompletionKey;

    fn shell() -> Shell {
        let mut cvars = CvarRegistry::new();
        for (name, help) in [("con_scale", "Console size"), ("con_style", "")] {
            cvars
                .register(CvarDefinition::new(name, 1.0, CvarFlags::NONE, help))
                .unwrap();
        }
        let mut shell = Shell::new(cvars, BindTable::new());
        shell.commands = CommandRegistry::new();
        shell
    }

    #[test]
    fn listing_colours_labels_values_and_descriptions_as_ejk() {
        let mut shell = shell();
        shell
            .commands
            .register("con_dump", "Dump the console", |_| Ok(Vec::new()))
            .unwrap();
        let completed = shell.complete_line("con_", CompletionKey::Tab).unwrap();
        assert_eq!(completed, "con_");
        let lines: Vec<_> = shell.lines().map(|line| line.text.as_str()).collect();
        assert_eq!(
            lines,
            [
                "]con_",
                "^9Cmd   ^7con_dump",
                "^2      Dump the console",
                "^9Cvar  ^7con_scale = ^9\"^71.0^9\"^7",
                "^2      Console size",
                "^9Cvar  ^7con_style = ^9\"^71.0^9\"^7",
            ]
        );
    }

    fn cfgs(extension: &str) -> Vec<String> {
        assert_eq!(extension, "cfg");
        ["moj.cfg", "Mod.cfg", "autoexec.cfg", "MOJ.cfg"]
            .map(str::to_owned)
            .to_vec()
    }

    #[test]
    fn exec_completes_a_unique_cfg_name() {
        let mut shell = shell();
        let completed = shell.complete_line_with_files("exec mo", CompletionKey::Tab, cfgs);
        assert_eq!(completed.as_deref(), Some("exec mo"));
        let completed = shell.complete_line_with_files("/exec mo", CompletionKey::Tab, cfgs);
        assert_eq!(completed.as_deref(), Some("/exec mo"));
        let completed =
            shell.complete_line_with_files("echo x; execq MOJ", CompletionKey::Tab, cfgs);
        assert_eq!(completed.as_deref(), Some("echo x; execq moj.cfg "));
        let completed = shell.complete_line_with_files("exec a", CompletionKey::Tab, cfgs);
        assert_eq!(completed.as_deref(), Some("exec autoexec.cfg "));
    }

    #[test]
    fn exec_lists_the_cfgs_it_could_complete_to() {
        let mut shell = shell();
        let completed = shell.complete_line_with_files("exec ", CompletionKey::Tab, cfgs);
        assert_eq!(completed.as_deref(), Some("exec "));
        let lines: Vec<_> = shell.lines().map(|line| line.text.as_str()).collect();
        assert_eq!(
            lines,
            [
                "]exec ",
                "^9File  ^7autoexec.cfg",
                "^9File  ^7Mod.cfg",
                "^9File  ^7moj.cfg",
            ]
        );
    }

    #[test]
    fn file_arguments_complete_only_on_tab_and_only_the_first_argument() {
        let mut shell = shell();
        for line in ["exec moj.cfg x", "exec \"mo", "bind mo"] {
            assert_eq!(
                shell.complete_line_with_files(line, CompletionKey::Tab, cfgs),
                None,
                "{line}"
            );
        }
        assert_eq!(
            shell.complete_line_with_files("exec mo", CompletionKey::Enter, cfgs),
            None
        );
        assert_eq!(
            shell.complete_line_with_files("exec nothing", CompletionKey::Tab, cfgs),
            None
        );
    }

    fn maps(extension: &str) -> Vec<String> {
        assert_eq!(extension, super::MAP_EXTENSION);
        [
            "mp/ffa3",
            "mp/siege_hoth",
            "mp/siege_desert",
            "sjk_arena1",
            "mp/duel1",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    #[test]
    fn devmap_completes_a_map_by_its_name_after_the_folder() {
        let mut shell = shell();
        let completed = shell.complete_line_with_files("devmap siege_ho", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap mp/siege_hoth "));
        let completed = shell.complete_line_with_files("devmap mp/FF", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap mp/ffa3 "));
        let completed = shell.complete_line_with_files("devmap sjk", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap sjk_arena1 "));
        let completed = shell.complete_line_with_files("devmap siege", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap mp/siege_"));
        assert_eq!(
            shell.complete_line_with_files("devmap nothing", CompletionKey::Tab, maps),
            None
        );
    }

    #[test]
    fn devmap_keeps_what_was_typed_when_matches_are_in_several_folders() {
        let mut shell = shell();
        let maps = |_: &str| ["mp/duel1", "jof/duel_arena"].map(str::to_owned).to_vec();
        let completed = shell.complete_line_with_files("devmap duel", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap duel"));
        let completed = shell.complete_line_with_files("devmap jof/", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap jof/duel_arena "));
    }

    #[test]
    fn devmap_lists_every_map_on_tab() {
        let mut shell = shell();
        let completed = shell.complete_line_with_files("devmap ", CompletionKey::Tab, maps);
        assert_eq!(completed.as_deref(), Some("devmap "));
        let lines: Vec<_> = shell.lines().map(|line| line.text.as_str()).collect();
        assert_eq!(
            lines,
            [
                "]devmap ",
                "^9Map   ^7mp/duel1",
                "^9Map   ^7mp/ffa3",
                "^9Map   ^7mp/siege_desert",
                "^9Map   ^7mp/siege_hoth",
                "^9Map   ^7sjk_arena1",
            ]
        );
    }

    const EMOTES: &[super::ArgumentSource] = &[super::ArgumentSource {
        command: "sjkemote",
        list: "emote",
        label: "Emote",
    }];

    fn emotes(list: &str) -> Vec<String> {
        assert_eq!(list, "emote");
        ["autumn_tea", "stop", "sync", "comrade"]
            .map(str::to_owned)
            .to_vec()
    }

    #[test]
    fn a_callers_command_completes_its_first_argument_from_its_list() {
        let mut shell = shell();
        let completed =
            shell.complete_line_with_arguments("sjkemote aut", CompletionKey::Tab, EMOTES, emotes);
        assert_eq!(completed.as_deref(), Some("sjkemote autumn_tea "));
        let completed =
            shell.complete_line_with_arguments("/SJKEMOTE Com", CompletionKey::Tab, EMOTES, emotes);
        assert_eq!(completed.as_deref(), Some("/SJKEMOTE comrade "));
        let completed =
            shell.complete_line_with_arguments("sjkemote s", CompletionKey::Tab, EMOTES, emotes);
        assert_eq!(completed.as_deref(), Some("sjkemote s"));
        let lines: Vec<_> = shell.lines().map(|line| line.text.as_str()).collect();
        assert_eq!(lines, ["]sjkemote s", "^9Emote ^7stop", "^9Emote ^7sync"]);
        for line in ["sjkemote stop x", "sjkemote nothing"] {
            assert_eq!(
                shell.complete_line_with_arguments(line, CompletionKey::Tab, EMOTES, emotes),
                None,
                "{line}"
            );
        }
        assert_eq!(
            shell.complete_line_with_arguments(
                "sjkemote aut",
                CompletionKey::Enter,
                EMOTES,
                emotes
            ),
            None
        );
        // Without the caller's sources the command's argument is left alone.
        assert_eq!(
            shell.complete_line_with_files("sjkemote aut", CompletionKey::Tab, emotes),
            None
        );
    }

    #[test]
    fn typed_lines_echo_after_a_bare_prompt() {
        let mut shell = shell();
        let _ = shell.execute_line("echo hi");
        assert_eq!(shell.lines().next().unwrap().text, "]echo hi");
    }
}
