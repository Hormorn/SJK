//! The stage's model loaded off the render thread.
//!
//! Choosing a model in the model picker built it inside one frame: the `.glm`, its
//! skeleton and skin read and parsed (a rig model's borrowed emotes with them), its
//! textures decoded and their mips made, then uploaded, and the pipelines it brought
//! compiled. Turin's log (04/11/2026) has frames of 100 to 485 ms while he scrolled
//! models in a match (`view+menu+audio` in its `hitch:` lines), and no command leaves
//! the client during such a frame, which a server sees as commands growing old
//! ([`crate::server_view`], JoF's Heimdall kicks). As the players' own models do
//! ([`crate::player_loads`]), a worker now reads the model, skins its first pose and
//! decodes its textures ([`crate::world_materials::warm_entity_materials`]); the render
//! thread uploads one texture a frame, then makes the actor's materials and buffers,
//! and another worker compiles its new pipelines. The stage shows the model it had
//! until the new one is ready. One worker reads at a time: scrolling past models loads
//! the one the picker stops on, not every one it passed. A windowless client (world
//! shots) waits for each step within the frame, so its pictures stay exact.

use super::*;
use crate::world_materials::pipeline_jobs::Compiled;
use std::sync::mpsc::{self, Receiver, TryRecvError};

/// What a worker read for one model.
pub(super) struct Read {
    pub(super) preview: PlayerPreview,
    /// Its mesh in the pose it was read in, and its materials.
    pub(super) scene: FlattenedScene,
    pub(super) draws: Vec<ActorDraw>,
    pub(super) vertex_ranges: Vec<PreviewVertexRange>,
    /// The skeletons it parsed, kept for the next loads.
    cache: GlaCache,
    /// Its textures' mip chains, built there.
    textures: crate::world_materials::WarmTextures,
}

/// What the worker reads from.
struct Sources {
    vfs: Arc<VirtualFileSystem>,
    shaders: Arc<ShaderCatalog>,
    lightmap: wgpu::TextureView,
    mipmapped: bool,
    cache: GlaCache,
}

/// A model being read, by the `model` cvar value it is for.
struct Reading {
    model: String,
    receiver: Receiver<Result<Read, String>>,
}

/// A built actor waiting for the pipelines it brought.
struct Compiling {
    actor: StageActor,
    receiver: Receiver<Compiled>,
}

/// The stage model on its way, at most one step of each kind.
#[derive(Default)]
pub(super) struct Load {
    reading: Option<Reading>,
    uploading: Option<(String, Read)>,
    compiling: Option<Compiling>,
}

/// What the reading worker has for the wanted model.
enum Arrival {
    /// A worker still reads (the wanted model or, one at a time, another).
    Running,
    Read(Box<Read>),
    Failed(String),
    /// Nothing reads the wanted model: start it.
    Start,
}

impl Load {
    /// Take what the reading worker has for `model`, waiting for it when `blocking`.
    /// What another model's worker read is dropped once it is done, its skeletons kept
    /// in `cache`; a failure is only the wanted model's.
    fn arrival(&mut self, model: &str, blocking: bool, cache: &mut GlaCache) -> Arrival {
        let Some(reading) = self.reading.take() else {
            return Arrival::Start;
        };
        let result = if blocking {
            reading.receiver.recv().ok()
        } else {
            match reading.receiver.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => {
                    self.reading = Some(reading);
                    return Arrival::Running;
                }
                Err(TryRecvError::Disconnected) => None,
            }
        };
        let wanted = reading.model == model;
        match result {
            Some(Ok(mut read)) => {
                cache.absorb(std::mem::take(&mut read.cache));
                if wanted {
                    Arrival::Read(Box::new(read))
                } else {
                    Arrival::Start
                }
            }
            Some(Err(error)) if wanted => Arrival::Failed(error),
            None if wanted => Arrival::Failed("the model loader stopped".to_owned()),
            _ => Arrival::Start,
        }
    }
}

/// Where a frame left the load of the wanted model.
pub(super) enum Step {
    /// Still being read, uploaded or compiled: keep showing what the stage has.
    Waiting,
    Ready(Box<StageActor>),
    Failed(String),
}

