//! The launcher notice on the SJK UI's main page (`docs/sjk-ui.md`, "SJK Launcher"):
//! for a player who starts Sol JK without the SJK Launcher, a line top left saying it
//! is out, a link to its release page and × to hide it for good
//! (`ui_launcherNotice 0`).
//!
//! A player has the launcher when it started this run (it sets `JKA_GAME_DATA` since
//! 27/10/2026) or when its settings file is on this computer (an older launcher, or
//! the game started some other way): `launcher.json` in the SJK Launcher's or the
//! JoF Launcher's folder. Nothing is asked of any server.

use crate::console::ViewerConsole;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The setting that shows the line (on by default).
pub(crate) const CVAR: &str = "ui_launcherNotice";

/// The SJK Launcher's newest release.
pub(crate) const URL: &str = "https://github.com/Sol-Vulpes/sjk-launcher/releases/latest";

/// What the launcher sets when it starts the game (`sjk-launcher`'s `games.rs`).
const LAUNCHER_ENV: &str = "JKA_GAME_DATA";

/// Whether the line shows: the player has no launcher and did not hide it.
pub(crate) fn shown(console: Option<&ViewerConsole>) -> bool {
    static WITHOUT: OnceLock<bool> = OnceLock::new();
    let without = *WITHOUT.get_or_init(|| {
        let started_by_launcher = std::env::var_os(LAUNCHER_ENV).is_some();
        let installed = launcher_settings().into_iter().any(|path| path.is_file());
        !started_by_launcher && !installed
    });
    without && console.is_none_or(|console| console.bool_cvar(CVAR) != Some(false))
}

/// Where the SJK and JoF launchers keep `launcher.json` (the `directories` crate's
/// config folder for "SJK" / "Launcher" and "SJK" / "JoF Launcher").
fn launcher_settings() -> Vec<PathBuf> {
    let folders: Vec<PathBuf> = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|roaming| {
                ["Launcher", "JoF Launcher"]
                    .into_iter()
                    .map(|name| roaming.join("SJK").join(name).join("config"))
                    .collect()
            })
            .unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Library/Application Support"))
            .map(|support| {
                ["SJK.Launcher", "SJK.JoF-Launcher"]
                    .into_iter()
                    .map(|name| support.join(name))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|config| {
                ["launcher", "joflauncher"]
                    .into_iter()
                    .map(|name| config.join(name))
                    .collect()
            })
            .unwrap_or_default()
    };
    folders
        .into_iter()
        .map(|folder| folder.join("launcher.json"))
        .collect()
}

/// Open the launcher's release page in the player's browser.
pub(crate) fn open() {
    crate::log::progress(format_args!("launcher notice: opening {URL}"));
    if !cfg!(test) {
        crate::update::open_page(URL);
    }
}

/// Hide the line for good (its ×).
pub(crate) fn hide(console: &mut ViewerConsole) {
    console.set_cvar(CVAR, "0");
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_launcher_settings_are_looked_for_in_both_launchers_folders() {
        let paths = super::launcher_settings();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().all(|path| path.ends_with("launcher.json")));
    }
}
