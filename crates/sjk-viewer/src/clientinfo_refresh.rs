//! In-match reaction to clientinfo changes — JKA's `CG_NewClientInfo`
//! (`codemp/cgame/cg_players.c`, called from `CG_ConfigStringModified` in
//! `cg_servercmds.c:894-896` whenever a `CS_PLAYERS` string changes).
//!
//! The server rewrites a player's clientinfo when their model changes
//! (`ClientUserinfoChanged`, at once) or their sabers change (`ClientSpawn`,
//! on the next spawn). The load-time actor meshes only know the roster of
//! the first gamestate, so a player who changes model kept the old mesh and
//! a saber nobody carried at load fell back to `single_1`. This module
//! receives dirty client indices from the session and rebuilds that player's
//! mesh and missing hilts on a change, appending
//! the geometry and materials to the shared buffers. The model itself is read on
//! a worker ([`player_loads`]); the slot keeps its mesh until the new one is in.

use super::*;
use crate::actor_load::{build_actor_mesh, client_saber_names};
use crate::player_assets::{GlaCache, load_player_appearance_with};
use crate::shared_geometry::relocate;
use crate::world_materials::pipeline_jobs::{Compiled, Jobs as PipelineJobs};
use sjk_client::legacy_client_appearance_with_policy;

#[path = "corpse_actors.rs"]
mod corpse_actors;
#[path = "clientinfo_forcing.rs"]
mod forcing;
#[path = "player_loads.rs"]
pub(crate) mod player_loads;

/// A model install on the render thread this long (seconds) is logged with its parts.
const INSTALL_REPORT: f64 = 0.004;

/// Per-world appearance loading caches used by dirty clientinfo notifications.
pub(crate) struct ClientInfoWatch {
    pub(crate) body_commands: Vec<sjk_client::BaseServerCommandEvent>,
    pub(crate) gla_cache: GlaCache,
    pub(crate) model_policy: sjk_client::player_model_eligibility::PlayerModelEligibility,
    model_blacklist: String,
    respect_model_metadata: bool,
    forced: bool,
    forced_model: String,
    /// `cg_defaultModelRandom` as last applied.
    random_stand_ins: bool,
    overrides: [String; 3],
    local_identity: (i32, i32),
    /// Appearances whose load failed; players wearing one show their stand-in
    /// (Kyle, or a retail model with `cg_defaultModelRandom`), and the load is not
    /// retried in this world.
    pub(crate) failed: BTreeSet<Appearance>,
    /// Model folders `rig_reload` reads again: a player already wearing one loads it
    /// anew instead of keeping the mesh up.
    pub(crate) reload: BTreeSet<String>,
    /// Players' models being read on workers.
    loads: player_loads::Loads,
    /// The NPC model being read on a worker (`npc_loads.rs`).
    pub(crate) npc_loads: crate::npc_refresh::npc_loads::Load,
    /// Each actor material list appended in this world, where it starts and the
    /// pipeline keys it brought: a model installed again (Kyle standing in for every
    /// model this PC lacks, a player who rejoins) draws with those rather than compiling
    /// them again.
    pub(crate) appended: Vec<Appended>,
}

/// One material list [`ClientInfoWatch::appended`] keeps.
pub(crate) struct Appended {
    materials: Vec<ViewerMaterial>,
    base: usize,
    keys: std::ops::Range<usize>,
}

impl ClientInfoWatch {
    pub(crate) fn new(vfs: &VirtualFileSystem) -> Self {
        Self {
            body_commands: Vec::new(),
            gla_cache: GlaCache::default(),
            model_policy: sjk_client::player_model_eligibility::PlayerModelEligibility::load(vfs),
            model_blacklist: String::new(),
            respect_model_metadata: true,
            forced: false,
            forced_model: String::new(),
            random_stand_ins: false,
            overrides: std::array::from_fn(|_| String::new()),
            local_identity: (-1, -1),
            failed: BTreeSet::new(),
            reload: BTreeSet::new(),
            loads: player_loads::Loads::default(),
            npc_loads: Default::default(),
            appended: Vec::new(),
        }
    }
}

