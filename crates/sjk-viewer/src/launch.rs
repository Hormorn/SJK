//! Command-line compatibility and native no-argument launch resolution.

use std::env;
use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

/// Fully resolved startup request for the viewer application.
pub(crate) struct LaunchPlan {
    pub(crate) game_data: PathBuf,
    pub(crate) map_path: Option<String>,
    pub(crate) player_directory: Option<String>,
    pub(crate) connect_address: Option<String>,
    pub(crate) open_main_menu: bool,
}

/// Read only the installation hint before selecting storage. Constructing a
/// temporary console here would autosave the old user profile when dropped.
pub(crate) fn saved_game_data(config: &Path) -> Option<PathBuf> {
    std::fs::read_to_string(config)
        .ok()?
        .lines()
        .filter_map(|line| {
            let tokens = sjk_shell::tokenize(line.trim()).ok()?;
            match tokens.as_slice() {
                [command, name, value]
                    if command.eq_ignore_ascii_case("seta")
                        && name.eq_ignore_ascii_case("fs_gameData") =>
                {
                    Some(value.trim().to_owned())
                }
                _ => None,
            }
        })
        .last()
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Preserve the historical positional developer syntax while making an empty
/// argument list the normal native-client launch path.
pub(crate) fn resolve(
    arguments: impl Iterator<Item = OsString>,
    configured_game_data: Option<PathBuf>,
) -> Result<LaunchPlan, LaunchError> {
    let arguments = arguments.collect::<Vec<_>>();
    // Anywhere, not only first: on Linux Steam runs the game through Proton, so
    // `%command%` puts Proton's own program and verb before `jamp.exe`.
    if let Some(index) = arguments.iter().position(is_store_launched_program) {
        return store_launch(
            Path::new(&arguments[index]),
            &arguments[index + 1..],
            configured_game_data,
        );
    }
    if arguments.is_empty() {
        return Ok(LaunchPlan {
            game_data: find_game_data(configured_game_data.as_deref())?,
            map_path: None,
            player_directory: None,
            connect_address: None,
            open_main_menu: true,
        });
    }

    let (game_data, trailing) = if arguments[0] == "--connect" {
        (
            find_game_data(configured_game_data.as_deref())?,
            &arguments[..],
        )
    } else {
        let game_data = PathBuf::from(&arguments[0]);
        if !is_game_data(&game_data) {
            return Err(LaunchError::InvalidGameData(game_data));
        }
        (game_data, &arguments[1..])
    };
    let trailing = trailing
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut positionals = Vec::new();
    let mut connect_address = None;
    let mut index = 0;
    while index < trailing.len() {
        if trailing[index] == "--connect" {
            index += 1;
            connect_address = Some(
                trailing
                    .get(index)
                    .ok_or(LaunchError::MissingConnectAddress)?
                    .clone(),
            );
        } else {
            positionals.push(trailing[index].clone());
        }
        index += 1;
    }
    Ok(LaunchPlan {
        game_data,
        map_path: positionals.first().cloned(),
        player_directory: positionals.get(1).cloned(),
        connect_address,
        open_main_menu: false,
    })
}

/// The game's own programs, which a store passes to SJK when the player's launch options
/// read `"<path>\sjk.exe" %command%`: the store then starts SJK with the program it
/// would have run among the arguments (first on Windows, after Proton's on Linux), and
/// SJK opens in its place. The Mac edition's program is not listed: its name is not
/// known yet.
///
/// - `jamp.exe`: Steam's "Play Multiplayer" for app 6020 (`GameData\jamp.exe`).
/// - `jasp.exe`: Steam's "Play Single Player" for app 6020 (`GameData\jasp.exe`); SJK
///   is multiplayer only, so it opens its main menu for that one too.
const STORE_LAUNCHED_PROGRAMS: &[&str] = &["jamp.exe", "jasp.exe"];

fn is_store_launched_program(argument: &OsString) -> bool {
    Path::new(argument).file_name().is_some_and(|name| {
        STORE_LAUNCHED_PROGRAMS
            .iter()
            .any(|program| name.eq_ignore_ascii_case(program))
    })
}

/// SJK started by a store in place of `game`: the game's folder is the GameData when it
/// holds the retail files (else discovery runs as usual), and of the arguments after it
/// only a server to join (`--connect` or the game's `+connect`) is kept, as the rest
/// were written for the game's own program.
fn store_launch(
    game: &Path,
    rest: &[OsString],
    configured: Option<PathBuf>,
) -> Result<LaunchPlan, LaunchError> {
    let game_data = match game.parent().filter(|folder| is_game_data(folder)) {
        Some(folder) => folder.to_owned(),
        None => find_game_data(configured.as_deref())?,
    };
    let rest = rest
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let connect_address = rest
        .windows(2)
        .rev()
        .find(|pair| pair[0] == "--connect" || pair[0].eq_ignore_ascii_case("+connect"))
        .map(|pair| pair[1].clone());
    Ok(LaunchPlan {
        game_data,
        map_path: None,
        player_directory: None,
        open_main_menu: connect_address.is_none(),
        connect_address,
    })
}

fn find_game_data(configured: Option<&Path>) -> Result<PathBuf, LaunchError> {
    let current = env::current_dir().ok();
    let executable = env::current_exe().ok();
    let home = env::var_os("HOME").map(PathBuf::from);
    let environment = env::var_os("JKA_GAME_DATA")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    find_game_data_in(
        configured,
        environment.as_deref(),
        executable
            .as_deref()
            .and_then(crate::platform::install_folder),
        current.as_deref(),
        home.as_deref(),
    )
    // The stores last: asking the registry starts `reg.exe`, which a drop-in
    // installation never needs.
    .or_else(|| {
        crate::game_stores::game_data_candidates(home.as_deref())
            .into_iter()
            .find(|candidate| is_game_data(candidate))
    })
    .ok_or(LaunchError::GameDataNotFound)
}

// Explicit overrides win; a drop-in installation must not be redirected by a
// saved path from another installation. Never change cwd: other relative paths
// (for example a command-line demo path) still belong to the caller. The stores'
// installs (game_stores.rs) are asked after these.
fn find_game_data_in(
    configured: Option<&Path>,
    environment: Option<&Path>,
    executable_directory: Option<&Path>,
    current: Option<&Path>,
    home: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates = Vec::with_capacity(8);
    candidates.extend(environment.map(Path::to_owned));
    if let Some(directory) = executable_directory {
        candidates.push(directory.to_owned());
        candidates.push(directory.join("GameData"));
    }
    candidates.extend(configured.map(Path::to_owned));
    if let Some(current) = current {
        candidates.push(current.to_owned());
        candidates.push(current.join("GameData"));
        candidates.push(current.join("Star Wars Jedi Knight - Jedi Academy/GameData"));
    }
    if let Some(home) = home {
        candidates.push(home.join("Games/Jedi Academy/GameData"));
    }
    candidates
        .into_iter()
        .find(|candidate| is_game_data(candidate))
}

fn is_game_data(path: &Path) -> bool {
    path.join("base/assets0.pk3").is_file() && path.join("base/assets3.pk3").is_file()
}

#[derive(Debug)]
pub(crate) enum LaunchError {
    GameDataNotFound,
    InvalidGameData(PathBuf),
    MissingConnectAddress,
}

impl Display for LaunchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GameDataNotFound => formatter.write_str(concat!(
                "Jedi Academy GameData was not found, neither beside SJK nor in a Steam ",
                "library or GOG's games. Put SJK in the game's GameData ",
                "folder beside base/ (containing assets0.pk3 through assets3.pk3), ",
                "then launch it again. For a separate installation, set JKA_GAME_DATA, ",
                "set fs_gameData in the SJK config, or pass GameData as the first argument."
            )),
            Self::InvalidGameData(path) => write!(
                formatter,
                concat!(
                    "{} is not a Jedi Academy GameData directory ",
                    "(base/assets0.pk3 and assets3.pk3 are required)"
                ),
                path.display()
            ),
            Self::MissingConnectAddress => {
                formatter.write_str("--connect requires server-host:port")
            }
        }
    }
}

