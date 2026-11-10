use super::*;
use crate::blade_skin_file::tests::{SAMPLE, sample_with};
use crate::saber::{self, Blade, Color, Instance};
use crate::saber_rgb::{BLACK_MATERIAL, MATERIAL_COUNT, RGB_MATERIAL, SKIN_MATERIAL};
use crate::saber_submission::lights;

fn blade() -> Blade {
    Blade {
        base: [0.0; 3],
        direction: [0.0, 0.0, 1.0],
        length: 40.0,
        radius: 3.0,
    }
}

/// The test blade skin ([`SAMPLE`], made-up values) loaded as the catalogue's one blade
/// skin, numbered 0, under `generation`.
pub(crate) fn loaded_sample(generation: u64) -> LoadedSkins {
    let def = crate::blade_skin_file::parse("saber_tidal", SAMPLE).unwrap();
    let skin = LoadedSkin::new("saber_tidal", def, &VirtualFileSystem::new()).unwrap();
    LoadedSkins::of(vec![skin], generation)
}

/// [`loaded_sample`] with the same look loaded under a second id, the Frost, for two
/// sabers wearing different skins (not the Sun, whose sun extras want its own look).
pub(crate) fn loaded_pair(generation: u64) -> LoadedSkins {
    let skins = ["saber_tidal", "saber_frost"].map(|id| {
        let def = crate::blade_skin_file::parse(id, SAMPLE).unwrap();
        LoadedSkin::new(id, def, &VirtualFileSystem::new()).unwrap()
    });
    LoadedSkins::of(skins.into(), generation)
}

fn sample_color() -> SkinColor {
    loaded_sample(1).color_of("saber_tidal").unwrap()
}

#[test]
fn skins_have_their_materials_after_the_neutral_pair() {
    assert_eq!(SKIN_MATERIAL, RGB_MATERIAL + 1);
    assert_eq!(BLACK_MATERIAL, 7 + MAX_SKINS as u32);
    assert_eq!(MATERIAL_COUNT, 8 + MAX_SKINS);
    let skin = BladeColor::Skin(sample_color());
    assert_eq!(skin.material(), 7);
    let third = BladeColor::Skin(SkinColor {
        index: 2,
        ..sample_color()
    });
    assert_eq!(third.material(), 9);
    // Grouped into its own range, after every other material.
    let mut instances: Vec<Instance> = [
        skin,
        BladeColor::Retail(Color::Blue),
        BladeColor::from_rgb([10, 20, 30]),
        third,
        skin,
        BladeColor::Retail(Color::Red),
    ]
    .into_iter()
    .flat_map(|color| Instance::pair(blade(), color))
    .collect();
    let ranges = saber::material_ranges(&mut instances);
    assert_eq!(ranges[7], 6..10);
    assert_eq!(ranges[9], 10..12);
    assert_eq!(ranges[6], 4..6);
    assert_eq!(ranges[0], 0..2);
    assert_eq!(ranges[4], 2..4);
    assert!(instances[6..10].iter().all(|i| i.material() == 7));
}

#[test]
fn retail_and_rgb_colours_are_unchanged() {
    for color in Color::ALL {
        let retail = BladeColor::Retail(color);
        assert_eq!(retail.material(), color.index() as u32);
        assert_eq!(retail.tint(), [1.0; 3]);
        assert_eq!(retail.trail_rgb(), color.trail_rgb());
        assert_eq!(BladeColor::from_rgb(color.blade_rgb()), retail);
        assert_eq!(
            BladeColor::from_material(retail.material(), [1.0; 3]),
            retail
        );
    }
    let rgb = BladeColor::from_rgb([10, 200, 30]);
    assert_eq!(rgb.material(), RGB_MATERIAL);
    assert_eq!(rgb.trail_rgb(), [10.0 / 255.0, 200.0 / 255.0, 30.0 / 255.0]);
    assert_eq!(BladeColor::from_material(RGB_MATERIAL, rgb.tint()), rgb);
    assert_eq!(
        lights::rgb(BladeColor::Retail(Color::Blue)),
        [0.2, 0.4, 1.0]
    );
    assert_eq!(lights::rgb(rgb), rgb.tint());
}

#[test]
fn the_neutral_pair_is_generated_byte_for_byte_as_before() {
    // The neutral pair's formulas as they were written before the profiles.
    let half = 64.0_f32;
    let mut glow = Vec::new();
    for y in 0..128 {
        for x in 0..128 {
            let dx = (x as f32 + 0.5 - half) / half;
            let dy = (y as f32 + 0.5 - half) / half;
            let distance = (dx * dx + dy * dy).sqrt();
            let falloff = (-(distance / 0.5).powi(2)).exp() * 0.55;
            let edge = (1.0 - distance).clamp(0.0, 0.1) * 10.0;
            let value = (falloff * edge * 255.0).round() as u8;
            glow.extend_from_slice(&[value, value, value, 255]);
        }
    }
    assert!(crate::saber_rgb::neutral_glow() == glow);
    let mut core = Vec::new();
    for y in 0..256 {
        let from_end = (y as f32 + 0.5).min(256.0 - y as f32 - 0.5);
        let dy = (1.0 - from_end / 16.0).max(0.0);
        for x in 0..64 {
            let dx = (x as f32 + 0.5 - 32.0) / 32.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let channel = |value: f32| (value * 255.0).round() as u8;
            core.extend_from_slice(&[
                channel((-(distance / 0.41).powi(2)).exp()),
                channel((-(distance / 0.7).powi(2)).exp()),
                0,
                255,
            ]);
        }
    }
    assert!(crate::saber_rgb::neutral_core() == core);
}

#[test]
fn a_skin_trails_and_lights_as_its_file_says_with_a_gentle_flicker() {
    let skin = sample_color();
    let color = BladeColor::Skin(skin);
    assert_eq!(color.trail_rgb(), [0.2, 0.8, 0.9]);
    assert_eq!(lights::rgb(color), [0.2, 0.7, 0.9]);
    let levels: Vec<f32> = (0..400).map(|step| skin.flicker.at(step * 25, 3)).collect();
    let (low, high) = levels
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    // amount 0.1, one wave of weight 0.5: 0.9 ± 0.05.
    assert!(
        (0.84..0.87).contains(&low) && (0.93..=0.96).contains(&high),
        "{low} {high}"
    );
    // Slow: no frame-to-frame strobe.
    assert!(
        levels
            .windows(2)
            .all(|pair| (pair[1] - pair[0]).abs() < 0.05)
    );
    // Seeds do not pulse together; no flicker holds steady.
    assert_ne!(skin.flicker.at(1_000, 3), skin.flicker.at(1_000, 4));
    assert_eq!(LightFlicker::default().at(1_000, 3), 1.0);
    // The light itself: the skin's colour, at its flicker's level.
    let mut list = crate::dynamic_lights::PointLightList::default();
    let mut lit = [None; 8];
    lit[0] = Some(blade());
    lights::append(&mut list, &lit, color, 1, false, 1_000, 3, 3, 0.0);
    let gain = 1.8 * skin.flicker.at(1_000, 3);
    let expected = lights::rgb(color).map(|c| c * gain);
    assert!(list.as_slice().iter().any(|light| light.color == expected));
}

#[test]
fn a_skinned_blade_keeps_its_skin_through_contacts_and_reaches_further() {
    let skin = sample_color();
    let pair = Instance::pair(blade(), BladeColor::Skin(skin))
        .map(|i| i.with_contact(3, 0, 0, true, false));
    let (_, _, color, _) = pair[0].contact().expect("a contact source");
    let BladeColor::Skin(back) = color else {
        panic!("{color:?}");
    };
    assert_eq!(back.index, skin.index);
    // A contact lights with the skin's light.
    assert_eq!(lights::rgb(color), skin.light);
    // Wide enough that the capsule, not the hilt sprite, sets the reach.
    let wide = Blade {
        radius: 8.0,
        ..blade()
    };
    let reach = |color| Instance::pair(wide, color).map(|instance| instance.extent().2);
    let (skinned, stock) = (
        reach(BladeColor::Skin(skin)),
        reach(BladeColor::from_rgb([10, 20, 30])),
    );
    assert!(skinned[0] >= stock[0] * MAX_GLOW_REACH * 0.99);
    assert_eq!(skinned[1], stock[1]);
}

