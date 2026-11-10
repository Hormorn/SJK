//! What is on disk: SJK's JoF folder (`<root>/base`), the game's `base` and the
//! `assets` folder beside it, measured against the manifest.
//!
//! Hashing gigabytes on every look would be slow, so each file's MD5 is kept in
//! `<root>/sjk-jof-content.json` with the size and modification time it was
//! measured at; a file whose size or time changed is hashed again.

use crate::manifest::{Asset, Manifest, Target};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

/// The record's file name in the root.
pub const RECORD: &str = "sjk-jof-content.json";
/// Where the files go under the root, as the game's own `base`.
pub const CONTENT: &str = "base";
/// The tidy folder beside the game's `base`: its PK3s load as `base`'s, at any depth.
const ASSETS: &str = "assets";

/// Where one manifest file stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetState {
    /// In SJK's JoF folder, matching the manifest.
    Installed,
    /// In the game's `base` or `assets` with the manifest's MD5 (JoF's launcher
    /// put it there, or the player tidied it): used as it is, never fetched.
    InGameFolder,
    /// In SJK's JoF folder, but not the manifest's file (an older version).
    Outdated,
    Missing,
    /// For another client's folder (`EternalJK`): listed, never fetched.
    NotForSjk,
}

impl AssetState {
    /// Whether the game can use the file now.
    pub fn present(self) -> bool {
        matches!(self, Self::Installed | Self::InGameFolder)
    }
}

/// The manifest with every file's state, as one look at the disk found it.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub manifest: Manifest,
    /// `states[group][asset]`, in the manifest's order.
    pub states: Vec<Vec<AssetState>>,
    /// PK3s SJK downloaded into its JoF folder that are no longer wanted
    /// there: not in the manifest, or kept in the game folder already. Name and
    /// size.
    pub strays: Vec<(String, u64)>,
    /// Lower-case names of the PK3s in SJK's JoF folder that SJK downloaded:
    /// the only files a sync deletes. A file put there by hand stays.
    pub downloaded: BTreeSet<String>,
    /// The group each downloaded PK3 was fetched for, by lower-case name, when
    /// the record says (files fetched before SJK noted it have none): an
    /// automatic update removes only the files of its own groups.
    pub origins: BTreeMap<String, u32>,
}

