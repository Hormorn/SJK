//! Per-actor preparation and upload. Failures return to the caller's actor loop.
use super::{ActorMesh, gpu_skinning, server_bone_angles};
use crate::{GpuVertex, preview_gpu_vertex};
use sjk_client::pmove::MovementState;
use sjk_protocol::{GameState, Snapshot};
use sjk_runtime::{AnimationState, EntityId, World};
use std::error::Error;

/// `BOTH_CROUCH1IDLE`'s animation number (`anims.h`; `pmove_locomotion.rs`).
const BOTH_CROUCH1IDLE: usize = 1_005;
const CROUCH_IDLE: &str = "BOTH_CROUCH1IDLE";

/// Whether a protocol animation number is one of the standing animations: `BOTH_STAND*`
/// (the idles and their turns) and the saber stances' idles (`BOTH_SABER*_STANCE`), the
/// only ones a boxed player is crouched over.
fn standing_clip(clip: usize) -> bool {
    sjk_client::legacy_animation_name(clip).is_some_and(|name| {
        name.starts_with("BOTH_STAND")
            || (name.starts_with("BOTH_SABER") && name.ends_with("_STANCE"))
    })
}

/// Immutable inputs shared by independently prepared actors in one frame.
pub(super) struct Frame<'a> {
    pub flame: Option<&'static crate::actor_world_submission::force_powers::flame::Contract>,
    pub world: &'a World,
    pub local_entity: Option<EntityId>,
    pub local_animation: Option<AnimationState>,
    pub predicted: Option<&'a MovementState>,
    pub snapshot: Option<&'a Snapshot>,
    pub game_state: Option<&'a GameState>,
    pub presentation_time: i64,
    /// The world, for the wall a player in a wall rebound holds.
    pub collision: &'a crate::movement_collision::BspMovementCollision<'a>,
}