#[test]
fn the_uniform_holds_the_files_parameters_where_the_shader_reads_them() {
    let skins = loaded_sample(1);
    let uniforms = skins.uniforms();
    let one = uniforms[0];
    assert_eq!(one.core_white, [0.9, 1.0, 1.0, 0.5]);
    assert_eq!(one.core_fringe_cool, [0.1, 0.5, 0.6, 0.25]);
    assert_eq!(one.core_fringe_hot, [0.3, 0.9, 0.8, 0.5]);
    // w: the rounded tip, the default when the file does not say.
    assert_eq!(
        one.core_fringe,
        [0.8, 0.4, 0.9, crate::blade_skin_file::DEFAULT_TIP]
    );
    assert_eq!(one.core_breathe, [0.05, 8.0, 0.5, 3.0]);
    assert_eq!(one.rim_cool, [0.0, 0.2, 0.4, 0.7]);
    assert_eq!(one.rim_hot, [0.1, 0.5, 0.7, 1.0]);
    assert_eq!(one.inner, [0.4, 0.9, 0.9, 0.5]);
    assert_eq!(one.inner_mix, [0.5, 0.4, 0.2, 0.0]);
    assert_eq!(one.brightness, [0.6, 0.5, 1.0, 0.0]);
    assert_eq!(one.swell, [1.5, 0.1, 0.3, 0.0]);
    assert_eq!(one.grain_coarse, [0.3, 1.5, 0.4, 50.0]);
    assert_eq!(one.grain_fine, [0.9, 3.0, 0.8, 90.0]);
    assert_eq!(one.grain_mix, [0.7, 0.3, 0.2, 0.8]);
    assert_eq!(one.flare_rate, [0.5, 0.1, 0.05, 2.0]);
    assert_eq!(one.flare_shape, [5.0, 0.4, 0.5, 4.0]);
    assert_eq!(one.flare_size, [3.0, 1.0, 0.0, 0.0]);
    assert_eq!(one.shimmer_a, [0.04, 8.0, 0.6, 2.0]);
    assert_eq!(one.shimmer_b, [0.0; 4], "no second wave");
    assert_eq!(one.tongue_a, [0.4, 11.0, 2.0, 3.0]);
    assert_eq!(one.tongue_b, [0.3, 0.8, 0.4, 1.2]);
    // No arcs, motes or hue in the file: zeros, which the shader draws as nothing, so a
    // skin written before them (the Sun's) draws as it did.
    for (lane, name) in [
        (one.arc_color, "arc_color"),
        (one.arc_shape, "arc_shape"),
        (one.arc_strike, "arc_strike"),
        (one.arc_place, "arc_place"),
        (one.arc_motion, "arc_motion"),
        (one.mote_color, "mote_color"),
        (one.mote_field, "mote_field"),
        (one.mote_motion, "mote_motion"),
        (one.mote_band, "mote_band"),
        (one.hue, "hue"),
        (one.sputter_edge, "sputter_edge"),
        (one.sputter_cut, "sputter_cut"),
        (one.glitch_a, "glitch_a"),
        (one.glitch_b, "glitch_b"),
        (one.scan_color, "scan_color"),
        (one.scan_lines, "scan_lines"),
        (one.scan_edge, "scan_edge"),
        (one.scan_jitter, "scan_jitter"),
        (one.pulse_a, "pulse_a"),
        (one.pulse_b, "pulse_b"),
        (one.ember_color, "ember_color"),
        (one.ember_field, "ember_field"),
        (one.ember_shape, "ember_shape"),
        (one.vein_color, "vein_color"),
        (one.vein_shape, "vein_shape"),
        (one.team_red, "team_red"),
        (one.team_blue, "team_blue"),
        (one.team_none, "team_none"),
        (one.ambient, "ambient"),
        (one.glyph_color, "glyph_color"),
        (one.glyph_shape, "glyph_shape"),
        (one.star_color, "star_color"),
        (one.star_shape, "star_shape"),
        (one.star_motion, "star_motion"),
        (one.star_orbit, "star_orbit"),
        (one.hole_color, "hole_color"),
        (one.hole_shape, "hole_shape"),
        (one.wisp_color, "wisp_color"),
        (one.wisp_shape, "wisp_shape"),
        (one.wisp_field, "wisp_field"),
        (one.haze, "haze"),
        (one.band_color, "band_color"),
        (one.band_shape, "band_shape"),
        (one.band_mix, "band_mix"),
        (one.segment_a, "segment_a"),
        (one.segment_b, "segment_b"),
        (one.film, "film"),
        (one.film_b, "film_b"),
        (one.blob_color, "blob_color"),
        (one.blob_shape, "blob_shape"),
        (one.blob_more, "blob_more"),
        (one.front_color, "front_color"),
        (one.front_shape, "front_shape"),
        (one.front_edge, "front_edge"),
        (one.sketch_color, "sketch_color"),
        (one.sketch_shape, "sketch_shape"),
        (one.hour_color, "hour_color"),
        (one.hour_shape, "hour_shape"),
        (one.serpent_color, "serpent_color"),
        (one.serpent_shape, "serpent_shape"),
        (one.strand_shape, "strand_shape"),
        (one.strand_hue, "strand_hue"),
        (one.fly_color, "fly_color"),
        (one.fly_shape, "fly_shape"),
        (one.fly_roam, "fly_roam"),
        (one.wing_color, "wing_color"),
        (one.wing_shape, "wing_shape"),
        (one.copy_shape, "copy_shape"),
        (one.copy_more, "copy_more"),
        (one.ink_color, "ink_color"),
        (one.ink_shape, "ink_shape"),
        (one.ink_more, "ink_more"),
        (one.chrome_color, "chrome_color"),
        (one.chrome_shape, "chrome_shape"),
    ] {
        assert_eq!(lane, [0.0; 4], "{name}");
    }
    // Past the loaded skins, zeros.
    assert!(uniforms[1..].iter().all(|u| *u == SkinUniform::default()));
    let bytes: &[u8] = bytemuck::cast_slice(&uniforms);
    assert_eq!(bytes.len(), MAX_SKINS * 105 * 16);
    // The array stays inside the storage binding WebGPU guarantees (128 MiB).
    assert!(bytes.len() <= 128 * 1024 * 1024);
}

#[test]
fn arcs_motes_and_hue_reach_the_uniform_where_the_shader_reads_them() {
    let text = crate::blade_skin_file::tests::sample_with_effects();
    let def = crate::blade_skin_file::parse("saber_tidal", &text).unwrap();
    let one = SkinUniform::of(&def);
    assert_eq!(one.core_fringe[3], 2.5, "core.tip");
    assert_eq!(one.arc_color, [0.7, 0.8, 1.0, 2.0]);
    assert_eq!(one.arc_shape, [0.15, 0.5, 0.4, 0.6]);
    assert_eq!(one.arc_strike, [3.0, 6.0, 0.3, 1.5]);
    assert_eq!(one.arc_place, [1.1, 0.2, 0.5, 0.25]);
    assert_eq!(one.arc_motion, [20.0, 12.0, 0.0, 0.0]);
    assert_eq!(one.mote_color, [0.9, 0.9, 1.0, 1.5]);
    assert_eq!(one.mote_field, [0.3, 0.8, 2.5, 0.2]);
    assert_eq!(one.mote_motion, [-2.0, 1.5, 4.0, 2.0]);
    assert_eq!(one.mote_band, [0.4, 1.7, 0.6, 0.0]);
    assert_eq!(one.hue, [0.1, 0.02, 0.2, 0.0]);
    // The light turns with the blade's middle; without a hue it holds.
    let skin = LoadedSkin::new("saber_tidal", def, &VirtualFileSystem::new()).unwrap();
    let color = skin.color(0, 0);
    assert_eq!(
        color.hue,
        [0.1, 0.02 * 20.0],
        "the turn at the middle of 40 units"
    );
    assert_ne!(color.light_at(0), color.light_at(2_500));
    assert_eq!(color.light_at(0), turn_hue(color.light, color.hue[1]));
    let plain = sample_color();
    assert_eq!(plain.hue, [0.0; 2]);
    assert_eq!(plain.light_at(1_234), plain.light);
}

