//! Load-time actor meshes: one deformable mesh per advertised client (plus a
//! corpse-pool twin per appearance) and the shared Kyle fallback, appended to
//! the scene the viewer uploads once. [`build_actor_mesh`] is also how the
//! in-match clientinfo refresh ([`crate::clientinfo_refresh`]) builds a
//! replacement mesh for a player who changed model.

use super::*;
use sjk_client::{legacy_client_appearance_with_policy, legacy_client_saber_names};

/// Player loading/picker options; normalized lookups do not allocate on idle frames.
pub(crate) fn model_options(console: Option<&crate::console::ViewerConsole>) -> (&str, bool) {
    (
        console
            .and_then(|c| c.text_value("cg_modelblacklist"))
            .unwrap_or(""),
        console
            .and_then(|c| c.bool_cvar("cg_modeleligibility"))
            .unwrap_or(true),
    )
}

/// Apply player eligibility after model overrides, leaving NPCs and archived choices intact.
pub(crate) fn eligible_appearance(
    appearance: Appearance,
    player: bool,
    eligibility: &sjk_client::player_model_eligibility::PlayerModelEligibility,
    options: (&str, bool),
) -> Appearance {
    if player && !eligibility.allows(&appearance.model, options.0, options.1) {
        fallback_appearance()
    } else {
        appearance
    }
}

/// The player everyone falls back to when their model cannot be loaded.
pub(crate) fn fallback_appearance() -> Appearance {
    Appearance {
        model: "models/players/kyle".to_owned(),
        variant: "default".to_owned(),
    }
}

/// Saber names advertised for `client_num`, lower-cased for the hilt catalog;
/// `single_1` alone when the game state does not say.
pub(crate) fn client_saber_names(
    game_state: Option<&GameState>,
    client_num: u16,
) -> [Option<String>; 2] {
    game_state
        .map(|game_state| {
            legacy_client_saber_names(game_state, client_num)
                .map(|name| name.map(|name| name.to_ascii_lowercase()))
        })
        .unwrap_or_else(|| [Some("single_1".to_owned()), None])
}

/// Append `preview`'s bind-pose surfaces to `scene` and wrap them with the
/// pose storage one actor needs.
pub(crate) fn build_actor_mesh(
    scene: &mut FlattenedScene,
    mut preview: PlayerPreview,
    entity_id: Option<EntityId>,
    corpse_pool: bool,
    appearance: Appearance,
    saber_names: [Option<String>; 2],
) -> Result<ActorMesh, Box<dyn Error>> {
    preview.origin = [0.0; 3];
    preview.yaw = 0.0;
    let frame = preview.sequence.first_frame;
    let sample = (frame, frame, 0.0);
    let matrices = preview
        .animation
        .split_interpolated_frame_matrices(
            (sample.0, sample.1),
            sample.2,
            (sample.0, sample.1),
            sample.2,
            "lower_lumbar",
        )
        .ok();
    // Non-humanoid skeletons may lack the split root; keep loading their bodies
    // even when humanoid attachments cannot be evaluated.
    let weapon_attachments = matrices.as_ref().map_or([None; 2], |matrices| {
        saber::attachments_from_matrices(&preview, matrices)
    });
    let jetpack = matrices.as_ref().map_or_else(Default::default, |matrices| {
        crate::jetpack::Worn::new(&preview, matrices)
    });
    // Cap surfaces ride along, hidden until a limb is cut (`dismember`).
    let default_flags = crate::dismember::reveal_caps(&mut preview.mesh.hierarchy);
    let (draws, vertex_ranges) = match preview.rig.clone() {
        Some(rig) => append_rig_mesh(scene, &rig)?,
        None => append_actor_mesh(scene, &preview, frame)?,
    };
    // A rig model draws none of its driver's surfaces, so no surface hides a draw.
    let surfaces = crate::dismember::Surfaces::new(
        &preview.mesh.hierarchy,
        default_flags,
        vertex_ranges
            .iter()
            .filter(|_| preview.rig.is_none())
            .map(|range| range.surface_index),
    );
    let pose_vertex_capacity = vertex_ranges
        .iter()
        .map(|range| range.vertices.len())
        .max()
        .unwrap_or(0);
    let animator = actor_pose::storage(&preview.animation)?;
    // No precision/mark/dismemberment consumer exists yet. Do not buy an unused extra LOD.
    let retained_pose = actor_pose::RetainedPose::new(&preview.mesh, &preview.skin, false)?;
    Ok(ActorMesh {
        flame_pose: Default::default(),
        entity_id,
        corpse_pool,
        body_identity: None,
        body_copied: false,
        appearance,
        angle_controller: actor_pose::angle_storage(&preview.animation),
        force_bones: actor_pose::ForceBones::new(&preview.animation),
        holsters: crate::holstered_sabers::pose::Worn::new(&preview),
        preview,
        draws,
        vertex_ranges,
        current_frames: (frame, frame),
        weapon_attachments,
        driver_seat: None,
        vehicle_muzzles: Default::default(),
        saber_names,
        render_yaw_degrees: None,
        wall_hold: Default::default(),
        animator,
        audio_events: Default::default(),
        pose_vertices: Vec::with_capacity(pose_vertex_capacity),
        gpu_palette: None,
        retained_pose,
        cosmetics: Default::default(),
        jetpack,
        disintegration: None,
        body_fade: None,
        surfaces,
        limb: None,
        ragdoll: Default::default(),
    })
}

