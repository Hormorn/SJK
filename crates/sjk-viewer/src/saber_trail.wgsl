struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var blur_glow: texture_2d<f32>;
@group(1) @binding(1)
var blur_core: texture_2d<f32>;
@group(1) @binding(2)
var sword_trail: texture_2d<f32>;
@group(1) @binding(3)
var trail_sampler: sampler;
@group(1) @binding(4)
var black_trail: texture_2d<f32>;

// The scene behind the effects (`effect_layer.rs` `original`), for the heat wake: bound only
// for `fragment_wake`, drawn with the blades' heat haze before any other effect.
@group(2) @binding(0)
var wake_scene: texture_2d<f32>;
@group(2) @binding(1)
var wake_sampler: sampler;

// A vertex style's bit for a trail that bends what is behind it (`saber_trail::WAKE`), and
// the style itself in the low byte (`saber_trail::STYLE`).
const WAKE: u32 = 0x100u;
const STYLE: u32 = 0xffu;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // rgb the vertex colour; a the slice's age, 0 new to 1 gone (the plasma, flare and wake
    // read it; the stock blend ignores alpha).
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) style: u32,
    @location(3) world: vec3<f32>,
    // The flare's V at its slice's far edge, past the tip (`saber_trail::FLARE_REACH`).
    @location(4) @interpolate(flat) past: f32,
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) style: u32,
    @location(4) past: f32,
) -> VertexOutput {
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4(position, 1.0);
    // Stock fade (`CTrail::Update`, codemp/cgame/FxPrimitives.cpp:1771-1789):
    // `curST[0] = ST[0] * perc + (ST[0] + 1) * (1 - perc)`, clamped at 1.0,
    // computed per vertex before rasterization; `ST[1]` never moves and the
    // colour is left untouched. `color.a` carries `perc` (remaining life).
    let age = 1.0 - color.a;
    output.uv = vec2(min(uv.x + age, 1.0), uv.y);
    output.color = vec4(color.rgb, age);
    output.style = style;
    output.world = position;
    output.past = past;
    return output;
}

