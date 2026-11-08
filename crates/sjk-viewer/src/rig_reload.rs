//! `rig_reload`: read the rig models again without restarting SJK, for a model being
//! made in Blender. The SJK Rig Model Export add-on replaces `rig_<name>.pk3` while the
//! game holds the old one open; this command mounts the files again (the map's own
//! selection, as a map load does, on a worker), forgets the rig models' cached clip
//! lenders and loads every rig model a player wears anew from the new files.
//!
//! The map, its sounds and the shader scripts stay as they were: a changed shader
//! script still needs the next map.
use sjk_vfs::VirtualFileSystem;
use std::collections::BTreeSet;
use std::sync::Arc;

pub(crate) const COMMAND: &str = "rig_reload";
pub(crate) const HELP: &str = "Read the rig models again (a model re-exported from Blender)";

/// The model folders among `models` that are rig models in `vfs`, or were in `old` (a
/// folder that stopped being one loads as what it is now).
pub(crate) fn rig_folders<'a>(
    models: impl IntoIterator<Item = &'a str>,
    vfs: &VirtualFileSystem,
    old: Option<&VirtualFileSystem>,
) -> BTreeSet<String> {
    let is_rig = |vfs: &VirtualFileSystem, model: &str| {
        crate::rig_model::manifest(vfs, model.trim_end_matches('/'), "default")
            .ok()
            .flatten()
            .is_some()
    };
    models
        .into_iter()
        .filter(|model| is_rig(vfs, model) || old.is_some_and(|old| is_rig(old, model)))
        .map(str::to_owned)
        .collect()
}

impl crate::GpuState {
    pub(crate) fn rig_reload_command(&mut self) -> Result<Vec<String>, String> {
        let game = self
            .live_session
            .as_ref()
            .map(sjk_client::ClientSession::game_state)
            .or_else(|| self.demo_session.as_ref().map(|s| s.game_state()));
        let selection = game
            .map(crate::assets::session_content::Selection::from_game)
            .transpose()
            .map_err(|error| format!("rig_reload: {error}"))?;
        let root = self.game_data.clone();
        let console = self.console.as_mut().ok_or("Console unavailable")?;
        console
            .rig_reload_task()
            .start_mount(move || match selection {
                Some(selection) => selection.mount(&root),
                None => crate::assets::mount_game_data(&root),
            })?;
        crate::log::progress(format_args!("rig_reload: mounting the game files again"));
        Ok(vec!["Reading the rig models again...".into()])
    }

    /// Install a finished `rig_reload` mount: the world reads from it, and each player
    /// wearing a rig model loads it again.
    pub(crate) fn poll_rig_reload(&mut self) {
        let Some(console) = &mut self.console else {
            return;
        };
        let vfs = match console.rig_reload_task().poll() {
            None => return,
            Some(Ok(vfs)) => vfs,
            Some(Err(error)) => {
                crate::log::progress(format_args!("rig_reload failed: {error}"));
                console.push_log(format!("^1rig_reload failed: {error}"));
                return;
            }
        };
        crate::rig_model::forget_sources();
        let worn: Vec<String> = self
            .actor_meshes
            .iter()
            .filter(|mesh| !mesh.corpse_pool)
            .map(|mesh| mesh.appearance.model.clone())
            .collect();
        let folders = rig_folders(worn.iter().map(String::as_str), &vfs, self.vfs.as_deref());
        if let Some(menu) = &mut self.client_menu {
            menu.refresh_catalogue(Arc::clone(&vfs), console);
        }
        self.vfs = Some(vfs);
        let watch = &mut self.clientinfo_watch;
        watch
            .failed
            .retain(|appearance| !folders.contains(&appearance.model));
        // Materials are matched by name: a re-exported texture must be read again.
        watch.appended.clear();
        watch.reload.clone_from(&folders);
        let mut players = 0;
        for client in 0..32u16 {
            let entity = sjk_runtime::EntityId::new(u64::from(client) + 1);
            let wears = self.actor_meshes.iter().any(|mesh| {
                !mesh.corpse_pool
                    && mesh.entity_id == Some(entity)
                    && folders.contains(&mesh.appearance.model)
            });
            if !wears {
                continue;
            }
            match self.apply_clientinfo(client) {
                Ok(()) => players += 1,
                Err(error) => {
                    crate::log::progress(format_args!("rig_reload client {client}: {error}"));
                }
            }
        }
        self.clientinfo_watch.reload.clear();
        let names: Vec<&str> = folders
            .iter()
            .map(|folder| folder.rsplit('/').next().unwrap_or(folder))
            .collect();
        crate::log::progress(format_args!(
            "rig_reload: {} rig models ({}) on {players} players",
            folders.len(),
            names.join(", ")
        ));
        if let Some(console) = &mut self.console {
            console.push_log(if folders.is_empty() {
                "Rig models read again: nobody wears one.".to_owned()
            } else {
                format!(
                    "Rig models read again: {} on {players} players.",
                    names.join(", ")
                )
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vfs_with(files: &[(&str, &[u8])]) -> VirtualFileSystem {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "test",
            files.iter().map(|(path, bytes)| (*path, bytes.to_vec())),
        )
        .unwrap();
        vfs
    }

    #[test]
    fn only_rig_model_folders_are_read_again() {
        let manifest: &[u8] = b"model model.gltf\npreset mannequin\nscale auto\n";
        let new = vfs_with(&[("models/players/fox/sjk_rig.cfg", manifest)]);
        let old = vfs_with(&[("models/players/wolf/sjk_rig.cfg", manifest)]);
        let folders = rig_folders(
            [
                "models/players/fox",
                "models/players/kyle",
                "models/players/wolf",
            ],
            &new,
            Some(&old),
        );
        assert_eq!(
            folders.into_iter().collect::<Vec<_>>(),
            ["models/players/fox", "models/players/wolf"]
        );
    }
}
