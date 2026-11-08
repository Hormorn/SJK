//! Ghoul2-compatible actor pose evaluation and GPU upload.

#[path = "actor_sounds.rs"]
pub(crate) mod sounds;

use super::{ActorMesh, GpuState};
use glam::Quat;
use sjk_client::LegacyPlayerAngleController;
use sjk_model::Gla;
use sjk_runtime::{EntityId, EntityKind};
use std::error::Error;

#[path = "force_bones.rs"]
mod force_bones;
pub(crate) use force_bones::ForceBones;
#[cfg(test)]
pub(crate) use force_bones::PUSH_BONE_NAMES;

#[path = "actor_pose_retained.rs"]
pub(crate) mod retained;
pub(crate) use retained::RetainedPose;

#[path = "gpu_skinning.rs"]
pub(crate) mod gpu_skinning;

#[path = "server_bone_angles.rs"]
mod server_bone_angles;

#[path = "actor_pose_steps.rs"]
mod steps;

#[path = "actor_evaluation.rs"]
/// Independent presentation evaluators and persistent ownership-transferring workers.
pub(crate) mod evaluation;

/// Allocate one reusable generic skeleton evaluator for an actor model.
pub(crate) fn storage(animation: &Gla) -> Result<evaluation::Slot, Box<dyn Error>> {
    Ok(evaluation::Slot::new(animation)?)
}

/// Allocate the persistent `cent->pe`-equivalent player-angle state.
pub(crate) fn angle_storage(animation: &Gla) -> LegacyPlayerAngleController {
    let _ = animation;
    LegacyPlayerAngleController::new()
}

/// Replace the snapshot view-yaw root with cgame's evaluated legs yaw.
pub(crate) fn world_rotation(mesh: Option<&ActorMesh>, fallback: [f32; 4]) -> [f32; 4] {
    mesh.and_then(|mesh| mesh.render_yaw_degrees)
        .map_or(fallback, |yaw| {
            let roll = mesh.map_or(0.0, |mesh| mesh.angle_controller.root_roll_degrees());
            (Quat::from_rotation_z(yaw.to_radians()) * Quat::from_rotation_x(roll.to_radians()))
                .to_array()
        })
}

/// Update all live actor poses at the presented cgame time.
pub(crate) fn update(gpu: &mut GpuState, presentation_time: i64) -> Result<(), Box<dyn Error>> {
    let world = gpu
        .demo_session
        .as_ref()
        .map_or(&gpu.live_world, crate::demo_playback::Session::world);
    let local_entity = gpu
        .live_session
        .as_ref()
        .map(|session| EntityId::new(u64::from(session.latest_snapshot().player.client_num()) + 1));
    let authoritative = local_entity
        .and_then(|entity| world.entity(entity))
        .and_then(|entity| entity.animation());
    let authoritative_saber_move = gpu
        .live_session
        .as_ref()
        .map(|session| session.latest_snapshot().player.saber_move());
    let detached_flight = gpu.free_camera_active();
    let local_animation = local_entity.and_then(|entity| {
        gpu.local_actor_state.resolve(
            entity,
            gpu.local_prediction
                .predicted_state()
                .filter(|_| !detached_flight),
            authoritative,
            authoritative_saber_move,
        )
    });
    let predicted = gpu
        .local_prediction
        .predicted_state()
        .filter(|_| !detached_flight);
    // The entity states and configstrings `CG_G2ServerBoneAngles` reads.
    let snapshot = crate::first_person_view::presented_snapshot(
        gpu.live_session.as_ref(),
        gpu.demo_session.as_ref(),
        presentation_time as i32,
    );
    let game_state = gpu
        .live_session
        .as_ref()
        .map(|session| session.game_state())
        .or_else(|| {
            gpu.demo_session
                .as_ref()
                .map(|session| session.game_state())
        });
    let queue = &gpu.queue;
    let vertex_buffer = &gpu.geometry.vertex_buffer;
    // The world's walls, for the facing of a player holding one (`wall_hold_pose`).
    let collision = crate::movement_collision::BspMovementCollision::with_movers(
        &gpu.bsp,
        &mut gpu.trace_scratch,
        &[],
    );
    // Invalidate every entry first, including actors skipped below or after an upload error.
    for mesh in &mut gpu.actor_meshes {
        mesh.retained_pose.invalidate();
        mesh.animator.requested = None;
    }
    // The disintegrations this client starts itself follow the snapshot.
    if let Some(snapshot) = snapshot {
        let local = local_entity.and_then(|id| u16::try_from(id.get().saturating_sub(1)).ok());
        gpu.forced_disintegrations
            .update(snapshot, local, presentation_time);
    }
    let frame = steps::Frame {
        flame: crate::actor_world_submission::force_powers::flame::policy(
            game_state,
            gpu.console.as_ref(),
        ),
        world,
        local_entity,
        local_animation,
        predicted,
        snapshot,
        game_state,
        presentation_time,
        collision: &collision,
    };
    for mesh in &mut gpu.actor_meshes {
        if let Err(error) = steps::prepare(mesh, &frame) {
            steps::failed(mesh, error.as_ref());
        }
        track_disintegration(
            mesh,
            snapshot,
            local_entity,
            &gpu.forced_disintegrations,
            presentation_time,
        );
    }
    gpu.actor_workers
        .evaluate(&mut gpu.actor_meshes, presentation_time);
    // Application is always in actor-vector order, never in worker completion order.
    steps::apply_all(
        &mut gpu.actor_meshes,
        &mut gpu.geometry.skinning,
        queue,
        vertex_buffer,
        presentation_time,
    );
    Ok(())
}

