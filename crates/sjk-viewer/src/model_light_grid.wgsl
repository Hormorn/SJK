// R_SetupEntityLighting tr_light.cpp:343-412, now evaluated at the fragment position.
fn spatial_model_light(position: vec3<f32>) -> EntityLight {
    var light = sample_model_grid(position);
    return finish_model_light(position,light);
}

// Keep the grid's indirect fill and existing point-light law; replace only baked direction.
fn model_light_without_baked_direction(position: vec3<f32>) -> EntityLight {
    let grid = sample_model_grid(position);
    return finish_model_light(position,EntityLight(grid.ambient,vec3(0.0),vec3(0.0)));
}

fn finish_model_light(position: vec3<f32>, source: EntityLight) -> EntityLight {
    var light = source;
    light.ambient = light.ambient * 0.6 + vec3(32.0);
    var direction = light.direction * length(light.directed);
    for (var index = 0u; index < min(point_lights.metadata.x, 32u); index += 1u) {
        let source = point_lights.lights[index];
        let toward = source.origin_radius.xyz - position;
        let distance = max(length(toward), 16.0);
        // Within the light's reach, the static world between it and this pixel stops it
        // (`point_light_visibility`, with no surface plane: a model is not in the tile).
        let strength = 16.0 * source.origin_radius.w * source.origin_radius.w /
            (distance * distance) * point_light_visibility(index, position, vec3(0.0));
        light.directed += strength * source.color.rgb;
        direction += strength * grid_unit(toward);
    }
    light.direction = grid_unit(direction);
    if dot(direction, direction) == 0.0 {
        light.direction = vec3(0.42857, 0.28571, 0.85714);
    }
    light.ambient = min(light.ambient, vec3(255.0)) / 255.0;
    light.directed /= 255.0;
    return light;
}

// Toon models: stylized, banded light on a lit model instead of the smooth falloff, with
// a rim of light on its silhouette. Drawn on a model whose wearer chose it for their own
// (`cg_toonModel`, `toon_choice.rs`), told by its entity light's `a` of 2 (`entity_vertex`),
// while this PC shows them (`r_toonModels`, bits 1..3 of `point_lights.metadata.y`).
fn toon_model(lit: f32) -> bool {
    return lit > 1.5 && ((point_lights.metadata.y >> 1u) & 3u) != 0u;
}

// A staircase with soft risers: `x` rounded to whole steps, each step eased over a
// fifth of its width so the band edges stay smooth at any resolution.
fn toon_steps(x: f32) -> f32 {
    let whole = floor(x);
    return whole + smoothstep(0.4, 0.6, x - whole);
}

// The rim: grazing view angles (the silhouette) light up, eased in over a narrow band.
fn toon_rim(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    let toward = normalize(camera.camera_position - position);
    let facing = abs(dot(normalize(normal), toward));
    return smoothstep(0.55, 0.65, 1.0 - facing);
}

// Shadow and lit tones around a light level: the map's grid often gives a weak direction,
// so the tones are set apart from the mean (0.65x and 1.15x) instead of taken from it.
fn toon_tone(mean: vec3<f32>, band: f32) -> vec3<f32> {
    return mean * mix(0.65, 1.15, band);
}

// RB_CalcDiffuseColor's light as two tones and a half step between them, by which side
// faces the light, plus the rim, saturated like stock.
fn toon_diffuse(position: vec3<f32>, normal: vec3<f32>, light: EntityLight) -> vec3<f32> {
    let incoming = clamp(dot(normalize(normal), light.direction), 0.0, 1.0);
    let band = toon_steps(incoming * 2.0) * 0.5;
    let mean = light.ambient + 0.5 * light.directed;
    let rim = toon_rim(position, normal) * 0.5 * mean;
    return min(toon_tone(mean, band) + rim, vec3(1.0));
}

// Real-time lighting hands models one summed colour with the sun's shadows in it: its
// brightness goes in half steps (beyond 1 too) without ever dropping below 0.6 of
// itself, its hue unchanged, and the rim brightens the silhouette.
fn toon_lit(position: vec3<f32>, normal: vec3<f32>, lit: vec3<f32>) -> vec3<f32> {
    let level = max(dot(lit, vec3(0.2126, 0.7152, 0.0722)), 1e-4);
    let stepped = max(toon_steps(level * 2.0) * 0.5, level * 0.6);
    let banded = lit * stepped / level;
    return banded + toon_rim(position, normal) * 0.5 * max(banded, vec3(0.1));
}

// The textured result of a toon-lit stage, a fifth more saturated.
fn toon_colour(rgb: vec3<f32>) -> vec3<f32> {
    let grey = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
    return max(mix(vec3(grey), rgb, 1.2), vec3(0.0));
}
