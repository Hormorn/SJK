//! Actor, held-weapon, saber, and force-overlay frame submission.
#[path = "first_person_saber.rs"]
pub(crate) mod first_person_saber;

use super::*;

#[path = "flag_carrier.rs"]
pub(crate) mod flags;

#[path = "force_power_submission.rs"]
pub(crate) mod force_powers;

#[path = "thrown_saber.rs"]
mod thrown_saber;

#[path = "actor_model_scale.rs"]
pub(crate) mod model_scale;

#[path = "monster_hold.rs"]
pub(crate) mod monster_hold;

#[path = "player_sprites.rs"]
mod player_sprites;
#[path = "speed_trail.rs"]
pub(crate) mod speed_trail;

#[path = "grapple_rope.rs"]
mod grapple_rope;

#[path = "jetpack_submission.rs"]
mod jetpack_submission;

#[path = "holstered_saber_submission.rs"]
mod holster_submission;

#[path = "player_sphere_submission.rs"]
mod player_spheres;

/// `EF_DEAD`.
const EF_DEAD_FLAG: u32 = 1 << 1;

struct Sinks<'a> {
    /// The effect atlas, for the blend a blade skin's smoke shader draws with, and the
    /// smoke ribbons.
    particle_atlas: &'a crate::ParticleAtlas,
    smoke_ribbons: &'a mut crate::saber_ribbons::Ribbons,
    saber_sparks: &'a mut crate::saber_sparks::Sparks,
    /// Where blades clashed this frame (`impacts::Pool::clashes`).
    saber_clashes: &'a [[f32; 3]],
    /// What the blades of skins with a crackle are heard doing this frame.
    saber_heard: &'a mut crate::saber_smoke::Heard,
    /// TRACE-DISINTEGRATE (temporary): the forced disintegrations, to log a body of
    /// theirs drawn whole.
    forced: &'a crate::disintegration::Forced,
    grip_victim: bool,
    force_fx: force_powers::policy::Policy,
    flame: Option<&'static force_powers::flame::Contract>,
    flag_meshes: [Option<usize>; 2],
    shield_mesh: Option<usize>,
    shield_sphere: bool,
    shield_passes: u32,
    /// `cg_duelGhost`: JoF EJK's blue duel figures ([`crate::duel_ghost`]).
    duel_ghosts: bool,
    /// `g_gametype` from the serverinfo.
    gametype: i32,
    /// The rendered view's origin, which player spheres face.
    view_origin: Vec3,
    jetpack: crate::jetpack::Catalog,
    jetpack_sounds: &'a mut crate::jetpack::Sounds,
    /// The boombox's model and who carries one (bits by slot, [`crate::boombox`]).
    boombox: (Option<usize>, u64),
    /// The fishing rod's and line's models and each fisher's line ([`crate::fishing`]),
    /// and where each rod's tip is drawn (for the next frame's catch).
    fishing: (Option<usize>, Option<usize>, crate::fishing::Lines),
    fishing_tips: &'a mut [Option<[f32; 3]>; 64],
    /// The pickaxe's model and each miner's pickaxe ([`crate::mining`]).
    mining: (Option<usize>, crate::mining::Picks),
    /// The prop gun each player holds, by slot ([`crate::propgun`]).
    propgun: [Option<crate::propgun_frame::HeldDraw>; crate::looks::SLOTS],
    /// How bright each player's Screen Saver shell is ([`crate::monitor::saver`]).
    screen_savers: crate::monitor::Glows,
    world: &'a sjk_runtime::World,
    actor_meshes: &'a [ActorMesh],
    object_meshes: &'a [StaticModelMesh],
    actor_groups: &'a mut [Vec<ActorInstance>],
    object_groups: &'a mut [Vec<ActorInstance>],
    /// Where each emplaced gun's top was last turned ([`crate::emplaced_swivel`]).
    emplaced_turned: &'a mut crate::emplaced_swivel::Turned,
    overrides: &'a mut Vec<entity_materials::OverrideInstance>,
    entity_instances: &'a mut Vec<EntityInstance>,
    saber_hilts: Option<&'a saber::HiltCatalog>,
    /// Who wears which blade skin.
    saber_skins: &'a crate::saber_skins::SaberSkins,
    black_sabers: crate::saber_rgb::black::Policy,
    /// `cg_saberCoreSize` ([`crate::saber_core`]), read once a frame.
    saber_core: f32,
    saber_states: &'a mut saber_trail::StateSlab,
    saber_segments: &'a mut saber_trail::SegmentPool,
    /// Trail edges for held and flying blades; `None` with `cg_saberTrail 0`.
    trail_edges: Option<saber_trail::Edges<'a>>,
    saber_instances: &'a mut Vec<saber::Instance>,
    lights: &'a mut dynamic_lights::PointLightList,
    presentation_time: i64,
    muzzle_effects: &'a sjk_client::LegacyMuzzleEffects,
    /// Vehicle volleys to play at their muzzles, and the guns' `muzzleFX`.
    vehicle_fires: &'a mut crate::vehicle_fire::Pending,
    vehicle_flames: &'a mut crate::vehicle_burning::Flames,
    missile_effects: &'a sjk_client::LegacyMissileEffects,
    particles: &'a mut Vec<Particle>,
    effect_aux: &'a mut effect_aux::Runtime,
    effects: &'a mut EffectLibrary,
    vfs: &'a VirtualFileSystem,
    game_audio: &'a mut Option<GameAudio>,
    force_tracker: &'a sjk_client::LegacyForceOverlayTracker,
    speed_trails: &'a mut speed_trail::Trails,
    /// `centity_t::trickAlpha` per entity.
    trick_fades: &'a mut sjk_client::LegacyTrickFades,
    /// The viewer's `forcePowersActive`, for `CG_IsMindTricked`'s Force Sight check.
    viewer_force_powers_active: u32,
    /// `cg_speedTrail`.
    speed_trail: bool,
    holsters: crate::holstered_sabers::Policy,
    holster_geometry: crate::holstered_sabers::pose::Geometry,
    /// `cg_cosmetics`: whose hats and capes are drawn.
    cosmetics: crate::cosmetics::Visibility,
    material_overrides: model_materials::Overrides,
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,
    view_height: f32,
    predicted_force_powers_active: Option<u32>,
    predicted_local_state: Option<&'a sjk_client::pmove::MovementState>,
    predicted_vehicle: Option<crate::vehicle_pose::Predicted>,
    local_equipment: Option<sjk_runtime::HeldEquipment>,
    third_person: bool,
    portal_view: bool,
    entity_view_flags: u32,
    detached_camera: bool,
    detached_flight: bool,
    first_person_saber: bool,
    /// This frame's JA+ grapple hooks, drawn as ropes from their players' hands.
    hooks: grapple_rope::Hooks,
    /// Every player's weapon charge, for the glow on its muzzle.
    charges: crate::charge_flash::Charges,
    /// Client slots muted on this PC (bits): drawn with the default saber colour
    /// and without hat or cape (`muted_players.rs`).
    muted: u32,
    /// Client slots whose players chose toon shading for their model, while this PC shows
    /// it (`toon_choice.rs`, `r_toonModels`).
    toon_slots: u32,
    /// Each client slot's team blade colours in team games: `cg_saberTeamColors`, and
    /// `cg_ctfTeamSabers` (no skin over them) in CTF or CTY (`team_sabers.rs`); in
    /// Siege the class-forced colours (`siege_sabers.rs`).
    team_blades: crate::team_sabers::Blades,
    /// The player whose card is up with its outline on (`card_outline.rs`).
    card_outline: Option<u16>,
    /// The players outlined as teammates and enemies (`hud::team_outline`).
    team_outline: crate::hud::team_outline::Groups,
    /// Why each player's talk balloon is up, where known (`status_balloons.rs`).
    status_balloons: crate::status_balloons::Shown,
}

