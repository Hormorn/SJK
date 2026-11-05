//! The Saber Lab (`docs/client.md`, "Saber Lab"): blade skins read from a folder of loose
//! files and reloaded as they are saved, to make and tune one without a pack, a restart
//! or a rebuild.
//!
//! `saberlab <folder>` (or [`ENV`] at start) mounts the folder over the hub's packs
//! ([`crate::sjk_packs::mount`]): its `skins/blades/<id>.bladeskin`, additions files,
//! images and sounds replace a pack's of the same names, and a file for [`ID`] is a skin
//! of its own, the [`BLADE`] the catalogue knows only while the lab is on. The folder is
//! looked at every [`SCAN`]; when a file in it changed, the packs are mounted again and
//! the renderer takes the new skins on its next frame, as when the hub sends a pack. A
//! file that does not read keeps its skin's last good look, and the Shaders tab's lead
//! says why ([`status`]). Starting the lab opens the Collection's Shaders tab on the lab
//! blade, on the lone saber the tab turns; the preview is this screen's only, never sent.

use crate::saber_skins::LoadedSkins;
use crate::unlockables::{Kind, Rarity, Unlockable};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

/// Console command name.
pub(crate) const COMMAND: &str = "saberlab";
/// Help text for completion and `cmdlist`.
pub(crate) const HELP: &str =
    "Blade skins from a folder, reloaded as you save them: saberlab <folder> | off";
/// Starts the lab on this folder at launch.
pub(crate) const ENV: &str = "SJK_SABER_LAB";
/// The lab's own blade: a skin file of this id is the lab blade.
pub(crate) const ID: &str = "saber_lab";
/// How often the folder is looked at.
const SCAN: Duration = Duration::from_millis(500);
/// Most files of the folder watched; past them a change goes unseen.
const MAX_FILES: usize = 512;

/// The lab blade, in the catalogue while the lab is on ([`blade`]), and wearable here
/// ([`owns`]).
pub(crate) static BLADE: Unlockable = Unlockable {
    id: ID,
    // No catalogue number: a lab blade is this PC's alone.
    number: 0,
    name: "Saber Lab blade",
    kind: Kind::BladeSkin,
    tier: Rarity::Common,
    chroma: false,
    droppable: false,
    edition: "",
    smith: "You",
    description: "The blade of your Saber Lab folder's saber_lab.bladeskin, drawn again each time you save it.",
    how_to_get: "Only in the Saber Lab, on this PC: saberlab <folder>.",
};

struct Lab {
    folder: PathBuf,
    stamp: u64,
    next_scan: Instant,
    /// When the packs were last mounted with the folder.
    reloaded: Instant,
    /// The first file of the folder that does not read, and why.
    problem: Option<String>,
    /// The skins of the folder's files, by id.
    ids: Vec<String>,
    /// The Collection should open on the lab blade.
    open: bool,
}

static LAB: Mutex<Option<Lab>> = Mutex::new(None);
static ACTIVE: AtomicBool = AtomicBool::new(false);

fn lock() -> std::sync::MutexGuard<'static, Option<Lab>> {
    LAB.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Whether the lab is on.
pub(crate) fn active() -> bool {
    ACTIVE.load(Ordering::Acquire)
}

/// Whether the player may wear skin `id` on this PC because the lab is on: the lab blade
/// and every skin the folder holds a file for. Only the blade drawn here follows it; the
/// look sent to the hub keeps the player's real unlocks (`looks_frame.rs`), so the hub
/// never hears of a lab skin and other players see the player's usual blade.
pub(crate) fn owns(id: &str) -> bool {
    if !active() {
        return false;
    }
    let id = id.trim();
    id.eq_ignore_ascii_case(ID)
        || lock()
            .as_ref()
            .is_some_and(|lab| lab.ids.iter().any(|known| known.eq_ignore_ascii_case(id)))
}