// Value noise in 3D, 0 to 1, smooth: the plasma's flames and the wake's shimmer, laid in the
// world so they run on unbroken from one slice to the next.
fn trail_hash(p: vec3<f32>) -> f32 {
    let q = fract(p * vec3(0.1031, 0.1030, 0.0973));
    let r = q + dot(q, q.yxz + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn trail_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(trail_hash(i), trail_hash(i + vec3(1.0, 0.0, 0.0)), u.x);
    let b = mix(trail_hash(i + vec3(0.0, 1.0, 0.0)), trail_hash(i + vec3(1.0, 1.0, 0.0)), u.x);
    let c = mix(trail_hash(i + vec3(0.0, 0.0, 1.0)), trail_hash(i + vec3(1.0, 0.0, 1.0)), u.x);
    let d = mix(trail_hash(i + vec3(0.0, 1.0, 1.0)), trail_hash(i + vec3(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

// Two octaves, rising with the slice's age as heat does.
fn trail_flames(world: vec3<f32>, age: f32, scale: f32) -> f32 {
    let p = world * scale - vec3(0.0, 0.0, age * 2.4);
    return trail_noise(p) * 0.65 + trail_noise(p * 2.3 + vec3(5.2, 1.3, 7.7)) * 0.35;
}

// The plasma's colour as it cools (`fresh` 1 just left, 0 gone): white gold, the trail's own
// colour, then a deep red going dark. `tint` is the trail colour normalised to its brightest
// channel, so another blade's plasma would cool from its own colour.
fn plasma_colour(tint: vec3<f32>, fresh: f32) -> vec3<f32> {
    let white = vec3(1.0, 0.95, 0.8);
    let hot = mix(tint, white, 0.45);
    let deep = tint * vec3(0.75, 0.18, 0.1);
    let warm = mix(deep, tint, smoothstep(0.35, 0.75, fresh));
    return mix(warm, hot, smoothstep(0.82, 1.0, fresh));
}

// The plasma (style 5, the Sun's): white-hot where the blade just passed, cooling to deep red
// as the slice ages, brightest toward the tip, its trailing edge torn into rising flames.
fn plasma(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let flames = trail_flames(input.world, input.color.a, 0.11);
    // The edge the flames tear: further out where the noise is high, more ragged by the tip.
    let reach = 0.3 + 0.65 * flames * mix(1.0, 0.75, along);
    let body = smoothstep(reach - 0.2, reach + 0.04, fresh);
    // Strong toward the tip, fading out well before the hilt.
    let span = (1.0 - smoothstep(0.35, 0.85, along)) * (0.4 + 0.6 * (1.0 - along));
    // A white-hot seam where the tip just passed.
    let seam = exp(-along * along * 260.0) * smoothstep(0.7, 1.0, fresh);
    let level = body * span * (0.2 + 0.8 * fresh * fresh);
    return plasma_colour(tint, fresh) * level * 0.7 + vec3(1.0, 0.92, 0.75) * seam * 0.6;
}

// The flare (style 6): the plasma, and past the tip (V below 0, `saber_trail::FLARE_REACH`)
// flames and thin magnetic loops licking off it, as prominences off the Sun's limb.
fn flare(input: VertexOutput, tint: vec3<f32>, past: f32) -> vec3<f32> {
    if input.uv.y >= 0.0 {
        return plasma(input, tint);
    }
    // 0 at the tip to 1 at the slice's far edge.
    let up = clamp(input.uv.y / min(past, -0.0001), 0.0, 1.0);
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let tongues = trail_flames(input.world + vec3(13.0, 4.0, 9.0), input.color.a, 0.16);
    let lick = smoothstep(up - 0.12, up + 0.04, tongues * 1.15 - 0.12);
    // Thin loops where a second noise crosses its middle.
    let loops_noise = trail_flames(input.world * 0.6 + vec3(2.0, 31.0, 5.0), input.color.a * 0.5, 0.2);
    let loops = (1.0 - smoothstep(0.0, 0.045, abs(loops_noise - 0.5))) * (1.0 - up * 0.7);
    let level = (lick * (1.0 - up) * 0.9 + loops * 0.6) * smoothstep(0.0, 0.5, fresh);
    return plasma_colour(tint, fresh * (1.0 - 0.45 * up)) * level;
}

// A colour of a trail taken to the blade's own hue (and an RGB's): its brightness and whiteness stay.
fn trail_tinted(c: vec3<f32>, tint: vec3<f32>) -> vec3<f32> {
    let high = max(c.r, max(c.g, c.b));
    let low = min(c.r, min(c.g, c.b));
    return mix(tint * high, vec3(high), low / max(high, 0.001));
}

// The scythe (style 10, the Trayus Scythe blade's): a ribbon of pink light, a white seam where the tip
// just passed, glitter twinkling through it (cells in the slice's world position, so the sparks
// stay put as the ribbon fades), its trailing edge torn into wisps, cooling to violet as it ages.
fn scythe_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let wisps = trail_flames(input.world * 0.8 + vec3(7.0, 2.0, 11.0), input.color.a, 0.13);
    let reach = 0.25 + 0.6 * wisps;
    let body = smoothstep(reach - 0.25, reach + 0.05, fresh);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    let cell = floor(input.world * 1.6);
    let pick = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.547);
    let local = fract(input.world * 1.6) - vec3(0.5);
    let glint = step(0.86, pick) * exp(-dot(local, local) * 40.0) * smoothstep(0.1, 0.6, fresh);
    let violet = vec3(0.42, 0.08, 0.62);
    let colour = mix(violet, mix(tint, vec3(1.0, 0.82, 0.94), smoothstep(0.75, 1.0, fresh)), smoothstep(0.1, 0.6, fresh));
    let level = body * span * (0.25 + 0.75 * fresh);
    return colour * level * 0.75 + vec3(1.0, 0.95, 1.0) * (seam * 0.7 + glint * span * 1.2);
}

// The wolf spirit (style 11, the Vulptex Crystalfire blade's): red-orange spirit flame, gold where the
// tip just passed, its trailing edge boiling into round bubbles (cells in the slice's world
// position) that swell, pop and fade as the ribbon ages.
fn foxfire_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let flames = trail_flames(input.world * 0.9 + vec3(3.0, 8.0, 1.0), input.color.a, 0.12);
    let reach = 0.3 + 0.6 * flames;
    let body = smoothstep(reach - 0.2, reach + 0.05, fresh);
    let span = (1.0 - smoothstep(0.4, 0.9, along)) * (0.5 + 0.5 * (1.0 - along));
    let cell = floor(input.world * 0.9);
    let pick = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.547);
    let local = fract(input.world * 0.9) - vec3(0.5);
    let size = 0.12 + 0.25 * (1.0 - fresh);
    let ring = abs(length(local) - size);
    let bubble = step(0.7, pick) * exp(-ring * ring * 900.0) * smoothstep(0.0, 0.3, fresh) * (1.0 - smoothstep(0.5, 0.9, fresh));
    let colour = mix(vec3(0.55, 0.04, 0.02), mix(tint, vec3(1.0, 0.82, 0.35), smoothstep(0.7, 1.0, fresh)), smoothstep(0.05, 0.55, fresh));
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    return colour * body * span * (0.3 + 0.7 * fresh) * 0.8 + vec3(1.0, 0.7, 0.3) * bubble * span * 0.9
        + vec3(1.0, 0.9, 0.6) * seam * 0.6;
}

// The storm (style 12, the Zeffo Thunderchorus blade's): a white-hot seam where the tip passed and
// jagged cyan lightning forking back from it across the swept band, in lanes, each fork's zigzag
// drawn from noise along the slice, flickering out as it ages.
fn storm(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let lanes = 6.0;
    let lane = floor(along * lanes);
    let flick = step(0.35, fract(sin(lane * 17.1 + floor(input.color.a * 24.0)) * 43758.547));
    let zig = trail_noise(vec3(input.uv.x * 9.0, lane * 3.7, input.color.a * 6.0)) - 0.5;
    let centre = (lane + 0.5 + zig * 0.8) / lanes;
    let aa = max(fwidth(along), 0.0005);
    let bolt = exp(-pow((along - centre) / (aa * 1.5 + 0.004), 2.0)) * flick * smoothstep(0.15, 0.6, fresh);
    let haze = exp(-pow((along - centre) / 0.05, 2.0)) * flick * smoothstep(0.2, 0.8, fresh) * 0.35;
    let seam = exp(-along * along * 180.0) * smoothstep(0.55, 1.0, fresh);
    let reach = 1.0 - smoothstep(0.6, 0.95, along);
    return (vec3(0.9, 1.0, 1.0) * bolt + tint * haze) * reach + vec3(0.85, 0.97, 1.0) * seam * 0.9
        + tint * smoothstep(0.5, 1.0, fresh) * (1.0 - along) * 0.15;
}

// The erasure (style 13, the Moraband Erasure blade's): a deep violet ribbon that crumbles from its
// trailing edge into motes of light as it ages, as if erased; violet-white where the tip passed.
fn erasure(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let crumble = trail_noise(input.world * 0.35 + vec3(1.0, 5.0, 2.0));
    let body = smoothstep(0.35, 0.5, fresh + 0.35 * (crumble - 0.5) * (1.0 - fresh) * 2.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let cell = floor(input.world * 1.3);
    let pick = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.547);
    let local = fract(input.world * 1.3) - vec3(0.5);
    let mote = step(0.8, pick) * exp(-dot(local, local) * 60.0) * (1.0 - body) * smoothstep(0.05, 0.4, fresh);
    let seam = exp(-along * along * 200.0) * smoothstep(0.6, 1.0, fresh);
    let colour = mix(vec3(0.12, 0.0, 0.2), tint, smoothstep(0.3, 0.8, fresh));
    return colour * body * span * 0.85 + vec3(0.85, 0.7, 1.0) * mote * span * 1.1 + vec3(0.95, 0.88, 1.0) * seam * 0.6;
}

// Ragnos (style 14, the Korriban Ragnos blade's): the swept band darkened to black (the trail
// blends additively, so the black is a deep red-black glow, darkest where it is oldest), a crimson
// edge where the tip just passed, and thin red cracks of light (where a noise crosses its middle)
// flickering through it as it fades.
fn ragnos_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let crack_noise = trail_noise(input.world * 0.5 + vec3(3.0, 9.0, 4.0) + vec3(0.0, 0.0, input.color.a * 1.5));
    let crack = (1.0 - smoothstep(0.0, 0.04, abs(crack_noise - 0.5))) * smoothstep(0.1, 0.6, fresh);
    let seam = exp(-along * along * 200.0) * smoothstep(0.6, 1.0, fresh);
    let body = smoothstep(0.2, 0.7, fresh);
    let crimson = trail_tinted(vec3(0.9, 0.05, 0.08), tint);
    return trail_tinted(vec3(0.12, 0.0, 0.02), tint) * body * span + crimson * crack * span * 0.9 + trail_tinted(vec3(1.0, 0.25, 0.25), tint) * seam * 0.7;
}

// Glints in a trail: round points of light in cells of the slice's world position (each kept well
// inside its cell, so no cell edge shows), `density` of the cells lit.
fn trail_glints(world: vec3<f32>, scale: f32, density: f32) -> f32 {
    let cell = floor(world * scale);
    let pick = fract(sin(dot(cell, vec3(12.9898, 78.233, 37.719))) * 43758.547);
    let local = fract(world * scale) - vec3(0.5);
    return step(1.0 - density, pick) * exp(-dot(local, local) * 60.0);
}

