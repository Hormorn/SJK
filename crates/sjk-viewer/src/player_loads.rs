//! Players' models loaded off the render thread. A player who joins or changes model
//! (a `CS_PLAYERS` change, a forced-model or mute change) used to load and upload the
//! whole model inside one frame: the `.glm`, its skin and animation sounds read and
//! parsed, its textures decoded, then uploaded, which players felt as a stutter. Now a
//! worker reads and parses the model, builds its mesh and decodes its textures
//! ([`crate::world_materials::warm_entity_materials`]); the render thread only uploads
//! it: one texture a frame, as models a server names and NPCs do, then its materials and
//! geometry. All its textures in one frame cost 255 to 425 ms for an HD player model on
//! Slash's built-in GPU (`zzzpiza/original`, 24/10/2026). The worker also builds the
//! model's body copy, which a later frame puts in the body pool sharing the model's
//! materials: a player's first death with a model built it on the render thread, the
//! 35 to 310 ms hitches of the same log at kills (`corpse_actors.rs`). Meanwhile the slot keeps the model it had, or shows the
//! stand-in actor until the first one is in (`actor_world_submission.rs`). A model whose
//! materials need pipelines the map had not compiled waits for another worker to compile
//! them (`world_pipeline_jobs.rs`) before it is put on its player: compiled on the render
//! thread they cost up to 85 ms.

use super::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// Workers reading models at once; more requests wait their turn.
const IN_FLIGHT: usize = 3;

/// What a worker prepared for one player.
pub(crate) struct Loaded {
    pub(crate) mesh: ActorMesh,
    pub(crate) scene: FlattenedScene,
    /// The skeletons the worker parsed, kept for the next loads.
    pub(crate) cache: GlaCache,
    /// Why the requested model could not be loaded; its stand-in is drawn.
    pub(crate) failed: Option<String>,
    /// Its textures' mip chains, built here.
    pub(crate) textures: crate::world_materials::WarmTextures,
    /// The same model again, for the body pool.
    pub(crate) body: Option<(ActorMesh, FlattenedScene)>,
}

/// A model's body copy waiting for a frame of its own, with where the model's
/// materials start.
struct Body {
    appearance: Appearance,
    mesh: ActorMesh,
    scene: FlattenedScene,
    material_base: usize,
}

type Prepared = Result<Loaded, String>;

/// One player's model to load.
pub(crate) struct Request {
    pub(crate) client: u16,
    pub(crate) appearance: Appearance,
    pub(crate) entity_id: EntityId,
    pub(crate) saber_names: [Option<String>; 2],
    /// The model failed in this world before: its stand-in at once.
    pub(crate) known_failed: bool,
    /// What is drawn when the model cannot be loaded: Kyle, or a retail model with
    /// `cg_defaultModelRandom` ([`crate::actor_load::stand_in`]).
    pub(crate) stand_in: Appearance,
    /// Build a body copy for the body pool: the player's bodies wear this model, not a
    /// forced or overridden one ([`GpuState::body_appearance`]).
    pub(crate) pool_body: bool,
}

struct Task {
    client: u16,
    appearance: Appearance,
    receiver: Receiver<Prepared>,
}

/// A model uploaded but not yet drawn: its pipelines are being compiled.
struct Compiling {
    client: u16,
    appearance: Appearance,
    /// `None` once the player wanted another model: the pipelines still go in.
    mesh: Option<ActorMesh>,
    receiver: Receiver<Compiled>,
}

/// A finished model going up one texture a frame, before its materials and geometry.
struct Uploading {
    client: u16,
    appearance: Appearance,
    loaded: Loaded,
}

/// The loads of one world.
#[derive(Default)]
pub(crate) struct Loads {
    running: Vec<Task>,
    waiting: Vec<Request>,
    uploading: Option<Uploading>,
    body: Option<Body>,
    compiling: Vec<Compiling>,
}

/// What a worker needs from the world.
pub(crate) struct Sources {
    pub(crate) vfs: Arc<VirtualFileSystem>,
    pub(crate) shaders: Arc<ShaderCatalog>,
    pub(crate) lightmap: wgpu::TextureView,
    pub(crate) cache: GlaCache,
    pub(crate) mipmapped: bool,
}

