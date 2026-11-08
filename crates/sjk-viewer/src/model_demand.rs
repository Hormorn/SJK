//! World-owned first-use rigid-model loading. Parse on a worker, install on the GPU owner.
//! The worker also decodes the model's textures and builds their mip chains
//! ([`crate::world_materials::warm_entity_materials`]), as for players' models: decoded
//! at install, HD texture packs' 15 MB JPEGs for `falcon2.md3` stalled a frame 2.1 s.
//! Its textures then go up one a frame: an HD pack's texture with its mips is near 90 MB
//! to copy, and the falcon's all at once still held a frame 208 ms. The pipelines its
//! materials need and the map lacks compile on another worker (`world_pipeline_jobs.rs`)
//! before the model is drawn, as for players' models.
//!
//! Models wanted by name rather than by an entity go the same way
//! ([`GpuState::want_model`]): the hats and capes players wear and the models an effect
//! needs. Read inside the frame that asked, each held it 6 to 10 ms on Sol's RTX 5080
//! and far longer on a weaker PC, for every new hat a joining player wore (#236).
use super::*;
use crate::world_materials::pipeline_jobs::Compiled;
use std::sync::mpsc::{Receiver, TryRecvError};

type Prepared = Result<
    (
        StaticModelMesh,
        FlattenedScene,
        crate::world_materials::WarmTextures,
    ),
    String,
>;

#[derive(Default)]
pub(super) struct Demand {
    initialized: bool,
    /// Includes attempted failures so a broken community model is not retried every frame.
    attempted: BTreeSet<Appearance>,
    task: Option<(Appearance, Receiver<Prepared>)>,
    /// A model read, its textures going up one a frame.
    uploading: Option<(
        StaticModelMesh,
        FlattenedScene,
        crate::world_materials::WarmTextures,
    )>,
    /// A model uploaded, drawn once its pipelines are compiled.
    compiling: Option<(StaticModelMesh, Receiver<Compiled>)>,
    /// Models asked for by name ([`GpuState::want_model`]), loaded in turn.
    queued: Vec<Appearance>,
}

fn rigid(appearance: &Appearance) -> bool {
    !appearance.model.starts_with(['*', '$', '@'])
        && !sjk_client::legacy_npc_body_model(&appearance.model)
}

impl GpuState {
    /// Load the rigid model `model` off the render thread, as a world entity's model
    /// is; it is drawable once it is in [`Self::object_meshes`]. A queued model coming in
    /// has players' cosmetics read again, as one of them may wear it.
    pub(crate) fn want_model(&mut self, model: &str) {
        let appearance = Appearance {
            model: model.to_owned(),
            variant: String::new(),
        };
        let models = &mut self.config_string_refresh.models;
        if !rigid(&appearance)
            || models.attempted.contains(&appearance)
            || models.queued.contains(&appearance)
        {
            return;
        }
        models.queued.push(appearance);
    }

    /// A model asked for by name went in (or failed): players' cosmetics read again.
    fn queued_model_done(&mut self, appearance: &Appearance) {
        let models = &mut self.config_string_refresh.models;
        let before = models.queued.len();
        models.queued.retain(|queued| queued != appearance);
        if models.queued.len() != before {
            for mesh in &mut self.actor_meshes {
                mesh.cosmetics.invalidate();
            }
        }
    }

