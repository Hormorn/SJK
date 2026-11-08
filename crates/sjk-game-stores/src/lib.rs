//! Jedi Academy installs that game stores know of, for a client kept outside the
//! game's `GameData` folder (a ZIP extracted to Downloads, say).
//!
//! Steam: its install is found in the registry on Windows and at its usual folders
//! elsewhere; `steamapps/libraryfolders.vdf` lists every library (games on another
//! drive live in their own), and a library holds the game when it has the app's
//! `appmanifest_<id>.acf`, whose `installdir` names its folder under
//! `steamapps/common`. GOG Galaxy and GOG's offline installers record each game's folder
//! in the registry on Windows.
//!
//! The candidates are not checked here: [`find`] keeps the first that the caller's own
//! test accepts (SJK's client wants the retail `base/assets0.pk3` to `assets3.pk3`).
//! Shared by SJK's client (`sjk-viewer`'s launch discovery, after the folder beside the
//! program and the saved path) and the SJK Launcher. The registry is read through
//! Windows' own `reg.exe`, as the workspace allows no `unsafe` here.

use std::path::{Path, PathBuf};

/// A store's entry for Jedi Academy: its app ID and the folder it installs to.
struct SteamApp {
    /// The app ID in `appmanifest_<id>.acf`.
    id: u32,
    /// The folder under `steamapps/common` when the manifest is missing or unreadable.
    folder: &'static str,
}

/// The Steam apps whose install holds a Jedi Academy `GameData`.
///
/// - 6020: Star Wars Jedi Knight: Jedi Academy (store.steampowered.com/app/6020); its
///   folder is `steamapps/common/Jedi Academy`, with `GameData` inside on Windows and
///   Linux and the Mac edition's app bundle inside on a Mac.
const STEAM_APPS: &[SteamApp] = &[SteamApp {
    id: 6020,
    folder: "Jedi Academy",
}];

/// Registry values naming Steam's own folder on Windows, first found first used.
///
/// - `HKCU\Software\Valve\Steam` `SteamPath`: written by the Steam client for the
///   signed-in Windows user (forward slashes, lower case).
/// - `HKLM\SOFTWARE\WOW6432Node\Valve\Steam` `InstallPath`: written by Steam's
///   installer, for every user.
#[cfg(windows)]
const STEAM_REGISTRY: &[(&str, &str)] = &[
    (r"HKCU\Software\Valve\Steam", "SteamPath"),
    (r"HKLM\SOFTWARE\WOW6432Node\Valve\Steam", "InstallPath"),
];

/// The registry key whose subkeys are GOG's installed games, each with its folder in
/// `path` (GOG Galaxy and GOG's offline installers write it).
#[cfg(windows)]
const GOG_GAMES_REGISTRY: &str = r"HKLM\SOFTWARE\WOW6432Node\GOG.com\Games";

/// Every `GameData` folder a store says Jedi Academy may be in, Steam's first, then
/// GOG's; `home` is the user's home folder for Steam's Linux and Mac locations.
pub fn game_data_candidates(home: Option<&Path>) -> Vec<PathBuf> {
    let mut installs = Vec::new();
    for library in steam_libraries(&steam_roots(home)) {
        installs.extend(steam_installs(&library));
    }
    let mut candidates = Vec::new();
    for install in installs {
        push_unique(&mut candidates, install.join("GameData"));
        candidates.extend(mac_bundles(&install));
    }
    for install in gog_installs() {
        push_unique(&mut candidates, install.join("GameData"));
    }
    candidates
}

/// The first of [`game_data_candidates`] that `is_game_data` accepts.
pub fn find(home: Option<&Path>, is_game_data: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    game_data_candidates(home)
        .into_iter()
        .find(|candidate| is_game_data(candidate))
}

/// Steam's own folders on this machine: the registry's on Windows, then the folders
/// Steam installs to by default.
fn steam_roots(home: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    for (key, value) in STEAM_REGISTRY {
        if let Some(path) = registry_value(key, value) {
            push_unique(&mut roots, PathBuf::from(path));
        }
    }
    #[cfg(windows)]
    if let Some(programs) = std::env::var_os("ProgramFiles(x86)") {
        push_unique(&mut roots, PathBuf::from(programs).join("Steam"));
    }
    if let Some(home) = home {
        for folder in [
            ".local/share/Steam",
            ".steam/steam",
            // Steam from Flathub.
            ".var/app/com.valvesoftware.Steam/.local/share/Steam",
            // Steam from the Snap Store.
            "snap/steam/common/.local/share/Steam",
            "Library/Application Support/Steam",
        ] {
            push_unique(&mut roots, home.join(folder));
        }
    }
    roots
}

