//! The command and cvar reference: every entry `cmdlist`, `cvarlist` and `help`
//! know on a fresh client, plus the commands that exist only on a server (the
//! client mods', the server profiles' completions) and the key binding actions,
//! written as one Markdown page from the same tables. `export_command_reference`
//! writes it to the path in `SJK_COMMAND_REFERENCE` (`docs/development.md`,
//! "Command reference").

use super::ViewerConsole;
use sjk_shell::{CommandSource, CvarFlags, CvarValue};
use std::fmt::Write as _;

/// The cvars' sections by name prefix (any case), the first match winning. A
/// cvar no prefix claims goes under [`OTHER_AREA`].
const AREAS: &[(&str, &str)] = &[
    ("mod_", "Client mods and JoF content"),
    ("japlus.", "Client mods and JoF content"),
    ("jof.", "Client mods and JoF content"),
    ("r_", "Rendering (r_)"),
    ("gl_", "Rendering (r_)"),
    ("vid_", "Rendering (r_)"),
    ("cg_", "Game view, HUD and effects (cg_)"),
    ("ui_", "Menus (ui_)"),
    ("con_", "Console (con_)"),
    ("cl_", "Client, network and chat (cl_)"),
    ("s_", "Sound (s_)"),
    ("snd_", "Sound (s_)"),
    ("m_", "Input (m_, in_, j_, joy_)"),
    ("in_", "Input (m_, in_, j_, joy_)"),
    ("j_", "Input (m_, in_, j_, joy_)"),
    ("joy_", "Input (m_, in_, j_, joy_)"),
    ("com_", "Engine (com_, fs_, net_)"),
    ("fs_", "Engine (com_, fs_, net_)"),
    ("net_", "Engine (com_, fs_, net_)"),
];

/// The section of the cvars [`AREAS`] does not claim.
const OTHER_AREA: &str = "Player and other";

/// Where a [`TAGS`] pattern is looked for (any case).
#[derive(Clone, Copy)]
enum Field {
    Name,
    Help,
    HelpStart,
}

/// Patterns that mark an entry as one a player would rarely type, with the tag
/// they give: a guess from the entry's name and help line, shown as a tag, never
/// a reason to leave an entry out.
const TAGS: &[(Field, &str, &str)] = &[
    (Field::Name, "debug", "developer"),
    (Field::Name, "developer", "developer"),
    (Field::Name, "perfmark", "developer"),
    (Field::Help, "(diagnostic", "developer"),
    (Field::Help, "rehearse", "developer"),
    (Field::Help, "SJK_", "developer"),
    (Field::Help, "SJK staff", "staff"),
    (Field::Help, "cheats", "cheats"),
    (Field::HelpStart, "experimental", "experimental"),
    (Field::Name, "experimental", "experimental"),
    (Field::HelpStart, "internal", "internal"),
    (Field::Help, "migration marker", "internal"),
    (Field::Help, "defaults this profile has", "internal"),
];

/// The tags [`TAGS`] gives `name` and `help`, each once.
fn tags(name: &str, help: &str) -> Vec<&'static str> {
    let (name, help) = (name.to_ascii_lowercase(), help.to_ascii_lowercase());
    let mut found = Vec::new();
    for &(field, pattern, tag) in TAGS {
        let pattern = pattern.to_ascii_lowercase();
        let hit = match field {
            Field::Name => name.contains(&pattern),
            Field::Help => help.contains(&pattern),
            Field::HelpStart => help.starts_with(&pattern),
        };
        if hit && !found.contains(&tag) {
            found.push(tag);
        }
    }
    found
}