/// Prepare one actor without allowing a bad attachment or angle query to stop its peers.
pub(super) fn prepare(mesh: &mut ActorMesh, frame: &Frame<'_>) -> Result<(), Box<dyn Error>> {
    let Frame {
        flame,
        world,
        local_entity,
        local_animation,
        predicted,
        snapshot,
        game_state,
        presentation_time,
        collision,
    } = *frame;
    // A cut-off limb plays its copied animator; any request lets it evaluate.
    if let Some(limb) = &mesh.limb {
        if mesh.entity_id.is_some() {
            mesh.animator.requested = Some(limb.state);
        }
        return Ok(());
    }
    let frame_millis = mesh.angle_controller.begin_frame(presentation_time);
    let requested = mesh
        .entity_id
        .and_then(|entity_id| world.entity(entity_id))
        .map(|entity| {
            let pose = entity.sample_pose(presentation_time);
            if mesh.entity_id == local_entity {
                let animation = local_animation.or_else(|| entity.animation());
                (animation, crate::local_actor_state::pose(pose, predicted))
            } else {
                (entity.animation(), pose)
            }
        });
    let Some((Some(state), pose)) = requested else {
        mesh.flame_pose = Default::default();
        return Ok(());
    };
    let mut state = state;
    let flame_clip = flame
        .filter(|rule| {
            !mesh.corpse_pool
                && mesh.animator.humanoid()
                && mesh.preview.rig.is_none()
                && snapshot
                    .zip(mesh.entity_id)
                    .and_then(|(snapshot, id)| {
                        crate::actor_world_submission::force_powers::Caster::find(
                            snapshot,
                            id.get().saturating_sub(1) as u16,
                            predicted,
                        )
                    })
                    .is_some_and(|caster| rule.firing(caster))
        })
        .and_then(|rule| {
            let index = rule.pose_index;
            mesh.preview
                .config
                .get_by_index(index)
                .filter(|clip| clip.name == rule.pose)
                .map(|_| index)
        });
    mesh.flame_pose
        .apply(mesh.entity_id, flame_clip, presentation_time, &mut state);
    // A boxed player who stands is drawn crouching: the server stands them up while they
    // talk (the chat, the free camera's balloon; `cardboard_box::crouched_in_box`). Only
    // standing legs are replaced, so a lunge, a kick, a roll or a knockdown plays as
    // sent; the torso follows only when it stands too (a gun's ready pose stays). Clips
    // are protocol animation numbers, looked up by name (`ghoul2_pose.rs`), not
    // `animation.cfg` lines. A rig model follows the stock skeleton that drives it, so
    // it crouches too (a boxed rig model stood up through its carton, 28/10/2026).
    if !mesh.corpse_pool
        && mesh.animator.humanoid()
        && mesh
            .entity_id
            .and_then(|id| usize::try_from(id.get().checked_sub(1)?).ok())
            .is_some_and(crate::cardboard_box::crouched_in_box)
        && standing_clip(state.lower.clip)
        && mesh.preview.config.get_exact(CROUCH_IDLE).is_some()
    {
        let upper = standing_clip(state.upper.clip);
        for (track, crouch) in [(&mut state.lower, true), (&mut state.upper, upper)] {
            if crouch {
                track.clip = BOTH_CROUCH1IDLE;
                track.forced_frame = None;
                track.transition = None;
            }
        }
    }
    // A miner's swing is the whole body's (`mining`): the strong saber style's blow from
    // overhead, from the swing's start, the legs too while they stand (a step plays as sent), blended in
    // over 100 ms as `CG_SetLerpFrameAnimation` blends the torso.
    if !mesh.corpse_pool
        && mesh.animator.humanoid()
        && let Some(slot) = mesh
            .entity_id
            .and_then(|id| usize::try_from(id.get().checked_sub(1)?).ok())
        && let Some(elapsed) = crate::mining::swinging(slot)
        && let Some((swing, name)) = crate::mining::swing_pose()
        && mesh.preview.config.get_exact(name).is_some()
    {
        let started = presentation_time - elapsed;
        let legs = standing_clip(state.lower.clip);
        for (track, play) in [(&mut state.upper, true), (&mut state.lower, legs)] {
            if !play {
                continue;
            }
            let outgoing = *track;
            track.clip = swing;
            track.started_at_millis = started;
            track.phase_millis = 0;
            track.speed_milli = 1_000;
            track.forced_frame = None;
            track.transition = (elapsed < 100 && outgoing.clip != swing).then_some(
                sjk_runtime::AnimationTrackTransition {
                    clip: outgoing.clip,
                    revision: outgoing.revision,
                    started_at_millis: outgoing.started_at_millis,
                    phase_millis: outgoing.phase_millis,
                    speed_milli: outgoing.speed_milli,
                    forced_frame: outgoing.forced_frame,
                    blend_started_at_millis: started,
                    blend_duration_millis: 100,
                },
            );
        }
    }
    // A fisher holds the rod before the chest, as a rifle's ready pose (`fishing`), and a
    // miner the pickaxe (`mining`): only a standing torso, so a jump or a hit plays as
    // sent (and puts the rod away).
    if !mesh.corpse_pool
        && mesh.animator.humanoid()
        && mesh
            .entity_id
            .and_then(|id| usize::try_from(id.get().checked_sub(1)?).ok())
            .and_then(|slot| {
                if crate::mining::holding_pick(slot) {
                    crate::mining::hold_pose()
                } else if crate::fishing::holding_rod(slot) {
                    crate::fishing::hold_pose()
                } else {
                    None
                }
            })
            .is_some_and(|(_, name)| mesh.preview.config.get_exact(name).is_some())
        && standing_clip(state.upper.clip)
        && let Some((hold, _)) = mesh
            .entity_id
            .and_then(|id| usize::try_from(id.get().checked_sub(1)?).ok())
            .and_then(|slot| {
                if crate::mining::holding_pick(slot) {
                    crate::mining::hold_pose()
                } else {
                    crate::fishing::hold_pose()
                }
            })
    {
        state.upper.clip = hold;
        state.upper.forced_frame = None;
        state.upper.transition = None;
    }
    if mesh.corpse_pool && !mesh.body_copied {
        state.lower.forced_frame =
            sjk_client::legacy_body_frame(&mesh.preview.config, state.lower.clip);
        state.upper.forced_frame =
            sjk_client::legacy_body_frame(&mesh.preview.config, state.upper.clip);
        state.lower.transition = None;
        state.upper.transition = None;
    }
    // Non-humanoids skip `CG_G2PlayerAngles` and face their entity yaw
    // (`cg_players.c:4274`, `:4366`).
    if !mesh.animator.humanoid() {
        mesh.render_yaw_degrees = None;
        if let (Some(snapshot), Some(game_state), Some(id)) = (snapshot, game_state, mesh.entity_id)
            && let Some(state) = snapshot
                .entities
                .iter()
                .find(|state| u64::from(state.number()) + 1 == id.get())
        {
            server_bone_angles::apply(mesh, state, game_state);
        }
    } else if let Some(pose) = pose {
        let origin = mesh
            .entity_id
            .and_then(|id| world.entity(id))
            .map_or([0.0; 3], |entity| {
                entity.sample(presentation_time).translation
            });
        let origin = if mesh.entity_id == local_entity {
            predicted.map_or(origin, |state| state.origin)
        } else {
            origin
        };
        let pose = mesh
            .wall_hold
            .apply(pose, mesh.entity_id, state.lower.clip, origin, collision);
        let mut inputs =
            sjk_client::LegacyPlayerAngleInputs::from_world(pose, origin, world, presentation_time);
        inputs.frame_millis = Some(frame_millis);
        if pose.angle.correct_animation_motion {
            inputs.motion_angles = mesh
                .animator
                .player_motion_angles(&mesh.preview.animation, presentation_time)?;
        }
        let angles = mesh
            .angle_controller
            .evaluate_with_inputs(pose, presentation_time, inputs);
        mesh.animator
            .set_player_angles(&mesh.preview.animation, angles)?;
        mesh.render_yaw_degrees = Some(angles.legs_yaw_degrees);
    } else {
        mesh.animator.clear_player_angles();
    }
    mesh.animator.requested = Some(state);
    Ok(())
}