// Infinite (style 15): blue flame, white where the tip just passed, glints twinkling.
fn infinite_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let flames = trail_flames(input.world * 0.8 + vec3(2.0, 6.0, 3.0), input.color.a, 0.13);
    let reach = 0.25 + 0.6 * flames;
    let body = smoothstep(reach - 0.25, reach + 0.05, fresh);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    let glint = trail_glints(input.world, 1.5, 0.15) * smoothstep(0.1, 0.6, fresh);
    // Update 26: as the blade's surge turns it red, its deep and pale tones turn too.
    let red = smoothstep(0.5, 1.0, tint.r - tint.b);
    let colour = mix(mix(vec3(0.05, 0.2, 0.7), vec3(0.6, 0.02, 0.04), red), mix(tint, mix(vec3(0.8, 0.95, 1.0), vec3(1.0, 0.75, 0.65), red), smoothstep(0.75, 1.0, fresh)), smoothstep(0.1, 0.6, fresh));
    return colour * body * span * (0.3 + 0.7 * fresh) * 0.8 + vec3(0.9, 0.97, 1.0) * (seam * 0.7 + glint * span);
}

// The forge (style 16): molten metal, white-hot where the tip just passed, cooling to dark red
// with glowing cracks.
fn forge(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let heat = fresh * fresh;
    let colour = mix(trail_tinted(vec3(0.25, 0.02, 0.0), tint), mix(trail_tinted(vec3(1.0, 0.35, 0.05), tint), trail_tinted(vec3(1.0, 0.92, 0.7), tint), smoothstep(0.6, 1.0, heat)), smoothstep(0.1, 0.6, heat));
    let crack_noise = trail_noise(input.world * 0.6 + vec3(4.0, 1.0, 7.0));
    let crack = (1.0 - smoothstep(0.0, 0.05, abs(crack_noise - 0.5))) * smoothstep(0.0, 0.5, 1.0 - fresh) * smoothstep(0.0, 0.3, fresh);
    let body = smoothstep(0.1, 0.5, fresh);
    return colour * body * span * 0.85 + trail_tinted(vec3(1.0, 0.45, 0.08), tint) * crack * span * 0.8;
}

// The sand (style 17): golden sand streaming back, grains glinting, thinning as it fades.
fn sand(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let streams = trail_flames(input.world * 1.4 + vec3(9.0, 3.0, 5.0), input.color.a * 0.3, 0.2);
    let body = smoothstep(0.3, 0.7, streams + fresh * 0.6) * smoothstep(0.05, 0.5, fresh);
    let grains = trail_glints(input.world, 3.0, 0.3) * smoothstep(0.0, 0.5, fresh);
    let colour = mix(vec3(0.45, 0.28, 0.08), mix(tint, vec3(1.0, 0.9, 0.6), smoothstep(0.7, 1.0, fresh)), smoothstep(0.1, 0.6, fresh));
    return colour * body * span * 0.7 + vec3(1.0, 0.9, 0.6) * grains * span * 0.9;
}

// Vitiate (style 18): magenta and violet with a dark heart, its edge torn into rising wisps.
fn vitiate_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let wisps = trail_flames(input.world * 0.7 + vec3(1.0, 9.0, 4.0), input.color.a, 0.14);
    let reach = 0.2 + 0.7 * wisps;
    let body = smoothstep(reach - 0.25, reach + 0.05, fresh);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let heart = exp(-pow((along - 0.3) / 0.15, 2.0)) * smoothstep(0.3, 0.8, fresh);
    let seam = exp(-along * along * 200.0) * smoothstep(0.6, 1.0, fresh);
    let colour = mix(vec3(0.3, 0.02, 0.4), mix(tint, vec3(1.0, 0.6, 0.95), smoothstep(0.75, 1.0, fresh)), smoothstep(0.1, 0.6, fresh));
    return colour * body * span * (1.0 - 0.6 * heart) * 0.85 + vec3(1.0, 0.85, 1.0) * seam * 0.7;
}

// Equilibrium (style 19): a thin silver ribbon, a white seam, a cool shimmer running through it.
fn equilibrium_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.35, 0.85, along)) * (0.5 + 0.5 * (1.0 - along));
    let shimmer = trail_flames(input.world * 1.2 + vec3(5.0, 2.0, 8.0), input.color.a * 0.5, 0.2);
    let seam = exp(-along * along * 260.0) * smoothstep(0.5, 1.0, fresh);
    let body = smoothstep(0.2, 0.8, fresh) * (0.5 + 0.5 * shimmer);
    return mix(trail_tinted(vec3(0.5, 0.6, 0.8), tint), trail_tinted(vec3(0.95, 0.97, 1.0), tint), fresh) * body * span * 0.6 + vec3(1.0) * seam * 0.8;
}

// The vortex (style 20): a deep blue band whose light swirls round as it fades.
fn vortex(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let swirl = 0.5 + 0.5 * sin(input.uv.x * 14.0 + along * 9.0 - input.color.a * 6.0 + 3.0 * trail_noise(input.world * 0.3));
    let body = smoothstep(0.1, 0.6, fresh);
    let seam = exp(-along * along * 200.0) * smoothstep(0.6, 1.0, fresh);
    let colour = mix(vec3(0.02, 0.03, 0.2), tint, smoothstep(0.3, 0.9, fresh));
    return colour * body * span * (0.4 + 0.6 * swirl * swirl) + vec3(0.7, 0.85, 1.0) * seam * 0.6;
}

// The starlight (style 21): white-gold full of twinkling stars, fading to soft blue.
fn starlight(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let stars = trail_glints(input.world, 2.2, 0.22) * smoothstep(0.05, 0.5, fresh);
    let body = smoothstep(0.2, 0.8, fresh);
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    let colour = mix(trail_tinted(vec3(0.3, 0.45, 0.9), tint), trail_tinted(vec3(1.0, 0.95, 0.8), tint), smoothstep(0.5, 1.0, fresh));
    return colour * body * span * 0.55 + trail_tinted(vec3(1.0, 0.97, 0.9), tint) * (stars * span * 1.2 + seam * 0.7);
}

// The convergence (style 22): red on the swing's leading half, blue on its trailing half, both
// running into a deep violet as it ages, a white seam where the tip passed.
fn convergence(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let side = smoothstep(0.3, 0.7, along + 0.15 * trail_noise(input.world * 0.4));
    let young = mix(trail_tinted(vec3(1.0, 0.15, 0.2), tint), trail_tinted(vec3(0.2, 0.4, 1.0), tint), side);
    let colour = mix(trail_tinted(vec3(0.5, 0.1, 0.9), tint), young, smoothstep(0.35, 0.9, fresh));
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    return colour * smoothstep(0.1, 0.7, fresh) * span * 0.65 + trail_tinted(vec3(1.0, 0.92, 1.0), tint) * seam * 0.7;
}