impl GpuState {
    pub(crate) fn update_actor_animations(
        &mut self,
        presentation_time: i64,
        audio: &mut Option<crate::GameAudio>,
    ) -> Result<(), Box<dyn Error>> {
        update(self, presentation_time)?;
        sounds::update(self, presentation_time, audio);
        self.play_emote_sounds(audio);
        self.play_cardboard_box_sounds(audio);
        self.play_fishing_sounds(audio);
        self.play_quest_sounds(audio);
        self.play_order66_sounds(audio);
        self.play_hax_sounds(audio);
        self.play_ragdoll_sounds(audio);
        self.play_drone_sounds(audio);
        self.play_monitor_sounds(audio);
        self.play_banana_sounds(audio);
        self.play_tag_sounds(audio);
        self.play_propgun_sounds(audio);
        self.play_boombox_sounds(audio);
        self.play_mystery_box_sounds(audio);
        self.play_wheel_of_doom_sounds(audio);
        self.play_toy_master_sounds(audio);
        self.spawn_hax_glyphs(audio, presentation_time);
        Ok(())
    }

    pub(crate) fn assign_corpse_meshes(&mut self, presentation_time: i64) {
        self.apply_body_commands(presentation_time);
        for mesh in self
            .actor_meshes
            .iter_mut()
            .filter(|mesh| mesh.corpse_pool && mesh.body_identity.is_none())
        {
            mesh.entity_id = None;
        }
        let world = self
            .demo_session
            .as_ref()
            .map_or(&self.live_world, crate::demo_playback::Session::world);
        for corpse in world
            .entities()
            .filter(|entity| entity.kind == EntityKind::Corpse)
        {
            if self
                .actor_meshes
                .iter()
                .any(|mesh| mesh.entity_id == Some(corpse.id) && mesh.body_identity.is_some())
            {
                continue;
            }
            let Some(appearance) = corpse.appearance() else {
                continue;
            };
            if let Some(mesh) = self.actor_meshes.iter_mut().find(|mesh| {
                mesh.corpse_pool && mesh.entity_id.is_none() && &mesh.appearance == appearance
            }) {
                mesh.entity_id = Some(corpse.id);
            }
        }
        self.track_body_fades();
    }
}

/// Start, follow or end `EF_DISINTEGRATION` for one actor, or a disintegration this
/// client started itself (`forced`). On the first frame the pose freezes at the
/// presented legs frame (`cg_players.c`); when a live player's flag clears (respawn) the
/// frozen animator is replaced by a fresh one.
fn track_disintegration(
    mesh: &mut ActorMesh,
    snapshot: Option<&sjk_protocol::Snapshot>,
    local_entity: Option<EntityId>,
    forced: &crate::disintegration::Forced,
    presentation_time: i64,
) {
    let (Some(snapshot), Some(id)) = (snapshot, mesh.entity_id) else {
        return;
    };
    let Ok(number) = u16::try_from(id.get().saturating_sub(1)) else {
        return;
    };
    let hit = crate::disintegration::hit_location(snapshot, Some(id) == local_entity, number)
        .map(|hit| (hit, presentation_time, false))
        .or_else(|| {
            forced
                .active(number, presentation_time)
                .map(|(hit, started)| (hit, started, true))
        });
    match (hit, mesh.disintegration.as_mut()) {
        (Some((hit, _, _)), Some(state)) => state.hit = hit,
        (Some((hit, started, by_us)), None) => {
            if let Err(error) = mesh
                .animator
                .freeze_for_disintegration(&mesh.preview.animation, presentation_time)
            {
                crate::log::progress(format_args!(
                    "actor {}: disintegration pose not frozen: {error}",
                    number
                ));
            }
            let mut state = crate::disintegration::State::new(started, hit);
            state.forced = by_us;
            crate::disintegration::trace(format_args!(
                "mesh of {number} (corpse pool {}) burns from {started} at {presentation_time}, forced {by_us}",
                mesh.corpse_pool
            ));
            mesh.disintegration = Some(state);
        }
        (None, Some(_)) => {
            crate::disintegration::trace(format_args!(
                "mesh of {number} (corpse pool {}) drawn again at {presentation_time}",
                mesh.corpse_pool
            ));
            mesh.disintegration = None;
            if !mesh.corpse_pool
                && let Ok(fresh) = storage(&mesh.preview.animation)
            {
                mesh.animator = fresh;
            }
        }
        (None, None) => {}
    }
}
