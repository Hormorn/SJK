//! The Sith cloud in the client (`docs/sith-cloud.md`): the S community's own
//! server of maps, skins and extras, chosen and downloaded on the same card as JoF
//! content ([`crate::jof_content::card`]) with its own settings.
//!
//! Unlike JoF content its files go straight in the game's `base`, so the game,
//! EternalJK and SJK all load them, or in `GameData/assets/S/base` once the SJK
//! Launcher has made that folder (its default): SJK then follows it, and takes a
//! file already in `base` as it is. Each place keeps one record of what was fetched
//! (`sjk-jof-content.json` beside its `base`), shared with the launcher.
//! Off by default: nothing is fetched until the player turns `sith.cloud` on and
//! presses Download.
//!
//! The SJK hub says where the server is (`GET /v1/content-sources`), so SJK can move
//! it or switch it off for everyone without a release; `sith.cloudUrl` overrides it.

use crate::console::ViewerConsole;
use crate::jof_content::card::Words;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::path::Path;
use std::sync::Mutex;

/// Allow downloads from the Sith cloud (opt in).
pub(crate) const SWITCH: &str = "sith.cloud";
/// The groups chosen ([`sjk_jof_content::Selection`]).
pub(crate) const GROUPS: &str = "sith.cloudGroups";
/// Single files picked against their group's tick (`+<id>` / `-<id>`).
pub(crate) const FILES: &str = "sith.cloudFiles";
/// Keep the chosen groups up to date by themselves (opt in).
pub(crate) const AUTO: &str = "sith.cloudAuto";
/// The groups kept up to date ([`sjk_jof_content::AutoGroups`]).
pub(crate) const AUTO_GROUPS: &str = "sith.cloudAutoGroups";
/// The Sith cloud's settings, as the generic content code reads them.
pub(crate) const NAMES: crate::jof_content::card::Names = crate::jof_content::card::Names {
    switch: SWITCH,
    groups: GROUPS,
    files: FILES,
    auto: AUTO,
    auto_groups: AUTO_GROUPS,
};
/// The server's address, set only to use another server than the one the hub names.
pub(crate) const URL: &str = "sith.cloudUrl";
/// The card's console command (all lower case: commands are matched so).
pub(crate) const COMMAND: &str = "sith.downloads";
pub(crate) const HELP: &str =
    "Open the Sith cloud: choose and download the S community's maps, skins and extras";
/// The client commands the Sith cloud adds.
pub(crate) const COMMANDS: &[(&str, &str)] = &[(COMMAND, HELP)];

pub(crate) const WORDS: Words = Words {
    title: "Sith cloud",
    about: "The S community's maps, skins and extras, from the Sith cloud server, into your game folder.",
    notice: "The Sith cloud's hosts choose and publish these files and are responsible for them; SJK only downloads what you pick and is not responsible for their content.",
    switch: "Use the Sith cloud",
    on: "On: what you pick downloads into the game folder, where SJK loads it from the next map.",
    off: "Off by default. Turn it on to download the Sith cloud's files; nothing is fetched until you press Download.",
    folder_note: "shared with the SJK Launcher's Sith cloud tab",
    repository: "the Sith cloud",
    auto_on: "On: checks at start and every hour, also in a match; new files of the groups marked Auto load from the next map.",
};

pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    let archive = CvarFlags::ARCHIVE;
    cvars.register(CvarDefinition::new(
        SWITCH,
        false,
        archive,
        "Allow downloads from the Sith cloud (the S community's maps, skins and extras) into the game folder; off by default; sith.downloads chooses and fetches them",
    ))?;
    cvars.register(CvarDefinition::new(
        GROUPS,
        sjk_jof_content::plan::DEFAULT,
        archive,
        "Sith cloud groups to keep: default (the server's recommended ones), none, or group ids",
    ))?;
    cvars.register(CvarDefinition::new(
        FILES,
        "",
        archive,
        "Sith cloud files picked one by one against their group: +<id> taken, -<id> left out (the card writes it)",
    ))?;
    cvars.register(CvarDefinition::new(
        AUTO,
        false,
        archive,
        "Update the Sith cloud's files by themselves: at start and every hour, download what changed in the groups marked Auto (sith.cloudAutoGroups); off by default; needs sith.cloud",
    ))?;
    cvars.register(CvarDefinition::new(
        AUTO_GROUPS,
        sjk_jof_content::plan::AUTO_ALL,
        archive,
        "Sith cloud groups updated by themselves: all (every group you take files from), or group ids; empty for none (the card's Auto ticks write it)",
    ))?;
    cvars.register(CvarDefinition::new(
        URL,
        "",
        archive,
        "Another Sith cloud server's address (it serves /manifest.json and /repository); empty uses the one the SJK hub names",
    ))?;
    Ok(())
}

