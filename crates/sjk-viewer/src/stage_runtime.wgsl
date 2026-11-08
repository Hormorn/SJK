// Quake 3 world stage evaluator. The only frame-varying uniform is
// (camera.shader_time - stage.emission.z); material tables and animation arrays are map-lifetime.

// Set for RF_FORCE_ENT_ALPHA pipelines (`world_forced_alpha.rs`).
override forced_entity_alpha: bool = false;

@group(1) @binding(0) var stage_images: texture_2d_array<f32>;
@group(1) @binding(1) var stage_sampler: sampler;
@group(1) @binding(2) var secondary_images: texture_2d_array<f32>;
@group(1) @binding(3) var secondary_sampler: sampler;
@group(1) @binding(4) var lightmap_image: texture_2d<f32>;
@group(1) @binding(5) var<uniform> stage: Stage;
@group(1) @binding(6) var<uniform> point_lights: PointLightBlock;

// Entity light computed per instance by `entity_lighting`
// (R_SetupEntityLighting, tr_light.cpp:304-412). Passed as vertex-stage
// parameters so the world path does not carry unused light attributes.

fn no_entity_light() -> EntityLight {
    return EntityLight(vec3(0.0), vec3(0.0), vec3(0.0, 0.0, 1.0));
}

fn entity_light(instance: InstanceInput) -> EntityLight {
    return EntityLight(instance.light_ambient, instance.light_directed, instance.light_direction);
}

// RB_CalcDiffuseColor (tr_shade_calc.cpp:1141-1191): ambient when the light
// arrives from behind, otherwise ambient + incoming * directed, saturated.
fn diffuse_light(normal: vec3<f32>, light: EntityLight) -> vec3<f32> {
    let incoming = dot(normalize(normal), light.direction);
    if incoming <= 0.0 {
        return light.ambient;
    }
    return min(light.ambient + incoming * light.directed, vec3(1.0));
}

// RB_CalcSpecularAlpha (`tr_shade_calc.cpp:1080-1134`): the view vector
// against the light reflected off the normal, raised to the fourth power.
// Models reflect their entity light; world surfaces the fixed `lightOrigin`
// (-960, 1980, 96) rd-vanilla never made dynamic. World space here, local
// space there — the dot products are the same.
fn specular_alpha(position: vec3<f32>, normal: vec3<f32>, light: EntityLight,
                  entity_lit: bool) -> f32 {
    let world_light = normalize(vec3(-960.0, 1980.0, 96.0) - position);
    let light_direction = select(world_light, light.direction, entity_lit);
    let unit_normal = normalize(normal);
    let reflected = unit_normal * (2.0 * dot(unit_normal, light_direction)) - light_direction;
    let viewer = normalize(camera.camera_position - position);
    let facing = dot(reflected, viewer);
    if facing < 0.0 {
        return 0.0;
    }
    let squared = facing * facing;
    return min(squared * squared, 1.0);
}

struct VertexOutput {
    // Invariant: the mirror composite, the fog pass and ambient occlusion
    // all test for *equal* depth against what this program wrote.
    @builtin(position) @invariant position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) stage_uv: vec2<f32>,
    @location(4) secondary_uv: vec2<f32>,
    @location(5) entity_color: vec4<f32>,
    // rgb: `rgbGen lightingDiffuse`; a: `alphaGen lightingSpecular`.
    @location(6) lighting_diffuse: vec4<f32>,
    @location(7) entity_control: vec2<f32>,
    @location(8) @interpolate(flat) light_ambient: vec4<f32>,
    @location(9) @interpolate(flat) light_directed: vec3<f32>,
    @location(10) @interpolate(flat) light_direction: vec3<f32>,
    @location(11) @interpolate(flat) animation_index: i32,
};