impl Loads {
    /// Whether `client`'s model `appearance` is being loaded or waits.
    pub(crate) fn pending(&self, client: u16, appearance: &Appearance) -> bool {
        self.running
            .iter()
            .any(|task| task.client == client && task.appearance == *appearance)
            || self
                .waiting
                .iter()
                .any(|request| request.client == client && request.appearance == *appearance)
            || self.uploading.as_ref().is_some_and(|uploading| {
                uploading.client == client && uploading.appearance == *appearance
            })
            || self.compiling.iter().any(|compiling| {
                compiling.client == client
                    && compiling.appearance == *appearance
                    && compiling.mesh.is_some()
            })
    }

    /// Forget `client`'s load: the player wants another model, the one they wear, or
    /// left. A worker already reading finishes and its result is dropped.
    pub(crate) fn cancel(&mut self, client: u16) {
        self.running.retain(|task| task.client != client);
        self.waiting.retain(|request| request.client != client);
        // Its textures already up stay in the material cache for the next load.
        if self
            .uploading
            .as_ref()
            .is_some_and(|uploading| uploading.client == client)
        {
            self.uploading = None;
        }
        for compiling in &mut self.compiling {
            if compiling.client == client {
                compiling.mesh = None;
            }
        }
    }

    /// The first model whose pipelines are compiled, with them; workers that stopped
    /// leave their slots to compile on first draw.
    fn take_compiled(&mut self) -> Option<(Compiling, Option<Compiled>)> {
        for position in 0..self.compiling.len() {
            let compiled = match self.compiling[position].receiver.try_recv() {
                Ok(compiled) => Some(compiled),
                Err(TryRecvError::Empty) => continue,
                Err(TryRecvError::Disconnected) => None,
            };
            return Some((self.compiling.remove(position), compiled));
        }
        None
    }

    /// Queue `request` in place of any earlier one for its player.
    pub(crate) fn request(&mut self, request: Request) {
        if self.pending(request.client, &request.appearance) {
            return;
        }
        self.cancel(request.client);
        self.waiting.push(request);
    }

    /// Start waiting requests while fewer than [`IN_FLIGHT`] run.
    fn start(&mut self, sources: impl Fn() -> Option<Sources>) {
        while self.running.len() < IN_FLIGHT && !self.waiting.is_empty() {
            let Some(sources) = sources() else {
                return;
            };
            let request = self.waiting.remove(0);
            let (sender, receiver) = mpsc::channel();
            self.running.push(Task {
                client: request.client,
                appearance: request.appearance.clone(),
                receiver,
            });
            let spawned = std::thread::Builder::new()
                .name("sjk-player-load".into())
                .spawn(move || {
                    let _ = sender.send(prepare(request, sources));
                });
            if let Err(error) = spawned {
                log::progress(format_args!("player model loader: {error}"));
                self.running.pop();
                return;
            }
        }
    }

    /// The first finished load, if any, with its player; failed workers are dropped.
    fn take_finished(&mut self) -> Option<(u16, Appearance, Prepared)> {
        let mut finished = None;
        self.running.retain(|task| {
            if finished.is_some() {
                return true;
            }
            match task.receiver.try_recv() {
                Ok(prepared) => {
                    finished = Some((task.client, task.appearance.clone(), prepared));
                    false
                }
                Err(TryRecvError::Empty) => true,
                Err(TryRecvError::Disconnected) => {
                    finished = Some((
                        task.client,
                        task.appearance.clone(),
                        Err("player model loader stopped".to_owned()),
                    ));
                    false
                }
            }
        });
        finished
    }
}

/// The worker: the model (its stand-in when it cannot be loaded), its mesh and its
/// textures.
fn prepare(request: Request, sources: Sources) -> Prepared {
    let Sources {
        vfs,
        shaders,
        lightmap,
        mut cache,
        mipmapped,
    } = sources;
    let mut load = |appearance: &Appearance| {
        load_player_appearance_with(
            &vfs,
            &appearance.model,
            &appearance.variant,
            [0.0; 3],
            0.0,
            &mut cache,
        )
        .map_err(|error| error.to_string())
    };
    let kyle = crate::actor_load::fallback_appearance();
    let (preview, failed) = match (!request.known_failed).then(|| load(&request.appearance)) {
        Some(Ok(preview)) => (preview, None),
        outcome => {
            let preview = match load(&request.stand_in) {
                Err(_) if request.stand_in != kyle => load(&kyle)?,
                loaded => loaded?,
            };
            (preview, outcome.and_then(Result::err))
        }
    };
    // As the world load pairs each player with one (`actor_load.rs`).
    let body = request
        .pool_body
        .then(|| {
            let mut body_scene = FlattenedScene::default();
            build_actor_mesh(
                &mut body_scene,
                preview.clone(),
                None,
                true,
                request.appearance.clone(),
                request.saber_names.clone(),
            )
            .ok()
            .map(|mesh| (mesh, body_scene))
        })
        .flatten();
    let mut scene = FlattenedScene::default();
    let mesh = build_actor_mesh(
        &mut scene,
        preview,
        Some(request.entity_id),
        false,
        request.appearance,
        request.saber_names,
    )
    .map_err(|error| error.to_string())?;
    let textures = crate::world_materials::warm_entity_materials(
        &vfs,
        &shaders,
        &lightmap,
        &scene.materials,
        mipmapped,
    );
    Ok(Loaded {
        mesh,
        scene,
        cache,
        failed,
        textures,
        body,
    })
}

