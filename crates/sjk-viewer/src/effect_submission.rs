//! Builds the fixed effect draw lists shared by billboard and geometry FX.

use super::*;
use crate::particle_types::PrimitiveShape;

pub(crate) struct Inputs<'a> {
    pub(crate) geometry: &'a mut crate::effect_geometry_gpu::Runtime,
    pub(crate) queue: &'a crate::frame_queue::FrameQueue,
    pub(crate) encoder: &'a mut wgpu::CommandEncoder,
    pub(crate) particles: &'a mut [Particle],
    pub(crate) decals: &'a mut crate::decal_store::DecalStore,
    pub(crate) atlas: &'a ParticleAtlas,
    pub(crate) now: Instant,
    pub(crate) global_seconds: f32,
    pub(crate) camera: Vec3,
    pub(crate) field_of_view: f32,
    /// The view's width over its height, which sizes a screen flash
    /// ([`crate::effect_flash::screen_half_extents`]).
    pub(crate) aspect: f32,
    pub(crate) bsp: &'a Bsp,
    pub(crate) trace_scratch: &'a mut TraceScratch,

    pub(crate) entity_instances: &'a mut Vec<EntityInstance>,
    pub(crate) blended: &'a mut [Vec<EntityInstance>; crate::effect_blend::PIPELINE_COUNT],
}

pub(crate) struct Ranges {
    pub(crate) opaque: Range<u32>,
    blended: [Range<u32>; crate::effect_blend::PIPELINE_COUNT],
    /// The dynamic glow layers at the end of each blended range.
    glow: [Range<u32>; crate::effect_blend::PIPELINE_COUNT],
}

impl Ranges {
    pub(crate) fn blended(&self) -> impl Iterator<Item = Range<u32>> + '_ {
        self.blended.iter().cloned()
    }

    /// Per blend slot, the billboards whose shader stage glows (`r_DynamicGlow`).
    pub(crate) fn glow(&self) -> impl Iterator<Item = Range<u32>> + '_ {
        self.glow.iter().cloned()
    }
}

/// Marks a glowing billboard layer while the slots are gathered; cleared on packing.
const GLOW_MARK: u32 = 1 << 31;

/// Append one blend slot's billboards, glowing layers last, and return the slot's range
/// and its glowing tail. The partition is stable and allocation-free; it only moves
/// glowing layers behind the others of their slot, which changes no additive or
/// modulating result and is drawn back to front within each part for alpha blending.
fn append_slot(
    destination: &mut Vec<EntityInstance>,
    group: &[EntityInstance],
) -> (Range<u32>, Range<u32>) {
    let index =
        |destination: &Vec<EntityInstance>| u32::try_from(destination.len()).unwrap_or(1_024);
    let start = index(destination);
    destination.extend(group.iter().filter(|i| i.kind & GLOW_MARK == 0).copied());
    let glow = index(destination);
    destination.extend(
        group
            .iter()
            .filter(|i| i.kind & GLOW_MARK != 0)
            .map(|i| EntityInstance {
                kind: i.kind & !GLOW_MARK,
                ..*i
            }),
    );
    let end = index(destination);
    (start..end, glow..end)
}