/// The worker: the model, its first pose and its textures, in `stance` when it has it.
fn read(model: &str, stance: &'static str, sources: Sources) -> Result<Read, String> {
    let Sources {
        vfs,
        shaders,
        lightmap,
        mipmapped,
        mut cache,
    } = sources;
    let (directory, variant) = split_model_cvar(model);
    let mut preview =
        load_player_appearance_with(&vfs, &directory, variant, [0.0; 3], 0.0, &mut cache)
            .map_err(|error| error.to_string())?;
    preview.origin = [0.0; 3];
    preview.yaw = 0.0;
    if let Some(sequence) = preview.config.get(stance).cloned() {
        preview.sequence = sequence;
    }
    let frame = animation_frame(&preview.sequence, 0.0);
    let mut scene = FlattenedScene::default();
    let (draws, vertex_ranges) =
        append_actor_mesh(&mut scene, &preview, frame).map_err(|error| error.to_string())?;
    let textures = crate::world_materials::warm_entity_materials(
        &vfs,
        &shaders,
        &lightmap,
        &scene.materials,
        mipmapped,
    );
    Ok(Read {
        preview,
        scene,
        draws,
        vertex_ranges,
        cache,
        textures,
    })
}

impl GpuState {
    /// Take the load of `model` one step this frame: start reading it, upload one of its
    /// textures, build its actor or install its pipelines. A load for another model is
    /// dropped once its worker is done (one worker reads at a time).
    pub(super) fn step_stage_load(
        &mut self,
        stage: Option<Stage>,
        model: &str,
        now: Instant,
    ) -> Step {
        // A windowless client takes every step in this frame.
        let blocking = self.headless_frame.is_some();
        let mut load = std::mem::take(&mut self.menu_stage.load);
        let step = loop {
            let step = self.step_load(&mut load, stage, model, now, blocking);
            if !blocking || !matches!(step, Step::Waiting) {
                break step;
            }
        };
        self.menu_stage.load = load;
        step
    }

    fn step_load(
        &mut self,
        load: &mut Load,
        stage: Option<Stage>,
        model: &str,
        now: Instant,
        blocking: bool,
    ) -> Step {
        if let Some(compiling) = load.compiling.take()
            && compiling.actor.model == model
        {
            let compiled = if blocking {
                compiling.receiver.recv().ok()
            } else {
                match compiling.receiver.try_recv() {
                    Ok(compiled) => Some(compiled),
                    Err(TryRecvError::Empty) => {
                        load.compiling = Some(compiling);
                        return Step::Waiting;
                    }
                    // The slots compile on the first draw instead.
                    Err(TryRecvError::Disconnected) => None,
                }
            };
            if let Some(compiled) = compiled {
                self.world_materials.install_pipelines(compiled);
            }
            return Step::Ready(Box::new(compiling.actor));
        }
        if let Some((wanted, mut read)) = load.uploading.take()
            && wanted == model
        {
            // A texture already up (a model shown again) costs no frame of its own.
            while let Some(texture) = read.textures.pop() {
                if self
                    .world_materials
                    .upload_prepared_texture(&self.device, &self.queue, texture)
                    && !blocking
                {
                    load.uploading = Some((wanted, read));
                    return Step::Waiting;
                }
            }
            return self.finish_stage_load(load, stage, model, read, now, blocking);
        }
        match load.arrival(model, blocking, &mut self.menu_stage.gla_cache) {
            Arrival::Running => return Step::Waiting,
            Arrival::Read(read) => {
                load.uploading = Some((model.to_owned(), *read));
                return Step::Waiting;
            }
            Arrival::Failed(error) => return Step::Failed(error),
            Arrival::Start => {}
        }
        let Some(vfs) = self.vfs.clone() else {
            return Step::Failed("player stage has no VFS".to_owned());
        };
        let sources = Sources {
            vfs,
            shaders: Arc::clone(&self.shaders),
            lightmap: self.world_materials.entity_lightmap(),
            mipmapped: self.world_materials.mipmapped(),
            cache: self.menu_stage.gla_cache.clone(),
        };
        let stance = self
            .menu_stage
            .actor
            .as_ref()
            .map_or(DEFAULT_STANCE, |actor| actor.stance);
        let (sender, receiver) = mpsc::channel();
        let owned = model.to_owned();
        let spawned = std::thread::Builder::new()
            .name("sjk-stage-model".into())
            .spawn(move || {
                let _ = sender.send(read(&owned, stance, sources));
            });
        match spawned {
            Ok(_) => {
                load.reading = Some(Reading {
                    model: model.to_owned(),
                    receiver,
                });
                Step::Waiting
            }
            Err(error) => Step::Failed(format!("the model loader: {error}")),
        }
    }

