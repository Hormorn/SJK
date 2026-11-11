//! JoF content in the client (`docs/jof-content.md`): its settings, the JoF
//! folder joining the search paths while the switch is on, and the card that
//! downloads ([`card`]). The work itself is `sjk-jof-content`'s.
//!
//! Off by default: JoF's files load only once the player turns
//! `jof.content` on, and are fetched only when the player asks on the card.

use crate::console::ViewerConsole;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[path = "jof_content_card.rs"]
pub(crate) mod card;

/// Load the JoF folder's files (opt in).
pub(crate) const SWITCH: &str = "jof.content";
/// The groups chosen ([`sjk_jof_content::Selection`]).
pub(crate) const GROUPS: &str = "jof.contentGroups";
/// Single files picked against their group's tick (`+<id>` / `-<id>`).
pub(crate) const FILES: &str = "jof.contentFiles";
/// Keep the chosen groups up to date by themselves (opt in).
pub(crate) const AUTO: &str = "jof.contentAuto";
/// The groups kept up to date ([`sjk_jof_content::AutoGroups`]).
pub(crate) const AUTO_GROUPS: &str = "jof.contentAutoGroups";
/// JoF content's settings, as the generic content code reads them.
pub(crate) const NAMES: card::Names = card::Names {
    switch: SWITCH,
    groups: GROUPS,
    files: FILES,
    auto: AUTO,
    auto_groups: AUTO_GROUPS,
};
/// The repository host ([`sjk_jof_content::SourceChoice`]).
pub(crate) const SOURCE: &str = "jof.contentSource";
/// The folder, when not the default `<SJK folder>/jof`.
pub(crate) const FOLDER: &str = "jof.contentFolder";
/// The card's console command (all lower case: commands are matched so).
pub(crate) const COMMAND: &str = "jof.downloads";
pub(crate) const HELP: &str =
    "Open JoF content: choose and download JoF's maps, skins, hilts and packs";
/// The client commands JoF content adds.
pub(crate) const COMMANDS: &[(&str, &str)] = &[(COMMAND, HELP)];
/// The source choices the settings row offers, as `jof.contentSource` takes them.
pub(crate) const SOURCES: [&str; 3] = ["auto", "mirror", "origin"];

/// The owner's name in the search paths' extra roots.
const ROOT_OWNER: &str = "jof";

pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    let archive = CvarFlags::ARCHIVE;
    cvars.register(CvarDefinition::new(
        SWITCH,
        false,
        archive,
        "Load JoF content (JoF's maps, skins, hilts and packs) from SJK's JoF folder; off by default; jof.downloads chooses and fetches them",
    ))?;
    cvars.register(CvarDefinition::new(
        GROUPS,
        sjk_jof_content::plan::DEFAULT,
        archive,
        "JoF content groups to keep: default (the repository's recommended ones), none, or group ids",
    ))?;
    cvars.register(CvarDefinition::new(
        FILES,
        "",
        archive,
        "JoF content files picked one by one against their group: +<id> taken, -<id> left out (the card writes it)",
    ))?;
    cvars.register(CvarDefinition::new(
        AUTO,
        false,
        archive,
        "Update JoF content by itself: at start and every hour, download what changed in the groups marked Auto (jof.contentAutoGroups); off by default; needs jof.content",
    ))?;
    cvars.register(CvarDefinition::new(
        AUTO_GROUPS,
        sjk_jof_content::plan::AUTO_ALL,
        archive,
        "JoF content groups updated by themselves: all (every group you take files from), or group ids; empty for none (the card's Auto ticks write it)",
    ))?;
    cvars.register(CvarDefinition::new(
        SOURCE,
        "auto",
        archive,
        "Where JoF content comes from: auto (JoF Cloud, then jofhub.eu), mirror (JoF Cloud) or origin (jofhub.eu)",
    ))?;
    cvars.register(CvarDefinition::new(
        FOLDER,
        "",
        archive,
        "Folder for JoF content; empty is the jof folder beside SJK's settings",
    ))?;
    Ok(())
}

/// The JoF folder: `jof.contentFolder`, else `jof` beside the settings.
pub(crate) fn folder(console: &ViewerConsole) -> PathBuf {
    folder_from(
        console.text_value(FOLDER).unwrap_or(""),
        console.config_directory(),
    )
}

fn folder_from(setting: &str, config_directory: &Path) -> PathBuf {
    let setting = setting.trim();
    if setting.is_empty() {
        config_directory.join("jof")
    } else {
        PathBuf::from(setting)
    }
}

/// The switch and folder last given to the search paths.
static FOLLOWED: Mutex<Option<(bool, String)>> = Mutex::new(None);

/// Put the JoF folder in the search paths while the switch is on, and take it
/// out when it goes off. Cheap when nothing changed; called every frame.
pub(crate) fn follow(console: &ViewerConsole) {
    let on = console.bool_cvar(SWITCH).unwrap_or(false);
    let setting = console.text_value(FOLDER).unwrap_or("");
    let mut followed = FOLLOWED.lock().unwrap_or_else(|error| error.into_inner());
    if followed
        .as_ref()
        .is_some_and(|(known_on, known)| *known_on == on && known == setting)
    {
        return;
    }
    *followed = Some((on, setting.to_owned()));
    let root = on.then(|| folder_from(setting, console.config_directory()));
    if crate::assets::search_paths::set_extra_root(ROOT_OWNER, root.clone()) {
        match root {
            Some(root) => crate::log::progress(format_args!(
                "JoF content: loading {} from the next map",
                root.join("base").display()
            )),
            None => crate::log::progress(format_args!("JoF content: off")),
        }
    }
}