fn vertex_result(input: VertexInput, position: vec3<f32>, normal: vec3<f32>,
                 entity_color: vec4<f32>, shader_tex_coord: vec2<f32>,
                 entity_control: vec2<f32>, light: EntityLight, entity_lit: bool) -> VertexOutput {
    var output: VertexOutput;
    output.position = clip_position(position, 0.0);
    output.world_position = position;
    output.world_normal = normal;
    output.color = input.color;
    if i32(stage.generators.y) == 11 {
        output.color.a = floor(clamp(distance(position, camera.camera_position) /
            stage.wave_functions.z, 0.0, 1.0) * 255.0) / 255.0;
    }
    output.entity_color = entity_color;
    output.entity_control = entity_control;
    output.animation_index = -1;
    output.light_ambient = vec4(light.ambient, select(0.0, 1.0, entity_lit));
    output.light_directed = light.directed;
    output.light_direction = light.direction;
    output.lighting_diffuse = vec4(
        select(input.color.rgb, diffuse_light(normal, light), entity_lit),
        specular_alpha(position, normal, light, entity_lit));
    var uv = select(input.texture_coordinates, input.lightmap_coordinates,
        i32(stage.animation.w) == 1);
    // RB_CalcEnvironmentTexCoords (`tr_shade_calc.cpp:939-966`) reflects the
    // view vector about the transformed surface normal. This world-space form
    // is algebraically identical to rd-vanilla's entity-local calculation.
    if i32(stage.animation.w) == 2 {
        let reflected = reflect(normalize(position - camera.camera_position), normalize(normal));
        uv = reflected.yz * vec2(0.5, -0.5) + vec2(0.5);
    }
    // TCGEN_VECTOR (`tr_shade.cpp` ComputeTexCoords): the position dotted with the stage's
    // two axes. World surfaces are in world space as in rd-vanilla; an entity's surface
    // projects from its placed position rather than its model space.
    if i32(stage.animation.w) == 3 {
        uv = vec2(dot(position, stage.texture_vectors[0].xyz),
                  dot(position, stage.texture_vectors[1].xyz));
    }
    output.stage_uv = apply_tcmods(uv, position, shader_tex_coord);
    var secondary_uv = select(input.texture_coordinates, input.lightmap_coordinates,
        i32(stage.secondary_animation.w) == 1);
    if i32(stage.secondary_animation.w) == 2 {
        let reflected = reflect(normalize(position - camera.camera_position), normalize(normal));
        secondary_uv = reflected.yz * vec2(0.5, -0.5) + vec2(0.5);
    }
    output.secondary_uv = apply_secondary_tcmods(secondary_uv, position, shader_tex_coord);
    return output;
}

@vertex fn vertex_main(input: VertexInput, @builtin(vertex_index) index: u32) -> VertexOutput {
    let vertex = material_vertex(input, index, vec3(0.0), vec4(0.0, 0.0, 0.0, 1.0), vec3(1.0));
    return vertex_result(vertex, vertex.position, vertex.normal, vec4(1.0), vec2(0.0), vec2(0.0),
        no_entity_light(), false);
}

// One instanced vertex entry for world statics (the identity instance), movers and
// Ghoul2/MD3 entities, so a pipeline key costs the driver one compile. Instances flagged
// as world geometry (`view_flags` bit 4) take no entity light and keep the world's fixed
// specular origin.
@vertex fn entity_vertex_main(input: VertexInput, instance: InstanceInput,
    @builtin(vertex_index) index: u32) -> VertexOutput {
    return entity_vertex(input, instance, index);
}
fn entity_vertex(input: VertexInput, instance: InstanceInput, index: u32) -> VertexOutput {
    let vertex = material_vertex(input, index, instance.position.xyz, instance.rotation,
        instance.scale);
    let position = instance_position(vertex, instance);
    let world = (instance.view_flags & 4u) != 0u;
    var light = entity_light(instance);
    if world { light = no_entity_light(); }
    var output = vertex_result(vertex, position,
        normalize(rotate_vector(instance.rotation, vertex.normal)),
        instance.entity_color, instance.shader_tex_coord, instance.entity_control,
        light, !world);
    if (instance.view_flags & 8u) != 0u { output.animation_index = i32((instance.view_flags >> 8u) & 65535u); }
    // RF_DEPTHHACK: rd-vanilla narrows the depth range to 0..0.3
    // (`tr_backend.cpp:906`) so the view weapon draws over the world.
    output.position = clip_position(position, instance.position.w);
    let disintegrate = (instance.view_flags >> 24u) & 3u;
    if disintegrate != 0u { disintegrate_vertex(&output, vertex, instance, disintegrate); }
    if (instance.view_flags & (1u << 26u)) != 0u { volumetric_vertex(&output, vertex.normal, instance); }
    // Display-space `GL_DST_COLOR GL_ONE` (`DISPLAY_FILTER_ADD`, `body_fade.rs`): told to
    // `stage_fragment` by entity_control.y = -1, which no forced-alpha test reads.
    if (instance.view_flags & (1u << 27u)) != 0u { output.entity_control.y = -1.0; }
    // A secondary map view must not inherit the main camera's view weapon.
    if !instance_visible(instance) { output.position = vec4(2.0, 2.0, 2.0, 1.0); }
    return output;
}

