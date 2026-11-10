//! Presentation of one snapshot's impact events: the EFX graphs and the
//! code-built beam lines selected by [`impacts::plan`].
//!
//! Beam lines are `FX_AddLine` primitives (`codemp/client/FxUtil.cpp`):
//! a static particle whose streak spans start to end, with linear size and
//! alpha fades and no physics. Disruptor beams fired by the local
//! first-person client start at the view gun's flash point recorded on the
//! previous frame (`cg.lastFPFlashPoint`, `cg_event.c:2442-2500`).

use super::*;
use crate::particle_types::PrimitiveShape;
use sjk_client::LegacyImpactKind;
use sjk_protocol::Snapshot;

/// Particle sinks shared by every impact of a snapshot.
pub(crate) struct Sinks<'a> {
    pub(crate) particles: &'a mut Vec<Particle>,
    pub(crate) auxiliary: &'a mut crate::effect_aux::Runtime,
    pub(crate) effects: &'a mut EffectLibrary,
    pub(crate) vfs: &'a VirtualFileSystem,
    pub(crate) audio: &'a mut Option<GameAudio>,
    /// The map's vehicle weapons, for their shots' `impactFX`.
    pub(crate) missiles: &'a sjk_client::LegacyMissileEffects,
}

/// What `CG_EntityEvent` knows about the local viewer when it picks a beam
/// start: the client number, whether the view is third person, and the view
/// gun's `tag_flash` of the last rendered frame, and every drawn player's world
/// gun muzzle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LocalView {
    pub(crate) client_num: u16,
    pub(crate) third_person: bool,
    pub(crate) flash_point: Option<[f32; 3]>,
    pub(crate) world_muzzles: crate::world_muzzles::Points,
}

/// Counts produced while presenting one received snapshot.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Observation {
    pub(crate) events: usize,
    pub(crate) visual_requests: usize,
    pub(crate) flare_latches: usize,
}

impl LocalView {
    /// `cg_event.c:2805-2860`: a disruptor beam starts at the shooter's gun
    /// instead of the server muzzle: the local first-person shooter's at the
    /// view gun's flash point, anyone else's (and the local player's in third
    /// person) at the world gun's flash bolt. A gun not drawn on the last frame
    /// keeps the server muzzle.
    pub(crate) fn beam_start(self, event: sjk_client::LegacyImpactEvent) -> [f32; 3] {
        if !matches!(
            event.kind,
            LegacyImpactKind::DisruptorMainShot | LegacyImpactKind::DisruptorSniperShot
        ) {
            return event.start;
        }
        let shooter = u16::from(event.event_parameter);
        let gun = if shooter == self.client_num && !self.third_person {
            self.flash_point
        } else {
            self.world_muzzles.of(shooter)
        };
        gun.unwrap_or(event.start)
    }
}

/// Decode a received snapshot's impacts and spawn their visuals at `now`.
pub(crate) fn observe_snapshot(
    pool: &mut impacts::Pool,
    clash_flare: &mut LegacySaberClashFlare,
    snapshot: &Snapshot,
    local: LocalView,
    mut sinks: Sinks<'_>,
    now: Instant,
) -> Observation {
    let count = pool.observe(snapshot);
    let flesh_sparks = pool.flesh_sparks();
    let mut visual_requests = 0;
    let mut flare_latches = 0;
    for index in 0..count {
        let Some(impact) = pool.event(index) else {
            continue;
        };
        flare_latches += usize::from(latch_clash_flare(clash_flare, impact, snapshot.server_time));
        visual_requests += spawn_impact(impact, local, flesh_sparks, &mut sinks, now);
    }
    Observation {
        events: count,
        visual_requests,
        flare_latches,
    }
}

/// Apply the two stock event branches that write `cg_saberFlashTime/Pos`.
///
/// A saber-on-saber block latches only for nonzero `eventParm`, after the
/// block EFX, unless a custom saber sets `SFL2_NO_CLASH_FLARE`
/// (`codemp/cgame/cg_event.c:2287-2356`). The standalone flare event always
/// latches (`cg_event.c:2372-2377`). SJK does not yet resolve custom per-blade
/// saber flags, so this applies the stock branch used by the bundled demos.
pub(crate) fn latch_clash_flare(
    clash_flare: &mut LegacySaberClashFlare,
    impact: sjk_client::LegacyImpactEvent,
    server_time: i32,
) -> bool {
    let latches = sjk_client::legacy_impact_latches_saber_flare(impact);
    if latches {
        clash_flare.observe(impact.origin, server_time);
        // The blades near it clash too (the Blade Lab designs answer it).
        crate::saber_motion::note_clash(impact.origin);
    }
    latches
}