// The shadowflame (style 23): fire where the blade just passed, turning to dark smoke shot with
// embers (dimming what it covers: it adds almost nothing) as it fades.
fn shadowflame(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    // Update 24 (Creyon: Byss's trail was not right; it was orange fire): the storm's own colour, a dark
    // turning band with blue-white Force lightning forking through it.
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let swirl = trail_flames(input.world * 0.6, input.color.a * 0.5, 0.2);
    let band = smoothstep(0.15, 0.85, fresh) * (0.25 + 0.75 * swirl);
    let zig = abs(fract(input.uv.x * 7.0 + trail_noise(input.world * 0.7) * 2.5) - 0.5);
    let bolt = exp(-pow((along - 0.08 - zig * 0.5) / 0.022, 2.0)) * smoothstep(0.35, 0.8, fresh);
    let seam = exp(-along * along * 240.0) * smoothstep(0.7, 1.0, fresh);
    return tint * band * span * 0.62 + vec3(0.75, 0.85, 1.0) * bolt * span * 1.2 + mix(tint, vec3(1.0), 0.7) * seam * 0.6;
}

// Bone line (style 28, Tatooine Krayt Spine): pale sand blown back in a sheet, with notches of bone
// along the edge the tip cut and grains falling out of it.
fn bone_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.4, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let sheet = trail_noise(input.world * vec3(0.5, 0.5, 0.5)) * smoothstep(0.0, 0.8, fresh);
    let grains = trail_glints(input.world, 2.6, 0.25) * smoothstep(0.0, 0.5, fresh);
    let notch = smoothstep(0.55, 0.8, 0.5 + 0.5 * sin(input.uv.x * 34.0)) * exp(-pow(along / 0.05, 2.0)) * smoothstep(0.35, 0.9, fresh);
    let sand = mix(vec3(0.75, 0.55, 0.28), tint, 0.35);
    return sand * sheet * span * 0.6 + vec3(1.0, 0.86, 0.55) * grains * span * 0.8 + vec3(0.98, 0.93, 0.78) * notch;
}

// Hunger (style 29, Nihilus): almost no light at all: a thin burning edge and pale motes of stolen life
// being pulled along it toward the blade.
fn hunger_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = 1.0 - smoothstep(0.5, 0.98, along);
    let seam = exp(-along * along * 500.0) * smoothstep(0.5, 1.0, fresh);
    let drawn = trail_glints(input.world + vec3(0.0, 0.0, input.color.a * 2.0), 1.7, 0.16) * smoothstep(0.0, 0.5, fresh);
    let ember = tint * smoothstep(0.55, 1.0, fresh) * exp(-along * 7.0) * 0.45;
    return tint * seam * 1.25 + mix(vec3(0.75, 0.88, 1.0), tint, smoothstep(0.6, 1.0, fresh)) * drawn * span * 0.8 + ember;
}

// Kathol (style 34, the Kathol Rift blade's): a ribbon twisted along the swing, night violet and gold changing
// places down it, pinching thin where they cross and burning there, unwinding as it fades.
fn kathol_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let phase = input.uv.x * 16.0 + input.color.a * 3.0;
    let turn = cos(phase);
    let width = (0.16 + 0.84 * abs(turn)) * (0.4 + 0.6 * fresh);
    let band = 1.0 - smoothstep(width * 0.72, width, along);
    let face = 0.5 + 0.5 * turn / (abs(turn) + 0.18);
    let colour = mix(tint * 0.75, mix(tint, vec3(1.0), 0.6), face);
    let burn = 1.0 - smoothstep(0.0, 0.3, abs(turn));
    let seam = exp(-along * along * 400.0) * smoothstep(0.5, 1.0, fresh);
    return (colour + mix(tint, vec3(1.0), 0.7) * burn * 0.8) * band * smoothstep(0.0, 0.45, fresh) * 0.75 + mix(tint, vec3(1.0), 0.75) * seam;
}

// Temple (style 35, Exar Kun Temple): a violet band with gold seams wandering through it.
fn temple_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = 1.0 - smoothstep(0.55, 1.0, along);
    let n = trail_noise(input.world * 0.45 + vec3(input.color.a));
    let seam = exp(-pow((n - 0.5) / 0.05, 2.0));
    return (tint * 0.55 * smoothstep(0.0, 0.55, fresh) + vec3(1.0, 0.78, 0.34) * seam * smoothstep(0.1, 0.7, fresh)) * span;
}

// Ash (style 36, Ziost Silence): a pale band crumbling from its old end into flakes.
fn ash_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = 1.0 - smoothstep(0.5, 1.0, along);
    let crumble = smoothstep(1.0 - fresh - 0.15, 1.0 - fresh + 0.15, trail_noise(input.world * 0.9));
    let flakes = trail_glints(input.world, 1.1, 0.14) * (1.0 - fresh) * 0.9;
    return tint * (0.5 * crumble * fresh + flakes) * span;
}

// Rust (style 37, Nathema Deadworld): a dull low band with embers dying in it.
fn rust_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = 1.0 - smoothstep(0.45, 0.95, along);
    let embers = trail_glints(input.world * vec3(1.0, 1.0, 0.5), 1.3, 0.12) * smoothstep(0.0, 0.5, fresh);
    return tint * (0.22 * fresh * fresh + embers * 1.1) * span;
}

// Neon (style 38, Taris Undercity): two thin tubes of light; the teal one stutters along its length.
fn neon_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let orange = exp(-pow((along - 0.12) / 0.05, 2.0));
    let stutter = step(0.32, trail_noise(vec3(floor(input.uv.x * 24.0), input.color.a * 9.0, 2.0)));
    let teal = exp(-pow((along - 0.42) / 0.05, 2.0)) * stutter;
    return (tint * orange + vec3(0.1, 0.95, 0.98) * teal) * smoothstep(0.0, 0.5, fresh) * 1.1;
}

// Hush (style 39, Ysalamir Hush): a very soft breath of light with no edge at all.
fn hush_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    return tint * 0.42 * (1.0 - along) * (1.0 - along) * fresh * fresh;
}

// Blood (style 40, Naga Sadow Alchemy): a thick band sagging into fat drops as it ages.
fn blood_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let sag = 0.5 + 0.4 * trail_noise(vec3(input.uv.x * 8.0, input.color.a * 5.0, 1.0)) * (1.0 - fresh);
    let band = 1.0 - smoothstep(sag - 0.2, sag + 0.1, along);
    let drops = trail_glints(input.world * vec3(1.0, 1.0, 0.4), 1.1, 0.1) * (1.0 - fresh * 0.6);
    return tint * band * smoothstep(0.0, 0.5, fresh) * 0.85 + mix(tint, vec3(1.0), 0.4) * drops * (1.0 - along);
}

// Carapace (style 41, Orbalisk Carapace): burning cells that crack apart as they fade.
fn carapace_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let n = trail_noise(input.world * 0.8);
    let seam = exp(-pow((n - 0.5) / (0.03 + 0.1 * fresh), 2.0));
    return tint * seam * (1.0 - smoothstep(0.55, 1.0, along)) * smoothstep(0.0, 0.5, fresh) * 1.2;
}