/// The Steam libraries of `roots`: each root, then the folders its
/// `steamapps/libraryfolders.vdf` lists.
fn steam_libraries(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    for root in roots {
        if !root.join("steamapps").is_dir() {
            continue;
        }
        push_unique(&mut libraries, root.clone());
        let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) else {
            continue;
        };
        for path in library_paths(&text) {
            push_unique(&mut libraries, PathBuf::from(path));
        }
    }
    libraries
}

/// The game's folders in `library`: the folder its manifest names, then the usual one.
fn steam_installs(library: &Path) -> Vec<PathBuf> {
    let common = library.join("steamapps/common");
    let mut installs = Vec::new();
    for app in STEAM_APPS {
        let manifest = library.join(format!("steamapps/appmanifest_{}.acf", app.id));
        if let Some(folder) = std::fs::read_to_string(manifest)
            .ok()
            .and_then(|text| install_dir(&text))
            .filter(|folder| is_plain_folder_name(folder))
        {
            push_unique(&mut installs, common.join(folder));
        }
        push_unique(&mut installs, common.join(app.folder));
    }
    installs
}

/// The Mac edition's app bundles in a Steam install, whose `Contents` holds `base`.
fn mac_bundles(install: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(install) else {
        return Vec::new();
    };
    let mut bundles: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "app"))
        .map(|path| path.join("Contents"))
        .collect();
    bundles.sort();
    bundles
}