    /// At most one CPU preparation and one GPU installation in flight per world.
    pub(crate) fn refresh_demand_models(&mut self) {
        if !self.config_string_refresh.models.initialized {
            self.config_string_refresh.models.attempted.extend(
                self.object_meshes
                    .iter()
                    .map(|mesh| mesh.appearance.clone()),
            );
            self.config_string_refresh.models.initialized = true;
        }
        if self.advance_demand_model() {
            return;
        }
        let world = self
            .demo_session
            .as_ref()
            .map_or(&self.live_world, demo_playback::Session::world);
        // Borrow existing appearances; no owned names are made during an idle scan.
        let wanted = world
            .entities()
            .filter(|entity| !matches!(entity.kind, EntityKind::Actor | EntityKind::Corpse))
            .filter_map(|entity| entity.appearance())
            .find(|appearance| {
                rigid(appearance)
                    && !self
                        .config_string_refresh
                        .models
                        .attempted
                        .contains(*appearance)
            })
            .cloned()
            .or_else(|| {
                // A mover's second model (`mover_models`) is not a world appearance.
                let (game, snapshot) = match (&self.live_session, &self.demo_session) {
                    (Some(live), _) => (live.game_state(), live.latest_snapshot()),
                    (None, Some(demo)) => (demo.game_state(), demo.latest_snapshot()),
                    (None, None) => return None,
                };
                let attempted = &self.config_string_refresh.models.attempted;
                crate::mover_models::wanted(game, snapshot, |appearance| {
                    attempted.contains(appearance)
                })
            })
            .or_else(|| {
                // The two parts of an emplaced gun being worked (`emplaced_swivel`).
                let snapshot = match (&self.live_session, &self.demo_session) {
                    (Some(live), _) => live.latest_snapshot(),
                    (None, Some(demo)) => demo.latest_snapshot(),
                    (None, None) => return None,
                };
                let attempted = &self.config_string_refresh.models.attempted;
                crate::emplaced_swivel::wanted(snapshot, world, |appearance| {
                    attempted.contains(appearance)
                })
            })
            .or_else(|| {
                // A model a map edit places (`map_edits`).
                let attempted = &self.config_string_refresh.models.attempted;
                self.map_edits
                    .wanted_models()
                    .map(|model| Appearance {
                        model: model.to_owned(),
                        variant: String::new(),
                    })
                    .find(|appearance| !attempted.contains(appearance))
            })
            .or_else(|| {
                // The models of the toys the Toy Master hid (`toy_master`).
                let attempted = &self.config_string_refresh.models.attempted;
                self.toy_master
                    .wanted_models()
                    .map(|model| Appearance {
                        model: model.to_owned(),
                        variant: String::new(),
                    })
                    .find(|appearance| !attempted.contains(appearance))
            })
            .or_else(|| {
                // Models asked for by name: worn cosmetics, effects' models.
                let models = &self.config_string_refresh.models;
                models
                    .queued
                    .iter()
                    .find(|appearance| !models.attempted.contains(*appearance))
                    .cloned()
            })
            .or_else(|| {
                // The mystery boxes' models: the box, what rolls over it (`mystery_box`).
                let attempted = &self.config_string_refresh.models.attempted;
                self.mystery_boxes
                    .wanted_models()
                    .map(|model| Appearance {
                        model: model.to_owned(),
                        variant: String::new(),
                    })
                    .find(|appearance| !attempted.contains(appearance))
            });
        let (Some(appearance), Some(vfs)) = (wanted, self.vfs.clone()) else {
            return;
        };
        self.config_string_refresh
            .models
            .attempted
            .insert(appearance.clone());
        // Configstring/effect changes may already have installed this model.
        if self
            .object_meshes
            .iter()
            .any(|mesh| mesh.appearance == appearance)
        {
            self.queued_model_done(&appearance);
            return;
        }
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let requested = appearance.clone();
        let shaders = Arc::clone(&self.shaders);
        let lightmap = self.world_materials.entity_lightmap();
        let mipmapped = self.world_materials.mipmapped();
        match std::thread::Builder::new()
            .name("sjk-model-demand".into())
            .spawn(move || {
                let mut scene = FlattenedScene::default();
                let result = object_meshes::load_one(&vfs, &requested, &mut scene)
                    .map(|mesh| {
                        let textures = crate::world_materials::warm_entity_materials(
                            &vfs,
                            &shaders,
                            &lightmap,
                            &scene.materials,
                            mipmapped,
                        );
                        (mesh, scene, textures)
                    })
                    .map_err(|error| error.to_string());
                // Dropping the owning world discards old results, never installs them in another map.
                let _ = sender.send(result);
            }) {
            Ok(_) => self.config_string_refresh.models.task = Some((appearance, receiver)),
            Err(error) => log::progress(format_args!("could not start model loader: {error}")),
        }
    }