/// What a refresh or sync works with, from the settings.
pub(crate) fn job_settings(
    console: &ViewerConsole,
    game_data: &Path,
) -> sjk_jof_content::job::Settings {
    let choice = sjk_jof_content::SourceChoice::parse(console.text_value(SOURCE).unwrap_or("auto"));
    sjk_jof_content::job::Settings {
        root: folder(console),
        game_base: Some(game_data.join("base")),
        sources: choice.sources(),
        // The hub names the hosts; the built-in ones are kept for when it can't be asked.
        directory: Some(sjk_jof_content::Directory {
            hub: console.text_value("cl_hubUrl").unwrap_or("").to_owned(),
            collection: "jof",
            only: match choice {
                sjk_jof_content::SourceChoice::Auto => None,
                sjk_jof_content::SourceChoice::Only(source) => Some(source.name.to_owned()),
            },
        }),
        version: crate::build_info::VERSION.to_owned(),
    }
}

impl crate::GpuState {
    /// Each frame: the content collections' automatic updates
    /// ([`ViewerConsole::content_auto`]), a comparison while none is due; one that
    /// changed files gets its card at the top of the screen.
    pub(crate) fn update_content_auto(&mut self) {
        let Some(console) = self.console.as_mut() else {
            return;
        };
        for updated in console
            .content_auto(std::time::Instant::now(), &self.game_data)
            .into_iter()
            .flatten()
        {
            self.unlock_toast.push(updated);
        }
    }

    /// Each frame: model sync's tickets, results and automatic downloads
    /// ([`crate::model_sync::update`]); the players met are read only when it asks.
    pub(crate) fn update_model_sync(&mut self) {
        let Some(console) = self.console.as_mut() else {
            return;
        };
        let session = self.live_session.as_ref();
        // An inspected player wearing a model this game lacks: say how to get it.
        let inspected = self.hud.card.pinned_client().and_then(|client| {
            let game = session?.game_state();
            let info =
                game.config_string(crate::player_identity::CS_PLAYERS + usize::from(client))?;
            let info = sjk_client::LegacyClientInfo::new(info);
            let model = info
                .text("model")
                .and_then(crate::installed_models::folder)?;
            let name = info
                .bytes("n")
                .map(|bytes| sjk_client::decode_legacy(bytes).into_owned())
                .unwrap_or_default();
            Some((name, model))
        });
        crate::model_sync::offer(console, &self.game_data, inspected);
        crate::model_sync::update(console, &self.game_data, || {
            let Some(session) = session else {
                return Vec::new();
            };
            let game = session.game_state();
            (0..64)
                .filter(|&slot| i32::try_from(slot).ok() != Some(game.client_num))
                .filter_map(|slot| game.config_string(crate::player_identity::CS_PLAYERS + slot))
                .filter_map(|info| {
                    sjk_client::LegacyClientInfo::new(info)
                        .text("model")
                        .and_then(crate::installed_models::folder)
                })
                .collect()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_is_beside_the_settings_unless_one_is_given() {
        let config = Path::new("/game/SJK");
        assert_eq!(folder_from("", config), config.join("jof"));
        assert_eq!(folder_from("  D:/JoF  ", config), PathBuf::from("D:/JoF"));
    }

    #[test]
    fn an_automatic_update_waits_for_both_switches_and_a_group() {
        use crate::jof_content::card::{AUTO_EVERY, AUTO_FIRST};
        let directory = tempfile::tempdir().unwrap();
        let mut console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        let game_data = directory.path().join("GameData");
        let start = std::time::Instant::now();
        let mut at = start + AUTO_FIRST;
        let tick = |console: &mut ViewerConsole, at: std::time::Instant| {
            assert!(
                console
                    .content_auto(at, &game_data)
                    .iter()
                    .all(Option::is_none)
            );
            assert!(!console.jof_content.busy() && !console.sith_cloud.busy());
        };
        tick(&mut console, start);
        // Off by default.
        tick(&mut console, at);
        // The automatic switch alone does nothing.
        at += AUTO_EVERY;
        console.set_cvar(AUTO, "1");
        console.set_cvar(crate::sith_cloud::AUTO, "1");
        tick(&mut console, at);
        // Both on, but no group marked Auto.
        at += AUTO_EVERY;
        console.set_cvar(SWITCH, "1");
        console.set_cvar(crate::sith_cloud::SWITCH, "1");
        console.set_cvar(AUTO_GROUPS, "");
        console.set_cvar(crate::sith_cloud::AUTO_GROUPS, "none");
        tick(&mut console, at);
    }

    #[test]
    fn the_switch_is_off_until_the_player_turns_it_on() {
        let directory = tempfile::tempdir().unwrap();
        let console = ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
        assert_eq!(console.bool_cvar(SWITCH), Some(false));
        assert_eq!(console.bool_cvar(AUTO), Some(false), "no automatic updates");
        assert_eq!(console.text_value(AUTO_GROUPS), Some("all"));
        assert_eq!(console.text_value(GROUPS), Some("default"));
        assert_eq!(console.text_value(SOURCE), Some("auto"));
    }
}