/// Stage one successfully evaluated pose; the caller flushes even when another actor fails.
pub(super) fn apply(
    mesh: &mut ActorMesh,
    skinning: &mut gpu_skinning::Buffers,
    queue: &crate::frame_queue::FrameQueue,
    vertex_buffer: &wgpu::Buffer,
    presentation_time: i64,
) -> Result<(), Box<dyn Error>> {
    let Some(state) = mesh.animator.requested else {
        return Ok(());
    };
    mesh.animator.completed()?;
    let driver = mesh.animator.matrices();
    if let Some(limb) = &mut mesh.limb {
        limb.update_pivot(&mesh.preview.animation, driver);
    }
    // A stock model playing a rig model's emote clip is drawn, and carries its
    // bolts, in the clip's pose; saber traces keep the driver's.
    let emote = match mesh.preview.stock_emotes.clone() {
        Some(stock) if mesh.preview.rig.is_none() => mesh
            .entity_id
            .and_then(|id| u8::try_from(id.get().checked_sub(1)?).ok())
            .and_then(|slot| crate::emotes::playing(slot, std::time::Instant::now()))
            .and_then(|playing| {
                stock.play(
                    &playing.clip,
                    playing.seconds,
                    playing.remaining,
                    playing.looping,
                )
            })
            .is_some_and(|play| stock.pose(driver, play, &mut mesh.preview.stock_pose)),
        _ => false,
    };
    // A limp player (the Ragdoll toy) is drawn, bolts and all, as the ragdoll lies.
    let ragdoll = mesh
        .entity_id
        .filter(|_| mesh.limb.is_none() && !mesh.corpse_pool)
        .and_then(crate::ragdoll::shown_entity)
        .is_some_and(|points| {
            crate::ragdoll::pose(&mut mesh.ragdoll, &mesh.preview.animation, &points)
        });
    mesh.ragdoll.posed = ragdoll;
    let matrices: &[[[f32; 4]; 3]] = if ragdoll {
        &mesh.ragdoll.matrices
    } else if emote {
        &mesh.preview.stock_pose.matrices
    } else {
        driver
    };
    // A rig model is posed first, with the emote clip it plays: the bolts below
    // read its pose.
    if let Some(rig) = mesh.preview.rig.clone() {
        let play = mesh
            .entity_id
            .filter(|_| !rig.clips.is_empty())
            .and_then(|id| u8::try_from(id.get().checked_sub(1)?).ok())
            .and_then(|slot| crate::emotes::playing(slot, std::time::Instant::now()))
            .and_then(|playing| {
                rig.play(
                    &playing.clip,
                    playing.seconds,
                    playing.remaining,
                    playing.looping,
                )
            });
        rig.pose(matrices, play, &mut mesh.preview.rig_pose);
    }
    if let Some(palette) = &mut mesh.gpu_palette {
        match &mesh.preview.rig {
            Some(_) => palette.stage(skinning, &mesh.preview.rig_pose.palette)?,
            None => palette.stage(skinning, matrices)?,
        }
        mesh.force_bones.update(&mesh.preview, matrices);
        mesh.force_bones.update_head(&mesh.preview, matrices);
        mesh.weapon_attachments = crate::saber::attachments_from_matrices(&mesh.preview, matrices);
        mesh.driver_seat = crate::vehicle_pose::driver_seat(&mesh.preview, matrices);
        crate::vehicle_fire::muzzles(&mesh.preview, matrices, &mut mesh.vehicle_muzzles);
        mesh.cosmetics.update_bolts(&mesh.preview, matrices);
        mesh.jetpack.update(&mesh.preview, matrices);
        mesh.holsters.update(&mesh.preview, matrices);
        mesh.current_frames = (
            state.lower.forced_frame.unwrap_or(state.lower.clip),
            state.upper.forced_frame.unwrap_or(state.upper.clip),
        );
        if !mesh.retained_pose.trace_required() {
            return Ok(());
        }
    }
    let surfaces = mesh
        .preview
        .mesh
        .skin_pose_matrices(&mesh.preview.skin, 0, matrices)?;
    mesh.retained_pose
        .update_trace_lod(&mesh.preview.mesh, driver)?;
    mesh.force_bones.update(&mesh.preview, matrices);
    mesh.force_bones.update_head(&mesh.preview, matrices);
    mesh.weapon_attachments = crate::saber::attachments_from_matrices(&mesh.preview, matrices);
    mesh.driver_seat = crate::vehicle_pose::driver_seat(&mesh.preview, matrices);
    crate::vehicle_fire::muzzles(&mesh.preview, matrices, &mut mesh.vehicle_muzzles);
    mesh.cosmetics.update_bolts(&mesh.preview, matrices);
    mesh.jetpack.update(&mesh.preview, matrices);
    mesh.holsters.update(&mesh.preview, matrices);
    if let Some(rig) = &mesh.preview.rig
        && mesh.gpu_palette.is_none()
    {
        for range in &mesh.vertex_ranges {
            let surface = rig
                .surfaces
                .get(range.surface_index)
                .ok_or("rig surface disappeared")?;
            mesh.pose_vertices.clear();
            mesh.pose_vertices
                .extend(surface.vertices.iter().map(|vertex| {
                    crate::actor_load::rig_gpu_vertex(vertex, Some(&mesh.preview.rig_pose.palette))
                }));
            let byte_offset = u64::try_from(range.vertices.start)?
                .checked_mul(u64::try_from(std::mem::size_of::<GpuVertex>())?)
                .ok_or("animated actor buffer offset overflow")?;
            queue.write_buffer(
                vertex_buffer,
                byte_offset,
                bytemuck::cast_slice(&mesh.pose_vertices),
            );
        }
    }
    for range in &mesh.vertex_ranges {
        if mesh.gpu_palette.is_some() || mesh.preview.rig.is_some() {
            break;
        }
        let surface = surfaces
            .get(range.surface_index)
            .ok_or("animated actor surface disappeared")?;
        if surface.vertices.len() != range.vertices.len() {
            return Err("animated actor vertex count changed".into());
        }
        mesh.pose_vertices.clear();
        mesh.pose_vertices.extend(
            surface
                .vertices
                .iter()
                .map(|vertex| preview_gpu_vertex(&mesh.preview, vertex)),
        );
        let byte_offset = u64::try_from(range.vertices.start)?
            .checked_mul(u64::try_from(std::mem::size_of::<GpuVertex>())?)
            .ok_or("animated actor buffer offset overflow")?;
        queue.write_buffer(
            vertex_buffer,
            byte_offset,
            bytemuck::cast_slice(&mesh.pose_vertices),
        );
    }
    mesh.current_frames = (
        state.lower.forced_frame.unwrap_or(state.lower.clip),
        state.upper.forced_frame.unwrap_or(state.upper.clip),
    );
    if let Some(entity) = mesh.entity_id {
        mesh.retained_pose
            .publish(entity, presentation_time, surfaces);
    }
    Ok(())
}