// RF_DISINTEGRATE1/2 (`CG_Disintegration`): rd-vanilla's RB_CalcDisintegrateColors and
// RB_CalcDisintegrateVertDeform (`tr_shade_calc.cpp`). The instance carries the hit point
// in light_direction and the burn radius in light_directed.x (`disintegration.rs`). The
// colours replace every generator (`killGen`); entity_control.x = 3 tells the fragment
// stage, which also applies DISINTEGRATE1's alpha test (`tr_shade.cpp`).
fn disintegrate_vertex(output: ptr<function, VertexOutput>, vertex: VertexInput,
                       instance: InstanceInput, mode: u32) {
    let threshold = instance.light_directed.x * instance.light_directed.x;
    let world = (*output).world_position;
    let offset = instance.light_direction - world;
    let distance = dot(offset, offset);
    var color = vec4(1.0);
    if mode == 1u {
        if distance < threshold { color = vec4(1.0, 1.0, 1.0, 0.0); }
        else if distance < threshold + 60.0 { color = vec4(0.0, 0.0, 0.0, 1.0); }
        else if distance < threshold + 150.0 { color = vec4(vec3(111.0 / 255.0), 1.0); }
        else if distance < threshold + 180.0 { color = vec4(vec3(175.0 / 255.0), 1.0); }
    } else {
        var push = vec3(0.0);
        if distance < threshold {
            color = vec4(0.0);
            push = vertex.normal * vec3(2.0, 2.0, 0.5);
        } else if distance < threshold + 50.0 {
            push = vec3(vertex.normal.xy, 0.0);
        }
        let moved = world + rotate_vector(instance.rotation, push * instance.scale);
        (*output).world_position = moved;
        (*output).position = clip_position(moved, instance.position.w);
    }
    (*output).color = color;
    (*output).entity_control = vec2(3.0, 0.0);
}

// RF_VOLUMETRIC (`tr_shade.cpp:1164-1189`, the DEMP2 bubble of `demp2_shell.rs`): every
// channel is shaderRGBA's red times one less the fourth power of the model normal's dot
// with the view axis, that power clamped to zero under 0.2, so the rim glows and the
// middle stays clear. rd-vanilla reads the unrotated model normal ("does not work for
// rotated models"); the colours replace every generator, as disintegration's do, with
// entity_control.x = 4 so the fragment stage skips DISINTEGRATE1's alpha test.
fn volumetric_vertex(output: ptr<function, VertexOutput>, normal: vec3<f32>,
                     instance: InstanceInput) {
    var facing = dot(normal, camera.view_forward);
    facing = facing * facing * facing * facing;
    if facing < 0.2 { facing = 0.0; }
    (*output).color = vec4(floor(instance.entity_color.r * 255.0 * (1.0 - facing)) / 255.0);
    (*output).entity_control = vec2(4.0, 0.0);
}

// Ordered RB_Calc{Turbulent,Scale,Scroll,Transform,Rotate}TexCoords and
// RB_CalcStretchTexCoords: tr_shade_calc.cpp:96-113,968-1067.
fn apply_tcmods(initial: vec2<f32>, position: vec3<f32>, entity_translate: vec2<f32>) -> vec2<f32> {
    var uv = initial;
    for (var index = 0; index < 4; index++) {
        if index >= i32(stage.generators.w) { break; }
        let a = stage.tcmod_a[index];
        let b = stage.tcmod_b[index];
        let kind = i32(a.x);
        if kind == 1 {
            uv += fract(vec2(a.y, a.z) * (camera.shader_time - stage.emission.z));
        } else if kind == 2 {
            uv *= vec2(a.y, a.z);
        } else if kind == 3 {
            let cycle = -a.y * (camera.shader_time - stage.emission.z) / 360.0;
            let sine = table_value(0, cycle);
            let cosine = table_value(0, cycle + 0.25);
            let centered = uv - vec2(0.5);
            uv = vec2(cosine * centered.x - sine * centered.y,
                      sine * centered.x + cosine * centered.y) + vec2(0.5);
        } else if kind == 4 {
            let now = a.w + (camera.shader_time - stage.emission.z) * b.x;
            uv.x += table_value(0, (position.x + position.z) / 1024.0 + now) * a.z;
            uv.y += table_value(0, position.y / 1024.0 + now) * a.z;
        } else if kind == 5 {
            let value = 1.0 / wave(vec4(a.z, a.w, b.x, b.y), i32(a.y));
            uv = uv * value + vec2(0.5 - 0.5 * value);
        } else if kind == 6 {
            uv = vec2(uv.x * a.y + uv.y * a.w + b.y,
                      uv.x * a.z + uv.y * b.x + b.z);
        } else if kind == 7 {
            uv += entity_translate;
        }
    }
    return uv;
}