/// Prepare unchanged geometry, uploads, billboards and stable sort with disjoint host timings.
pub(crate) fn prepare(timing: &mut frame_pacing::budget::Timer, inputs: Inputs<'_>) -> Ranges {
    inputs.geometry.prepare(
        timing,
        inputs.queue,
        inputs.encoder,
        inputs.particles,
        inputs.decals,
        inputs.atlas,
        inputs.now,
        inputs.global_seconds,
        inputs.camera,
        inputs.field_of_view,
        inputs.bsp,
        inputs.trace_scratch,
    );
    timing.mark(frame_pacing::budget::Phase::EffectBillboards);

    let capacity =
        crate::particle_types::INSTANCE_CAPACITY.saturating_sub(inputs.entity_instances.len());
    let reduced_flashes = crate::effect_flash::reduced();
    'particles: for particle in inputs.particles.iter_mut() {
        if !matches!(
            particle.shape,
            PrimitiveShape::Billboard
                | PrimitiveShape::FrameBillboard
                | PrimitiveShape::ScreenFlash
                | PrimitiveShape::Spark
        ) {
            continue;
        }
        let age = inputs.now.saturating_duration_since(particle.spawned_at);
        if age < particle.delay {
            continue;
        }
        // Reduced flashes (`cg_reduceFlashes`): a pack's flare-picture particles too.
        if reduced_flashes && crate::effect_flash::is_clash_flare_picture(&particle.shader) {
            continue;
        }
        crate::effect_geometry::resolve_traced_streak(particle, inputs.bsp, inputs.trace_scratch);
        let particle = &*particle;
        let seconds = age.saturating_sub(particle.delay).as_secs_f32();
        let motion = particle.motion.sample();
        let (size, base_alpha, color) = particle.sample_envelopes(seconds);
        let direction = match (particle.shape, particle.streak) {
            // Along the way it goes now, the tail behind it.
            (PrimitiveShape::Spark, Some(_)) => {
                streak_direction(particle, -motion.velocity, seconds)
            }
            (_, Some(streak)) => streak_direction(particle, streak, seconds),
            (_, None) => Vec3::ZERO,
        };
        let direction = particle.normal.unwrap_or(direction).to_array();
        let (color_fade, vertex_alpha) =
            effect_runtime::particle_fade(particle.use_alpha, base_alpha);
        let shader_seconds = particle.shader_seconds(seconds, inputs.global_seconds);
        for layer in inputs
            .atlas
            .layers_for(&particle.shader, shader_seconds)
            .iter()
        {
            let emitted = inputs.blended.iter().map(Vec::len).sum::<usize>();
            if emitted >= capacity {
                break 'particles;
            }
            let uv_transform = billboard_uv_transform(particle.shape, layer.uv_transform);
            let screen_flash = matches!(particle.shape, PrimitiveShape::ScreenFlash);
            let kind = if screen_flash {
                6 // Clip-space quad over the view, in front of everything.
            } else if particle.normal.is_some() {
                5
            } else if particle.streak.is_some() {
                4
            } else if matches!(particle.shape, PrimitiveShape::FrameBillboard) {
                7 // World icon: retain texture alpha without soft-particle fading.
            } else {
                3
            };
            let tint = layer.tint(color.map(|channel| channel * color_fade));
            let (position, size, direction) = if screen_flash {
                let [width, height] = crate::effect_flash::screen_half_extents(inputs.aspect);
                ([0.0; 3], 1.0, [width, height, 0.0])
            } else {
                (motion.origin.to_array(), size, direction)
            };
            let instance = EntityInstance {
                position,
                kind: if layer.glow { kind | GLOW_MARK } else { kind },
                size,
                alpha: vertex_alpha * layer.alpha,
                uv_rect: layer.uv_rect,
                color: [
                    tint[0] * layer.rgb,
                    tint[1] * layer.rgb,
                    tint[2] * layer.rgb,
                    1.0,
                ],
                direction,
                rotation: motion.rotation_degrees,
                uv_transform,
            };
            inputs.blended[crate::effect_blend::slot(layer.blend)].push(instance);
        }
    }
    timing.mark(frame_pacing::budget::Phase::EffectSort);

    inputs.blended[0].sort_by(|left, right| {
        let left_distance = Vec3::from_array(left.position).distance_squared(inputs.camera);
        let right_distance = Vec3::from_array(right.position).distance_squared(inputs.camera);
        right_distance.total_cmp(&left_distance)
    });
    let opaque = 0..u32::try_from(inputs.entity_instances.len()).unwrap_or(1_024);
    let mut glow = std::array::from_fn(|_| 0..0);
    let blended = std::array::from_fn(|index| {
        if !inputs.atlas.any_glow {
            return append_instance_group(inputs.entity_instances, &inputs.blended[index]);
        }
        let (range, glowing) = append_slot(inputs.entity_instances, &inputs.blended[index]);
        glow[index] = glowing;
        range
    });
    Ranges {
        opaque,
        blended,
        glow,
    }
}

/// The `[scale_u, scale_v, offset_u, offset_v]` texture transform for one billboard layer.
///
/// Frame icons use `RT_SPRITE`'s top-down image coordinates: OpenJK's
/// `RB_AddQuadStamp` puts t=0 at +up. The particle quad in `entity.wgsl` instead
/// has local v=1 there. Reflect its local v before applying the authored tcMod so
/// chat/connection and simple-item icons are upright; FX keep their existing
/// texture convention.
pub(crate) fn billboard_uv_transform(shape: PrimitiveShape, uv_transform: [f32; 4]) -> [f32; 4] {
    let [scale_u, scale_v, offset_u, offset_v] = uv_transform;
    if matches!(shape, PrimitiveShape::FrameBillboard) {
        [scale_u, -scale_v, offset_u, offset_v + scale_v]
    } else {
        uv_transform
    }
}