    /// The textures of `read` are up: build the actor, and compile the pipelines it
    /// brought on a worker before it is shown.
    fn finish_stage_load(
        &mut self,
        load: &mut Load,
        stage: Option<Stage>,
        model: &str,
        read: Read,
        now: Instant,
        blocking: bool,
    ) -> Step {
        // A card out of video memory loses this model, not the game (a 3 GB card
        // browsing the creator's legs, 21/10/2026); the textures it uploaded go with it.
        let kept = self.world_materials.texture_keys();
        let device = self.device.clone();
        let built =
            crate::gpu_guard::capture(&device, || self.build_stage_actor(stage, model, read, now))
                .unwrap_or_else(|error| {
                    self.world_materials.forget_textures_since(&kept);
                    Err(format!("the graphics card: {error}").into())
                });
        let (actor, jobs) = match built {
            Ok(built) => built,
            Err(error) => return Step::Failed(error.to_string()),
        };
        if jobs.is_empty() {
            return Step::Ready(Box::new(actor));
        }
        if blocking {
            self.world_materials.install_pipelines(jobs.compile());
            return Step::Ready(Box::new(actor));
        }
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("sjk-stage-pipelines".into())
            .spawn(move || {
                let _ = sender.send(jobs.compile());
            });
        if let Err(error) = spawned {
            // The slots compile on the first draw instead.
            crate::log::progress(format_args!("stage pipeline compiler: {error}"));
            return Step::Ready(Box::new(actor));
        }
        load.compiling = Some(Compiling { actor, receiver });
        Step::Waiting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reading(model: &str) -> (Load, mpsc::Sender<Result<Read, String>>) {
        let (sender, receiver) = mpsc::channel();
        let load = Load {
            reading: Some(Reading {
                model: model.to_owned(),
                receiver,
            }),
            ..Default::default()
        };
        (load, sender)
    }

    #[test]
    fn nothing_reading_starts_the_wanted_model() {
        let mut load = Load::default();
        let mut cache = GlaCache::default();
        assert!(matches!(
            load.arrival("kyle/default", false, &mut cache),
            Arrival::Start
        ));
    }

    #[test]
    fn one_worker_reads_at_a_time_and_another_models_read_is_dropped() {
        let mut cache = GlaCache::default();
        // The picker moved on while the first model is read: wait for that worker.
        let (mut load, sender) = reading("models/players/a");
        assert!(matches!(
            load.arrival("models/players/b", false, &mut cache),
            Arrival::Running
        ));
        assert!(load.reading.is_some());
        // Once it is done, its result (even a failure) is not the wanted model's.
        sender.send(Err("broken".to_owned())).unwrap();
        assert!(matches!(
            load.arrival("models/players/b", false, &mut cache),
            Arrival::Start
        ));
        assert!(load.reading.is_none());
    }

    #[test]
    fn the_wanted_models_failure_is_reported() {
        let mut cache = GlaCache::default();
        let (mut load, sender) = reading("models/players/a");
        sender.send(Err("no model.glm".to_owned())).unwrap();
        assert!(matches!(
            load.arrival("models/players/a", false, &mut cache),
            Arrival::Failed(error) if error == "no model.glm"
        ));
        // A worker that stopped without an answer is a failure too, not a wait.
        let (mut load, sender) = reading("models/players/a");
        drop(sender);
        assert!(matches!(
            load.arrival("models/players/a", true, &mut cache),
            Arrival::Failed(_)
        ));
        let (mut load, sender) = reading("models/players/a");
        drop(sender);
        assert!(matches!(
            load.arrival("models/players/b", false, &mut cache),
            Arrival::Start
        ));
    }
}