/// The skins the lab's tab offers: the lab blade first when the folder has its file, then
/// every other skin the folder holds a file for, by the catalogue's ids.
pub(crate) fn blades() -> Vec<&'static str> {
    let ids = lock()
        .as_ref()
        .map(|lab| lab.ids.clone())
        .unwrap_or_default();
    let mut blades: Vec<&'static str> = ids
        .iter()
        .filter_map(|id| crate::unlockables::blade_skin(id).map(|skin| skin.id))
        .collect();
    blades.sort_by_key(|id| (*id != ID, !id.starts_with(EXTRA_PREFIX)));
    blades
}

/// The lab blade, or another of the lab's (`saber_lab_<name>`, [`EXTRA_PREFIX`]), when
/// `id` names one and the lab is on ([`crate::unlockables::find`]).
pub(crate) fn blade(id: &str) -> Option<&'static Unlockable> {
    if !active() {
        return None;
    }
    let id = id.trim();
    if id.eq_ignore_ascii_case(ID) {
        return Some(&BLADE);
    }
    let name = id
        .get(..EXTRA_PREFIX.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(EXTRA_PREFIX))
        .map(|_| &id[EXTRA_PREFIX.len()..])
        .filter(|name| !name.is_empty())?;
    crate::blade_skin_file::valid_id(&id.to_ascii_lowercase()).then(|| extra_blade(id, name))?
}

/// Another lab blade's file names begin so: `saber_lab_<name>.bladeskin` is the lab
/// blade "Saber Lab: <name>", worn on this PC as the lab blade is.
pub(crate) const EXTRA_PREFIX: &str = "saber_lab_";

/// The lab blade of id `id` (named `name`), made once and kept: a catalogue entry is
/// `'static`, and a folder holds few of them.
fn extra_blade(id: &str, name: &str) -> Option<&'static Unlockable> {
    static EXTRA: Mutex<Vec<&'static Unlockable>> = Mutex::new(Vec::new());
    let mut extra = EXTRA
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(known) = extra.iter().find(|known| known.id.eq_ignore_ascii_case(id)) {
        return Some(known);
    }
    if extra.len() >= 32 {
        return None;
    }
    let blade: &'static Unlockable = Box::leak(Box::new(Unlockable {
        id: Box::leak(id.to_ascii_lowercase().into_boxed_str()),
        name: Box::leak(format!("Saber Lab: {}", name.replace('_', " ")).into_boxed_str()),
        description: "Another blade of your Saber Lab folder, drawn again each time you save it.",
        ..BLADE
    }));
    extra.push(blade);
    Some(blade)
}

/// Start the lab on `folder` (a lab already on moves to it); the folder must exist.
/// Returns the folder as it will be read.
pub(crate) fn start(folder: &Path) -> Result<PathBuf, String> {
    let folder = std::fs::canonicalize(folder)
        .map(plain)
        .map_err(|error| format!("{}: {error}", folder.display()))?;
    if !folder.is_dir() {
        return Err(format!("{} is not a folder", folder.display()));
    }
    let now = Instant::now();
    *lock() = Some(Lab {
        stamp: stamp(&folder),
        folder: folder.clone(),
        next_scan: now + SCAN,
        reloaded: now,
        problem: None,
        ids: Vec::new(),
        open: true,
    });
    ACTIVE.store(true, Ordering::Release);
    crate::sjk_packs::remount();
    Ok(folder)
}

/// Start the lab on [`ENV`]'s folder at launch, when it names one.
pub(crate) fn start_from_environment() {
    let Some(folder) = std::env::var_os(ENV).filter(|folder| !folder.is_empty()) else {
        return;
    };
    match start(Path::new(&folder)) {
        Ok(folder) => crate::log::progress(format_args!("saber lab: {}", folder.display())),
        Err(why) => crate::log::progress(format_args!("saber lab ({ENV}): {why}")),
    }
}

