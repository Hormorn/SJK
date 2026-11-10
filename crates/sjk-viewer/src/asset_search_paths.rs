//! MP startup search-path policy. Generic mount mechanics remain in sjk-vfs.
use crate::console::ViewerConsole;
use sjk_shell::{CvarDefinition, CvarFlags, CvarRegistry};
use sjk_vfs::VirtualFileSystem;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

/// The settings every mount reads: frozen at startup ([`initialize`]), save for the
/// pack choices, which `fs_restart` takes again ([`reload_pack_choices`]).
static STARTUP: RwLock<Option<Arc<Options>>> = RwLock::new(None);

/// Further content roots that stand in for files installed in the game's own
/// folders, by owner: SJK's JoF content folder while its switch is on
/// (`jof_content.rs`), holding what JoF's launcher would put in `GameData/base`.
/// Their PK3s sort by name among the installation's in the same game folder, as
/// if they were there, so a player's own `zz` pack still wins over them. Unlike
/// the startup settings they are read at every mount, so a change applies from
/// the next map.
static EXTRA_ROOTS: RwLock<Vec<(&'static str, PathBuf)>> = RwLock::new(Vec::new());

/// Add, move or (with None) remove `owner`'s content root. Returns whether it
/// changed.
#[cfg_attr(not(feature = "mod-jof"), allow(dead_code))]
pub(crate) fn set_extra_root(owner: &'static str, root: Option<PathBuf>) -> bool {
    let mut roots = EXTRA_ROOTS
        .write()
        .unwrap_or_else(|error| error.into_inner());
    let index = roots.iter().position(|(known, _)| *known == owner);
    match (index, root) {
        (Some(index), Some(root)) if roots[index].1 == root => false,
        (Some(index), Some(root)) => {
            roots[index].1 = root;
            true
        }
        (None, Some(root)) => {
            roots.push((owner, root));
            true
        }
        (Some(index), None) => {
            roots.remove(index);
            true
        }
        (None, None) => false,
    }
}

fn extra_roots() -> Vec<PathBuf> {
    EXTRA_ROOTS
        .read()
        .unwrap_or_else(|error| error.into_inner())
        .iter()
        .map(|(_, root)| root.clone())
        .collect()
}

#[path = "asset_pack_policy.rs"]
pub(crate) mod pack_policy;
/// The cosmetics packs were named in the log (every world mounts them).
static COSMETICS_LOGGED: AtomicBool = AtomicBool::new(false);

/// The folder JoF EJK and EternalJK keep their own content in, where JoF's
/// launcher installs hat and cape packs.
const COSMETICS_GAME: &str = "EternalJK";
/// Where a pack's hats and capes are; a PK3 with models there is a
/// cosmetics pack.
const COSMETIC_FOLDERS: [&str; 2] = ["models/cosmetics/hats", "models/cosmetics/capes"];
/// JoF EJK's client pictures (`jofclient-assets.pk3`): the Force wheel's Repulse,
/// Dash and flamethrower icons. The pack also holds the sounds and effects JoF
/// servers play, such as `sound/jof/repulse.mp3`.
const JOF_CLIENT_FOLDER: &str = "gfx/jof";

/// Startup settings shared by world-loading workers.
#[derive(Clone)]
pub(crate) struct Options {
    game: String,
    basegame: String,
    home: Option<PathBuf>,
    portable: bool,
    directory_first: bool,
    debug: bool,
    /// Prefer base-game shader definitions; immutable after startup.
    pub(crate) protect_shaders: bool,
    disabled: pack_policy::Disabled,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            game: String::new(),
            basegame: String::new(),
            home: None,
            portable: true,
            directory_first: false,
            debug: false,
            protect_shaders: true,
            disabled: Default::default(),
        }
    }
}

/// Register startup preferences; edits apply on next process launch, not a map reload.
pub(crate) fn register(cvars: &mut CvarRegistry) -> Result<(), sjk_shell::CvarError> {
    for definition in [
        CvarDefinition::new(
            pack_policy::CVAR,
            "[]",
            CvarFlags::ARCHIVE,
            "Disabled installed PK3 packs (JSON list); edit in assetbrowser; applies at fs_restart",
        ),
        CvarDefinition::new(
            "fs_game",
            "",
            CvarFlags::ARCHIVE,
            "Mod directory; restart the client after changing",
        ),
        CvarDefinition::new(
            "fs_basegame",
            "",
            CvarFlags::ARCHIVE,
            "Intermediate game directory; client restart required",
        ),
        CvarDefinition::new(
            "fs_homepath",
            "",
            CvarFlags::ARCHIVE,
            "Optional explicit content home; empty disables it; restart required",
        ),
    ] {
        cvars.register(definition)?;
    }
    for (name, value, help) in [
        (
            "fs_portable",
            true,
            "Ignore optional content home; client restart required",
        ),
        (
            "fs_dirbeforepak",
            false,
            "Prefer loose files within each directory; restart required",
        ),
        (
            "fs_debug",
            false,
            "Log mounted sources and asset reads; client restart required",
        ),
        (
            "fs_protectShaders",
            true,
            "Prefer base-game shader definitions; client restart required",
        ),
    ] {
        cvars.register(CvarDefinition::new(name, value, CvarFlags::ARCHIVE, help))?;
    }
    Ok(())
}

