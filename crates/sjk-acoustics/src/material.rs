//! What the walls of a level are made of, for the sound: how much of each band a
//! surface swallows, how much it scatters, and how much of each band gets through a
//! wall of it.
//!
//! A surface's material is Jedi Academy's (the footsteps' `MATERIAL_*` of
//! `surfaceflags.h`, the low five bits of a shader's surface flags). Few levels name
//! one, so [`ShaderMaterials`] holds a table of shader names and materials first,
//! `data/materials.txt`: VisionJK's (Blackwolf, GPL-2.0, used with his agreement),
//! the material each shader's script names or, where none does, what its texture
//! looks like. The numbers per material are SJK's own, after common published
//! absorption tables, and game-friendly rather than physical: hard surfaces
//! swallow a few times more than bare ones do (a level is a furnished place, not
//! a sealed concrete box ringing for seconds), and walls let more through (a real
//! concrete wall lets through almost nothing).

use std::collections::HashMap;

/// `MATERIAL_MASK`: the material id's bits in `surfaceFlags`.
pub const MATERIAL_MASK: u32 = 0x1f;
/// `SURF_SKY`.
pub const SURF_SKY: u32 = 0x0000_2000;
/// `SURF_NODRAW`.
pub const SURF_NODRAW: u32 = 0x0000_0080;

/// One material's acoustics, per band (low, mid, high).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Acoustic {
    /// Share of the energy a reflection swallows.
    pub absorption: [f32; 3],
    /// Share of what is reflected that scatters (the rest mirrors).
    pub scattering: f32,
    /// Share of the sound's amplitude that gets through one wall of it.
    pub transmission: [f32; 3],
}

const fn acoustic(absorption: [f32; 3], scattering: f32, transmission: [f32; 3]) -> Acoustic {
    Acoustic {
        absorption,
        scattering,
        transmission,
    }
}

/// The index of [`SKY`] in [`MATERIALS`]: open air, it swallows every ray.
pub const SKY: u8 = 32;

const SOLID: [f32; 3] = [0.18, 0.06, 0.02];
const THIN: [f32; 3] = [0.35, 0.15, 0.05];
const LOOSE: [f32; 3] = [0.7, 0.5, 0.3];

/// Jedi Academy's 32 materials by id, then the sky.
pub const MATERIALS: [Acoustic; 33] = [
    acoustic([0.10, 0.15, 0.20], 0.10, SOLID), // 0 none: plain painted walls
    acoustic([0.15, 0.10, 0.10], 0.10, [0.15, 0.06, 0.02]), // 1 solid wood
    acoustic([0.25, 0.10, 0.10], 0.10, THIN),  // 2 hollow wood
    acoustic([0.06, 0.06, 0.08], 0.05, [0.12, 0.04, 0.01]), // 3 solid metal
    acoustic([0.10, 0.08, 0.08], 0.05, THIN),  // 4 hollow metal
    acoustic([0.11, 0.26, 0.60], 0.40, SOLID), // 5 short grass
    acoustic([0.20, 0.40, 0.70], 0.60, SOLID), // 6 long grass
    acoustic([0.15, 0.35, 0.50], 0.40, SOLID), // 7 dirt
    acoustic([0.15, 0.40, 0.55], 0.30, SOLID), // 8 sand
    acoustic([0.25, 0.50, 0.70], 0.50, SOLID), // 9 gravel
    acoustic([0.18, 0.06, 0.04], 0.05, THIN),  // 10 glass
    acoustic([0.05, 0.07, 0.10], 0.10, SOLID), // 11 concrete
    acoustic([0.04, 0.05, 0.07], 0.05, [0.12, 0.04, 0.01]), // 12 marble
    acoustic([0.04, 0.05, 0.07], 0.05, THIN),  // 13 water
    acoustic([0.45, 0.75, 0.90], 0.50, SOLID), // 14 snow
    acoustic([0.04, 0.05, 0.07], 0.05, SOLID), // 15 ice
    acoustic([0.30, 0.50, 0.60], 0.30, THIN),  // 16 flesh
    acoustic([0.15, 0.35, 0.45], 0.40, SOLID), // 17 mud
    acoustic([0.15, 0.05, 0.04], 0.05, [0.22, 0.08, 0.03]), // 18 bulletproof glass
    acoustic([0.30, 0.50, 0.70], 0.70, LOOSE), // 19 dry leaves
    acoustic([0.30, 0.50, 0.70], 0.70, LOOSE), // 20 green leaves
    acoustic([0.15, 0.50, 0.70], 0.40, [0.6, 0.4, 0.2]), // 21 fabric
    acoustic([0.10, 0.35, 0.55], 0.30, [0.6, 0.4, 0.2]), // 22 canvas
    acoustic([0.06, 0.08, 0.10], 0.40, [0.12, 0.04, 0.01]), // 23 rock
    acoustic([0.05, 0.10, 0.15], 0.10, THIN),  // 24 rubber
    acoustic([0.05, 0.08, 0.10], 0.10, THIN),  // 25 plastic
    acoustic([0.04, 0.05, 0.07], 0.05, SOLID), // 26 tiles
    acoustic([0.08, 0.35, 0.65], 0.30, SOLID), // 27 carpet
    acoustic([0.10, 0.05, 0.04], 0.10, [0.22, 0.08, 0.03]), // 28 plaster
    acoustic([0.18, 0.06, 0.04], 0.05, THIN),  // 29 shatter glass
    acoustic([0.06, 0.06, 0.08], 0.05, [0.12, 0.04, 0.01]), // 30 armor
    acoustic([0.10, 0.08, 0.08], 0.10, THIN),  // 31 computer
    acoustic([1.0, 1.0, 1.0], 0.0, [1.0, 1.0, 1.0]), // sky
];

