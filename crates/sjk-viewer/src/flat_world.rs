//! `r_flatWorld` (Sol's Tactical mode at Max, 11/11/2026, `tactical.rs`): the world in
//! flat colours, the "flat competitive" look. Each world surface's texture is drawn as
//! its average colour (its smallest mip level) under the map's own light (lightmap or
//! vertex colours, live light too), and only the stages up to the first later stage that
//! is no lightmap are drawn, as `r_lightmap` stops them: no animated overlays, glows or
//! environment shines. Alpha still cuts grates and foliage. Models, effects and the HUD
//! are untouched.
//!
//! Live: the switch rides bit [`BIT`] of the model word of the scene-light block
//! (`point_lights.metadata.y`, `model_light_settings.rs`), read in `stage_runtime.wgsl`.

use sjk_shell::{CvarDefinition, CvarError, CvarFlags, CvarRegistry, CvarValue};
use std::sync::atomic::{AtomicBool, Ordering};

/// The cvar's name.
pub(crate) const CVAR: &str = "r_flatWorld";
/// Its bit in the model word (bit 0 is per-pixel grid light, bits 1..3 the toon mode).
pub(crate) const BIT: u32 = 1 << 4;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// The model word's bits for the switch.
pub(crate) fn bits() -> u32 {
    if ENABLED.load(Ordering::Relaxed) {
        BIT
    } else {
        0
    }
}

fn set(value: &CvarValue) {
    let on = match value {
        CvarValue::Integer(value) => *value != 0,
        CvarValue::Bool(on) => *on,
        CvarValue::Float(value) => *value != 0.,
        CvarValue::Text(_) => false,
    };
    ENABLED.store(on, Ordering::Relaxed);
}

/// Register `r_flatWorld` (archived, off by default: the stock look) and follow it.
pub(crate) fn bind(cvars: &mut CvarRegistry) -> Result<(), CvarError> {
    cvars.register(CvarDefinition::new(
        CVAR,
        0_i64,
        CvarFlags::ARCHIVE | CvarFlags::OMIT_DEFAULT,
        "Flat world: each surface in its texture's average colour under the map's light, \
         no shader overlays (Tactical max); live",
    ))?;
    set(&cvars.get(CVAR).unwrap().value);
    cvars.on_change(CVAR, |change| set(&change.current))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_is_off_by_default_and_sets_its_bit_live() {
        let mut cvars = CvarRegistry::new();
        bind(&mut cvars).unwrap();
        assert_eq!(cvars.get(CVAR).unwrap().value, CvarValue::Integer(0));
        cvars.set_text(CVAR, "1").unwrap();
        assert_eq!(bits(), BIT);
        cvars.set_text(CVAR, "0").unwrap();
        assert_eq!(bits(), 0);
    }

    #[test]
    fn the_bit_is_free_of_the_model_words_others() {
        // Bit 0 per-pixel light, bits 1..3 the toon mode (`model_light_settings.rs`).
        assert_eq!(BIT & 0b111, 0);
    }
}