impl GpuState {
    /// What the workers read from this world.
    pub(crate) fn player_load_sources(&self) -> Option<Sources> {
        Some(Sources {
            vfs: self.vfs.clone()?,
            shaders: Arc::clone(&self.shaders),
            lightmap: self.world_materials.entity_lightmap(),
            cache: self.clientinfo_watch.gla_cache.clone(),
            mipmapped: self.world_materials.mipmapped(),
        })
    }

    /// Once a frame: start waiting loads, put a model whose pipelines are ready on its
    /// player, and take the model being uploaded one step: one texture, or its materials
    /// and geometry once they are all up. Only then is the next finished model taken.
    pub(crate) fn poll_player_loads(&mut self) {
        let mut loads = std::mem::take(&mut self.clientinfo_watch.loads);
        loads.start(|| self.player_load_sources());
        let uploading = loads.uploading.take();
        let finished = if uploading.is_none() && loads.body.is_none() {
            loads.take_finished()
        } else {
            None
        };
        let compiled = loads.take_compiled();
        self.clientinfo_watch.loads = loads;
        if let Some((compiling, pipelines)) = compiled {
            if let Some(pipelines) = pipelines {
                self.world_materials.install_pipelines(pipelines);
            }
            if let Some(mesh) = compiling.mesh {
                self.place_client_mesh(compiling.client, mesh, &compiling.appearance);
            }
        }
        if uploading.is_none()
            && let Some(body) = self.clientinfo_watch.loads.body.take()
        {
            self.install_body_copy(body);
            return;
        }
        if let Some(mut uploading) = uploading {
            // A texture already up (a model shown again) costs no frame of its own.
            while let Some(texture) = uploading.loaded.textures.pop() {
                if self
                    .world_materials
                    .upload_prepared_texture(&self.device, &self.queue, texture)
                {
                    self.clientinfo_watch.loads.uploading = Some(uploading);
                    return;
                }
            }
            self.install_player_model(uploading.client, uploading.appearance, uploading.loaded);
            return;
        }
        let Some((client, appearance, prepared)) = finished else {
            return;
        };
        let mut loaded = match prepared {
            Ok(loaded) => loaded,
            Err(error) => {
                log::progress(format_args!(
                    "client {client}: could not load {}/{}: {error}",
                    appearance.model, appearance.variant
                ));
                self.clientinfo_watch.failed.insert(appearance);
                return;
            }
        };
        self.clientinfo_watch
            .gla_cache
            .absorb(std::mem::take(&mut loaded.cache));
        if let Some(error) = &loaded.failed {
            log::progress(format_args!(
                "could not load {}/{}: {error}; using Kyle",
                appearance.model, appearance.variant
            ));
            self.clientinfo_watch.failed.insert(appearance.clone());
        }
        self.clientinfo_watch.loads.uploading = Some(Uploading {
            client,
            appearance,
            loaded,
        });
    }

    /// Put a model whose textures are all up on its player: its materials and geometry
    /// now, its new pipelines on a worker first.
    fn install_player_model(&mut self, client: u16, appearance: Appearance, loaded: Loaded) {
        let (mesh, jobs, material_base) =
            match self.upload_built_actor_with_base(loaded.mesh, loaded.scene, &appearance) {
                Ok(uploaded) => uploaded,
                Err(error) => {
                    log::progress(format_args!(
                        "client {client}: model upload failed: {error}"
                    ));
                    return;
                }
            };
        // One body copy per model in the pool is enough: deaths reuse it.
        let pooled = self
            .actor_meshes
            .iter()
            .any(|mesh| mesh.corpse_pool && mesh.appearance == appearance);
        if let Some((body, scene)) = loaded.body.filter(|_| !pooled) {
            self.clientinfo_watch.loads.body = Some(Body {
                appearance: appearance.clone(),
                mesh: body,
                scene,
                material_base,
            });
        }
        if jobs.is_empty() {
            self.place_client_mesh(client, mesh, &appearance);
            return;
        }
        let pipelines = jobs.len();
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("sjk-model-pipelines".into())
            .spawn(move || {
                let _ = sender.send(jobs.compile());
            });
        if let Err(error) = spawned {
            // The slots compile on the model's first draw instead.
            log::progress(format_args!("model pipeline compiler: {error}"));
            self.place_client_mesh(client, mesh, &appearance);
            return;
        }
        log::progress(format_args!(
            "client {client}: compiling {pipelines} pipelines for {}/{} off the render thread",
            appearance.model, appearance.variant
        ));
        self.clientinfo_watch.loads.compiling.push(Compiling {
            client,
            appearance,
            mesh: Some(mesh),
            receiver,
        });
    }
}

