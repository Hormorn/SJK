//! NPC bodies and vehicles read off the render thread, as players' models are
//! (`player_loads.rs`). An NPC an admin spawns or a vehicle a map brings in (`$swoop`,
//! `$tauntaun`, a humanoid NPC's model) used to be read, parsed, skinned and its
//! textures decoded inside one frame: logs showed 36 to 70 ms, and 965 ms for a
//! community model with large textures. Now a worker reads the model, builds its mesh
//! and decodes its textures; the render thread uploads them one a frame, then the
//! model, and its new pipelines compile on another worker before it is drawn
//! (`world_pipeline_jobs.rs`). One NPC
//! loads at a time; the NPC shows nothing until its model is in, as before a frame
//! built it.

use super::*;
use crate::actor_load::build_actor_mesh;
use crate::player_assets::{GlaCache, load_npc_appearance_with};
use crate::world_materials::pipeline_jobs::Compiled;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// What the worker prepared for one NPC.
struct Read {
    mesh: ActorMesh,
    scene: FlattenedScene,
    cache: GlaCache,
    textures: crate::world_materials::WarmTextures,
}

enum Stage {
    Reading(Receiver<Result<Read, String>>),
    /// Read; its textures go up one a frame (`model_demand.rs` says why).
    Uploading(Read),
    /// Uploaded; drawn once its pipelines are compiled.
    Compiling(Option<ActorMesh>, Receiver<Compiled>),
}

struct Pending {
    entity_id: EntityId,
    appearance: Appearance,
    stage: Stage,
}

/// The NPC model of one world being loaded, if any.
#[derive(Default)]
pub(crate) struct Load {
    pending: Option<Pending>,
}

impl Load {
    /// Whether no NPC model is on its way: another may start. Only one loads at a time.
    pub(super) fn idle(&self) -> bool {
        self.pending.is_none()
    }
}

/// The worker: the NPC's model, its mesh and its textures' mip chains.
fn read(
    appearance: Appearance,
    entity_id: EntityId,
    saber_names: [Option<String>; 2],
    sources: crate::clientinfo_refresh::player_loads::Sources,
) -> Result<Read, String> {
    let crate::clientinfo_refresh::player_loads::Sources {
        vfs,
        shaders,
        lightmap,
        mut cache,
        mipmapped,
    } = sources;
    let preview =
        load_npc_appearance_with(&vfs, &appearance.model, &appearance.variant, &mut cache)
            .map_err(|error| error.to_string())?;
    let mut scene = FlattenedScene::default();
    let mesh = build_actor_mesh(
        &mut scene,
        preview,
        Some(entity_id),
        false,
        appearance,
        saber_names,
    )
    .map_err(|error| error.to_string())?;
    let textures = crate::world_materials::warm_entity_materials(
        &vfs,
        &shaders,
        &lightmap,
        &scene.materials,
        mipmapped,
    );
    Ok(Read {
        mesh,
        scene,
        cache,
        textures,
    })
}

impl GpuState {
    /// Start reading `appearance` for the NPC `entity_id` on a worker.
    pub(super) fn start_npc_load(
        &mut self,
        entity_id: EntityId,
        appearance: Appearance,
        saber_names: [Option<String>; 2],
    ) -> Result<(), Box<dyn Error>> {
        for name in saber_names.iter().flatten() {
            self.load_hilt(name)?;
        }
        let sources = self.player_load_sources().ok_or("no VFS")?;
        let (sender, receiver) = mpsc::sync_channel(1);
        let requested = appearance.clone();
        std::thread::Builder::new()
            .name("sjk-npc-load".into())
            .spawn(move || {
                // Dropping the owning world discards the result.
                let _ = sender.send(read(requested, entity_id, saber_names, sources));
            })?;
        self.clientinfo_watch.npc_loads.pending = Some(Pending {
            entity_id,
            appearance,
            stage: Stage::Reading(receiver),
        });
        Ok(())
    }