/// `CTail::UpdateLength` keeps the start value unless a length flag changes it.
/// Signed lengths are intentional: exhaust tails can extend along their velocity.
fn streak_direction(particle: &Particle, streak: Vec3, seconds: f32) -> Vec3 {
    let Some(length) = particle.length else {
        return streak;
    };
    streak.normalize_or_zero()
        * length.sample(
            seconds * 1_000.0,
            particle.lifetime.as_secs_f32() * 1_000.0,
            particle.seed.wrapping_add(35),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(kind: u32, size: f32) -> EntityInstance {
        EntityInstance {
            position: [0.0; 3],
            kind,
            size,
            alpha: 1.0,
            uv_rect: [0.0; 4],
            color: [1.0; 4],
            direction: [0.0; 3],
            rotation: 0.0,
            uv_transform: [1.0, 1.0, 0.0, 0.0],
        }
    }

    fn effect_particle(kind: &str, flags: &str) -> Particle {
        let source = format!(
            "{kind} {{ life 120 velocity -2 0 0 origin2 3 4 0 length {{ start -24 end -60 {flags} }} shaders [ gfx/test/exhaust ] }}"
        );
        let mut vfs = sjk_vfs::VirtualFileSystem::new();
        vfs.mount_memory("test", [("effects/test/exhaust.efx", source)])
            .unwrap();
        let mut particles = Vec::new();
        crate::effect_runtime::spawn_effect(
            &mut particles,
            &mut crate::effect_aux::Runtime::default(),
            &mut crate::EffectLibrary::default(),
            &vfs,
            "test/exhaust",
            Vec3::ZERO,
            Instant::now(),
            1,
            0,
            &mut None,
            glam::Quat::IDENTITY,
        );
        assert_eq!(particles.len(), 1);
        particles.pop().unwrap()
    }

    #[test]
    fn exhaust_tails_keep_signed_length_unless_a_length_flag_changes_it() {
        let fixed = effect_particle("Tail", "");
        let linear = effect_particle("Tail", "flags linear");
        for step in [8, 7, 4, 3] {
            for millis in (0..120).step_by(step) {
                let seconds = millis as f32 / 1_000.0;
                let fixed_direction = streak_direction(&fixed, fixed.streak.unwrap(), seconds);
                assert!(fixed_direction.abs_diff_eq(-Vec3::X * 24.0, 1e-5));
                let expected = -24.0 + (-60.0 + 24.0) * millis as f32 / 120.0;
                assert!(
                    (streak_direction(&linear, linear.streak.unwrap(), seconds).x - expected).abs()
                        < 1e-4
                );
            }
        }
        let line = effect_particle("Line", "flags linear");
        assert!(line.length.is_none());
        assert_eq!(
            streak_direction(&line, line.streak.unwrap(), 0.06),
            Vec3::new(3.0, 4.0, 0.0)
        );
    }

    #[test]
    fn glowing_billboards_follow_the_rest_of_their_slot_in_order() {
        let mut packed = Vec::with_capacity(16);
        packed.push(instance(3, 0.0));
        let group = [
            instance(3 | GLOW_MARK, 1.0),
            instance(4, 2.0),
            instance(5 | GLOW_MARK, 3.0),
            instance(3, 4.0),
        ];
        let (range, glow) = append_slot(&mut packed, &group);
        assert_eq!(range, 1..5);
        assert_eq!(glow, 3..5);
        let sizes: Vec<f32> = packed[1..].iter().map(|i| i.size).collect();
        assert_eq!(sizes, [2.0, 4.0, 1.0, 3.0]);
        // The mark never reaches the GPU's billboard kinds.
        let kinds: Vec<u32> = packed[1..].iter().map(|i| i.kind).collect();
        assert_eq!(kinds, [4, 3, 3, 5]);
        let (range, glow) = append_slot(&mut packed, &[instance(3, 5.0)]);
        assert_eq!((range, glow), (5..6, 6..6));
    }
}
