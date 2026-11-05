//! Platform composition for per-user SJK state.

use std::env;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[path = "platform/storage.rs"]
mod storage;

#[cfg(target_os = "linux")]
mod main_stack;

/// Match the link-time main-thread stack allowance on Linux, before client startup.
pub(crate) fn configure_main_stack() {
    #[cfg(target_os = "linux")]
    main_stack::configure();
}

static CONFIG_FILE: OnceLock<PathBuf> = OnceLock::new();

/// Give mounted PK3 archives the process headroom the host already permits.
///
/// Failure is non-fatal: retaining the inherited limit preserves the old startup
/// behaviour, while VFS lifetime diagnostics identify any ownership leak.
pub(crate) fn raise_open_file_limit() {
    #[cfg(unix)]
    {
        let (old_soft, hard) = match rlimit::Resource::NOFILE.get() {
            Ok(limits) => limits,
            Err(error) => {
                crate::log::progress(format_args!(
                    "warning: could not read open-file limit: {error}"
                ));
                return;
            }
        };
        match rlimit::increase_nofile_limit(u64::MAX) {
            Ok(new_soft) => crate::log::progress(format_args!(
                "open-file limit: soft {old_soft} -> {new_soft}, hard {}",
                Limit(hard)
            )),
            Err(error) => crate::log::progress(format_args!(
                "warning: could not raise open-file limit from soft {old_soft} (hard {}): {error}",
                Limit(hard)
            )),
        }
    }
}

#[cfg(unix)]
struct Limit(u64);

#[cfg(unix)]
impl std::fmt::Display for Limit {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == rlimit::INFINITY {
            output.write_str("unlimited")
        } else {
            self.0.fmt(output)
        }
    }
}

/// Select writable client storage once, before constructing any UI or workers.
pub(crate) fn initialize_storage(game_data: &std::path::Path) -> Result<(), Error> {
    if CONFIG_FILE.get().is_some() {
        return Err(Error::other("client storage already initialized"));
    }
    let per_user = per_user_config_file().ok();
    let selection = storage::select(game_data, per_user.as_deref())?;
    if let Some(reason) = selection.fallback_reason {
        crate::log::progress(format_args!(
            "GameData/SJK is not writable ({reason}); using user storage"
        ));
    }
    crate::log::progress(format_args!("client files: {}", selection.config.display()));
    CONFIG_FILE
        .set(selection.config)
        .map_err(|_| Error::other("client storage already initialized"))
}

/// Ask the native interface-listing utility; no external probe packets or DNS tricks.
pub(crate) fn interface_addresses() -> Result<String, Error> {
    let (program, args): (&str, &[&str]) = if cfg!(target_os = "windows") {
        ("ipconfig", &[])
    } else if cfg!(target_os = "linux") {
        ("ip", &["-brief", "address", "show"])
    } else {
        ("ifconfig", &[])
    };
    let output = hidden_command(program).args(args).output()?;
    if !output.status.success() {
        return Err(Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// A helper program started without a window of its own. A Windows release client
/// has no console (`windows_subsystem` in main.rs), so a console program it starts
/// (the server, PowerShell, `cmd`) would otherwise open one.
pub(crate) fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut command = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// The folder a program is installed in: the one holding it, or the one holding
/// its macOS app bundle (`SJK.app/Contents/MacOS/sjk-client` is installed in the
/// folder beside `SJK.app`, with `base/` and `sjk-server`).
pub(crate) fn install_folder(program: &Path) -> Option<&Path> {
    let directory = program.parent()?;
    if directory.ends_with("Contents/MacOS") {
        let bundle = directory.parent()?.parent()?;
        if bundle
            .extension()
            .is_some_and(|extension| extension == "app")
        {
            return bundle.parent();
        }
    }
    Some(directory)
}

/// The selected client configuration; all file consumers share this location.
pub(crate) fn user_config_file() -> Result<PathBuf, Error> {
    CONFIG_FILE
        .get()
        .cloned()
        .map_or_else(per_user_config_file, Ok)
}

/// The per-user configuration, used when `GameData/SJK` is not writable and read
/// for a saved `fs_gameData` before the game data is known.
pub(crate) fn per_user_config_file() -> Result<PathBuf, Error> {
    let root = if cfg!(target_os = "windows") {
        env_path("APPDATA")?
    } else if cfg!(target_os = "macos") {
        env_path("HOME")?.join("Library/Application Support")
    } else if let Some(path) = env::var_os("XDG_CONFIG_HOME").filter(|path| !path.is_empty()) {
        PathBuf::from(path)
    } else {
        env_path("HOME")?.join(".config")
    };
    Ok(root.join("SJK/config.cfg"))
}

fn env_path(name: &str) -> Result<PathBuf, Error> {
    env::var_os(name)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("{name} is not set")))
}

#[cfg(test)]
mod tests {
    use super::install_folder;
    use std::path::Path;

    #[test]
    fn a_bundled_program_is_installed_beside_its_bundle() {
        assert_eq!(
            install_folder(Path::new("/g/GameData/SJK.app/Contents/MacOS/sjk-client")),
            Some(Path::new("/g/GameData"))
        );
        assert_eq!(
            install_folder(Path::new("/g/GameData/sjk")),
            Some(Path::new("/g/GameData"))
        );
        // Only an .app folder is a bundle.
        assert_eq!(
            install_folder(Path::new("/g/Other/Contents/MacOS/sjk")),
            Some(Path::new("/g/Other/Contents/MacOS"))
        );
    }
}