/// Complete the upload batch even when one actor has an invalid pose.
pub(super) fn apply_all(
    meshes: &mut [ActorMesh],
    skinning: &mut gpu_skinning::Buffers,
    queue: &crate::frame_queue::FrameQueue,
    vertex_buffer: &wgpu::Buffer,
    presentation_time: i64,
) {
    for mesh in meshes {
        if mesh.animator.requested.is_none() {
            continue;
        }
        match apply(mesh, skinning, queue, vertex_buffer, presentation_time) {
            Ok(()) => mesh.animator.clear_error_report(),
            Err(error) => failed(mesh, error.as_ref()),
        }
    }
    // A bad actor must never strand healthy actors' staged palettes.
    skinning.flush(queue);
}

/// Report an actor's failure once until it successfully poses again.
pub(super) fn failed(mesh: &mut ActorMesh, error: &dyn Error) {
    mesh.retained_pose.invalidate();
    if mesh.animator.suppress_failed_frame() {
        crate::log::progress(format_args!(
            "actor {:?} ({}/{}): holding pose after animation error: {error}",
            mesh.entity_id.map(|id| id.get().saturating_sub(1)),
            mesh.appearance.model,
            mesh.appearance.variant,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_crouch_is_the_protocol_crouch() {
        assert_eq!(
            sjk_client::legacy_animation_name(BOTH_CROUCH1IDLE),
            Some(CROUCH_IDLE)
        );
        let index = |name| sjk_game_jka::legacy_animation_index(name).unwrap();
        // Standing (the stances' idles) is crouched over in a box ...
        for stand in [
            "BOTH_STAND1",
            "BOTH_STAND2",
            "BOTH_STAND5",
            "BOTH_STAND1IDLE1",
            "BOTH_SABERFAST_STANCE",
            "BOTH_SABERDUAL_STANCE",
            "BOTH_SABERSTAFF_STANCE",
        ] {
            assert!(standing_clip(index(stand)), "{stand}");
        }
        // ... a move the server plays is not (#231's review: lunge, kicks, rolls,
        // knockdowns), nor a crouch already.
        for other in [
            "BOTH_LUNGE2_B__T_",
            "BOTH_A7_KICK_F",
            "BOTH_ROLL_STAB",
            "BOTH_KNOCKDOWN1",
            "BOTH_CROUCH1WALK",
            "BOTH_CROUCH1IDLE",
        ] {
            assert!(!standing_clip(index(other)), "{other}");
        }
    }
}