/// Material names of `data/materials.txt`, in id order.
const NAMES: [&str; 32] = [
    "none",
    "solidwood",
    "hollowwood",
    "solidmetal",
    "hollowmetal",
    "shortgrass",
    "longgrass",
    "dirt",
    "sand",
    "gravel",
    "glass",
    "concrete",
    "marble",
    "water",
    "snow",
    "ice",
    "flesh",
    "mud",
    "bpglass",
    "dryleaves",
    "greenleaves",
    "fabric",
    "canvas",
    "rock",
    "rubber",
    "plastic",
    "tiles",
    "carpet",
    "plaster",
    "shatterglass",
    "armor",
    "computer",
];

static TABLE: &str = include_str!("../data/materials.txt");

/// The shader-name table: what a shader is made of when the level does not say.
pub struct ShaderMaterials {
    names: HashMap<String, u8>,
}

impl ShaderMaterials {
    /// The bundled table.
    pub fn bundled() -> Self {
        Self::parse(TABLE)
    }

    fn parse(text: &str) -> Self {
        let names = text
            .lines()
            .filter(|line| !line.starts_with("//"))
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let (shader, material) = (words.next()?, words.next()?);
                let id = NAMES.iter().position(|name| *name == material)?;
                Some((shader.to_ascii_lowercase(), id as u8))
            })
            .collect();
        Self { names }
    }

    /// Shaders the table names.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// What a shader of `name` with `surface_flags` is made of: the table's word
    /// first, then the level's, else `None` (the generic wall).
    pub fn material(&self, name: &str, surface_flags: u32) -> Option<u8> {
        if surface_flags & SURF_SKY != 0 {
            return Some(SKY);
        }
        let name = name.trim_end_matches('\0').to_ascii_lowercase();
        let name = name
            .strip_suffix(".tga")
            .or(name.strip_suffix(".jpg"))
            .unwrap_or(&name);
        self.names.get(name).copied().or_else(|| {
            let id = (surface_flags & MATERIAL_MASK) as u8;
            (id != 0).then_some(id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_table_names_more_than_a_thousand_shaders() {
        assert!(ShaderMaterials::bundled().len() > 1_000);
    }

    #[test]
    fn the_table_goes_before_the_level_and_the_sky_before_both() {
        let table = ShaderMaterials::parse("textures/a/wall concrete\n// a note\n");
        assert_eq!(table.material("textures/a/wall", 3), Some(11));
        assert_eq!(table.material("TEXTURES/A/WALL", 0), Some(11));
        assert_eq!(table.material("textures/b/floor", 3), Some(3));
        assert_eq!(table.material("textures/b/floor", 0), None);
        assert_eq!(table.material("textures/a/wall", SURF_SKY), Some(SKY));
    }

    #[test]
    fn every_material_lets_less_of_the_highs_through_than_of_the_lows() {
        for material in &MATERIALS[..32] {
            assert!(material.transmission[2] <= material.transmission[0]);
            assert!(material.absorption.iter().all(|a| (0.0..1.0).contains(a)));
        }
    }
}