/// Submit all presented actor-like entities without allocating frame storage.
pub(crate) fn submit(
    gpu: &mut GpuState,
    local_entity_id: Option<u64>,
    fallback_mesh: Option<usize>,
    presentation_time: i64,
    visual_now: Instant,
    game_audio: &mut Option<GameAudio>,
) -> usize {
    let snapshot = first_person_view::presented_snapshot(
        gpu.live_session.as_ref(),
        gpu.demo_session.as_ref(),
        presentation_time as i32,
    );
    let active_world = gpu
        .demo_session
        .as_ref()
        .map_or(&gpu.live_world, demo_playback::Session::world);
    let game_state = gpu
        .live_session
        .as_ref()
        .map(ClientSession::game_state)
        .or_else(|| {
            gpu.demo_session
                .as_ref()
                .map(demo_playback::Session::game_state)
        });
    let local_mesh = gpu.actor_meshes.iter().position(|mesh| {
        mesh.entity_id.map(|id| id.get()) == local_entity_id && mesh.entity_id.is_some()
    });
    let local_equipment = local_entity_id
        .and_then(|id| active_world.entity(sjk_runtime::EntityId::new(id)))
        .and_then(|entity| {
            local_actor_state::equipment(
                entity.equipment(),
                gpu.local_prediction
                    .predicted_state()
                    .filter(|_| !gpu.detached_camera),
                snapshot.map(|s| s.player.saber_move()),
            )
        });
    let saber_body = first_person_saber::visible(
        true,
        gpu.third_person,
        gpu.detached_camera,
        local_equipment.map(|held| held.weapon),
        snapshot.map(|s| &s.player),
    ) && gpu
        .local_prediction
        .predicted_state()
        .is_none_or(|p| p.health > 0 && p.entity_flags & 2 == 0)
        && local_mesh.is_some_and(|index| {
            !gpu.actor_meshes[index].surfaces.weapon_lost
                && gpu.actor_meshes[index].body_identity.is_none()
                // A rig model is one glTF surface: no head to hide, so no body view.
                && gpu.actor_meshes[index].preview.rig.is_none()
        });
    for (index, mesh) in gpu.actor_meshes.iter_mut().enumerate() {
        mesh.surfaces.first_person_saber(
            saber_body && local_mesh == Some(index),
            &mesh.preview.mesh.hierarchy,
        );
    }
    let aura_shell = gpu
        .console
        .as_ref()
        .and_then(|console| console.integer_cvar("cg_auraShell"))
        .unwrap_or(1)
        != 0;
    let force_fx = force_powers::policy::Policy::sample(gpu.console.as_ref(), game_state);
    let combined_protect_absorb = force_fx.combined
        && gpu
            .console
            .as_ref()
            .and_then(|console| console.bool_cvar("cg_spprotabscolor"))
            .unwrap_or(true);
    // 1 draws multiplayer's sphere around a shield hit; 0 hugs the body like single player.
    let shield_sphere = gpu
        .console
        .as_ref()
        .and_then(|console| console.integer_cvar("cg_shieldSphere"))
        .unwrap_or(0)
        != 0;
    let duel_ghosts = gpu
        .console
        .as_ref()
        .and_then(|console| console.bool_cvar(crate::duel_ghost::CVAR))
        .unwrap_or(true);
    let shield_passes = gpu
        .console
        .as_ref()
        .and_then(|console| console.integer_cvar("cg_shieldBrightness"))
        .unwrap_or(2)
        .clamp(1, 12) as u32;
    let trails = gpu
        .console
        .as_ref()
        .and_then(|console| console.integer_cvar("cg_saberTrail"))
        .unwrap_or(1)
        != 0;
    let speed_trail = gpu
        .console
        .as_ref()
        .and_then(|console| console.integer_cvar("cg_speedTrail"))
        .unwrap_or(1)
        != 0;
    let cosmetics = crate::cosmetics::Visibility::from_cvar(
        gpu.console
            .as_ref()
            .and_then(|console| console.integer_cvar(crate::cosmetics::VISIBILITY_CVAR))
            .unwrap_or(1),
    );
    let saber_contact = gpu.effect_aux.saber_contacts.enabled;
    let detached_flight = gpu.free_camera_active();
    let gametype = game_state
        .and_then(|game| game.config_string(0))
        .and_then(|info| sjk_client::LegacyClientInfo::new(info).integer("g_gametype"))
        .unwrap_or(0);
    let propgun = gpu.propgun_held_draws();
    // A demo plays without the hub: stock's balloons.
    let status_balloons = if gpu.demo_session.is_none() {
        gpu.status_balloons.shown
    } else {
        crate::status_balloons::Shown::default()
    };
    #[cfg(test)]
    let status_balloons = gpu.status_balloons.shot.unwrap_or(status_balloons);
    let mut sinks = Sinks {
        force_fx,
        grip_victim: force_fx.animation
            && gpu
                .console
                .as_ref()
                .and_then(|console| console.bool_cvar("cg_gripvictimfx"))
                .unwrap_or(true),
        flame: force_powers::flame::policy(game_state, gpu.console.as_ref()),
        flag_meshes: gpu.pickup_catalog.carrier_meshes[flags::model_set(gametype)],
        shield_mesh: gpu
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == "models/weaphits/testboom.md3"),
        shield_sphere,
        shield_passes,
        duel_ghosts,
        gametype,
        view_origin: gpu.view_origin,
        jetpack: gpu.jetpack,
        jetpack_sounds: &mut gpu.jetpack_sounds,
        screen_savers: if gpu.demo_session.is_none() {
            gpu.monitor.glows()
        } else {
            crate::monitor::Glows::default()
        },
        boombox: (
            gpu.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == crate::boombox::MODEL),
            gpu.boombox.carriers(),
        ),
        fishing: (
            gpu.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == crate::fishing::ROD),
            gpu.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == crate::fishing::LINE),
            gpu.fishing.lines,
        ),
        fishing_tips: &mut gpu.fishing.tips,
        mining: (
            gpu.object_meshes
                .iter()
                .position(|mesh| mesh.appearance.model == crate::mining::PICK),
            gpu.mining.picks,
        ),
        propgun,
        world: active_world,
        actor_meshes: &gpu.actor_meshes,
        object_meshes: &gpu.object_meshes,
        actor_groups: &mut gpu.actor_groups,
        object_groups: &mut gpu.object_groups,
        emplaced_turned: &mut gpu.emplaced_turned,
        overrides: &mut gpu.pickup_override_instances,
        entity_instances: &mut gpu.entity_instances,
        saber_hilts: gpu.saber_hilts.as_ref(),
        saber_skins: &gpu.saber_skins,
        black_sabers: gpu.black_sabers.policy(
            game_state,
            gpu.console
                .as_ref()
                .and_then(|c| c.bool_cvar(crate::saber_rgb::black::CVAR))
                .unwrap_or(true),
            gpu.console
                .as_ref()
                .and_then(|c| c.integer_cvar("cp_plugindisable"))
                .unwrap_or(i64::from(sjk_client::PLUGIN_DISABLE_DEFAULT)),
            gpu.muted_players.slots().muted,
        ),
        saber_core: crate::saber_core::read(gpu.console.as_ref()),
        particle_atlas: &gpu.particle_atlas,
        smoke_ribbons: &mut gpu.smoke_ribbons,
        saber_sparks: &mut gpu.saber_sparks,
        saber_clashes: gpu.impacts.clashes(),
        saber_heard: &mut gpu.saber_heard,
        forced: &gpu.forced_disintegrations,
        saber_states: &mut gpu.saber_states,
        saber_segments: &mut gpu.saber_trail_segments,
        trail_edges: trails.then(|| {
            saber_trail::Edges::new(saber_contact.then_some((&gpu.bsp, &mut gpu.trace_scratch)))
        }),
        saber_instances: &mut gpu.saber_instances,
        lights: &mut gpu.dynamic_lights,
        presentation_time,
        muzzle_effects: &gpu.muzzle_effects,
        vehicle_fires: &mut gpu.vehicle_fires,
        vehicle_flames: &mut gpu.vehicle_flames,
        missile_effects: &gpu.missile_effects,
        particles: &mut gpu.particles,
        effect_aux: &mut gpu.effect_aux,
        effects: &mut gpu.effects,
        vfs: gpu
            .vfs
            .as_deref()
            .expect("sessions retain their mounted VFS"),
        game_audio,
        force_tracker: &gpu.force_overlays,
        speed_trails: &mut gpu.speed_trails,
        trick_fades: &mut gpu.trick_fades,
        viewer_force_powers_active: gpu
            .local_prediction
            .predicted_state()
            .map(|state| state.force_powers_active)
            .or_else(|| snapshot.map(|snapshot| snapshot.player.force_powers_active()))
            .unwrap_or(0),
        speed_trail,
        holsters: crate::holstered_sabers::Policy::read(
            gpu.console.as_ref(),
            &mut gpu.holster_server,
            game_state,
        ),
        holster_geometry: crate::holstered_sabers::pose::Geometry::read(gpu.console.as_ref()),
        cosmetics,
        material_overrides: gpu.model_material_overrides,
        camera_position: gpu.camera_position,
        camera_yaw: gpu.camera_yaw,
        camera_pitch: gpu.camera_pitch,
        view_height: gpu.local_prediction.view_height(),
        predicted_force_powers_active: gpu
            .local_prediction
            .predicted_state()
            .map(|state| state.force_powers_active),
        predicted_local_state: gpu
            .local_prediction
            .predicted_state()
            .filter(|_| !detached_flight),
        predicted_vehicle: gpu
            .live_session
            .as_ref()
            .and_then(|_| gpu.local_prediction.vehicle_pose()),
        local_equipment,
        third_person: gpu.third_person,
        portal_view: gpu.scene_views.has_portal_view(),
        entity_view_flags: 0,
        detached_camera: gpu.detached_camera,
        detached_flight,
        first_person_saber: saber_body,
        hooks: grapple_rope::Hooks::collect(snapshot, game_state, presentation_time as i32),
        charges: crate::charge_flash::Charges::collect(
            snapshot,
            gpu.live_session
                .as_ref()
                .and_then(|_| gpu.local_prediction.predicted_state()),
        ),
        muted: gpu.muted_players.slots().muted,
        toon_slots: if gpu.context.dynamic_light_settings.toon_models() {
            crate::toon_choice::slots(&gpu.looks)
        } else {
            0
        },
        team_blades: crate::team_sabers::with_classes(
            crate::team_sabers::blades(gpu.console.as_ref(), game_state),
            gpu.siege_sabers.forced(game_state, gpu.vfs.as_deref()),
        ),
        card_outline: gpu.hud.card.outline().map(|(client, _)| client),
        team_outline: gpu.hud.team_outline.groups(),
        status_balloons,
    };
    let thrown = snapshot
        .zip(game_state)
        .map(|(snapshot, game)| sjk_client::LegacyThrownSabers::new(snapshot, game));
    let mut overlay_count = 0;
    if let Some(snapshot) = snapshot {
        force_powers::flame::submit_hits(
            &mut sinks,
            snapshot,
            presentation_time as i32,
            visual_now,
        );
    }
    if let Some(audio) = sinks.game_audio.as_mut() {
        audio.begin_holsters();
    }
    for entity in active_world
        .entities()
        .filter(|entity| {
            !matches!(
                entity.kind,
                EntityKind::Projectile | EntityKind::Mover | EntityKind::Item | EntityKind::Effect
            )
        })
        .take(1_024)
    {
        sinks.entity_view_flags = u16::try_from(entity.id.get().saturating_sub(1))
            .ok()
            .map_or(0, |number| {
                crate::actor_instance::scene_flags(game_state, snapshot, number)
                    | crate::toon_choice::flag(sinks.toon_slots, number)
            });
        let mut transform = entity.sample(presentation_time);
        if matches!(entity.kind, EntityKind::Actor | EntityKind::Corpse) {
            overlay_count += submit_actor(
                &mut sinks,
                entity,
                &mut transform,
                snapshot,
                game_state,
                local_entity_id,
                fallback_mesh,
                presentation_time,
                visual_now,
                aura_shell,
                combined_protect_absorb,
            );
        } else if submit_limb(&mut sinks, entity, &transform, visual_now) {
            // A cut-off limb draws its own copy of the owner (`dismember`).
        } else if thrown_saber::submit(&mut sinks, thrown.as_ref(), entity, transform) {
            // The flying hilt is owned by this branch, including its blades.
        } else if submit_emplaced(&mut sinks, entity, &transform, snapshot, local_entity_id) {
            // A turned emplaced gun is drawn in its two parts.
        } else if submit_eweb(&mut sinks, entity, &transform, snapshot, game_state) {
            // An E-Web set down is drawn in its three parts.
        } else if let Some(mesh) = entity.appearance().and_then(|appearance| {
            sinks
                .object_meshes
                .iter()
                .position(|mesh| &mesh.appearance == appearance)
        }) {
            let mut instance =
                ActorInstance::new(transform.translation, transform.rotation, transform.scale);
            instance.view_flags = sinks.entity_view_flags;
            sinks.object_groups[mesh].push(instance);
        }
    }
    crate::demp2_shell::submit(
        &mut gpu.demp2_shells,
        sinks
            .object_meshes
            .iter()
            .position(|mesh| mesh.appearance.model == crate::demp2_shell::MODEL),
        sinks.material_overrides.demp2_shell,
        sinks.overrides,
        visual_now,
    );
    // Volleys of vehicles not drawn this frame play where their events were.
    while let Some(fire) = sinks.vehicle_fires.take_any(visual_now) {
        crate::vehicle_fire::play(&mut sinks.vehicle_fire(), &fire, |_| None, visual_now);
    }
    if let Some(audio) = sinks.game_audio.as_mut() {
        audio.end_holsters();
    }
    overlay_count
}