/// Spawn every visual `CG_EntityEvent` selects for one impact, a saber hit's sparks
/// at `cg_fleshSparks` `flesh_sparks` ([`impacts::plan`]).
pub(crate) fn spawn_impact(
    mut impact: sjk_client::LegacyImpactEvent,
    local: LocalView,
    flesh_sparks: u8,
    sinks: &mut Sinks<'_>,
    now: Instant,
) -> usize {
    impact.start = local.beam_start(impact);
    // `CG_VehicleWeaponImpact` (`cg_weapons.c`): a vehicle weapon's shot with an
    // `impactFX` plays it alone, in place of its carrier weapon's impact.
    if let Some(name) = sinks.missiles.vehicle_impact_effect(impact.vehicle_weapon) {
        effect_runtime::spawn_effect(
            sinks.particles,
            sinks.auxiliary,
            sinks.effects,
            sinks.vfs,
            name,
            Vec3::from_array(impact.origin),
            now,
            u32::from(impact.entity_number) | (u32::from(impact.event) << 16),
            0,
            sinks.audio,
            combat_effects::rotation_from_direction(impact.direction),
        );
        return 1;
    }
    let mut spawned = 0;
    for visual in impacts::plan(impact, flesh_sparks).iter() {
        // Each effect of a run has its own seed, as if it were its own visual.
        let seed = |visual_index: usize| {
            u32::from(impact.entity_number)
                | (u32::from(impact.event) << 16)
                | (spawned + visual_index) as u32
        };
        match visual {
            impacts::Visual::Effect { name, direction } => effect_runtime::spawn_effect(
                sinks.particles,
                sinks.auxiliary,
                sinks.effects,
                sinks.vfs,
                name,
                Vec3::from_array(impact.origin),
                now,
                seed(0),
                0,
                sinks.audio,
                combat_effects::rotation_from_direction(direction),
            ),
            impacts::Visual::EffectRun(run) => {
                for index in 0..run.count {
                    effect_runtime::spawn_effect(
                        sinks.particles,
                        sinks.auxiliary,
                        sinks.effects,
                        sinks.vfs,
                        run.name,
                        Vec3::from_array(run.position(index)),
                        now,
                        seed(index),
                        0,
                        sinks.audio,
                        combat_effects::rotation_from_direction(run.direction),
                    );
                }
            }
            impacts::Visual::Line(line) => {
                spawn_line(sinks.particles, sinks.effects, line, now, seed(0));
            }
        }
        spawned += visual.count();
    }
    spawned
}

/// `FX_AddLine` (`FxUtil.cpp:661-706`) as one static streak particle.
pub(crate) fn spawn_line(
    particles: &mut Vec<Particle>,
    effects: &mut EffectLibrary,
    line: impacts::Line,
    now: Instant,
    seed: u32,
) {
    if !crate::particle_room::effect_fits(particles.len()) {
        return;
    }
    let linear = sjk_effect::CurveFlags {
        linear: true,
        ..Default::default()
    };
    let constant = |value: f32| {
        crate::effect_envelope::Envelope::from_values(value, value, 0.0, Default::default())
    };
    let start = Vec3::from_array(line.start);
    particles.push(Particle {
        motion: crate::particle_motion::Motion::new(start, Vec3::ZERO, Vec3::ZERO, 0.0, 0.0, now),

        spawned_at: now,
        delay: Duration::ZERO,
        lifetime: Duration::from_millis(u64::from(line.lifetime_millis.max(1))),
        size: crate::effect_envelope::Envelope::from_values(
            line.size[0],
            line.size[1],
            0.0,
            linear,
        ),
        length: None,
        streak: Some(Vec3::from_array(line.end) - start),
        trace_streak: false,
        normal: None,
        alpha: crate::effect_envelope::Envelope::from_values(
            line.alpha[0],
            line.alpha[1],
            0.0,
            linear,
        ),
        use_alpha: false,
        set_shader_time: false,
        rgb: line.color.map(constant),
        seed,
        shader: effects.shader(line.shader),
        physics: crate::particle_physics::State::new(
            effects.code_primitive_definition(),
            0,
            0,
            0.0,
            sjk_effect::PrimitiveFlags::default(),
        ),
        shape: PrimitiveShape::Billboard,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(kind: LegacyImpactKind, shooter: u8) -> sjk_client::LegacyImpactEvent {
        sjk_client::LegacyImpactEvent {
            entity_number: 40,
            event: 35,
            kind,
            origin: [500.0, 0.0, 0.0],
            start: [1.0, 1.0, 1.0],
            direction: [0.0, 0.0, 1.0],
            event_parameter: shooter,
            weapon: 5,
            alternate: false,
            charge: 0,
            full_charge: false,
            shot: [0.0; 3],
            ring_direction: [1.0, 0.0, 0.0],
            vehicle_weapon: 0,
        }
    }

    fn view(third_person: bool) -> LocalView {
        let mut world_muzzles = crate::world_muzzles::Points::default();
        world_muzzles.record(2, [20.0, 0.0, 0.0]);
        world_muzzles.record(5, [50.0, 0.0, 0.0]);
        LocalView {
            client_num: 2,
            third_person,
            flash_point: Some([9.0, 9.0, 9.0]),
            world_muzzles,
        }
    }

    #[test]
    fn another_players_beam_starts_at_their_world_gun() {
        for kind in [
            LegacyImpactKind::DisruptorMainShot,
            LegacyImpactKind::DisruptorSniperShot,
        ] {
            assert_eq!(view(false).beam_start(shot(kind, 5)), [50.0, 0.0, 0.0]);
        }
    }

    #[test]
    fn the_local_beam_starts_at_the_view_gun_in_first_person_and_the_world_gun_in_third() {
        let event = shot(LegacyImpactKind::DisruptorSniperShot, 2);
        assert_eq!(view(false).beam_start(event), [9.0, 9.0, 9.0]);
        assert_eq!(view(true).beam_start(event), [20.0, 0.0, 0.0]);
    }

    #[test]
    fn a_gun_not_drawn_keeps_the_server_muzzle() {
        let event = shot(LegacyImpactKind::DisruptorMainShot, 7);
        assert_eq!(view(false).beam_start(event), [1.0, 1.0, 1.0]);
        let hidden_view_gun = LocalView {
            flash_point: None,
            ..view(false)
        };
        let local = shot(LegacyImpactKind::DisruptorMainShot, 2);
        assert_eq!(hidden_view_gun.beam_start(local), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn other_impacts_keep_their_start() {
        let event = shot(LegacyImpactKind::DisruptorHit, 5);
        assert_eq!(view(false).beam_start(event), [1.0, 1.0, 1.0]);
    }
}
