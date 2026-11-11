override visible_emission: bool = false;

// One material-composition hook. Zero keeps the previous arithmetic and blend ordering.
fn apply_lighting_mode(input: VertexOutput, primary: vec4<f32>, secondary: vec4<f32>) -> vec4<f32> {
    let mode = point_lights.metadata.z;
    let fullbright = (mode & 1u) != 0u;
    let lightmap_only = (mode & 2u) != 0u;
    let flags = u32(stage.wave_functions.w);
    // `r_flatWorld` (`flat_world.rs`): world surfaces only, the flag being per draw.
    let flat_world = (point_lights.metadata.y & 16u) != 0u && input.light_ambient.a == 0.0 &&
        input.entity_control.x <= 0.5;
    // Stock stops the stage loop at the first later non-lightmap stage; so does the
    // flat world, which draws no overlay.
    if (lightmap_only || flat_world) && (flags & 2u) != 0u { discard; }
    var color = generated_color(input);
    // Apply texture-bundle radiance before the expensive lighting paths. Straight
    // arithmetic avoids another control-flow split in the shared material shader.
    var gains = vec2(1.0);
    if visible_emission {
        gains = select(vec2(1.0), max(stage.emission.xy, vec2(1.0)),
            realtime_active() && (mode & 3u) == 0u);
    }
    var base = vec4(primary.rgb*gains.x, primary.a);
    var extra = vec4(secondary.rgb*gains.y, secondary.a);
    let base_lightmap = i32(stage.animation.w) == 1;
    let extra_lightmap = i32(stage.secondary_animation.w) == 1;
    if flat_world {
        // A texture's smallest mip is its average colour; alpha still cuts.
        if !base_lightmap {
            let frame = animated_image_frame(stage.animation, input.animation_index);
            let flat = textureSampleLevel(stage_images, stage_sampler, input.stage_uv, frame, 16.0);
            base = vec4(flat.rgb*gains.x, base.a);
        }
        if stage.secondary_control.z > 0.5 && !extra_lightmap {
            let frame = animated_image_frame(stage.secondary_animation, input.animation_index);
            let flat = textureSampleLevel(secondary_images, secondary_sampler, input.secondary_uv, frame, 16.0);
            extra = vec4(flat.rgb*gains.y, extra.a);
        }
    }
    if fullbright {
        // R_BindAnimatedImage binds tr.whiteImage: actual white, never neutral gray.
        if base_lightmap { base = vec4(1.0); }
        if extra_lightmap { extra = vec4(1.0); }
        let rgb = i32(stage.generators.x);
        if (flags & 1u) != 0u && (rgb == 2 || rgb == 3) && input.entity_control.x <= 0.5 {
            color = vec4(vec3(1.0), color.a);
        }
        // Stock's fullbright grid override saturates ordinary model diffuse to white.
        // Disintegration colours (entity_control.x = 3) replace every generator
        // (`killGen`), so they are kept, as `generated_color` returns them.
        if input.light_ambient.a != 0.0 && (rgb == 4 || rgb == 9) && input.entity_control.x <= 2.5 {
            color = vec4(select(vec3(1.0), input.entity_color.rgb,
                rgb == 9 || input.entity_control.x > 0.5), color.a);
        }
    }
    color = vec4(model_sun_color(input,color.rgb),color.a);
    let emitted = select(vec3(0.0), emitted_light(input), !fullbright);
    // Real-time lighting replaces baked illumination; authored textures retain their material role.
    if !fullbright && base_lightmap { base = realtime_lightmap(input, base); base = vec4(base.rgb+emitted,base.a); }
    // A relit shine (flag 8) carries lightmap coordinates without a secondary bundle.
    if !fullbright && extra_lightmap && stage.secondary_control.z > 0.5 { extra = realtime_lightmap(input, extra); extra = vec4(extra.rgb+emitted,extra.a); }
    {
        let rgb = i32(stage.generators.x);
        // Some legacy stacks restore the base texture over additive effects. That
        // diffuse cover must receive the same light, or its opaque paint erases shadows.
        let vertex_bake = (flags & 1u) != 0u && (rgb == 2 || rgb == 3);
        let diffuse_cover = (flags & 4u) != 0u;
        if realtime_active() && !fullbright && (vertex_bake || diffuse_cover) &&
            input.entity_control.x <= 0.5 && input.light_ambient.a == 0.0 {
            color = vec4(realtime_lightmap(input,vec4(1.0)).rgb+emitted,color.a);
        }
    }
    // An unlit environment shine over relit paint (`relighting::RELIT_SHINE`): retail adds
    // it to paint lit by the bake, so here it follows the live light relative to that bake
    // (the lightmap texel, or the vertex colour, display-encoded), never above authored.
    if realtime_active() && !fullbright && (flags & 8u) != 0u && input.entity_control.x <= 0.5 {
        var baked = pow(max(input.color.rgb, vec3(0.0)), vec3(2.2));
        if extra_lightmap {
            baked = textureSampleLevel(lightmap_image, secondary_sampler, input.secondary_uv, 0.0).rgb;
        }
        let live = realtime_lightmap(input, vec4(1.0)).rgb;
        let weights = vec3(0.2126, 0.7152, 0.0722);
        let relit = clamp(dot(live, weights)/max(dot(baked, weights), 1e-4), 0.0, 1.0);
        color = vec4(color.rgb*relit, color.a);
    }
    if stage.secondary_control.z > 0.5 {
        // DrawMultitextured's GL_REPLACE replaces RGB and alpha, including primary color.
        if lightmap_only { return extra; }
        if i32(stage.secondary_control.y) == 1 { base *= extra; }
        else if i32(stage.secondary_control.y) == 2 { base += extra; }
    }
    let result = base * color;
    let rgb = i32(stage.generators.x);
    if toon_model(input.light_ambient.a) && !fullbright &&
        input.entity_control.x <= 0.5 && (rgb == 4 || rgb == 9) {
        return vec4(toon_colour(result.rgb), result.a);
    }
    return result;
}