impl Sinks<'_> {
    /// What [`crate::vehicle_fire::play`] needs.
    fn vehicle_fire(&mut self) -> crate::vehicle_fire::Sinks<'_> {
        crate::vehicle_fire::Sinks {
            particles: self.particles,
            auxiliary: self.effect_aux,
            effects: self.effects,
            vfs: self.vfs,
            audio: self.game_audio,
            missiles: self.missile_effects,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn submit_actor(
    sinks: &mut Sinks<'_>,
    entity: &sjk_runtime::SceneEntity,
    transform: &mut sjk_runtime::Transform,
    snapshot: Option<&Snapshot>,
    game_state: Option<&GameState>,
    local_entity_id: Option<u64>,
    fallback_mesh: Option<usize>,
    presentation_time: i64,
    visual_now: Instant,
    aura_shell: bool,
    combined_protect_absorb: bool,
) -> usize {
    let draw_actor =
        sinks.third_person || sinks.detached_camera || Some(entity.id.get()) != local_entity_id;
    if Some(entity.id.get()) == local_entity_id && !sinks.detached_camera {
        let (translation, rotation) =
            camera::local_actor_root(sinks.camera_position, sinks.view_height, sinks.camera_yaw);
        transform.translation = translation;
        transform.rotation = rotation;
    }
    let mesh = sinks
        .actor_meshes
        .iter()
        .position(|mesh| mesh.entity_id == Some(entity.id))
        .or_else(|| {
            (entity.kind != EntityKind::Corpse)
                .then_some(fallback_mesh)
                .flatten()
        });
    transform.rotation = actor_pose::world_rotation(
        mesh.and_then(|index| sinks.actor_meshes.get(index)),
        transform.rotation,
    );
    let state = snapshot.and_then(|snapshot| {
        snapshot
            .entities
            .iter()
            .find(|state| u64::from(state.number()) + 1 == entity.id.get())
    });
    crate::vehicle_pose::place(
        sinks.world,
        sinks.actor_meshes,
        mesh,
        entity,
        transform,
        snapshot,
        state,
        Some(entity.id.get()) == local_entity_id,
        presentation_time,
        sinks.predicted_vehicle,
    );
    let local_player = snapshot
        .filter(|_| Some(entity.id.get()) == local_entity_id)
        .map(|snapshot| &snapshot.player);
    if let Some((percent, npc_class)) = model_scale::source(state, local_player) {
        model_scale::apply(transform, percent, npc_class);
    }
    // A limp player (the Ragdoll toy) is drawn where the ragdoll lies, posed in its
    // model space (`actor_pose_steps`).
    if entity.kind != EntityKind::Corpse
        && mesh.is_some_and(|index| sinks.actor_meshes[index].ragdoll.posed)
        && let Some(points) = crate::ragdoll::shown_entity(entity.id)
    {
        crate::ragdoll::place(transform, &points);
    }
    // Players around the Toy Master's spinning toy spin and squish with it, their looks
    // alone (`toy_master::dance_actor`); not a limp one nor a vehicle.
    if entity.kind == EntityKind::Actor
        && !mesh.is_some_and(|index| {
            sinks.actor_meshes[index].ragdoll.posed
                || sinks.actor_meshes[index].preview.vehicle.is_some()
        })
    {
        crate::toy_master::dance_actor(transform, 24.0);
    }
    // `EV_VEH_FIRE`: this vehicle's volleys, at its muzzles as posed now.
    if let Some(index) = mesh
        && sinks.actor_meshes[index].preview.vehicle.is_some()
        && let Ok(number) = u16::try_from(entity.id.get().saturating_sub(1))
    {
        let muzzles = sinks.actor_meshes[index].vehicle_muzzles;
        let placed = *transform;
        while let Some(fire) = sinks.vehicle_fires.take(number) {
            crate::vehicle_fire::play(
                &mut sinks.vehicle_fire(),
                &fire,
                |muzzle| muzzles[muzzle].map(|at| crate::vehicle_fire::world_socket(at, &placed)),
                visual_now,
            );
        }
    }
    // `CG_VehicleEffects`: a destroyed vehicle burns until it blows up.
    if let Some(index) = mesh
        && let Some(kind) = sinks.actor_meshes[index].preview.vehicle
    {
        let burning = crate::vehicle_burning::burns(kind, state.map_or(0, |s| s.e_flags()));
        let number = u16::try_from(entity.id.get().saturating_sub(1)).unwrap_or(u16::MAX);
        if sinks.vehicle_flames.due(number, burning, visual_now) {
            let seed = entity.id.get() as u32 | 0x6000_0000;
            let fire = sinks.vehicle_fire();
            crate::effect_runtime::spawn_effect(
                fire.particles,
                fire.auxiliary,
                fire.effects,
                fire.vfs,
                crate::vehicle_burning::EFFECT,
                Vec3::from_array(transform.translation),
                visual_now,
                seed,
                0,
                fire.audio,
                crate::combat_effects::rotation_from_direction([0.0, 0.0, 1.0]),
            );
        }
    }
    // `EF_DISINTEGRATION`: `CG_Disintegration` draws the body and nothing else
    // (`cg_players.c`, `cg_ents.c`).
    let meshes = sinks.actor_meshes;
    if let Some(index) = mesh
        && let Some(burning) = meshes[index].disintegration.as_ref()
    {
        submit_disintegration(
            sinks, index, burning, entity, transform, draw_actor, visual_now,
        );
        return 0;
    }
    // `EV_BODYFADE`: `CG_General` draws a fading body's layers and nothing else
    // (`cg_ents.c`, [`crate::body_fade`]).
    if entity.kind == EntityKind::Corpse
        && (draw_actor || sinks.portal_view)
        && let Some(index) = mesh
        && let Some(fade) = meshes[index].body_fade.as_ref()
    {
        let rotation = weapon_view::actor_world_rotation(transform.rotation);
        let mut body =
            ActorInstance::new(transform.translation, rotation.to_array(), transform.scale)
                .with_model_color(entity.color());
        body.view_flags = sinks.entity_view_flags | u32::from(!draw_actor);
        let mut frame = crate::body_fade::Frame {
            overrides: sinks.overrides,
            materials: sinks.material_overrides,
            audio: sinks.game_audio.as_mut(),
            vfs: sinks.vfs,
            now: presentation_time,
        };
        if crate::body_fade::submit(&mut frame, index, fade, entity, body) {
            meshes[index]
                .retained_pose
                .mark_drawn(entity.id, presentation_time);
            return 0;
        }
    }
    // TRACE-DISINTEGRATE (temporary): a body of a burning or burnt one drawn whole.
    if crate::disintegration::tracing()
        && let Ok(number) = u16::try_from(entity.id.get().saturating_sub(1))
        && sinks.forced.watching(
            number,
            state
                .filter(|_| entity.kind == EntityKind::Corpse)
                .map(|state| state.client_num()),
        )
    {
        crate::disintegration::trace(format_args!(
            "{number} ({:?}, client {:?}, flags {:?}) drawn whole at {} with mesh {mesh:?}, at {:?}",
            entity.kind,
            state.map(|state| state.client_num()),
            state.map(|state| state.e_flags()),
            sinks.presentation_time,
            transform.translation
        ));
    }
    // A monster's victim is drawn in its hand or jaw (`cg_players.c:9220-9244`).
    let local = Some(entity.id.get()) == local_entity_id;
    let trick = trick_fade(sinks, snapshot, state, local, presentation_time);
    if let Some(snapshot) = snapshot.filter(|_| local || state.is_some()) {
        monster_hold::place(
            sinks.world,
            sinks.actor_meshes,
            snapshot,
            state.filter(|_| !local),
            transform,
            presentation_time,
        );
    }
    if let Some(snapshot) = snapshot {
        player_sprites::submit(
            sinks,
            entity.kind,
            transform.translation,
            snapshot,
            state,
            local,
            draw_actor,
            visual_now,
        );
    }
    // A JA+ hook's rope runs from the right hand (`cg_ents.c:2785-2792`).
    if entity.kind == EntityKind::Actor
        && let Some(attachment) =
            mesh.and_then(|mesh| sinks.actor_meshes[mesh].weapon_attachments[0])
        && let Ok(client) = u16::try_from(entity.id.get().saturating_sub(1))
    {
        let (hand, _) = saber::world_attachment(
            Vec3::from_array(transform.translation),
            weapon_view::actor_world_rotation(transform.rotation),
            attachment.scaled(transform.scale),
        );
        grapple_rope::submit(sinks, client, hand, visual_now);
    }
    if let (Some(mesh), Some(snapshot)) = (mesh, snapshot) {
        flags::submit(
            sinks,
            mesh,
            entity,
            *transform,
            snapshot,
            presentation_time,
            draw_actor,
        );
        force_powers::submit(
            sinks,
            mesh,
            entity,
            *transform,
            snapshot,
            presentation_time as i32,
            visual_now,
        );
    }
    let mut equipment = if Some(entity.id.get()) == local_entity_id {
        sinks.local_equipment
    } else {
        entity.equipment()
    };
    if let Some(body) = mesh.and_then(|index| sinks.actor_meshes[index].body_identity.as_ref()) {
        // CG_BodyQueueCopy removes dropped weapons above WP_BRYAR_PISTOL.
        equipment = equipment
            .filter(|_| (1..=4).contains(&body.weapon))
            .map(|mut held| {
                held.weapon = body.weapon as u8;
                held.kind = if body.weapon == 3 {
                    sjk_runtime::HeldItemKind::EnergyBlade
                } else {
                    sjk_runtime::HeldItemKind::Ranged
                };
                held.active = false;
                held.secondary_active = false;
                held.primary_in_flight = false;
                held
            });
    }
    // The weapon went with a cut-off arm, hand or waist (`CG_General`).
    if mesh.is_some_and(|index| sinks.actor_meshes[index].surfaces.weapon_lost) {
        equipment = None;
    }
    // An NPC's blades without a colour override wear their `.sab` colour, and with
    // no saber entity its primary saber is not drawn (`sjk_client::npc_identity`).
    let npc = state.filter(|state| state.entity_type() == 13);
    let npc_primary_missing = npc.is_some_and(sjk_client::legacy_npc_primary_saber_missing);
    if let (Some(npc), Some(catalog), Some(index)) = (npc, sinks.saber_hilts, mesh) {
        let overrides = sjk_client::legacy_npc_saber_color_overrides(npc);
        let names = &sinks.actor_meshes[index].saber_names;
        let definition = |saber: usize| {
            names[saber].as_deref().map(|name| {
                sjk_client::legacy_npc_definition_rgb(
                    catalog.get_any_case(name).default_color,
                    npc.number(),
                )
            })
        };
        equipment = equipment.map(|mut held| {
            if overrides[0].is_none()
                && let Some(rgb) = definition(0)
            {
                held.color = rgb;
            }
            if overrides[1].is_none()
                && let Some(rgb) = definition(1)
            {
                held.secondary_color = rgb;
            }
            held
        });
    }
    // A player muted on this PC, or their body, has the default blade colour.
    let client = match mesh.and_then(|index| sinks.actor_meshes[index].body_identity.as_ref()) {
        Some(body) => u64::from(body.client_num) + 1,
        None if entity.kind == EntityKind::Actor => entity.id.get(),
        None => 0,
    };
    let muted = crate::muted_players::entity_muted(sinks.muted, client);
    if muted {
        equipment = equipment.map(crate::muted_players::equipment);
    }
    // In team games the blades take the team's colour, muted or not
    // (`cg_saberTeamColors`, `cg_ctfTeamSabers`), and in Siege the class's forced
    // colour (`siege_sabers.rs`): the game sets those, not the player.
    let team_blade = crate::team_sabers::of(&sinks.team_blades, client);
    if team_blade.any() {
        equipment = equipment.map(|mut held| {
            held.color = team_blade.color(0, held.color);
            held.secondary_color = team_blade.color(1, held.secondary_color);
            held
        });
    }
    let holster = mesh.map_or_else(holster_submission::Plan::default, |index| {
        holster_submission::submit(
            sinks,
            index,
            entity,
            *transform,
            snapshot,
            state,
            equipment,
            local,
            draw_actor,
            trick.hidden || trick.fading,
        )
    });
    if holster.blades_in {
        equipment = equipment.map(|mut held| {
            held.active = false;
            held.secondary_active = false;
            held
        });
    }
    if holster.on_back {
        // Do not retain a lit blade or trail while its hilt moves onto the back.
        for blade in 0..8 {
            if let Some(state) = sinks.saber_states.blade_mut(entity.id.get(), 0, blade) {
                state.stow(presentation_time);
            }
        }
    }
    // A hidden trickster keeps only a saber in flight (`cg_players.c:11732-11746`).
    if let Some((equipment, held_mesh)) = equipment
        .filter(|held| !trick.hidden || held.primary_in_flight)
        .zip(mesh)
    {
        // Whether its saber is in an attack (`saberMove`), for the blade skins that answer a
        // swing: only an attack is one (Creyon's round 14: a run's arm sway set the Clone
        // Wars blade's words off); unknown without a snapshot.
        let saber_move = if local {
            snapshot.map(|snapshot| snapshot.player.saber_move())
        } else {
            state.map(sjk_protocol::EntityState::saber_move)
        };
        let attacking = saber_move.map(|saber_move| {
            sjk_game_jka::saber_rules::in_attack(saber_move)
                || sjk_game_jka::saber_rules::in_special(saber_move)
        });
        submit_equipment(
            sinks,
            entity,
            equipment,
            held_mesh,
            *transform,
            draw_actor,
            local,
            holster.on_back || npc_primary_missing,
            trick.fading.then_some(trick.alpha),
            presentation_time,
            visual_now,
            attacking,
        );
    }
    let ghosts = match (mesh, snapshot) {
        (Some(_), Some(snapshot)) => {
            speed_ghosts(sinks, transform, snapshot, state, local, trick.fading)
        }
        _ => [None; 2],
    };
    let number = u16::try_from(entity.id.get().saturating_sub(1)).unwrap_or(u16::MAX);
    let e_flags = if local {
        snapshot.map_or(0, |snapshot| snapshot.player.entity_flags())
    } else {
        state.map_or(0, sjk_protocol::EntityState::e_flags)
    };
    if entity.kind == EntityKind::Actor
        && let (Some(mesh), Some(snapshot)) = (mesh, snapshot)
    {
        let tricked = !local
            && state.is_some_and(|state| {
                sjk_client::legacy_mind_tricked(
                    sjk_client::legacy_entity_trick_targets(state),
                    snapshot.player.client_num(),
                    sinks.viewer_force_powers_active,
                )
            });
        jetpack_submission::submit(
            sinks,
            jetpack_submission::Body {
                mesh,
                number,
                transform: *transform,
                e_flags,
                tricked,
                hidden: trick.hidden,
                fade: trick.fading.then_some(trick.alpha),
                draw_actor,
                muted,
            },
            presentation_time as i32,
            visual_now,
        );
    }
    // The boombox on a carrier's shoulder, as their jetpack: none on the dead.
    if entity.kind == EntityKind::Actor
        && let (Some(mesh), (Some(boombox), carriers)) = (mesh, sinks.boombox)
        && number < 64
        && carriers & 1 << number != 0
        && e_flags & EF_DEAD_FLAG == 0
        && !trick.hidden
    {
        let actor = &sinks.actor_meshes[mesh];
        if let Some(mut instance) = actor.animator.requested.and_then(|_| {
            crate::boombox::instance(&actor.preview, actor.animator.matrices(), *transform)
        }) {
            instance.view_flags = sinks.entity_view_flags | u32::from(!draw_actor);
            sinks.object_groups[boombox].push(instance);
        }
    }
    // A fisher's rod in their right hand and the line from its tip, as the boombox: the
    // rod in the hand only in mirrors in first person, where the own rod is held in view
    // instead (and the line runs from that one's tip).
    if entity.kind == EntityKind::Actor
        && let (Some(mesh), (Some(rod), Some(line_mesh), lines)) = (mesh, sinks.fishing)
        && number < 64
        && let Some(end) = lines.ends[usize::from(number)]
        && e_flags & EF_DEAD_FLAG == 0
        && !trick.hidden
    {
        let actor = &sinks.actor_meshes[mesh];
        if let Some((mut instance, tip)) = actor.animator.requested.and_then(|_| {
            crate::fishing::rod_in_hand(
                &actor.preview,
                actor.animator.matrices(),
                *transform,
                lines.bend[usize::from(number)],
            )
        }) {
            instance.view_flags = sinks.entity_view_flags | u32::from(!draw_actor);
            sinks.object_groups[rod].push(instance);
            let mut tip = tip;
            if !draw_actor && Some(entity.id.get()) == local_entity_id {
                let (mut held, held_tip) = crate::fishing::rod_in_view(
                    sinks.camera_position,
                    sinks.camera_yaw,
                    sinks.camera_pitch,
                    lines.bend[usize::from(number)],
                );
                held.view_flags = sinks.entity_view_flags;
                sinks.object_groups[rod].push(held);
                tip = held_tip;
            }
            if let Some(mut line) = crate::fishing::line(tip, Vec3::from_array(end)) {
                line.view_flags = sinks.entity_view_flags;
                sinks.object_groups[line_mesh].push(line);
            }
            sinks.fishing_tips[usize::from(number)] = Some(tip.to_array());
        }
    }
    // A miner's pickaxe in their right hand, turned with it as a hilt is, so it follows
    // the hold and the swing; in first person too, where the own hands hold it. The
    // weapon or saber that hand held is not drawn (`submit_weapons`).
    if entity.kind == EntityKind::Actor
        && let (Some(mesh), (Some(pick), picks)) = (mesh, sinks.mining)
        && number < 64
        && picks.held[usize::from(number)]
        && e_flags & EF_DEAD_FLAG == 0
        && !trick.hidden
    {
        let actor = &sinks.actor_meshes[mesh];
        if let Some(mut instance) = actor.animator.requested.and_then(|_| {
            crate::mining::pick_in_hand(
                &actor.preview,
                actor.animator.matrices(),
                *transform,
                picks.turn,
            )
        }) {
            instance.view_flags = sinks.entity_view_flags;
            sinks.object_groups[pick].push(instance);
        }
    }
    // Everything after stock's mind-trick cut-off is skipped for a hidden trickster:
    // the body, its afterimages and its shells (`cg_players.c:11351-11356`).
    if trick.hidden {
        return 0;
    }
    // A Grip victim's puffs come after that cut-off too (`cg_players.c:13221`), so a
    // gripped trickster hidden from the viewer gives away nothing.
    if entity.kind == EntityKind::Actor
        && let (Some(mesh), Some(snapshot)) = (mesh, snapshot)
    {
        force_powers::victim::submit(
            sinks,
            mesh,
            entity,
            *transform,
            snapshot,
            presentation_time as i32,
            visual_now,
        );
    }
    // An electrified player crackles (`electrify.rs`), the local one in first person too.
    if entity.kind == EntityKind::Actor
        && !muted
        && let Some(snapshot) = snapshot
    {
        let until = if local {
            snapshot.player.electrify_time()
        } else {
            state.map_or(0, sjk_protocol::EntityState::emplaced_owner)
        };
        let now = presentation_time as i32;
        if until > now
            && sinks.effect_aux.continuous.due(visual_now)
            && crate::electrify::crackles(until, now, crate::electrify::draws(number, now))
            && let Some(audio) = sinks.game_audio.as_mut()
        {
            audio.play_on_channel(
                crate::electrify::CRACKLE,
                1.0,
                transform.translation,
                sjk_audio::SourceId(u32::from(number)),
                sjk_audio::ChannelId(0),
            );
        }
    }
    // `CG_DrawPlayerSphere`, the local player's included in first person.
    if entity.kind == EntityKind::Actor
        && let Some(snapshot) = snapshot
    {
        let powerups = if local {
            (0..16).fold(0, |bits, powerup| {
                bits | u32::from(snapshot.player.powerups[powerup] != 0) << powerup
            })
        } else {
            state.map_or(0, sjk_protocol::EntityState::powerups)
        };
        player_spheres::submit(
            sinks,
            Vec3::from_array(transform.translation),
            e_flags,
            powerups,
        );
    }
    if let (Some(mesh), Some(snapshot)) = (mesh, snapshot) {
        force_powers::submit_confusion(
            sinks,
            mesh,
            entity,
            *transform,
            snapshot,
            presentation_time as i32,
            visual_now,
        );
    }
    let saber_body = local && sinks.first_person_saber;
    if (draw_actor || saber_body || sinks.portal_view)
        && let Some(mesh) = mesh
    {
        let mut instance = ActorInstance::new(
            transform.translation,
            weapon_view::actor_world_rotation(transform.rotation).to_array(),
            transform.scale,
        )
        .with_model_color(entity.color());
        let duel = snapshot.map_or_else(DuelBody::default, |snapshot| {
            duel_body(snapshot, state, local, entity.color(), sinks.duel_ghosts)
        });
        let inside_body = local
            && sinks.detached_flight
            && !limp(&sinks.actor_meshes[mesh])
            && crate::free_camera::inside_body(
                sinks.camera_position,
                Vec3::from_array(transform.translation),
                sinks.view_height,
            );
        // The first-person saber body is never on with a detached camera, so it never
        // meets the free camera's hidden body.
        instance.view_flags =
            sinks.entity_view_flags | u32::from((!draw_actor && !saber_body) || inside_body);
        // The body as drawn: flat in the duel colour where `CG_Player` tints it. The
        // shells over it and the outline mask keep the untinted instance.
        let body = match duel.tint {
            Some([r, g, b]) => {
                let [r, g, b, a] = [r, g, b, entity.color()[3]].map(|c| f32::from(c) / 255.0);
                instance.with_rgb_tint([r, g, b, a])
            }
            None => instance,
        };
        if trick.fading {
            // `RF_FORCE_ENT_ALPHA` at `trickAlpha` (`cg_players.c:11358-11367`).
            if sinks.overrides.len() < sinks.overrides.capacity() {
                sinks.overrides.push(entity_materials::OverrideInstance {
                    mesh: entity_materials::OverrideMesh::Actor(mesh),
                    material: None,
                    instance: body.with_forced_alpha(trick.alpha),
                    no_depth: false,
                    forced_alpha: true,
                });
            }
        } else if duel.ghost {
            // JoF EJK's duel figure in place of the body (`crate::duel_ghost`).
            if let Some(material) = sinks.material_overrides.force(crate::duel_ghost::SHADER)
                && sinks.overrides.len() < sinks.overrides.capacity()
            {
                sinks.overrides.push(entity_materials::OverrideInstance {
                    mesh: entity_materials::OverrideMesh::Actor(mesh),
                    material: Some(material),
                    instance,
                    no_depth: false,
                    forced_alpha: false,
                });
            }
        } else {
            sinks.actor_groups[mesh].push(body);
            // A player outline's mask: the body again, drawn only into the mask; the
            // card's outline before a team's.
            let client = entity.id.get().wrapping_sub(1);
            let outline = if sinks
                .card_outline
                .is_some_and(|card| u64::from(card) == client)
            {
                Some(crate::card_outline::Group::Card)
            } else {
                sinks.team_outline.group(client)
            };
            if draw_actor
                && let Some(group) = outline
                && sinks.overrides.len() < sinks.overrides.capacity()
            {
                sinks.overrides.push(entity_materials::OverrideInstance {
                    mesh: entity_materials::OverrideMesh::Actor(mesh),
                    material: Some(sinks.material_overrides.outline(group)),
                    instance,
                    no_depth: false,
                    forced_alpha: false,
                });
            }
        }
        // JoF EJK's hat and cape, on the body's bolts (`CG_Player`); none on a
        // player muted on this PC.
        if entity.kind == EntityKind::Actor && !muted {
            // `EF_DEAD`; the local player's flags are its player state's.
            const EF_DEAD: u32 = 1 << 1;
            let flags = if local {
                snapshot.map(|snapshot| snapshot.player.entity_flags())
            } else {
                state.map(sjk_protocol::EntityState::e_flags)
            };
            crate::cosmetics::actors::submit(
                &sinks.actor_meshes[mesh].cosmetics,
                &crate::cosmetics::actors::Frame {
                    transform: *transform,
                    visibility: sinks.cosmetics,
                    local,
                    draw_actor,
                    view_flags: sinks.entity_view_flags,
                    tricked: trick.fading,
                    dead: flags.is_some_and(|flags| flags & EF_DEAD != 0),
                    state_visible: snapshot.is_none_or(|snapshot| {
                        crate::cosmetics::actors::visible_in_state(&snapshot.player, state, local)
                    }),
                },
                sinks.object_groups,
            );
        }
        // The copies share the actor's pose, scale, colour (its duel tint too) and view
        // flags; a duellist drawn as JoF's figure leaves none of its solid body behind.
        for ghost in ghosts.into_iter().flatten().filter(|_| !duel.ghost) {
            if sinks.overrides.len() == sinks.overrides.capacity() {
                break;
            }
            let mut copy = body.with_forced_alpha(ghost.alpha);
            copy.position = ghost.origin.to_array();
            sinks.overrides.push(entity_materials::OverrideInstance {
                mesh: entity_materials::OverrideMesh::Actor(mesh),
                material: None,
                instance: copy,
                no_depth: false,
                forced_alpha: true,
            });
        }
        sinks.actor_meshes[mesh]
            .retained_pose
            .mark_drawn(entity.id, presentation_time);
        // The Monitor's Screen Saver: Force Absorb's body shell, gold, as bright as the
        // toy says (it pulses, and flickers before it ends).
        let glow = usize::try_from(entity.id.get().wrapping_sub(1))
            .ok()
            .and_then(|slot| sinks.screen_savers.0.get(slot).copied())
            .unwrap_or(0);
        if glow > 0
            && draw_actor
            && !inside_body
            && entity.kind == EntityKind::Actor
            && let Some(material) = sinks.material_overrides.force("gfx/misc/personalshield")
        {
            let gold = [
                glow,
                (f32::from(glow) * 0.72) as u8,
                (f32::from(glow) * 0.12) as u8,
                255,
            ];
            for _ in 0..sinks.shield_passes.max(1) {
                if sinks.overrides.len() == sinks.overrides.capacity() {
                    break;
                }
                sinks.overrides.push(entity_materials::OverrideInstance {
                    mesh: entity_materials::OverrideMesh::Actor(mesh),
                    material: Some(material),
                    instance: instance.with_entity_color(gold),
                    no_depth: false,
                    forced_alpha: false,
                });
            }
        }
        if let (Some(snapshot), Some(game_state)) = (snapshot, game_state) {
            return force_overlay_submission::submit(
                sinks.overrides,
                mesh,
                instance,
                entity.id,
                snapshot,
                game_state,
                sinks.force_tracker,
                sinks.material_overrides,
                presentation_time as i32,
                sinks.third_person,
                aura_shell,
                combined_protect_absorb,
                sinks.predicted_force_powers_active,
                sinks.shield_mesh,
                sinks.shield_sphere,
                sinks.shield_passes,
                sinks.duel_ghosts,
            );
        }
    } else if draw_actor {
        sinks.entity_instances.push(EntityInstance {
            position: transform.translation,
            kind: 0,
            size: 1.0,
            alpha: 1.0,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            color: [1.0; 4],
            direction: [0.0; 3],
            rotation: 0.0,
            uv_transform: [1.0, 1.0, 0.0, 0.0],
        });
    }
    0
}

/// What the duel rules do to one actor's body.
#[derive(Clone, Copy, Default)]
struct DuelBody {
    /// `RF_RGB_TINT` (`sjk_client::legacy_duel_tint`).
    tint: Option<[u8; 3]>,
    /// Drawn as JoF EJK's blue figure instead (`sjk_client::legacy_duel_ghost`).
    ghost: bool,
}

/// `CG_Player`'s duel rules for one actor's body.
fn duel_body(
    snapshot: &Snapshot,
    state: Option<&sjk_protocol::EntityState>,
    local: bool,
    color: [u8; 4],
    ghosts: bool,
) -> DuelBody {
    // `EF_DEAD`.
    const EF_DEAD: u32 = 1 << 1;
    let player = &snapshot.player;
    let actor = if local {
        sjk_client::DuelActor {
            number: player.client_num(),
            duelling: player.duel_in_progress(),
            dead: player.health() <= 0,
            color: [color[0], color[1], color[2]],
        }
    } else {
        // Only players duel: a mover or turret uses `bolt1` for itself.
        const ET_PLAYER: u8 = 1;
        let Some(state) = state.filter(|state| state.entity_type() == ET_PLAYER) else {
            return DuelBody::default();
        };
        sjk_client::DuelActor {
            number: state.number(),
            duelling: state.bolt1(),
            dead: state.e_flags() & EF_DEAD != 0,
            color: [color[0], color[1], color[2]],
        }
    };
    let viewer = sjk_client::DuelViewer {
        client: player.client_num(),
        duelling: player.duel_in_progress(),
        opponent: player.duel_index(),
        ghosts,
    };
    DuelBody {
        tint: sjk_client::legacy_duel_tint(actor, viewer),
        ghost: sjk_client::legacy_duel_ghost(actor, viewer),
    }
}

/// Run `CG_Player`'s mind-trick fade for one actor. The local player never tricks
/// itself; others fade while they trick the viewer (`cg_players.c:10191-10345`).
fn trick_fade(
    sinks: &mut Sinks<'_>,
    snapshot: Option<&Snapshot>,
    state: Option<&sjk_protocol::EntityState>,
    local: bool,
    presentation_time: i64,
) -> sjk_client::LegacyTrickFade {
    let (Some(snapshot), Some(state)) = (snapshot, state) else {
        return sjk_client::LegacyTrickFade::OPAQUE;
    };
    if local {
        return sjk_client::LegacyTrickFade::OPAQUE;
    }
    let tricked = sjk_client::legacy_mind_tricked(
        sjk_client::legacy_entity_trick_targets(state),
        snapshot.player.client_num(),
        sinks.viewer_force_powers_active,
    );
    sinks
        .trick_fades
        .advance(state.number(), tricked, presentation_time as i32)
}

/// Advance the Force Speed trail of one actor that stock passes through
/// `CG_Player`. The local player is `cg.predictedPlayerEntity`, whose state
/// comes from the predicted player state; others use their snapshot state.
fn speed_ghosts(
    sinks: &mut Sinks<'_>,
    transform: &sjk_runtime::Transform,
    snapshot: &Snapshot,
    state: Option<&sjk_protocol::EntityState>,
    local: bool,
    trick_fading: bool,
) -> [Option<speed_trail::Ghost>; 2] {
    // `PW_SPEED`, as `BG_PlayerStateToEntityState` maps powerups to bits.
    const PW_SPEED: usize = 10;
    let local_client = snapshot.player.client_num();
    let (number, velocity, trailing) = if local {
        let velocity = sinks
            .predicted_local_state
            .map_or_else(|| snapshot.player.velocity(), |state| state.velocity);
        let trailing = snapshot.player.powerups[PW_SPEED] != 0;
        (local_client, velocity, trailing)
    } else if let Some(state) = state {
        // Stock's `doAlpha`: the trick fade, out and back in (`cg_players.c:10842`).
        let trailing = state.powerups() & (1 << PW_SPEED) != 0 && !trick_fading;
        (state.number(), state.trajectory_delta(), trailing)
    } else {
        return [None; 2];
    };
    sinks.speed_trails.advance(
        number,
        Vec3::from_array(transform.translation),
        Vec3::from_array(velocity),
        trailing && sinks.speed_trail,
    )
}

/// Whether the player's body is drawn as their ragdoll lies (the Ragdoll toy): a free
/// camera never starts in that body's head, which is not where the server holds the
/// player, so it is never hidden from the camera (Dave's test, 27/10/2026: a ragdoll
/// vanished while the camera flew above it). Only a body the ragdoll posed counts: a
/// model off the stock skeleton stays where the server holds it, and is hidden as
/// usual where the flight starts in its head (#232's review).
fn limp(mesh: &crate::actor_mesh::ActorMesh) -> bool {
    mesh.ragdoll.posed
}

#[allow(clippy::too_many_arguments)]
fn submit_equipment(
    sinks: &mut Sinks<'_>,
    entity: &sjk_runtime::SceneEntity,
    equipment: sjk_runtime::HeldEquipment,
    actor_mesh: usize,
    transform: sjk_runtime::Transform,
    draw_actor: bool,
    local: bool,
    holstered_primary: bool,
    forced_alpha: Option<u8>,
    presentation_time: i64,
    visual_now: Instant,
    attacking: Option<bool>,
) {
    let rotation = weapon_view::actor_world_rotation(transform.rotation);
    let origin = Vec3::from_array(transform.translation);
    // A held prop gun replaces the weapon or saber in the hand (`propgun`); in first
    // person the view model stands in for it.
    let prop = usize::try_from(entity.id.get())
        .ok()
        .and_then(|id| id.checked_sub(1))
        .and_then(|slot| sinks.propgun.get(slot).copied().flatten());
    if let Some(prop) = prop {
        if (!draw_actor && !sinks.portal_view) || limp(&sinks.actor_meshes[actor_mesh]) {
            return;
        }
        if let Some(attachment) = sinks.actor_meshes[actor_mesh].weapon_attachments[0] {
            let (grip, hand) =
                saber::world_attachment(origin, rotation, attachment.scaled(transform.scale));
            let scale = Vec3::from_array(transform.scale) * prop.scale;
            let hand = hand * crate::propgun_frame::HAND_TURN;
            let at = grip + hand * (prop.offset * scale);
            let mut instance = ActorInstance::new(
                at.to_array(),
                (hand * prop.rotation).normalize().to_array(),
                scale.to_array(),
            );
            instance.view_flags = sinks.entity_view_flags;
            sinks.object_groups[prop.mesh].push(instance);
        }
        return;
    }
    // A pickaxe in the hand (`mining`) replaces the weapon or saber there.
    if usize::try_from(entity.id.get())
        .ok()
        .and_then(|id| id.checked_sub(1))
        .is_some_and(|slot| sinks.mining.1.held.get(slot).copied().unwrap_or(false))
    {
        return;
    }
    let team_plain = crate::team_sabers::of(&sinks.team_blades, entity.id.get()).plain;
    if saber_submission::submit(
        entity.id.get(),
        sinks
            .saber_skins
            .get_both(entity.id.get())
            .map(|skin| skin.filter(|_| !team_plain)),
        // A team colour replaces a black blade too (it is past `SABER_PURPLE`).
        sinks.black_sabers.without(
            entity.id.get(),
            crate::team_sabers::of(&sinks.team_blades, entity.id.get())
                .colors
                .map(|color| color.is_some()),
        ),
        std::array::from_fn(|hand| {
            if hand == 0 && holstered_primary {
                return None;
            }
            let mesh = &sinks.actor_meshes[actor_mesh];
            mesh.saber_names[hand]
                .as_deref()
                .zip(mesh.weapon_attachments[hand].map(|bolt| bolt.scaled(transform.scale)))
        }),
        equipment,
        origin,
        rotation,
        transform.scale,
        sinks.saber_hilts,
        sinks.saber_states,
        sinks.saber_segments,
        sinks.object_groups,
        sinks.saber_instances,
        presentation_time,
        sinks.trail_edges.as_mut(),
        sinks.lights,
        attacking,
        sinks.saber_core,
        &mut crate::saber_smoke::Sink {
            particles: sinks.particles,
            effects: sinks.effects,
            atlas: Some(sinks.particle_atlas),
            ribbons: sinks.smoke_ribbons,
            sparks: sinks.saber_sparks,
            clashes: sinks.saber_clashes,
            heard: Some(&mut *sinks.saber_heard),
            now: visual_now,
        },
    ) {
        return;
    }
    // The local first-person player's model carries `RF_THIRD_PERSON`
    // (`cg_players.c:8901-8911`, sabers excepted), so its bolted world gun is
    // not drawn and the world flash is skipped (`cg_weapons.c:701-703`); the
    // view model and its `tag_flash` stand in (`first_person_weapon.rs`).
    if !draw_actor && !sinks.portal_view {
        return;
    }
    let Some(attachment) = sinks.actor_meshes[actor_mesh].weapon_attachments[0] else {
        return;
    };
    let Some(model) = weapon_view::held_model(equipment.weapon) else {
        return;
    };
    let (grip, weapon_rotation) =
        saber::world_attachment(origin, rotation, attachment.scaled(transform.scale));
    let Some(weapon_mesh) = sinks
        .object_meshes
        .iter()
        .position(|mesh| mesh.appearance.variant.is_empty() && mesh.appearance.model == model)
    else {
        return;
    };
    let mut instance =
        ActorInstance::new(grip.to_array(), weapon_rotation.to_array(), transform.scale);
    let inside_body = local
        && sinks.detached_flight
        && !limp(&sinks.actor_meshes[actor_mesh])
        && crate::free_camera::inside_body(
            sinks.camera_position,
            Vec3::from_array(transform.translation),
            sinks.view_height,
        );
    instance.view_flags = sinks.entity_view_flags | u32::from(!draw_actor || inside_body);
    match forced_alpha {
        // The gun is a bolt-on of the body's Ghoul2 instance in stock, so it fades with it.
        Some(alpha) if sinks.overrides.len() < sinks.overrides.capacity() => {
            sinks.overrides.push(entity_materials::OverrideInstance {
                mesh: entity_materials::OverrideMesh::Object(weapon_mesh),
                material: None,
                instance: instance.with_forced_alpha(alpha),
                no_depth: false,
                forced_alpha: true,
            });
        }
        Some(_) => {}
        None => sinks.object_groups[weapon_mesh].push(instance),
    }
    // The main first-person flash already owns event/audio spawning. A
    // second view must never advance effects or play a second sound.
    if !draw_actor {
        return;
    }
    let client = u16::try_from(entity.id.get().saturating_sub(1)).ok();
    let request = client.and_then(|client| sinks.muzzle_effects.request(client));
    let socket = sinks.object_meshes[weapon_mesh]
        .flash_bolt
        .and_then(|flash| muzzle_flash::world_socket(flash, grip, weapon_rotation));
    if let (Some(charge), Some(socket)) = (client.and_then(|c| sinks.charges.of(c)), socket) {
        crate::charge_flash::push(
            sinks.particles,
            sinks.effects,
            charge,
            Vec3::from_array(socket.origin),
            presentation_time as i32,
            visual_now,
        );
    }
    if let (Some(request), Some(socket)) = (request, socket) {
        muzzle_effects::spawn(
            sinks.particles,
            sinks.effect_aux,
            sinks.effects,
            sinks.vfs,
            sinks.game_audio,
            request,
            socket,
            local,
            visual_now,
            presentation_time as i32,
        );
    }
}

/// `CG_Disintegration` (`cg_ents.c`): the burning pass with `gfx/effects/burn`, the
/// body eaten away from the hit point, and puffs of `disruptor/death_smoke` from the
/// lower back for the first second. A player is gone after 1.5 s; a body draws until
/// the server frees it.
fn submit_disintegration(
    sinks: &mut Sinks<'_>,
    mesh: usize,
    state: &crate::disintegration::State,
    entity: &sjk_runtime::SceneEntity,
    transform: &sjk_runtime::Transform,
    draw_actor: bool,
    visual_now: Instant,
) {
    use crate::disintegration::{PLAYER_MILLIS, RF_DISINTEGRATE1, RF_DISINTEGRATE2, SMOKE_EFFECT};
    let now = sinks.presentation_time;
    // A body this client burnt away itself goes too: the server keeps it.
    if (entity.kind == EntityKind::Actor || state.forced) && now - state.started > PLAYER_MILLIS {
        return;
    }
    // TRACE-DISINTEGRATE (temporary): a forced one drawn in its burn.
    if state.forced {
        crate::disintegration::trace(format_args!(
            "{} ({:?}) drawn burning at {now}, {} ms in",
            entity.id.get().saturating_sub(1),
            entity.kind,
            now - state.started
        ));
    }
    if !(draw_actor || sinks.portal_view) {
        return;
    }
    let rotation = weapon_view::actor_world_rotation(transform.rotation);
    let mut body = ActorInstance::new(transform.translation, rotation.to_array(), transform.scale)
        .with_model_color(entity.color());
    body.view_flags = sinks.entity_view_flags | u32::from(!draw_actor);
    let mut burn = body;
    state.mark(&mut burn, RF_DISINTEGRATE2, now);
    if sinks.overrides.len() < sinks.overrides.capacity() {
        sinks.overrides.push(entity_materials::OverrideInstance {
            mesh: entity_materials::OverrideMesh::Actor(mesh),
            material: Some(sinks.material_overrides.disruptor_burn),
            instance: burn,
            no_depth: false,
            forced_alpha: false,
        });
    }
    state.mark(&mut body, RF_DISINTEGRATE1, now);
    sinks.actor_groups[mesh].push(body);
    sinks.actor_meshes[mesh]
        .retained_pose
        .mark_drawn(entity.id, now);
    let Some(lumbar) = sinks.actor_meshes[mesh].force_bones.lumbar else {
        return;
    };
    if !state.smoke_due(now) {
        return;
    }
    // `fxOrg`: the lower_lumbar bolt, 18 units toward the viewer, up or down by up
    // to 20; one puff, and a second half the time.
    let local =
        Vec3::from_array(crate::bolt::column(&lumbar, 3)) * Vec3::from_array(transform.scale);
    let yaw = sinks.camera_yaw;
    let toward_viewer = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
    let seed = (entity.id.get() as u32).wrapping_mul(0x9e37_79b9) ^ (now as u32);
    let unit = |salt: u32| {
        let mixed = seed
            .wrapping_add(salt)
            .wrapping_mul(0x85eb_ca6b)
            .rotate_left(13);
        (mixed >> 8) as f32 / (1 << 24) as f32
    };
    let origin = Vec3::from_array(transform.translation) + rotation * local - toward_viewer * 18.0
        + Vec3::Z * ((unit(1) * 2.0 - 1.0) * 20.0);
    let puffs = if unit(2) > 0.5 { 2 } else { 1 };
    for puff in 0..puffs {
        effect_runtime::spawn_effect(
            sinks.particles,
            sinks.effect_aux,
            sinks.effects,
            sinks.vfs,
            SMOKE_EFFECT,
            origin,
            visual_now,
            seed.wrapping_add(puff),
            0,
            sinks.game_audio,
            combat_effects::rotation_from_direction([0.0, 1.0, 0.0]),
        );
    }
}

/// An emplaced gun whose top a gunner has turned ([`crate::emplaced_swivel`]). The
/// gunner's view is this client's own for the local player and the player's view angles
/// for anyone else (`cg_ents.c:1401-1409`). Returns whether `entity` was drawn so.
fn submit_emplaced(
    sinks: &mut Sinks<'_>,
    entity: &sjk_runtime::SceneEntity,
    transform: &sjk_runtime::Transform,
    snapshot: Option<&Snapshot>,
    local_entity_id: Option<u64>,
) -> bool {
    let (Some(snapshot), Some(appearance)) = (snapshot, entity.appearance()) else {
        return false;
    };
    // Every rigid entity comes through here each frame: the snapshot's entities are
    // sorted by number, so look the state up without a linear scan.
    let Some(state) = u16::try_from(entity.id.get().wrapping_sub(1))
        .ok()
        .and_then(|number| {
            snapshot
                .entities
                .binary_search_by_key(&number, |state| state.number())
                .ok()
        })
        .map(|index| &snapshot.entities[index])
    else {
        return false;
    };
    let (world, presentation_time) = (sinks.world, sinks.presentation_time);
    let local_view = [
        -sinks.camera_pitch.to_degrees(),
        sinks.camera_yaw.to_degrees(),
        0.0,
    ];
    crate::emplaced_swivel::submit(
        sinks.emplaced_turned,
        sinks.object_meshes,
        sinks.object_groups,
        state,
        appearance,
        transform,
        sinks.entity_view_flags,
        |gunner| {
            let id = u64::from(gunner) + 1;
            if Some(id) == local_entity_id {
                return Some(local_view);
            }
            world
                .entity(EntityId::new(id))?
                .sample_pose(presentation_time)
                .map(|pose| pose.view_angles_degrees)
        },
    )
}

/// An E-Web set down, its top turned by the server's bone angles ([`crate::eweb_view`]).
/// Returns whether `entity` was drawn so.
fn submit_eweb(
    sinks: &mut Sinks<'_>,
    entity: &sjk_runtime::SceneEntity,
    transform: &sjk_runtime::Transform,
    snapshot: Option<&Snapshot>,
    game_state: Option<&GameState>,
) -> bool {
    let (Some(snapshot), Some(appearance)) = (snapshot, entity.appearance()) else {
        return false;
    };
    if !appearance
        .model
        .eq_ignore_ascii_case(crate::eweb_view::MODEL)
    {
        return false;
    }
    let Some(state) = u16::try_from(entity.id.get().wrapping_sub(1))
        .ok()
        .and_then(|number| {
            snapshot
                .entities
                .binary_search_by_key(&number, |state| state.number())
                .ok()
        })
        .map(|index| &snapshot.entities[index])
    else {
        return false;
    };
    crate::eweb_view::submit(
        sinks.object_meshes,
        sinks.object_groups,
        state,
        appearance,
        transform,
        sinks.entity_view_flags,
        game_state,
    )
}

/// A cut-off limb (`CG_General`'s client-limb case): its own mesh, rooted at the limb
/// surface and pivoting about its bone at the entity's origin, smoking from the cut
/// while it flies. Returns whether `entity` was a limb this viewer draws.
fn submit_limb(
    sinks: &mut Sinks<'_>,
    entity: &sjk_runtime::SceneEntity,
    transform: &sjk_runtime::Transform,
    visual_now: Instant,
) -> bool {
    let meshes = sinks.actor_meshes;
    let Some((index, limb)) = meshes.iter().enumerate().find_map(|(index, mesh)| {
        mesh.limb
            .as_ref()
            .filter(|_| mesh.entity_id == Some(entity.id))
            .map(|limb| (index, limb))
    }) else {
        return false;
    };
    let (position, rotation) = crate::dismember::limb_instance(limb, transform);
    let mut instance =
        ActorInstance::new(position.to_array(), rotation.to_array(), transform.scale)
            .with_model_color(entity.color());
    instance.view_flags = sinks.entity_view_flags;
    sinks.actor_groups[index].push(instance);
    meshes[index]
        .retained_pose
        .mark_drawn(entity.id, sinks.presentation_time);
    if crate::dismember::trail_due(limb, transform.translation, sinks.presentation_time) {
        effect_runtime::spawn_effect(
            sinks.particles,
            sinks.effect_aux,
            sinks.effects,
            sinks.vfs,
            crate::dismember::SMOKE,
            Vec3::from_array(transform.translation),
            visual_now,
            (entity.id.get() as u32) ^ (sinks.presentation_time as u32).rotate_left(9),
            0,
            sinks.game_audio,
            combat_effects::rotation_from_direction([0.0, 1.0, 0.0]),
        );
    }
    true
}