/// Stop the lab; the hub's packs alone are mounted again. Whether it was on.
pub(crate) fn stop() -> bool {
    let was = lock().take().is_some();
    ACTIVE.store(false, Ordering::Release);
    if was {
        crate::sjk_packs::remount();
    }
    was
}

/// The folder the lab reads, while it is on.
pub(crate) fn folder() -> Option<PathBuf> {
    lock().as_ref().map(|lab| lab.folder.clone())
}

/// Look at the folder when [`SCAN`] has passed (called each frame): a change mounts the
/// packs again.
pub(crate) fn poll(now: Instant) {
    if !active() {
        return;
    }
    let folder = {
        let mut lab = lock();
        let Some(lab) = lab.as_mut() else { return };
        if now < lab.next_scan {
            return;
        }
        lab.next_scan = now + SCAN;
        lab.folder.clone()
    };
    let stamp = stamp(&folder);
    let changed = lock()
        .as_mut()
        .is_some_and(|lab| std::mem::replace(&mut lab.stamp, stamp) != stamp);
    if changed {
        crate::sjk_packs::remount();
    }
}

/// The blade-skin editor wrote the folder's files and put their look in place itself
/// (`crate::blade_editor`): note them as read, so that the packs are not mounted again
/// for them.
pub(crate) fn wrote() {
    let Some(folder) = folder() else { return };
    let stamp = stamp(&folder);
    if let Some(lab) = lock().as_mut() {
        lab.stamp = stamp;
    }
}

/// Mount the packs again now, with what the folder holds (the editor added a skin's
/// files to it).
pub(crate) fn reload() {
    wrote();
    crate::sjk_packs::remount();
}

/// The folder's stamp as last read: it changes when a file of it changed and the packs
/// were mounted again for it (not for what [`wrote`] noted).
pub(crate) fn read_stamp() -> Option<u64> {
    lock().as_ref().map(|lab| lab.stamp)
}

/// Whether the Collection should open on the lab blade now (once a start).
pub(crate) fn take_open() -> bool {
    lock()
        .as_mut()
        .is_some_and(|lab| std::mem::take(&mut lab.open))
}

/// Mount the lab's folder last, over the packs: called by [`crate::sjk_packs::mount`].
pub(crate) fn mount_over(packs: &mut sjk_vfs::VirtualFileSystem) {
    let Some(folder) = folder() else { return };
    if let Err(error) = packs.mount_directory(&folder) {
        crate::log::progress(format_args!("saber lab: {}: {error}", folder.display()));
    }
}

/// After the packs were mounted with the folder: note what its files gave and keep, from
/// `previous`, the last good look of a skin whose file no longer reads.
pub(crate) fn loaded(skins: &mut LoadedSkins, previous: Option<&LoadedSkins>) {
    let Some(folder) = folder() else { return };
    let ids = skin_ids(&folder);
    let mut problem = None;
    for id in &ids {
        let why = if crate::unlockables::blade_skin(id).is_none() {
            Some(format!(
                "{id}: not a blade skin this client knows; name the file {ID}, {EXTRA_PREFIX}<name> or an existing skin's id"
            ))
        } else {
            match crate::blade_skin_file::read(skins.packs(), id) {
                Err(why) => Some(format!("{id}: {why}")),
                // Read, but its images or sounds did not load: the log says which.
                Ok(_) if skins.get(id).is_none() => {
                    Some(format!("{id}: did not load (the console says why)"))
                }
                Ok(_) => None,
            }
        };
        let Some(why) = why else { continue };
        if let Some(kept) = previous.and_then(|previous| previous.get(id)) {
            skins.keep(kept.clone());
        }
        problem.get_or_insert(why);
    }
    let summary = match &problem {
        None => format!("saber lab: {} skin(s) from {}", ids.len(), folder.display()),
        Some(why) => format!("saber lab: {why}"),
    };
    crate::log::progress(format_args!("{summary}"));
    if let Some(lab) = lock().as_mut() {
        lab.reloaded = Instant::now();
        lab.problem = problem;
        lab.ids = ids;
    }
}

