//! Flying primary hilt and blade submission, `codemp/cgame/cg_players.c:10050-10305`.
//!
//! The hilt keeps the entity's evaluated position, spin (yaw) and roll; its pitch
//! is the owner's client-side tilt towards 90, which lays the saber flat
//! (`saber_trail::tilt`), and a saber being pulled back faces away from its owner.
//! Blade ROLL is zeroed. The sockets, world-blade transform and glow/core pair are
//! shared with held and menu-stage sabers (`saber_submission.rs`,
//! `menu_stage/sabers.rs`); the menu's own throw animation is separate
//! (`menu_stage/throw.rs`).
//!
//! The flying blades carry on the hand's blade state: their trail, the motion the
//! swing-driven designs read, and a skin's afterimages and blossoms, kept in the world
//! while the saber flies (`flight_motion`, `flight_wake`; hub report 127).

use crate::actor_instance::ActorInstance;
use crate::saber::{self, Instance};
use crate::saber_hilts::HiltCatalog;
use crate::saber_rgb::BladeColor;
use crate::saber_trail::{BladeState, StateSlab, tilt};
use glam::{EulerRot, Quat, Vec3};
use sjk_client::{LegacyThrownSaber, LegacyThrownSabers};
use sjk_runtime::{EntityId, SceneEntity, Transform};

/// Claim a flying saber before the ordinary rigid-object path can draw it twice.
pub(super) fn submit(
    sinks: &mut super::Sinks<'_>,
    thrown: Option<&LegacyThrownSabers<'_>>,
    entity: &SceneEntity,
    transform: Transform,
) -> bool {
    let Some(number) = entity
        .id
        .get()
        .checked_sub(1)
        .and_then(|id| u16::try_from(id).ok())
    else {
        return false;
    };
    let Some(mut owner) = thrown.and_then(|thrown| thrown.get(number)) else {
        return false;
    };
    let Some(catalog) = sinks.saber_hilts else {
        return false;
    };
    if owner.definition_color {
        // An NPC's blade without an override wears its `.sab` colour.
        owner.color = sjk_client::legacy_npc_definition_rgb(
            catalog.get_any_case(owner.name).default_color,
            owner.owner,
        );
    }
    // The owner is a client slot or an NPC's entity number (`thrown_sabers.rs`).
    // A player muted on this PC throws the default saber in the default colour.
    if crate::muted_players::entity_muted(sinks.muted, u64::from(owner.owner) + 1) {
        owner.name = crate::muted_players::SABER;
        owner.color = crate::muted_players::BLADE;
    }
    // In team games the primary's team colour; with CTF team sabers no skin over it.
    let team_blade = crate::team_sabers::of(&sinks.team_blades, u64::from(owner.owner) + 1);
    let team_color = team_blade.colors[0];
    if let Some(rgb) = team_color {
        owner.color = rgb;
    }
    let owner_entity = u64::from(owner.owner) + 1;
    let Some(owner_tilt) = sinks.saber_states.tilt_mut(owner_entity) else {
        return false;
    };
    if !owner.in_flight {
        // Caught: the lingering entity draws nothing and the tilt starts over
        // (`cg_ents.c:871`, `cg_players.c:10448-10449`).
        owner_tilt.reset();
        return true;
    }
    let presented = Quat::from_array(transform.rotation);
    let (_, server_pitch, _) = presented.to_euler(EulerRot::ZYX);
    let pitch = owner_tilt.advance(server_pitch.to_degrees(), sinks.presentation_time);
    let returning_from = owner
        .returning
        .then(|| {
            sinks
                .world
                .entity(EntityId::new(owner_entity))
                .map(|owner| {
                    (
                        Vec3::from_array(transform.translation),
                        Vec3::from_array(owner.sample(sinks.presentation_time).translation),
                    )
                })
        })
        .flatten();
    let (hilt_rotation, blade_rotation) = tilt::flight_rotations(presented, pitch, returning_from);
    let pose = Transform {
        rotation: hilt_rotation.to_array(),
        ..transform
    };
    // A team colour replaces a black blade too; a skin is worn over it unless plain.
    let stock = if team_color.is_some() {
        BladeColor::from_rgb(owner.color)
    } else {
        sinks
            .black_sabers
            .color(owner_entity, 0, BladeColor::from_rgb(owner.color))
    };
    let color = if team_blade.plain {
        stock
    } else {
        sinks.saber_skins.blade_color(owner_entity, stock)
    };
    if let Some((hilt, blades)) = submit_hilt(
        owner,
        color,
        pose,
        blade_rotation,
        catalog,
        sinks.object_groups,
        sinks.saber_instances,
        sinks.saber_states,
        sinks.presentation_time,
        sinks.saber_core,
    ) {
        add_trails(sinks, owner_entity, hilt, &blades, color);
        crate::saber_submission::lights::append(
            sinks.lights,
            &blades,
            color,
            hilt.num_blades,
            hilt.no_dlight,
            sinks.presentation_time,
            entity.id.get(),
            // The seeds of its blades' instances (`submit_hilt`).
            owner_entity.wrapping_mul(24) as u32,
            0.0,
        );
    }
    true
}

