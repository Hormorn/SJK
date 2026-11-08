//! Normal maps on models (SJK, 05/11/2026): a model stage lit per entity
//! (`rgbGen lightingDiffuse` or `lightingDiffuseEntity`) that names its normal map
//! with rend2's `normalMap` keyword is lit per pixel through it.
//!
//! The world's material program does not fit models: it reads tangents stored per
//! vertex of the flattened world, and entity vertices (`GpuVertex`, skinned by
//! Ghoul2) carry none. Here the map rides in the stage's second texture slot, which
//! an uncollapsed model stage leaves idle, uploaded linear (it is data, not colour;
//! [`KEY_PREFIX`] marks it for the forge), and the stage program builds the tangent
//! frame from the screen-space derivatives of the position and the picture
//! coordinates (`model_normal_prepare` in `stage_runtime.wgsl`), so MD3 and Ghoul2
//! models need no tangents. [`STAGE_FLAG`] in `wave_functions.w` turns it on;
//! `secondary_animation.xy` carries the stage's `normalScale`.
//!
//! Only an explicit keyword counts: a `<diffuse>_n` image found by name, which rend2
//! texture packs ship for the world, is never applied to a model, so stock and
//! community models look as they always have. `r_normalMapping` governs it like the
//! world's maps.

use super::{CONTROLS, DEFAULT_ON, LATCH, LATCHED, MapImage, images};
use crate::world_stage::{CompiledStage, LIGHTMAP_NONE};
use image::RgbaImage;
use sjk_shader::{ShaderCatalog, TextureGenerator};
use sjk_vfs::VirtualFileSystem;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Texture-cache key prefix of a model's normal map: the forge uploads it linear.
pub(crate) const KEY_PREFIX: &str = "$normal:";
/// `wave_functions.w` bit of a stage lit through its model normal map (1, 2, 4 and 8
/// are the vertex-light, debug-end, diffuse-cover and relit-shine bits).
pub(crate) const STAGE_FLAG: u32 = 16;

/// A model stage's normal map and its strength (`normalScale` x and y).
#[derive(Clone, Debug)]
pub(crate) struct ModelNormal {
    /// The texture-cache key, [`KEY_PREFIX`] first.
    pub(crate) key: String,
    pub(crate) pixels: Arc<RgbaImage>,
    pub(crate) scale: [f32; 2],
}

/// Whether `r_normalMapping` is on as the running viewer sampled it (on before the
/// first sample, its default). Late entity and detached materials compile without the
/// world's settings, so they read the latch here.
pub(crate) fn enabled() -> bool {
    enabled_in(LATCH.load(Ordering::Relaxed))
}

fn enabled_in(latch: u8) -> bool {
    debug_assert_eq!(CONTROLS[0], "r_normalMapping");
    if latch & LATCHED == 0 {
        DEFAULT_ON
    } else {
        latch & 1 != 0
    }
}

/// Whether a hardware stage can be lit through a model normal map: a model's
/// (`lightmap` none), uncollapsed (its second slot free), a picture lit per entity.
pub(crate) fn eligible(stage: &CompiledStage, lightmap: i32) -> bool {
    lightmap == LIGHTMAP_NONE
        && stage.secondary.is_none()
        && stage.primary.texture_generator == TextureGenerator::Base
        && stage.primary.material.normal_map.is_some()
        && stage
            .primary
            .rgb_generator
            .as_deref()
            .is_some_and(|generator| {
                generator.eq_ignore_ascii_case("lightingDiffuse")
                    || generator.eq_ignore_ascii_case("lightingDiffuseEntity")
            })
}

/// The normal map of a model stage, or `None` when the stage cannot take one, names
/// none, it is missing, or `r_normalMapping` is off.
pub(crate) fn resolve(
    vfs: &VirtualFileSystem,
    shaders: &ShaderCatalog,
    stage: &CompiledStage,
    lightmap: i32,
    cache: &mut HashMap<String, Arc<RgbaImage>>,
) -> Result<Option<ModelNormal>, Box<dyn Error>> {
    if !enabled() || !eligible(stage, lightmap) {
        return Ok(None);
    }
    let Some(MapImage { key, pixels }) =
        images::explicit_normal(vfs, shaders, &stage.primary, cache)?
    else {
        return Ok(None);
    };
    let scale = stage.primary.material.normal_scale;
    Ok(Some(ModelNormal {
        key: format!("{KEY_PREFIX}{key}"),
        pixels,
        scale: [scale[0], scale[1]],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_stage::LIGHTMAP_BY_VERTEX;
    use crate::world_stage::collapse_multitexture;

    fn stage(text: &str) -> CompiledStage {
        let definitions = sjk_shader::parse_shader_script(text.as_bytes(), "scripts/t.shader")
            .expect("script parses");
        collapse_multitexture(&definitions[0].stages).remove(0)
    }

    const LIT: &str =
        "models/a\n{\n{\nmap models/a\nnormalMap models/a_n\nrgbGen lightingDiffuse\n}\n}";

    #[test]
    fn only_a_named_map_on_a_model_stage_lit_per_entity_counts() {
        assert!(eligible(&stage(LIT), LIGHTMAP_NONE));
        let entity = LIT.replace("lightingDiffuse", "lightingDiffuseEntity");
        assert!(eligible(&stage(&entity), LIGHTMAP_NONE));
        // A world surface keeps the world's material maps.
        assert!(!eligible(&stage(LIT), 0));
        assert!(!eligible(&stage(LIT), LIGHTMAP_BY_VERTEX));
        // No keyword: a `_n` image found by name is never applied to a model.
        let unnamed = LIT.replace("normalMap models/a_n\n", "");
        assert!(!eligible(&stage(&unnamed), LIGHTMAP_NONE));
        // Other colour generators keep their authored shading.
        let identity = LIT.replace("lightingDiffuse", "identity");
        assert!(!eligible(&stage(&identity), LIGHTMAP_NONE));
        // A collapsed stage's second slot is taken.
        let mut collapsed = stage(LIT);
        collapsed.secondary = Some(collapsed.primary.clone());
        assert!(!eligible(&collapsed, LIGHTMAP_NONE));
    }

    #[test]
    fn the_switch_follows_r_normal_mapping_once_sampled() {
        assert_eq!(enabled_in(0), DEFAULT_ON, "before the sample: the default");
        assert!(enabled_in(LATCHED | 1));
        assert!(!enabled_in(LATCHED | 0b1110));
    }
}