impl std::error::Error for LaunchError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GameData with the two retail archives discovery looks for (empty files).
    fn game_data() -> tempfile::TempDir {
        let folder = tempfile::tempdir().expect("scratch folder");
        std::fs::create_dir(folder.path().join("base")).expect("base");
        for archive in ["assets0.pk3", "assets3.pk3"] {
            std::fs::write(folder.path().join("base").join(archive), b"").expect("archive");
        }
        folder
    }

    #[test]
    fn steam_command_opens_the_menu_on_the_games_own_folder() {
        let folder = game_data();
        let jamp = folder.path().join("JAMP.EXE");
        let plan = resolve([jamp.into_os_string()].into_iter(), None).expect("plan");
        assert_eq!(plan.game_data, folder.path());
        assert!(plan.open_main_menu);
        assert_eq!(plan.map_path, None);
        assert_eq!(plan.connect_address, None);
    }

    #[test]
    fn steam_command_keeps_only_a_server_to_join() {
        let folder = game_data();
        let arguments = [
            folder.path().join("jamp.exe").into_os_string(),
            "+set".into(),
            "fs_game".into(),
            "japlus".into(),
            "+connect".into(),
            "203.0.113.5:29070".into(),
        ];
        let plan = resolve(arguments.into_iter(), None).expect("plan");
        assert_eq!(plan.game_data, folder.path());
        assert!(!plan.open_main_menu);
        assert_eq!(plan.map_path, None);
        assert_eq!(plan.connect_address.as_deref(), Some("203.0.113.5:29070"));
    }

    #[test]
    fn steam_command_through_proton_finds_the_game_after_protons_program() {
        let folder = game_data();
        let arguments = [
            "/home/sol/.steam/steam/steamapps/common/Proton 9.0/proton".into(),
            "waitforexitandrun".into(),
            folder.path().join("jamp.exe").into_os_string(),
        ];
        let plan = resolve(arguments.into_iter(), None).expect("plan");
        assert_eq!(plan.game_data, folder.path());
        assert!(plan.open_main_menu);
    }

    #[test]
    fn another_program_is_still_a_game_data_argument() {
        assert!(is_store_launched_program(&OsString::from(
            "Steam/steamapps/common/Jedi Academy/GameData/jasp.exe"
        )));
        assert!(!is_store_launched_program(&OsString::from("jamp.exe.bak")));
        assert!(!is_store_launched_program(&OsString::from(
            "Games/GameData"
        )));
    }
}