// Serpent (style 42, Amphistaff Serpent): a narrow band wriggling more and more as it fades.
fn serpent_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let centre = 0.32 + 0.2 * (1.0 - fresh) * sin(input.uv.x * 26.0 + input.color.a * 6.0);
    let band = exp(-pow((along - centre) / (0.1 + 0.08 * fresh), 2.0));
    return mix(tint, vec3(1.0), 0.25) * band * smoothstep(0.0, 0.4, fresh) * 1.1;
}

// Nebula (style 43, Vergence Scatter): a soft cloud of blue and gold with stars in it.
fn nebula_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let n = trail_noise(input.world * 0.3 + vec3(input.color.a));
    let cloud = smoothstep(0.25, 0.75, n) * (1.0 - along) * smoothstep(0.0, 0.6, fresh);
    let stars = trail_glints(input.world, 1.6, 0.16) * smoothstep(0.0, 0.5, fresh);
    return mix(tint, vec3(1.0, 0.76, 0.35), smoothstep(0.4, 0.75, n)) * cloud * 0.7 + vec3(1.0) * stars * (1.0 - along);
}

// Sonic (style 44, Voxyn Scream): bands of sound running along the swing.
fn sonic_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let wave = pow(0.5 + 0.5 * sin(input.uv.x * 42.0 + along * 3.0), 3.0);
    return mix(tint, vec3(0.28, 1.0, 0.45), 1.0 - fresh) * wave * (1.0 - along) * smoothstep(0.0, 0.5, fresh);
}

// Plague (style 45, Rakghoul Plague): a sick band with spots of pus that outlast it.
fn plague_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let spots = trail_glints(input.world, 1.2, 0.14) * (1.0 - along);
    let sick = smoothstep(0.3, 0.72, trail_noise(input.world * 0.6)) * (1.0 - smoothstep(0.4, 0.95, along));
    return tint * sick * fresh * 0.7 + vec3(1.0, 0.96, 0.5) * spots;
}

// Growth (style 46, Telos Restoration): a white band with green coming into it as it ages.
fn growth_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let leaf = smoothstep(0.4, 0.68, trail_noise(input.world * 0.8));
    return mix(vec3(0.9, 1.0, 0.85), tint, smoothstep(0.75, 0.2, fresh)) * (0.3 + 0.7 * leaf) * (1.0 - along) * smoothstep(0.0, 0.45, fresh);
}

// Ion (style 47, Malevolence Ion Beam): thin filaments that flicker along the swing.
fn ion_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let w = trail_noise(vec3(input.uv.x * 11.0, along * 9.0, input.color.a * 8.0));
    let line = exp(-pow((w - 0.5) / 0.035, 2.0));
    return mix(tint, vec3(1.0), 0.5) * line * (1.0 - smoothstep(0.55, 1.0, along)) * smoothstep(0.0, 0.5, fresh) * 1.2;
}

// Cinder (style 48, Massassi Altar): dark smoke with embers caught in it.
fn cinder_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let smoke = smoothstep(0.3, 0.8, trail_noise(input.world * 0.5 + vec3(0.0, 0.0, input.color.a)));
    let embers = trail_glints(input.world * vec3(1.0, 1.0, 0.5), 1.4, 0.15) * smoothstep(0.0, 0.6, fresh);
    return tint * 0.18 * smoke * fresh * (1.0 - along) + mix(tint, vec3(1.0, 0.86, 0.5), 0.5) * embers * (1.0 - along);
}

// Tremor (style 49, Sith Tremor Sword): a band shaken into bars that die quickly.
fn tremor_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let bars = smoothstep(0.1, 0.9, sin(input.uv.x * 70.0 + along * 5.0));
    let seam = exp(-along * along * 240.0);
    return tint * bars * (1.0 - along) * fresh * fresh * 0.9 + mix(tint, vec3(1.0), 0.5) * seam * smoothstep(0.55, 1.0, fresh);
}

// Press (style 50, Wayland Gravity Press): a heavy violet band sagging down as it ages, gold grains sinking from it.
fn press_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let sag = along - 0.32 * (1.0 - fresh);
    let band = exp(-sag * sag / 0.09) * smoothstep(0.0, 0.5, fresh);
    let grains = trail_glints(input.world * vec3(1.0, 1.0, 0.45), 1.2, 0.12) * smoothstep(0.0, 0.5, fresh);
    return tint * band * 0.85 + vec3(1.0, 0.78, 0.4) * grains * (1.0 - along);
}

// Cloud (style 51, the Angel blade): billows of soft white mist hanging where the blade passed, thinning slowly from their edges.
fn cloud_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let p = input.world * 0.2 + vec3(0.0, 0.0, input.color.a);
    let puffs = 0.65 * trail_noise(p) + 0.35 * trail_noise(p * 2.3 + vec3(5.0));
    let thin = 0.56 - 0.5 * fresh;
    let body = smoothstep(thin, thin + 0.34, puffs) * (1.0 - smoothstep(0.7, 1.0, along));
    return mix(tint * 0.74, tint, smoothstep(0.42, 0.78, puffs)) * body * smoothstep(0.0, 0.55, fresh);
}

// Hex (style 30, Nightsister Ichor): a thick glowing sap that hangs in the air, its fat drops sagging out
// of the band as it fades.
fn hex_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.5, 0.95, along)) * (0.45 + 0.55 * (1.0 - along));
    let sap = smoothstep(0.32, 0.72, trail_noise(input.world * 0.7 + vec3(0.0, 0.0, input.color.a))) * smoothstep(0.1, 0.8, fresh);
    let drops = trail_glints(input.world * vec3(1.0, 1.0, 0.35), 1.2, 0.1) * smoothstep(0.0, 0.5, fresh);
    let seam = exp(-along * along * 160.0) * smoothstep(0.65, 1.0, fresh);
    return tint * sap * span * 0.75 + mix(tint, vec3(1.0), 0.45) * (drops * span * 1.1 + seam * 0.7);
}

// Gilded (style 31, Ossus Scroll): one long stroke of a brush full of gold, its bristles showing as it dries.
fn gilded_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.42, 0.9, along)) * (0.6 + 0.4 * (1.0 - along));
    let bristle = 0.55 + 0.45 * sin(along * 90.0 + 2.5 * trail_noise(input.world * 0.25));
    let dry = smoothstep(fresh * 1.15 - 0.1, fresh * 1.15 + 0.1, 0.25 + 0.75 * bristle * trail_noise(input.world * 0.4));
    let gold = mix(trail_tinted(vec3(0.9, 0.58, 0.14), tint), trail_tinted(vec3(1.0, 0.92, 0.72), tint), smoothstep(0.55, 1.0, fresh));
    let flecks = trail_glints(input.world, 2.2, 0.1) * smoothstep(0.0, 0.5, fresh);
    return gold * (1.0 - dry * 0.75) * smoothstep(0.02, 0.5, fresh) * span * 0.85 + trail_tinted(vec3(1.0, 0.9, 0.6), tint) * flecks * span * 0.7;
}