#[test]
fn turning_the_hue_keeps_grey_and_goes_round_the_colours() {
    let close = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
    // Grey stays grey; no turn is no change; a whole turn comes back.
    assert!(close(turn_hue([0.4; 3], 0.37), [0.4; 3]));
    assert_eq!(turn_hue([0.9, 0.3, 0.1], 0.0), [0.9, 0.3, 0.1]);
    assert!(close(turn_hue([0.9, 0.3, 0.1], 1.0), [0.9, 0.3, 0.1]));
    // A third of a turn takes red to green and green to blue.
    assert!(close(turn_hue([1.0, 0.0, 0.0], 1.0 / 3.0), [0.0, 1.0, 0.0]));
    assert!(close(turn_hue([0.0, 1.0, 0.0], 1.0 / 3.0), [0.0, 0.0, 1.0]));
    // Never a negative channel (the blend adds).
    for step in 0..24 {
        let turned = turn_hue([1.0, 0.05, 0.1], step as f32 / 24.0);
        assert!(turned.iter().all(|c| *c >= 0.0), "{turned:?}");
    }
    // The shader turns it the same way.
    let shader = include_str!("saber.wgsl");
    assert!(shader.contains(
        "let turned = color * c + cross(axis, color) * s + axis * dot(axis, color) * (1.0 - c);"
    ));
}

/// `saber.wgsl`'s `skin_tip_taper`, mirrored: how much of the core line's width is left
/// `along` units from the hilt with a rounded tip `cap` units long.
fn tip_taper(along: f32, length: f32, cap: f32) -> f32 {
    let into = ((along - (length - cap)) / cap.max(0.0001)).clamp(0.0, 1.0);
    if into <= 0.0 {
        return 1.0;
    }
    (1.0 - into * into).max(0.0001).sqrt()
}

/// `saber.wgsl`'s `skin_glow` past the tip, mirrored: the distance out from the blade (the
/// tip past it) of a point `x` across and `y` along the projected blade, `shaft` long, and
/// how far round the tip it lies (`around` less `along`) for a capsule of radius `r`.
fn glow_out(x: f32, y: f32, shaft: f32, r: f32) -> (f32, f32) {
    let beyond = (y - shaft).max(0.0);
    let radial = if beyond > 0.0 {
        x.hypot(beyond)
    } else {
        x.abs()
    };
    let around = if beyond > 0.0 {
        r * beyond.atan2(x.abs().max(0.0001))
    } else {
        0.0
    };
    (radial, around)
}

#[test]
fn a_skinned_blade_ends_round_not_square() {
    let (length, radius) = (40.0, 1.0);
    let cap = crate::blade_skin_file::DEFAULT_TIP * radius;
    // The core line keeps its whole width along the shaft (the look there is unchanged)...
    for along in [-1.0, 0.0, 10.0, 30.0, length - cap] {
        assert_eq!(tip_taper(along, length, cap), 1.0, "{along}");
    }
    // ...and narrows on a quarter circle to a point at the tip.
    for into in [0.2_f32, 0.6, 0.8, 0.95] {
        let along = length - cap + into * cap;
        let left = tip_taper(along, length, cap);
        assert!(
            (left * left + into * into - 1.0).abs() < 1e-4,
            "{into}: {left}"
        );
    }
    assert!(tip_taper(length, length, cap) <= 0.01);
    // The quad's corners at the tip (across ±half the line) map past the texture's edge,
    // so they are cut: no square corner. Across is `x / 2r`, the texture edge at 0.5.
    for (x, along) in [(radius, length), (0.9 * radius, length - 0.1 * cap)] {
        let across = x / (2.0 * radius) / tip_taper(along, length, cap);
        assert!(across > 0.5, "{x} at {along}: {across}");
    }
    // On the tip's last row the cut's softening is the straight line's pixel, stretched as
    // the line is: everything but the centre stays dark. (The derivatives of the stretched
    // coordinate ran to a hundred there, and the cut left half the width lit: a bar.)
    let pixel = 0.4 / (2.0 * radius);
    let left = tip_taper(length, length, cap);
    for x in [0.25 * radius, 0.5 * radius, radius] {
        let across = x / (2.0 * radius) / left;
        let footprint = pixel / left;
        let cut = ((0.5 - across) / footprint + 0.5).clamp(0.0, 1.0);
        assert_eq!(cut, 0.0, "{x}");
    }
    // The tip's centre line is inside it.
    let across = 0.0 / tip_taper(length - 0.05 * cap, length, cap);
    assert!(across < 0.5);
    // The glow past the tip: its distance out is the same in every direction from the
    // tip (round), where it used to be the distance across only (a straight band).
    let shaft = 40.0;
    let r = 3.0;
    for distance in [0.5_f32, 1.5, 3.0] {
        let outs: Vec<f32> = (0..=8)
            .map(|step| {
                let angle = std::f32::consts::FRAC_PI_2 * step as f32 / 8.0;
                let (x, y) = (distance * angle.cos(), shaft + distance * angle.sin());
                glow_out(x, y, shaft, r).0
            })
            .collect();
        assert!(
            outs.iter().all(|out| (out - distance).abs() < 1e-4),
            "{outs:?}"
        );
    }
    // Along the shaft, the distance across and the place along exactly as before.
    for x in [-2.5_f32, 0.0, 0.7, 4.0] {
        assert_eq!(glow_out(x, 12.0, shaft, r), (x.abs(), 0.0));
    }
    // Round the tip, the tongues' place runs on from the side to the top, continuously.
    let (_, side) = glow_out(2.0, shaft + 0.0001, shaft, r);
    let (_, top) = glow_out(0.0, shaft + 2.0, shaft, r);
    assert!(side < 0.001 && (top - r * std::f32::consts::FRAC_PI_2).abs() < 1e-3);
    // The shader does what is mirrored here.
    let shader = include_str!("saber.wgsl");
    for line in [
        "let into = clamp((along - (length - cap)) / max(cap, 0.0001), 0.0, 1.0);",
        "return select(sqrt(max(1.0 - into * into, 0.0001)), 1.0, into <= 0.0);",
        "/ skin_tip_taper(along, input.length, skin.core_fringe.w * input.radius);",
        "return core_coordinates_at(input, line, line.stretch);",
        "let across = input.blade.x / (2.0 * input.radius) * stretch - line.moved;",
        "let footprint = pixel / (2.0 * input.radius) * line.stretch;",
        "let beyond = max(input.blade.y - input.shaft, 0.0);",
        "shaded.blade = vec2(x / widen, min(input.blade.y, input.shaft) + beyond / widen);",
        "let radial = select(abs(x), length(vec2(x, beyond)), beyond > 0.0);",
        "let out = radial / (r * widen);",
        "let around = select(along, along + r * atan2(beyond, max(abs(x), 0.0001)), beyond > 0.0);",
        "core *= clamp((0.5 - abs(uv.x - 0.5)) / max(footprint, 0.00001) + 0.5, 0.0, 1.0);",
    ] {
        assert!(shader.contains(line), "saber.wgsl lost {line:?}");
    }
}

#[test]
fn a_skinned_blades_tip_is_filtered_as_its_shaft_is() {
    // Round 23 (players' reports after 2026.1021.1): the core is sampled with the
    // derivatives of its coordinates before the rounded tip narrows them, else the tip's
    // narrowing picks the core image's smallest mip and draws a bright rim round the tip.
    // Along the shaft the two coordinates are the same, so the shaft looks as before.
    let (length, radius) = (40.0, 1.0);
    let cap = crate::blade_skin_file::DEFAULT_TIP * radius;
    for along in [-1.0, 0.0, 20.0, length - cap] {
        assert_eq!(tip_taper(along, length, cap), 1.0, "{along}");
    }
    let shader = include_str!("saber.wgsl");
    for line in [
        // One skin lookup (`core_line`) gives both coordinates, not one lookup each.
        "let line = core_line(input);",
        "let core_uv = core_coordinates_at(input, line, line.stretch);",
        "let filter_uv = core_coordinates_at(input, line, line.breath);",
        "let core_dx = dpdx(filter_uv);",
        "let core_dy = dpdy(filter_uv);",
        "let stretch = breath\n        / skin_tip_taper(along, input.length, skin.core_fringe.w * input.radius);",
        // A signature's core fades out before its quad's sides.
        "core *= 1.0 - smoothstep(0.36, 0.5, abs(uv.x - 0.5));",
    ] {
        assert!(shader.contains(line), "saber.wgsl lost {line:?}");
    }
    // No second skin lookup per pixel for the filter alone (it ran on every skinned blade).
    for gone in [
        "dpdx(core_uv)",
        "dpdy(core_uv)",
        "fn core_breath(",
        "fn core_stretch(",
    ] {
        assert!(!shader.contains(gone), "saber.wgsl has {gone:?} again");
    }
}

