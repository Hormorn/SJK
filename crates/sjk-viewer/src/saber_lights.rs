//! Presentation-only light requests from the same transformed blades used for drawing.
use crate::dynamic_lights::{PointLight, PointLightList};
use crate::saber::{Blade, Color};
use crate::saber_rgb::BladeColor;
use glam::Vec3;

/// Keep CG_DoSaber/CG_DoSaberLight source selection with a broader emissive light.
/// Flicker uses a deterministic presentation-time seed, not the simulation RNG; a blade
/// skin's light also flickers in brightness, as its file says
/// ([`crate::saber_skins::LightFlicker`]), and turns its hue with the blade's when the
/// skin's does ([`crate::saber_skins::SkinColor::light_at`]); a skin's emitter stars
/// crackle in it when its file says ([`FlashLight`]), each blade's with its own stars:
/// `blade_seed` is the first blade's animation seed
/// ([`crate::saber::Instance::with_animation`]), blade `n`'s `blade_seed + n`, as every
/// caller numbers them, and `surge` how much they surge after the blade lit
/// ([`crate::saber_skins::SkinColor::flash_surge`], 0 none).
#[allow(clippy::too_many_arguments)]
pub(crate) fn append(
    output: &mut PointLightList,
    blades: &[Option<Blade>; 8],
    color: BladeColor,
    blade_count: u8,
    disabled: bool,
    time: i64,
    seed: u64,
    blade_seed: u32,
    surge: f32,
) {
    if disabled || color == BladeColor::Black {
        return;
    }
    // A skin's light flickers (and turns its hue) as its file says; the stock colours
    // hold steady.
    let flash = match color {
        BladeColor::Skin(skin) => skin.flash_light,
        _ => None,
    };
    let crackle = |index: usize| {
        flash.map_or(1.0, |flash| {
            flash.gain(time, blade_seed.wrapping_add(index as u32), surge)
        })
    };
    let (light, gain) = match color {
        BladeColor::Skin(skin) => (skin.light_at(time), 1.8 * skin.flicker.at(time, seed)),
        _ => (rgb(color), 1.8),
    };
    let color = light.map(|c| c * gain);
    let jitter = ((time as u64)
        .wrapping_mul(1664525)
        .wrapping_add(seed.wrapping_mul(1013904223))
        & 0xffff) as f32
        / 65535.0;
    // codemp/cgame/cg_players.c:6531 selects individual lights for one/two blades.
    if blade_count < 3 {
        for (index, blade) in lit(blades) {
            let crackle = crackle(index);
            output.push_radiant(PointLight {
                origin: (Vec3::from_array(blade.base)
                    + Vec3::from_array(blade.direction) * (blade.length * 0.5))
                    .to_array(),
                radius: blade.length * 2.4 + jitter * 3.0,
                color: color.map(|c| c * crackle),
            });
        }
        return;
    }
    // CG_DoSaberLight, cg_players.c:5275-5355. Bounded eight-tip calculation,
    // not a sweep over all actors. Current presentation uses one colour per hilt.
    let mut tips = [Vec3::ZERO; 8];
    let mut count = 0;
    let mut radius = 0.0_f32;
    let mut midpoint = Vec3::ZERO;
    let mut crackles = 0.0;
    for (index, blade) in lit(blades) {
        crackles += crackle(index);
        let base = Vec3::from_array(blade.base);
        let direction = Vec3::from_array(blade.direction);
        tips[count] = base + direction * blade.length;
        midpoint = base + direction * (blade.length * 0.5);
        radius = radius.max(blade.length * 2.4);
        count += 1;
    }
    if count == 0 {
        return;
    }
    if count > 1 {
        midpoint = tips[..count].iter().copied().sum::<Vec3>() / count as f32;
        for a in &tips[..count] {
            for b in &tips[..count] {
                radius = radius.max(a.distance(*b));
            }
        }
    }
    // One light for them all, crackling with their stars together.
    let crackle = crackles / count as f32;
    output.push_radiant(PointLight {
        origin: midpoint.to_array(),
        radius: radius + jitter * 8.0,
        color: color.map(|c| c * crackle),
    });
}