impl GpuState {
    /// Put a model's body copy in the body pool, drawing with the model's materials, so
    /// a death takes it rather than building one ([`Self::apply_body_commands`]).
    fn install_body_copy(&mut self, body: Body) {
        if self
            .actor_meshes
            .iter()
            .any(|mesh| mesh.corpse_pool && mesh.appearance == body.appearance)
        {
            return;
        }
        match self.place_actor_geometry(
            body.mesh,
            body.scene,
            body.material_base,
            &body.appearance,
            ("body copy", std::time::Duration::ZERO),
        ) {
            Ok(mut mesh) => {
                mesh.corpse_pool = true;
                mesh.entity_id = None;
                self.actor_meshes.push(mesh);
                self.actor_groups.push(Vec::with_capacity(4));
            }
            Err(error) => log::progress(format_args!(
                "body copy of {}/{} not prepared: {error}",
                body.appearance.model, body.appearance.variant
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn appearance(model: &str) -> Appearance {
        Appearance {
            model: model.to_owned(),
            variant: "default".to_owned(),
        }
    }

    fn request(client: u16, model: &str) -> Request {
        Request {
            client,
            appearance: appearance(model),
            entity_id: EntityId::new(u64::from(client) + 1),
            saber_names: [None, None],
            known_failed: false,
            stand_in: crate::actor_load::fallback_appearance(),
            pool_body: true,
        }
    }

    #[test]
    fn a_new_request_replaces_the_players_earlier_one() {
        let mut loads = Loads::default();
        loads.request(request(3, "models/players/a"));
        loads.request(request(4, "models/players/b"));
        assert!(loads.pending(3, &appearance("models/players/a")));
        loads.request(request(3, "models/players/c"));
        assert!(!loads.pending(3, &appearance("models/players/a")));
        assert!(loads.pending(3, &appearance("models/players/c")));
        assert!(loads.pending(4, &appearance("models/players/b")));
        // The same request again changes nothing.
        loads.request(request(3, "models/players/c"));
        assert_eq!(loads.waiting.len(), 2);
        loads.cancel(4);
        assert!(!loads.pending(4, &appearance("models/players/b")));
    }

    #[test]
    fn finished_loads_come_out_one_at_a_time_and_cancelled_ones_never() {
        let mut loads = Loads::default();
        let mut senders = Vec::new();
        for client in 0..3_u16 {
            let (sender, receiver) = mpsc::channel();
            senders.push(sender);
            loads.running.push(Task {
                client,
                appearance: appearance("models/players/kyle"),
                receiver,
            });
        }
        assert!(loads.take_finished().is_none());
        loads.cancel(1);
        for sender in &senders {
            let _ = sender.send(Err("test".to_owned()));
        }
        assert_eq!(loads.take_finished().map(|(client, ..)| client), Some(0));
        assert_eq!(loads.take_finished().map(|(client, ..)| client), Some(2));
        assert!(loads.take_finished().is_none());
        // A worker that stopped without an answer is reported, not waited for.
        let (sender, receiver) = mpsc::channel::<Prepared>();
        drop(sender);
        loads.running.push(Task {
            client: 5,
            appearance: appearance("models/players/kyle"),
            receiver,
        });
        assert!(matches!(loads.take_finished(), Some((5, _, Err(_)))));
    }

    #[test]
    fn no_more_than_the_limit_start_without_sources() {
        let mut loads = Loads::default();
        for client in 0..5 {
            loads.request(request(client, "models/players/kyle"));
        }
        // Without a world to read from nothing starts and nothing is lost.
        loads.start(|| None);
        assert!(loads.running.is_empty());
        assert_eq!(loads.waiting.len(), 5);
    }
}