/// A flying primary always trails (`saberInFlight && saberNum == 0` in
/// `CG_AddSaberBlade`'s draw test), for as long as the owner's current
/// `saberMove` authors, or the 40 ms `SABER_TRAIL_TIME` when it authors none.
/// It advances the same per-blade state the hilt used in the hand.
fn add_trails(
    sinks: &mut super::Sinks<'_>,
    owner_entity: u64,
    hilt: crate::saber_hilts::Hilt,
    blades: &[Option<saber::Blade>; 8],
    color: BladeColor,
) {
    let Some(edges) = sinks.trail_edges.as_mut() else {
        return;
    };
    let authored = sinks
        .world
        .entity(EntityId::new(owner_entity))
        .and_then(|owner| owner.equipment())
        .map_or(0, |equipment| equipment.trail_duration_millis);
    for (index, blade) in blades.iter().enumerate() {
        let (Some(blade), Some(hilt_blade)) = (blade, hilt.blade(index)) else {
            continue;
        };
        let Some(state) = sinks.saber_states.blade_mut(owner_entity, 0, index) else {
            continue;
        };
        if let Some(quad) = state.trail.update(
            edges.edge(*blade),
            sinks.presentation_time,
            authored,
            hilt_blade.trail_style,
            color,
            true,
        ) {
            sinks.saber_segments.insert(quad, sinks.presentation_time);
        }
    }
}

/// Emit one hilt at `transform` and each lit primary blade at `rotation` in `color`
/// (the owner's skin over its saber colour).
#[allow(clippy::too_many_arguments)]
pub(super) fn submit_hilt(
    owner: LegacyThrownSaber<'_>,
    color: BladeColor,
    transform: Transform,
    rotation: Quat,
    catalog: &HiltCatalog,
    objects: &mut [Vec<ActorInstance>],
    blades: &mut Vec<Instance>,
    states: &mut StateSlab,
    presentation_time: i64,
    core_scale: f32,
) -> Option<(crate::saber_hilts::Hilt, [Option<saber::Blade>; 8])> {
    // CG_Player owns manual rendering only while saberInFlight (10050-10055);
    // CG_General ignores the lingering modelGhoul2=127 entity (cg_ents.c:871).
    if !owner.in_flight {
        return None;
    }
    let hilt = catalog.get_any_case(owner.name);
    objects[hilt.mesh_index].push(ActorInstance::new(
        transform.translation,
        transform.rotation,
        transform.scale,
    ));
    let origin = Vec3::from_array(transform.translation);
    let owner_entity = u64::from(owner.owner) + 1;
    let mut light_blades = [None; 8];
    for index in 0..usize::from(hilt.num_blades) {
        if index > 0 && !owner.extra_blades {
            break;
        }
        let Some(blade) = hilt.blade(index) else {
            continue;
        };
        let blade = saber::world_blade(origin, rotation, blade.socket, blade.length, blade.radius);
        // The seed of the same blade in the hand, so its animation carries on in flight.
        let key = owner_entity.wrapping_mul(24) + index as u64;
        // The hand's state of this blade (its trail's too, `add_trails`), so the parts the
        // swing drives carry on in flight (hub report 127).
        let mut state = states.blade_and_petals(owner_entity, 0, index);
        let motion = state
            .as_mut()
            .map_or(crate::saber_motion::STILL, |(state, _)| {
                flight_motion(state, blade, color, presentation_time)
            });
        blades.extend(Instance::pair(blade, color).map(|i| {
            i.with_core_scale(core_scale)
                .with_contact(owner_entity, 2, index, !hilt.no_wall_marks, hilt.no_dlight)
                .with_animation(presentation_time as f64 * 0.001, key as u32)
                .with_motion(motion)
        }));
        blades.extend(saber::skin_extras(
            blade,
            color,
            presentation_time as f64 * 0.001,
            key as u32,
        ));
        if let Some((state, petals)) = state {
            flight_wake(
                state,
                petals,
                blade,
                color,
                presentation_time,
                key as u32,
                blades,
            );
        }
        light_blades[index] = Some(blade);
    }
    Some((hilt, light_blades))
}

/// A flying blade's motion for the designs that answer a swing (`saber_motion.rs`), as the
/// hand's measures it: the spin is its swing. Measured in the world, there being no wearer
/// to turn with it; whether its wearer attacks is not asked, so a swing must turn the blade.
fn flight_motion(
    state: &mut BladeState,
    blade: saber::Blade,
    color: BladeColor,
    now: i64,
) -> [f32; 4] {
    let skin = match color {
        BladeColor::Skin(skin) => Some(skin),
        _ => None,
    };
    let mut motion = state.motion.record_turned(
        blade,
        blade.direction,
        now,
        false,
        None,
        skin.is_some_and(|skin| skin.each_swing),
    );
    if skin.is_some_and(|skin| skin.stirred) {
        motion[3] = state.motion.stir();
    }
    motion
}