// Gale (style 32, Utapau Whirlwind): air, not light: fine streaks combed along the swing with dust in them.
fn gale_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.5, 0.98, along)) * (0.55 + 0.45 * (1.0 - along));
    let wave = along * 26.0 + 2.0 * sin(input.uv.x * 5.0 + along * 7.0);
    let combed = smoothstep(0.35, 1.0, 0.5 + 0.5 * sin(wave)) * smoothstep(0.0, 0.7, fresh) * (0.5 + 0.5 * trail_noise(input.world * 0.3));
    let dust = trail_glints(input.world, 2.8, 0.14) * smoothstep(0.0, 0.5, fresh);
    return tint * combed * span * 0.45 + vec3(0.72, 0.6, 0.42) * dust * span * 0.6;
}

// Salt (style 33, Crait Salt Flats): the red scored open behind a skim of white salt.
fn salt_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let plume = trail_flames(input.world * 0.45, input.color.a * 0.5, 0.12) * smoothstep(0.0, 0.75, fresh);
    let salt = trail_glints(input.world, 3.0, 0.3) * smoothstep(0.15, 0.7, fresh);
    let edge = exp(-along * along * 300.0) * smoothstep(0.5, 1.0, fresh);
    return tint * plume * span * 0.75 + vec3(1.0, 0.98, 0.95) * (salt * span * 0.8 + edge * 0.9);
}

// Sorcery (style 24): gold with dark beads strung along it, fading to white.
fn sorcery_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let beads = smoothstep(0.75, 0.95, 0.5 + 0.5 * sin(input.uv.x * 30.0)) * exp(-pow((along - 0.3) / 0.08, 2.0));
    let colour = mix(trail_tinted(vec3(1.0, 0.95, 0.85), tint), trail_tinted(vec3(1.0, 0.75, 0.25), tint), smoothstep(0.3, 0.9, fresh));
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    return colour * smoothstep(0.1, 0.7, fresh) * span * 0.6 * (1.0 - 0.8 * beads) + vec3(1.0) * seam * 0.6;
}

// Wildheart (style 25): green with jagged lightning crawling across it.
fn wildheart_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let zig = abs(fract(input.uv.x * 9.0 + trail_noise(input.world * 0.5) * 2.0) - 0.5);
    let bolt = exp(-pow((along - 0.25 - zig * 0.4) / 0.03, 2.0)) * smoothstep(0.3, 0.8, fresh);
    let seam = exp(-along * along * 200.0) * smoothstep(0.6, 1.0, fresh);
    return trail_tinted(vec3(0.3, 1.0, 0.25), tint) * smoothstep(0.1, 0.7, fresh) * span * 0.55 + trail_tinted(vec3(0.85, 1.0, 0.8), tint) * (bolt * span + seam * 0.6);
}

// Darksaber (style 26): a dark band (adding almost no light) with a crackling white edge.
fn darksaber(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let crackle = 0.6 + 0.4 * trail_noise(input.world * 2.0);
    let edge = exp(-along * along * 160.0) * smoothstep(0.4, 1.0, fresh) * crackle;
    return trail_tinted(vec3(0.08, 0.08, 0.1), tint) * smoothstep(0.2, 0.8, fresh) * span + trail_tinted(vec3(0.95, 0.97, 1.0), tint) * edge * 0.9;
}

// Thoughtbomb (style 27): silver-white pale flame with a gold seam.
fn thoughtbomb_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.45, 0.95, along)) * (0.5 + 0.5 * (1.0 - along));
    let flame = trail_flames(input.world * 0.9, input.color.a * 0.6, 0.25);
    let seam = exp(-along * along * 220.0) * smoothstep(0.6, 1.0, fresh);
    return mix(trail_tinted(vec3(0.6, 0.75, 1.0), tint), trail_tinted(vec3(0.95, 0.97, 1.0), tint), flame) * smoothstep(0.15, 0.8, fresh) * span * 0.6 + trail_tinted(vec3(1.0, 0.85, 0.4), tint) * seam * 0.7;
}

// A hashed cell of the world, 0 to 1 (the moonlight's glitter).
fn trail_hash3(p: vec3<f32>) -> vec3<f32> {
    return vec3(trail_hash(p), trail_hash(p + vec3(17.1, 3.7, 9.2)), trail_hash(p + vec3(5.3, 29.4, 13.8)));
}

// Glitter laid in the world: one speck in some cells, twinkling, so it runs on unbroken from one
// slice to the next and seems to hang where the blade passed. `keep` is the share of cells lit.
fn moon_glitter(world: vec3<f32>, age: f32, scale: f32, keep: f32) -> f32 {
    // drifting slowly down as it ages: moonlight falls where the Sun's flames rise
    let p = world * scale + vec3(0.0, 0.0, age * 1.6);
    let cell = floor(p);
    let h = trail_hash3(cell);
    if h.x > keep {
        return 0.0;
    }
    let at = cell + 0.2 + 0.6 * h;
    let d = length(p - at);
    let twinkle = 0.5 + 0.5 * sin(age * 40.0 + h.y * 60.0);
    return exp(-d * d * 40.0) * (0.4 + 0.6 * twinkle);
}

// The moonlight's colour as it fades (`fresh` 1 just left, 0 gone): silver white, the trail's
// own colour (Creyon's lilac, or the wearer's colour on a chroma), then a deep violet going dark.
fn moonlight_colour(tint: vec3<f32>, fresh: f32) -> vec3<f32> {
    let silver = vec3(0.93, 0.94, 1.0);
    let bright = mix(tint, silver, 0.6);
    let deep = tint * vec3(0.45, 0.3, 0.75);
    let cool = mix(deep, tint, smoothstep(0.3, 0.7, fresh));
    return mix(cool, bright, smoothstep(0.8, 1.0, fresh));
}

