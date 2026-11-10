//! Directory discovery and strict or diagnostic PK3 mounting policies.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{MountId, VfsError, VirtualFileSystem};

impl VirtualFileSystem {
    /// Mounts every `*.pk3` directly inside `directory` in Quake 3 search
    /// order: names compared case-insensitively, later names taking
    /// precedence over earlier ones, `dl_*` downloads over everything else
    /// (`FS_AddGameDirectory` / `paksort`). Returns the mounts in that order.
    ///
    /// Stops at the first failure; earlier successful mounts remain mounted.
    pub fn mount_pk3_directory(
        &mut self,
        directory: impl AsRef<Path>,
    ) -> Result<Vec<MountId>, VfsError> {
        pk3_archives(directory.as_ref())?
            .iter()
            .map(|archive| self.mount_pk3(archive))
            .collect()
    }

    /// Mounts readable PK3 catalogues in the same order as
    /// [`Self::mount_pk3_directory`], reporting and skipping failed mounts.
    ///
    /// This matches `FS_AddGameDirectory` continuing when `FS_LoadZipFile`
    /// fails: one unusable add-on must not prevent mounting unrelated content.
    /// Directory-listing failures still return an error. Assets are validated
    /// on demand, and their read errors are never suppressed by this policy.
    pub fn mount_pk3_directory_with_warnings(
        &mut self,
        directory: impl AsRef<Path>,
        mut warning: impl FnMut(&Path, &VfsError),
    ) -> Result<Vec<MountId>, VfsError> {
        let mut mounts = Vec::new();
        for archive in pk3_archives(directory.as_ref())? {
            match self.mount_pk3(&archive) {
                Ok(id) => mounts.push(id),
                Err(error) => warning(&archive, &error),
            }
        }
        Ok(mounts)
    }

    /// Mounts the PK3s of `game_data`'s `base` and of its [assets folders](assets_folders)
    /// as one folder's, reporting and skipping failed mounts as
    /// [`Self::mount_pk3_directory_with_warnings`] does.
    pub fn mount_base_pk3s_with_warnings(
        &mut self,
        game_data: impl AsRef<Path>,
        mut warning: impl FnMut(&Path, &VfsError),
    ) -> Result<Vec<MountId>, VfsError> {
        let game_data = game_data.as_ref();
        let mut folders = vec![game_data.join("base")];
        folders.extend(assets_folders(game_data));
        let mut mounts = Vec::new();
        for archive in pk3_search_order_merged(&folders)? {
            match self.mount_pk3(&archive) {
                Ok(id) => mounts.push(id),
                Err(error) => warning(&archive, &error),
            }
        }
        Ok(mounts)
    }
}

/// The folder beside `base` in `GameData` where a player can tidy PK3s away.
pub const ASSETS_DIRECTORY: &str = "assets";

/// `game_data`'s [`ASSETS_DIRECTORY`] and every folder below it, sorted: empty when
/// there is none. Their PK3s count as `base`'s, by way of
/// [`pk3_search_order_merged`], so moving a pack in or out never changes what it
/// replaces. A folder that cannot be listed is left out, and links are not followed,
/// so a link cannot make the walk go round for ever.
pub fn assets_folders(game_data: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let mut waiting = vec![game_data.as_ref().join(ASSETS_DIRECTORY)];
    while let Some(folder) = waiting.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        waiting.extend(
            entries
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.path()),
        );
        folders.push(folder);
    }
    folders.sort();
    folders
}

/// The `*.pk3` files directly inside `directory`, lowest priority first: the
/// order [`VirtualFileSystem::mount_pk3_directory`] mounts them in. Tools that
/// must leave out an archive (their own earlier output) mount from this list.
pub fn pk3_search_order(directory: impl AsRef<Path>) -> Result<Vec<PathBuf>, VfsError> {
    pk3_archives(directory.as_ref())
}

/// The `*.pk3` files of several directories in the order one folder holding them
/// all would mount them ([`pk3_search_order`]): content kept in a folder of its own
/// that stands in for files installed beside the game's. Equal names keep the
/// directories' order, so the later directory's copy wins.
pub fn pk3_search_order_merged<P: AsRef<Path>>(
    directories: &[P],
) -> Result<Vec<PathBuf>, VfsError> {
    let mut archives = Vec::new();
    for directory in directories {
        archives.extend(pk3_archives(directory.as_ref())?);
    }
    // A stable sort: equal names stay in directory order.
    archives.sort_by_cached_key(|archive| pak_order(archive));
    Ok(archives)
}

/// Direct child PK3s (case-insensitive extension), in legacy search order.
fn pk3_archives(directory: &Path) -> Result<Vec<PathBuf>, VfsError> {
    let io_error = |source| VfsError::Io {
        operation: "list PK3s",
        path: directory.to_owned(),
        source,
    };
    let mut archives = Vec::new();
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        let is_pk3 = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pk3"));
        if is_pk3 && path.is_file() {
            archives.push(path);
        }
    }
    archives.sort_by(|a, b| pak_order(a).cmp(&pak_order(b)));
    Ok(archives)
}

/// `paksort`: downloads sort last; otherwise case-insensitive `FS_PathCmp`.
fn pak_order(archive: &Path) -> (bool, Vec<u8>) {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .into_bytes();
    (name.starts_with(b"dl_"), name)
}