/// What a flying skinned blade leaves behind it, as in the hand (`saber_submission.rs`):
/// its afterimages and the blossoms its swing sheds (hub report 127: the throw left none).
/// Both are kept in the world in flight, a signature's (`GhostSpec::wearer`) too: the
/// blade has left its wearer's frame, and its spin and flight are what it leaves them along.
fn flight_wake(
    state: &mut BladeState,
    petals: &mut Vec<crate::saber_petals::PetalSpawn>,
    blade: saber::Blade,
    color: BladeColor,
    now: i64,
    key: u32,
    instances: &mut Vec<Instance>,
) {
    state.fly(true);
    let BladeColor::Skin(skin) = color else {
        return;
    };
    if let Some(spec) = skin.ghosts {
        state.ghosts.record(blade, now, spec);
        instances.extend(
            state
                .ghosts
                .instances(blade, color, spec)
                .map(|i| i.with_animation(now as f64 * 0.001, key)),
        );
    }
    if let Some(spec) = skin.petals {
        let base = Vec3::from_array(blade.base);
        let along = Vec3::from_array(blade.direction);
        let tip = base + along * blade.length;
        state
            .petals
            .shed(base, along, blade.length, tip, now, key, spec, petals);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::saber_ghosts::GhostSpec;
    use crate::saber_skins::SkinColor;

    fn skin(ghosts: Option<GhostSpec>) -> BladeColor {
        let BladeColor::Skin(base) =
            BladeColor::from_material(crate::saber_rgb::SKIN_MATERIAL, [1.0; 3])
        else {
            unreachable!()
        };
        BladeColor::Skin(SkinColor { ghosts, ..base })
    }

    /// The blade spinning flat about its hilt at `origin`, `degrees` round.
    fn spun(origin: Vec3, degrees: f32) -> saber::Blade {
        let (sin, cos) = degrees.to_radians().sin_cos();
        saber::Blade {
            base: origin.to_array(),
            direction: [cos, sin, 0.0],
            length: 40.0,
            radius: 3.0,
        }
    }

    const SIGNATURE: GhostSpec = GhostSpec {
        count: 3,
        spacing_millis: 30.0,
        fade: 0.5,
        wearer: true,
    };

    #[test]
    fn a_thrown_blade_leaves_its_afterimages_in_the_world() {
        let color = skin(Some(SIGNATURE));
        let mut state = BladeState::default();
        let mut petals = Vec::new();
        let mut instances = Vec::new();
        for step in 0..8 {
            instances.clear();
            let blade = spun(Vec3::new(step as f32 * 15.0, 0.0, 0.0), step as f32 * 60.0);
            flight_wake(
                &mut state,
                &mut petals,
                blade,
                color,
                1_000 + step * 30,
                7,
                &mut instances,
            );
        }
        // The newest pose kept is the blade itself (not drawn); the two before it are.
        assert_eq!(instances.len(), 2, "afterimages behind the spinning blade");
        assert!(instances.iter().all(Instance::is_afterimage));
        // Where the blade was in the world, a step back along its flight.
        let (start, end, _) = instances[0].extent();
        let middle = (Vec3::from_array(start) + Vec3::from_array(end)) * 0.5;
        let was = spun(Vec3::new(90.0, 0.0, 0.0), 360.0);
        let was_middle = Vec3::from_array(was.base) + Vec3::from_array(was.direction) * 20.0;
        assert!(
            middle.distance(was_middle) < 6.0,
            "{middle} against {was_middle}"
        );
    }

    #[test]
    fn hand_and_flight_do_not_share_poses_across_the_change_of_frame() {
        let color = skin(Some(SIGNATURE));
        let mut state = BladeState::default();
        let mut petals = Vec::new();
        let mut instances = Vec::new();
        // Poses kept in the hand, in the wearer's frame.
        state.fly(false);
        for step in 0..4 {
            state.ghosts.record(
                spun(Vec3::ZERO, step as f32 * 90.0),
                1_000 + step * 30,
                SIGNATURE,
            );
        }
        // Thrown: none of them is drawn about the flying blade.
        let thrown = spun(Vec3::new(500.0, 0.0, 0.0), 45.0);
        flight_wake(
            &mut state,
            &mut petals,
            thrown,
            color,
            1_120,
            7,
            &mut instances,
        );
        assert!(instances.is_empty());
        // Caught: the poses kept in flight go too.
        state.ghosts.record(thrown, 1_150, SIGNATURE);
        state.fly(false);
        assert_eq!(
            state
                .ghosts
                .instances(spun(Vec3::ZERO, 0.0), color, SIGNATURE)
                .count(),
            0
        );
    }

    #[test]
    fn a_spinning_thrown_blade_swings() {
        let mut state = BladeState::default();
        let mut motion = crate::saber_motion::STILL;
        for step in 0..12 {
            let blade = spun(Vec3::ZERO, step as f32 * 40.0);
            motion = flight_motion(&mut state, blade, skin(None), 1_000 + step * 16);
        }
        assert!(motion[0] > 0.5, "the spin is a swing: {motion:?}");
        assert!(
            motion[3] < crate::saber_motion::SETTLED,
            "and a swing began"
        );
    }
}