/// The Shaders tab's lead while the lab is on: the folder and when it was read, or the
/// first file that does not read.
pub(crate) fn status() -> Option<String> {
    let lab = lock();
    let lab = lab.as_ref()?;
    Some(match &lab.problem {
        Some(why) => format!("Saber Lab: {why} (the last good look stays)."),
        None => format!(
            "Saber Lab: {} skin(s), read {} s ago from {}.",
            lab.ids.len(),
            lab.reloaded.elapsed().as_secs(),
            lab.folder.display()
        ),
    })
}

/// What `saberlab` prints alone, with the lab off.
const USAGE: &str = "saberlab <folder>: blade skins from <folder>/skins/blades, reloaded as you save them (saber_lab.bladeskin is the lab blade); saberlab off stops";

impl crate::GpuState {
    /// `saberlab [<folder> | off]`: start the lab on a folder (a path may hold spaces),
    /// stop it, or say what it reads.
    pub(crate) fn saber_lab_command(&mut self, args: &[String]) -> Vec<String> {
        match args {
            [] => vec![status().unwrap_or_else(|| USAGE.to_owned())],
            [word] if word.eq_ignore_ascii_case("off") => vec![if stop() {
                "Saber Lab off: the hub's packs alone again.".to_owned()
            } else {
                "The Saber Lab is not on.".to_owned()
            }],
            words => match start(Path::new(&words.join(" "))) {
                Ok(folder) => vec![format!(
                    "Saber Lab on {}: save a file there to see it on the saber.",
                    folder.display()
                )],
                Err(why) => vec![format!("saberlab: {why}")],
            },
        }
    }

    /// Each frame: look at the lab's folder, and once a start open the Collection's
    /// Shaders tab on the lab blade (the SJK UI's Collection screen, or the classic
    /// menus' page).
    pub(crate) fn follow_saber_lab(&mut self) {
        poll(Instant::now());
        if !take_open() {
            return;
        }
        let hub = self.in_game_menu.is_sjk();
        if hub {
            self.open_profile_hub(crate::profile_hub::Tab::Shaders);
        }
        if let Some(console) = &mut self.console {
            console.show_saber_lab(hub);
        }
        self.sync_cursor_policy();
    }
}

/// `path` without the `\\?\` Windows puts before a canonical local path, as players
/// write it (a network path keeps its prefix).
fn plain(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => path,
    }
}

/// The ids of `folder`'s skin files (`skins/blades/<id>.bladeskin`), sorted.
fn skin_ids(folder: &Path) -> Vec<String> {
    let blades = folder.join(crate::blade_skin_file::FOLDER);
    let mut ids: Vec<String> = std::fs::read_dir(blades)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_suffix(crate::blade_skin_file::EXTENSION)?;
            Some(id.to_owned())
        })
        .collect();
    ids.sort();
    ids
}