/// Append a rig model's surfaces in their bind pose; each range's `surface_index`
/// is the rig surface it holds.
fn append_rig_mesh(
    scene: &mut FlattenedScene,
    rig: &crate::rig_model::RigModel,
) -> Result<(Vec<ActorDraw>, Vec<PreviewVertexRange>), Box<dyn Error>> {
    let mut draws = Vec::with_capacity(rig.surfaces.len());
    let mut vertex_ranges = Vec::with_capacity(rig.surfaces.len());
    for (surface_index, surface) in rig.surfaces.iter().enumerate() {
        let vertex_base: u32 = scene.vertices.len().try_into()?;
        let vertex_start = scene.vertices.len();
        let index_start: u32 = scene.indices.len().try_into()?;
        scene.vertices.extend(
            surface
                .vertices
                .iter()
                .map(|vertex| rig_gpu_vertex(vertex, None)),
        );
        for index in &surface.indices {
            scene.indices.push(
                vertex_base
                    .checked_add(*index)
                    .ok_or("rig mesh index overflow")?,
            );
        }
        let material = scene.materials.len();
        scene.materials.push(ViewerMaterial {
            shader: surface.shader.clone(),
            lightmap: -1,
        });
        draws.push(ActorDraw {
            indices: index_start..scene.indices.len().try_into()?,
            material,
        });
        vertex_ranges.push(PreviewVertexRange {
            surface_index,
            vertices: vertex_start..scene.vertices.len(),
        });
    }
    Ok((draws, vertex_ranges))
}

/// A rig vertex for the shared buffer: bind pose, or skinned with `palette` (CPU path).
pub(crate) fn rig_gpu_vertex(
    vertex: &crate::rig_model::Vertex,
    palette: Option<&[[[f32; 4]; 3]]>,
) -> GpuVertex {
    let (position, normal) = palette.map_or((vertex.position, vertex.normal), |palette| {
        crate::rig_model::skin(vertex, palette)
    });
    GpuVertex {
        position,
        normal,
        color: [1.0; 4],
        texture_coordinates: vertex.texture_coordinates,
        lightmap_coordinates: [0.0; 2],
    }
}

/// Load `appearance`, falling back to `stand_in` ([`stand_in`]) when it cannot be
/// loaded.
fn load_or_fallback(
    vfs: &VirtualFileSystem,
    appearance: &Appearance,
    stand_in: &Appearance,
    fallback: &Appearance,
    cache: &mut crate::player_assets::GlaCache,
) -> Result<PlayerPreview, Box<dyn Error>> {
    match load_appearance(vfs, appearance, cache) {
        Ok(actor) => Ok(actor),
        Err(error) if appearance != fallback => {
            eprintln!(
                "could not load actor appearance {}/{}: {error}; using {}/{}",
                appearance.model, appearance.variant, stand_in.model, stand_in.variant
            );
            load_stand_in(vfs, stand_in, fallback, cache)
        }
        Err(error) => Err(error),
    }
}

