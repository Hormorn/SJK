//! Per-instance entity colour without legacy render-effect overrides.
//!
//! BaseJKA copies player `shaderRGBA` onto the player refent, while attached
//! weapon refents are initialized independently by `CG_AddPlayerWeapon`
//! (`codemp/cgame/cg_weapons.c:421-498`). Consequently this helper changes
//! only the actor instance and deliberately leaves `entity_control` clear:
//! ordinary `shaderRGBA` is consumed only by entity-aware shader generators.

use super::ActorInstance;

impl ActorInstance {
    /// Supply shader entity colour without forcing `CGEN_ENTITY`.
    pub(super) fn with_entity_color(mut self, color: [u8; 4]) -> Self {
        self.entity_color = color.map(|channel| f32::from(channel) / 255.0);
        self
    }

    /// Supply a model's `customRGBA` (the `char_color_*` colour) as the entity colour.
    /// Retail multiplies the texel by it in display values; SJK lights the world in
    /// linear light from sRGB-decoded textures, so the colour is decoded as well, or a
    /// channel of 128 would show at about 73 % instead of half and dark colours pale.
    /// Alpha stays as it is. `oneMinusEntity` then reads one less the decoded colour,
    /// close enough for the stages that use it.
    pub(super) fn with_model_color(self, color: [u8; 4]) -> Self {
        let mut instance = self.with_entity_color(color);
        for channel in &mut instance.entity_color[..3] {
            *channel = crate::ui_target::srgb_to_linear(*channel);
        }
        instance
    }

    /// Apply rd-vanilla's `RF_RGB_TINT` override for callers that request it.
    pub(super) fn with_rgb_tint(mut self, color: [f32; 4]) -> Self {
        self.entity_color = color;
        self.entity_control[0] = 1.0;
        self
    }

    /// Apply rd-vanilla's RGB and alpha render-effect overrides.
    pub(super) fn with_rgba_tint(mut self, color: [f32; 4]) -> Self {
        self.entity_color = color;
        self.entity_control = [1.0; 2];
        self
    }

    /// Apply rd-vanilla's `RF_FORCE_ENT_ALPHA` vertex alpha (`ForceAlpha`,
    /// `tr_shade.cpp:1546-1556`), keeping the entity's RGB and RGB generators.
    /// The draw must also select the forced-alpha pipeline.
    pub(super) fn with_forced_alpha(mut self, alpha: u8) -> Self {
        self.entity_color[3] = f32::from(alpha) / 255.0;
        self.entity_control[1] = 1.0;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ActorInstance;

    #[test]
    fn a_model_colour_is_decoded_to_linear_light_but_its_alpha_is_not() {
        let instance = ActorInstance::new([0.0; 3], [0.0, 0.0, 0.0, 1.0], [1.0; 3])
            .with_model_color([255, 128, 0, 128]);
        let [r, g, b, a] = instance.entity_color;
        assert_eq!((r, b), (1.0, 0.0));
        // Display 128 is linear 0.216: drawn and encoded again it shows as half.
        assert!((g - 0.2158).abs() < 1e-3, "{g}");
        assert!((a - 128.0 / 255.0).abs() < 1e-6);
        assert_eq!(instance.entity_control, [0.0; 2]);
    }
}