/// A hash of the names, sizes and modification times of `folder`'s files (at most
/// [`MAX_FILES`], found depth first): it changes when one is saved, added or removed.
fn stamp(folder: &Path) -> u64 {
    let mut files: Vec<(PathBuf, u64, Option<SystemTime>)> = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            if files.len() == MAX_FILES {
                break;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                files.push((entry.path(), metadata.len(), metadata.modified().ok()));
            }
        }
    }
    files.sort();
    let mut hasher = DefaultHasher::new();
    files.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stamp changes when a file is saved with new contents, added or removed, and
    /// not otherwise.
    #[test]
    fn the_stamp_follows_saves_additions_and_removals() {
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join("skins/blades");
        std::fs::create_dir_all(&blades).unwrap();
        let file = blades.join("saber_lab.bladeskin");
        std::fs::write(&file, "{}").unwrap();
        let first = stamp(folder.path());
        assert_eq!(stamp(folder.path()), first, "nothing changed");
        std::fs::write(&file, "{ \"version\": 1 }").unwrap();
        let saved = stamp(folder.path());
        assert_ne!(saved, first, "a save with new contents");
        std::fs::write(blades.join("saber_lab.bladeextra"), "{}").unwrap();
        let added = stamp(folder.path());
        assert_ne!(added, saved, "a file added");
        std::fs::remove_file(blades.join("saber_lab.bladeextra")).unwrap();
        assert_eq!(stamp(folder.path()), saved, "the file removed again");
    }

    /// The folder's skins are its `skins/blades/<id>.bladeskin` files, additions and
    /// other files left out.
    #[test]
    fn skin_ids_are_the_skin_files() {
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join("skins/blades");
        std::fs::create_dir_all(&blades).unwrap();
        for name in [
            "saber_lab.bladeskin",
            "saber_lab.bladeextra",
            "saber_sun.bladeskin",
            "notes.txt",
        ] {
            std::fs::write(blades.join(name), "{}").unwrap();
        }
        assert_eq!(skin_ids(folder.path()), ["saber_lab", "saber_sun"]);
        assert!(skin_ids(&folder.path().join("missing")).is_empty());
    }

    /// A folder of loose files gives skins as a pack does; a save that no longer reads
    /// gives none, says why, and the earlier look can be kept, once.
    #[test]
    fn a_loose_folder_is_read_like_a_pack_and_a_broken_save_keeps_the_last_look() {
        // Not a signature, whose built-in file stands in for a broken one.
        let folder = tempfile::tempdir().unwrap();
        let blades = folder.path().join(crate::blade_skin_file::FOLDER);
        std::fs::create_dir_all(&blades).unwrap();
        let file = blades.join("saber_storm.bladeskin");
        std::fs::write(&file, crate::blade_skin_file::tests::SAMPLE).unwrap();
        let read = |generation| {
            let mut vfs = sjk_vfs::VirtualFileSystem::new();
            vfs.mount_directory(folder.path()).unwrap();
            LoadedSkins::load(vfs, generation)
        };
        let good = read(1);
        assert!(good.get("saber_storm").is_some());
        // Saved with a byte-order mark, as Notepad and PowerShell write UTF-8.
        let marked = format!("\u{feff}{}", crate::blade_skin_file::tests::SAMPLE);
        std::fs::write(&file, marked).unwrap();
        assert!(read(3).get("saber_storm").is_some());
        std::fs::write(&file, "{ \"version\": 1, ").unwrap();
        let mut broken = read(2);
        assert!(broken.get("saber_storm").is_none());
        assert!(crate::blade_skin_file::read(broken.packs(), "saber_storm").is_err());
        let last = good.get("saber_storm").unwrap().clone();
        assert!(broken.keep(last.clone()));
        assert!(broken.get("saber_storm").is_some());
        assert!(!broken.keep(last), "a skin is kept once");
    }

    /// A folder reads as players write it, without Windows' `\\?\`; a network path keeps
    /// its prefix.
    #[test]
    fn a_folder_reads_as_written() {
        assert_eq!(
            plain(PathBuf::from(r"\\?\C:\lab")),
            PathBuf::from(r"C:\lab")
        );
        let network = PathBuf::from(r"\\?\UNC\host\lab");
        assert_eq!(plain(network.clone()), network);
        assert_eq!(plain(PathBuf::from("lab")), PathBuf::from("lab"));
    }

    /// The lab blade is a blade skin no holocron drops, and the catalogue knows it only
    /// while the lab is on.
    #[test]
    fn the_lab_blade_is_known_only_while_the_lab_is_on() {
        assert!(BLADE.is_blade_skin() && !BLADE.droppable);
        if !active() {
            assert!(blade(ID).is_none());
            assert!(crate::unlockables::blade_skin(ID).is_none());
        }
    }
}