#[test]
fn skins_load_from_the_packs_files_with_their_images_and_sounds() {
    // A grey 32×32 glow image in the pack, the core generated from the profile.
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(32, 32, image::Rgba([90, 90, 90, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let with_image = sample_with(
        r#""core_profile": {"width": 0.3, "fringe_width": 0.6},"#,
        r#""core_profile": {"width": 0.3, "fringe_width": 0.6}, "glow_image": "gfx/test/glow.png","#,
    );
    let mut packs = VirtualFileSystem::new();
    packs
        .mount_memory(
            "pack",
            [
                (
                    "skins/blades/saber_tidal.bladeskin",
                    with_image.into_bytes(),
                ),
                ("gfx/test/glow.png", png),
                // Not a blade skin this client knows, and a broken one: left out.
                (
                    "skins/blades/saber_starfall.bladeskin",
                    SAMPLE.as_bytes().to_vec(),
                ),
                ("skins/blades/saber_bad.bladeskin", b"{".to_vec()),
            ],
        )
        .unwrap();
    let skins = LoadedSkins::load(packs, 4);
    assert_eq!(skins.generation(), 4);
    // The pack's skin, then the signature skins no pack carries, from their built-in files.
    let ids: Vec<_> = skins.ids().collect();
    assert_eq!(ids[0], "saber_tidal");
    assert!(
        ids[1..]
            .iter()
            .copied()
            .eq(crate::blade_signatures::SIGNATURES
                .iter()
                .map(|signature| signature.id))
    );
    let skin = skins.get("saber_tidal").unwrap();
    assert_eq!(skin.glow.dimensions(), (32, 32));
    assert_eq!(skin.glow.get_pixel(3, 3).0, [90, 90, 90, 255]);
    assert_eq!(skin.core.dimensions(), (64, 256));
    assert_eq!(skins.color_of("saber_tidal").map(|c| c.index), Some(0));
    assert_eq!(skins.color_of("saber_starfall"), None);
    let sets = skins.sound_sets();
    assert_eq!(sets.len(), 1 + crate::blade_signatures::SIGNATURES.len());
    assert_eq!(sets[0].hum, "sound/test/blade/hum.wav");
    assert_eq!(sets[0].swings[1], "sound/test/blade/s2.wav");
    assert_eq!(skins.swing(0, 4), Some("sound/test/blade/s2.wav"));
    assert_eq!(
        skins.swing(sets.len() as crate::saber_skins::SkinIndex, 0),
        None
    );
    // A missing image refuses the skin: the stock blade.
    let mut missing = VirtualFileSystem::new();
    missing
        .mount_memory(
            "pack",
            [(
                "skins/blades/saber_tidal.bladeskin",
                sample_with(
                    r#""core_profile": {"width": 0.3, "fringe_width": 0.6},"#,
                    r#""core_profile": {"width": 0.3, "fringe_width": 0.6}, "core_image": "gfx/none.png","#,
                )
                .into_bytes(),
            )],
        )
        .unwrap();
    assert_eq!(
        LoadedSkins::load(missing, 5)
            .get("saber_tidal")
            .map(|skin| skin.id),
        None
    );
}

#[test]
fn the_table_gives_each_players_body_its_skin() {
    let skin = sample_color();
    let mut skins = SaberSkins::default();
    assert_eq!(skins.get(1), None);
    skins.set(4, Some(skin));
    // Entity ids are client slots plus one.
    assert_eq!(skins.get(5), Some(skin));
    assert_eq!(skins.get(4), None);
    assert_eq!(skins.get(0), None);
    assert_eq!(skins.get(33), None);
    assert_eq!(skins.get(1_000), None);
    skins.set(32, Some(skin));
    assert!(
        skins
            .sound_sets()
            .iter()
            .filter(|set| set.is_some())
            .count()
            == 1
    );
    let red = BladeColor::Retail(Color::Red);
    assert_eq!(skins.blade_color(5, red), BladeColor::Skin(skin));
    assert_eq!(skins.blade_color(6, red), red);
    skins.set(4, None);
    assert_eq!(skins.get(5), None);
    assert_eq!(skins.sound_sets(), [None; MAX_CLIENTS]);
}

#[test]
fn the_local_choice_follows_the_players_slot() {
    let skin = sample_color();
    let mut skins = SaberSkins::default();
    // In the menus: no slot, the preview still wears it.
    skins.set_local(None, Some(skin));
    assert_eq!(skins.local(), Some(skin));
    assert_eq!(skins.sound_sets(), [None; MAX_CLIENTS]);
    skins.set_local(Some(2), Some(skin));
    assert_eq!(skins.get(3), Some(skin));
    assert_eq!(skins.sound_sets()[2], Some(0));
    // A new slot (another server) clears the old one.
    skins.set_local(Some(7), Some(skin));
    assert_eq!(skins.get(3), None);
    assert_eq!(skins.get(8), Some(skin));
    skins.set_local(Some(7), None);
    assert_eq!(skins.get(8), None);
    assert_eq!(skins.local(), None);
}

#[test]
fn a_skin_whose_pack_is_not_loaded_is_the_stock_blade() {
    let none = LoadedSkins::default();
    assert_eq!(none.generation(), 0);
    assert_eq!(none.color_of("saber_tidal"), None);
    assert!(none.sound_sets().is_empty());
    assert_eq!(none.uniforms(), vec![SkinUniform::default(); MAX_SKINS]);
    let mut skins = SaberSkins::default();
    skins.follow_looks(1, none.generation(), |_| none.color_of("saber_tidal"));
    assert_eq!(skins.get(1), None);
}

#[test]
fn other_players_skins_follow_the_looks_and_the_packs_only_when_they_change() {
    use std::cell::Cell;
    let skin = sample_color();
    let mut skins = SaberSkins::default();
    skins.set_local(Some(2), Some(skin));
    let asked = Cell::new(0);
    let looks = |client: usize| {
        asked.set(asked.get() + 1);
        // The hub says slot 2 (the local player's own) wears nothing: its own wins.
        (client == 5).then_some(skin)
    };
    skins.follow_looks(7, 1, looks);
    assert_eq!(asked.get(), MAX_CLIENTS);
    assert_eq!(skins.get(6), Some(skin));
    assert_eq!(skins.get(3), Some(skin), "the local player's own");
    let sets = skins.sound_sets();
    assert_eq!(sets[5], Some(0));
    assert_eq!(sets[2], Some(0));
    assert_eq!(sets.iter().filter(|set| set.is_some()).count(), 2);
    // The same revision and generation are not read again.
    skins.follow_looks(7, 1, looks);
    assert_eq!(asked.get(), MAX_CLIENTS);
    // New skins from a pack are, under the same looks.
    skins.follow_looks(7, 2, looks);
    assert_eq!(asked.get(), 2 * MAX_CLIENTS);
    // A new revision is; the slot that took its look off goes stock, sounds too.
    skins.follow_looks(8, 2, |_| None);
    assert_eq!(skins.get(6), None);
    assert_eq!(skins.sound_sets()[5], None);
    assert_eq!(skins.get(3), Some(skin));
    // The local player leaves the game: its old slot shows the hub's look again.
    skins.set(2, Some(skin));
    skins.set_local(None, None);
    assert_eq!(skins.get(3), Some(skin));
    assert_eq!(skins.local(), None);
}

#[test]
fn the_local_skin_is_gated_by_the_own_profile_and_its_pack() {
    let loaded = loaded_sample(1);
    let mut looks = crate::looks::Looks::default();
    // `cg_saberSkin saber_tidal` without the unlock: the stock blade.
    let unowned = crate::looks::Worn::own("saber_tidal", "", |_| false, false);
    looks.set_own(None, unowned);
    assert_eq!(
        looks.own_saber_skin().and_then(|id| loaded.color_of(id)),
        None
    );
    let owned = crate::looks::Worn::own("saber_tidal", "", |id| id == "saber_tidal", false);
    looks.set_own(Some(4), owned);
    assert_eq!(
        looks.own_saber_skin().and_then(|id| loaded.color_of(id)),
        Some(sample_color())
    );
    // Owned, but no pack: the stock blade.
    assert_eq!(
        looks
            .own_saber_skin()
            .and_then(|id| LoadedSkins::default().color_of(id)),
        None
    );
    // The local slot in the looks wears it too, so following them agrees.
    let mut skins = SaberSkins::default();
    skins.follow_looks(looks.revision(), loaded.generation(), |client| {
        looks
            .saber_skin_id(client)
            .and_then(|id| loaded.color_of(id))
    });
    assert_eq!(skins.get(5), Some(sample_color()));
}

#[test]
fn the_cvar_is_registered_archived_and_empty() {
    let directory = tempfile::tempdir().unwrap();
    let console = crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
    assert_eq!(
        console.cvar(crate::unlockables::SABER_SKIN_CVAR),
        Some(&sjk_shell::CvarValue::Text(String::new()))
    );
}

#[test]
fn following_another_player_draws_their_look_not_the_own_skin() {
    use crate::looks::{Looks, ViewSlots, Worn};
    let loaded = loaded_sample(1);
    // The local player, in slot 2, wears the skin and follows slot 5 (`PMF_FOLLOW`),
    // whose hub look is the stock blade: the snapshot's player state is slot 5's.
    let game_state = sjk_protocol::GameState::empty_local(2);
    let mut followed = sjk_protocol::PlayerState::zero();
    followed.set_client_num(5);
    followed.set_movement_flags(0x1000);
    let mut looks = Looks::default();
    let stock = sjk_identity::Look {
        saber: String::new(),
        illuminate: true,
        saber_off: Vec::new(),
        toys: Vec::new(),
        saber2: None,
        saber2_off: Vec::new(),
        saber_on: Vec::new(),
        saber2_on: Vec::new(),
        saber_sounds: String::new(),
        tag: None,
        finisher: String::new(),
    };
    looks.apply_event(5, "Fox", &stock);
    let slots = ViewSlots::of(&game_state, &followed);
    looks.set_own(
        slots.own.and_then(|slot| u8::try_from(slot).ok()),
        Worn::own("saber_tidal", "", |_| true, false),
    );
    looks.rebuild(|slot| (slot == 5).then(|| "Fox".to_owned()));
    let mut skins = SaberSkins::default();
    skins.follow_looks(looks.revision(), loaded.generation(), |client| {
        looks
            .saber_skin_id(client)
            .and_then(|id| loaded.color_of(id))
    });
    skins.set_local(
        slots.own,
        looks.own_saber_skin().and_then(|id| loaded.color_of(id)),
    );
    assert_eq!(skins.get(6), None, "the followed player's own look");
    assert_eq!(skins.sound_sets()[5], None);
    assert_eq!(
        skins.get(3),
        Some(sample_color()),
        "the local player's slot"
    );
}

/// The test blade skin loaded as a chroma (the Storm's id, [`SAMPLE`]'s values).
fn chroma_sample() -> SkinColor {
    let def = crate::blade_skin_file::parse("saber_storm", SAMPLE).unwrap();
    let skin = LoadedSkin::new("saber_storm", def, &VirtualFileSystem::new()).unwrap();
    LoadedSkins::of(vec![skin], 1)
        .color_of("saber_storm")
        .unwrap()
}

fn close(a: f32, b: f32) -> bool {
    let d = (a - b).rem_euclid(1.0);
    // Turning clips channels that would go negative, which moves a hue a little.
    d.min(1.0 - d) < 0.02
}

#[test]
fn hues_run_round_the_grey_axis_as_turn_hue_turns() {
    assert!(close(hue_of([1.0, 0.0, 0.0]).unwrap(), 0.0));
    assert!(close(hue_of([0.0, 1.0, 0.0]).unwrap(), 1.0 / 3.0));
    assert!(close(hue_of([0.0, 0.0, 1.0]).unwrap(), 2.0 / 3.0));
    // Grey, white, black and almost grey colours have none; a pastel has its hue.
    assert_eq!(hue_of([0.5, 0.5, 0.5]), None);
    assert_eq!(hue_of([1.0, 1.0, 1.0]), None);
    assert_eq!(hue_of([0.0, 0.0, 0.0]), None);
    assert_eq!(hue_of([1.0, 0.99, 0.98]), None);
    assert!(hue_of([0.81, 0.725, 0.918]).is_some());
    // Turning a colour by the gap between two hues lands on the second.
    let from = [0.9, 0.3, 0.1];
    for target in [[0.1, 0.4, 1.0], [0.2, 1.0, 0.3], [1.0, 0.0, 0.8]] {
        let turn = hue_of(target).unwrap() - hue_of(from).unwrap();
        let turned = turn_hue(from, turn);
        assert!(close(hue_of(turned).unwrap(), hue_of(target).unwrap()));
        // Brightness (the grey axis component) is kept.
        let sum = |c: [f32; 3]| c[0] + c[1] + c[2];
        assert!((sum(turned) - sum(from)).abs() < 0.05);
    }
}

#[test]
fn only_chromas_take_the_wearers_colour() {
    let sun = sample_color();
    assert_eq!(sun.chroma, Chroma::default());
    assert_eq!(sun.worn_with(BladeColor::Retail(Color::Green)), sun);
    let storm = chroma_sample();
    let base = storm.chroma.base.expect("the Storm is a chroma");
    assert_eq!(storm.chroma.turn, 0.0);
    for color in Color::ALL {
        let worn = storm.worn_with(BladeColor::Retail(color));
        let rgb = color.blade_rgb().map(|c| f32::from(c) / 255.0);
        assert!(
            close(base + worn.chroma.turn, hue_of(rgb).unwrap()),
            "{color:?}"
        );
        // Its light and trail turn with it.
        if let Some(light) = hue_of(storm.light) {
            let turned = hue_of(worn.light_color()).unwrap();
            assert!(close(turned, light + worn.chroma.turn), "{color:?}");
        }
        assert_eq!(worn.trail_color(), turn_hue(storm.trail, worn.chroma.turn));
        assert_eq!(BladeColor::Skin(worn).trail_rgb(), worn.trail_color());
        // Wearing it again changes nothing.
        assert_eq!(worn.worn_with(BladeColor::Retail(color)), worn);
    }
    // A custom colour too.
    let custom = storm.worn_with(BladeColor::Rgb([255, 0, 200]));
    assert!(close(
        base + custom.chroma.turn,
        hue_of([1.0, 0.0, 200.0 / 255.0]).unwrap()
    ));
    // White has no hue to turn to: it greys the blade instead (Dante's idea).
    let white = storm.worn_with(BladeColor::Rgb([255, 255, 255]));
    assert_eq!(white.chroma.turn, 0.0);
    assert_eq!(white.chroma.grey, 1.0);
}

fn grey_enough(rgb: [f32; 3]) -> bool {
    let high = rgb[0].max(rgb[1]).max(rgb[2]);
    let low = rgb[0].min(rgb[1]).min(rgb[2]);
    high - low < 1e-4
}

#[test]
fn a_chroma_worn_white_or_grey_is_drawn_grey_at_its_own_brightness() {
    let storm = chroma_sample();
    assert!(storm.chroma.saturation > 0.0);
    for rgb in [[255, 255, 255], [128, 128, 128]] {
        let worn = storm.worn_with(BladeColor::Rgb(rgb));
        assert_eq!(worn.chroma.grey, 1.0, "{rgb:?}");
        let light = worn.light_color();
        assert!(grey_enough(light), "{rgb:?}: {light:?}");
        let mean = |c: [f32; 3]| (c[0] + c[1] + c[2]) / 3.0;
        assert!((mean(light) - mean(storm.light)).abs() < 1e-4);
        assert!(grey_enough(worn.trail_color()));
        // The shader gets it packed: no turn, the whole greying.
        assert_eq!(worn.chroma.packed(), 255.0);
        assert!(grey_enough(wear_chroma(
            [0.2, 0.5, 1.0],
            worn.chroma.packed()
        )));
        // Wearing it again changes nothing.
        assert_eq!(worn.worn_with(BladeColor::Rgb(rgb)), worn);
    }
}

#[test]
fn stock_and_strong_custom_colours_keep_a_chromas_saturation() {
    let storm = chroma_sample();
    for color in Color::ALL {
        assert_eq!(storm.worn_with(BladeColor::Retail(color)).chroma.grey, 0.0);
    }
    for rgb in [[255, 0, 0], [255, 51, 51], [40, 255, 60], [0, 0, 255]] {
        let worn = storm.worn_with(BladeColor::Rgb(rgb));
        assert_eq!(worn.chroma.grey, 0.0, "{rgb:?}");
        // Packed with no greying, the turn is the shader's as it was.
        assert_eq!(worn.chroma.packed(), worn.chroma.turn);
        assert_eq!(worn.trail_color(), turn_hue(storm.trail, worn.chroma.turn));
    }
}

#[test]
fn a_pastel_custom_colour_makes_a_chroma_paler_in_step() {
    let storm = chroma_sample();
    let pastel = storm.worn_with(BladeColor::Rgb([207, 190, 234]));
    let paler = storm.worn_with(BladeColor::Rgb([230, 222, 244]));
    assert!(pastel.chroma.grey > 0.0 && pastel.chroma.grey < 1.0);
    assert!(paler.chroma.grey > pastel.chroma.grey);
    // Its hue is still the pastel's.
    let base = storm.chroma.base.unwrap();
    assert!(close(
        base + pastel.chroma.turn,
        hue_of([207.0 / 255.0, 190.0 / 255.0, 234.0 / 255.0]).unwrap()
    ));
    let sat = |c: [f32; 3]| saturation_of(c);
    assert!(sat(paler.trail_color()) < sat(pastel.trail_color()));
    assert!(sat(pastel.trail_color()) < sat(turn_hue(storm.trail, pastel.chroma.turn)));
    // Worn pastel, then strong again: the effects get their saturation back.
    let strong = pastel.worn_with(BladeColor::Rgb([255, 0, 200]));
    let direct = storm.worn_with(BladeColor::Rgb([255, 0, 200]));
    assert_eq!(strong.chroma.grey, 0.0);
    assert!(close(strong.chroma.turn, direct.chroma.turn));
}

#[test]
fn packed_chromas_unpack_to_their_turn_and_greying() {
    assert_eq!(pack_chroma(0.0, 0.0), 0.0);
    assert_eq!(wear_chroma([0.9, 0.3, 0.1], 0.0), [0.9, 0.3, 0.1]);
    let packed = pack_chroma(0.25, 0.5);
    assert!((packed.fract() - 0.25).abs() < 1e-4);
    assert_eq!(packed.floor(), 128.0);
    // A turn of a whole round is none.
    assert_eq!(pack_chroma(1.0, 0.0), 0.0);
    assert_eq!(pack_chroma(-0.25, 0.0), 0.75);
    let rgb = [1.0, 0.0, 0.0];
    let worn = wear_chroma(rgb, pack_chroma(1.0 / 3.0, 0.5));
    let expected = greyed(turn_hue(rgb, 1.0 / 3.0), 128.0 / 255.0);
    for i in 0..3 {
        assert!(
            (worn[i] - expected[i]).abs() < 1e-3,
            "{worn:?} {expected:?}"
        );
    }
    // Greying keeps the brightness (the mean).
    let grey = greyed([0.9, 0.3, 0.0], 1.0);
    assert!(grey.iter().all(|c| (c - 0.4).abs() < 1e-6));
}

#[test]
fn each_saber_of_a_pair_takes_its_own_colour() {
    let storm = chroma_sample();
    let first = storm.worn_with(BladeColor::Retail(Color::Red));
    let second = storm.worn_with(BladeColor::Retail(Color::Blue));
    assert!(!close(first.chroma.turn, second.chroma.turn));
    // The instances carry the turn the shader applies.
    let [glow, core] = Instance::pair(blade(), BladeColor::Skin(second));
    for instance in [glow, core] {
        assert_eq!(
            bytemuck::bytes_of(&instance)[76..80],
            second.chroma.packed().to_ne_bytes()
        );
    }
}

#[test]
fn black_holes_reach_the_uniform_and_take_the_glints_places() {
    use crate::blade_skin_file::extras::{Section, option_bit};
    let text = crate::blade_skin_file::extras::merged(
        &crate::blade_skin_file::tests::sample_with_additions(),
        crate::blade_skin_file::tests::HOLES,
    )
    .unwrap();
    let def = crate::blade_skin_file::parse("saber_void", &text).unwrap();
    let one = SkinUniform::of(&def);
    assert_eq!(one.hole_color, [0.8, 0.6, 1.0, 2.0]);
    assert_eq!(one.hole_shape[..3], [1.2, 1.0, 0.5]);
    assert!((one.hole_shape[3] - 5.4).abs() < 1e-6, "the quad's reach");
    assert_eq!(one.star_color, [0.0; 4], "no glint");
    assert_eq!(one.star_motion[3], 2.0, "two holes");
    assert_eq!(one.star_orbit, [12.0, 5.0, 0.3, 0.25]);
    let skin = LoadedSkin::new("saber_void", def, &VirtualFileSystem::new()).unwrap();
    let holes = skin.color(0, 0).star.unwrap();
    assert_eq!((holes.at, holes.count), (12.0, 2));
    assert!((holes.length - 5.4).abs() < 1e-6);
    // Switched off, they are not made at all.
    let off = skin.color(0, option_bit("holes"));
    assert_eq!(off.star, None);
    assert_eq!(off.off, Section::Holes.bit());
}

#[test]
fn the_orbit_moves_the_glints_below_the_tip_for_the_cpu_and_the_gpu() {
    let text = crate::blade_skin_file::extras::merged(
        &crate::blade_skin_file::tests::sample_with_additions(),
        crate::blade_skin_file::tests::LATER_ADDITIONS,
    )
    .unwrap();
    let def = crate::blade_skin_file::parse("saber_tidal", &text).unwrap();
    let one = SkinUniform::of(&def);
    assert_eq!(one.star_motion[3], 2.0, "two glints");
    assert_eq!(one.star_orbit, [10.0, 4.5, 0.4, 0.3]);
    let skin = LoadedSkin::new("saber_tidal", def, &VirtualFileSystem::new()).unwrap();
    let glint = skin.color(0, 0).star.unwrap();
    assert_eq!((glint.length, glint.at, glint.count), (8.0, 10.0, 2));
    // The circle's reach, its tilt included.
    assert!((glint.reach - 4.5 / 0.3_f32.cos()).abs() < 1e-5);
    // Without an orbit: one glint at the tip, and the orbit's lanes zeros.
    let plain = crate::blade_skin_file::tests::sample_with_additions();
    let one = SkinUniform::of(&crate::blade_skin_file::parse("saber_tidal", &plain).unwrap());
    assert_eq!((one.star_motion[3], one.star_orbit), (0.0, [0.0; 4]));
}

#[test]
fn a_wearers_options_leave_out_their_parts_on_the_cpu_and_the_gpu() {
    use crate::blade_skin_file::extras::{Section, option_bit};
    let text = crate::blade_skin_file::tests::sample_with_additions().replacen(
        r#""trail""#,
        r#""ghosts": {"count": 2, "spacing": 40.0, "fade": 0.5}, "trail""#,
        1,
    );
    let def = crate::blade_skin_file::parse("saber_tidal", &text).unwrap();
    let skin = LoadedSkin::new("saber_tidal", def, &VirtualFileSystem::new()).unwrap();
    let all = skin.color(0, 0);
    assert_eq!(all.off, 0);
    let glint = all.star.unwrap();
    assert_eq!((glint.length, glint.at, glint.count), (8.0, 0.0, 1));
    assert!(all.echo.is_some() && all.ghosts.is_some());
    assert_eq!(all.room, 9.0);
    assert_ne!(all.hue, [0.0; 2]);
    let none = skin.color(
        0,
        option_bit("glint") | option_bit("haze") | option_bit("smoke") | option_bit("storm"),
    );
    assert_eq!(
        none.off,
        Section::Star.bit()
            | Section::Haze.bit()
            | Section::Wisps.bit()
            | Section::Echo.bit()
            | Section::Arcs.bit()
            | Section::Motes.bit()
    );
    assert_eq!((none.star, none.echo, none.room), (None, None, 0.0));
    // The parts no option names stay.
    assert!(none.ghosts.is_some());
    assert_ne!(none.hue, [0.0; 2]);
    // The instances carry the mask; the glint and the echo are made only when on.
    let blade = blade();
    let pair = crate::saber::Instance::pair(blade, BladeColor::Skin(none));
    assert_eq!(
        crate::saber::skin_extras(blade, BladeColor::Skin(none), 1.0, 3).count(),
        0
    );
    let extras: Vec<_> = crate::saber::skin_extras(blade, BladeColor::Skin(all), 1.0, 3).collect();
    assert_eq!(extras.len(), 2);
    assert!(extras[0].is_star() && !extras[1].is_star());
    assert!(pair.iter().all(|instance| instance.contact().is_none()));
}

#[test]
fn a_custom_tint_never_jumps_hue_between_neighbouring_values() {
    // Kushina's report: (207, 185, 234) drew violet, (207, 190, 234) cyan, because
    // pastels under a saturation cliff kept the skin's own hue. Step every channel by
    // 5 across the cube: the hue the tint gives a chroma moves no more than the
    // input's own, never by a jump, and any non-grey tint has one.
    let hue = |rgb: [u8; 3]| hue_of(rgb.map(|c| f32::from(c) / 255.0));
    let gap = |a: f32, b: f32| (a - b).abs().min(1.0 - (a - b).abs());
    for red in (0..=250u8).step_by(5) {
        for green in (0..=250u8).step_by(5) {
            for blue in (0..=250u8).step_by(5) {
                let here = [red, green, blue];
                let high = here.iter().copied().max().unwrap();
                let low = here.iter().copied().min().unwrap();
                if high < 120 || f32::from(high - low) / f32::from(high) < 0.2 {
                    // Near grey the hue moves fast by nature; only demand it exists
                    // above the grey threshold.
                    continue;
                }
                let Some(a) = hue(here) else {
                    panic!("{here:?} has no hue")
                };
                for step in 0..3 {
                    let mut next = here;
                    next[step] += 5;
                    if let Some(b) = hue(next) {
                        assert!(gap(a, b) < 0.06, "{here:?} -> {next:?}: {a} -> {b}");
                    }
                }
            }
        }
    }
    let pastel = |green| {
        hue(crate::saber_rgb::BladeColor::Rgb([207, green, 234])
            .tint()
            .map(|c| (c * 255.0).round() as u8))
    };
    assert!(gap(pastel(185).unwrap(), pastel(190).unwrap()) < 0.02);
}

#[test]
fn a_skins_blur_trail_is_drawn_in_its_own_colour() {
    use crate::saber_trail::vertex_style;
    let skin = BladeColor::Skin(sample_color());
    // The blur trail of a skin tints its white core too (style 3)...
    assert_eq!(vertex_style(0, skin), 3);
    // ...while the sword trail, stock blades and black blades keep their styles.
    assert_eq!(vertex_style(1, skin), 1);
    assert_eq!(vertex_style(0, BladeColor::Retail(Color::Blue)), 0);
    assert_eq!(vertex_style(1, BladeColor::Retail(Color::Blue)), 1);
    assert_eq!(vertex_style(0, BladeColor::Black), 2);
    assert_eq!(vertex_style(1, BladeColor::Black), 1);
}

#[test]
fn each_saber_wears_its_own_skin() {
    let worn = sample_color();
    let other = SkinColor { index: 5, ..worn };
    let mut skins = SaberSkins::default();
    skins.follow_looks_each(1, 1, |client| match client {
        2 => (Some(worn), Second::Own(Some(other)), None),
        3 => (Some(worn), Second::Own(None), None),
        4 => (Some(worn), Second::Same, None),
        _ => (None, Second::Same, None),
    });
    let indices = |skins: &SaberSkins, entity| {
        skins
            .get_both(entity)
            .map(|skin| skin.map(|skin| skin.index))
    };
    assert_eq!(indices(&skins, 3), [Some(worn.index), Some(5)]);
    assert_eq!(indices(&skins, 4), [Some(worn.index), None]);
    assert_eq!(indices(&skins, 5), [Some(worn.index), Some(worn.index)]);
    assert_eq!(indices(&skins, 6), [None, None]);
    // The local player's own slot takes its own second saber.
    skins.set_local_each(Some(2), None, Second::Own(Some(other)), None);
    assert_eq!(indices(&skins, 3), [None, Some(5)]);
}

#[test]
fn a_staffs_other_half_wears_the_second_skin() {
    let first = sample_color();
    let second = SkinColor { index: 5, ..first };
    let skins = [Some(first), Some(second)];
    let index = |two, saber, other_half| {
        skin_of_blade(skins, two, saber, other_half).map(|skin| skin.index)
    };
    // One staff: its first half the first skin, the other half the second.
    assert_eq!(index(false, 0, false), Some(first.index));
    assert_eq!(index(false, 0, true), Some(5));
    // Two sabers: every blade of the second saber, none of the first.
    assert_eq!(index(true, 0, true), Some(first.index));
    assert_eq!(index(true, 1, false), Some(5));
    set_local_pair([Some(true), None]);
    assert_eq!(local_pair(), Pair::Staff);
    set_local_pair([Some(false), Some(false)]);
    assert_eq!(local_pair(), Pair::TwoSabers);
    // One saber of one blade, or a crossguard's three blades in one direction's half.
    set_local_pair([Some(false), None]);
    assert_eq!(local_pair(), Pair::None);
}

#[test]
fn the_preview_dresses_one_side_and_leaves_the_other_as_worn() {
    let worn_first = sample_color();
    let shown = SkinColor {
        index: 7,
        ..worn_first
    };
    let index = |pair: [Option<SkinColor>; 2]| pair.map(|skin| skin.map(|skin| skin.index));
    // The second wears the first's skin (none of its own): previewing on the first leaves
    // it so, as Equip will.
    let worn = [Some(worn_first), Some(worn_first)];
    assert_eq!(
        index(previewed(worn, Some(Some(shown)), [true, false])),
        [Some(7), Some(worn_first.index)]
    );
    assert_eq!(
        index(previewed(worn, Some(None), [false, true])),
        [Some(worn_first.index), None]
    );
    assert_eq!(index(previewed(worn, None, [true, true])), index(worn));
}

#[test]
fn the_sun_trails_plasma_and_its_extras_change_it_only_when_switched_on() {
    use crate::blade_extras::TrailKind;
    use crate::blade_skin_file::extras::option_bit;
    use crate::saber_trail::{STYLE, WAKE, vertex_style};
    let skins = LoadedSkins::load(VirtualFileSystem::new(), 1);
    let plain = skins.color_of("saber_sun").unwrap();
    assert_eq!(plain.trail_kind, TrailKind::Plasma, "every wearer's");
    assert!(!plain.wake && plain.petals.is_none());
    assert_eq!(vertex_style(0, BladeColor::Skin(plain)), 5);
    let with = |ids: &[&str]| {
        skins
            .color_with(
                "saber_sun",
                ids.iter().fold(0, |bits, id| bits | option_bit(id)),
            )
            .unwrap()
    };
    let flare = with(&["flare_trail"]);
    assert_eq!(flare.trail_kind, TrailKind::Flare);
    assert_eq!(vertex_style(0, BladeColor::Skin(flare)), 6);
    let all = with(&["flare_trail", "swing_embers", "heat_wake", "haze"]);
    let style = vertex_style(0, BladeColor::Skin(all));
    assert_eq!((style & STYLE, style & WAKE), (6, WAKE));
    assert_eq!(
        all.petals.map(|petals| petals.kind),
        Some(crate::saber_petals::PetalKind::Ember)
    );
    // The parts switched off still go off with the extras on.
    assert_ne!(all.off, plain.off);
    // The sword trail stays the sword trail.
    assert_eq!(vertex_style(1, BladeColor::Skin(all)), 1);
}

#[test]
fn the_varykino_blossom_sheds_petals_until_its_wearer_switches_them_off() {
    let skins = LoadedSkins::load(VirtualFileSystem::new(), 1);
    let color = skins.color_of("saber_varykino_blossom").unwrap();
    let petals = color
        .petals
        .expect("its signature's @petals reaches the colour");
    assert_eq!(petals.part, 3);
    // Switched off in the Collection: none.
    let off = skins.get("saber_varykino_blossom").unwrap().options();
    let falling = off.iter().find(|o| o.name == "Falling blossoms").unwrap();
    assert!(
        skins
            .color_with(
                "saber_varykino_blossom",
                crate::blade_skin_file::extras::option_bit(&falling.id)
            )
            .unwrap()
            .petals
            .is_none()
    );
}

#[test]
fn a_duel_draws_every_blade_stock() {
    let worn = sample_color();
    let mut skins = SaberSkins::default();
    skins.follow_looks(1, 1, |client| (client == 2).then_some(worn));
    assert_eq!(skins.get(3).map(|skin| skin.index), Some(worn.index));
    assert_eq!(skins.sound_sets()[2], Some(worn.index));
    // In a duel: the stock blade in the wearer's own colour, and the stock sounds.
    skins.set_plain(true);
    assert!(skins.get(3).is_none());
    let stock = BladeColor::Retail(Color::Green);
    assert!(matches!(
        skins.blade_color(3, stock),
        BladeColor::Retail(Color::Green)
    ));
    assert_eq!(skins.sound_sets()[2], None);
    skins.set_plain(false);
    assert_eq!(skins.get(3).map(|skin| skin.index), Some(worn.index));
}

#[test]
fn a_duel_draws_the_second_saber_stock_too() {
    let worn = sample_color();
    let other = SkinColor { index: 5, ..worn };
    let mut skins = SaberSkins::default();
    skins.follow_looks_each(1, 1, |client| {
        if client == 2 {
            (Some(worn), Second::Own(Some(other)), None)
        } else {
            (None, Second::Same, None)
        }
    });
    skins.set_plain(true);
    assert_eq!(skins.get_both(3), [None, None]);
    skins.set_plain(false);
    assert_eq!(
        skins.get_both(3).map(|skin| skin.map(|skin| skin.index)),
        [Some(worn.index), Some(5)]
    );
}

#[test]
fn a_click_on_a_staff_goes_to_the_half_it_lands_on() {
    set_staff_halves(Some([[100.0, 300.0], [500.0, 300.0]]));
    assert_eq!(staff_half_at([140.0, 290.0]), Some(false));
    assert_eq!(staff_half_at([460.0, 320.0]), Some(true));
    set_staff_halves(None);
    assert_eq!(staff_half_at([460.0, 320.0]), None);
}

/// Stock blades for weak PCs (`cg_stockBlades`): 1 draws other players' blades stock,
/// the local player's own (in its slot) keeping its skin; 2 every blade. Thrown blades
/// follow (`blade_color`), the menus' preview (`local`) and the sounds do not.
#[test]
fn stock_blades_draw_others_or_every_blade_stock() {
    use crate::stock_blades::StockBlades;
    let worn = sample_color();
    let mut skins = SaberSkins::default();
    skins.follow_looks(1, 1, |client| (client == 5).then_some(worn));
    skins.set_local(Some(2), Some(worn));
    let index = |skins: &SaberSkins, entity| skins.get(entity).map(|skin| skin.index);
    // Off (the default): everyone's skin.
    assert_eq!(index(&skins, 3), Some(worn.index), "the local player's own");
    assert_eq!(index(&skins, 6), Some(worn.index));
    skins.set_stock(StockBlades::Others);
    assert_eq!(index(&skins, 3), Some(worn.index), "yours keeps its skin");
    assert_eq!(skins.get(6), None);
    let stock = BladeColor::Retail(Color::Green);
    assert!(matches!(
        skins.blade_color(6, stock),
        BladeColor::Retail(Color::Green)
    ));
    assert!(matches!(skins.blade_color(3, stock), BladeColor::Skin(_)));
    skins.set_stock(StockBlades::All);
    assert_eq!(skins.get(3), None);
    assert_eq!(skins.get(6), None);
    // The menus still show what the player wears, and everyone is heard as chosen.
    assert_eq!(skins.local().map(|skin| skin.index), Some(worn.index));
    assert_eq!(skins.sound_sets()[5], Some(worn.index));
    assert_eq!(skins.sound_sets()[2], Some(worn.index));
    assert_eq!(skins.sound_of(6), Some(worn.index));
    // With no session (no slot of one's own), every player is another.
    skins.set_local(None, Some(worn));
    skins.set_stock(StockBlades::Others);
    assert_eq!(skins.get(6), None);
    assert_eq!(skins.local().map(|skin| skin.index), Some(worn.index));
    skins.set_stock(StockBlades::Off);
    assert_eq!(index(&skins, 6), Some(worn.index));
}

/// The sound set each slot is heard with is taken apart from its blade (Nib's idea,
/// 23/10/2026): another shader's, the stock sounds, or the blade's; the local player's
/// own goes in its slot, the animation swings read the same table, and a duel's stock
/// blades are heard stock.
#[test]
fn sounds_are_taken_apart_from_the_blade_for_others_and_the_local_player() {
    let worn = sample_color();
    let mut skins = SaberSkins::default();
    skins.follow_looks_each(1, 1, |client| match client {
        // A blade heard with another skin's set (number 3).
        2 => (Some(worn), Second::Same, Some(3)),
        // A blade heard with the stock sounds.
        4 => (Some(worn), Second::Same, None),
        // The stock blade heard with a skin's set.
        6 => (None, Second::Same, Some(worn.index)),
        _ => (None, Second::Same, None),
    });
    let sets = skins.sound_sets();
    assert_eq!(
        (sets[2], sets[4], sets[6]),
        (Some(3), None, Some(worn.index))
    );
    assert_eq!(
        skins.get(5).map(|skin| skin.index),
        Some(worn.index),
        "blade kept"
    );
    assert_eq!(skins.sound_of(3), Some(3), "the swings' set, by entity");
    assert_eq!(skins.sound_of(5), None);
    // The local player in slot 4, heard with set 1 on the stock blade.
    skins.set_local_each(Some(4), None, Second::Same, Some(1));
    assert_eq!(skins.sound_sets()[4], Some(1));
    assert_eq!(skins.get(5), None);
    skins.set_plain(true);
    assert_eq!(skins.sound_sets(), [None; MAX_CLIENTS]);
    assert_eq!(skins.sound_of(3), None);
}

#[test]
fn the_skins_built_ahead_are_those_worn_in_the_slots_and_by_the_local_player() {
    let skin = |index| SkinColor {
        index,
        ..sample_color()
    };
    let worn = |skins: &SaberSkins| {
        let (revision, worn) = skins.worn_ahead();
        let mut worn: Vec<SkinIndex> = worn.collect();
        worn.sort_unstable();
        worn.dedup();
        (revision, worn)
    };
    let mut skins = SaberSkins::default();
    assert_eq!(worn(&skins), (0, vec![]));
    // In the menus: the local player's own, before any session.
    skins.set_local_each(None, Some(skin(4)), Second::Own(Some(skin(9))), None);
    let (menus, ahead) = worn(&skins);
    assert_eq!(ahead, [4, 9]);
    // In a session: the other slots' skins on both sabers, the local one's in its slot.
    skins.follow_looks_each(1, 1, |client| match client {
        3 => (Some(skin(1)), Second::Same, None),
        5 => (Some(skin(2)), Second::Own(Some(skin(7))), None),
        // The hub's look for the local slot loses to the local choice.
        8 => (Some(skin(30)), Second::Same, None),
        _ => (None, Second::Same, None),
    });
    skins.set_local_each(Some(8), Some(skin(4)), Second::Own(Some(skin(9))), None);
    let (session, ahead) = worn(&skins);
    assert!(session > menus);
    assert_eq!(ahead, [1, 2, 4, 7, 9]);
    // Nothing changed: the same revision, so nothing is asked again.
    skins.follow_looks_each(1, 1, |_| unreachable!());
    skins.set_stock(crate::stock_blades::StockBlades::Off);
    assert_eq!(worn(&skins).0, session);
    // Others drawn stock: theirs are not built; the own blade still is.
    skins.set_stock(crate::stock_blades::StockBlades::Others);
    let (stocked, ahead) = worn(&skins);
    assert!(stocked > session);
    assert_eq!(ahead, [4, 9]);
    skins.set_stock(crate::stock_blades::StockBlades::All);
    assert_eq!(worn(&skins).1, Vec::<SkinIndex>::new());
    // A duel's stock blades end: its skins are kept ahead.
    skins.set_stock(crate::stock_blades::StockBlades::Off);
    skins.set_plain(true);
    assert_eq!(worn(&skins).1, [1, 2, 4, 7, 9]);
}
