//! `fs_restart`: mount the installed game files again without restarting SJK, as
//! the original game's `FS_Restart` searched its paths again. PK3s added or changed
//! since the start, and the pack choices (`fs_disabledPaks`), reach what lists
//! installed content: the profile's models and sabers, Create game's maps, the key
//! bindings' pictures, and the scripts `exec` reads from the packs.
//!
//! The world on show keeps the files it was built from; each map load mounts the
//! files afresh anyway (`session_content.rs`), so the next map has them. The
//! startup-only search settings (`fs_game`, `fs_basegame`, `fs_homepath` and the
//! others whose help says so) still need a restart.
use sjk_vfs::VirtualFileSystem;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(crate) const COMMAND: &str = "fs_restart";
pub(crate) const HELP: &str = "Mount the game files again: new or changed PK3s and pack choices";

type Mounted = Result<Arc<VirtualFileSystem>, String>;

/// The mount in flight on its worker; it goes with the console across map changes.
#[derive(Default)]
pub(crate) struct Task(Option<Receiver<Mounted>>);

impl Task {
    /// Mount the files under `root` on a worker, so a large install does not stall
    /// the frame.
    fn start(&mut self, root: PathBuf) -> Result<(), String> {
        self.start_mount(move || crate::assets::mount_game_data(&root))
    }

    /// Run `mount` on a worker (`rig_reload` mounts the map's own selection this way).
    pub(crate) fn start_mount(
        &mut self,
        mount: impl FnOnce() -> Result<VirtualFileSystem, Box<dyn std::error::Error>> + Send + 'static,
    ) -> Result<(), String> {
        if self.0.is_some() {
            return Err("The game files are already being mounted again".into());
        }
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("sjk-fs-restart".into())
            .spawn(move || {
                let mounted = mount().map(Arc::new).map_err(|error| error.to_string());
                let _ = sender.send(mounted);
            })
            .map_err(|error| format!("Could not start mounting the game files: {error}"))?;
        self.0 = Some(receiver);
        Ok(())
    }

    /// The mount, once the worker has it.
    pub(crate) fn poll(&mut self) -> Option<Mounted> {
        let result = match self.0.as_ref()?.try_recv() {
            Ok(mounted) => mounted,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("the mount worker stopped".into()),
        };
        self.0 = None;
        Some(result)
    }
}

/// The PK3s among `vfs`'s sources, for the console's answer.
fn packs(vfs: &VirtualFileSystem) -> usize {
    vfs.mounts()
        .filter(|mount| mount.name.to_ascii_lowercase().ends_with(".pk3"))
        .count()
}

impl crate::GpuState {
    pub(crate) fn fs_restart_command(&mut self) -> Result<Vec<String>, String> {
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        crate::assets::search_paths::reload_pack_choices(console)
            .map_err(|error| format!("fs_disabledPaks: {error}"))?;
        console.fs_restart_task().start(self.game_data.clone())?;
        crate::log::progress(format_args!("fs_restart: mounting the game files again"));
        Ok(vec!["Mounting the game files again...".into()])
    }

    /// Hand a finished `fs_restart` mount to the menus and the console's scripts.
    pub(crate) fn poll_fs_restart(&mut self) {
        let Some(console) = &mut self.console else {
            return;
        };
        let vfs = match console.fs_restart_task().poll() {
            None => return,
            Some(Ok(vfs)) => vfs,
            Some(Err(error)) => {
                crate::log::progress(format_args!("fs_restart failed: {error}"));
                console.push_log(format!("^1fs_restart failed: {error}"));
                return;
            }
        };
        console.attach_script_vfs(Arc::clone(&vfs));
        let packs = packs(&vfs);
        if let Some(menu) = &mut self.client_menu {
            menu.refresh_catalogue(vfs, console);
        }
        crate::log::progress(format_args!("fs_restart: {packs} PK3s mounted"));
        console.push_log(format!(
            "Game files mounted again: {packs} PK3s. The map on show keeps its files until the next map."
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn finish(task: &mut Task) -> Mounted {
        loop {
            if let Some(mounted) = task.poll() {
                return mounted;
            }
            std::thread::yield_now();
        }
    }

    #[test]
    fn a_new_pack_is_mounted_and_a_second_start_waits() {
        let install = tempfile::tempdir().unwrap();
        let base = install.path().join("base");
        std::fs::create_dir_all(&base).unwrap();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(base.join("zz_new.pk3")).unwrap());
        zip.start_file(
            "models/players/fresh/model.glm",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();

        let mut task = Task::default();
        assert!(task.poll().is_none());
        task.start(install.path().to_path_buf()).unwrap();
        assert!(task.start(install.path().to_path_buf()).is_err());
        let vfs = finish(&mut task).unwrap();
        assert!(vfs.contains("models/players/fresh/model.glm").unwrap());
        assert!(packs(&vfs) >= 1);
        // Done: the next fs_restart may start.
        task.start(install.path().to_path_buf()).unwrap();
        finish(&mut task).unwrap();
    }
}
