//! Actor-to-hilt, blade, and motion-trail submission.

use crate::actor_instance::ActorInstance;
use crate::saber::{self, Instance};
use crate::saber_hilts::HiltCatalog;
use crate::saber_rgb::BladeColor;
use crate::saber_trail::{Edges, SegmentPool, StateSlab};
use glam::{Quat, Vec3};
use sjk_runtime::{HeldEquipment, HeldItemKind};

#[path = "saber_lights.rs"]
pub(crate) mod lights;

/// Submit both hilts and every numbered blade for one actor. `trails` is
/// `None` with `cg_saberTrail 0`; `skins` are the blade skins the actor wears on its first
/// and second saber, which replace those sabers' colours. `hands` are the bolts already at
/// the actor's `scale` ([`saber::Attachment::scaled`]); the hilts are drawn at it too, as
/// stock draws the bolted saber model with the wearer's `modelScale`. `core_scale` is
/// `cg_saberCoreSize` ([`crate::saber_core`]). A skin with smoke lets its puffs go into
/// `smoke`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn submit(
    entity_id: u64,
    skins: [Option<crate::saber_skins::SkinColor>; 2],
    black_sabers: crate::saber_rgb::black::Policy,
    hands: [Option<(&str, saber::Attachment)>; 2],
    equipment: HeldEquipment,
    actor_origin: Vec3,
    actor_rotation: Quat,
    scale: [f32; 3],
    catalog: Option<&HiltCatalog>,
    states: &mut StateSlab,
    segments: &mut SegmentPool,
    object_groups: &mut [Vec<ActorInstance>],
    saber_instances: &mut Vec<Instance>,
    presentation_time: i64,
    mut trails: Option<&mut Edges<'_>>,
    lights: &mut crate::dynamic_lights::PointLightList,
    attacking: Option<bool>,
    core_scale: f32,
    smoke: &mut crate::saber_smoke::Sink<'_>,
) -> bool {
    if equipment.kind != HeldItemKind::EnergyBlade {
        return false;
    }
    let Some(catalog) = catalog else {
        return true;
    };
    if !equipment.primary_in_flight
        && let Some(tilt) = states.tilt_mut(entity_id)
    {
        // The primary is in the hand: the next throw tilts from its own pitch
        // (`cg_players.c:10429-10449`).
        tilt.reset();
    }
    for (saber_index, hand) in hands.into_iter().enumerate() {
        if saber_index == 0 && equipment.primary_in_flight {
            // The flying hilt carries this saber's blades and trail state
            // (`thrown_saber.rs`), as `CG_AddSaberBlade` shares
            // `client->saber[0].blade[n].trail` between hand and flight.
            continue;
        }
        let Some((name, attachment)) = hand else {
            continue;
        };
        let two_sabers = hands[1].is_some();
        let hilt = catalog.get(name);
        let (grip, weapon_rotation) =
            saber::world_attachment(actor_origin, actor_rotation, attachment);
        object_groups[hilt.mesh_index].push(ActorInstance::new(
            grip.to_array(),
            weapon_rotation.to_array(),
            scale,
        ));
        let stock = black_sabers.color(
            entity_id,
            saber_index,
            BladeColor::from_rgb(if saber_index == 0 {
                equipment.color
            } else {
                equipment.secondary_color
            }),
        );
        // A chroma skin takes this saber's colour.
        let worn = |skin: Option<crate::saber_skins::SkinColor>| {
            skin.map_or(stock, |skin| BladeColor::Skin(skin.worn_with(stock)))
        };
        // The blades lit by each skin this saber wears: a staff's other half may wear the
        // second (Sol's request of 21/10/2026), so its light is its own.
        let mut light_blades = [[None; 8]; 2];
        let mut light_colors = [stock; 2];
        // How far each half's stars surge as its blades light, for its light.
        let mut surge = [0.0_f32; 2];
        for blade_index in 0..usize::from(hilt.num_blades) {
            // The second skin on the second saber, or on a staff's other half (a blade
            // pointing away from the first; a crossguard's side blades keep the first's).
            // (Only when the two differ: otherwise one light as before.)
            let other_half = hilt.is_other_half(blade_index);
            let half = usize::from(!two_sabers && other_half && skins[0] != skins[1]);
            let skin =
                crate::saber_skins::skin_of_blade(skins, two_sabers, saber_index, other_half);
            let color = worn(skin);
            light_colors[half] = color;
            let Some(mut hilt_blade) = hilt.blade(blade_index) else {
                continue;
            };
            hilt_blade.socket = hilt_blade.socket.scaled(scale);
            let active = blade_is_active(&equipment, saber_index, blade_index);
            let Some((state, petal_queue)) =
                states.blade_and_petals(entity_id, saber_index, blade_index)
            else {
                continue;
            };
            state.fly(false);
            let length = state
                .extension
                .update(active, hilt_blade.length, presentation_time);
            let since = state.ignition.update(active, presentation_time);
            state.ignition.retracted(length, presentation_time);
            // A skin's lit blade where a blade or a wall struck it this frame: its unrest
            // flares up it and its sparks and bolts go at once.
            let clashed = matches!(color, BladeColor::Skin(_))
                && length >= 0.5
                && smoke.clashed(saber::world_blade(
                    grip,
                    weapon_rotation,
                    hilt_blade.socket,
                    length,
                    hilt_blade.radius,
                ));
            if clashed {
                state.motion.clash(presentation_time);
            }
            let moments = match color {
                BladeColor::Skin(skin) => {
                    skin.flash_moments(since, state.ignition.gone_for(presentation_time))
                }
                _ => saber::Moments::default(),
            };
            // The emitter smokes while lit and a while after, from where it is: puffs,
            // ribbons or both.
            if let BladeColor::Skin(skin) = color
                && (skin.smoke.is_some()
                    || skin.ribbon.is_some()
                    || skin.sparks.iter().any(Option::is_some))
            {
                let reach = saber::world_blade(
                    grip,
                    weapon_rotation,
                    hilt_blade.socket,
                    length,
                    hilt_blade.radius,
                );
                let at = |along: f32| {
                    Vec3::from_array(reach.base)
                        + Vec3::from_array(reach.direction) * (reach.length * along)
                };
                let lit = length >= 0.5;
                let blade = (saber_index * 8 + blade_index) as u64;
                if let Some(spec) = skin.smoke {
                    let seed = (entity_id as u32).wrapping_mul(24) + blade as u32;
                    state
                        .smoke
                        .emit(&spec, at(spec.along), lit, presentation_time, seed, smoke);
                }
                let key = entity_id.wrapping_mul(24) + blade;
                if let Some(spec) = skin.ribbon {
                    smoke
                        .ribbons
                        .emit(key, &spec, at(spec.along), lit, presentation_time);
                }
                let blade = Vec3::from_array(reach.direction);
                for (set, spec) in skin.sparks.iter().enumerate() {
                    let Some(spec) = spec else { continue };
                    let key = key.wrapping_mul(8) + set as u64;
                    if clashed {
                        smoke.kick(key, spec.clash);
                    }
                    let bursts =
                        smoke.spark(key, spec, at(spec.along), blade, lit, presentation_time);
                    // A crackle snaps with the bolts.
                    if skin.crackle
                        && bursts > 0
                        && matches!(spec.shape, crate::saber_sparks::Shape::Bolts { .. })
                    {
                        smoke.hear(
                            crate::saber_smoke::Sound {
                                at: at(spec.along).to_array(),
                                entity: entity_id.saturating_sub(1) as u32,
                                skin: skin.index,
                                count: bursts,
                            },
                            false,
                        );
                    }
                }
            }
            // A design whose words outlast its blade (`SkinColor::linger`): from when the
            // blade begins going in, a whole-length blade that draws only its words, so they
            // float on and can be read (Creyon's round 14). No contact, light or trail.
            if let Some(motion) = skin
                .filter(|skin| skin.linger > 0.0)
                .and_then(|skin| state.motion.lingering(presentation_time, skin.linger))
            {
                // Set upright from the hilt here; the vertex stage then stands it up the
                // screen, so the word faces the camera from every side (round 14: lying along
                // a holstered hilt, it was seen flat; round 15: upright in the world, it was
                // read only from in front).
                let held = saber::world_blade(
                    grip,
                    weapon_rotation,
                    hilt_blade.socket,
                    hilt_blade.length,
                    hilt_blade.radius,
                );
                let whole = saber::Blade {
                    direction: [0.0, 0.0, 1.0],
                    ..held
                };
                let key = entity_id.wrapping_mul(24) + (saber_index * 8 + blade_index) as u64;
                saber_instances.extend(Instance::pair(whole, color).map(|i| {
                    i.with_core_scale(core_scale)
                        .with_animation(presentation_time as f64 * 0.001, key as u32)
                        .with_motion(motion)
                }));
            }
            let key = entity_id.wrapping_mul(24) + (saber_index * 8 + blade_index) as u64;
            let emitter = saber::world_blade(
                grip,
                weapon_rotation,
                hilt_blade.socket,
                hilt_blade.length,
                hilt_blade.radius,
            );
            // A crackle's sound from the emitter: its buzz while lit, a last snap as its
            // sputter ends.
            if let BladeColor::Skin(skin) = color
                && skin.crackle
            {
                let die = skin.flash.map_or(0.0, |flash| flash.die);
                let ended = length < 0.5 && state.ignition.sputtered(die, presentation_time);
                if length >= 0.5 || ended {
                    smoke.hear(
                        crate::saber_smoke::Sound {
                            at: emitter.base,
                            entity: entity_id.saturating_sub(1) as u32,
                            skin: skin.index,
                            count: 1,
                        },
                        length >= 0.5,
                    );
                }
            }
            if length < 0.5 {
                // Out: no history kept from where it went out, for when it lights again
                // (hub report 116: switching sabers, a swing drew a trail slice, and a
                // skin its afterimages, from the old place to the new).
                let out = saber::world_blade(
                    grip,
                    weapon_rotation,
                    hilt_blade.socket,
                    0.0,
                    hilt_blade.radius,
                );
                state.trail.hold(out, presentation_time);
                state.ghosts = Default::default();
                // Put out, a skin's emitter stars sputter on a while (`flash.die`), on the
                // whole blade's line, where they were while it was lit.
                if moments.dying > 0.0 {
                    let blade = emitter;
                    saber_instances.extend(Instance::flashes(blade, color).map(|i| {
                        i.with_animation(presentation_time as f64 * 0.001, key as u32)
                            .with_moments(moments)
                    }));
                }
                continue;
            }
            let blade = saber::world_blade(
                grip,
                weapon_rotation,
                hilt_blade.socket,
                length,
                hilt_blade.radius,
            );
            let flicker = saber::Flicker::sample(key, presentation_time);
            // The swing is measured in the wearer's own frame (Creyon's round 11): turning the
            // view or running turns the whole body, which is no swing, and the effects that
            // answer a swing must not stir then.
            let body = (actor_rotation.inverse() * Vec3::from_array(blade.direction)).to_array();
            let mut motion = state.motion.record_turned(
                blade,
                body,
                presentation_time,
                length < hilt_blade.length,
                attacking,
                skin.is_some_and(|skin| skin.each_swing),
            );
            if skin.is_some_and(|skin| skin.stirred) {
                motion[3] = state.motion.stir();
            }
            // A direction across the blade for the signatures' three-dimensional effects
            // (`Instance::with_frame`): the wearer's forward, square to the blade, so what spreads
            // to the blade's sides (Ashla Radiance's wings) spreads to the wearer's left and right
            // (Creyon's round 20: one wing was at the side and the other somewhere else, the frame
            // having been any square to the hilt model's axis).
            let along = Vec3::from_array(blade.direction).normalize_or_zero();
            let forward = actor_rotation * Vec3::X;
            let square = forward - along * forward.dot(along);
            let own = weapon_rotation
                * Vec3::from_array(hilt_blade.socket.direction)
                    .normalize_or_zero()
                    .any_orthonormal_vector();
            // The hilt's own leans in smoothly as the blade comes round to point forward.
            let across = (square + own * 0.15).normalize_or(own).to_array();
            saber_instances.extend(
                Instance::pair_flickering(blade, color, hilt_blade.length, flicker).map(|i| {
                    i.with_core_scale(core_scale)
                        .with_contact(
                            entity_id,
                            saber_index,
                            blade_index,
                            !hilt.no_wall_marks,
                            hilt.no_dlight,
                        )
                        .with_animation(presentation_time as f64 * 0.001, key as u32)
                        .with_motion(motion)
                        .with_frame(across, hilt_blade.length)
                }),
            );
            saber_instances.extend(
                saber::skin_extras(blade, color, presentation_time as f64 * 0.001, key as u32)
                    .map(|i| i.with_moments(moments).with_motion(motion)),
            );
            surge[half] = surge[half].max(moments.surge);
            if let BladeColor::Skin(skin) = color
                && let Some(spec) = skin.ghosts
            {
                let animate =
                    |i: Instance| i.with_animation(presentation_time as f64 * 0.001, key as u32);
                if spec.wearer {
                    // Kept in the wearer's own frame: turning or running leaves none behind.
                    let origin = actor_origin;
                    let to_body = |b: saber::Blade| saber::Blade {
                        base: (actor_rotation.inverse() * (Vec3::from_array(b.base) - origin))
                            .to_array(),
                        direction: (actor_rotation.inverse() * Vec3::from_array(b.direction))
                            .to_array(),
                        ..b
                    };
                    let to_world = move |b: saber::Blade| saber::Blade {
                        base: (actor_rotation * Vec3::from_array(b.base) + origin).to_array(),
                        direction: (actor_rotation * Vec3::from_array(b.direction)).to_array(),
                        ..b
                    };
                    state.ghosts.record(to_body(blade), presentation_time, spec);
                    saber_instances.extend(
                        state
                            .ghosts
                            .instances_placed(to_body(blade), color, spec, to_world)
                            .map(animate),
                    );
                } else {
                    state.ghosts.record(blade, presentation_time, spec);
                    saber_instances.extend(state.ghosts.instances(blade, color, spec).map(animate));
                }
            }
            // Blossoms shed by its swing, measured in its wearer's frame as the afterimages are.
            if let BladeColor::Skin(skin) = color
                && let Some(spec) = skin.petals
            {
                // A chroma's coloured petals take the wearer's colour.
                let spec = crate::saber_petals::PetalSpec {
                    turn: skin.chroma.turn,
                    ..spec
                };
                let base = Vec3::from_array(blade.base);
                let along = Vec3::from_array(blade.direction);
                // Its whole length's tip, so a blade growing out (or going in) sheds none.
                let whole = base + along * hilt_blade.length;
                state.petals.shed(
                    base,
                    along,
                    blade.length,
                    actor_rotation.inverse() * (whole - actor_origin),
                    presentation_time,
                    key as u32,
                    spec,
                    petal_queue,
                );
            }
            light_blades[half][blade_index] = Some(blade);
            // The Angel blade's cloud trail waits for its full form, as its clouds do.
            let trail_ready = !matches!(color, BladeColor::Skin(skin)
                if skin.trail_kind == crate::blade_extras::TrailKind::Cloud)
                || state
                    .motion
                    .lit_for(presentation_time)
                    .is_some_and(|lit| lit >= 9.5);
            if let Some(edges) = trails.as_deref_mut()
                && let Some(quad) = state.trail.update(
                    edges.edge(blade),
                    presentation_time,
                    equipment.trail_duration_millis,
                    hilt_blade.trail_style,
                    color,
                    equipment.trail_duration_millis > 0 && trail_ready,
                )
            {
                segments.insert(quad, presentation_time);
            }
        }
        for (half, (blades, color)) in light_blades.iter().zip(light_colors).enumerate() {
            if blades.iter().any(Option::is_some) {
                self::lights::append(
                    lights,
                    blades,
                    color,
                    hilt.num_blades,
                    hilt.no_dlight,
                    presentation_time,
                    entity_id.wrapping_add((saber_index + half * 8) as u64),
                    // The blades' own seeds, which their stars crackle by (`skin_flash`).
                    (entity_id.wrapping_mul(24) + (saber_index * 8) as u64) as u32,
                    surge[half],
                );
            }
        }
    }
    true
}

/// Partial holster keeps primary blade 1 but retracts staff extras and saber 2.
pub(crate) fn blade_is_active(
    equipment: &HeldEquipment,
    saber_index: usize,
    blade_index: usize,
) -> bool {
    if saber_index == 0 && blade_index == 0 {
        equipment.active
    } else {
        equipment.secondary_active
    }
}