/// The folders of GOG's installed games (none outside Windows).
fn gog_installs() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        registry_query(&[GOG_GAMES_REGISTRY, "/s", "/v", "path"])
            .map(|output| {
                registry_values(&output, "path")
                    .into_iter()
                    .map(PathBuf::from)
                    .collect()
            })
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// A `manifest`'s `installdir` must be one folder under `steamapps/common`, never a
/// path that leaves it.
fn is_plain_folder_name(folder: &str) -> bool {
    !folder.is_empty() && folder != "." && folder != ".." && !folder.contains(['/', '\\', ':'])
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

/// The library folders a `libraryfolders.vdf` lists: each library's `"path"` (Steam
/// since 2021), or, in the older file, the folders under numbered keys.
fn library_paths(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for (depth, key, value) in key_values(text) {
        let numbered = depth == 1 && key.bytes().all(|byte| byte.is_ascii_digit());
        if (key.eq_ignore_ascii_case("path") || numbered)
            && !value.is_empty()
            && !paths.contains(&value)
        {
            paths.push(value);
        }
    }
    paths
}

/// The `installdir` of an `appmanifest_<id>.acf`.
fn install_dir(text: &str) -> Option<String> {
    key_values(text)
        .into_iter()
        .find(|(depth, key, _)| *depth == 1 && key.eq_ignore_ascii_case("installdir"))
        .map(|(_, _, value)| value)
}

/// The `"key" "value"` pairs of a Valve KeyValues text (`.vdf`, `.acf`), each with how
/// many `{` blocks hold it; `//` comments are skipped, and `\\`, `\"`, `\n` and `\t`
/// are unescaped as Steam writes them.
fn key_values(text: &str) -> Vec<(usize, String, String)> {
    #[derive(PartialEq)]
    enum Token {
        Text(String),
        Open,
        Close,
    }
    let mut tokens = Vec::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '{' => tokens.push(Token::Open),
            '}' => tokens.push(Token::Close),
            '"' => {
                let mut value = String::new();
                while let Some(character) = characters.next() {
                    match character {
                        '"' => break,
                        '\\' => match characters.next() {
                            Some('n') => value.push('\n'),
                            Some('t') => value.push('\t'),
                            Some(other) => value.push(other),
                            None => break,
                        },
                        other => value.push(other),
                    }
                }
                tokens.push(Token::Text(value));
            }
            '/' if characters.peek() == Some(&'/') => {
                for character in characters.by_ref() {
                    if character == '\n' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    let mut pairs = Vec::new();
    let mut depth = 0_usize;
    let mut tokens = tokens.into_iter().peekable();
    while let Some(token) = tokens.next() {
        match token {
            Token::Open => depth += 1,
            Token::Close => depth = depth.saturating_sub(1),
            Token::Text(key) => {
                if let Some(Token::Text(_)) = tokens.peek() {
                    let Some(Token::Text(value)) = tokens.next() else {
                        unreachable!("peeked a text token");
                    };
                    pairs.push((depth, key, value));
                }
            }
        }
    }
    pairs
}

/// One value of a registry key, through Windows' own `reg.exe`; `None` when the key or value is missing.
#[cfg(windows)]
fn registry_value(key: &str, value: &str) -> Option<String> {
    registry_values(&registry_query(&[key, "/v", value])?, value)
        .into_iter()
        .next()
}

/// What `reg query` printed for `arguments`; `None` when it failed (a missing key).
#[cfg(windows)]
fn registry_query(arguments: &[&str]) -> Option<String> {
    // By its full path: a `reg.exe` in the working directory must not answer.
    let system = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
    let mut command = std::process::Command::new(Path::new(&system).join(r"System32\reg.exe"));
    {
        // No console window flashing up from a windowed program (a Windows release of
        // SJK or the launcher has no console of its own).
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command
        .arg("query")
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The data of each string value named `name` in `reg query`'s output, whose value
/// lines read `    <name>    REG_SZ    <data>`.
#[cfg_attr(not(windows), allow(dead_code))]
fn registry_values(output: &str, name: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| {
            let (value_name, rest) = line.trim_start().split_once("    ")?;
            if !value_name.eq_ignore_ascii_case(name) {
                return None;
            }
            let rest = rest.trim_start();
            let data = rest
                .strip_prefix("REG_SZ")
                .or_else(|| rest.strip_prefix("REG_EXPAND_SZ"))?;
            let data = data.trim();
            (!data.is_empty()).then(|| data.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBRARY_FOLDERS: &str = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"228980"		"420600418"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
			"6020"		"1427830016"
		}
	}
}
"#;

    #[test]
    fn library_folders_list_every_library_path() {
        assert_eq!(
            library_paths(LIBRARY_FOLDERS),
            [r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );
    }

    #[test]
    fn the_older_library_folders_file_lists_folders_under_numbered_keys() {
        let text = r#""LibraryFolders"
{
	// Steam before 2021.
	"TimeNextStatsReport"		"1601234567"
	"ContentStatsID"		"-123"
	"1"		"E:\\Games\\Steam"
}"#;
        assert_eq!(library_paths(text), [r"E:\Games\Steam"]);
    }

    #[test]
    fn a_manifest_names_its_install_folder() {
        let manifest = r#""AppState"
{
	"appid"		"6020"
	"name"		"STAR WARS™ Jedi Knight: Jedi Academy™"
	"installdir"		"Jedi Academy"
	"UserConfig"
	{
		"installdir"		"elsewhere"
	}
}"#;
        assert_eq!(install_dir(manifest).as_deref(), Some("Jedi Academy"));
        assert!(!is_plain_folder_name(".."));
        assert!(!is_plain_folder_name(r"..\..\Windows"));
        assert!(is_plain_folder_name("Jedi Academy"));
    }

    #[test]
    fn registry_output_gives_each_string_value() {
        let output = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamExe    REG_SZ    c:/program files (x86)/steam/steam.exe\r\n    SteamPath    REG_SZ    c:/program files (x86)/steam\r\n\r\n";
        assert_eq!(
            registry_values(output, "SteamPath"),
            ["c:/program files (x86)/steam"]
        );
        let gog = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\1\r\n    path    REG_SZ    D:\\GOG Games\\Star Wars Jedi Knight - Jedi Academy\r\n\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\GOG.com\\Games\\2\r\n    PATH    REG_SZ    C:\\GOG Games\\Other\r\n\r\nEnd of search: 2 match(es) found.\r\n";
        assert_eq!(
            registry_values(gog, "path"),
            [
                r"D:\GOG Games\Star Wars Jedi Knight - Jedi Academy",
                r"C:\GOG Games\Other"
            ]
        );
    }

    #[test]
    fn a_game_in_a_second_library_is_found_through_its_manifest() {
        let scratch = tempfile::tempdir().expect("scratch folder");
        let steam = scratch.path().join("Steam");
        let library = scratch.path().join("SteamLibrary");
        std::fs::create_dir_all(steam.join("steamapps")).expect("steam");
        std::fs::create_dir_all(library.join("steamapps/common/JA Renamed/GameData"))
            .expect("library");
        let vdf = format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
            steam.display().to_string().replace('\\', "\\\\"),
            library.display().to_string().replace('\\', "\\\\"),
        );
        std::fs::write(steam.join("steamapps/libraryfolders.vdf"), vdf).expect("vdf");
        std::fs::write(
            library.join("steamapps/appmanifest_6020.acf"),
            "\"AppState\"\n{\n\t\"appid\"\t\t\"6020\"\n\t\"installdir\"\t\t\"JA Renamed\"\n}\n",
        )
        .expect("manifest");

        let libraries = steam_libraries(std::slice::from_ref(&steam));
        assert_eq!(libraries, [steam.clone(), library.clone()]);
        let installs = steam_installs(&library);
        assert_eq!(
            installs.first(),
            Some(&library.join("steamapps/common/JA Renamed"))
        );
        assert!(installs.contains(&library.join("steamapps/common/Jedi Academy")));
    }
}