fn apply_secondary_tcmods(initial: vec2<f32>, position: vec3<f32>,
                          entity_translate: vec2<f32>) -> vec2<f32> {
    var uv = initial;
    for (var index = 0; index < 4; index++) {
        if index >= i32(stage.secondary_control.x) { break; }
        let a = stage.secondary_tcmod_a[index];
        let b = stage.secondary_tcmod_b[index];
        let kind = i32(a.x);
        if kind == 1 {
            uv += fract(vec2(a.y, a.z) * (camera.shader_time - stage.emission.z));
        } else if kind == 2 {
            uv *= vec2(a.y, a.z);
        } else if kind == 3 {
            let cycle = -a.y * (camera.shader_time - stage.emission.z) / 360.0;
            let sine = table_value(0, cycle);
            let cosine = table_value(0, cycle + 0.25);
            let centered = uv - vec2(0.5);
            uv = vec2(cosine * centered.x - sine * centered.y,
                      sine * centered.x + cosine * centered.y) + vec2(0.5);
        } else if kind == 4 {
            let now = a.w + (camera.shader_time - stage.emission.z) * b.x;
            uv.x += table_value(0, (position.x + position.z) / 1024.0 + now) * a.z;
            uv.y += table_value(0, position.y / 1024.0 + now) * a.z;
        } else if kind == 5 {
            let value = 1.0 / wave(vec4(a.z, a.w, b.x, b.y), i32(a.y));
            uv = uv * value + vec2(0.5 - 0.5 * value);
        } else if kind == 6 {
            uv = vec2(uv.x * a.y + uv.y * a.w + b.y,
                      uv.x * a.z + uv.y * b.x + b.z);
        } else if kind == 7 {
            uv += entity_translate;
        }
    }
    return uv;
}

// RB_CalcWaveColor and alpha counterpart: tr_shade_calc.cpp:725-757.
// Entity/diffuse/spec sources are dispatched in RB_ComputeColors at
// tr_shade.cpp:1175-1348. RB_CalcDiffuseColor is tr_shade_calc.cpp:1138-1191;
// Legacy evaluation is per vertex. Optional diffuse samples the grid in world space.
fn generated_color(input: VertexOutput) -> vec4<f32> {
    // RF_DISINTEGRATE1/2 and RF_VOLUMETRIC replace every generator
    // (`disintegrate_vertex`, `volumetric_vertex`).
    if input.entity_control.x > 2.5 { return input.color; }
    var color = vec4(1.0);
    let rgb_kind = i32(stage.generators.x);
    var diffuse = input.lighting_diffuse.rgb;
    // Bit 0: `r_modelPixelLight`; bits 1..3: `r_toonModels`, which needs each pixel's
    // own normal (bands drawn per vertex would follow the triangles).
    // A model normal map lights each pixel through its own bent normal as well.
    if (point_lights.metadata.y != 0u || model_normal_on) && input.light_ambient.a != 0.0 &&
        (rgb_kind == 4 || rgb_kind == 9) {
        var light = EntityLight(input.light_ambient.rgb,
            input.light_directed, input.light_direction);
        if (point_lights.metadata.y & 1u) != 0u && model_grid.bounds.w != 0u {
            light = spatial_model_light(input.world_position);
        }
        let normal = model_shading_normal(input.world_normal);
        if toon_models() {
            diffuse = toon_diffuse(input.world_position, normal, light);
        } else {
            diffuse = diffuse_light(normal, light);
        }
    }
    if rgb_kind == 1 { color = vec4(vec3(1.0), color.a); }
    if rgb_kind == 2 || rgb_kind == 3 { color = vec4(input.color.rgb, color.a); }
    if rgb_kind == 4 { color = vec4(diffuse, color.a); }
    if rgb_kind == 5 {
        color = vec4(vec3(clamp(wave(stage.rgb_wave, i32(stage.wave_functions.x)), 0.0, 1.0)),
            color.a);
    }
    if rgb_kind == 6 { color = vec4(stage.constant_color.rgb, color.a); }
    if rgb_kind == 7 { color = vec4(input.entity_color.rgb, color.a); }
    if rgb_kind == 8 { color = vec4(vec3(1.0) - input.entity_color.rgb, color.a); }
    // RB_CalcDiffuseEntityColor (`tr_shade_calc.cpp:1197-1240`) scales both the
    // ambient and directed light by shaderRGBA.
    if rgb_kind == 9 { color = vec4(diffuse * input.entity_color.rgb, color.a); }
    // The entity generators write shaderRGBA[3] too, but rd-vanilla's alphaGen
    // switch reads `pStage->alphaGen` rather than `forceAlphaGen`
    // (`tr_shade.cpp:1305-1315`), so AGEN_IDENTITY still forces 0xff.
    let alpha_kind = i32(stage.generators.y);
    if alpha_kind == 2 || alpha_kind == 3 || alpha_kind == 4 {
        color = vec4(color.rgb, input.color.a);
    }
    if alpha_kind == 5 {
        color = vec4(color.rgb,
            clamp(wave(stage.alpha_wave, i32(stage.wave_functions.y)), 0.0, 1.0));
    }
    if alpha_kind == 6 { color = vec4(color.rgb, stage.constant_color.a); }
    if alpha_kind == 7 { color = vec4(color.rgb, input.entity_color.a); }
    if alpha_kind == 8 { color = vec4(color.rgb, 1.0 - input.entity_color.a); }
    if alpha_kind == 10 { color = vec4(color.rgb, input.lighting_diffuse.a); }
    if alpha_kind == 11 { color = vec4(color.rgb, input.color.a); }
    // RF_RGB_TINT and RF_FORCE_ENT_ALPHA override the stage generators in
    // rd-vanilla `tr_shade.cpp:1648-1675`; cgame supplies those flags for
    // disabled/respawning pickups.
    if input.entity_control.x > 0.5 { color = vec4(input.entity_color.rgb, color.a); }
    if input.entity_control.y > 0.5 { color = vec4(color.rgb, input.entity_color.a); }
    return color;
}