impl GpuState {
    /// Rebuild live player meshes only when the forced-model policy changes.
    pub(crate) fn refresh_forced_models(&mut self) {
        let forced = self
            .console
            .as_ref()
            .and_then(|c| c.bool_cvar("cg_forcemodel"))
            .unwrap_or(false);
        let model = self
            .console
            .as_ref()
            .and_then(|c| c.text_cvar("model").ok())
            .unwrap_or("kyle/default");
        let overrides = [
            "cg_forceenemymodel",
            "cg_forceallymodel",
            "cg_forceownsaber",
        ]
        .map(|name| {
            self.console
                .as_ref()
                .and_then(|c| c.text_cvar(name).ok())
                .unwrap_or("none")
        });
        let game = self
            .live_session
            .as_ref()
            .map(ClientSession::game_state)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(demo_playback::Session::game_state)
            });
        let snapshot = self
            .live_session
            .as_ref()
            .map(ClientSession::latest_snapshot)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(demo_playback::Session::latest_snapshot)
            });
        let has_game = game.is_some();
        let local = game.map_or((-1, -1), |game| {
            (
                game.client_num,
                snapshot.map_or(0, |snap| i32::from(snap.player.team())),
            )
        });
        let (blacklist, respect_metadata) = crate::actor_load::model_options(self.console.as_ref());
        let random = crate::actor_load::random_stand_ins(self.console.as_ref());
        let random_changed = self.clientinfo_watch.random_stand_ins != random;
        if !random_changed
            && self.clientinfo_watch.model_blacklist == blacklist
            && self.clientinfo_watch.respect_model_metadata == respect_metadata
            && self.clientinfo_watch.forced == forced
            && (!forced || self.clientinfo_watch.forced_model == model)
            && self
                .clientinfo_watch
                .overrides
                .iter()
                .map(String::as_str)
                .eq(overrides)
            && self.clientinfo_watch.local_identity == local
        {
            return;
        }
        self.clientinfo_watch.model_blacklist.clear();
        self.clientinfo_watch.model_blacklist.push_str(blacklist);
        self.clientinfo_watch.respect_model_metadata = respect_metadata;
        self.clientinfo_watch.forced = forced;
        self.clientinfo_watch.forced_model.clear();
        self.clientinfo_watch.forced_model.push_str(model);
        for (stored, value) in self.clientinfo_watch.overrides.iter_mut().zip(overrides) {
            stored.clear();
            stored.push_str(value);
        }
        self.clientinfo_watch.local_identity = local;
        self.clientinfo_watch.random_stand_ins = random;
        // Without a game state there is no player to rebuild; the first one
        // changes `local` and comes back here.
        if !has_game {
            return;
        }
        // Players drawn with a stand-in take the new one: their slot still carries the
        // appearance they asked for, which an unchanged clientinfo would keep.
        if random_changed {
            let failed = self.clientinfo_watch.failed.iter();
            // Players drawn with a stand-in since the world loaded (`actor_load`) are
            // not in `failed`: a model whose `model.glm` is missing stands in too.
            let vfs = self.vfs.as_ref();
            let missing = self
                .actor_meshes
                .iter()
                .filter(|mesh| !mesh.corpse_pool && mesh.entity_id.is_some())
                .map(|mesh| &mesh.appearance)
                .filter(|appearance| {
                    vfs.is_some_and(|vfs| {
                        !vfs.contains(&format!("{}/model.glm", appearance.model))
                            .unwrap_or(true)
                    })
                });
            let folders = failed
                .chain(missing)
                .map(|appearance| appearance.model.clone())
                .collect();
            self.clientinfo_watch.reload = folders;
        }
        for client in 0..32 {
            if let Err(error) = self.apply_clientinfo(client) {
                log::progress(format_args!("force-model client {client}: {error}"));
            }
        }
        if random_changed {
            self.clientinfo_watch.reload.clear();
        }
    }

    /// What `client` (none for an actor that is no player) is drawn with when
    /// `requested` cannot be loaded: Kyle, or a retail model with `cg_defaultModelRandom`.
    pub(crate) fn stand_in(&self, client: Option<u16>, requested: &Appearance) -> Appearance {
        let game_state = self
            .live_session
            .as_ref()
            .map(ClientSession::game_state)
            .or_else(|| {
                self.demo_session
                    .as_ref()
                    .map(demo_playback::Session::game_state)
            });
        crate::actor_load::stand_in(
            game_state,
            client,
            requested,
            crate::actor_load::random_stand_ins(self.console.as_ref()),
        )
    }

    /// Apply one dirty player slot using the existing actor and hilt caches.
    pub(crate) fn apply_clientinfo(&mut self, client: u16) -> Result<(), Box<dyn Error>> {
        let (appearance, saber_names, body) = {
            let game_state = self
                .live_session
                .as_ref()
                .map(ClientSession::game_state)
                .or_else(|| {
                    self.demo_session
                        .as_ref()
                        .map(demo_playback::Session::game_state)
                })
                .ok_or("no game state")?;
            let forced = self
                .clientinfo_watch
                .forced
                .then_some(self.clientinfo_watch.forced_model.as_str());
            let override_model = forcing::model(
                forced,
                &self.clientinfo_watch.overrides,
                client,
                self.clientinfo_watch.local_identity,
                forcing::team(game_state, client),
            );
            // A player muted on this PC is drawn as Kyle with the default saber
            // (`muted_players.rs`), whatever else forces a model.
            let muted = self.muted_players.contains(client);
            let model = if muted {
                Some(crate::muted_players::MODEL)
            } else {
                override_model.as_deref().or(forced)
            };
            let Some(appearance) =
                legacy_client_appearance_with_policy(game_state, client, model, |appearance| {
                    crate::actor_load::eligible_appearance(
                        appearance,
                        true,
                        &self.clientinfo_watch.model_policy,
                        crate::actor_load::model_options(self.console.as_ref()),
                    )
                })
            else {
                self.clientinfo_watch.loads.cancel(client);
                return Ok(());
            };
            // What a body of this player will be drawn with (`corpse_actors.rs`).
            let body = sjk_client::legacy_client_appearance(game_state, client)
                .map(|own| self.body_appearance(client, &own));
            let mut names = client_saber_names(Some(game_state), client);
            if i32::from(client) == game_state.client_num {
                forcing::sabers(&mut names, &self.clientinfo_watch.overrides[2]);
            }
            if muted {
                let hilts = self.saber_hilts.as_ref();
                crate::muted_players::sabers(&mut names, |name| {
                    hilts.is_some_and(|catalog| catalog.is_staff(name))
                });
                (crate::muted_players::appearance(&appearance), names, body)
            } else {
                (appearance, names, body)
            }
        };
        for name in saber_names.iter().flatten() {
            self.load_hilt(name)?;
        }
        let entity_id = EntityId::new(u64::from(client) + 1);
        let index = self
            .actor_meshes
            .iter()
            .position(|mesh| !mesh.corpse_pool && mesh.entity_id == Some(entity_id));
        if let Some(index) = index {
            self.actor_meshes[index].saber_names = saber_names.clone();
            self.actor_meshes[index].holsters.invalidate();
            // `c1`/`c2` may name another hat or cape (JoF EJK).
            self.actor_meshes[index].cosmetics.invalidate();
            if self.actor_meshes[index].appearance == appearance
                && !self.clientinfo_watch.reload.contains(&appearance.model)
            {
                self.clientinfo_watch.loads.cancel(client);
                return Ok(());
            }
        }
        let known_failed = self.clientinfo_watch.failed.contains(&appearance);
        let stand_in = self.stand_in(Some(client), &appearance);
        // A forced or overridden model's copy would never be taken by a death.
        let pool_body = body.as_ref() == Some(&appearance);
        self.clientinfo_watch.loads.request(player_loads::Request {
            client,
            appearance,
            entity_id,
            saber_names,
            known_failed,
            stand_in,
            pool_body,
        });
        Ok(())
    }

    /// Put `mesh`, just uploaded, on `client`'s slot.
    pub(crate) fn place_client_mesh(
        &mut self,
        client: u16,
        mesh: ActorMesh,
        appearance: &Appearance,
    ) {
        let entity_id = EntityId::new(u64::from(client) + 1);
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
        let wears = if self.clientinfo_watch.failed.contains(appearance) {
            "a stand-in in place of"
        } else {
            "now wears"
        };
        log::progress(format_args!(
            "client {client} {wears} {}/{}",
            appearance.model, appearance.variant
        ));
    }

    /// [`Self::build_live_actor`], or `stand_in` (Kyle, or a retail model with
    /// `cg_defaultModelRandom`) for an appearance that cannot be loaded here.
    ///
    /// `CG_LoadClientInfo` (`codemp/cgame/cg_players.c`) registers
    /// `DEFAULT_MODEL` when a player's model fails, so a model this client lacks
    /// does not leave the slot showing its previous model. The stand-in carries
    /// the requested appearance, as at world load (`actor_load`), so an
    /// unchanged clientinfo does not rebuild it and a body copy of that player
    /// reuses it. A failed appearance is only tried once per world.
    pub(crate) fn build_live_actor_or_stand_in(
        &mut self,
        appearance: &Appearance,
        stand_in: &Appearance,
        entity_id: EntityId,
        saber_names: [Option<String>; 2],
    ) -> Result<ActorMesh, Box<dyn Error>> {
        if !self.clientinfo_watch.failed.contains(appearance) {
            match self.build_live_actor(appearance, entity_id, saber_names.clone()) {
                Ok(mesh) => return Ok(mesh),
                Err(error) => {
                    log::progress(format_args!(
                        "could not load {}/{}: {error}; using {}/{}",
                        appearance.model, appearance.variant, stand_in.model, stand_in.variant
                    ));
                    self.clientinfo_watch.failed.insert(appearance.clone());
                }
            }
        }
        let kyle = crate::actor_load::fallback_appearance();
        let vfs = self.vfs.clone().ok_or("no VFS")?;
        let preview = crate::actor_load::load_stand_in(
            &vfs,
            stand_in,
            &kyle,
            &mut self.clientinfo_watch.gla_cache,
        )?;
        self.upload_actor(preview, appearance, entity_id, saber_names)
    }

    /// Load `appearance` for `entity_id` and append it to the shared buffers.
    pub(crate) fn build_live_actor(
        &mut self,
        appearance: &Appearance,
        entity_id: EntityId,
        saber_names: [Option<String>; 2],
    ) -> Result<ActorMesh, Box<dyn Error>> {
        let vfs = self.vfs.clone().ok_or("no VFS")?;
        let preview = load_player_appearance_with(
            &vfs,
            &appearance.model,
            &appearance.variant,
            [0.0; 3],
            0.0,
            &mut self.clientinfo_watch.gla_cache,
        )?;
        self.upload_actor(preview, appearance, entity_id, saber_names)
    }

    /// Upload a captured preview into independent pose storage and vertex ranges.
    pub(crate) fn upload_actor(
        &mut self,
        preview: PlayerPreview,
        appearance: &Appearance,
        entity_id: EntityId,
        saber_names: [Option<String>; 2],
    ) -> Result<ActorMesh, Box<dyn Error>> {
        let mut scene = FlattenedScene::default();
        let mesh = build_actor_mesh(
            &mut scene,
            preview,
            Some(entity_id),
            false,
            appearance.clone(),
            saber_names,
        )?;
        self.upload_built_actor(mesh, scene, appearance)
    }

    /// Upload a mesh built from a preview ([`build_actor_mesh`]): its materials, its
    /// geometry into the shared buffers and its GPU skin; its new pipelines compile now.
    pub(crate) fn upload_built_actor(
        &mut self,
        mesh: ActorMesh,
        scene: FlattenedScene,
        appearance: &Appearance,
    ) -> Result<ActorMesh, Box<dyn Error>> {
        let (mesh, jobs) = self.upload_built_actor_deferred(mesh, scene, appearance)?;
        self.world_materials.install_pipelines(jobs.compile());
        Ok(mesh)
    }

    /// [`Self::upload_built_actor`], leaving its new pipelines to the returned jobs: the
    /// mesh must not be drawn before they are installed, or its first draw compiles them.
    pub(crate) fn upload_built_actor_deferred(
        &mut self,
        mesh: ActorMesh,
        scene: FlattenedScene,
        appearance: &Appearance,
    ) -> Result<(ActorMesh, PipelineJobs), Box<dyn Error>> {
        self.upload_built_actor_with_base(mesh, scene, appearance)
            .map(|(mesh, jobs, _)| (mesh, jobs))
    }

    /// [`Self::upload_built_actor_deferred`], with where its materials start: a copy of
    /// the same model ([`Self::place_actor_geometry`]) draws with them.
    pub(crate) fn upload_built_actor_with_base(
        &mut self,
        mesh: ActorMesh,
        scene: FlattenedScene,
        appearance: &Appearance,
    ) -> Result<(ActorMesh, PipelineJobs, usize), Box<dyn Error>> {
        let started = Instant::now();
        let appended = self
            .clientinfo_watch
            .appended
            .iter()
            .find(|appended| appended.materials == scene.materials);
        let (material_base, jobs) = match appended {
            // Still compiling for the first install, they are compiled for this one too.
            Some(appended) => (
                appended.base,
                self.world_materials.reused_pipeline_jobs(
                    appended.keys.clone(),
                    appended.base,
                    appended.materials.len(),
                ),
            ),
            None => {
                let vfs = self.vfs.clone().ok_or("no VFS")?;
                let keys = self.world_materials.pipeline_key_count();
                let (base, jobs) = self.world_materials.append_entity_materials_deferred(
                    &self.device,
                    &self.queue,
                    &vfs,
                    &self.shaders,
                    &scene.materials,
                )?;
                self.clientinfo_watch.appended.push(Appended {
                    materials: scene.materials.clone(),
                    base,
                    keys: keys..self.world_materials.pipeline_key_count(),
                });
                (base, jobs)
            }
        };
        let materials = started.elapsed();
        let mesh = self.place_actor_geometry(
            mesh,
            scene,
            material_base,
            appearance,
            ("model install", materials),
        )?;
        Ok((mesh, jobs, material_base))
    }

    /// Write `mesh`'s geometry into the shared buffers and its GPU skin, drawing with the
    /// materials from `material_base` on. An install that takes long says where its time
    /// went, named `what` ("model install", "body copy") with the time its materials
    /// took before, for the hitch lines it causes.
    pub(crate) fn place_actor_geometry(
        &mut self,
        mut mesh: ActorMesh,
        scene: FlattenedScene,
        material_base: usize,
        appearance: &Appearance,
        (what, materials): (&str, std::time::Duration),
    ) -> Result<ActorMesh, Box<dyn Error>> {
        let started = Instant::now();
        let placement =
            self.geometry
                .append(&self.device, &self.queue, &scene.vertices, &scene.indices)?;
        self.world_materials.bind_geometry(&self.geometry);
        relocate(
            &mut mesh.draws,
            &mut mesh.vertex_ranges,
            placement,
            material_base,
        );
        let geometry = started.elapsed();
        // Palette-skin the new mesh like the actors known at world load; on a topology
        // mismatch this one mesh keeps the CPU path.
        match self
            .geometry
            .append_actor_skin(&self.device, &self.queue, &mut mesh)
        {
            Ok(true) => self.world_materials.bind_geometry(&self.geometry),
            Ok(false) => {}
            Err(error) => log::progress(format_args!(
                "GPU skinning unavailable for {}: {error}",
                appearance.model
            )),
        }
        let skin = started.elapsed() - geometry;
        let total = materials + started.elapsed();
        if total.as_secs_f64() >= INSTALL_REPORT {
            log::progress(format_args!(
                "{what} {}/{}: {:.1} ms (materials {:.1}, geometry {:.1} for {} vertices, \
                 skin {:.1})",
                appearance.model,
                appearance.variant,
                total.as_secs_f64() * 1e3,
                materials.as_secs_f64() * 1e3,
                geometry.as_secs_f64() * 1e3,
                scene.vertices.len(),
                skin.as_secs_f64() * 1e3
            ));
        }
        Ok(mesh)
    }

    /// Make sure the hilt catalog can answer for `name`, loading the `.sab`
    /// model into the shared buffers if it never was.
    pub(crate) fn load_hilt(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        let Some(catalog) = &mut self.saber_hilts else {
            return Ok(());
        };
        if catalog.is_resolved(name) {
            return Ok(());
        }
        let vfs = self.vfs.clone().ok_or("no VFS")?;
        let mut scene = FlattenedScene::default();
        if !catalog.load(&vfs, name, &mut scene, &mut self.object_meshes) {
            return Ok(());
        }
        // The catalog appended one object mesh; give it a group slot even if
        // the upload fails, so `mesh_index` stays valid (an empty draw list
        // is an invisible hilt, not a crash).
        self.object_groups
            .push(crate::saber_hilts::instance_group());
        let materials = &scene.materials;
        let uploaded = self
            .world_materials
            .append_entity_materials(&self.device, &self.queue, &vfs, &self.shaders, materials)
            .and_then(|material_base| {
                let placement = self.geometry.append(
                    &self.device,
                    &self.queue,
                    &scene.vertices,
                    &scene.indices,
                )?;
                self.world_materials.bind_geometry(&self.geometry);
                Ok((placement, material_base))
            });
        let mesh = self.object_meshes.last_mut().ok_or("hilt mesh vanished")?;
        match uploaded {
            Ok((placement, material_base)) => {
                relocate(&mut mesh.draws, &mut [], placement, material_base);
                log::progress(format_args!("loaded saber hilt {name} mid-match"));
                Ok(())
            }
            Err(error) => {
                mesh.draws.clear();
                Err(error)
            }
        }
    }
}