/// The usage a help line spells out for `name` (`devmap <map>: ...`,
/// `...: remapShader <old> <new>`): the name as written, then its words up to
/// the first plain word after an argument (one in `<>` or `[]`, or an `a|b`
/// choice), or a `:`, `,`, `;`, `(` or `)` outside brackets. `None` when the
/// help shows no argument.
fn usage(name: &str, help: &str) -> Option<String> {
    let mut from = 0;
    while let Some(found) = help[from..].find(name) {
        let start = from + found;
        let end = start + name.len();
        from = end;
        let boundary = help[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '.' | '+'));
        if !boundary || !help[end..].starts_with(' ') {
            continue;
        }
        let mut words = vec![name];
        let mut argument = false;
        let mut word_start = None;
        let mut depth = 0_u32;
        let tail = &help[end..];
        let mut stop = tail.len();
        for (index, c) in tail.char_indices() {
            if depth == 0 && matches!(c, ':' | ',' | ';' | '(' | ')') {
                stop = index;
                break;
            }
            match c {
                '<' | '[' => depth += 1,
                '>' | ']' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if c == ' ' && depth == 0 {
                if let Some(begin) = word_start.take()
                    && !take_word(&tail[begin..index], &mut words, &mut argument)
                {
                    break;
                }
            } else if word_start.is_none() {
                word_start = Some(index);
            }
        }
        if let Some(begin) = word_start
            && depth == 0
        {
            take_word(&tail[begin..stop], &mut words, &mut argument);
        }
        if argument {
            return Some(words.join(" ").trim_end_matches('.').to_owned());
        }
    }
    None
}

/// Add `word` to a usage being read, unless it is a plain word after an
/// argument, which ends the usage (`false`).
fn take_word<'a>(word: &'a str, words: &mut Vec<&'a str>, argument: &mut bool) -> bool {
    let is_argument = word.contains(['<', '[', '|']);
    if !is_argument && *argument {
        return false;
    }
    *argument |= is_argument;
    words.push(word);
    true
}

/// `text` with runs of spaces as one and a full stop at the end.
fn sentence(text: &str) -> String {
    let mut out = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if out.is_empty() {
        out.push_str("(no description)");
    }
    if !out.ends_with(['.', '!', '?']) {
        out.push('.');
    }
    out
}

/// A command's line: `name`, its help, usage, where it applies (when its
/// section does not say) and its tags.
fn command_line(out: &mut String, name: &str, help: &str, applies: Option<&str>) {
    let _ = write!(out, "- `{name}`: {}", sentence(help));
    if let Some(usage) = usage(name, help) {
        let _ = write!(out, " Usage: `{usage}`.");
    }
    if let Some(applies) = applies {
        let _ = write!(out, " Applies: {applies}.");
    }
    for tag in tags(name, help) {
        let _ = write!(out, " [{tag}]");
    }
    out.push('\n');
}

fn value_kind(value: &CvarValue) -> &'static str {
    match value {
        CvarValue::Bool(_) => "on/off",
        CvarValue::Integer(_) => "integer",
        CvarValue::Float(_) => "number",
        CvarValue::Text(_) => "text",
    }
}

fn flag_words(flags: CvarFlags) -> impl Iterator<Item = &'static str> {
    [
        (CvarFlags::ARCHIVE, "saved"),
        (CvarFlags::USER_INFO, "sent to the server (userinfo)"),
        (CvarFlags::SERVER_INFO, "serverinfo"),
        (CvarFlags::READ_ONLY, "read-only"),
    ]
    .into_iter()
    .filter(move |(flag, _)| flags.contains(*flag))
    .map(|(_, word)| word)
}

/// The servers of `kind` in words, for the server commands' headings.
fn kind_name(kind: sjk_mod::ServerKind) -> &'static str {
    match kind {
        sjk_mod::ServerKind::BaseJka => "base JKA",
        sjk_mod::ServerKind::JaPlus => "JA+",
        sjk_mod::ServerKind::JaPro => "jaPRO",
        sjk_mod::ServerKind::Other => "other",
    }
}

/// Counts of what [`markdown`] wrote, for the export's report.
#[derive(Debug, Default)]
struct Counts {
    commands: usize,
    mod_commands: usize,
    server_commands: usize,
    cvars: usize,
    bind_actions: usize,
}

/// A titled list of server commands.
type ServerSection<'a> = (String, Vec<(&'a str, &'a str)>);