/// The blades long enough to light, by their place on the hilt.
fn lit(blades: &[Option<Blade>; 8]) -> impl Iterator<Item = (usize, &Blade)> {
    blades
        .iter()
        .enumerate()
        .filter_map(|(index, blade)| Some((index, blade.as_ref()?)))
        .filter(|(_, blade)| blade.length >= 0.5)
}

/// A skin's emitter stars ([`crate::blade_skin_file::smoke::Flash`]) as the blade's light
/// follows them (`flash.light`): `count` stars of `rays` rays, shuffled `rate` times a
/// second, thrown about by `chaos`. At each shuffle the light takes what `saber.wgsl`'s
/// `skin_flash` draws, from the same hashes: each star's flicker, and its rays' lengths and
/// brightness, as a share of a star's brightest, times `light`; more as they surge once
/// lit (`surge`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FlashLight {
    pub(crate) count: u8,
    pub(crate) rays: u8,
    pub(crate) rate: f32,
    pub(crate) chaos: f32,
    pub(crate) light: f32,
    pub(crate) surge: f32,
}

impl FlashLight {
    /// The light's gain (1 and up) at `time` (ms) for a blade of animation seed `seed`,
    /// its stars surging by `surge` (0 to 1).
    fn gain(&self, time: i64, seed: u32, surge: f32) -> f32 {
        if self.count == 0 {
            return 1.0;
        }
        // As the instance carries its time and seed (`Instance::with_animation`).
        let seconds =
            (time as f64 * 0.001).rem_euclid(crate::saber::Instance::ANIMATION_PERIOD) as f32;
        let shuffle = (seconds * self.rate).floor() as i32;
        let base = crate::saber::animation_seed(seed);
        let rays = i32::from(self.rays.max(1));
        let chaos = self.chaos;
        let mut stars = 0.0;
        for star in 0..self.count {
            // `star_glow` hands star n `seed + n * 0.37`; `skin_flash` makes its row.
            let row = ((base + f32::from(star) * 0.37) * 4096.0) as i32;
            let flicker = 1.0 - 0.6 * chaos * shader_hash(shuffle, row + 3);
            let mut lit = 0.0;
            for ray in 0..rays {
                let long = 1.0 - 0.75 * chaos * shader_hash(shuffle, row + ray * 31 + 17);
                let bright = 1.0 - 0.6 * chaos * shader_hash(shuffle, row + ray * 31 + 23);
                lit += long * bright;
            }
            stars += flicker * lit / rays as f32;
        }
        1.0 + self.light * stars / f32::from(self.count) * (1.0 + self.surge * surge)
    }
}

/// `saber.wgsl`'s `skin_hash`, bit for bit, so the light crackles as the stars do.
fn shader_hash(x: i32, y: i32) -> f32 {
    let mut v = (x as u32)
        .wrapping_mul(747_796_405)
        .wrapping_add(2_891_336_453);
    v ^= (y as u32).wrapping_mul(2_654_435_761);
    v = v.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let word = ((v >> ((v >> 28) + 4)) ^ v).wrapping_mul(277_803_737);
    ((word >> 22) ^ word) as f32 / 4_294_967_295.0
}

pub(crate) fn rgb(color: BladeColor) -> [f32; 3] {
    // CG_RGBForSaberColor, cg_players.c:5246-5273; not the white retail sprite tint.
    match color {
        BladeColor::Black => [0.0; 3],
        BladeColor::Retail(Color::Red) => [1.0, 0.2, 0.2],
        BladeColor::Retail(Color::Orange) => [1.0, 0.5, 0.1],
        BladeColor::Retail(Color::Yellow) => [1.0, 1.0, 0.2],
        BladeColor::Retail(Color::Green) => [0.2, 1.0, 0.2],
        BladeColor::Retail(Color::Blue) => [0.2, 0.4, 1.0],
        BladeColor::Retail(Color::Purple) => [0.9, 0.2, 1.0],
        BladeColor::Rgb(rgb) => rgb.map(|v| f32::from(v) / 255.0),
        BladeColor::Skin(skin) => skin.light_color(),
    }
}