// `R_BindAnimatedImage`: RF_SETANIMINDEX overrides the clock for this entity.
fn animated_image_frame(animation: vec4<f32>, index: i32) -> i32 {
    let count = max(i32(animation.y), 1);
    var frame = max(i32(floor((camera.shader_time - stage.emission.z) * animation.x)), 0);
    if index >= 0 { frame = index; }
    return select(frame % count, min(frame, count - 1), animation.z > 0.5);
}

@fragment fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return stage_fragment(input);
}
// The player card outline's mask (`world_card_outline.rs`): where the body is in view.
@fragment fn card_outline_mask_fragment() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}
fn stage_fragment(input: VertexOutput) -> vec4<f32> {
    fragment_point_mask = finite_point_mask(input.world_position);
    // Before any branch on a varying: the frame takes screen-space derivatives. The
    // flag is the stage's, so the whole draw takes the same side.
    model_normal_on = false;
    if (u32(stage.wave_functions.w) & 16u) != 0u { model_normal_prepare(input); }
    let uv = input.stage_uv;
    // Uploaded BSP lightmaps (including the fallback) have alpha one. In these
    // modes the composition hook replaces their RGB, so sampling them is wasted.
    let replaced_lightmap = realtime_active() || (point_lights.metadata.z & 1u) != 0u;
    var texel = vec4(1.0);
    if i32(stage.animation.w) == 1 {
        if !replaced_lightmap { texel = textureSample(lightmap_image, stage_sampler, uv); }
    } else {
        let frame = animated_image_frame(stage.animation, input.animation_index);
        texel = textureSample(stage_images, stage_sampler, uv, frame);
    }
    var secondary_texel = vec4(0.0);
    if stage.secondary_control.z > 0.5 {
        secondary_texel = vec4(1.0);
        if i32(stage.secondary_animation.w) == 1 {
            if !replaced_lightmap { secondary_texel = textureSample(lightmap_image, secondary_sampler, input.secondary_uv); }
        } else {
            let secondary_frame = animated_image_frame(stage.secondary_animation, input.animation_index);
            secondary_texel = textureSample(secondary_images, secondary_sampler,
                input.secondary_uv, secondary_frame);
        }
    }
    var output = apply_lighting_mode(input, texel, secondary_texel);
    // RF_FORCE_ENT_ALPHA replaces the stage's GL_State, alpha-test bits included
    // (rd-vanilla `tr_shade.cpp:1745-1757`); blending still hides transparent texels.
    let alpha_test = select(i32(stage.generators.z), 0, forced_entity_alpha);
    if alpha_test == 1 && output.a <= 0.0 { discard; }
    if alpha_test == 2 && output.a >= (128.0 / 255.0) { discard; }
    if alpha_test == 3 && output.a < (128.0 / 255.0) { discard; }
    if alpha_test == 4 && output.a < (192.0 / 255.0) { discard; }
    // RF_DISINTEGRATE1 tests alpha >= 0.5 (GLS_ATEST_GE_C0); burnt-away vertices are 0.
    if input.entity_control.x > 2.5 && input.entity_control.x < 3.5 && output.a < 0.5 {
        discard;
    }
    // rd-vanilla blends `GL_DST_COLOR GL_ONE` on display values: the scene times one plus
    // the stage's display colour. The scene here is linear, where that factor is the sRGB
    // decode of (1 + display), extended past 1, so the stage writes that less one; blended
    // as written, it would brighten the scene far less, leaving only the texture's
    // brightest texels visible. An 8-bit scene (`r_sceneHdr 0`) clamps what is written
    // to 1, so there a stage lifts the scene at most twice in linear light.
    if input.entity_control.y < -0.5 {
        let source = vec3(1.0) + max(stage_display_value(texel.rgb) * generated_color(input).rgb,
            vec3(0.0));
        output = vec4(pow((source + 0.055) / 1.055, vec3(2.4)) - vec3(1.0), output.a);
    }
    if stage.secondary_control.w > 0.5 {
        output = vec4(output.rgb * (vec3(1.0) + dynamic_light_modulation(input)), output.a);
    }
    return output;
}