/// Load `stand_in`, or Kyle (`fallback`) if even the stand-in cannot be loaded.
pub(crate) fn load_stand_in(
    vfs: &VirtualFileSystem,
    stand_in: &Appearance,
    fallback: &Appearance,
    cache: &mut crate::player_assets::GlaCache,
) -> Result<PlayerPreview, Box<dyn Error>> {
    match load_appearance(vfs, stand_in, cache) {
        Ok(actor) => Ok(actor),
        Err(error) if stand_in != fallback => {
            eprintln!(
                "could not load stand-in {}/{}: {error}; using Kyle",
                stand_in.model, stand_in.variant
            );
            load_appearance(vfs, fallback, cache)
        }
        Err(error) => Err(error),
    }
}

fn load_appearance(
    vfs: &VirtualFileSystem,
    appearance: &Appearance,
    cache: &mut crate::player_assets::GlaCache,
) -> Result<PlayerPreview, Box<dyn Error>> {
    crate::player_assets::load_player_appearance_with(
        vfs,
        &appearance.model,
        &appearance.variant,
        [0.0; 3],
        0.0,
        cache,
    )
}

/// Whether `cg_defaultModelRandom` is on: a missing model is drawn as a retail model
/// picked from its name rather than as Kyle (`sjk_client::default_model_random`).
pub(crate) fn random_stand_ins(console: Option<&crate::console::ViewerConsole>) -> bool {
    console
        .and_then(|c| c.bool_cvar("cg_defaultmodelrandom"))
        .unwrap_or(false)
}

/// The model drawn for `client`'s `requested` appearance if it cannot be loaded:
/// Kyle, or with `random` a retail model picked from its name.
pub(crate) fn stand_in(
    game_state: Option<&GameState>,
    client: Option<u16>,
    requested: &Appearance,
    random: bool,
) -> Appearance {
    match (game_state, client) {
        (Some(game_state), Some(client)) => {
            sjk_client::default_model_random::legacy_client_stand_in(
                game_state, client, requested, random,
            )
        }
        _ => fallback_appearance(),
    }
}