    /// Once a frame: upload the NPC model a worker read, or put it on its NPC once its
    /// pipelines are compiled. Nothing in flight costs one branch.
    pub(super) fn poll_npc_load(&mut self) {
        let Some(mut pending) = self.clientinfo_watch.npc_loads.pending.take() else {
            return;
        };
        match pending.stage {
            Stage::Reading(receiver) => match receiver.try_recv() {
                Ok(Ok(read)) => {
                    pending.stage = Stage::Uploading(read);
                    self.clientinfo_watch.npc_loads.pending = Some(pending);
                }
                Err(TryRecvError::Empty) => {
                    pending.stage = Stage::Reading(receiver);
                    self.clientinfo_watch.npc_loads.pending = Some(pending);
                }
                Ok(Err(error)) => {
                    self.npc_load_failed(pending.entity_id, pending.appearance, &error)
                }
                Err(TryRecvError::Disconnected) => {
                    self.npc_load_failed(
                        pending.entity_id,
                        pending.appearance,
                        "NPC model loader stopped",
                    );
                }
            },
            Stage::Uploading(mut read) => {
                if let Some(texture) = read.textures.pop() {
                    self.world_materials.upload_prepared_texture(
                        &self.device,
                        &self.queue,
                        texture,
                    );
                    pending.stage = Stage::Uploading(read);
                    self.clientinfo_watch.npc_loads.pending = Some(pending);
                    return;
                }
                self.clientinfo_watch.gla_cache.absorb(read.cache);
                let uploaded =
                    self.upload_built_actor_deferred(read.mesh, read.scene, &pending.appearance);
                match uploaded {
                    Ok((mesh, jobs)) if jobs.is_empty() => {
                        self.place_npc_mesh(pending.entity_id, mesh, &pending.appearance);
                    }
                    Ok((mesh, jobs)) => {
                        let (sender, receiver) = mpsc::channel();
                        let spawned = std::thread::Builder::new()
                            .name("sjk-model-pipelines".into())
                            .spawn(move || {
                                let _ = sender.send(jobs.compile());
                            });
                        if spawned.is_err() {
                            // The pipelines compile on the model's first draw instead.
                            self.place_npc_mesh(pending.entity_id, mesh, &pending.appearance);
                            return;
                        }
                        pending.stage = Stage::Compiling(Some(mesh), receiver);
                        self.clientinfo_watch.npc_loads.pending = Some(pending);
                    }
                    Err(error) => self.npc_load_failed(
                        pending.entity_id,
                        pending.appearance,
                        &error.to_string(),
                    ),
                }
            }
            Stage::Compiling(mesh, receiver) => {
                match receiver.try_recv() {
                    Ok(compiled) => self.world_materials.install_pipelines(compiled),
                    Err(TryRecvError::Empty) => {
                        pending.stage = Stage::Compiling(mesh, receiver);
                        self.clientinfo_watch.npc_loads.pending = Some(pending);
                        return;
                    }
                    // The pipelines compile on the model's first draw instead.
                    Err(TryRecvError::Disconnected) => {}
                }
                if let Some(mesh) = mesh {
                    self.place_npc_mesh(pending.entity_id, mesh, &pending.appearance);
                }
            }
        }
    }

    fn npc_load_failed(&mut self, entity_id: EntityId, appearance: Appearance, error: &str) {
        log::progress(format_args!(
            "npc {}: could not load {}/{}: {error}",
            entity_id.get() - 1,
            appearance.model,
            appearance.variant
        ));
        self.clientinfo_watch.failed.insert(appearance);
    }

    /// Give `entity_id` its mesh, in place of the one it had.
    fn place_npc_mesh(&mut self, entity_id: EntityId, mesh: ActorMesh, appearance: &Appearance) {
        let index = self
            .actor_meshes
            .iter()
            .position(|mesh| !mesh.corpse_pool && mesh.entity_id == Some(entity_id));
        match index {
            Some(index) => self.actor_meshes[index] = mesh,
            None => {
                self.actor_meshes.push(mesh);
                self.actor_groups.push(Vec::with_capacity(4));
            }
        }
        log::progress(format_args!(
            "npc {} wears {}/{}",
            entity_id.get() - 1,
            appearance.model,
            appearance.variant
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance(model: &str) -> Appearance {
        Appearance {
            model: model.into(),
            variant: String::new(),
        }
    }

    fn reading(entity: u64, model: &str) -> (Load, mpsc::SyncSender<Result<Read, String>>) {
        let (sender, receiver) = mpsc::sync_channel(1);
        let load = Load {
            pending: Some(Pending {
                entity_id: EntityId::new(entity),
                appearance: appearance(model),
                stage: Stage::Reading(receiver),
            }),
        };
        (load, sender)
    }

    #[test]
    fn an_npc_load_starts_when_none_runs() {
        assert!(Load::default().idle());
    }

    #[test]
    fn a_running_npc_load_holds_the_next_back() {
        let (load, _sender) = reading(112, "$swoop");
        assert!(!load.idle());
    }
}