// A model's normal map (SJK; `models` in the material maps module): the stage's second
// slot holds it, linear, and `secondary_animation.xy` its `normalScale`. MD3 and Ghoul2
// vertices carry no tangents, so the frame comes from the screen-space derivatives of the
// position and the picture coordinates (Schuler's cotangent frame), its t axis the
// picture's downward one, as the world's maps read it. Two-sided faces seen from behind
// turn toward the viewer.
var<private> model_normal: vec3<f32>;
var<private> model_normal_on: bool;

fn model_normal_prepare(input: VertexOutput) {
    let uv = input.stage_uv;
    let uv_dx = dpdx(uv);
    let uv_dy = dpdy(uv);
    let p_dx = dpdx(input.world_position);
    let p_dy = dpdy(input.world_position);
    let texel = textureSampleGrad(secondary_images, secondary_sampler, uv, 0, uv_dx, uv_dy);
    var geometric = normalize(input.world_normal);
    if dot(geometric, camera.camera_position - input.world_position) < 0.0 {
        geometric = -geometric;
    }
    let across_dy = cross(p_dy, geometric);
    let across_dx = cross(geometric, p_dx);
    let tangent = across_dy*uv_dx.x + across_dx*uv_dy.x;
    let bitangent = across_dy*uv_dx.y + across_dx*uv_dy.y;
    let lengths = vec2(dot(tangent, tangent), dot(bitangent, bitangent));
    model_normal = geometric;
    model_normal_on = true;
    if min(lengths.x, lengths.y) > 1e-20 {
        // rend2 `CalcNormal`, with `r_normalMapStrength` as the world's maps take it.
        let strength = f32((((point_lights.metadata.z >> 16u) + 64u) & 255u))/64.0;
        var n = texel.rgb - vec3(0.5);
        n = vec3(n.xy*stage.secondary_animation.xy*strength, 0.0);
        n.z = sqrt(clamp((0.25 - n.x*n.x) - n.y*n.y, 0.0, 1.0));
        model_normal = normalize(n.x*tangent*inverseSqrt(lengths.x)
            + n.y*bitangent*inverseSqrt(lengths.y) + n.z*geometric);
    }
}

// The normal a model pixel is lit by: its bent one when the stage has a normal map.
fn model_shading_normal(normal: vec3<f32>) -> vec3<f32> {
    return select(normal, model_normal, model_normal_on);
}

// The sRGB display value of linear `c`, as the stage's sRGB image held it.
fn stage_display_value(c: vec3<f32>) -> vec3<f32> {
    let unit = clamp(c, vec3(0.0), vec3(1.0));
    return select(unit * 12.92, 1.055 * pow(unit, vec3(1.0 / 2.4)) - 0.055,
        unit > vec3(0.0031308));
}