    /// Take the model on its way one step further: its read, one texture, its upload
    /// or its pipelines. `true` while it is still on its way: no other starts.
    fn advance_demand_model(&mut self) -> bool {
        let models = &mut self.config_string_refresh.models;
        if let Some((mesh, receiver)) = models.compiling.take() {
            match receiver.try_recv() {
                Ok(compiled) => self.world_materials.install_pipelines(compiled),
                Err(TryRecvError::Empty) => {
                    models.compiling = Some((mesh, receiver));
                    return true;
                }
                // The pipelines compile on the model's first draw instead.
                Err(TryRecvError::Disconnected) => {}
            }
            let appearance = mesh.appearance.clone();
            self.push_config_model(mesh);
            self.queued_model_done(&appearance);
            return false;
        }
        if let Some((mesh, scene, mut textures)) = models.uploading.take() {
            if let Some(texture) = textures.pop() {
                self.world_materials
                    .upload_prepared_texture(&self.device, &self.queue, texture);
                self.config_string_refresh.models.uploading = Some((mesh, scene, textures));
                return true;
            }
            let requested = mesh.appearance.clone();
            let appearance = mesh.appearance.model.clone();
            match self.upload_config_model(mesh, scene) {
                Ok(Some((mesh, jobs))) if jobs.is_empty() => self.push_config_model(mesh),
                Ok(Some((mesh, jobs))) => {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    let spawned = std::thread::Builder::new()
                        .name("sjk-model-pipelines".into())
                        .spawn(move || {
                            let _ = sender.send(jobs.compile());
                        });
                    match spawned {
                        Ok(_) => {
                            self.config_string_refresh.models.compiling = Some((mesh, receiver));
                            return true;
                        }
                        // The pipelines compile on the model's first draw instead.
                        Err(_) => self.push_config_model(mesh),
                    }
                }
                Ok(None) => {}
                Err(error) => log::progress(format_args!("model {appearance}: {error}")),
            }
            self.queued_model_done(&requested);
            return false;
        }
        let Some((appearance, receiver)) = models.task.take() else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => {
                models.task = Some((appearance, receiver));
                return true;
            }
            Err(TryRecvError::Disconnected) => Err("model loader stopped".to_owned()),
        };
        match result {
            Ok(read) => {
                models.uploading = Some(read);
                true
            }
            Err(error) => {
                log::progress(format_args!("model {}: {error}", appearance.model));
                self.queued_model_done(&appearance);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_models_and_deformable_bodies_stay_with_their_owners() {
        for model in ["*1", "$swoop", "@single_1", "models/players/kyle/model.glm"] {
            assert!(!rigid(&Appearance {
                model: model.into(),
                variant: String::new()
            }));
        }
        assert!(rigid(&Appearance {
            model: "models/props/crate.md3".into(),
            variant: String::new()
        }));
    }

    #[test]
    #[ignore = "needs a GPU and JKA_GAME_DATA; retail assets are external"]
    fn a_new_world_model_is_prepared_and_installed_once_on_a_gpu() {
        crate::world_shot::on_big_stack(|| {
            let started = std::time::Instant::now();
            let (mut gpu, _profile) = crate::world_shot::open(
                "maps/mp/duel6.bsp",
                [640, 480],
                None,
                &[("r_hdr", "0"), ("cg_materialMaps", "0")],
            )
            .expect("a GPU adapter");
            let model = Appearance {
                model: std::env::var("SJK_TEST_MODEL")
                    .unwrap_or_else(|_| "models/map_objects/imperial/antenna.md3".into()),
                variant: String::new(),
            };
            assert!(
                !gpu.object_meshes
                    .iter()
                    .any(|mesh| mesh.appearance == model)
            );
            let count = gpu.object_meshes.len();
            let entity = EntityId::new(900);
            gpu.live_world.upsert(
                entity,
                EntityKind::Other,
                sjk_runtime::MotionSample {
                    time_millis: 0,
                    transform: sjk_runtime::Transform::IDENTITY,
                },
            );
            gpu.live_world.set_appearance(entity, Some(model.clone()));
            let loading = std::time::Instant::now();
            let deadline = loading + std::time::Duration::from_secs(30);
            // The longest call is the frame a player feels.
            let mut worst = std::time::Duration::ZERO;
            loop {
                let call = std::time::Instant::now();
                gpu.refresh_demand_models();
                worst = worst.max(call.elapsed());
                if gpu
                    .object_meshes
                    .iter()
                    .any(|mesh| mesh.appearance == model)
                {
                    break;
                }
                assert!(std::time::Instant::now() < deadline, "model did not arrive");
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            for _ in 0..100 {
                gpu.refresh_demand_models();
            }
            assert_eq!(gpu.object_meshes.len(), count + 1);
            assert_eq!(gpu.object_groups.len(), gpu.object_meshes.len());
            assert!(matches!(
                gpu.render(&mut None),
                crate::gpu_context::FrameStatus::Rendered
            ));
            println!(
                "duel6 startup {:.1} ms; demanded model {:.1} ms, worst frame {:.1} ms; {} initial rigid models",
                (loading - started).as_secs_f64() * 1000.0,
                loading.elapsed().as_secs_f64() * 1000.0,
                worst.as_secs_f64() * 1000.0,
                count
            );
        });
    }
}