/// Freeze settings once before the first native world load; evidence defaults stay isolated.
pub(crate) fn initialize(console: &ViewerConsole) -> Result<(), Box<dyn Error>> {
    let options = Options::from_console(console)?;
    let mut startup = STARTUP.write().unwrap_or_else(|error| error.into_inner());
    if startup.is_some() {
        return Err("asset startup settings already initialized".into());
    }
    *startup = Some(Arc::new(options));
    Ok(())
}

/// Snapshot shared by all subsequent mounts without config reads.
pub(crate) fn startup() -> Arc<Options> {
    if let Some(options) = &*STARTUP.read().unwrap_or_else(|error| error.into_inner()) {
        return Arc::clone(options);
    }
    let mut startup = STARTUP.write().unwrap_or_else(|error| error.into_inner());
    Arc::clone(startup.get_or_insert_with(|| Arc::new(Options::default())))
}

/// `fs_restart`: take the pack choices (`fs_disabledPaks`) again for the mounts from
/// now on. The other settings keep their startup values, as their help says.
pub(crate) fn reload_pack_choices(console: &ViewerConsole) -> Result<(), Box<dyn Error>> {
    let disabled =
        pack_policy::Disabled::parse(console.text_value(pack_policy::CVAR).unwrap_or("[]"))?;
    let current = startup();
    let mut startup = STARTUP.write().unwrap_or_else(|error| error.into_inner());
    *startup = Some(Arc::new(current.with_pack_choices(disabled)));
    Ok(())
}

impl Options {
    /// These settings with `disabled` as the pack choices.
    fn with_pack_choices(&self, disabled: pack_policy::Disabled) -> Self {
        Self {
            disabled,
            ..self.clone()
        }
    }

    pub(crate) fn from_console(console: &ViewerConsole) -> Result<Self, Box<dyn Error>> {
        let game = console.text_value("fs_game").unwrap_or("").to_owned();
        let basegame = console.text_value("fs_basegame").unwrap_or("").to_owned();
        validate_directory(&game)?;
        validate_directory(&basegame)?;
        let home = console
            .text_value("fs_homepath")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Ok(Self {
            disabled: pack_policy::Disabled::parse(
                console.text_value(pack_policy::CVAR).unwrap_or("[]"),
            )?,
            game,
            basegame,
            home,
            portable: console.bool_cvar("fs_portable").unwrap_or(true),
            directory_first: console.bool_cvar("fs_dirbeforepak").unwrap_or(false),
            debug: console.bool_cvar("fs_debug").unwrap_or(false),
            protect_shaders: console.bool_cvar("fs_protectShaders").unwrap_or(true),
        })
    }

    /// Low-to-high priority directories, matching FS_Startup's nested ordering;
    /// within each game folder the installation, the extra roots
    /// ([`set_extra_root`]), then the content home; last the
    /// [assets folders](sjk_vfs::assets_folders), which hold PK3s only.
    pub(crate) fn directories(&self, install: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
        Ok(self
            .directory_groups(install, &extra_roots())
            .into_iter()
            .flatten()
            .chain(sjk_vfs::assets_folders(install))
            .collect())
    }