// The moonlight (style 7, the Moon blade's, the Sun's plasma's night sister): a soft silver
// ribbon where the blade just passed, light rippling across it as moonlight on water, cooling to
// lilac and deep violet as it ages, its trailing edge dissolving into glittering stardust that
// drifts down and twinkles out.
fn moonlight(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    // ripples of light across it, laid in the world
    let ripple = trail_flames(input.world, input.color.a * 0.6, 0.09);
    let water = 0.65 + 0.7 * smoothstep(0.55, 0.8, ripple);
    // the ribbon fades out smoothly toward the hilt and as it ages; its old edge is soft
    let span = (1.0 - smoothstep(0.35, 0.85, along)) * (0.4 + 0.6 * (1.0 - along));
    let body = smoothstep(0.1, 0.55, fresh) * span * (0.25 + 0.75 * fresh * fresh) * water;
    // stardust where the ribbon dissolves: most in its middle age, along the outer half
    let dust_age = smoothstep(0.05, 0.4, input.uv.x) * (1.0 - smoothstep(0.7, 1.0, input.uv.x));
    let dust = (moon_glitter(input.world, input.color.a, 0.32, 0.22)
        + moon_glitter(input.world + vec3(7.0, 3.0, 1.0), input.color.a, 0.55, 0.14) * 0.7)
        * dust_age * (1.0 - smoothstep(0.55, 0.95, along));
    // a fine silver seam where the tip just passed
    let seam = exp(-along * along * 260.0) * smoothstep(0.7, 1.0, fresh);
    return moonlight_colour(tint, fresh) * body * 0.85
        + mix(vec3(1.0), tint, 0.35) * dust * 1.8
        + vec3(0.92, 0.94, 1.0) * seam * 0.5;
}

// The eclipse (style 8): the moonlight's ribbon hollowed out, only a ring of light left along its
// edges, as the corona round the hidden moon, white within and the trail's colour without, and a
// diamond-ring flash where the tip just passed; stardust still drifting off it.
fn eclipse_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.4, 0.9, along));
    // the ribbon's outline: near the tip's edge (along 0) and the slice's newest edge
    // (fresh 1), and the hilt-side edge where the ribbon fades out
    let edge_tip = exp(-along * along * 90.0);
    let edge_new = exp(-pow((1.0 - fresh) / 0.06, 2.0));
    let flick = trail_flames(input.world, input.color.a, 0.12);
    let edge_old = exp(-pow((fresh - (0.25 + 0.3 * flick)) / 0.05, 2.0)) * 0.7;
    let ring = max(max(edge_tip, edge_new), edge_old) * span * smoothstep(0.1, 0.5, fresh);
    let corona = ring * (0.35 + 0.65 * fresh);
    let col = mix(tint, vec3(0.97, 0.96, 1.0), smoothstep(0.6, 1.0, ring));
    // the diamond ring at the tip just passed
    let diamond = exp(-(along * along * 900.0 + (1.0 - fresh) * (1.0 - fresh) * 400.0));
    let dust_age = smoothstep(0.05, 0.4, input.uv.x) * (1.0 - smoothstep(0.7, 1.0, input.uv.x));
    let dust = moon_glitter(input.world, input.color.a, 0.32, 0.18) * dust_age * span;
    return col * corona * 0.9 + vec3(1.0) * diamond * 1.2 + tint * dust * 1.2;
}

// The starfall (style 9): the moonlight's ribbon faint, and shooting stars streaking out of it
// and falling away from the swing, each a bright head with a fading silver tail, laid in the world
// in lanes across the swept band so they run on from slice to slice.
fn starfall_trail(input: VertexOutput, tint: vec3<f32>) -> vec3<f32> {
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    let span = (1.0 - smoothstep(0.4, 0.9, along));
    let ribbon = moonlight(input, tint) * 0.45;
    // lanes across the band (by along), each with a star whose head sits at its own age
    let lanes = 9.0;
    let lane = floor(along * lanes);
    let x = fract(along * lanes) - 0.5;
    let h = trail_hash(vec3(lane, 3.1, 7.7));
    if h > 0.6 {
        return ribbon;
    }
    let head = 0.1 + 0.6 * trail_hash(vec3(lane, 9.2, 1.3));
    let behind = input.uv.x - head;
    // a head at `head` and a tail streaming back toward the newer side
    let tail = select(0.0, exp(behind * 5.0), behind < 0.0) * smoothstep(-0.6, 0.0, behind);
    let core = exp(-x * x * 120.0);
    let dot_ = exp(-(behind * behind * 900.0 + x * x * 160.0));
    let star = (core * tail * 0.6 + dot_ * 1.6) * span * smoothstep(0.0, 0.25, fresh + 0.2);
    return ribbon + mix(vec3(0.95, 0.96, 1.0), tint, 0.3 * (1.0 - fresh)) * star;
}

// Both stock materials blend GL_ONE GL_ONE, so only rgb matters here (style 3, a
// blade skin's blur trail, is SJK's own):
// `saberTrail` = blurglow (rgbGen vertex) + blurcore (rgbGen identity),
// `swordTrail` = swordtrail (rgbGen vertex).
// Every trail of the round ends softly along the tip's own path: a bright hard edge there showed
// as a thin line standing past the tip in a swing (Creyon, 4/11/2026).
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let c = trail_colour(input);
    if (input.style & STYLE) >= 10u {
        return vec4(c.rgb * smoothstep(0.0, 0.16, clamp(input.uv.y, 0.0, 1.0)), c.a);
    }
    return c;
}