/// The server commands the mods complete and the server profiles list: each
/// kind of server first, then the families SJK knows by name, a family showing
/// only what its kind does not already give.
fn server_sections(modules: &[Box<dyn sjk_mod::ClientMod>]) -> Vec<ServerSection<'static>> {
    let kinds = [
        sjk_mod::ServerKind::BaseJka,
        sjk_mod::ServerKind::JaPlus,
        sjk_mod::ServerKind::JaPro,
        sjk_mod::ServerKind::Other,
    ]
    .into_iter()
    .map(|kind| {
        let server = sjk_mod::Server {
            kind,
            address: String::new(),
            info: String::new(),
        };
        (format!("{} servers", kind_name(kind)), server)
    });
    let families = sjk_mod::KNOWN_SERVERS.iter().map(|family| {
        let server = sjk_mod::Server {
            kind: family.kind,
            address: String::new(),
            info: format!("\\{}\\{}", family.key, family.value),
        };
        let name = format!("{} servers ({})", family.name, kind_name(family.kind));
        (name, server)
    });
    let mut sections = Vec::new();
    let mut listed: Vec<(&str, &str)> = Vec::new();
    for (servers, server) in kinds.chain(families) {
        for module in modules {
            if !module.loads_on(&server) {
                continue;
            }
            let fresh: Vec<_> = module
                .server_commands(&server)
                .iter()
                .filter(|command| !listed.contains(&(module.id(), command.name)))
                .map(|command| (command.name, command.help))
                .collect();
            if fresh.is_empty() {
                continue;
            }
            listed.extend(fresh.iter().map(|(name, _)| (module.id(), *name)));
            let title = format!(
                "On {servers}, completed by {} (`{}`)",
                module.title(),
                crate::mods::switch(module.id())
            );
            sections.push((title, fresh));
        }
    }
    for profile in [
        sjk_client::CompatProfile::BaseJka,
        sjk_client::CompatProfile::JaPlus { version: None },
        sjk_client::CompatProfile::TaystJk,
    ] {
        let commands: Vec<_> = sjk_client::compat_console_commands(&profile)
            .map(|command| (command.name, command.description))
            .collect();
        if !commands.is_empty() {
            sections.push((format!("On {profile} servers, completed by SJK"), commands));
        }
    }
    sections
}