    /// [`Self::directories`] in mount groups, low to high priority: each game
    /// folder's installation directory with the extra roots' (their PK3s mount
    /// as one folder's, [`sjk_vfs::pk3_search_order_merged`]), then the content
    /// home's alone, above them as FS_Startup puts the home path.
    fn directory_groups(&self, install: &Path, extra: &[PathBuf]) -> Vec<Vec<PathBuf>> {
        let home = if self.portable {
            None
        } else {
            // Downloads are session-selected, never an implicit global content home.
            self.home.as_deref()
        };
        let mut seen = Vec::new();
        let mut groups = Vec::new();
        for game in ["base", self.basegame.as_str(), self.game.as_str()] {
            if game.is_empty() {
                continue;
            }
            let installed = std::iter::once(install).chain(extra.iter().map(PathBuf::as_path));
            for roots in [installed.collect::<Vec<_>>(), home.into_iter().collect()] {
                let group: Vec<PathBuf> = roots
                    .into_iter()
                    .map(|root| root.join(game))
                    .filter(|path| !seen.contains(path))
                    .collect();
                seen.extend(group.iter().cloned());
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
        groups
    }

    /// The PK3s in `install/EternalJK` that carry hats or capes
    /// (`models/cosmetics/`) or JoF's client pictures (`gfx/jof/`), lowest
    /// priority first: JoF EJK's cosmetics and client assets, found without
    /// mounting the rest of that folder (its menus, HUD and strings). None
    /// when the folder is a game directory already.
    pub(crate) fn cosmetic_packs(&self, install: &Path) -> Vec<PathBuf> {
        let mounted = [self.basegame.as_str(), self.game.as_str()]
            .iter()
            .any(|game| game.eq_ignore_ascii_case(COSMETICS_GAME));
        let directory = install.join(COSMETICS_GAME);
        if mounted || !directory.is_dir() {
            return Vec::new();
        }
        sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .filter(|archive| self.disabled.allows(archive))
            .filter(|archive| carries_jof_content(archive))
            .collect()
    }

    /// The `install/EternalJK` folder when its PK3s are to be probed for single
    /// pictures: None when it is absent or already a game directory.
    fn eternaljk_directory(&self, install: &Path) -> Option<PathBuf> {
        let mounted = [self.basegame.as_str(), self.game.as_str()]
            .iter()
            .any(|game| game.eq_ignore_ascii_case(COSMETICS_GAME));
        let directory = install.join(COSMETICS_GAME);
        (!mounted && directory.is_dir()).then_some(directory)
    }

    /// EternalJK's own crosshair pictures, `gfx/2d/crosshaira` and `crosshairj`,
    /// from the highest-priority PK3 in `install/EternalJK` that has each (jaPRO's
    /// `japro-assets.pk3`), as `(archive, path, image bytes)`. EternalJK mounts that
    /// folder above `base`, so these replace the base pictures there. Empty when
    /// the folder is a game directory already.
    pub(crate) fn eternaljk_crosshairs(&self, install: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
        let Some(directory) = self.eternaljk_directory(install) else {
            return Vec::new();
        };
        let archives: Vec<(PathBuf, VirtualFileSystem)> = sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter(|archive| self.disabled.allows(archive))
            .filter_map(|archive| {
                let mut probe = VirtualFileSystem::new();
                probe.mount_pk3(&archive).ok()?;
                Some((archive, probe))
            })
            .collect();
        ["a", "j"]
            .iter()
            .filter_map(|letter| {
                archives.iter().find_map(|(archive, probe)| {
                    ["tga", "png", "jpg"].iter().find_map(|extension| {
                        let path = format!("gfx/2d/crosshair{letter}.{extension}");
                        let asset = probe.read(&path).ok()??;
                        Some((archive.clone(), path, asset.bytes))
                    })
                })
            })
            .collect()
    }

    /// EternalJK's black saber pictures, `gfx/effects/sabers/blackglow`, `blackcore` and
    /// `blacktrail`, from the highest-priority PK3 in `install/EternalJK` that has each
    /// (jaPRO's `japro-assets.pk3`), as `(archive, path, image bytes)`: JoF EJK draws its
    /// black blades with them (`cg_main.c`, `blackSaberGlowShader` and the others), so
    /// SJK's look the same where they are installed (Dave's test, 27/10/2026). Only these
    /// pictures, nothing else from the pack. Empty when the folder is a game directory.
    pub(crate) fn eternaljk_black_sabers(&self, install: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
        let Some(directory) = self.eternaljk_directory(install) else {
            return Vec::new();
        };
        let archives: Vec<(PathBuf, VirtualFileSystem)> = sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter(|archive| self.disabled.allows(archive))
            .filter_map(|archive| {
                let mut probe = VirtualFileSystem::new();
                probe.mount_pk3(&archive).ok()?;
                Some((archive, probe))
            })
            .collect();
        ["glow", "core", "trail"]
            .iter()
            .filter_map(|kind| {
                archives.iter().find_map(|(archive, probe)| {
                    ["png", "tga", "jpg"].iter().find_map(|extension| {
                        let path = format!("gfx/effects/sabers/black{kind}.{extension}");
                        let asset = probe.read(&path).ok()??;
                        Some((archive.clone(), path, asset.bytes))
                    })
                })
            })
            .collect()
    }

    /// jaPRO's movement key pictures, `gfx/hud/keys/<key>_on` and `_off`, from the
    /// highest-priority PK3 in `install/EternalJK` that has each (jaPRO's
    /// `japro-assets.pk3`), as `(archive, path, image bytes)`: JoF EJK's
    /// `cg_movementKeys` draws them (`cg_main.c`, `keyJumpOnShader` and the others), so
    /// SJK's keys look the same where they are installed ([`crate::hud::movement_keys`]).
    /// Only these pictures, nothing else from the pack. Empty when the folder is a game
    /// directory.
    pub(crate) fn eternaljk_movement_keys(
        &self,
        install: &Path,
    ) -> Vec<(PathBuf, String, Vec<u8>)> {
        let Some(directory) = self.eternaljk_directory(install) else {
            return Vec::new();
        };
        let archives: Vec<(PathBuf, VirtualFileSystem)> = sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter(|archive| self.disabled.allows(archive))
            .filter_map(|archive| {
                let mut probe = VirtualFileSystem::new();
                probe.mount_pk3(&archive).ok()?;
                Some((archive, probe))
            })
            .collect();
        crate::hud::movement_keys::PICTURES
            .iter()
            .filter_map(|name| {
                archives.iter().find_map(|(archive, probe)| {
                    ["tga", "png", "jpg"].iter().find_map(|extension| {
                        let path = format!("{name}.{extension}");
                        let asset = probe.read(&path).ok()??;
                        Some((archive.clone(), path, asset.bytes))
                    })
                })
            })
            .collect()
    }

    /// jaPRO's hats and capes in the folders JoF's pieces used before
    /// `models/cosmetics/` ([`crate::cosmetics::MOVED_FOLDERS`]): their pictures and any
    /// `.shader` file defining only such shaders (`japro_hats.shader`), from the
    /// highest-priority PK3 in `install/EternalJK` that has each (jaPRO's
    /// `japro-assets.pk3`), as `(archive, path, bytes)`. Seven of JoF's hats name these
    /// folders and JoF EJK draws them from here (hub report #192); only these files are
    /// taken from the pack. Empty when the folder is a game directory already.
    pub(crate) fn eternaljk_old_hats(&self, install: &Path) -> Vec<(PathBuf, String, Vec<u8>)> {
        let Some(directory) = self.eternaljk_directory(install) else {
            return Vec::new();
        };
        let mut found: Vec<(PathBuf, String, Vec<u8>)> = Vec::new();
        for archive in sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter(|archive| self.disabled.allows(archive))
        {
            let mut probe = VirtualFileSystem::new();
            if probe.mount_pk3(&archive).is_err() {
                continue;
            }
            let take = |path: String, found: &mut Vec<(PathBuf, String, Vec<u8>)>| {
                if found
                    .iter()
                    .any(|(_, known, _)| known.eq_ignore_ascii_case(&path))
                {
                    return None;
                }
                let bytes = probe.read(&path).ok()??.bytes;
                Some((archive.clone(), path, bytes))
            };
            for (old, _) in crate::cosmetics::MOVED_FOLDERS {
                for extension in [".png", ".tga", ".jpg", ".jpeg"] {
                    for name in probe.list_files(old, extension) {
                        if let Some(file) = take(format!("{old}{name}"), &mut found) {
                            found.push(file);
                        }
                    }
                }
            }
            for name in probe.list_files("shaders", ".shader") {
                let path = format!("shaders/{name}");
                let only_hats = probe.read(&path).ok().flatten().is_some_and(|asset| {
                    crate::cosmetics::only_old_folder_shaders(&String::from_utf8_lossy(
                        &asset.bytes,
                    ))
                });
                if only_hats && let Some(file) = take(path, &mut found) {
                    found.push(file);
                }
            }
        }
        found
    }

    /// JoF EternalJK's chat emoji pictures, every `gfx/emoji/*.png` in the PK3s of
    /// `install/EternalJK` (jaPRO's `japro-assets.pk3` ships 175), as `(path as the
    /// archive stores it, image bytes)`, the highest-priority archive's copy of each
    /// name. EternalJK mounts that folder whole; only these pictures are taken from
    /// it here. Empty when the folder is a game directory already.
    pub(crate) fn eternaljk_emojis(&self, install: &Path) -> Vec<(String, Vec<u8>)> {
        let Some(directory) = self.eternaljk_directory(install) else {
            return Vec::new();
        };
        let mut found: Vec<(String, Vec<u8>)> = Vec::new();
        for archive in sjk_vfs::pk3_search_order(&directory)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .filter(|archive| self.disabled.allows(archive))
        {
            let mut probe = VirtualFileSystem::new();
            if probe.mount_pk3(&archive).is_err() {
                continue;
            }
            for name in probe.list_files(crate::chat::emoji::FOLDER, ".png") {
                let path = format!("{}/{name}", crate::chat::emoji::FOLDER);
                if found
                    .iter()
                    .any(|(known, _)| known.eq_ignore_ascii_case(&path))
                {
                    continue;
                }
                let Some(asset) = probe.read(&path).ok().flatten() else {
                    continue;
                };
                found.push((probe.original_name(&path).unwrap_or(path), asset.bytes));
            }
        }
        found
    }

    /// Mount existing directories; unreadable archives warn without discarding other packs.
    pub(crate) fn mount(&self, install: &Path) -> Result<VirtualFileSystem, Box<dyn Error>> {
        self.mount_with(install, &extra_roots())
    }

    /// [`Self::mount`] with these extra roots ([`set_extra_root`]).
    fn mount_with(
        &self,
        install: &Path,
        extra: &[PathBuf],
    ) -> Result<VirtualFileSystem, Box<dyn Error>> {
        // Large offline imports may opt into a higher per-asset ceiling. Keep the
        // ordinary viewer default and generic VFS policy unchanged.
        let limit = std::env::var("SJK_MAX_ASSET_MIB")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(1024)
            .checked_mul(1024 * 1024)
            .filter(|&n| n > 0)
            .ok_or("SJK_MAX_ASSET_MIB must be positive and fit in u64")?;
        let mut vfs = VirtualFileSystem::with_max_asset_bytes(limit);
        vfs.set_read_diagnostics(self.debug);
        // SJK's own content (Illuminate's holocron, the card's outline, the hub's
        // packs with the unlockables' art), below everything so game data with the same paths
        // replaces it.
        crate::illuminate::mount(&mut vfs)?;
        crate::cardboard_box::mount(&mut vfs)?;
        crate::fishing::mount(&mut vfs)?;
        crate::mining::mount(&mut vfs)?;
        crate::order66::mount(&mut vfs)?;
        crate::hax::mount(&mut vfs)?;
        crate::ragdoll::mount(&mut vfs)?;
        crate::drone::mount(&mut vfs)?;
        crate::boombox::mount(&mut vfs)?;
        crate::monitor::mount(&mut vfs)?;
        crate::box_hunt::mount(&mut vfs)?;
        crate::quests::mount(&mut vfs)?;
        crate::banana::mount(&mut vfs)?;
        crate::tags::mount(&mut vfs)?;
        crate::mystery_box::mount(&mut vfs)?;
        crate::wheel_of_doom::mount(&mut vfs)?;
        crate::saber_petals::mount(&mut vfs)?;
        crate::status_balloons::mount(&mut vfs)?;
        super::force_presets::mount(&mut vfs)?;
        crate::card_outline::mount(&mut vfs)?;
        crate::duel_ghost::mount(&mut vfs)?;
        crate::sjk_packs::mount_below_game_data(&mut vfs);
        // JoF EJK's hats and capes, below everything else so they never
        // replace other content.
        let log = !COSMETICS_LOGGED.swap(true, Ordering::Relaxed);
        for pack in self.cosmetic_packs(install) {
            match vfs.mount_pk3(&pack) {
                Ok(_) if log => crate::log::progress(format_args!(
                    "JoF EJK content (hats, capes, client assets) from {}",
                    pack.display(),
                )),
                Ok(_) => {}
                Err(error) => crate::log::progress(format_args!(
                    "warning: skipping cosmetics PK3 {}: {error}",
                    pack.display(),
                )),
            }
        }
        // Older generated packs in the game directories, left out when SJK's own
        // folder has one (`generated_packs.rs`).
        let generated = super::generated::present();
        let mut replaced = Vec::new();
        for group in self.directory_groups(install, extra) {
            let group: Vec<PathBuf> = group.into_iter().filter(|path| path.is_dir()).collect();
            if !self.directory_first {
                for directory in &group {
                    vfs.mount_directory(directory)?;
                }
            }
            // The PK3s tidied into the assets folder count as base's.
            let mut pack_folders = group.clone();
            if group.contains(&install.join("base")) {
                pack_folders.extend(sjk_vfs::assets_folders(install));
            }
            for archive in sjk_vfs::pk3_search_order_merged(&pack_folders)? {
                if self.disabled.allows(&archive) {
                    match vfs.mount_pk3(&archive) {
                        Ok(id) if generated && super::generated::is_generated(&vfs, id) => {
                            super::generated::log_replaced(&archive);
                            replaced.push(id);
                        }
                        Ok(_) => {}
                        Err(error) => crate::log::progress(format_args!(
                            "warning: skipping PK3 {}: {error}",
                            archive.display()
                        )),
                    }
                }
            }
            if self.directory_first {
                for directory in &group {
                    vfs.mount_directory(directory)?;
                }
            }
        }
        // EternalJK's crosshair pictures over the game's own, as EternalJK itself
        // mounts its folder above `base`; only those images, nothing else from
        // the pack.
        let crosshairs = self.eternaljk_crosshairs(install);
        if !crosshairs.is_empty() {
            if log {
                for (archive, path, _) in &crosshairs {
                    crate::log::progress(format_args!(
                        "crosshair picture {path} from {}",
                        archive.display(),
                    ));
                }
            }
            vfs.mount_memory(
                "EternalJK crosshairs",
                crosshairs.into_iter().map(|(_, path, bytes)| (path, bytes)),
            )?;
        }
        // EternalJK's black saber pictures, likewise only those.
        let black = self.eternaljk_black_sabers(install);
        if !black.is_empty() {
            if log {
                for (archive, path, _) in &black {
                    crate::log::progress(format_args!(
                        "black saber picture {path} from {}",
                        archive.display(),
                    ));
                }
            }
            vfs.mount_memory(
                "EternalJK black sabers",
                black.into_iter().map(|(_, path, bytes)| (path, bytes)),
            )?;
        }
        // jaPRO's movement key pictures, likewise only those.
        let keys = self.eternaljk_movement_keys(install);
        if !keys.is_empty() {
            if log {
                crate::log::progress(format_args!(
                    "{} movement key pictures from {}",
                    keys.len(),
                    install.join(COSMETICS_GAME).display(),
                ));
            }
            vfs.mount_memory(
                "EternalJK movement keys",
                keys.into_iter().map(|(_, path, bytes)| (path, bytes)),
            )?;
        }
        // The hats and capes whose models name the folders JoF's pieces used before
        // `models/cosmetics/`: jaPRO's copies from the EternalJK folder, likewise only
        // those, as JoF EJK draws them, and where it has none the same picture from the
        // new folder under the old name (`cosmetics::old_folder_pictures`, hub report
        // #192: they drew grey).
        let mut hats: Vec<(String, Vec<u8>)> = Vec::new();
        for (archive, path, bytes) in self.eternaljk_old_hats(install) {
            if log {
                crate::log::progress(format_args!("hat file {path} from {}", archive.display()));
            }
            hats.push((path, bytes));
        }
        let pieces = crate::cosmetics::piece_models(&vfs);
        let moved = crate::cosmetics::old_folder_pictures(&vfs, &pieces, |path| {
            hats.iter()
                .any(|(known, _)| known.eq_ignore_ascii_case(path))
        });
        if log {
            for (path, _) in &moved {
                crate::log::progress(format_args!("hat picture {path} from the new folder"));
            }
        }
        hats.extend(moved);
        if !hats.is_empty() {
            vfs.mount_memory("EternalJK hats", hats)?;
        }
        // JoF's pieces built inside out (McFox), drawn two-sided as JoF EJK shows them.
        if let Some((path, text)) = crate::cosmetics::inside_out_shaders(&pieces) {
            if log {
                crate::log::progress(format_args!(
                    "two-sided cosmetics: {}",
                    String::from_utf8_lossy(&text)
                        .lines()
                        .filter(|line| !line.starts_with(['\t', '{', '}', '/']))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            vfs.mount_memory("SJK two-sided cosmetics", [(path, text)])?;
        }
        // JoF EternalJK's chat emojis, likewise only those pictures from the folder.
        let emojis = self.eternaljk_emojis(install);
        if !emojis.is_empty() {
            if log {
                crate::log::progress(format_args!(
                    "{} chat emoji pictures from {}",
                    emojis.len(),
                    install.join(COSMETICS_GAME).display(),
                ));
            }
            vfs.mount_memory("EternalJK chat emojis", emojis)?;
        }
        // `SJK_CONTENT=<dir>[:<dir>...]`: further content directories (loose files and
        // PK3s), above the installation: locally made content that has no place in it.
        for directory in std::env::var_os("SJK_CONTENT")
            .iter()
            .flat_map(std::env::split_paths)
            .filter(|directory| directory.is_dir())
        {
            vfs.mount_pk3_directory_with_warnings(&directory, |path, error| {
                crate::log::progress(format_args!(
                    "warning: skipping PK3 {}: {error}",
                    path.display()
                ));
            })?;
            vfs.mount_directory(&directory)?;
        }
        if !replaced.is_empty() {
            vfs = vfs.without_mounts(&replaced);
        }
        // The player's generated packs, above everything: no other pack replaces them.
        super::generated::mount_on_top(&mut vfs);
        if self.debug {
            for mount in vfs.mounts() {
                crate::log::progress(format_args!(
                    "fs mount: {} ({} entries)",
                    mount.name, mount.entries,
                ));
            }
        }
        Ok(vfs)
    }
}

/// Whether `archive` holds hat or cape models.
fn carries_jof_content(archive: &Path) -> bool {
    let mut probe = VirtualFileSystem::new();
    probe.mount_pk3(archive).is_ok()
        && (COSMETIC_FOLDERS
            .iter()
            .any(|folder| !probe.list_files(folder, ".md3").is_empty())
            || !probe.list_files(JOF_CLIENT_FOLDER, "").is_empty())
}

fn validate_directory(name: &str) -> Result<(), Box<dyn Error>> {
    if name == "."
        || name.contains("..")
        || name.contains(['/', '\\', ':'])
        || name.chars().any(char::is_control)
        || name.trim() != name
    {
        return Err(
            format!("invalid game directory {name:?}: expected a single directory name",).into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn pk3(path: &Path, entries: &[&str]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for entry in entries {
            zip.start_file(*entry, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extra_roots_follow_the_installation_in_each_game_folder() {
        let options = Options {
            game: "japlus".into(),
            ..Options::default()
        };
        let install = Path::new("/game");
        let jof = PathBuf::from("/sjk/jof");
        assert_eq!(
            options.directory_groups(install, std::slice::from_ref(&jof)),
            [
                vec![install.join("base"), jof.join("base")],
                vec![install.join("japlus"), jof.join("japlus")],
            ]
        );
    }

    #[test]
    fn extra_root_packs_sort_by_name_among_the_installations() {
        // Kushina's report (hub #100, 19/10/2026): JoF content beat her own packs.
        let install = tempfile::tempdir().unwrap();
        let jof = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        let jof_base = jof.path().join("base");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&jof_base).unwrap();
        pk3(
            &base.join("assets0.pk3"),
            &["gfx/sphere.tga", "gfx/anim.tga"],
        );
        pk3(
            &jof_base.join("jof_assets.pk3"),
            &["gfx/sphere.tga", "gfx/anim.tga"],
        );
        pk3(&base.join("zz_mine.pk3"), &["gfx/sphere.tga"]);
        let vfs = Options::default()
            .mount_with(install.path(), &[jof.path().to_owned()])
            .unwrap();
        let source = |path: &str| {
            vfs.read(path)
                .unwrap()
                .unwrap()
                .source
                .mount_name
                .to_string()
        };
        // The player's zz pack wins, JoF's still beats retail.
        assert!(source("gfx/sphere.tga").ends_with("zz_mine.pk3"));
        assert!(source("gfx/anim.tga").ends_with("jof_assets.pk3"));
    }

    #[test]
    fn packs_in_the_assets_folder_and_below_sort_among_base() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        let nested = install.path().join("assets/jof/maps");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&nested).unwrap();
        pk3(
            &base.join("b.pk3"),
            &["gfx/a.tga", "gfx/b.tga", "gfx/c.tga"],
        );
        pk3(&install.path().join("assets/a.pk3"), &["gfx/a.tga"]);
        pk3(&nested.join("c.pk3"), &["gfx/c.tga"]);
        let source = |options: &Options, path: &str| {
            let vfs = options.mount(install.path()).unwrap();
            vfs.read(path)
                .unwrap()
                .unwrap()
                .source
                .mount_name
                .to_string()
        };
        let mut options = Options::default();
        // By file name, whichever folder holds the pack.
        assert!(source(&options, "gfx/a.tga").ends_with("b.pk3"));
        assert!(source(&options, "gfx/c.tga").ends_with("c.pk3"));
        // The asset browser's switch reaches them.
        options.disabled.toggle("maps/c.pk3");
        assert!(source(&options, "gfx/c.tga").ends_with("b.pk3"));
        // They are PK3s only: a loose file there is no game file.
        std::fs::write(nested.join("loose.tga"), b"x").unwrap();
        std::fs::write(install.path().join("assets/loose.tga"), b"x").unwrap();
        let vfs = options.mount(install.path()).unwrap();
        assert!(!vfs.contains("loose.tga").unwrap());
    }

    #[test]
    fn only_eternaljk_packs_with_hats_capes_or_jof_pictures_are_found() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &folder.join("zzz_jof_cosmetics.pk3"),
            &[
                "models/cosmetics/hats/santahat.md3",
                "shaders/japro_hats.shader",
            ],
        );
        pk3(
            &folder.join("capes_only.pk3"),
            &["models/cosmetics/capes/royalcape.md3"],
        );
        pk3(&folder.join("menus.pk3"), &["ui/jamp/main.menu"]);
        pk3(
            &folder.join("jofclient-assets.pk3"),
            &["gfx/jof/force_dash.tga", "sound/jof/repulse.mp3"],
        );
        let options = Options::default();
        let names: Vec<String> = options
            .cosmetic_packs(install.path())
            .iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 3, "{names:?}");
        assert!(names.iter().all(|name| name != "menus.pk3"));
        // The hats reach the mounted file system, the menus do not.
        let vfs = options.mount(install.path()).unwrap();
        assert!(vfs.contains("models/cosmetics/hats/santahat.md3").unwrap());
        assert!(vfs.contains("gfx/jof/force_dash.tga").unwrap());
        assert!(!vfs.contains("ui/jamp/main.menu").unwrap());
        // With the folder as a game directory, it is mounted whole instead.
        let whole = Options {
            basegame: "EternalJK".to_owned(),
            ..Options::default()
        };
        assert!(whole.cosmetic_packs(install.path()).is_empty());
    }

    #[test]
    fn disabling_a_pack_restores_the_lower_priority_asset_and_can_be_undone() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        std::fs::create_dir_all(&base).unwrap();
        pk3(&base.join("a.pk3"), &["models/fixture.md3"]);
        pk3(&base.join("zzz.pk3"), &["models/fixture.md3"]);
        let mut options = Options::default();
        let source = |options: &Options| {
            options
                .mount(install.path())
                .unwrap()
                .read("models/fixture.md3")
                .unwrap()
                .unwrap()
                .source
                .mount_name
                .to_string()
        };
        assert!(source(&options).ends_with("zzz.pk3"));
        options.disabled.toggle("BASE/ZZZ.PK3");
        assert!(source(&options).ends_with("a.pk3"));
        options.disabled.toggle("base/zzz.pk3");
        assert!(source(&options).ends_with("zzz.pk3"));
    }

    #[test]
    fn fs_restart_takes_new_pack_choices_and_keeps_the_rest() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        std::fs::create_dir_all(&base).unwrap();
        pk3(&base.join("zzz.pk3"), &["models/fixture.md3"]);
        let started = Options {
            game: "mymod".into(),
            protect_shaders: false,
            ..Options::default()
        };
        let mut disabled = pack_policy::Disabled::default();
        disabled.toggle("base/zzz.pk3");
        let restarted = started.with_pack_choices(disabled);
        assert_eq!(restarted.game, "mymod");
        assert!(!restarted.protect_shaders);
        assert!(!restarted.disabled.allows(&base.join("zzz.pk3")));
        assert!(started.disabled.allows(&base.join("zzz.pk3")));
    }

    #[test]
    fn disabled_eternaljk_packs_do_not_reenter_as_supplemental_assets() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &folder.join("extras.pk3"),
            &[
                "models/cosmetics/hats/example.md3",
                "gfx/2d/crosshaira.png",
                "gfx/emoji/example.png",
            ],
        );
        let mut options = Options::default();
        assert_eq!(options.cosmetic_packs(install.path()).len(), 1);
        assert_eq!(options.eternaljk_crosshairs(install.path()).len(), 1);
        assert_eq!(options.eternaljk_emojis(install.path()).len(), 1);
        options.disabled.toggle("eternaljk/extras.pk3");
        assert!(options.cosmetic_packs(install.path()).is_empty());
        assert!(options.eternaljk_crosshairs(install.path()).is_empty());
        assert!(options.eternaljk_emojis(install.path()).is_empty());
    }