/// The Sith cloud's root in the assets folder; its files are in that root's `base`.
fn assets_root(game_data: &Path) -> std::path::PathBuf {
    game_data.join(sjk_vfs::ASSETS_DIRECTORY).join("S")
}

/// Whether the launcher has put the Sith cloud in the assets folder.
fn in_assets(game_data: &Path) -> bool {
    assets_root(game_data).join("base").is_dir()
}

/// Where the files go: shown on the card.
pub(crate) fn folder(game_data: &Path) -> std::path::PathBuf {
    if in_assets(game_data) {
        assets_root(game_data).join("base")
    } else {
        game_data.join("base")
    }
}

/// What a refresh or sync works with, from the settings. The root's `base` is where
/// files go: the game's own, with GameData as the root, or the assets folder's, which
/// then also looks in the game's `base`, so nothing is fetched twice.
pub(crate) fn job_settings(
    console: &ViewerConsole,
    game_data: &Path,
) -> sjk_jof_content::job::Settings {
    let url = console.text_value(URL).unwrap_or("").trim();
    let (sources, directory) = if url.is_empty() {
        let directory = sjk_jof_content::Directory {
            hub: console.text_value("cl_hubUrl").unwrap_or("").to_owned(),
            collection: "sith",
            only: None,
        };
        (Vec::new(), Some(directory))
    } else {
        (vec![source(url)], None)
    };
    let (root, game_base) = if in_assets(game_data) {
        (assets_root(game_data), Some(game_data.join("base")))
    } else {
        (game_data.to_owned(), None)
    };
    sjk_jof_content::job::Settings {
        root,
        game_base,
        sources,
        directory,
        version: crate::build_info::VERSION.to_owned(),
    }
}

/// The last address's source, kept so asking again does not leak its text anew.
static SOURCE: Mutex<Option<(String, sjk_jof_content::Source)>> = Mutex::new(None);

/// The server at `url` as a source: `<url>/manifest.json` and `<url>/repository`.
fn source(url: &str) -> sjk_jof_content::Source {
    let url = url.trim().trim_end_matches('/');
    let mut known = SOURCE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some((known_url, source)) = known.as_ref()
        && known_url == url
    {
        return *source;
    }
    let leak = |text: String| -> &'static str { Box::leak(text.into_boxed_str()) };
    let source = sjk_jof_content::Source {
        name: "sith",
        manifest: leak(format!("{url}/manifest.json")),
        files: leak(format!("{url}/repository")),
    };
    *known = Some((url.to_owned(), source));
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_the_servers_list_and_files() {
        let found = source("http://example.test:8080/ ");
        assert_eq!(found.manifest, "http://example.test:8080/manifest.json");
        assert_eq!(found.files, "http://example.test:8080/repository");
    }

    #[test]
    fn the_cloud_follows_the_folder_the_launcher_made() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let game_data = directory.path();
        let settings = job_settings(&console, game_data);
        assert_eq!(
            (settings.root, settings.game_base),
            (game_data.to_owned(), None)
        );
        assert_eq!(folder(game_data), game_data.join("base"));
        let tidied = game_data.join("assets").join("S");
        std::fs::create_dir_all(tidied.join("base")).unwrap();
        let settings = job_settings(&console, game_data);
        assert_eq!(settings.root, tidied);
        assert_eq!(settings.game_base, Some(game_data.join("base")));
        assert_eq!(folder(game_data), tidied.join("base"));
    }

    #[test]
    fn the_switch_is_off_until_the_player_turns_it_on() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.bool_cvar(SWITCH), Some(false));
        assert_eq!(console.bool_cvar(AUTO), Some(false), "no automatic updates");
        assert_eq!(console.text_value(AUTO_GROUPS), Some("all"));
        assert_eq!(console.text_value(GROUPS), Some("default"));
        assert_eq!(console.text_value(URL), Some(""));
    }

    #[test]
    fn its_files_go_in_the_games_base() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let game_data = Path::new("/game/GameData");
        let settings = job_settings(&console, game_data);
        assert_eq!(settings.root, game_data);
        assert_eq!(settings.game_base, None);
        assert_eq!(folder(game_data), game_data.join("base"));
    }

    #[test]
    fn the_hub_names_the_server_unless_the_player_set_one() {
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let game_data = Path::new("/game/GameData");
        let asked = job_settings(&console, game_data);
        assert!(asked.sources.is_empty(), "no address of its own");
        let asked = asked.directory.expect("the hub is asked");
        assert_eq!(asked.collection, "sith");
        assert_eq!(asked.hub, console.text_value("cl_hubUrl").unwrap());
        console.set_cvar(URL, "http://example.test:8080");
        let set = job_settings(&console, game_data);
        assert_eq!(set.directory, None);
        assert_eq!(
            set.sources[0].manifest,
            "http://example.test:8080/manifest.json"
        );
    }
}