/// Build the meshes of every client the game state advertises, every actor
/// already in `world`, and the shared fallback.
///
/// With `local_only` (a live map load) only the local player's model is built of
/// the clients: the others come from the workers of `player_loads.rs` once the
/// map is live (`refresh_forced_models` asks for every slot on the world's first
/// frame), each drawn as its stand-in until its model is in. Built here, they held
/// the loading screen 2.4 s on JoF with about 25 players (11/11/2026).
pub(crate) fn load_actor_meshes(
    vfs: &VirtualFileSystem,
    game_state: Option<&GameState>,
    world: &World,
    scene: &mut FlattenedScene,
    eligibility: &sjk_client::player_model_eligibility::PlayerModelEligibility,
    console: Option<&crate::console::ViewerConsole>,
    local_only: bool,
) -> Result<Vec<ActorMesh>, Box<dyn Error>> {
    let local = game_state.map(|game| i64::from(game.client_num));
    let built_now = |client: u64| !local_only || local == Some(client as i64);
    let fallback = fallback_appearance();
    let mut cache = crate::player_assets::GlaCache::default();
    // Player entity numbers are their client slots. Build a dedicated
    // deformable mesh for every advertised client, even if that player
    // was outside the first snapshot's PVS. Otherwise a later arrival
    // can only use the shared Kyle fallback and cannot hold its own pose.
    let options = model_options(console);
    let random = random_stand_ins(console);
    let mut actors = BTreeMap::new();
    if let Some(game_state) = game_state {
        for client_num in (0..32_u16).filter(|&client| built_now(u64::from(client))) {
            if let Some(appearance) =
                legacy_client_appearance_with_policy(game_state, client_num, None, |appearance| {
                    eligible_appearance(appearance, true, eligibility, options)
                })
            {
                actors.insert(EntityId::new(u64::from(client_num) + 1), appearance);
            }
        }
    }
    for entity in world.entities().filter(|entity| {
        entity.kind == EntityKind::Actor
            && (entity.id.get() > 32 || built_now(entity.id.get().saturating_sub(1)))
    }) {
        actors.entry(entity.id).or_insert_with(|| {
            eligible_appearance(
                entity
                    .appearance()
                    .cloned()
                    .unwrap_or_else(|| fallback.clone()),
                entity.id.get() <= 32,
                eligibility,
                options,
            )
        });
    }
    let mut meshes = Vec::with_capacity(actors.len() * 2 + 1);
    for (entity_id, appearance) in actors {
        // NPC entities (numbers past the client slots) get their hilts from
        // `npc_refresh` once the first snapshot names them.
        let client_num = u16::try_from(entity_id.get().saturating_sub(1))
            .ok()
            .filter(|client_num| *client_num < 32);
        let saber_names = client_num
            .map(|client_num| client_saber_names(game_state, client_num))
            .unwrap_or_else(|| [Some("single_1".to_owned()), None]);
        let stand_in = stand_in(game_state, client_num, &appearance, random);
        // A failed build leaves the scene as it was, so the Kyle fallback does
        // not sit behind half-appended geometry and materials.
        let build = |scene: &mut FlattenedScene, actor: PlayerPreview, appearance: &Appearance| {
            let lengths = (
                scene.vertices.len(),
                scene.indices.len(),
                scene.materials.len(),
                scene.draws.len(),
            );
            let mut pair = Vec::with_capacity(2);
            for (assigned_entity, corpse_pool, preview) in
                [(Some(entity_id), false, actor.clone()), (None, true, actor)]
            {
                let built = build_actor_mesh(
                    scene,
                    preview,
                    assigned_entity,
                    corpse_pool,
                    appearance.clone(),
                    saber_names.clone(),
                );
                match built {
                    Ok(mesh) => pair.push(mesh),
                    Err(error) => {
                        scene.vertices.truncate(lengths.0);
                        scene.indices.truncate(lengths.1);
                        scene.materials.truncate(lengths.2);
                        scene.draws.truncate(lengths.3);
                        return Err(error);
                    }
                }
            }
            Ok::<_, Box<dyn Error>>(pair)
        };
        match load_or_fallback(vfs, &appearance, &stand_in, &fallback, &mut cache) {
            // One player's model must not fail the map: a mesh that loads but
            // cannot be built is replaced by its stand-in like one that cannot load.
            Ok(actor) => match build(scene, actor, &appearance) {
                Ok(pair) => meshes.extend(pair),
                Err(error) if appearance != fallback => {
                    eprintln!(
                        "could not build actor appearance {}/{}: {error}; using {}/{}",
                        appearance.model, appearance.variant, stand_in.model, stand_in.variant
                    );
                    let stand_in = load_stand_in(vfs, &stand_in, &fallback, &mut cache)?;
                    meshes.extend(build(scene, stand_in, &appearance)?);
                }
                Err(error) => return Err(error),
            },
            Err(error) => eprintln!(
                "could not load actor appearance {}/{}: {error}",
                appearance.model, appearance.variant
            ),
        }
    }
    if let Ok(actor) = crate::player_assets::load_player_appearance_with(
        vfs,
        &fallback.model,
        &fallback.variant,
        [0.0; 3],
        0.0,
        &mut cache,
    ) {
        let names = [Some("single_1".to_owned()), None];
        meshes.push(build_actor_mesh(
            scene, actor, None, false, fallback, names,
        )?);
    }
    Ok(meshes)
}

#[cfg(test)]
mod eligibility_tests {
    use super::*;
    #[test]
    fn player_overrides_fall_back_but_the_same_npc_appearance_remains() {
        let mut vfs = VirtualFileSystem::new();
        vfs.mount_memory(
            "synthetic",
            [(
                "models/players/creature/settings.txt",
                b"notInMP 1".to_vec(),
            )],
        )
        .unwrap();
        let policy = sjk_client::player_model_eligibility::PlayerModelEligibility::load(&vfs);
        let model = Appearance {
            model: "models/players/creature".into(),
            variant: "red".into(),
        };
        assert_eq!(
            eligible_appearance(model.clone(), true, &policy, ("", true)),
            fallback_appearance()
        );
        assert_eq!(
            eligible_appearance(model.clone(), false, &policy, ("creature", true)),
            model
        );
        assert_eq!(
            eligible_appearance(model.clone(), true, &policy, ("", false)),
            model
        );
        assert_eq!(
            eligible_appearance(model, true, &policy, ("CREATURE/default", false)),
            fallback_appearance()
        );
    }
}