/// The reference page of `console`'s commands and cvars, a fresh client's.
fn markdown(console: &ViewerConsole) -> (String, Counts) {
    let mut counts = Counts::default();
    let shell = &console.shell;
    let mut out = String::new();
    let _ = writeln!(out, "# SJK console commands and cvars\n");
    let _ = writeln!(
        out,
        "Generated from {} by `export_command_reference` (docs/development.md, \
         \"Command reference\"); do not edit by hand.\n",
        crate::build_info::label()
    );
    out.push_str(HEADER);

    let mut builtins = Vec::new();
    let mut client = Vec::new();
    for (name, help, source) in shell.command_help() {
        match source {
            CommandSource::Shell => builtins.push((name, help)),
            CommandSource::Application | CommandSource::External => client.push((name, help)),
        }
    }
    for list in [&mut builtins, &mut client] {
        list.sort_by_key(|(name, _)| name.to_ascii_lowercase());
        list.dedup_by_key(|(name, _)| name.to_ascii_lowercase());
    }
    let (held, client): (Vec<_>, Vec<_>) = client
        .into_iter()
        .partition(|(name, _)| name.starts_with(['+', '-']));
    out.push_str("\n## Console and scripting commands\n\n");
    out.push_str(
        "The console's own commands, as in the original game's console: cvars, binds, \
         scripts and files. They work everywhere.\n\n",
    );
    for (name, help) in &builtins {
        command_line(&mut out, name, help, None);
    }
    out.push_str("\n## Client commands\n\n");
    out.push_str("SJK's own commands. They work everywhere, on any server or none.\n\n");
    for (name, help) in &client {
        command_line(&mut out, name, help, None);
    }
    out.push_str(
        "\n### Held actions\n\nCommands for key binds: `+name` runs while the key is \
         held, `-name` when it is let go (`bind mouse1 +attack`).\n\n",
    );
    for (name, help) in &held {
        command_line(&mut out, name, help, None);
    }
    counts.commands = builtins.len() + client.len() + held.len();

    out.push_str("\n## Client mod commands\n\n");
    out.push_str(
        "Commands of the client mods built into SJK. A mod's commands exist only while \
         its switch (`mod_<id>`, on by default) is on and the client is on a server the \
         mod loads on; elsewhere the console says where they work. `mods` lists the \
         mods and whether each is on.\n",
    );
    let modules = crate::mods::builtin();
    for module in &modules {
        let switch = crate::mods::switch(module.id());
        let _ = writeln!(
            out,
            "\n### {} (`{switch}`)\n\n{}\n",
            module.title(),
            sentence(module.about())
        );
        let applies = format!(
            "only on {} servers, while `{switch}` is on",
            module.servers()
        );
        for command in module.commands() {
            command_line(&mut out, command.name, command.help, Some(&applies));
            counts.mod_commands += 1;
        }
    }

    out.push_str("\n## Server commands SJK knows\n\n");
    out.push_str(
        "Commands the server runs, which SJK completes and lists in `cmdlist` while the \
         client is on such a server. SJK sends a command it does not know to the server \
         it is on, so servers have many more than these; the server decides what each \
         does and who may use it (admin commands need an admin login there).\n",
    );
    for (title, commands) in server_sections(&modules) {
        let _ = writeln!(out, "\n### {title}\n");
        for (name, help) in commands {
            command_line(&mut out, name, help, None);
            counts.server_commands += 1;
        }
    }

    out.push_str("\n## Key binding actions\n\n");
    out.push_str(
        "The actions Settings > Controls (key bindings) offers, with their console \
         command and default key: `bind <key> <command>` binds one from the console.\n",
    );
    let mut category = None;
    for action in crate::keybind_editor::ACTIONS.iter() {
        if category != Some(action.category) {
            category = Some(action.category);
            let caption = crate::keybind_editor::CATEGORIES[action.category as usize];
            let _ = writeln!(out, "\n### {caption}\n");
        }
        let key = if action.default_key.is_empty() {
            "unbound by default".to_owned()
        } else {
            format!("default key {}", action.default_key)
        };
        let _ = writeln!(out, "- {}: `{}` ({key})", action.label, action.command);
        counts.bind_actions += 1;
    }

    out.push_str("\n## Cvars\n\n");
    out.push_str(
        "Settings the console reads and sets: `<name> <value>` sets one, `<name>` alone \
         shows its value, `reset <name>` restores the default. Each line gives the \
         default, the value type and flags (\"saved\": kept in the profile's config.cfg \
         between runs). \"Settings:\" names the Settings group and row that edit it \
         (searching Settings for the row's name finds it) and the row's help line. A \
         cvar without a Settings row is changed from the console only.\n",
    );
    let mod_settings: Vec<(&str, String)> = modules
        .iter()
        .flat_map(|module| {
            let owner = format!(
                "{} setting, used on {} servers",
                module.title(),
                module.servers()
            );
            module
                .settings()
                .iter()
                .map(move |setting| (setting.name, owner.clone()))
        })
        .collect();
    let mut cvars: Vec<_> = shell.cvars.iter().collect();
    cvars.sort_by_key(|cvar| cvar.name.to_ascii_lowercase());
    let area_of = |name: &str| {
        let lower = name.to_ascii_lowercase();
        AREAS
            .iter()
            .find(|(prefix, _)| lower.starts_with(prefix))
            .map_or(OTHER_AREA, |(_, area)| *area)
    };
    let mut areas: Vec<&str> = AREAS.iter().map(|(_, area)| *area).collect();
    areas.dedup();
    areas.push(OTHER_AREA);
    for area in areas {
        let mut in_area = cvars
            .iter()
            .filter(|cvar| area_of(&cvar.name) == area)
            .peekable();
        if in_area.peek().is_none() {
            continue;
        }
        let _ = writeln!(out, "\n### {area}\n");
        for cvar in in_area {
            let _ = write!(
                out,
                "- `{}` (default `{}`, {}",
                cvar.name,
                cvar.default.as_text(),
                value_kind(&cvar.default)
            );
            for word in flag_words(cvar.flags) {
                let _ = write!(out, ", {word}");
            }
            let _ = write!(out, "): {}", sentence(&cvar.description));
            if let Some((_, owner)) = mod_settings
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(&cvar.name))
            {
                let _ = write!(out, " Applies: {owner}.");
            }
            if let Some((group, label, help)) = crate::settings::place(&cvar.name) {
                let _ = write!(out, " Settings: {group} > \"{label}\"");
                match help {
                    Some(help) => {
                        let _ = write!(out, ", {}", sentence(help));
                    }
                    None => out.push('.'),
                }
            }
            for tag in tags(&cvar.name, &cvar.description) {
                let _ = write!(out, " [{tag}]");
            }
            out.push('\n');
            counts.cvars += 1;
        }
    }
    (out, counts)
}

/// How players reach these entries, shown under the title.
const HEADER: &str = "\
## How to use this list

