//! Model-authored audio follows evaluated bones, including the first-person actor.
//! A step also leaves its dust and footprint on the ground it finds ([`crate::footprints`]).
use crate::{GameAudio, GpuState};
use glam::{Quat, Vec3};
use sjk_client::animation_events::{Clips, Cue, Cursor, Handover, Played, footsteps};

#[derive(Default)]
pub(crate) struct State {
    cursor: Cursor,
    // The local player's, between the predicted animation and the server's (#171).
    handover: Handover,
    last_time: Option<i64>,
    identity: Option<sjk_runtime::EntityId>,
    motion_revision: u64,
}

pub(crate) fn update(gpu: &mut GpuState, time: i64, audio: &mut Option<GameAudio>) {
    let steps = crate::footprints::Settings::read(gpu.console.as_ref());
    if audio.is_none() && !steps.prints {
        return;
    }
    let now = std::time::Instant::now();
    let paused = gpu.resident.map_change_pending;
    let world = gpu
        .demo_session
        .as_ref()
        .map_or(&gpu.live_world, crate::demo_playback::Session::world);
    let snapshot = crate::first_person_view::presented_snapshot(
        gpu.live_session.as_ref(),
        gpu.demo_session.as_ref(),
        time as i32,
    );
    let local = snapshot.map(|snapshot| u64::from(snapshot.player.client_num()) + 1);
    let predicted = gpu
        .local_prediction
        .predicted_state()
        .filter(|_| !gpu.free_camera_active());
    // The loaded skins' swings, shared (one count, no copy) so the meshes can be walked.
    let skins = std::sync::Arc::clone(&gpu.blade_skins);
    for mesh in &mut gpu.actor_meshes {
        if let Some(prefetch) = mesh.preview.event_sounds.take()
            && let Some(audio) = audio.as_mut()
        {
            audio.absorb_animation_prefetch(&prefetch);
        }
        let entity = mesh.entity_id.and_then(|id| world.entity(id));
        let Some(entity) = entity.filter(|_| {
            !paused && !mesh.corpse_pool && mesh.limb.is_none() && mesh.animator.requested.is_some()
        }) else {
            mesh.audio_events.cursor.reset();
            mesh.audio_events.handover.reset();
            mesh.audio_events.last_time = None;
            continue;
        };
        let revision = entity.sampled_motion_revision(time);
        if mesh.audio_events.motion_revision != revision
            || mesh.audio_events.identity != mesh.entity_id
            || mesh.audio_events.last_time.is_some_and(|old| time < old)
        {
            mesh.audio_events.cursor.reset();
            mesh.audio_events.handover.reset();
        }
        mesh.audio_events.identity = mesh.entity_id;
        mesh.audio_events.motion_revision = revision;
        mesh.audio_events.last_time = Some(time);
        let id = entity.id.get();
        let is_local = local == Some(id);
        let skin = gpu.saber_skins.sound_of(id);
        let footstep_class = snapshot
            .and_then(|s| {
                s.entities
                    .binary_search_by_key(&((id - 1) as u16), |e| e.number())
                    .ok()
                    .map(|i| s.entities[i].npc_class())
            })
            .unwrap_or(0);
        let transform = entity.sample(time);
        let origin = if is_local {
            predicted.map_or(transform.translation, |p| p.origin)
        } else {
            transform.translation
        };
        let rotation = mesh
            .render_yaw_degrees
            .map_or(Quat::from_array(transform.rotation), |yaw| {
                Quat::from_rotation_z(yaw.to_radians())
            });
        // The print turns with the legs (`cent->pe.legs.yawAngle`).
        let legs_yaw = mesh.render_yaw_degrees.unwrap_or_else(|| {
            let forward = rotation * Vec3::X;
            forward.y.atan2(forward.x).to_degrees()
        });
        let frames = mesh.animator.event_frames(&mesh.preview.animation, time);
        let matrices = mesh.animator.matrices();
        let play = |cue: &Cue, variant: usize| match cue {
            Cue::Sound { paths, channel } => {
                let Some(audio) = audio.as_mut() else {
                    return;
                };
                let standard = &paths[variant % paths.len()];
                let path = gpu
                    .saber_hilts
                    .as_ref()
                    .and_then(|hilts| {
                        hilts.animation_sound(mesh.saber_names[0].as_deref()?, standard, variant)
                    })
                    .unwrap_or(standard);
                // A blade skin's swing plays over the stock or the hilt's own, on a
                // channel of its own so that it does not cut it; with
                // `s_saberSkinSounds 2` it plays in their place, with 0 not at all. One
                // not in the game data (yet) is left out, the stock one playing alone.
                let swing = skin
                    .filter(|_| !audio.skin_sounds_off)
                    .and_then(|skin| skin_swing(&skins, skin, standard, variant))
                    .filter(|swing| audio.has_sound(swing));
                match swing {
                    Some(swing) if audio.skin_sounds_only => {
                        audio.play_animation(swing, *channel, false, origin, id, is_local);
                    }
                    _ => {
                        audio.play_animation(path, *channel, false, origin, id, is_local);
                        if let Some(swing) = swing {
                            audio.play_animation(swing, 0, false, origin, id, is_local);
                        }
                    }
                }
            }
            Cue::Footstep {
                right,
                heavy,
                extra,
            } => {
                if !footsteps::allowed_class(footstep_class) || (*extra && !steps.jump) {
                    return;
                }
                let bolt = if *right { "*r_leg_foot" } else { "*l_leg_foot" };
                let Some(bolt) = mesh.preview.bolt(bolt, matrices) else {
                    return;
                };
                let point = Vec3::new(bolt[0][3], bolt[1][3], bolt[2][3])
                    * Vec3::from_array(transform.scale);
                let start = Vec3::from_array(origin) + rotation * point + Vec3::Z * 15.0;
                let Some(ground) =
                    crate::footprints::ground(&gpu.bsp, &mut gpu.trace_scratch, start, steps.guess)
                else {
                    return;
                };
                if let Some(audio) = audio.as_mut() {
                    let path = footsteps::PATHS[ground.family][usize::from(*heavy)][variant % 4];
                    audio.play_animation(path, 6, true, origin, id, is_local);
                }
                if !steps.prints {
                    return;
                }
                // `FX_PlayEffectID` along the ground's normal, then the print.
                if let (Some((effect, _)), Some(vfs)) =
                    (footsteps::trace_of(ground.family), gpu.vfs.as_deref())
                {
                    crate::effect_runtime::spawn_effect(
                        &mut gpu.particles,
                        &mut gpu.effect_aux,
                        &mut gpu.effects,
                        vfs,
                        effect,
                        ground.point,
                        now,
                        (id as u32) ^ (variant as u32).rotate_left(16),
                        0,
                        &mut None,
                        crate::combat_effects::rotation_from_direction(ground.normal.to_array()),
                    );
                }
                crate::footprints::leave_print(
                    &mut gpu.effect_aux.decals,
                    &ground,
                    *right,
                    *heavy,
                    legs_yaw,
                );
            }
        };
        let state = &mut mesh.audio_events;
        if is_local {
            // The predicted animation, or the server's while a saber move is not
            // predicted (`local_actor_state`); each track's revision tells a restart,
            // the outgoing one's while a blend still shows its clip.
            let requested = mesh.animator.requested;
            let tracks = [
                requested.map(|state| state.lower),
                requested.map(|state| state.upper),
            ];
            let played = std::array::from_fn(|track| {
                frames[track].map(|(clip, frame)| Played {
                    clip,
                    revision: tracks[track].map_or(0, |state| {
                        if state.clip == clip {
                            state.revision
                        } else {
                            state
                                .transition
                                .filter(|previous| previous.clip == clip)
                                .map_or(0, |previous| previous.revision)
                        }
                    }),
                    frame,
                })
            });
            state.handover.advance(
                &mut state.cursor,
                Clips {
                    events: &mesh.preview.events,
                    config: &mesh.preview.config,
                },
                played,
                gpu.local_actor_state.is_active(),
                time,
                play,
            );
        } else {
            state
                .cursor
                .advance(&mesh.preview.events, &mesh.preview.config, frames, play);
        }
    }
}

/// The swing the loaded skin numbered `skin` (the sound set a player is heard with)
/// plays for a stock `saberhup` animation cue.
fn skin_swing<'a>(
    skins: &'a crate::saber_skins::LoadedSkins,
    skin: crate::saber_skins::SkinIndex,
    standard: &str,
    variant: usize,
) -> Option<&'a str> {
    if !standard.starts_with("sound/weapons/saber/saberhup") {
        return None;
    }
    skins.swing(skin, variant)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_adds_to_only_the_stock_swing_cues() {
        let skins = crate::saber_skins::tests::loaded_sample(1);
        let skin = skins.color_of("saber_tidal").unwrap().index;
        assert_eq!(
            skin_swing(&skins, skin, "sound/weapons/saber/saberhup3.wav", 4),
            Some("sound/test/blade/s2.wav")
        );
        assert_eq!(
            skin_swing(&skins, skin, "sound/weapons/saber/saberspin1.wav", 0),
            None
        );
    }
}