fn trail_colour(input: VertexOutput) -> vec4<f32> {
    let style = input.style & STYLE;
    if style == 2u { discard; }
    if style == 1u {
        let sword = textureSample(sword_trail, trail_sampler, input.uv);
        return vec4(sword.rgb * input.color.rgb, 1.0);
    }
    let glow = textureSample(blur_glow, trail_sampler, input.uv);
    let core = textureSample(blur_core, trail_sampler, input.uv);
    // A blade skin (style 3): glow and core both take the skin's trail colour, deepened
    // (normalised to its brightest channel, then squared) so a pale colour does not wash
    // out to white once the two add up.
    // The Clone Wars blade (style 4, Creyon's round 19): comic speed lines streaking back
    // along the swing instead of the blur, each in its own lane across the swept band with its
    // own length, the blade's colour round a white core, and a bold line where the tip went;
    // they draw back toward the blade as the trail fades.
    if style == 5u || style == 6u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 5u {
            return vec4(plasma(input, tint), 1.0);
        }
        return vec4(flare(input, tint, input.past), 1.0);
    }
    if style == 50u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4(press_trail(input, tint), 1.0);
    }
    if style == 51u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4(cloud_trail(input, tint), 1.0);
    }
    if style >= 45u && style <= 49u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 45u { return vec4(plague_trail(input, tint), 1.0); }
        if style == 46u { return vec4(growth_trail(input, tint), 1.0); }
        if style == 47u { return vec4(ion_trail(input, tint), 1.0); }
        if style == 48u { return vec4(cinder_trail(input, tint), 1.0); }
        return vec4(tremor_trail(input, tint), 1.0);
    }
    if style >= 40u && style <= 44u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 40u { return vec4(blood_trail(input, tint), 1.0); }
        if style == 41u { return vec4(carapace_trail(input, tint), 1.0); }
        if style == 42u { return vec4(serpent_trail(input, tint), 1.0); }
        if style == 43u { return vec4(nebula_trail(input, tint), 1.0); }
        return vec4(sonic_trail(input, tint), 1.0);
    }
    if style >= 35u && style <= 39u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 35u { return vec4(temple_trail(input, tint), 1.0); }
        if style == 36u { return vec4(ash_trail(input, tint), 1.0); }
        if style == 37u { return vec4(rust_trail(input, tint), 1.0); }
        if style == 38u { return vec4(neon_trail(input, tint), 1.0); }
        return vec4(hush_trail(input, tint), 1.0);
    }
    if style == 34u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4(kathol_trail(input, tint), 1.0);
    }
    if style >= 28u && style <= 33u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 28u { return vec4(bone_trail(input, tint), 1.0); }
        if style == 29u { return vec4(hunger_trail(input, tint), 1.0); }
        if style == 30u { return vec4(hex_trail(input, tint), 1.0); }
        if style == 31u { return vec4(gilded_trail(input, tint), 1.0); }
        if style == 32u { return vec4(gale_trail(input, tint), 1.0); }
        return vec4(salt_trail(input, tint), 1.0);
    }
    if style >= 24u && style <= 27u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 24u { return vec4(sorcery_trail(input, tint), 1.0); }
        if style == 25u { return vec4(wildheart_trail(input, tint), 1.0); }
        if style == 26u { return vec4(darksaber(input, tint), 1.0); }
        return vec4(thoughtbomb_trail(input, tint), 1.0);
    }
    if style == 22u || style == 23u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 22u { return vec4(convergence(input, tint), 1.0); }
        return vec4(shadowflame(input, tint), 1.0);
    }
    if style >= 19u && style <= 21u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 19u { return vec4(equilibrium_trail(input, tint), 1.0); }
        if style == 20u { return vec4(vortex(input, tint), 1.0); }
        return vec4(starlight(input, tint), 1.0);
    }
    if style >= 15u && style <= 18u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 15u { return vec4(infinite_trail(input, tint), 1.0); }
        if style == 16u { return vec4(forge(input, tint), 1.0); }
        if style == 17u { return vec4(sand(input, tint), 1.0); }
        return vec4(vitiate_trail(input, tint), 1.0);
    }
    if style == 14u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4(ragnos_trail(input, tint), 1.0);
    }
    if style >= 11u && style <= 13u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 11u { return vec4(foxfire_trail(input, tint), 1.0); }
        if style == 12u { return vec4(storm(input, tint), 1.0); }
        return vec4(erasure(input, tint), 1.0);
    }
    if style == 10u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4(scythe_trail(input, tint), 1.0);
    }
    if style >= 7u && style <= 9u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        if style == 7u {
            return vec4(moonlight(input, tint), 1.0);
        }
        if style == 8u {
            return vec4(eclipse_trail(input, tint), 1.0);
        }
        return vec4(starfall_trail(input, tint), 1.0);
    }
    if style == 4u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        let lanes = 14.0;
        let across = input.uv.y * lanes;
        let lane = floor(across);
        let pick = fract(sin(lane * 12.9898 + 4.1) * 43758.547);
        let reach = 0.25 + 0.6 * fract(sin(lane * 78.233 + 1.7) * 43758.547);
        let shift = (fract(sin(lane * 39.346 + 8.3) * 43758.547) - 0.5) * 0.4;
        // more of them toward the tip, where the blade moves fastest; each tapering to a point
        let shown = step(0.25 + 0.5 * input.uv.y, pick);
        let half = (0.06 + 0.12 * pick) * (1.0 - 0.8 * clamp(input.uv.x / reach, 0.0, 1.0));
        let aa = max(fwidth(across), 0.0001);
        let line = (1.0 - smoothstep(half - aa, half + aa, abs(fract(across) - 0.5 - shift)))
            * shown * (1.0 - smoothstep(reach - 0.1, reach, input.uv.x));
        let aa_tip = max(fwidth(input.uv.y), 0.0001);
        let tip = (1.0 - smoothstep(0.035 - aa_tip, 0.035 + aa_tip, input.uv.y))
            * (1.0 - smoothstep(0.7, 0.95, input.uv.x));
        let lit = max(line, tip) * smoothstep(0.0, 0.02, input.uv.x);
        let core = max(line, tip) * 0.55;
        return vec4((tint * 0.9 * lit + vec3(core * lit)) * (1.0 - input.uv.x * 0.5), 1.0);
    }
    if style == 3u {
        let tint = input.color.rgb / max(max(input.color.r, input.color.g), max(input.color.b, 0.001));
        return vec4((glow.rgb + core.rgb * 0.6) * tint * tint, 1.0);
    }
    return vec4(glow.rgb * input.color.rgb + core.rgb, 1.0);
}

@fragment
fn fragment_black(input: VertexOutput) -> @location(0) vec4<f32> {
    // Explicit gradients keep sampling valid for mixed ordinary/black vertices.
    let dx = dpdx(input.uv);
    let dy = dpdy(input.uv);
    if input.style != 2u { discard; }
    let texel = textureSampleGrad(black_trail, trail_sampler, input.uv, dx, dy);
    // EJK's blacktrail shader draws the same alpha stage twice.
    return vec4(texel.rgb, 1.0 - (1.0 - texel.a) * (1.0 - texel.a));
}

// A heat wake (a slice with `WAKE`, the Sun's extra): what is behind the trail shimmers,
// the air bent by heat, strongest where the blade just passed and gone as the slice ages and
// at its edges, so the scene there is unchanged and the merge sees no seam. Drawn first in
// the effect pass with the blades' heat haze, replacing what it covers with the scene's
// pixels read at an offset (no blending), as `saber.wgsl`'s `fragment_haze` does.
@fragment
fn fragment_wake(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow): world units across a pixel.
    let dx = dpdx(input.world);
    let dy = dpdy(input.world);
    if (input.style & WAKE) == 0u { discard; }
    let size = vec2<f32>(textureDimensions(wake_scene));
    let fresh = 1.0 - clamp(input.uv.x, 0.0, 1.0);
    let along = clamp(input.uv.y, 0.0, 1.0);
    // Gone at both edges along the blade and toward the slice's end.
    let fade = sin(along * 3.14159) * smoothstep(0.0, 0.45, fresh);
    let age = input.color.a;
    let p = input.world * 0.09 - vec3(0.0, 0.0, age * 6.0);
    let n = vec2(trail_noise(p), trail_noise(p + vec3(19.1, 7.3, 3.7))) - 0.5;
    // 2.5 units of bend at most, as pixels here, never more than 2 % of the screen
    // (`HAZE_REACH`, the room the effect rectangle keeps).
    let units = max(max(length(dx), length(dy)), 0.0001);
    var pixels = n * 2.0 * 2.5 * fade / units;
    let most = 0.02 * max(size.x, size.y);
    let len = length(pixels);
    if len > most {
        pixels *= most / len;
    }
    let uv = (input.clip_position.xy + pixels) / size;
    return vec4(textureSampleLevel(wake_scene, wake_sampler, uv, 0.0).rgb, 1.0);
}