- Commands and cvars are typed in the console: open it with the console key (the \
key under Escape, `~` or `\u{b2}` on most layouts; Shift+Escape also toggles it) and \
type the name, then Enter. One leading `/` or `\\` is optional (`/connect` and \
`connect` are the same). Chat does not run commands: a line typed in the chat box is \
sent as a message.
- `cmdlist [text]` and `cvarlist [text]` list the commands and cvars whose names \
contain the text, `help <name>` describes one, and `consolebrowser` (F3 in the \
console) searches both and edits cvar values.
- A command SJK does not know is sent to the server the client is on, so servers \
(JA+, jaPRO, JoF) have commands of their own that this list does not hold.
- If a command or cvar is not in this list, say you are not sure rather than guess.
- Tags: [developer] marks diagnostics and test tools a player rarely needs, [staff] \
the SJK team's tools (they need a staff key), [cheats] what needs cheats (a local \
`devmap`), [experimental] features that are off by default, [internal] values the \
client keeps for itself (players never set them). Tags are a guess from the name and \
help text; untagged entries can still be technical.
";

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> (tempfile::TempDir, ViewerConsole) {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        (directory, console)
    }

    #[test]
    fn the_reference_holds_every_command_cvar_and_mod_command() {
        let (_directory, console) = console();
        let (page, counts) = markdown(&console);
        for (name, _, _) in console.shell.command_help() {
            assert!(page.contains(&format!("- `{name}`:")), "command {name}");
        }
        for cvar in console.shell.cvars.iter() {
            let line = format!("- `{}` (default", cvar.name);
            assert!(page.contains(&line), "cvar {}", cvar.name);
        }
        for module in crate::mods::builtin() {
            for command in module.commands() {
                let line = format!("- `{}`:", command.name);
                assert!(page.contains(&line), "{}", command.name);
            }
        }
        assert_eq!(counts.cvars, console.shell.cvars.iter().count());
        assert!(counts.commands > 0 && counts.server_commands > 0);
        assert_eq!(counts.bind_actions, crate::keybind_editor::ACTIONS.len());
    }

    #[test]
    fn usage_is_read_from_the_help_line() {
        let cases = [
            (
                "remapShader",
                "Replace a local shader: remapShader <old> <new>",
                Some("remapShader <old> <new>"),
            ),
            (
                "devmap",
                "devmap <map>: play a local map with cheats enabled",
                Some("devmap <map>"),
            ),
            ("connect", "Connect to host[:port]", None),
            ("play", "Play local sound files", None),
            (
                "sjkemote",
                "Play one (sjkemote alone lists them; sjkemote sync [player] joins one)",
                Some("sjkemote sync [player]"),
            ),
            (
                "sjkavatar",
                "sjkavatar <file> shows a picture",
                Some("sjkavatar <file>"),
            ),
            (
                "+wheel",
                "Hold: +wheel <page> (general, weather...)",
                Some("+wheel <page>"),
            ),
            (
                "demo_camera",
                "demo_camera front|back [seconds]; stop",
                Some("demo_camera front|back [seconds]"),
            ),
            ("guntele", "japlus.guntele [distance] (JA+ admin)", None),
        ];
        for (name, help, expected) in cases {
            assert_eq!(usage(name, help).as_deref(), expected, "{name}");
        }
    }

    #[test]
    fn tags_mark_tools_players_rarely_need() {
        let cases: [(&str, &str, &[&str]); 5] = [
            ("ghost", "Showcase tools, SJK staff only: ...", &["staff"]),
            ("graphicsquality", "vanilla, sjk or experimental", &[]),
            ("fs_debug", "Log mounted sources", &["developer"]),
            (
                "rateDefaultVersion",
                "Internal migration marker",
                &["internal"],
            ),
            ("staff", "Open the SJK staff tools", &["staff"]),
        ];
        for (name, help, expected) in cases {
            assert_eq!(tags(name, help), expected, "{name}");
        }
    }

    /// Write the reference to the file `SJK_COMMAND_REFERENCE` names.
    #[test]
    #[ignore = "writes the command reference; set SJK_COMMAND_REFERENCE to the output file"]
    fn export_command_reference() {
        let path = std::env::var_os("SJK_COMMAND_REFERENCE")
            .expect("set SJK_COMMAND_REFERENCE to the Markdown file to write");
        let (_directory, console) = console();
        let (page, counts) = markdown(&console);
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, page).unwrap();
        println!("wrote {} ({counts:?})", path.display());
    }
}