impl Snapshot {
    pub fn state(&self, group: usize, asset: usize) -> AssetState {
        self.states
            .get(group)
            .and_then(|states| states.get(asset))
            .copied()
            .unwrap_or(AssetState::Missing)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
struct Measured {
    size: u64,
    /// Nanoseconds since 1970 of the file's modification time.
    modified: u64,
    md5: String,
    /// SJK wrote it (a download); only such files are ever deleted.
    #[serde(default)]
    downloaded: bool,
    /// The manifest group it was downloaded for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<u32>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Record {
    /// Measured files by full path.
    #[serde(default)]
    files: BTreeMap<String, Measured>,
}

/// SJK's JoF folder and the game's `base` to look in.
pub struct Store {
    root: PathBuf,
    game_base: Option<PathBuf>,
    record: Record,
}

impl Store {
    /// Open the folder at `root` (made when the first file arrives). The game's
    /// `base` is only read, never written.
    pub fn open(root: impl Into<PathBuf>, game_base: Option<PathBuf>) -> Self {
        let root = root.into();
        let record = std::fs::read(root.join(RECORD))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            root,
            game_base,
            record,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/base`, where the files go.
    pub fn content(&self) -> PathBuf {
        self.root.join(CONTENT)
    }

    /// Whether `<root>/base` is the game's own `base` (a `jof.contentFolder`
    /// set to the game's folder): SJK then refuses to work there.
    pub fn is_game_folder(&self) -> bool {
        self.game_base
            .as_deref()
            .is_some_and(|game| same_folder(&self.content(), game))
    }

    /// Write the record back; a failure only costs hashing again next time.
    pub fn save(&self) {
        if self.record.files.is_empty() && !self.root.exists() {
            return;
        }
        if std::fs::create_dir_all(&self.root).is_ok()
            && let Ok(text) = serde_json::to_vec_pretty(&self.record)
        {
            let temporary = self.root.join(format!("{RECORD}.tmp"));
            if std::fs::write(&temporary, text).is_ok() {
                let _ = std::fs::rename(&temporary, self.root.join(RECORD));
            }
        }
    }

    /// Remember a file SJK just downloaded and checked, for the manifest
    /// `group` it was fetched for: it is not hashed again, and a later sync may
    /// delete it.
    pub fn remember(&mut self, path: &Path, md5: &[u8; 16], group: Option<u32>) {
        if let Some((size, modified)) = stamp(path) {
            self.record.files.insert(
                key(path),
                Measured {
                    size,
                    modified,
                    md5: crate::md5_hex(md5),
                    downloaded: true,
                    group,
                },
            );
        }
    }

    pub fn forget(&mut self, path: &Path) {
        self.record.files.remove(&key(path));
    }

    /// The file's MD5, from the record when its size and time are unchanged.
    /// `on_bytes` hears the bytes hashed. None when it cannot be read or
    /// `cancel` was raised.
    pub fn md5(
        &mut self,
        path: &Path,
        cancel: &AtomicBool,
        on_bytes: &mut dyn FnMut(u64),
    ) -> Option<[u8; 16]> {
        let (size, modified) = stamp(path)?;
        if let Some(known) = self.record.files.get(&key(path))
            && known.size == size
            && known.modified == modified
        {
            return crate::parse_md5(&known.md5);
        }
        let digest = hash_file(path, cancel, on_bytes)?;
        let (downloaded, group) = self
            .record
            .files
            .get(&key(path))
            .map_or((false, None), |known| (known.downloaded, known.group));
        self.record.files.insert(
            key(path),
            Measured {
                size,
                modified,
                md5: crate::md5_hex(&digest),
                downloaded,
                group,
            },
        );
        Some(digest)
    }

    /// Measure every manifest file. `on_progress(done, total)` counts the bytes
    /// that need hashing (files already in the record cost nothing).
    pub fn scan(
        &mut self,
        manifest: Manifest,
        cancel: &AtomicBool,
        on_progress: &mut dyn FnMut(u64, u64),
    ) -> Snapshot {
        let ours = pk3s(&self.content());
        let game = self
            .game_base
            .as_deref()
            .map(|base| game_pk3s(base, &self.content()))
            .unwrap_or_default();
        let candidates = |asset: &Asset| {
            let name = asset.name.to_ascii_lowercase();
            [ours.get(&name), game.get(&name)]
        };
        let total: u64 = manifest
            .assets()
            .filter(|(_, asset)| asset.target == Target::Base)
            .flat_map(|(_, asset)| candidates(asset).map(|path| (asset.size, path)))
            .filter_map(|(wanted, path)| Some((wanted, path?)))
            .filter(|(_, path)| !self.cached(path))
            .filter_map(|(wanted, path)| {
                stamp(path)
                    .map(|(size, _)| size)
                    .filter(|&size| size == wanted)
            })
            .sum();
        let mut done = 0;
        on_progress(done, total);
        let mut states = Vec::with_capacity(manifest.groups.len());
        let mut wanted_here = std::collections::HashSet::new();
        for group in &manifest.groups {
            let mut group_states = Vec::with_capacity(group.assets.len());
            for asset in &group.assets {
                if asset.target != Target::Base {
                    group_states.push(AssetState::NotForSjk);
                    continue;
                }
                let [our, theirs] = candidates(asset);
                let mut matches = |path: Option<&PathBuf>, store: &mut Self| {
                    let path = path?;
                    if stamp(path)?.0 != asset.size {
                        return Some(false);
                    }
                    let cached = store.cached(path);
                    let digest = store.md5(path, cancel, &mut |bytes| {
                        if !cached {
                            on_progress(done + bytes, total);
                        }
                    });
                    if !cached {
                        done += stamp(path).map_or(0, |(size, _)| size);
                    }
                    Some(digest == Some(asset.md5))
                };
                let state = match (matches(our, self), matches(theirs, self)) {
                    (Some(true), _) => AssetState::Installed,
                    (_, Some(true)) => AssetState::InGameFolder,
                    (Some(false), _) => AssetState::Outdated,
                    (None, _) => AssetState::Missing,
                };
                if matches!(state, AssetState::Installed | AssetState::Outdated) {
                    wanted_here.insert(asset.name.to_ascii_lowercase());
                }
                group_states.push(state);
            }
            states.push(group_states);
            if cancel.load(Ordering::Relaxed) {
                break;
            }
        }
        let downloaded: BTreeSet<String> = ours
            .iter()
            .filter(|(_, path)| {
                self.record
                    .files
                    .get(&key(path))
                    .is_some_and(|known| known.downloaded)
            })
            .map(|(name, _)| name.clone())
            .collect();
        let origins: BTreeMap<String, u32> = ours
            .iter()
            .filter_map(|(name, path)| {
                let known = self.record.files.get(&key(path))?;
                Some((name.clone(), known.group.filter(|_| known.downloaded)?))
            })
            .collect();
        let mut strays: Vec<(String, u64)> = ours
            .iter()
            .filter(|(name, _)| !wanted_here.contains(*name) && downloaded.contains(*name))
            .filter_map(|(_, path)| {
                let name = path.file_name()?.to_string_lossy().into_owned();
                Some((name, stamp(path).map_or(0, |(size, _)| size)))
            })
            .collect();
        strays.sort();
        self.save();
        Snapshot {
            manifest,
            states,
            strays,
            downloaded,
            origins,
        }
    }

    fn cached(&self, path: &Path) -> bool {
        let Some((size, modified)) = stamp(path) else {
            return true;
        };
        self.record
            .files
            .get(&key(path))
            .is_some_and(|known| known.size == size && known.modified == modified)
    }
}

/// The PK3s directly in `directory` by lower-case name.
fn pk3s(directory: &Path) -> HashMap<String, PathBuf> {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name.ends_with(".pk3").then(|| (name, entry.path()))
        })
        .collect()
}

/// The PK3s the game has by lower-case name: its `base`, then the `assets` folder
/// beside it at any depth, leaving out `content` (the store's own folder). Links
/// are not followed.
fn game_pk3s(base: &Path, content: &Path) -> HashMap<String, PathBuf> {
    let mut found = pk3s(base);
    let mut waiting: Vec<PathBuf> = base
        .parent()
        .map(|game| game.join(ASSETS))
        .into_iter()
        .collect();
    while let Some(directory) = waiting.pop() {
        if same_folder(&directory, content) {
            continue;
        }
        for (name, path) in pk3s(&directory) {
            found.entry(name).or_insert(path);
        }
        waiting.extend(
            std::fs::read_dir(&directory)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
                .map(|entry| entry.path()),
        );
    }
    found
}

fn same_folder(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a
            .to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy()),
    }
}

/// The record's key for a path: `/` separators, and lower case on Windows,
/// whose file names ignore case, so one file has one key however it was spelt.
fn key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}

/// Size and modification time (nanoseconds since 1970).
fn stamp(path: &Path) -> Option<(u64, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos() as u64);
    Some((metadata.len(), modified))
}

/// MD5 of a file, read in 1 MiB pieces.
pub(crate) fn hash_file(
    path: &Path,
    cancel: &AtomicBool,
    on_bytes: &mut dyn FnMut(u64),
) -> Option<[u8; 16]> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Md5::new();
    let mut buffer = vec![0_u8; 1 << 20];
    let mut done = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        done += read as u64;
        on_bytes(done);
    }
    Some(hasher.finalize().into())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A manifest of three files with real digests: `a.pk3` ("aaa"), `b.pk3`
    /// ("bbbb") in group 1 (recommended), `c.pk3` ("cc") in group 2 (optional).
    pub(crate) fn manifest() -> Manifest {
        let file = |id: u32, name: &str, body: &str| {
            let digest: [u8; 16] = Md5::digest(body.as_bytes()).into();
            format!(
                r#"{{"id":{id},"name":"{name}","size":{},"hash":"{}","targetfolder":"Base","localfolder":"assets"}}"#,
                body.len(),
                crate::md5_hex(&digest)
            )
        };
        let json = format!(
            r#"{{"data":{{"assetgroups":[
                {{"id":1,"name":"One","type":"RECOMMENDED","assets":[{},{}]}},
                {{"id":2,"name":"Two","type":"OPTIONAL","assets":[{}]}}]}}}}"#,
            file(10, "a.pk3", "aaa"),
            file(11, "b.pk3", "bbbb"),
            file(20, "c.pk3", "cc"),
        );
        Manifest::parse(json.as_bytes()).unwrap()
    }

    #[test]
    fn files_are_found_in_either_folder_and_checked_by_md5() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("jof");
        let game = directory.path().join("game/base");
        std::fs::create_dir_all(root.join(CONTENT)).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("A.PK3"), "aaa").unwrap();
        std::fs::write(root.join("base/b.pk3"), "bbb").unwrap();
        std::fs::write(root.join("base/old.pk3"), "zz").unwrap();
        std::fs::write(root.join("base/mine.pk3"), "by hand").unwrap();
        let mut store = Store::open(&root, Some(game.clone()));
        store.remember(
            &root.join("base/old.pk3"),
            &Md5::digest(b"zz").into(),
            Some(7),
        );
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let snapshot = store.scan(manifest(), &cancel, &mut |done, total| {
            assert!(done <= total);
            seen = total;
        });
        assert_eq!(seen, 3, "only a.pk3 was hashed: b.pk3 has the wrong size");
        assert_eq!(snapshot.state(0, 0), AssetState::InGameFolder);
        assert_eq!(snapshot.state(0, 1), AssetState::Outdated);
        assert_eq!(snapshot.state(1, 0), AssetState::Missing);
        assert_eq!(
            snapshot.strays,
            [("old.pk3".to_owned(), 2)],
            "mine.pk3 was not downloaded by SJK, so it stays"
        );
        assert_eq!(
            snapshot.origins,
            [("old.pk3".to_owned(), 7)].into(),
            "the group it was downloaded for is kept"
        );

        assert!(!store.is_game_folder());
        assert!(Store::open(game.parent().unwrap(), Some(game.clone())).is_game_folder());

        std::fs::write(root.join("base/b.pk3"), "bbbb").unwrap();
        let mut store = Store::open(&root, Some(game));
        let mut hashed = u64::MAX;
        let snapshot = store.scan(manifest(), &cancel, &mut |_, total| hashed = total);
        assert_eq!(snapshot.state(0, 1), AssetState::Installed);
        assert_eq!(hashed, 4, "only the changed file was hashed again");
    }

    #[test]
    fn a_file_tidied_into_the_assets_folder_is_not_fetched_again() {
        let directory = tempfile::tempdir().unwrap();
        let game_data = directory.path();
        let deep = game_data.join("assets/jof/models");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::create_dir_all(game_data.join("base")).unwrap();
        std::fs::write(deep.join("a.pk3"), "aaa").unwrap();
        let root = game_data.join("SJK/jof");
        let mut store = Store::open(&root, Some(game_data.join("base")));
        let snapshot = store.scan(manifest(), &AtomicBool::new(false), &mut |_, _| {});
        assert_eq!(snapshot.state(0, 0), AssetState::InGameFolder);
        assert_eq!(snapshot.state(0, 1), AssetState::Missing);

        // A store that lives in the assets folder finds its own files as its own.
        let root = game_data.join("assets/jof");
        std::fs::create_dir_all(root.join(CONTENT)).unwrap();
        std::fs::rename(deep.join("a.pk3"), root.join("base/a.pk3")).unwrap();
        let mut store = Store::open(&root, Some(game_data.join("base")));
        let snapshot = store.scan(manifest(), &AtomicBool::new(false), &mut |_, _| {});
        assert_eq!(snapshot.state(0, 0), AssetState::Installed);
    }
}