    #[test]
    fn eternaljk_black_saber_pictures_are_mounted_alone() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &folder.join("japro-assets.pk3"),
            &[
                "gfx/effects/sabers/blackglow.png",
                "gfx/effects/sabers/blackcore.png",
                "gfx/effects/sabers/blacktrail.png",
                "gfx/effects/sabers/red_glow2.png",
            ],
        );
        let options = Options::default();
        assert_eq!(options.eternaljk_black_sabers(install.path()).len(), 3);
        let vfs = options.mount(install.path()).unwrap();
        assert!(vfs.contains("gfx/effects/sabers/blacktrail.png").unwrap());
        assert!(!vfs.contains("gfx/effects/sabers/red_glow2.png").unwrap());
    }

    #[test]
    fn eternaljk_movement_key_pictures_are_mounted_alone() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        let mut entries: Vec<String> = crate::hud::movement_keys::PICTURES
            .iter()
            .map(|name| format!("{name}.tga"))
            .collect();
        entries.push("gfx/hud/speedometer.tga".to_owned());
        let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
        pk3(&folder.join("japro-assets.pk3"), &entries);
        let options = Options::default();
        assert_eq!(options.eternaljk_movement_keys(install.path()).len(), 12);
        let vfs = options.mount(install.path()).unwrap();
        assert!(vfs.contains("gfx/hud/keys/jump_on.tga").unwrap());
        assert!(vfs.contains("gfx/hud/keys/right_off.tga").unwrap());
        assert!(!vfs.contains("gfx/hud/speedometer.tga").unwrap());
    }

    #[test]
    fn eternaljk_emojis_are_mounted_without_the_rest_of_their_pack() {
        let install = tempfile::tempdir().unwrap();
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &folder.join("japro-assets.pk3"),
            &[
                "gfx/emoji/`poop`.png",
                "gfx/emoji/#~`!D.png",
                "gfx/emoji/readme.txt",
                "ui/jamp/ingame.menu",
            ],
        );
        let options = Options::default();
        let vfs = options.mount(install.path()).unwrap();
        let mut listed = vfs.list_files("gfx/emoji", ".png");
        listed.sort();
        assert_eq!(listed, ["#~`!d.png", "`poop`.png"]);
        // The stored name keeps its case for the `!` rule.
        assert_eq!(
            vfs.original_name("gfx/emoji/#~`!d.png").as_deref(),
            Some("gfx/emoji/#~`!D.png")
        );
        assert!(!vfs.contains("ui/jamp/ingame.menu").unwrap());
        assert!(!vfs.contains("gfx/emoji/readme.txt").unwrap());
    }

    #[test]
    fn eternaljk_crosshairs_override_the_base_ones_alone() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        let folder = install.path().join("EternalJK");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&folder).unwrap();
        pk3(
            &base.join("hd_icons.pk3"),
            &[
                "gfx/2d/crosshaira.tga",
                "gfx/2d/crosshairb.tga",
                "gfx/2d/crosshairj.tga",
            ],
        );
        pk3(
            &folder.join("japro-assets.pk3"),
            &[
                "gfx/2d/crosshaira.tga",
                "gfx/2d/crosshairj.tga",
                "ui/jamp/ingame.menu",
            ],
        );
        let options = Options::default();
        let found: Vec<String> = options
            .eternaljk_crosshairs(install.path())
            .into_iter()
            .map(|(_, path, _)| path)
            .collect();
        assert_eq!(found, ["gfx/2d/crosshaira.tga", "gfx/2d/crosshairj.tga"]);
        let vfs = options.mount(install.path()).unwrap();
        let from = |path: &str| {
            let asset = vfs.read(path).unwrap().unwrap();
            vfs.mounts()
                .find(|mount| mount.id == asset.source.mount_id)
                .map(|mount| mount.name.to_string())
                .unwrap()
        };
        assert_eq!(from("gfx/2d/crosshaira.tga"), "EternalJK crosshairs");
        assert_eq!(from("gfx/2d/crosshairj.tga"), "EternalJK crosshairs");
        assert_ne!(from("gfx/2d/crosshairb.tga"), "EternalJK crosshairs");
        // Nothing else from that pack is mounted.
        assert!(!vfs.contains("ui/jamp/ingame.menu").unwrap());
    }
}
