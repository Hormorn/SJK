// Saber blade: a smooth glow capsule and a flat hot core, blended GL_ONE GL_ONE into the
// legacy effect layer (display values, per-channel clamp; `effect_layer.rs`).
//
// Stock draws the glow as RB_SurfaceSaberGlow's chain of camera-facing sprites, one every
// 0.65·radius from the tip down, each 0.017 wider, plus a hilt sprite of radius 5.5-5.75
// (rd-vanilla tr_surface.cpp:470-490), and the core as RB_SurfaceLine's flat quad from the
// tip to one unit behind the hilt (tr_surface.cpp:511-566, CG_DoSaber cg_players.c:5359).
// SJK keeps one smooth capsule (owner decision, step 526i) with the same brightness: the
// chain's sum is integrated along the blade instead of summed per sprite, which is its
// continuous limit. For sprites at density 1/(0.65 r) along a blade whose projection is
// `shaft` long, a point at (x, y) receives
//     (L / (0.65 r)) · mean over v in [v_lo, v_hi] of glow(u, v),
//     u = 0.5 + x / 2r, v_hi = 0.5 + y / 2r, v_lo = 0.5 + (y - shaft) / 2r,
// and the mean comes from the glow image's column integral, `glow_integral`.
//
// Blade skins (saber_skins.rs) have their own grey glow/core pair and are coloured and
// animated here from the instance's time and seed and the skin's parameters, which come
// from its blade-skin file (blade_skin_file.rs) in the uniform array `skins`: a hot core
// with a fringe, a corona graded from its rim to the inside, granulation drifting along
// the blade, flame tongues licking outward, shimmer, and flares running from the hilt to
// the tip, swelling the glow; optionally lightning arcs, drifting motes and a turning hue,
// and since 10/10/2026 a sputtering edge and length, glitches, hologram scan lines, a
// heartbeat, falling embers, veins, a team or surroundings tint and the wearer's name in
// glyphs (afterimages are instances of their own); since 11/10/2026 a glint at the tip (an
// instance of its own, a quad round the tip), wisps of smoke rising off the blade, heat
// haze (`fragment_haze`, drawn first into the effect layer, bending the scene behind it)
// and an echo (an afterimage swaying beside the blade). The sections a wearer switched off
// (`off`, a bit each, blade_skin_extras.rs `Section`) are drawn as absent: `skin_at`
// zeroes their lanes, as a file without them has. A skin's blade ends round: past the tip the corona widens, grades from its inside to its
// rim and licks out about the tip, and the core line narrows to a rounded point over its
// last `tip` half-widths (the stock line ends flat, hidden under its glow). Nothing here
// knows any one skin. The dynamic glow pass shades the glow the same way, so its bloom
// follows the flares and arcs.
struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

@group(1) @binding(0)
var glow_texture: texture_2d<f32>;
@group(1) @binding(1)
var core_texture: texture_2d<f32>;
@group(1) @binding(2)
var saber_sampler: sampler;
// Row j holds the glow image's mean of rows 0..j-1 per column; row 0 is zero.
@group(1) @binding(3)
var glow_integral: texture_2d<f32>;
// The core line is sampled as stock samples it: rd-vulkan's box mip chain, trilinear with its
// default 2x anisotropy (saber.rs `Samplers`). At normal distances the line is a few pixels
// wide for 64 texels, and the mip average widens the white core's fringe.
@group(1) @binding(4)
var core_sampler: sampler;
// A picture a signature can paint its body with (`skinBody`), repeating along the blade.
@group(1) @binding(5)
var blade_body: texture_2d<f32>;
@group(1) @binding(6)
var blade_body_sampler: sampler;
fn skinBody(uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(blade_body, blade_body_sampler, uv, 0.0).rgb;
}

// One loaded skin's parameters (saber_skins.rs `SkinUniform`, blade_skin_file.rs for the
// meaning of each). `grain` is the granulation in [0, 1] and `flare` the flares' sum.
struct Skin {
    // Core: rgb the hot centre's colour, w how much a flare brightens it.
    core_white: vec4<f32>,
    // Fringe colours, cool and hot; w: fringe_heat base and grain.
    core_fringe_cool: vec4<f32>,
    core_fringe_hot: vec4<f32>,
    // Fringe brightness: base, grain, flare; w the rounded tip's length in half-widths.
    core_fringe: vec4<f32>,
    // The core's breathing across: amount, rate, along, seed.
    core_breathe: vec4<f32>,
    // Corona colours, rim cool and hot and the inside; w: inner_width, rim heat by
    // grain and by flare.
    rim_cool: vec4<f32>,
    rim_hot: vec4<f32>,
    inner: vec4<f32>,
    // The inside colour's share: base, grain, flare.
    inner_mix: vec4<f32>,
    // The glow's brightness: base, grain, flare.
    brightness: vec4<f32>,
    // Reach (widest, in stock capsules), swell by grain and by flare.
    swell: vec4<f32>,
    // Granulation octaves: scale, speed, evolve, offset.
    grain_coarse: vec4<f32>,
    grain_fine: vec4<f32>,
    // Octave weights, then the contrast's low and high.
    grain_mix: vec4<f32>,
    // Flares: rate, rate per track, rate per seed, track count.
    flare_rate: vec4<f32>,
    // Phase per seed and per track, the lit threshold, overshoot.
    flare_shape: vec4<f32>,
    // Size and its jitter.
    flare_size: vec4<f32>,
    // Shimmer waves: amount, rate, along, seed.
    shimmer_a: vec4<f32>,
    shimmer_b: vec4<f32>,
    // Tongues: along, offset, out, speed; then edge low and high, low, range.
    tongue_a: vec4<f32>,
    tongue_b: vec4<f32>,
    // Lightning arcs: colour and brightness; width, halo, jag, kinks; count, rate,
    // threshold, decay; reach, span low and high, tip share; jitter, crawl, the share
    // rooted at the emitter. No arcs: zeros.
    arc_color: vec4<f32>,
    arc_shape: vec4<f32>,
    arc_strike: vec4<f32>,
    arc_place: vec4<f32>,
    arc_motion: vec4<f32>,
    // Motes: colour and brightness; density, cells, rings, size; drift along and out,
    // twinkle, stretch; inner, outer, focus. No motes: zeros.
    mote_color: vec4<f32>,
    mote_field: vec4<f32>,
    mote_motion: vec4<f32>,
    mote_band: vec4<f32>,
    // Hue turning: turns a second, per unit along, per capsule radius out. None: zeros.
    hue: vec4<f32>,
    // The later sections (blade_skin_effects.rs); each zeros when absent.
    // Sputter: ragged, scale, speed; cut, rate, threshold.
    sputter_edge: vec4<f32>,
    sputter_cut: vec4<f32>,
    // Glitch: split, blocks, rate, threshold; shift, flash.
    glitch_a: vec4<f32>,
    glitch_b: vec4<f32>,
    // Scan (hologram): wireframe colour and brightness; lines, speed, depth, rings; edge,
    // width, hollow; jitter and its rate.
    scan_color: vec4<f32>,
    scan_lines: vec4<f32>,
    scan_edge: vec4<f32>,
    scan_jitter: vec4<f32>,
    // Pulse (heartbeat): rate, amount, second, gap; width, swell.
    pulse_a: vec4<f32>,
    pulse_b: vec4<f32>,
    // Embers: colour and brightness; density, cells, fall, life; size, spread.
    ember_color: vec4<f32>,
    ember_field: vec4<f32>,
    ember_shape: vec4<f32>,
    // Veins: colour and brightness; scale, speed, width, core.
    vein_color: vec4<f32>,
    vein_shape: vec4<f32>,
    // Team colours, red (w the amount), blue and none.
    team_red: vec4<f32>,
    team_blue: vec4<f32>,
    team_none: vec4<f32>,
    // The light where it is: amount, saturate, floor.
    ambient: vec4<f32>,
    // Glyphs: colour and brightness; size, spacing, speed, width.
    glyph_color: vec4<f32>,
    glyph_shape: vec4<f32>,
    // The third set (blade_skin_extras.rs); each zeros when absent.
    // Star (the tip's glint): colour and brightness; rays, length, width, halo; spin,
    // twinkle, depth, and the glints circling the blade (0: one at the tip). Their orbit:
    // how far below the tip (units), how far out (units), turns a second, tilt (radians).
    star_color: vec4<f32>,
    star_shape: vec4<f32>,
    star_motion: vec4<f32>,
    star_orbit: vec4<f32>,
    // Black holes in the glints' places: the ring's colour and brightness; the horizon's
    // radius (units), the lens (horizon radii), the swirl's turns a second, the quad's
    // reach (units).
    hole_color: vec4<f32>,
    hole_shape: vec4<f32>,
    // Wisps: colour and brightness; rise, speed, scale, curl; density.
    wisp_color: vec4<f32>,
    wisp_shape: vec4<f32>,
    wisp_field: vec4<f32>,
    // Heat haze: strength (units), scale, speed, reach (units past the glow).
    haze: vec4<f32>,
    // The fifth set (blade_skin_lab.rs); each zeros when absent.
    // Bands: colour and brightness; spacing (units), width (share), twist (bands a radius
    // across), speed (units a second); mix, pinch.
    band_color: vec4<f32>,
    band_shape: vec4<f32>,
    band_mix: vec4<f32>,
    // Segments: count, draws a second, threshold, depth; gap (units), hum.
    segment_a: vec4<f32>,
    segment_b: vec4<f32>,
    // Film: amount, scale, speed, saturation; clear.
    film: vec4<f32>,
    film_b: vec4<f32>,
    // Blobs: colour and brightness; count, size (radii), speed (units a second), wobble;
    // the pools' size.
    blob_color: vec4<f32>,
    blob_shape: vec4<f32>,
    blob_more: vec4<f32>,
    // Front: colour and amount; period (s), duty, hold, reach (shares); soft (units), jag,
    // veins.
    front_color: vec4<f32>,
    front_shape: vec4<f32>,
    front_edge: vec4<f32>,
    // The sixth set (blade_skin_figures.rs); each zeros when absent.
    // Sketch: colour and brightness; rate, wobble, hatch, glow kept.
    sketch_color: vec4<f32>,
    sketch_shape: vec4<f32>,
    // Hourglass: sand colour and brightness; period, waist, glass.
    hour_color: vec4<f32>,
    hour_shape: vec4<f32>,
    // Serpent: colour and brightness; coils, amplitude, speed, thickness.
    serpent_color: vec4<f32>,
    serpent_shape: vec4<f32>,
    // Strands: count, spread, sway, brightness; hue, saturation.
    strand_shape: vec4<f32>,
    strand_hue: vec4<f32>,
    // Butterflies: colour and brightness; count, size, flap, flight; roam.
    fly_color: vec4<f32>,
    fly_shape: vec4<f32>,
    fly_roam: vec4<f32>,
    // Wings: colour and brightness; span, rate, feathers.
    wing_color: vec4<f32>,
    wing_shape: vec4<f32>,
    // Copies: count, angle, fade, period; flicker.
    copy_shape: vec4<f32>,
    copy_more: vec4<f32>,
    // The seventh set (blade_skin_solids.rs), drawn blended after the blades; zeros when
    // absent. Ink: body colour and opacity; outline, width (blade radii), edge (radii),
    // point (units); gloss, halftone (dots a unit).
    ink_color: vec4<f32>,
    ink_shape: vec4<f32>,
    ink_more: vec4<f32>,
    // Chrome: tint and width (blade radii); ripple, beads, speed.
    chrome_color: vec4<f32>,
    chrome_shape: vec4<f32>,
    // Creyon's Blade Lab design it is drawn as (blade_lab_designs.rs): its number (0 none),
    // its glow's reach and its particles' room (metres), whether it has a solid pass.
    design: vec4<f32>,
    // Its colour (the lab's), for the light at its emitter.
    design_color: vec4<f32>,
    // Unrest (blade_skin_smoke.rs): x 1 when given, reach and fade (units from the
    // emitter), far (the share left up the blade). Zeros: unsettled all along.
    unrest: vec4<f32>,
    // x: how far (units along a capsule radius out) the tongues' spikes slant to the tip;
    // y: how much further (units) it reaches up a whole swing (the instance's `motion.x`);
    // z: how much the heat haze calms with it (0 none, 1 only where the blade is
    // unsettled); w: how much further its arcs reach at a clash (`unrest_clash`).
    unrest_lean: vec4<f32>,
    // Flash (the emitter's crackling star): colour and brightness; rays, length (units),
    // width (share), halo; rate (shuffles a second), chaos, jitter (units), lean.
    flash_color: vec4<f32>,
    flash_shape: vec4<f32>,
    flash_motion: vec4<f32>,
    // x: its place along the blade (a share, 0 the emitter); y and z how much brighter
    // and bigger it is the moment the blade lights (its instance's red lane, 0 to 1, the
    // share of that left).
    flash_place: vec4<f32>,
    // Its heat burst as the blade lights: strength (units), reach (units), width (units);
    // its instance's blue lane says what is left of it (`fragment_haze`).
    flash_burst: vec4<f32>,
    // The arcs' climb at a clash (`unrest.clash`, its units `unrest_lean.w`): x the
    // seconds it settles over, the clash's age the instance's `motion.y`.
    unrest_clash: vec4<f32>,
}

@group(2) @binding(0)
var<storage, read> skins: array<Skin, MAX_SKINS>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    // Across and along the projected blade, world units; along is 0 at the hilt.
    @location(1) blade: vec2<f32>,
    // The projected shaft's length; for a glint or hole, how hidden it is behind the blade.
    @location(2) @interpolate(flat) shaft: f32,
    @location(3) @interpolate(flat) radius: f32,
    @location(4) @interpolate(flat) length: f32,
    // Hilt sprite radius for the glow; zero selects the core line.
    @location(5) @interpolate(flat) hilt: f32,
    // KIND_RETAIL, KIND_NEUTRAL, or KIND_SKIN plus the skin's number.
    @location(6) @interpolate(flat) kind: u32,
    // Presentation seconds (wrapped) and the blade's seed in [0, 1).
    @location(7) @interpolate(flat) animation: vec2<f32>,
    // Who wears it (saber_persona.rs): the light where it is and the afterimage's
    // brightness; the team and the name's letters.
    @location(8) @interpolate(flat) persona: vec4<u32>,
    // The world's down in the projected blade's plane (across, along) and, in zw, the
    // blade's own reference direction across its axis, octahedron-packed (`frame`), for the
    // signatures' fixed three-dimensional frame.
    @location(9) @interpolate(flat) down: vec4<f32>,
    // A chroma skin's turn to its wearer's colour (saber_skins.rs `Chroma`); 0 otherwise.
    @location(10) @interpolate(flat) chroma: f32,
    // The sections its wearer switched off: x the `OFF_*` bits, y the `OFF2_*`; 0 otherwise.
    @location(11) @interpolate(flat) off: vec2<u32>,
    // A skinned blade's swing (0 to 1) and the seconds since its last clash, since it was
    // lit and since its last swing began (saber_motion.rs).
    @location(12) @interpolate(flat) motion: vec4<f32>,
    // The quad's point from the camera (the view ray through the fragment), and the
    // blade's hilt from the camera and its direction: what `blade_place` finds the true
    // place on the blade with, where the projected `blade` is off under perspective.
    @location(13) ray: vec3<f32>,
    @location(14) @interpolate(flat) hilt_at: vec3<f32>,
    // w: the blade's full length (`frame.w`), 0 when unknown.
    @location(15) @interpolate(flat) axis: vec4<f32>,
}

// TaystJK's CG_DoSaber submits the core RT_LINE twice (taystjk cgame cg_players.c:6429 and 6465;
// the second adds `saber` where `sbak` was meant). The owner's reference look is TaystJK's. Two
// GL_ONE GL_ONE adds with a clamp after each are one add of twice the colour.
const CORE_DRAWS: f32 = 2.0;
// Slot of the engine-generated neutral glow/core pair (saber_rgb.rs), and of the first
// loaded blade skin's (saber_rgb.rs `SKIN_MATERIAL`); skins loaded at once.
const NEUTRAL_MATERIAL: u32 = 6u;
const SKIN_MATERIAL: u32 = 7u;
const MAX_SKINS: u32 = 512u;
const BLACK_MATERIAL: u32 = 519u;
const KIND_RETAIL: u32 = 0u;
const KIND_NEUTRAL: u32 = 1u;
const KIND_BLACK: u32 = 2u;
const KIND_SKIN: u32 = 3u;
// Colour alpha of a skin's first glint (saber.rs `STAR`), glint n's `STAR - n`; a glow's
// is its hilt sprite's radius, a core's zero.
const STAR: f32 = -1.0;
// Colour alpha of a skin's first emitter star (saber.rs `FLASH`), star n's `FLASH - n`.
const FLASH: f32 = -16.0;
// Most rays an emitter star has (blade_skin_smoke.rs `MAX_FLASH_RAYS`).
const MAX_FLASH_RAYS: i32 = 12;
// How bright a glint shows behind the blade, where its glow covers it.
const GLINT_BEHIND: f32 = 0.3;
// The sections a wearer may switch off, a bit each (blade_skin_extras.rs `Section`).
const OFF_ARCS: u32 = 1u;
const OFF_MOTES: u32 = 2u;
const OFF_HUE: u32 = 4u;
const OFF_SPUTTER: u32 = 8u;
const OFF_GLITCH: u32 = 16u;
const OFF_SCAN: u32 = 32u;
const OFF_PULSE: u32 = 64u;
// The afterimages are the CPU's (it makes them), so their bit never reaches the shader as
// a section switched off: it marks an afterimage itself (saber.rs `Instance::ghost`).
const OFF_GHOSTS: u32 = 128u;
const OFF_EMBERS: u32 = 256u;
const OFF_VEINS: u32 = 512u;
const OFF_TEAM: u32 = 1024u;
const OFF_AMBIENT: u32 = 2048u;
const OFF_GLYPHS: u32 = 4096u;
const OFF_STAR: u32 = 8192u;
const OFF_WISPS: u32 = 16384u;
const OFF_HAZE: u32 = 32768u;
const OFF_HOLES: u32 = 131072u;
const OFF_BANDS: u32 = 262144u;
const OFF_SEGMENTS: u32 = 524288u;
const OFF_FILM: u32 = 1048576u;
const OFF_BLOBS: u32 = 2097152u;
const OFF_FRONT: u32 = 4194304u;
const OFF_SKETCH: u32 = 8388608u;
const OFF_HOURGLASS: u32 = 16777216u;
const OFF_SERPENT: u32 = 33554432u;
const OFF_STRANDS: u32 = 67108864u;
const OFF_BUTTERFLIES: u32 = 134217728u;
const OFF_WINGS: u32 = 268435456u;
const OFF_COPIES: u32 = 536870912u;
const OFF_INK: u32 = 1073741824u;
const OFF_CHROME: u32 = 2147483648u;
// The mask's second word (`off.y`), the sections from `Section::Smoke` on, a bit each in the
// same order: the CPU makes the others (smoke, ribbons, sparks, bolts, the stars, the
// crackle, the burning), so the shader reads the unrest's alone.
const OFF2_UNREST: u32 = 16u;
// RB_SurfaceSaberGlow: sprite spacing and per-sprite growth.
const SPACING: f32 = 0.65;
const GROWTH: f32 = 0.017;

const QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0),
    vec2(-1.0, 0.0), vec2(1.0, 1.0), vec2(-1.0, 1.0),
);

// Sprite radius at fraction `along` of the blade (0 hilt, 1 tip): the chain starts at the
// tip with the drawn radius and every sprite toward the hilt is GROWTH wider.
fn chain_radius(radius: f32, length: f32, along: f32) -> f32 {
    return radius + GROWTH * (1.0 - along) * length / (SPACING * radius);
}

// One blade instance, as the vertex buffer holds it (saber.rs `Instance`).
struct BladeInstance {
    @location(0) base: vec3<f32>,
    @location(1) length: f32,
    @location(2) direction: vec3<f32>,
    @location(3) radius: f32,
    @location(4) color: vec4<f32>,
    @location(5) material: u32,
    @location(6) animation: vec2<f32>,
    @location(7) persona: vec4<u32>,
    @location(8) chroma: f32,
    @location(9) off: vec2<u32>,
    @location(10) motion: vec4<f32>,
    // A direction across the blade fixed to its hilt (world, unit; zero when unknown) and
    // the blade's full length (saber.rs `Instance::with_frame`).
    @location(11) frame: vec4<f32>,
}

// How far (units) a skin's glow quad reaches past the glow for the sections its wearer
// left on: the wisps' rise, the haze's reach, the butterflies' flights and the wings'
// span (blade_skin_extras.rs `room`).
fn skin_room(skin: Skin, off: u32) -> f32 {
    var room = 0.0;
    if (off & OFF_WISPS) == 0u && skin.wisp_color.w > 0.0 { room = skin.wisp_shape.x; }
    if (off & OFF_HAZE) == 0u && skin.haze.x > 0.0 { room = max(room, skin.haze.w); }
    if (off & OFF_BUTTERFLIES) == 0u && skin.fly_shape.x >= 1.0 {
        room = max(room, skin.fly_roam.x + skin.fly_shape.y);
    }
    if (off & OFF_WINGS) == 0u && skin.wing_shape.x > 0.0 { room = max(room, skin.wing_shape.x); }
    return min(room, 16.0);
}

// Whether an instance is an afterimage: dimmed (the blade itself is 255) and marked with
// the afterimages' bit (the echo, dimmed too, is not; saber.rs `Instance::ghost`).
fn afterimage(persona: u32, off: u32) -> bool {
    return (persona >> 24u) != 255u && (off & OFF_GHOSTS) != 0u;
}

@vertex
fn vertex_main(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    return blade_vertex(vertex_index, blade);
}

// The half-size layer's quad: a panel facing the camera round the blade and its room. The blade's own quad lies along the blade:
// seen nearly along it, it is so foreshortened that its soft edge became a hard slanted line (Creyon's videos). The effects are
// worked out from the true view ray, so they do not care what they are drawn on; this is only where they are drawn.
@vertex
fn vertex_volume(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    if output.hilt <= 0.0 || output.kind < KIND_SKIN {
        return output;
    }
    let direction = output.axis.xyz;
    let skin = skins[skin_index(output)];
    let room = max(skin_room(skin, output.off.x), max(skin.design.y, skin.design.z) / LAB_METRE) + 3.0;
    let base = output.hilt_at + camera.position;
    let full = max(output.length, output.axis.w);
    let centre = base + direction * full * 0.5;
    let to_eye = camera.position - centre;
    let dist = max(length(to_eye), 0.001);
    let view = to_eye / dist;
    var side = cross(direction, view);
    let projected = length(side);
    if projected < 0.001 {
        side = cross(select(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0), abs(view.z) > 0.9), view);
    }
    side = normalize(side);
    let up = normalize(cross(view, side));
    // What is nearer than the panel looks bigger on it.
    let along_eye = abs(dot(direction, view)) * full * 0.5;
    let grow = dist / max(dist - along_eye - room, dist * 0.2);
    let half_w = room * grow;
    let half_h = (full * 0.5 * projected + room) * grow;
    let local = QUAD[vertex_index];
    let world = centre + side * (local.x * half_w) + up * ((local.y * 2.0 - 1.0) * half_h);
    output.clip_position = camera.view_projection * vec4(world, 1.0);
    output.ray = world - camera.position;
    output.blade = vec2(local.x * half_w, (local.y * 2.0 - 1.0) * half_h);
    return output;
}

// The haze's quads: a skin's glow quad when its haze is on, for the blade itself (not an
// afterimage); every other instance is folded away.
@vertex
fn vertex_haze(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    var hazy = output.kind >= KIND_SKIN && output.hilt > 0.0 && (blade.persona.x >> 24u) == 255u
        && (blade.off.x & OFF_HAZE) == 0u;
    if hazy {
        hazy = skins[skin_index(output)].haze.x > 0.0;
    }
    // A skin's first emitter star while its heat burst spreads (`flash.burst`).
    let bursting = output.kind >= KIND_SKIN && output.hilt <= FLASH + 0.5
        && output.hilt > FLASH - 0.5 && blade.color.b > 0.0 && (blade.off.x & OFF_HAZE) == 0u;
    if bursting {
        hazy = skins[skin_index(output)].flash_burst.x > 0.0;
    }
    if !hazy {
        output.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
    }
    return output;
}

// A unit direction folded onto an octahedron's two coordinates, and back; zero stays zero.
fn oct_pack(n0: vec3<f32>) -> vec2<f32> {
    let l1 = abs(n0.x) + abs(n0.y) + abs(n0.z);
    if l1 < 0.000001 {
        return vec2(0.0);
    }
    let n = n0 / l1;
    if n.z >= 0.0 {
        return n.xy;
    }
    return (vec2(1.0) - abs(n.yx)) * select(vec2(-1.0), vec2(1.0), n.xy >= vec2(0.0));
}

fn oct_unpack(e: vec2<f32>) -> vec3<f32> {
    var n = vec3(e, 1.0 - abs(e.x) - abs(e.y));
    if n.z < 0.0 {
        n = vec3((vec2(1.0) - abs(n.yx)) * select(vec2(-1.0), vec2(1.0), n.xy >= vec2(0.0)), n.z);
    }
    return n;
}

fn blade_vertex(vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    let blade_base = blade.base;
    let blade_length = blade.length;
    let blade_direction = blade.direction;
    let blade_radius = blade.radius;
    let blade_color = blade.color;
    let blade_material = blade.material;
    let blade_animation = blade.animation;
    let blade_persona = blade.persona;
    let blade_chroma = blade.chroma;
    var direction = normalize(blade_direction);
    // A blade gone in whose comic words float on (`saber_motion::BladeMotion::lingering`,
    // swing -1) stands up the screen, so its word faces the camera from every side and
    // from above as a lit blade's words do (Creyon's report of 20/10/2026: standing up in
    // the world, it was read only from in front and lay edge on from above).
    if blade.motion.x < -0.5 {
        let eye = normalize(camera.position - blade_base);
        let vp = camera.view_projection;
        let screen_up = vec3(vp[0].y, vp[1].y, vp[2].y);
        let upright = screen_up - eye * dot(screen_up, eye);
        if length(upright) > 0.00001 {
            direction = normalize(upright);
        }
    }
    let view_direction = normalize(camera.position - (blade_base + direction * blade_length * 0.5));
    var side = cross(direction, view_direction);
    let projected = length(side);
    if projected < 0.001 {
        // Pick an axis that cannot be parallel, including a blade aimed along Z.
        let axis = select(vec3(0.0,0.0,1.0),vec3(0.0,1.0,0.0),abs(view_direction.z) > 0.9);
        side = cross(axis,view_direction);
    }
    side = normalize(side);
    let up = normalize(cross(view_direction,side));
    let local = QUAD[vertex_index];
    let hilt = blade_color.a;
    var output: VertexOutput;
    output.kind = KIND_RETAIL;
    if blade_material == NEUTRAL_MATERIAL { output.kind = KIND_NEUTRAL; }
    if blade_material == BLACK_MATERIAL { output.kind = KIND_BLACK; }
    var reach = 1.0;
    var room = 0.0;
    var star = 0.0;
    var glints = 0.0;
    var orbit = vec4(0.0);
    output.off = vec2(0u);
    if blade_material >= SKIN_MATERIAL && blade_material < SKIN_MATERIAL + MAX_SKINS {
        output.kind = KIND_SKIN + (blade_material - SKIN_MATERIAL);
        output.off = blade.off;
        let skin = skins[blade_material - SKIN_MATERIAL];
        // A skin's glow reaches further, for its flares and shimmer, and its quad further
        // still for its wisps and haze.
        // An afterimage is the plain capsule (`afterimage_glow`): neither.
        if !afterimage(blade_persona.x, blade.off.x) {
            reach = skin.swell.x;
            room = skin_room(skin, blade.off.x);
        }
        // A Blade Lab design's quad reaches its glow's reach and its particles' room.
        if skin.design.x > 0.0 {
            room = max(room, max(skin.design.y, skin.design.z) / LAB_METRE);
        }
        star = skin.star_shape.y;
        if skin.hole_shape.x > 0.0 { star = skin.hole_shape.w; }
        if hilt <= FLASH + 0.5 {
            star = skin.flash_shape.y * (1.0 + skin.flash_motion.w);
        }
        glints = skin.star_motion.w;
        orbit = skin.star_orbit;
    }
    output.animation = blade_animation;
    output.motion = blade.motion;
    output.persona = blade_persona;
    output.chroma = blade_chroma;
    output.down = vec4(vec2(-side.z, -up.z), oct_pack(blade.frame.xyz));
    var world: vec3<f32>;
    if hilt <= STAR {
        // A skin's glint (or black hole): a square facing the camera, as wide as the rays
        // are long (the hole's reach), round the tip, or round its place on the orbit
        // below it; `blade` is the place on it from its heart, `shaft` how hidden it is.
        let n = STAR - hilt;
        let flash = hilt <= FLASH + 0.5;
        var size = max(star, 0.001) * select(glint_scale(n), 1.0, flash);
        var heart = blade_base + direction * blade_length;
        output.shaft = 0.0;
        if flash {
            // At the emitter (its share along the blade), jumping about it at each shuffle.
            let f = FLASH - hilt;
            let skin = skins[blade_material - SKIN_MATERIAL];
            let shuffle = i32(floor(blade_animation.x * skin.flash_motion.x + f * 0.37));
            let row = i32(f) * 977 + i32(blade_animation.y * 4096.0);
            let jump = vec2(skin_hash(vec2(shuffle, row)), skin_hash(vec2(shuffle, row + 71)))
                * 2.0 - 1.0;
            heart = blade_base + direction * (skin.flash_place.x * blade_length)
                + (side * jump.x + direction * jump.y * 0.5) * skin.flash_motion.z;
            // Grown as it surges once lit (`star_glow` draws it so).
            size *= 1.0 + skin.flash_place.z * blade_color.r;
            // The first is wide enough for the heat burst's ring while it spreads.
            if f < 0.5 && blade_color.b > 0.0 && skin.flash_burst.x > 0.0 {
                size = max(size, skin.flash_burst.y + skin.flash_burst.z);
            }
        } else if glints > 0.0 {
            // Round the blade's axis, measured from the camera's side of it (the near side
            // at a quarter turn), so the circle reads as a ring however the blade is seen;
            // the tilt lifts the near side and drops the far one.
            let toward = normalize(cross(side, direction));
            let angle = TAU * (blade_animation.x * orbit.z + blade_animation.y + n / glints);
            let across = cos(angle);
            let near = sin(angle);
            heart += direction * (orbit.y * tan(orbit.w) * near - min(orbit.x, blade_length))
                + (side * across + toward * near) * orbit.y;
            // Behind the blade, and close enough to its axis for the glow to cover it.
            let behind = smoothstep(0.1, -0.5, near);
            let covered = 1.0 - smoothstep(0.6, 1.6, abs(across) * orbit.y / max(blade_radius, 0.001));
            output.shaft = behind * covered;
        }
        let corner = vec2(local.x, local.y * 2.0 - 1.0);
        world = heart + (side * corner.x + up * corner.y) * size;
        output.blade = corner * size;
    } else if hilt > 0.0 {
        // Wide enough for the grown hilt-end sprites and the hilt sprite itself, and the
        // skin's room.
        let extent = max(chain_radius(blade_radius, blade_length, 0.0) * reach, hilt) + room;
        let shaft = blade_length * projected;
        world = blade_base + direction * (local.y * blade_length)
            + (side * local.x + up * (local.y * 2.0 - 1.0)) * extent;
        output.blade = vec2(local.x * extent, local.y * (shaft + 2.0 * extent) - extent);
        output.shaft = shaft;
    } else {
        // RB_SurfaceLine: flat, from one unit behind the hilt to the tip.
        world = blade_base + direction * mix(-1.0, blade_length, local.y)
            + side * (local.x * blade_radius);
        output.blade = vec2(local.x * blade_radius, local.y);
        output.shaft = 0.0;
    }
    output.clip_position = camera.view_projection * vec4(world, 1.0);
    output.ray = world - camera.position;
    output.hilt_at = blade_base - camera.position;
    output.axis = vec4(direction, blade.frame.w);
    output.color = blade_color.rgb;
    output.radius = blade_radius;
    output.length = blade_length;
    output.hilt = hilt;
    return output;
}

// The true place on the blade a glow quad's fragment shows, in units: across (signed as
// `blade.x`) and along (0 at the hilt). The point on the blade's axis nearest the view
// ray, and the ray's distance from it there. The projected `blade` treats the quad as
// seen square on, which strays far from the blade under a close perspective (first person,
// looking along it); the Blade Lab's shaders need the place itself.
fn blade_place(input: VertexOutput) -> vec2<f32> {
    let r = input.ray;
    let d = input.axis.xyz;
    let from_hilt = -input.hilt_at;
    let rr = dot(r, r);
    let rd = dot(r, d);
    let rw = dot(r, from_hilt);
    let dw = dot(d, from_hilt);
    let denom = max(rr - rd * rd, rr * 0.000001);
    let along = (rr * dw - rd * rw) / denom;
    let on_ray = (rd * dw - rw) / denom;
    let gap = from_hilt + r * on_ray - d * along;
    // Update 26 (Creyon: Byss showed in parts, cut across from tip to hilt): the side from the ray
    // and the axis, not the quad's `blade.x`: under a close perspective the quad's middle strays from
    // the blade's, so a slice of it drew mirrored, seen wherever a blade's two sides differ.
    let normal = cross(r, d);
    let across = dot(gap, normal) / max(length(normal), 0.000001);
    return vec2(across, along);
}

// The place a new blade's body and glow are drawn at. Off to the side of the blade, the ray's nearest point on it runs away toward
// infinity as the view comes round along the blade, so everything tied to a height along it was cut off in a hard slanted line
// and never reached the tip, seen from behind a player (Creyon, 4/11/2026). Away from the blade its height comes from the quad instead.
fn sig_place(input: VertexOutput) -> vec2<f32> {
    let p = blade_place(input);
    if !SIG_OWN_EFFECTS {
        return p;
    }
    let shaft = max(input.shaft, 0.0001);
    let over = max(input.blade.y - shaft, 0.0);
    let flat_y = min(input.blade.y, shaft) / shaft * input.length + over;
    // Above its end it is always the quad's height: halfway between the two, a thin line of blade stood over the tip.
    return vec2(p.x, mix(p.y, flat_y, max(smoothstep(0.9, 2.4, abs(p.x)), smoothstep(0.0, 1.5, over))));
}

fn glow(u: f32, v: f32) -> vec3<f32> {
    if u < 0.0 || u > 1.0 || v < 0.0 || v > 1.0 { return vec3(0.0); }
    return textureSampleLevel(glow_texture, saber_sampler, vec2(u, v), 0.0).rgb;
}

fn integral(u: f32, v: f32) -> vec3<f32> {
    let rows = f32(textureDimensions(glow_integral).y) - 1.0;
    let y = (clamp(v, 0.0, 1.0) * rows + 0.5) / (rows + 1.0);
    return textureSampleLevel(glow_integral, saber_sampler, vec2(u, y), 0.0).rgb;
}

fn glow_capsule(input: VertexOutput) -> vec3<f32> {
    let x = input.blade.x;
    let y = input.blade.y;
    let r = chain_radius(input.radius, input.length,
        clamp(y / max(input.shaft, 0.0001), 0.0, 1.0));
    var sum = vec3(0.0);
    let u = 0.5 + x / (2.0 * r);
    if u >= 0.0 && u <= 1.0 {
        let high = 0.5 + y / (2.0 * r);
        let low = high - input.shaft / (2.0 * r);
        let count = input.length / (SPACING * r);
        if high - low < 1.0 / 64.0 {
            // Seen almost end-on the sprites stack: `count` copies of one texel.
            sum = count * glow(u, 0.5 * (high + low));
        } else {
            sum = count * (integral(u, high) - integral(u, low)) / (high - low);
        }
    }
    // The hilt sprite, centred on the blade's base.
    let hilt = 0.5 + vec2(x, y) / (2.0 * input.hilt);
    return sum + glow(hilt.x, hilt.y);
}

// --- Blade skins ---------------------------------------------------------------------------

const PI: f32 = 3.14159265;
const TAU: f32 = 6.2831853;
// Most lightning arcs a skin may have (blade_skin_file.rs `MAX_ARCS`).
const MAX_ARCS: i32 = 4;

// PCG hash of a lattice point, in [0, 1].
fn skin_hash(cell: vec2<i32>) -> f32 {
    var v = bitcast<u32>(cell.x) * 747796405u + 2891336453u;
    v = v ^ (bitcast<u32>(cell.y) * 2654435761u);
    v = v * 747796405u + 2891336453u;
    let word = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    return f32((word >> 22u) ^ word) / 4294967295.0;
}

// Smooth value noise in [0, 1].
fn skin_noise(p: vec2<f32>) -> f32 {
    let cell = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = skin_hash(cell);
    let b = skin_hash(cell + vec2(1, 0));
    let c = skin_hash(cell + vec2(0, 1));
    let d = skin_hash(cell + vec2(1, 1));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Piecewise-linear noise in [-1, 1]: straight runs between random corners, one a unit of
// `k`, on lattice row `row`.
fn skin_zigzag(k: f32, row: i32) -> f32 {
    let i = floor(k);
    let a = skin_hash(vec2(i32(i), row));
    let b = skin_hash(vec2(i32(i) + 1, row));
    return mix(a, b, k - i) * 2.0 - 1.0;
}

// The skin's number, below MAX_SKINS.
fn skin_index(input: VertexOutput) -> u32 {
    return min(input.kind - KIND_SKIN, MAX_SKINS - 1u);
}

// The skin this instance wears, with the sections its wearer switched off zeroed: drawn as
// absent, as a file without them is (every section draws nothing when its lanes are zeros).
fn skin_at(input: VertexOutput) -> Skin {
    var skin = skins[skin_index(input)];
    // A swinging blade is unsettled further up (`unrest.swing`).
    skin.unrest.y += skin.unrest_lean.y * input.motion.x;
    // The mask's second word: of the eighth set's sections the shader draws the unrest alone.
    if (input.off.y & OFF2_UNREST) != 0u {
        skin.unrest = vec4(0.0);
        skin.unrest_lean = vec4(0.0);
        skin.unrest_clash = vec4(0.0);
    }
    let off = input.off.x;
    if off == 0u {
        return skin;
    }
    if (off & OFF_ARCS) != 0u { skin.arc_strike = vec4(0.0); }
    if (off & OFF_MOTES) != 0u { skin.mote_field = vec4(0.0); }
    if (off & OFF_HUE) != 0u { skin.hue = vec4(0.0); }
    if (off & OFF_SPUTTER) != 0u {
        skin.sputter_edge = vec4(0.0);
        skin.sputter_cut = vec4(0.0);
    }
    if (off & OFF_GLITCH) != 0u { skin.glitch_a = vec4(0.0); }
    if (off & OFF_SCAN) != 0u {
        skin.scan_lines = vec4(0.0);
        skin.scan_jitter = vec4(0.0);
    }
    if (off & OFF_PULSE) != 0u { skin.pulse_a = vec4(0.0); }
    if (off & OFF_EMBERS) != 0u { skin.ember_field = vec4(0.0); }
    if (off & OFF_VEINS) != 0u {
        skin.vein_color = vec4(0.0);
        skin.vein_shape = vec4(0.0);
    }
    if (off & OFF_TEAM) != 0u { skin.team_red = vec4(0.0); }
    if (off & OFF_AMBIENT) != 0u { skin.ambient = vec4(0.0); }
    if (off & OFF_GLYPHS) != 0u {
        skin.glyph_color = vec4(0.0);
        skin.glyph_shape = vec4(0.0);
    }
    if (off & OFF_STAR) != 0u { skin.star_color = vec4(0.0); }
    if (off & OFF_WISPS) != 0u { skin.wisp_color = vec4(0.0); }
    if (off & OFF_HAZE) != 0u { skin.haze = vec4(0.0); }
    if (off & OFF_HOLES) != 0u {
        skin.hole_color = vec4(0.0);
        skin.hole_shape = vec4(0.0);
    }
    if (off & OFF_BANDS) != 0u { skin.band_shape = vec4(0.0); }
    if (off & OFF_SEGMENTS) != 0u { skin.segment_a = vec4(0.0); }
    if (off & OFF_FILM) != 0u { skin.film = vec4(0.0); }
    if (off & OFF_BLOBS) != 0u { skin.blob_shape = vec4(0.0); }
    if (off & OFF_FRONT) != 0u { skin.front_shape = vec4(0.0); }
    if (off & OFF_SKETCH) != 0u { skin.sketch_shape = vec4(0.0); }
    if (off & OFF_HOURGLASS) != 0u { skin.hour_shape = vec4(0.0); }
    if (off & OFF_SERPENT) != 0u { skin.serpent_shape = vec4(0.0); }
    if (off & OFF_STRANDS) != 0u { skin.strand_shape = vec4(0.0); }
    if (off & OFF_BUTTERFLIES) != 0u { skin.fly_shape = vec4(0.0); }
    if (off & OFF_WINGS) != 0u { skin.wing_shape = vec4(0.0); }
    if (off & OFF_COPIES) != 0u { skin.copy_shape = vec4(0.0); }
    if (off & OFF_INK) != 0u {
        skin.ink_color = vec4(0.0);
        skin.ink_shape = vec4(0.0);
        skin.ink_more = vec4(0.0);
    }
    if (off & OFF_CHROME) != 0u { skin.chrome_color = vec4(0.0); }
    return skin;
}

// How far out the glow's own parts may reach (embers, arcs and motes fade out before it):
// the grown chain's widest at the reach, or the hilt sprite. The quad reaches further by
// the skin's room (vertex_main's `extent`), for its wisps and haze only, so switching
// those off changes nothing else.
fn skin_extent(skin: Skin, input: VertexOutput) -> f32 {
    return max(chain_radius(input.radius, input.length, 0.0) * skin.swell.x, input.hilt);
}

// How unsettled the blade is `along` units from the emitter: whole for `reach`, calming
// over `fade` to `far`; whole all along without the section.
fn skin_unrest(skin: Skin, along: f32) -> f32 {
    let u = skin.unrest;
    if u.x == 0.0 {
        return 1.0;
    }
    return mix(1.0, u.w, smoothstep(u.y, u.y + u.z, along));
}

// The granulation where the blade calms: pulled toward a steady middle.
const GRAIN_CALM: f32 = 0.45;

// Granulation at `along` world units from the hilt: two octaves drifting toward the tip and
// slowly changing as they go.
fn skin_granulation(skin: Skin, along: f32, time: f32, seed: f32) -> f32 {
    let c = skin.grain_coarse;
    let f = skin.grain_fine;
    let coarse = skin_noise(vec2(along * c.x - time * c.y, time * c.z + seed * c.w));
    let fine = skin_noise(vec2(along * f.x - time * f.y, time * f.z + seed * f.w));
    let mixed = skin.grain_mix;
    return smoothstep(mixed.z, mixed.w, coarse * mixed.x + fine * mixed.y);
}

// Flares at `along`: tracks each sending a bright knot from the hilt to the tip once a cycle,
// lit on the cycles whose draw reaches the threshold, so they come now and then and never in
// step.
fn skin_flares(skin: Skin, along: f32, length: f32, time: f32, seed: f32) -> f32 {
    let rates = skin.flare_rate;
    let shape = skin.flare_shape;
    let size = skin.flare_size;
    var total = 0.0;
    for (var track = 0; track < i32(rates.w); track++) {
        let rate = rates.x + rates.y * f32(track) + rates.z * seed;
        let phase = time * rate + seed * shape.x + f32(track) * shape.y;
        let cycle = floor(phase);
        let travel = fract(phase);
        let lit = step(shape.z, skin_hash(vec2(i32(cycle), track * 7919 + i32(seed * 4096.0))));
        let position = travel * (length + 2.0 * shape.w) - shape.w;
        let width = size.x + size.y * skin_hash(vec2(i32(cycle), track + 31));
        let offset = (along - position) / width;
        total += lit * sin(PI * travel) * exp(-offset * offset);
    }
    return total;
}

// One wave of shimmer or breathing: amount · sin(time · rate + along · along + seed · seed).
fn skin_wave(wave: vec4<f32>, time: f32, along: f32, seed: f32) -> f32 {
    return wave.x * sin(time * wave.y + along * wave.z + seed * wave.w);
}

// Along the blade from the hilt, world units: the glow's projected `y`, or the core line's
// texture `v` (one unit behind the hilt to the tip).
fn skin_along(input: VertexOutput) -> f32 {
    if input.hilt > 0.0 {
        return clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0) * input.length;
    }
    return mix(-1.0, input.length, input.blade.y);
}

// How much of the core line's width is left `along` units from the hilt: all of it until the
// last `cap` units, then a quarter circle down to nothing at the tip, so the line ends in a
// rounded point instead of a square.
fn skin_tip_taper(along: f32, length: f32, cap: f32) -> f32 {
    let into = clamp((along - (length - cap)) / max(cap, 0.0001), 0.0, 1.0);
    // Exactly 1 along the shaft, whatever the GPU's square root.
    return select(sqrt(max(1.0 - into * into, 0.0001)), 1.0, into <= 0.0);
}

// Lightning arcs at `along` (world units from the hilt, running on past the tip) and `x`
// (signed across): `count` arcs, each struck anew `rate` times a second at a random place
// (strikes whose draw is under `threshold` stay dark), a jagged filament leaving the blade,
// bulging out to one side up to `reach` capsule radii (`r`) and coming back a `span` of the
// blade further on, crawling `crawl` units a second; a share `tip` of the strikes leap from
// just below the tip into the air (up to `room` units past it) and die out there. A strike
// flashes and fades by `decay`, re-shaped `jitter` times a second (the crackle). A filament
// thinner than a pixel widens to it and dims, so it does not break up at a distance.
fn skin_arcs(
    skin: Skin, along: f32, x: f32, r: f32, length: f32, room: f32, pixel: f32, time: f32,
    seed: f32,
) -> f32 {
    let strike = skin.arc_strike;
    let place = skin.arc_place;
    let shape = skin.arc_shape;
    let motion = skin.arc_motion;
    let seeded = i32(seed * 4096.0);
    let width = max(shape.x, pixel);
    var total = 0.0;
    for (var arc = 0; arc < min(i32(strike.x), MAX_ARCS); arc++) {
        let phase = time * strike.y + seed * 3.1 + f32(arc) * 0.618;
        let cycle = i32(floor(phase));
        let age = fract(phase);
        let row = arc * 7919 + seeded;
        if skin_hash(vec2(cycle, row)) < strike.z {
            continue;
        }
        let pick = skin_hash(vec2(cycle, row + 101));
        let side = select(-1.0, 1.0, skin_hash(vec2(cycle, row + 303)) < 0.5);
        var t: f32;
        var bulge: f32;
        var fade = 1.0;
        if skin_hash(vec2(cycle, row + 404)) < place.w {
            // From just below the tip into the air past it, bending away and dying out.
            let span = (0.8 + 0.6 * pick) * room;
            t = (along - (length - 0.35 * span)) / span;
            bulge = sin(0.5 * PI * clamp(t, 0.0, 1.0));
            fade = 1.0 - clamp(t, 0.0, 1.0);
        } else {
            let span = mix(place.y, place.z, pick) * length;
            var start = skin_hash(vec2(cycle, row + 202)) * (length - span)
                + motion.y * age / max(strike.y, 0.0001);
            // Rooted at the emitter, running up the blade for its span.
            if skin_hash(vec2(cycle, row + 505)) < motion.z {
                start = 0.0;
            }
            t = (along - start) / span;
            bulge = sin(PI * clamp(t, 0.0, 1.0));
        }
        if t < 0.0 || t > 1.0 {
            continue;
        }
        let jagged = i32(floor(time * motion.x + f32(arc) * 0.37)) * 13 + arc * 131 + seeded;
        let k = along * shape.w + f32(arc) * 17.0;
        let jag = skin_zigzag(k, jagged) + 0.45 * skin_zigzag(k * 2.7 + 5.0, jagged + 57);
        // Where the blade calms (unrest) the arc hugs it, bulging and zigzagging less.
        let offset = (side * bulge * place.x * r + jag * shape.z * r * sqrt(bulge))
            * skin_unrest(skin, along);
        let d = (x - offset) / width;
        let filament = shape.x / width * exp(-d * d) + shape.y * 0.35 * exp(-d * d / 16.0);
        total += pow(1.0 - age, strike.w) * fade * filament;
    }
    return total;
}

// Motes at `around` (world units along the blade and on round its tip) and `out` (capsule
// radii from the blade's axis, or from the tip past it), on the `left` or right: a field of
// specks in cells, `cells` a unit along by `rings` a radius out, a share `density` of them
// holding one, `size` of its cell across and `stretch` times that along. The field drifts
// `drift_along` units a second toward the tip and `drift_out` radii a second outward
// (negative: inward); each speck twinkles about `twinkle` times a second; they show between
// `inner` and `outer` radii out, by `focus` only toward the tip, and not behind the hilt
// (`along`, unclamped).
fn skin_motes(
    skin: Skin, around: f32, out: f32, left: bool, along: f32, length: f32, time: f32,
    seed: f32,
) -> f32 {
    let field = skin.mote_field;
    let motion = skin.mote_motion;
    let band = skin.mote_band;
    let p_along = (around - time * motion.x) * field.y;
    let column = i32(floor(p_along));
    // Each column of cells is staggered outward by its own amount, so they do not line up.
    let p_out = (out - time * motion.y) * field.z + skin_hash(vec2(column, 4242));
    let cell = vec2(column, i32(floor(p_out)) + i32(seed * 4096.0) * 3 + select(0, 7777, left));
    if skin_hash(cell) >= field.x {
        return 0.0;
    }
    let half = vec2(field.w * motion.w, field.w);
    let centre = half + (1.0 - 2.0 * half)
        * vec2(skin_hash(cell + vec2(0, 911)), skin_hash(cell + vec2(37, 0)));
    let d = (vec2(fract(p_along), fract(p_out)) - centre) / half;
    let falloff = max(1.0 - dot(d, d), 0.0);
    var twinkle = 1.0;
    if motion.z > 0.0 {
        let rate = motion.z * (0.5 + skin_hash(cell + vec2(3, 5)));
        twinkle = 0.5 + 0.5 * sin(TAU * (time * rate + skin_hash(cell + vec2(7, 2))));
    }
    let span = band.y - band.x;
    let shown = smoothstep(band.x, band.x + 0.25 * span, out)
        * (1.0 - smoothstep(band.y - 0.4 * span, band.y, out));
    let toward_tip = mix(1.0, smoothstep(0.6 * length, length, around), band.z);
    return falloff * falloff * twinkle * shown * toward_tip * smoothstep(0.0, 2.0, along);
}

// `color` turned `turns` round the grey axis (a hue shift keeping its brightness and
// saturation), negative channels clipped: the blend adds.
fn skin_turn_hue(color: vec3<f32>, turns: f32) -> vec3<f32> {
    let axis = vec3(0.57735027);
    let c = cos(TAU * turns);
    let s = sin(TAU * turns);
    let turned = color * c + cross(axis, color) * s + axis * dot(axis, color) * (1.0 - c);
    return max(turned, vec3(0.0));
}

// --- Later effects (blade_skin_effects.rs): each draws nothing when its lanes are zeros ---

// Where the first heartbeat peaks in a cycle, seconds (saber_skins.rs `Beat::FIRST`).
const BEAT_FIRST: f32 = 0.1;

// The afterimage's brightness from the instance (1 for a blade itself).
fn persona_fade(input: VertexOutput) -> f32 {
    return f32(input.persona.x >> 24u) / 255.0;
}

// The light where the blade is, at full brightness (saber_persona.rs `pack_light`).
fn persona_light(input: VertexOutput) -> vec3<f32> {
    let packed = input.persona.x;
    return vec3(f32(packed & 255u), f32((packed >> 8u) & 255u), f32((packed >> 16u) & 255u))
        / 255.0;
}

// The wearer's letter `index` (5 bits; 0 ends the name).
fn persona_letter(input: VertexOutput, index: u32) -> u32 {
    let lanes = vec3(input.persona.y & 0x3fffffffu, input.persona.z, input.persona.w);
    return (lanes[index / 6u] >> (5u * (index % 6u))) & 31u;
}

// The heartbeat at `time`: two Gaussian beats a cycle, on the clock alone so the blade
// and its light (saber_skins.rs `Beat::at`) beat together.
fn skin_beat(skin: Skin, time: f32) -> f32 {
    let p = skin.pulse_a;
    if p.x <= 0.0 {
        return 0.0;
    }
    let into = fract(time * p.x) / p.x;
    let width = max(skin.pulse_b.x, 0.001);
    let a = (into - BEAT_FIRST) / width;
    let b = (into - BEAT_FIRST - p.w) / width;
    return p.y * (exp(-a * a) + p.z * exp(-b * b));
}

// How long the blade shows this moment: all of it, or, on a sputtering draw, cut short.
fn skin_cut(skin: Skin, length: f32, time: f32, seed: f32) -> f32 {
    let c = skin.sputter_cut;
    if c.x <= 0.0 || c.y <= 0.0 {
        return length;
    }
    let cycle = i32(floor(time * c.y));
    let seeded = i32(seed * 4096.0);
    if skin_hash(vec2(cycle, 5151 + seeded)) < c.z {
        return length;
    }
    return length * (1.0 - c.x * skin_hash(vec2(cycle, 6161 + seeded)));
}

// Glitch at `along` units: x the jump (capsule radii), y the colour split (radii), z the
// flash. A block glitches on its own draws; now and then the whole blade does.
fn skin_glitch(skin: Skin, along: f32, time: f32, seed: f32) -> vec3<f32> {
    let g = skin.glitch_a;
    if g.y <= 0.0 {
        return vec3(0.0);
    }
    let frame = i32(floor(time * g.z));
    let seeded = i32(seed * 4096.0);
    let block = i32(floor(along / g.y));
    let whole = skin_hash(vec2(frame, 919 + seeded)) >= 0.5 + 0.5 * g.w;
    let draw = skin_hash(vec2(block * 31 + seeded, frame));
    if !whole && draw < g.w {
        return vec3(0.0, g.x, 0.0);
    }
    let jump = skin.glitch_b.x * (skin_hash(vec2(block + 77, frame * 3 + seeded)) * 2.0 - 1.0);
    return vec3(select(jump, jump * 0.5, whole), 2.0 * g.x, skin.glitch_b.y);
}

// The hologram's jitter: the whole projection thrown sideways (capsule radii) on some draws.
fn skin_jitter(skin: Skin, time: f32, seed: f32) -> f32 {
    let j = skin.scan_jitter;
    if j.x <= 0.0 || j.y <= 0.0 {
        return 0.0;
    }
    let frame = i32(floor(time * j.y));
    let seeded = i32(seed * 4096.0);
    if skin_hash(vec2(frame, 4321 + seeded)) < 0.75 {
        return 0.0;
    }
    return j.x * (skin_hash(vec2(frame, 8765 + seeded)) * 2.0 - 1.0);
}

// Veins: ridged noise along and across (`x` units from the axis), bright where it crosses
// its middle, a network of cracks running along the blade.
fn skin_veins(skin: Skin, along: f32, x: f32, time: f32, seed: f32) -> f32 {
    let v = skin.vein_shape;
    if v.x <= 0.0 {
        return 0.0;
    }
    let p = vec2(along * v.x - time * v.y + seed * 31.0, x * v.x * 1.7 + seed * 17.0);
    let n = 0.65 * skin_noise(p) + 0.35 * skin_noise(p * 2.3 + vec2(7.0, 3.0));
    return 1.0 - smoothstep(0.5 * v.z, v.z, abs(n - 0.5));
}

// The colour a skin takes from its wearer's team or the light where it is: rgb (largest
// channel 1) and how much; none, zero.
fn skin_tint(skin: Skin, input: VertexOutput) -> vec4<f32> {
    if skin.team_red.w > 0.0 {
        let team = input.persona.y >> 30u;
        var color = skin.team_none.rgb;
        if team == 1u { color = skin.team_red.rgb; }
        if team == 2u { color = skin.team_blue.rgb; }
        return vec4(color, skin.team_red.w);
    }
    let a = skin.ambient;
    if a.x > 0.0 {
        let light = persona_light(input);
        let grey = vec3(dot(light, vec3(0.299, 0.587, 0.114)));
        let color = max(mix(grey, light, a.y), vec3(a.z));
        return vec4(color / max(max(color.r, color.g), max(color.b, 0.0001)), a.x);
    }
    return vec4(0.0);
}

// `color` recoloured toward `tint` at its own strongest channel's level.
fn skin_apply_tint(color: vec3<f32>, tint: vec4<f32>) -> vec3<f32> {
    if tint.w <= 0.0 {
        return color;
    }
    let level = max(color.r, max(color.g, color.b));
    return mix(color, level * tint.rgb, tint.w);
}

// Embers dripping from the blade and falling, at `p` (across, along the projected blade,
// units) with `down` the world's down in that plane and `scale` projected units a world
// unit along. Each drip place `cells` a unit along drips (if its draw is under `density`)
// one ember a `life`, accelerating `fall` units a second; where gravity runs along the
// blade, they are pushed off to either side. A fragment reads three places a side.
fn skin_embers(
    skin: Skin, p: vec2<f32>, down: vec2<f32>, scale: f32, blade_length: f32, time: f32,
    seed: f32, pixel: f32,
) -> f32 {
    let f = skin.ember_field;
    let shape = skin.ember_shape;
    let seeded = i32(seed * 4096.0);
    var total = 0.0;
    for (var s = 0; s < 2; s++) {
        let side = select(-1.0, 1.0, s == 1);
        var d = down;
        if abs(d.x) < 0.35 {
            d.x = side * 0.35;
        } else if sign(d.x) != side {
            continue;
        }
        d = normalize(d);
        if p.x * side < 0.0 {
            continue;
        }
        let travel_here = p.x / d.x;
        let start = (p.y - travel_here * d.y) / max(scale, 0.0001);
        let k = i32(floor(start * f.y));
        for (var dk = -1; dk <= 1; dk++) {
            let column = k + dk;
            let row = column * 2 + s + seeded * 5;
            if skin_hash(vec2(row, 1717)) >= f.x {
                continue;
            }
            let origin = (f32(column) + skin_hash(vec2(row, 2727))) / f.y;
            if origin < 0.5 || origin > blade_length {
                continue;
            }
            let phase = fract(time / f.w + skin_hash(vec2(row, 3737)));
            let travel = f.z * f.w * phase * phase;
            let stray = shape.y * (skin_hash(vec2(row, 4747)) * 2.0 - 1.0);
            let q = vec2(0.0, origin * scale) + d * travel + vec2(-d.y, d.x) * stray * travel * 0.3;
            let size = shape.x * (1.0 - 0.5 * phase);
            let width = max(size, pixel);
            let e = length(p - q) / width;
            total += exp(-e * e) * pow(1.0 - phase, 1.5) * size / width;
        }
    }
    return total;
}

// SJK's stroke glyphs: sixteen strokes on a 3 × 3 grid (six across, six up, four
// diagonals from the middle), each letter a fixed handful of them drawn from its code.
const GLYPH_STROKES: array<vec4<f32>, 16> = array<vec4<f32>, 16>(
    vec4(0.0, 0.0, 1.0, 0.0), vec4(1.0, 0.0, 2.0, 0.0),
    vec4(0.0, 1.0, 1.0, 1.0), vec4(1.0, 1.0, 2.0, 1.0),
    vec4(0.0, 2.0, 1.0, 2.0), vec4(1.0, 2.0, 2.0, 2.0),
    vec4(0.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 0.0, 2.0),
    vec4(1.0, 0.0, 1.0, 1.0), vec4(1.0, 1.0, 1.0, 2.0),
    vec4(2.0, 0.0, 2.0, 1.0), vec4(2.0, 1.0, 2.0, 2.0),
    vec4(1.0, 1.0, 0.0, 0.0), vec4(1.0, 1.0, 2.0, 0.0),
    vec4(1.0, 1.0, 0.0, 2.0), vec4(1.0, 1.0, 2.0, 2.0),
);

// The strokes letter `code` draws: three to seven of the sixteen, the same every time
// (saber_persona.rs's tests pin that every letter differs).
fn glyph_bits(code: u32) -> u32 {
    var v = code * 747796405u + 2891336453u;
    v = ((v >> ((v >> 28u) + 4u)) ^ v) * 277803737u;
    let h = (v >> 22u) ^ v;
    var w = (code + 101u) * 2654435761u;
    w = w ^ (w >> 15u);
    w = w * 2246822519u;
    w = w ^ (w >> 13u);
    var bits = (h & w) & 0xffffu;
    bits = bits | (1u << (w % 16u)) | (1u << ((w >> 8u) % 16u)) | (1u << ((h >> 4u) % 16u));
    // At most seven strokes: drop the lowest while there are more.
    for (var i = 0; i < 16 && countOneBits(bits) > 7u; i++) {
        bits = bits & (bits - 1u);
    }
    return bits;
}

// Letter `code`'s strokes at `uv` (0 to 1 across and along the glyph), `width` a stroke's
// half-width and `soft` a pixel, both in glyph units.
fn glyph(code: u32, uv: vec2<f32>, width: f32, soft: f32) -> f32 {
    let bits = glyph_bits(code);
    var nearest = 4.0;
    for (var i = 0u; i < 16u; i++) {
        if (bits & (1u << i)) == 0u {
            continue;
        }
        let stroke = GLYPH_STROKES[i];
        let a = vec2(0.12, 0.08) + stroke.xy * vec2(0.38, 0.42);
        let b = vec2(0.12, 0.08) + stroke.zw * vec2(0.38, 0.42);
        let ab = b - a;
        let t = clamp(dot(uv - a, ab) / dot(ab, ab), 0.0, 1.0);
        nearest = min(nearest, length(uv - a - ab * t));
    }
    let half = max(width, soft);
    return (1.0 - smoothstep(half - soft, half + soft, nearest)) * width / half;
}

// Glyphs at `along` units from the hilt and `x` across: the wearer's name, a letter a cell
// of `size` along plus `spacing`, scrolling toward the tip, a cell left empty between
// repeats; only along the blade, not round its tip.
fn skin_glyphs(
    skin: Skin, input: VertexOutput, along: f32, x: f32, time: f32, pixel: f32,
) -> f32 {
    let g = skin.glyph_shape;
    let half_width = g.x * 0.3;
    if g.x <= 0.0 || abs(x) > half_width + pixel || along < 1.0 || along > input.length - 1.0 {
        return 0.0;
    }
    var count = 0u;
    for (var i = 0u; i < 18u; i++) {
        if persona_letter(input, i) == 0u {
            break;
        }
        count = i + 1u;
    }
    let cell = g.x + g.y;
    let k = (along - time * g.z) / cell;
    let v = fract(k) * cell / g.x;
    if v > 1.0 || count == 0u {
        return 0.0;
    }
    let index = i32(floor(k));
    let slot = u32(((index % i32(count + 1u)) + i32(count + 1u)) % i32(count + 1u));
    if slot == count {
        return 0.0;
    }
    let uv = vec2(x / (2.0 * half_width) + 0.5, v);
    let ends = smoothstep(1.0, 3.0, along) * (1.0 - smoothstep(input.length - 3.0, input.length - 1.0, along));
    return glyph(persona_letter(input, slot), uv, g.w * 0.5, pixel / g.x) * ends;
}

// --- The third set (blade_skin_extras.rs): each draws nothing when its lanes are zeros ---

// The world's up in the projected blade's plane (across, along), from `down`; seen from
// straight above or below, where it has no length, the blade's own direction.
fn skin_up(down: vec2<f32>) -> vec2<f32> {
    let size = length(down);
    return select(vec2(0.0, 1.0), -down / max(size, 0.0001), size > 0.001);
}

// Rising noise at `p` (across, along the projected blade, units) for a skin's wisps and
// haze: two octaves of value noise in the plane turned so `up` runs up it, rising `speed`
// units a second and warped by `curl` (the smoke curling as it goes).
fn skin_rising(
    p: vec2<f32>, up: vec2<f32>, scale: f32, speed: f32, curl: f32, time: f32, seed: f32,
) -> f32 {
    let q = vec2(dot(p, vec2(up.y, -up.x)), dot(p, up) - time * speed) * scale
        + vec2(seed * 37.0, seed * 11.0);
    let warp = vec2(skin_noise(q * 0.7 + vec2(3.1, time * 0.21)),
        skin_noise(q * 0.7 + vec2(7.7, -time * 0.17))) - 0.5;
    let w = q + curl * 2.0 * warp;
    return 0.65 * skin_noise(w) + 0.35 * skin_noise(w * 2.3 + vec2(5.2, 1.3));
}

// [`skin_rising`] for Creyon's signature blades (round 12): broader features and its octaves
// turned against each other, so the value noise's square grid never shows.
fn sig_rising(
    p: vec2<f32>, up: vec2<f32>, scale: f32, speed: f32, curl: f32, time: f32, seed: f32,
) -> f32 {
    let q = vec2(dot(p, vec2(up.y, -up.x)), dot(p, up) - time * speed) * scale * 0.7
        + vec2(seed * 37.0, seed * 11.0);
    let r1 = mat2x2<f32>(0.825, 0.565, -0.565, 0.825);
    let r2 = mat2x2<f32>(-0.129, 0.992, -0.992, -0.129);
    let warp = vec2(skin_noise(r1 * q * 0.7 + vec2(3.1, time * 0.21)),
        skin_noise(r2 * q * 0.7 + vec2(7.7, -time * 0.17))) - 0.5;
    let w = q + curl * 2.0 * warp;
    return 0.6 * skin_noise(r2 * w) + 0.4 * skin_noise(r1 * w * 2.1 + vec2(5.2, 1.3));
}

// Where a point `p` is from the blade (the segment from the hilt to `shaft` along it):
// x how high it is above the nearest point of the blade, along `up`, y how far aside.
fn skin_above(p: vec2<f32>, shaft: f32, up: vec2<f32>) -> vec2<f32> {
    let v = p - vec2(0.0, clamp(p.y, 0.0, shaft));
    return vec2(dot(v, up), abs(dot(v, vec2(up.y, -up.x))));
}

// Wisps at `p`: smoke curling off the blade and rising up to `rise` units above it, a
// sheet over a blade held level, a plume over the tip of one held upright, thin along its
// sides; `r` the glow's radius there.
fn skin_wisps(skin: Skin, input: VertexOutput, r: f32, time: f32, seed: f32) -> f32 {
    let w = skin.wisp_shape;
    let up = skin_up(input.down.xy);
    let at = skin_above(input.blade, input.shaft, up);
    let rise = max(w.x, 0.001);
    let risen = clamp(at.x / rise, 0.0, 1.0);
    // Mostly above the blade, wider as it rises, gone at the top.
    let width = r * (0.7 + 1.6 * risen);
    let column = exp(-(at.y * at.y) / (width * width));
    let shown = smoothstep(-0.6 * r, 0.4 * r, at.x) * (1.0 - smoothstep(0.55 * rise, rise, at.x));
    let fall = column * shown * pow(1.0 - risen, 1.5);
    // Outside the column (below the blade, past the top, far aside) the smoke cannot
    // show: skip its four noises there.
    if fall < 1.0 / 1024.0 {
        return 0.0;
    }
    let density = skin.wisp_field.x;
    if skin.design.x == f32(LAB_SIGNATURE) {
        // Creyon's signature blades (round 12: "the smoke looks so squary"): finer, more
        // curled noise with a soft edge, so it reads as smoke rather than square puffs.
        let n = sig_rising(input.blade, up, w.z, w.y, w.w, time, seed);
        let smoke = smoothstep(1.0 - density - 0.2, 1.0 - density + 0.5, n);
        // None over the tip: it rose there as a square-looking plume.
        let below = 1.0 - smoothstep(-0.2 * rise, 0.15 * rise, at.x);
        return smoke * fall * below;
    }
    let n = skin_rising(input.blade, up, w.z, w.y, w.w, time, seed);
    let smoke = smoothstep(1.0 - density, 1.0 - density + 0.3, n);
    return smoke * fall;
}

// The tip's glint at `p` (units from the tip, facing the camera): `rays` rays turning
// `spin` turns a second, every other one shorter, tapering to their ends, and a round glow
// at the heart, twinkling `twinkle` times a second by `depth`.
fn skin_star(skin: Skin, p: vec2<f32>, time: f32, seed: f32, pixel: f32) -> f32 {
    let shape = skin.star_shape;
    let motion = skin.star_motion;
    let size = max(shape.y, 0.001);
    let rays = max(shape.x, 2.0);
    let r = length(p);
    let turn = atan2(p.y, p.x) / TAU + time * motion.x + seed;
    let k = turn * rays;
    let nearest = round(k);
    let reach = select(size, 0.55 * size, (i32(nearest) & 1) == 1 && rays >= 4.0);
    let aside = r * sin(abs(k - nearest) * TAU / rays);
    let along = clamp(1.0 - r / reach, 0.0, 1.0);
    let width = max(shape.z * reach * along, pixel);
    let ray = exp(-(aside * aside) / (width * width)) * along * along
        * (shape.z * reach * along + 0.0001) / width;
    let heart = shape.w * exp(-(r * r) / (0.03 * size * size));
    let twinkle = 1.0 - motion.z * (0.5 + 0.25 * sin(TAU * time * motion.y + seed * 13.0)
        + 0.25 * sin(TAU * time * motion.y * 1.618 + seed * 29.0));
    return (ray + heart) * twinkle;
}

// The glow, `pixel` world units across a screen pixel: the glow at `x` across (the glitch
// and the hologram's jitter move it, and split its colours), then what stays where the
// blade is: embers, glyphs, the sputtering cut and an afterimage's fading.
// --- The fifth set (blade_skin_lab.rs): each draws nothing when its lanes are zeros ------

// `color`'s strongest channel.
fn skin_level(color: vec3<f32>) -> f32 {
    return max(color.r, max(color.g, color.b));
}

// Hue `h` (turns), saturation and value as RGB.
fn skin_hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
    let k = fract(vec3(h) + vec3(0.0, 2.0 / 3.0, 1.0 / 3.0)) * 6.0;
    let rgb = clamp(abs(k - 3.0) - 1.0, vec3(0.0), vec3(1.0));
    return v * mix(vec3(1.0), rgb, s);
}

// How much of a band covers `around` units along and `across` capsule radii out (signed):
// bands `spacing` apart, a `width` share of it wide, slanting by `twist`, running `speed`
// units a second, their edges soft over a pixel.
fn skin_band(skin: Skin, around: f32, across: f32, time: f32, pixel: f32) -> f32 {
    let b = skin.band_shape;
    if b.x <= 0.0 {
        return 0.0;
    }
    let phase = (around - time * b.w) / b.x + b.z * across;
    let d = abs(fract(phase) - 0.5) * 2.0;
    let soft = 0.04 + pixel / b.x;
    return 1.0 - smoothstep(b.y - soft, b.y + soft, d);
}

// The light left of a segmented tube at `along`: a section that stutters on this draw, the
// dark gaps between sections, and a faint hum; 1 without segments.
fn skin_segments(skin: Skin, along: f32, length: f32, time: f32, seed: f32) -> f32 {
    let s = skin.segment_a;
    if s.x < 2.0 {
        return 1.0;
    }
    let q = clamp(along / max(length, 0.0001), 0.0, 0.9999) * s.x;
    let section = floor(q);
    var lit = 1.0;
    if s.y > 0.0 {
        let draw = skin_hash(vec2(i32(section) * 7 + i32(seed * 4096.0), i32(floor(time * s.y))));
        if draw >= s.z {
            lit = 1.0 - s.w;
        }
    }
    let b = skin.segment_b;
    var gap = 1.0;
    if b.x > 0.0 {
        let f = fract(q);
        // No gap before the first section or after the last.
        let below = select(f, 1.0, section < 0.5);
        let above = select(1.0 - f, 1.0, section > s.x - 1.5);
        let edge = min(below, above) * length / s.x;
        gap = smoothstep(0.2 * b.x, b.x, edge);
    }
    let hum = 1.0 - b.y * skin_noise(vec2(time * 23.0, seed * 9.0));
    return lit * gap * hum;
}

// A soap film's colour at `around` units along and `across` radii out.
fn skin_film(skin: Skin, around: f32, across: f32, time: f32) -> vec3<f32> {
    let f = skin.film;
    let p = vec2(around * f.y + time * f.z * 0.2, across * 1.6 - time * f.z * 0.35);
    let n = 0.6 * skin_noise(p) + 0.3 * skin_noise(p * 2.1 + vec2(5.0, 1.0))
        + 0.1 * skin_noise(p * 4.3 + vec2(11.0, 7.0))
        + 0.3 * sin(around * f.y * 2.0 - time * f.z * 0.8);
    return skin_hsv(n * 1.8 + time * 0.03, f.w, 1.0);
}

// Blobs of wax at `along` units (unclamped) and `x` units across: metaballs rising in the
// column, merging where they touch, with pools at the hilt and the tip; 0 to 1.
fn skin_blobs(skin: Skin, along: f32, x: f32, r: f32, length: f32, time: f32, seed: f32) -> f32 {
    let s = skin.blob_shape;
    if s.x < 1.0 {
        return 0.0;
    }
    let seeded = i32(seed * 4096.0);
    let size = s.y * r;
    let margin = 2.0 * size;
    let span = length + 2.0 * margin;
    var field = 0.0;
    for (var i = 0; i < min(i32(s.x), 8); i++) {
        let a = skin_hash(vec2(i, 77 + seeded));
        let b = skin_hash(vec2(i, 131 + seeded));
        let c = skin_hash(vec2(i, 199 + seeded));
        let pace = s.z * (0.6 + 0.8 * a);
        let place = fract(b + time * pace / span) * span - margin;
        let sway = sin(time * 0.6 + f32(i) * 2.0 + c * 6.0) * s.w * r * 0.5;
        let radius = size * (0.7 + 0.6 * c);
        let q = vec2((along - place) * 0.75, x - sway);
        let near = radius * radius / (dot(q, q) + 0.0001);
        // Squared, so a blob's pull falls off fast and they stay apart until they touch.
        field += near * near;
    }
    let pool = skin.blob_more.x * r;
    if pool > 0.0 {
        let hilt = vec2(along * 0.6, x);
        let tip = vec2((along - length) * 0.6, x);
        let low = pool * pool / (dot(hilt, hilt) + 0.0001);
        let high = pool * pool / (dot(tip, tip) + 0.0001);
        field += low * low + high * high;
    }
    return smoothstep(0.85, 1.15, field) * (1.0 - smoothstep(0.85 * r, r, abs(x)));
}

// How far up the blade a front is, a share of it (blade_skin_lab.rs `Front::height`).
fn skin_front_height(skin: Skin, time: f32, seed: f32) -> f32 {
    let s = skin.front_shape;
    if s.x <= 0.0 {
        return 0.0;
    }
    let p = fract(time / s.x + seed * 0.37);
    if p >= s.y {
        return 0.0;
    }
    let a = p / s.y;
    let ramp = max((1.0 - s.z) * 0.5, 0.001);
    return s.w * smoothstep(0.0, ramp, a) * (1.0 - smoothstep(1.0 - ramp, 1.0, a));
}

// How much the front covers `along` units (and `x` across, for its wandering edge): 0 to 1.
fn skin_front(skin: Skin, along: f32, x: f32, length: f32, time: f32, seed: f32) -> f32 {
    let height = skin_front_height(skin, time, seed);
    if height <= 0.0 {
        return 0.0;
    }
    let e = skin.front_edge;
    let line = height * length;
    let jag = e.y * e.x * 2.0 * (skin_noise(vec2(along * 0.35 + seed * 9.0, x * 0.8 + time * 0.7)) - 0.5);
    return (1.0 - smoothstep(line - e.x, line + e.x, along + jag)) * smoothstep(0.0, 0.05, height);
}

// `color` recoloured toward `rgb` at its own level by `amount`.
fn skin_recolour(color: vec3<f32>, rgb: vec3<f32>, amount: f32) -> vec3<f32> {
    return mix(color, rgb * skin_level(color), amount);
}

// --- The sixth set (blade_skin_figures.rs): figures in the glow, nothing when zeros -----

// A thin line's light `d` units from it: a Gaussian `width` wide, never thinner than a
// pixel (a thinner one widens to it and dims, so it does not break up at a distance).
fn skin_line(d: f32, width: f32, pixel: f32) -> f32 {
    let w = max(width, pixel);
    let e = d / w;
    return exp(-e * e) * width / w;
}

// A pencil sketch at `along` units (unclamped) and `x` across: two wobbly outlines and
// hatching, redrawn `rate` times a second; the ink's light (0 and up).
fn skin_sketch(skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32, pixel: f32) -> f32 {
    let s = skin.sketch_shape;
    if s.x <= 0.0 || along < 0.0 || along > blade_length {
        return 0.0;
    }
    let frame = floor(time * s.x);
    // Pointed at the tip over its last few units.
    let taper = 1.0 - smoothstep(blade_length - 2.8, blade_length, along);
    let outline = 0.45 * r * mix(0.08, 1.0, taper);
    let w1 = (skin_noise(vec2(along * 0.35, frame * 3.1)) - 0.5) * s.y * r;
    let w2 = (skin_noise(vec2(along * 0.28 + 7.0, frame * 5.7)) - 0.5) * s.y * r;
    let width = 0.05 * r;
    let l1 = skin_line(abs(abs(x - w1) - outline), width, pixel);
    let l2 = skin_line(abs(abs(x - w2) - outline * 1.05), width * 0.8, pixel) * 0.6;
    var hatch = 0.0;
    if s.z > 0.0 {
        let h = abs(fract((along * 0.7 + x) * s.z + skin_hash(vec2(i32(frame), 17))) - 0.5);
        hatch = smoothstep(0.08, 0.0, h) * step(abs(x), outline * 0.9) * 0.42;
    }
    let grain = 0.72 + 0.28 * skin_noise(vec2(along * 9.0, x * 9.0 + frame));
    return (l1 + l2 + hatch) * grain;
}

// The hourglass's width at `along`: `waist` at the middle, all of it at either end.
fn skin_hourglass_profile(skin: Skin, along: f32, blade_length: f32) -> f32 {
    let w = skin.hour_shape.y;
    let xn = clamp(along / max(blade_length, 0.0001), 0.0, 1.0);
    return w + (1.0 - w) * pow(abs(2.0 * xn - 1.0), 0.75);
}

// The hourglass at `along` (unclamped) and `x`: rgb its sand's share (x), the glass
// rim's (y) and the flip's flash (z), 0 to about 1 each.
fn skin_hourglass(
    skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32, seed: f32, pixel: f32,
) -> vec3<f32> {
    let h = skin.hour_shape;
    if h.x <= 0.0 || along < 0.0 || along > blade_length {
        return vec3(0.0);
    }
    let radius = 0.6 * r * skin_hourglass_profile(skin, along, blade_length);
    let inside = 1.0 - smoothstep(radius * 0.85, radius, abs(x));
    let rim = skin_line(abs(abs(x) - radius), 0.05 * r, pixel) * h.z;
    let p = fract(time / h.x + seed * 0.37);
    let middle = blade_length * 0.5;
    let top_end = middle + (1.0 - p) * blade_length * 0.36;
    let bottom_end = p * blade_length * 0.36;
    let sand_top = step(middle, along) * step(along, top_end);
    let sand_bottom = step(along, bottom_end);
    let grain = 0.7 + 0.3 * skin_noise(vec2(along * 6.0, x * 6.0));
    let stream = step(bottom_end, along) * step(along, middle)
        * skin_line(abs(x), 0.04 * r, pixel)
        * (0.55 + 0.45 * skin_noise(vec2(along * 1.5 - time * 12.0, 0.0))) * step(p, 0.985);
    let flash = exp(-p * 30.0) + exp(-(1.0 - p) * 60.0);
    return vec3((sand_top + sand_bottom) * inside * grain * 1.1 + stream * 1.4, rim,
        flash * inside);
}

// A serpent at `along` (unclamped) and `x`: x its body's light, y how much it hides the
// blade (in front of it), z its eyes', w its tongue's.
fn skin_serpent(
    skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32, pixel: f32,
) -> vec4<f32> {
    let s = skin.serpent_shape;
    if s.x <= 0.0 {
        return vec4(0.0);
    }
    let a = s.y * r;
    let k = TAU * s.x / max(blade_length, 0.0001);
    let w = TAU * s.z;
    let phase = k * along - time * w;
    let y = a * sin(phase);
    let z = cos(phase);
    let slope = a * k * cos(phase);
    let d = abs(x - y) / sqrt(1.0 + slope * slope);
    let head_at = blade_length * 0.9 + sin(time * 1.1) * 0.3;
    let tail = smoothstep(0.02 * blade_length, 0.25 * blade_length, along);
    let thick = s.w * r * mix(0.35, 1.0, tail);
    let on = step(0.02 * blade_length, along) * step(along, head_at - 0.2 * r);
    let body = smoothstep(thick, thick * 0.5, d) * on;
    let scales = 0.72 + 0.28 * sin(along * 6.5 - time * 2.0) * sin((x - y) / max(r, 0.0001) * 30.0);
    let shown = mix(0.28, 1.0, step(0.0, z));
    var light = body * scales * shown * 1.25 + exp(-d / max(0.35 * r, 0.0001)) * 0.22 * on * shown;
    // The head, its jaws opening now and then, two eyes and a tongue.
    let head_y = a * sin(k * head_at - time * w);
    let hp = vec2(along - head_at, x - head_y);
    let size = vec2(max(0.8 * a, 0.9 * r), max(0.45 * a, 0.5 * r));
    let hq = hp / size;
    let jaw = smoothstep(0.7, 1.0, 0.5 + 0.5 * sin(time * 1.7));
    var head = smoothstep(1.0, 0.75, length(hq - vec2(-0.1, 0.0)));
    head *= 1.0 - step(0.3, hq.x) * step(abs(hq.y), 0.5 * jaw);
    light += head * 1.05;
    let eye = 0.09 * size.y;
    let eyes = exp(-length(hp - vec2(0.18 * size.x, 0.4 * size.y)) / max(eye, pixel))
        + exp(-length(hp - vec2(0.18 * size.x, -0.4 * size.y)) / max(eye, pixel));
    let tongue = skin_line(abs(hp.y - sin(time * 40.0) * 0.1 * size.y), 0.04 * size.y, pixel)
        * step(0.9 * size.x, hp.x) * step(hp.x, 1.8 * size.x) * step(0.5, jaw);
    let hide = (body * step(0.0, z) + head) * 0.6;
    return vec4(light, min(hide, 0.9), eyes, tongue);
}

// Glass strands fanning from the hilt at `along` (unclamped) and `x`: their light.
fn skin_strands(
    skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32, seed: f32, pixel: f32,
) -> vec3<f32> {
    let s = skin.strand_shape;
    if s.x < 2.0 || along < -1.0 {
        return vec3(0.0);
    }
    let seeded = i32(seed * 4096.0);
    let xn = clamp(along / max(blade_length, 0.0001), 0.0, 1.2);
    var total = vec3(0.0);
    for (var i = 0; i < min(i32(s.x), 32); i++) {
        let fi = f32(i);
        let spread = skin_hash(vec2(i, 11 + seeded)) * 2.0 - 1.0;
        let tip = blade_length * (0.72 + 0.28 * skin_hash(vec2(i, 23 + seeded)));
        let tn = tip / max(blade_length, 0.0001);
        let colour = skin_hsv(fract(skin.strand_hue.x + skin_hash(vec2(i, 37)) * 0.35 + time * 0.04),
            skin.strand_hue.y, 1.0);
        let sway = sin(time * 1.2 + fi) * 0.25 * s.z * r;
        let y = spread * s.y * r * pow(xn, 1.5) + sway * xn;
        let on = step(0.0, along) * step(along, tip);
        total += colour * skin_line(abs(x - y), 0.03 * r, pixel) * 0.18 * on;
        let tip_y = spread * s.y * r * pow(tn, 1.5) + sway * tn;
        let dd = length(vec2(along - tip, x - tip_y));
        let twinkle = 0.7 + 0.3 * sin(time * 3.0 + fi * 2.0);
        total += colour * (exp(-dd / max(0.1 * r, pixel)) * 1.3 + exp(-dd / (0.45 * r)) * 0.16) * twinkle;
    }
    return total * s.w;
}

// Butterflies resting on the blade and flying off now and then, at `along` (unclamped)
// and `x`: their wings' light.
fn skin_butterflies(
    skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32, seed: f32,
) -> f32 {
    let s = skin.fly_shape;
    if s.x < 1.0 {
        return 0.0;
    }
    let seeded = i32(seed * 4096.0);
    let size = s.y;
    let roam = skin.fly_roam.x;
    var total = 0.0;
    for (var i = 0; i < min(i32(s.x), 8); i++) {
        let fi = f32(i);
        let h1 = skin_hash(vec2(i, 51 + seeded));
        let h2 = skin_hash(vec2(i, 67 + seeded));
        let h3 = skin_hash(vec2(i, 83 + seeded));
        let side = select(-1.0, 1.0, h2 < 0.5);
        var spot = vec2((0.12 + 0.78 * h1) * blade_length, side * 0.8 * r);
        let cycle = 6.0 + 4.0 * h3;
        let p = fract(time / cycle + h1 * 3.7);
        var open = 0.55 + 0.45 * sin(time * 0.8 + fi * 1.7);
        if p < s.w {
            // Out and back on a loop, flapping fast.
            let f = p / max(s.w, 0.0001);
            let away = sin(PI * f);
            spot += roam * away * vec2(0.6 * cos(TAU * f * 1.3 + h3 * 6.0),
                side * (0.6 + 0.4 * sin(TAU * f * 2.0)));
            open = 0.15 + 0.85 * abs(sin(time * s.z * PI + fi));
        }
        let q = vec2(along, x) - spot;
        if dot(q, q) > size * size * 1.2 {
            continue;
        }
        var wings = 0.0;
        for (var w = 0; w < 2; w++) {
            let ws = select(-1.0, 1.0, w == 1);
            let spread = max(open, 0.12);
            let upper = (q - vec2(0.18 * size, ws * 0.3 * size * spread))
                / vec2(0.32 * size, 0.28 * size * spread);
            let lower = (q - vec2(-0.16 * size, ws * 0.24 * size * spread))
                / vec2(0.24 * size, 0.2 * size * spread);
            for (var part = 0; part < 2; part++) {
                let e = select(lower, upper, part == 1);
                let l = length(e);
                // A dim rim and veins, as a monarch's wing.
                let wing = 1.0 - smoothstep(0.85, 1.0, l);
                let rim = smoothstep(1.0, 0.7, l);
                let veins = 0.75 + 0.25 * sin(atan2(e.y, e.x) * 9.0);
                wings = max(wings, wing * (0.35 + 0.65 * rim) * veins);
            }
        }
        total += wings;
    }
    return total;
}

// Wings of fire spreading from near the hilt at `along` (unclamped) and `x`: their light.
fn skin_wings(skin: Skin, along: f32, x: f32, r: f32, blade_length: f32, time: f32) -> f32 {
    let s = skin.wing_shape;
    if s.x <= 0.0 {
        return 0.0;
    }
    let origin = vec2(0.15 * blade_length, 0.0);
    let q = vec2(along, x) - origin;
    if length(q) > s.x * 1.1 {
        return 0.0;
    }
    let unfold = 0.35 + 0.65 * smoothstep(-0.3, 0.6, sin(TAU * time * s.y));
    let feathers = i32(s.z);
    var total = 0.0;
    for (var w = 0; w < 2; w++) {
        let ws = select(-1.0, 1.0, w == 1);
        for (var f = 0; f < feathers; f++) {
            let k = f32(f) / max(f32(feathers - 1), 1.0);
            // Fanned from along the blade toward its side as the wing opens.
            let angle = (0.25 + 1.05 * k) * unfold;
            let dir = vec2(cos(angle), ws * sin(angle));
            let span = s.x * (0.55 + 0.45 * (1.0 - abs(k - 0.6)));
            let t = clamp(dot(q, dir) / span, 0.0, 1.0);
            let d = length(q - dir * t * span);
            let width = (0.12 + 0.18 * sin(PI * t)) * r;
            let flame = 0.6 + 0.6 * skin_noise(vec2(t * 6.0 - time * 3.0, f32(f) * 3.1 + ws * 7.0));
            total += smoothstep(width, width * 0.3, d) * (1.0 - 0.6 * t) * flame * unfold;
        }
    }
    return min(total, 2.0);
}

// An afterimage (`afterimage`): the glow alone, at its mean. The capsule unwidened in the
// corona's colours at their middle heat (the granulation's noise at its middle, no flare,
// the tongues' mean flicker), beating, tinted and turned as the blade is, without the
// noise, flares, figures, wisps or motes: a swing's afterimages cost about what stock
// glows do, where each was the whole blade's work over its widest quad (hub report 64).
fn afterimage_glow(input: VertexOutput) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let along = skin_along(input);
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let x = input.blade.x;
    let beyond = max(input.blade.y - input.shaft, 0.0);
    let out = select(abs(x), length(vec2(x, beyond)), beyond > 0.0) / r;
    let mixed = skin.grain_mix;
    let grain = smoothstep(mixed.z, mixed.w, 0.5 * (mixed.x + mixed.y));
    let beat = skin_beat(skin, time);
    let tb = skin.tongue_b;
    let flicker = mix(1.0, tb.z + 0.5 * tb.w, smoothstep(tb.x, tb.y, out));
    let inner = 1.0 - smoothstep(0.0, skin.rim_cool.w, out);
    let rim = mix(skin.rim_cool.rgb, skin.rim_hot.rgb, clamp(grain * skin.rim_hot.w, 0.0, 1.0));
    let share = skin.inner_mix;
    let color = mix(rim, skin.inner.rgb,
        clamp(inner * inner * (share.x + share.y * grain), 0.0, 1.0));
    let light = skin.brightness;
    var glow = glow_capsule(input) * color * (light.x + light.y * grain) * flicker
        * (1.0 + beat);
    glow = skin_apply_tint(glow, skin_tint(skin, input));
    let hue = skin.hue;
    if any(hue.xyz != vec3(0.0)) {
        glow = skin_turn_hue(glow, time * hue.x + along * hue.y + out * hue.z);
    }
    if input.chroma != 0.0 {
        glow = skin_turn_hue(glow, input.chroma);
    }
    return glow * persona_fade(input);
}

// Whether a pixel of a skin's glow quad is past everything the glow itself draws (the
// capsule widened at most `corona.reach` times, round the tip and behind the hilt, which
// the embers, arcs and motes fade out before): only the quad's room for the wisps (or
// haze, butterflies, wings) reaches there. Never for a skin whose glow moves or reaches
// further (glitch, the hologram's jitter or scan, copies, glyphs, the figures), which
// then shades every pixel as before. A blade up close fills the screen with that room
// (hub report 63).
fn skin_past_glow(skin: Skin, input: VertexOutput, beyond: f32) -> bool {
    let plain = skin.glitch_a.y <= 0.0 && skin.scan_jitter.x <= 0.0 && skin.scan_lines.x <= 0.0
        && skin.copy_shape.x < 1.0 && skin.glyph_color.w <= 0.0 && skin.sketch_shape.x <= 0.0
        && skin.hour_shape.x <= 0.0 && skin.serpent_shape.x <= 0.0
        && skin.strand_shape.x < 2.0 && skin.blob_shape.x < 1.0 && skin.fly_shape.x < 1.0
        && skin.wing_shape.x <= 0.0;
    // A signature shader draws wherever its quad reaches.
    if !plain || skin.design.x == f32(LAB_SIGNATURE) {
        return false;
    }
    let reach = max(chain_radius(input.radius, input.length, 0.0), input.hilt) * skin.swell.x;
    // Behind the hilt the arcs may still strike.
    let behind = input.blade.y < -reach && skin.arc_strike.x <= 0.0;
    return abs(input.blade.x) > reach || beyond > reach || behind;
}

fn skin_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {
    if afterimage(input.persona.x, input.off.x) {
        return afterimage_glow(input);
    }
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let beyond = max(input.blade.y - input.shaft, 0.0);
    // Past the glow's own reach the quad holds only the room for the wisps (or haze,
    // butterflies, wings), where the glow and its parts draw nothing: skip them there.
    var glow = vec3(0.0);
    if !skin_past_glow(skin, input, beyond) {
        let glitch = skin_glitch(skin, along, time, seed);
        let x = input.blade.x - (glitch.x + skin_jitter(skin, time, seed)) * r;
        if glitch.y > 0.0 {
            let split = glitch.y * r;
            glow = vec3(
                skin_glow_at(input, pixel, x + split).r,
                skin_glow_at(input, pixel, x).g,
                skin_glow_at(input, pixel, x - split).b,
            ) * (1.0 + glitch.z);
        } else {
            glow = skin_glow_at(input, pixel, x);
        }
        let copies = skin.copy_shape;
        if copies.x >= 1.0 {
            // Ghost copies at other angles drifting apart, then collapsing with a flash.
            let p = fract(time / copies.w + seed * 0.37);
            let apart = smoothstep(0.0, 0.85, p) * (1.0 - smoothstep(0.92, 1.0, p));
            let flicker = 1.0 - skin.copy_more.x * skin_hash(vec2(i32(floor(time * 12.0)), 4141));
            var ghosts = vec3(0.0);
            // Their angle in the quad's own width (an angle of 0.1 reaches 0.6 of it at the
            // tip, the outer copy), so they stay inside it whatever the blade's size.
            let reach = skin_extent(skin, input) * 0.6 * copies.y / 0.1 / max(copies.x, 1.0);
            for (var c = 1; c <= min(i32(copies.x), 2); c++) {
                for (var s = 0; s < 2; s++) {
                    let offset = select(-1.0, 1.0, s == 1) * f32(c) * reach * apart
                        * along / max(input.length, 0.0001);
                    // Its glow, and a thin bright line of its own core.
                    ghosts += (skin_glow_at(input, pixel, x - offset)
                        + skin_line(abs(x - offset), 0.1 * input.radius, pixel) * skin.core_white.rgb * 1.5)
                        / f32(c);
                }
            }
            glow = glow * (1.0 + 0.8 * exp(-p * 25.0)) + ghosts * copies.z * flicker * apart;
        }
        let extent = skin_extent(skin, input);
        let room = (1.0 - smoothstep(0.75, 1.0, abs(input.blade.x) / extent))
            * (1.0 - smoothstep(0.75, 1.0, beyond / extent))
            * (1.0 - smoothstep(0.75, 1.0, -min(input.blade.y, 0.0) / extent));
        if skin.ember_field.x > 0.0 {
            let scale = input.shaft / max(input.length, 0.0001);
            let embers = skin_embers(skin, input.blade, input.down.xy, scale, input.length, time, seed,
                pixel);
            glow += embers * room * skin.ember_color.rgb * skin.ember_color.w;
        }
        if skin.glyph_color.w > 0.0 {
            glow += skin_glyphs(skin, input, along, x, time, pixel) * skin.glyph_color.rgb
                * skin.glyph_color.w;
        }
    }
    if skin.wisp_color.w > 0.0 {
        var smoke = skin_wisps(skin, input, r, time, seed) * skin.wisp_color.rgb
            * skin.wisp_color.w;
        smoke = skin_apply_tint(smoke, skin_tint(skin, input));
        if input.chroma != 0.0 {
            smoke = skin_turn_hue(smoke, input.chroma);
        }
        glow += smoke;
    }
    let cut = skin_cut(skin, input.length, time, seed);
    if cut < input.length {
        // Past the cut nothing shows, the embers already falling aside.
        let reach = select(along, input.length + beyond, beyond > 0.0);
        glow *= 1.0 - smoothstep(cut - 1.5, cut + 0.5, reach);
    }
    return glow * persona_fade(input);
}

// The glow at `x` across (the fragment's own, or moved by a glitch).
fn skin_glow_at(input: VertexOutput, pixel: f32, x: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let unrest = skin_unrest(skin, along);
    let grain = mix(GRAIN_CALM, skin_granulation(skin, along, time, seed), unrest);
    let flare = skin_flares(skin, along, input.length, time, seed) * unrest;
    let beat = skin_beat(skin, time);
    // Shimmer and swell: the corona breathes a little, a flare widens it, a beat too.
    let shimmer = (skin_wave(skin.shimmer_a, time, along, seed)
        + skin_wave(skin.shimmer_b, time, along, seed)) * unrest;
    let swell = skin.swell;
    var widen = min(1.0 + shimmer + swell.y * (grain - 0.5) + swell.z * flare
        + skin.pulse_b.y * beat, swell.x);
    // Past the tip (`beyond`, projected units) the capsule ends round: it widens about the
    // tip as it widens about the shaft, and the distance out is taken from the tip, so the
    // corona's grading and its tongues go round the end instead of running on straight.
    let beyond = max(input.blade.y - input.shaft, 0.0);
    let r = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let radial = select(abs(x), length(vec2(x, beyond)), beyond > 0.0);
    let around = select(along, along + r * atan2(beyond, max(abs(x), 0.0001)), beyond > 0.0);
    let ragged = skin.sputter_edge;
    if ragged.x > 0.0 {
        // A frayed edge: the width wanders by noise, each side its own.
        let fray = skin_noise(vec2(around * ragged.y - time * ragged.z,
            time * ragged.z * 0.37 + seed * 13.0 + select(0.0, 50.0, x < 0.0)));
        widen = clamp(widen * (1.0 + ragged.x * unrest * (fray * 2.0 - 1.0)), 0.3, swell.x);
    }
    // Rings squeezing the glow where the bands pass (taken on the axis, so they stay
    // round).
    let pinch = skin.band_mix.y;
    if pinch > 0.0 {
        widen *= 1.0 - pinch * skin_band(skin, around, 0.0, time, pixel);
    }
    // An hourglass's pinched waist.
    if skin.hour_shape.x > 0.0 {
        widen *= 0.2 + 0.8 * skin_hourglass_profile(skin, along, input.length);
    }
    var shaded = input;
    shaded.blade = vec2(x / widen, min(input.blade.y, input.shaft) + beyond / widen);
    let capsule = glow_capsule(shaded) / widen;
    let out = radial / (r * widen);
    // Flame tongues licking outward in the corona's edge.
    let ta = skin.tongue_a;
    let tb = skin.tongue_b;
    // Slanted toward the tip as they go out, when the skin leans them.
    let slant = around - out * skin.unrest_lean.x;
    let tongue = mix(0.5,
        skin_noise(vec2(slant * ta.x + seed * ta.y, out * ta.z - time * ta.w)), unrest);
    let edge = smoothstep(tb.x, tb.y, out);
    let flicker = mix(1.0, tb.z + tb.w * tongue, edge);
    // The inside colour by the core, the rim's at the edge; granulation and flares heat it.
    let inner = 1.0 - smoothstep(0.0, skin.rim_cool.w, out);
    let rim = mix(skin.rim_cool.rgb, skin.rim_hot.rgb,
        clamp(grain * skin.rim_hot.w + flare * skin.inner.w, 0.0, 1.0));
    let share = skin.inner_mix;
    let color = mix(rim, skin.inner.rgb,
        clamp(inner * inner * (share.x + share.y * grain) + flare * share.z, 0.0, 1.0));
    let light = skin.brightness;
    var brightness = (light.x + light.y * grain + light.z * flare) * flicker * (1.0 + beat);
    var colour = color;
    let band = skin_band(skin, around, x / max(r * widen, 0.0001), time, pixel);
    if band > 0.0 {
        colour = mix(colour, skin.band_color.rgb, skin.band_mix.x * band);
        brightness *= mix(1.0, skin.band_color.w, band);
    }
    var glow = capsule * colour * brightness;
    // Where a front covers the blade (veins may be kept to it).
    let front = skin_front(skin, along, x, input.length, time, seed);
    if skin.vein_color.w > 0.0 {
        // A signature's veins start at the hilt's mouth (Creyon's round 20: Cockpit Console's
        // circuits reached below the hilt).
        let veins = skin_veins(skin, along, x, time, seed)
            * mix(1.0, front, skin.front_edge.z) * sig_from_mouth(skin, along);
        glow += veins * (1.0 - smoothstep(0.15, 0.6, out)) * capsule.r * skin.vein_color.rgb
            * skin.vein_color.w * (1.0 + beat);
    }
    if skin.blob_shape.x >= 1.0 {
        let unclamped = input.blade.y / max(input.shaft, 0.0001) * input.length;
        glow += skin_blobs(skin, unclamped, x, r, input.length, time, seed)
            * skin.blob_color.rgb * skin.blob_color.w;
    }
    if skin.film.x > 0.0 {
        // A bubble: clear inside, bright at its rim, in the film's sliding colours.
        let tint = skin_film(skin, around, x / max(r * widen, 0.0001), time);
        let rim = exp(-pow((out - 0.8) / 0.22, 2.0));
        glow *= (1.0 - skin.film_b.x * (1.0 - smoothstep(0.1, 0.7, out)))
            * (1.0 + 1.5 * skin.film_b.x * rim);
        glow = skin_recolour(glow, tint, skin.film.x);
    }
    if front > 0.0 {
        glow = skin_recolour(glow, skin.front_color.rgb, skin.front_color.w * front);
    }
    // The sixth set's figures, on the blade's place along it unclamped.
    let place = input.blade.y / max(input.shaft, 0.0001) * input.length;
    if skin.sketch_shape.x > 0.0 {
        glow = glow * skin.sketch_shape.w + skin_sketch(skin, place, x, r, input.length, time, pixel)
            * skin.sketch_color.rgb * skin.sketch_color.w;
    }
    if skin.hour_shape.x > 0.0 {
        let h = skin_hourglass(skin, place, x, r, input.length, time, seed, pixel);
        glow = glow * (0.45 + 1.5 * h.z) + h.x * skin.hour_color.rgb * skin.hour_color.w
            + h.y * skin.rim_hot.rgb;
    }
    if skin.serpent_shape.x > 0.0 {
        let s = skin_serpent(skin, place, x, r, input.length, time, pixel);
        glow = glow * (1.0 - s.y) + s.x * skin.serpent_color.rgb * skin.serpent_color.w
            + s.z * vec3(2.2, 0.18, 0.08) + s.w * vec3(1.0, 0.35, 0.4);
    }
    if skin.strand_shape.x >= 2.0 {
        glow += skin_strands(skin, place, x, r, input.length, time, seed, pixel);
    }
    if skin.fly_shape.x >= 1.0 {
        glow += skin_butterflies(skin, place, x, r, input.length, time, seed)
            * skin.fly_color.rgb * skin.fly_color.w;
    }
    if skin.wing_shape.x > 0.0 {
        let fire = skin_wings(skin, place, x, r, input.length, time);
        glow += fire * mix(skin.wing_color.rgb, vec3(1.0, 0.9, 0.6), smoothstep(1.2, 2.0, fire))
            * skin.wing_color.w;
    }
    let scan = skin.scan_lines;
    if scan.x > 0.0 {
        // A hologram: scan lines darken it as they run, its inside is hollow, and a
        // wireframe (side lines and rings) is drawn over it.
        let lines = 1.0 - scan.z * (0.5 + 0.5 * sin(TAU * (around * scan.x - time * scan.y)));
        let e = skin.scan_edge;
        glow *= lines * (1.0 - e.z * (1.0 - smoothstep(0.0, e.x, out)));
        let soft = pixel / max(r * widen, 0.0001);
        // Not behind the hilt, where the place along stops at the hilt.
        let shown = smoothstep(-pixel, 0.0, input.blade.y);
        let sides = (1.0 - smoothstep(e.y, e.y + soft, abs(out - e.x))) * shown;
        var rings = 0.0;
        if scan.w > 0.0 {
            let f = fract(around * scan.w);
            let units = min(f, 1.0 - f) / scan.w;
            let half = e.y * r * 0.5;
            rings = (1.0 - smoothstep(half, half + pixel, units))
                * (1.0 - smoothstep(e.x, e.x + soft, out)) * shown;
        }
        glow += (sides + rings) * lines * skin.scan_color.rgb * skin.scan_color.w;
    }
    // A signature's echo (a faded copy swaying beside it) is its blade alone (Creyon's round
    // 12: the echo's own prominences were cut off at its edge and drawn at full strength).
    let sig_echo = skin.design.x == f32(LAB_SIGNATURE) && (input.persona.x >> 24u) != 255u;
    if (skin.arc_strike.x > 0.0 || skin.mote_field.x > 0.0) && !sig_echo {
        // Both fade out before the quad's edge (vertex_main's `extent`) instead of being cut.
        // A signature's quad also holds its room (Creyon's round 12: its prominences were
        // cut off at the stock glow's width).
        let extent = skin_extent(skin, input);
        var faded = extent;
        if skin.design.x == f32(LAB_SIGNATURE) {
            faded += max(skin.design.y, skin.design.z) / LAB_METRE;
        }
        let room = (1.0 - smoothstep(0.75, 1.0, abs(x) / faded))
            * (1.0 - smoothstep(0.75, 1.0, beyond / faded));
        if skin.arc_strike.x > 0.0 && skin.design.x != f32(LAB_SIGNATURE) {
            // Along the axis and straight on past the tip.
            let reach = select(input.blade.y / max(input.shaft, 0.0001) * input.length,
                input.length + beyond, beyond > 0.0);
            // A clashing blade's arcs alone reach further up (`unrest.clash`).
            var struck = skin;
            let left = max(0.0, 1.0 - input.motion.y / max(skin.unrest_clash.x, 0.01));
            struck.unrest.y += skin.unrest_lean.w * left * left;
            let arcs = skin_arcs(struck, reach, x, r, input.length, extent, pixel, time, seed)
                * skin_unrest(struck, reach);
            glow += arcs * room * skin.arc_color.rgb * skin.arc_color.w;
        }
        if skin.mote_field.x > 0.0 {
            var unclamped = input.blade.y / max(input.shaft, 0.0001) * input.length;
            var mote_around = around;
            var mote_out = radial / r;
            if skin.design.x == f32(LAB_SIGNATURE) {
                // along the blade itself, as `sig_arcs` (round 12)
                let place = blade_place(input);
                unclamped = place.y;
                // Round 16 (Creyon: a signature's motes were cut in half at the tip): straight
                // on past the tip, not round its end, whose seam split a mote crossing it.
                mote_around = place.y;
                mote_out = abs(place.x) / r;
            }
            var motes = skin_motes(skin, mote_around, mote_out, x < 0.0, unclamped,
                input.length, time, seed);
            if skin.design.x == f32(LAB_SIGNATURE) {
                // Round 17 (Creyon: the motes rose far above the tip and spread so wide they
                // went through the wearer's head): they fade out over the blade's last units,
                // none past its tip, and stay within a hand's reach of it.
                let place = blade_place(input);
                motes *= (1.0 - smoothstep(input.length - 4.0, input.length - 0.5, place.y))
                    * (1.0 - smoothstep(2.0, 3.2, abs(place.x)));
            }
            glow += motes * room * skin.mote_color.rgb * skin.mote_color.w;
        }
    }
    let halftone = skin.ink_more.y;
    if halftone > 0.0 {
        // The glow printed as halftone dots, bigger where it is brighter; the inside stays
        // solid, as a comic's flat colour.
        let level = clamp(skin_level(glow) / 1.2, 0.0, 1.0);
        let cell = fract(vec2(around, x) * halftone) - 0.5;
        let dots = clamp((0.5 * sqrt(level) - length(cell)) / max(pixel * halftone, 0.02) + 0.5,
            0.0, 1.0);
        let printed = smoothstep(0.35, 0.6, out);
        glow = mix(glow, glow / max(level, 0.05) * dots * 0.9, printed);
    }
    glow *= skin_segments(skin, along, input.length, time, seed);
    glow = skin_apply_tint(glow, skin_tint(skin, input));
    let hue = skin.hue;
    if any(hue.xyz != vec3(0.0)) {
        glow = skin_turn_hue(glow, time * hue.x + around * hue.y + radial / r * hue.z);
    }
    // A signature shader takes the glow the file drew and draws the blade its own way.
    if skin.design.x == f32(LAB_SIGNATURE) {
        sig_take(input);
        let place = sig_place(input);
        // The file's glow round the emitter stays, as a stock blade's covers the hilt's
        // mouth.
        let mouth = (1.0 - smoothstep(0.0, 2.2, length(vec2(place.x * 0.6, max(place.y, 0.0))))) * 0.75;
        // What its glow draws past the tip the blade has grown to waits until the blade is
        // out (Creyon's round 12: a Star Dragon's coils stood at full length while the
        // blade was still igniting).
        let shown_glow = max(sig_full(input),
            1.0 - smoothstep(input.length - 1.0, input.length + 1.0, place.y));
        let own = max(sig_glow(glow, place + vec2(x - input.blade.x, 0.0), r, pixel), vec3(0.0))
            * sig_kept(input, place);
        glow = mix(glow, own, shown_glow) + glow * mouth * step(-6.0, place.y);
        // Round 20 (Creyon: close up, the blade's glow came to the front and its effects hid
        // behind it; from afar they showed): the glow round the blade (not its core, drawn on
        // its own) softens as the camera comes near, to about half within a few metres.
        let to_hilt = input.hilt_at;
        let axis_dir = normalize(input.axis.xyz);
        let along_axis = clamp(-dot(to_hilt, axis_dir), 0.0, input.length);
        let distance = length(to_hilt + axis_dir * along_axis);
        let near = 1.0 - smoothstep(40.0, 140.0, distance);
        glow *= 1.0 - 0.45 * near;
        let kept_glow = max(uKeep, vec3(0.0)) * shown_glow;
        uKeep = vec3(0.0);
        // Every effect round the blade is drawn in three dimensions along the true view ray
        // in the hilt's frame (Creyon's round 19: seen from above, the flat ones followed the
        // camera), so it is the same thing from every side.
        if sig_echo {
            return sig_coloured(input, glow * sig_edge(input)) * persona_fade(input);
        }
        let shown = sig_full(input);
        var own_effects = vec3(0.0);
        // In a view its effects in the air are drawn apart, at half size (`fragment_volume`).
        if !SIG_SPLIT {
            let air = sig_air(input, skin, place, pixel);
            glow += air.shared_light;
            own_effects = air.own;
        }
        if SIG_OWN_EFFECTS {
            glow *= 1.0 + 0.3 * smoothstep(55.0, 240.0, length(input.ray));
        }
        let edge = sig_edge(input);
        return sig_coloured(input, (glow + max(uTurnFx, vec3(0.0)) * shown) * edge) + (kept_glow + max(uKeep, vec3(0.0)) * shown + sig_coloured_air(input, own_effects)) * edge;
    }
    // A chroma takes its wearer's colour: everything it drew, turned at once.
    if input.chroma != 0.0 {
        glow = skin_turn_hue(glow, input.chroma);
    }
    return glow;
}

// The core line: `uv` its texture coordinates (`core_coordinates`) and `footprint` how far
// they move in a pixel, which softens the rounded tip's edge.
fn skin_core(input: VertexOutput, texel: vec4<f32>, uv: vec2<f32>, footprint: f32) -> vec3<f32> {
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let along = skin_along(input);
    let unrest = skin_unrest(skin, along);
    let grain = mix(GRAIN_CALM, skin_granulation(skin, along, time, seed), unrest);
    let flare = skin_flares(skin, along, input.length, time, seed) * unrest;
    let beat = skin_beat(skin, time);
    let bright = skin.core_fringe;
    var fringe = mix(skin.core_fringe_cool.rgb, skin.core_fringe_hot.rgb,
        skin.core_fringe_cool.w + skin.core_fringe_hot.w * grain)
        * (bright.x + bright.y * grain + bright.z * flare);
    fringe = skin_apply_tint(fringe, skin_tint(skin, input));
    let hue = skin.hue;
    if any(hue.xyz != vec3(0.0)) {
        fringe = skin_turn_hue(fringe, time * hue.x + along * hue.y);
    }
    var core = CORE_DRAWS * (skin.core_white.rgb * texel.r * (1.0 + skin.core_white.w * flare)
        + fringe * texel.g) * (1.0 + beat);
    let x = (uv.x - 0.5) * 2.0 * input.radius;
    let front = skin_front(skin, along, x, input.length, time, seed);
    if skin.vein_color.w > 0.0 {
        core += skin_veins(skin, along, x, time, seed) * (texel.r + texel.g) * skin.vein_color.rgb
            * skin.vein_shape.w * (1.0 + beat) * mix(1.0, front, skin.front_edge.z)
            * sig_from_mouth(skin, along);
    }
    // The fifth set: the bands' colour (on the line's own axis), the front's, the
    // segments' stutter.
    let band = skin_band(skin, along, 0.0, time, 0.0);
    if band > 0.0 {
        core = skin_recolour(core, skin.band_color.rgb, skin.band_mix.x * band)
            * mix(1.0, skin.band_color.w, band);
    }
    if front > 0.0 {
        core = skin_recolour(core, skin.front_color.rgb, skin.front_color.w * front);
    }
    core *= skin_segments(skin, along, input.length, time, seed);
    // The sixth set: a sketch keeps its glow's share of the line, an hourglass's glass and
    // a bundle of strands keep a faint one (their sand and fibres carry the light), a
    // serpent hides it where its body passes in front.
    if skin.sketch_shape.x > 0.0 {
        core *= skin.sketch_shape.w;
    }
    if skin.hour_shape.x > 0.0 || skin.strand_shape.x >= 2.0 {
        core *= 0.25;
    }
    if skin.serpent_shape.x > 0.0 {
        core *= 1.0 - skin_serpent(skin, along, x, input.radius * 3.0, input.length, time, 0.0).y;
    }
    let scan = skin.scan_lines;
    if scan.x > 0.0 {
        core *= 1.0 - scan.z * (0.5 + 0.5 * sin(TAU * (along * scan.x - time * scan.y)));
    }
    // In the rounded tip the line is cut at the narrowing edge `core_coordinates` maps to the
    // texture's border (which the sampler would otherwise smear out to the quad's corners).
    if along > input.length - skin.core_fringe.w * input.radius {
        core *= clamp((0.5 - abs(uv.x - 0.5)) / max(footprint, 0.00001) + 0.5, 0.0, 1.0);
    }
    let cut = skin_cut(skin, input.length, time, seed);
    if cut < input.length {
        core *= 1.0 - smoothstep(cut - 1.5, cut + 0.5, along);
    }
    // A signature shader's core, which never runs on past the tip.
    if skin.design.x == f32(LAB_SIGNATURE) {
        sig_take(input);
        core = max(sig_core(core, vec2(x, along), footprint), vec3(0.0))
            * (1.0 - smoothstep(input.length - 0.2, input.length + 0.4, along));
        // Round 23 (players' reports after the 2026.1021.1 release): the line fades out before
        // its quad's sides. The core image's fringe is still a tenth as bright at its border,
        // and a signature's softer glow no longer hid the straight edge that left along both
        // sides of the line up close, nor the faint box past the narrowed tip, where the line's
        // coordinates run out of the image and it repeats its border.
        core *= 1.0 - smoothstep(0.36, 0.5, abs(uv.x - 0.5));
        return sig_coloured(input, core);
    }
    if input.chroma != 0.0 {
        core = skin_turn_hue(core, input.chroma);
    }
    return core;
}

// The core line's texture coordinates; a skin's core breathes a little across, narrows to
// its rounded tip and moves with a glitch or the hologram's jitter (in core half-widths,
// the glow's radius about three of them).
fn core_coordinates(input: VertexOutput) -> vec2<f32> {
    let line = core_line(input);
    return core_coordinates_at(input, line, line.stretch);
}

// The core line's texture coordinates stretched across by `stretch` and moved as `line` says.
fn core_coordinates_at(input: VertexOutput, line: CoreLine, stretch: f32) -> vec2<f32> {
    let across = input.blade.x / (2.0 * input.radius) * stretch - line.moved;
    return vec2(0.5 + across, 1.0 - input.blade.y);
}

// What a skin does to its core line at this fragment, worked out once (one skin lookup):
// `fragment_main` takes both stretches, the glitch's move and its colour split from it.
struct CoreLine {
    // The breathing across, without the rounded tip's narrowing (1 for a stock line).
    breath: f32,
    // The breathing with the rounded tip, where the width left goes to nothing.
    stretch: f32,
    // How far a glitch or the hologram's jitter moves the line across (texture units).
    moved: f32,
    // The colour split a glitch gives the core (texture units across).
    split: f32,
}

fn core_line(input: VertexOutput) -> CoreLine {
    if input.kind < KIND_SKIN {
        return CoreLine(1.0, 1.0, 0.0, 0.0);
    }
    let along = mix(-1.0, input.length, input.blade.y);
    let skin = skin_at(input);
    let time = input.animation.x;
    let seed = input.animation.y;
    let breath = 1.0 + skin_wave(skin.core_breathe, time, along, seed);
    let stretch = breath
        / skin_tip_taper(along, input.length, skin.core_fringe.w * input.radius);
    let glitch = skin_glitch(skin, along, time, seed);
    let moved = 1.5 * (glitch.x + skin_jitter(skin, time, seed));
    return CoreLine(breath, stretch, moved, 1.5 * glitch.y);
}

// Glint n's size: every other one of an orbit is smaller, a companion to the first.
fn glint_scale(n: f32) -> f32 {
    return select(1.0, 0.7, (i32(round(n)) & 1) == 1);
}

// A black hole's light at `p` (units from its heart, facing the camera): a thin photon
// ring just outside the horizon and a fainter accretion swirl round it, two spiral arms
// turning `spin` turns a second; nothing inside the horizon.
fn hole_ring(skin: Skin, p: vec2<f32>, time: f32, seed: f32, pixel: f32) -> f32 {
    let size = skin.hole_shape.x;
    let r = length(p);
    let width = max(0.07 * size, pixel);
    let ring = exp(-pow((r - 1.15 * size) / width, 2.0));
    let turn = atan2(p.y, p.x) / TAU;
    let arms = 0.5 + 0.5 * sin(TAU * (2.0 * turn + time * skin.hole_shape.z + seed)
        + 2.5 * r / size);
    let swirl = exp(-pow((r - 1.7 * size) / (0.55 * size), 2.0)) * (0.3 + 0.7 * arms) * 0.6;
    return (ring + swirl) * smoothstep(0.9 * size, 1.05 * size, r);
}

// A skin's glint, coloured as its glow is (the wearer's tint and chroma, an afterimage's
// fading), dimmed behind the blade; each of an orbit twinkles and turns at its own pace.
// A black hole's ring and swirl instead when the skin has holes.
// The emitter's crackling star at `p` (units from its heart, facing the camera, +y up the
// blade): `rays` thin rays round a hot heart, shuffled `rate` times a second, their angles
// and lengths thrown about by `chaos` and their light flickering by it; `lean` stretches
// the rays up the blade and shortens the ones pointing back.
fn skin_flash(skin: Skin, p: vec2<f32>, time: f32, seed: f32, pixel: f32) -> f32 {
    let shape = skin.flash_shape;
    let motion = skin.flash_motion;
    let size = max(shape.y, 0.001);
    let rays = clamp(i32(shape.x), 2, MAX_FLASH_RAYS);
    let chaos = motion.y;
    let lean = motion.w;
    let shuffle = i32(floor(time * motion.x));
    let row = i32(seed * 4096.0);
    let turn = skin_hash(vec2(shuffle, row + 5)) * TAU;
    var total = 0.0;
    for (var i = 0; i < MAX_FLASH_RAYS; i++) {
        if i >= rays {
            break;
        }
        let h = skin_hash(vec2(shuffle, row + i * 31 + 11));
        let h2 = skin_hash(vec2(shuffle, row + i * 31 + 17));
        let h3 = skin_hash(vec2(shuffle, row + i * 31 + 23));
        let angle = turn + TAU * (f32(i) + chaos * (h - 0.5)) / f32(rays);
        let way = vec2(cos(angle), sin(angle));
        let reach = size * (1.0 - 0.75 * chaos * h2) * (1.0 + lean * way.y);
        let t = dot(p, way);
        if t <= 0.0 || t >= reach {
            continue;
        }
        let aside = abs(p.x * way.y - p.y * way.x);
        let along = 1.0 - t / reach;
        let width = max(shape.z * reach * along, pixel);
        total += exp(-(aside * aside) / (width * width)) * along * along
            * (shape.z * reach * along + 0.0001) / width * (1.0 - 0.6 * chaos * h3);
    }
    let r = length(p);
    let heart = shape.w * exp(-(r * r) / (0.03 * size * size));
    let flicker = 1.0 - 0.6 * chaos * skin_hash(vec2(shuffle, row + 3));
    return (total + heart) * flicker;
}

fn star_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {
    let skin = skin_at(input);
    if input.hilt <= FLASH + 0.5 {
        // The emitter's crackling star.
        if skin.flash_color.w <= 0.0 {
            return vec3(0.0);
        }
        let f = FLASH - input.hilt;
        // Bigger and brighter as it surges once lit (its instance's red lane).
        let surge = input.color.r;
        let grown = 1.0 + skin.flash_place.z * surge;
        var bright = 1.0 + skin.flash_place.y * surge;
        // Sputtering out once put out (the green lane, what is left): at each shuffle out
        // or flaring, out more often and dimmer as it dies.
        let dying = input.color.g;
        if dying > 0.0 {
            let shuffle = i32(floor(input.animation.x * skin.flash_motion.x));
            let row = i32((input.animation.y + f * 0.37) * 4096.0);
            // Alone, without the blade's glow round it, a flare shows brighter than its
            // crackle did.
            let on = skin_hash(vec2(shuffle, row + 53)) < 0.3 + 0.5 * dying;
            bright *= select(0.0, 1.5 + 2.0 * skin_hash(vec2(shuffle, row + 59)), on)
                * sqrt(dying);
        }
        var flash = skin_flash(skin, input.blade / grown, input.animation.x,
            input.animation.y + f * 0.37, pixel / grown) * skin.flash_color.rgb
            * skin.flash_color.w * bright;
        flash = skin_apply_tint(flash, skin_tint(skin, input));
        if input.chroma != 0.0 {
            flash = skin_turn_hue(flash, input.chroma);
        }
        return flash * persona_fade(input);
    }
    let n = STAR - input.hilt;
    let scale = glint_scale(n);
    var glint: vec3<f32>;
    if skin.hole_shape.x > 0.0 {
        glint = hole_ring(skin, input.blade / scale, input.animation.x,
            input.animation.y + n * 0.37, pixel / scale) * skin.hole_color.rgb
            * skin.hole_color.w;
    } else if skin.star_color.w > 0.0 {
        glint = skin_star(skin, input.blade / scale, input.animation.x,
            input.animation.y + n * 0.37, pixel / scale) * skin.star_color.rgb
            * skin.star_color.w;
    } else {
        return vec3(0.0);
    }
    glint = skin_apply_tint(glint, skin_tint(skin, input));
    if input.chroma != 0.0 {
        glint = skin_turn_hue(glint, input.chroma);
    }
    return glint * persona_fade(input) * (1.0 - (1.0 - GLINT_BEHIND) * input.shaft);
}

// Dynamic glow: the glow capsule only; the core line's shader has no `glow` stage.
@fragment
fn fragment_glow(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow).
    let pixel = fwidth(input.blade.x);
    if input.kind == KIND_BLACK { discard; }
    if input.hilt <= STAR { return vec4(star_glow(input, pixel), 1.0); }
    if input.hilt <= 0.0 { discard; }
    if lab_design(input) != 0u {
        // Only a little of a design reaches the bloom, its white core least.
        return vec4(min(lab_glow(input, pixel), vec3(0.6)) * LAB_BLOOM, 1.0);
    }
    if input.kind >= KIND_SKIN {
        // A signature blooms less (Creyon's round 20: its bloom hid its effects close up).
        if skins[skin_index(input)].design.x == f32(LAB_SIGNATURE) {
            return vec4(min(skin_glow(input, pixel), vec3(1.2)) * SIG_BLOOM, 1.0);
        }
        return vec4(skin_glow(input, pixel), 1.0);
    }
    return vec4(glow_capsule(input) * input.color, 1.0);
}

// How much of a signature reaches the dynamic glow (the bloom).
const SIG_BLOOM: f32 = 0.55;

// A glow fragment's colour and its depth (`sig_depth`).
struct GlowOut {
    @location(0) colour: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}

// How deep a glow fragment is tested (1/11/2026, Creyon: straight lines cut through a
// signature's effects where its flat quad passed into a wall, the floor or its wearer's body,
// which hid effects standing in front of them): a signature's three-dimensional effects stand
// up to its reach in front of the blade, so its glow is tested as if that much nearer the
// camera. Near the hilt it is not, so the hand holding it still covers it; nor nearer than a
// few units to the camera. Every other blade keeps its quad's depth.
fn sig_depth(input: VertexOutput) -> f32 {
    let flat_depth = input.clip_position.z;
    if input.hilt <= 0.0 || input.kind < KIND_SKIN {
        return flat_depth;
    }
    let skin = skins[skin_index(input)];
    if skin.design.x != f32(LAB_SIGNATURE) {
        return flat_depth;
    }
    let room = max(skin_room(skin, input.off.x), max(skin.design.y, skin.design.z) / LAB_METRE);
    let extent = max(chain_radius(input.radius, input.length, 0.0) * skin.swell.x, input.hilt) + room;
    let along = blade_place(input).y;
    let dist = length(input.ray);
    let pull = min(0.6 * extent * smoothstep(3.0, 12.0, along), max(dist - 8.0, 0.0));
    if pull <= 0.0 || dist <= 0.0001 {
        return flat_depth;
    }
    let clip = camera.view_projection * vec4(camera.position + input.ray * ((dist - pull) / dist), 1.0);
    if clip.w <= 0.0001 {
        return flat_depth;
    }
    return clamp(clip.z / clip.w, 0.0, 1.0);
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return main_colour(input);
}

// True in the main pass of a view, whose effects in the air `fragment_volume` draws instead.
override SIG_SPLIT: bool = false;

// What a signature's effects in the air give: light added with the glow, and its own effects'.
struct SigAir {
    shared_light: vec3<f32>,
    own: vec3<f32>,
}

fn sig_air(input: VertexOutput, skin: Skin, place: vec2<f32>, pixel: f32) -> SigAir {
    var air: SigAir;
    let ray = sig_ray(input);
    labRayPrepare(ray.o, ray.d);
    let shown = sig_full(input);
    let effects = (max(sig_vol(ray.o, ray.d, pixel), vec3(0.0))
        + max(sig_orbit(ray.o, ray.d, pixel), vec3(0.0))) * shown;
    if SIG_OWN_EFFECTS {
        air.own = effects;
    } else {
        air.shared_light = effects;
    }
    if skin.arc_strike.x > 0.0 {
        air.shared_light += sig_arcs(skin, input, ray, pixel) * shown;
    }
    if SIG_MYTHICAL {
        air.shared_light += sig_mythic(input, place, ray, pixel) * shown;
    }
    return air;
}

// The view's depth, for the half-size layer of effects (`saber_volume.rs`).
@group(3) @binding(2)
var volume_depth: texture_depth_2d;

// How near the view ray comes to the blade's axis, from the hilt (y 0) to `top`, in the hilt's frame.
fn sig_ray_gap(ray: SigRay, top: f32) -> f32 {
    let w = ray.o;
    let d = ray.d;
    let b = d.y;
    let e = w.y;
    let c = dot(d, w);
    let denom = max(1.0 - b * b, 0.000001);
    var along = clamp((e - b * c) / denom, 0.0, top);
    let t = max(along * b - c, 0.0);
    along = clamp(w.y + d.y * t, 0.0, top);
    return length(w + d * t - vec3(0.0, along, 0.0));
}

// A signature's effects in the air alone, into the half-size layer, as the main pass would add
// them: hidden by what stands before them, and where its quad ends.
@fragment
fn fragment_volume(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow).
    // The layer is half size, but the picture it ends in is not: sized by the full picture's pixel.
    let pixel = fwidth(input.blade.x) * 0.5;
    if input.hilt <= 0.0 || input.kind < KIND_SKIN || (input.persona.x >> 24u) != 255u
        || afterimage(input.persona.x, input.off.x) {
        discard;
    }
    let skin = skin_at(input);
    if skin.design.x != f32(LAB_SIGNATURE) { discard; }
    let last = vec2<i32>(textureDimensions(volume_depth)) - vec2(1);
    let at = clamp(vec2<i32>(input.clip_position.xy * 2.0), vec2(0), last);
    // Fades against what stands before it, in linear units, so the quad's plane meeting a wall draws no line.
    let scene_z = textureLoad(volume_depth, at, 0);
    let near_units = 4.0 / max(1.0 - sig_depth(input), 0.00001);
    let wall_units = 4.0 / max(1.0 - scene_z, 0.00001);
    let soft = smoothstep(-26.0, 6.0, wall_units - near_units);
    if soft <= 0.0 { discard; }
    sig_take(input);
    let place = blade_place(input);
    let air = sig_air(input, skin, place, pixel);
    let shown = sig_full(input);
    // Past the room it fades by how near the ray itself comes to the blade, the same from every
    // side: anything measured on the panel met the effects as a straight line (Creyon's hard lines).
    let ray = sig_ray(input);
    let room = max(skin_room(skin, input.off.x), max(skin.design.y, skin.design.z) / LAB_METRE);
    let edge = 1.0 - smoothstep(room + 0.5, room + 3.0, sig_ray_gap(ray, max(input.length, input.axis.w)));
    let out = sig_coloured(input, (air.shared_light + max(uTurnFx, vec3(0.0)) * shown) * edge)
        + (max(uKeep, vec3(0.0)) * shown + sig_coloured_air(input, air.own)) * edge;
    // From third person and further the effects would shrink to nothing: they gain with distance.
    let far_off = smoothstep(55.0, 240.0, length(input.ray));
    return vec4(out * persona_fade(input) * soft * (1.0 + 0.9 * far_off), clamp(uAirCover, 0.0, 1.0) * shown * edge * soft);
}

// A signature's own glow pipeline (`saber_gpu.rs` `SignatureBuilder::build`): its colour, and
// its depth pulled toward the camera (`sig_depth`).
@fragment
fn fragment_main_deep(input: VertexOutput) -> GlowOut {
    var out: GlowOut;
    out.colour = main_colour(input);
    out.depth = sig_depth(input);
    return out;
}

fn main_colour(input: VertexOutput) -> vec4<f32> {
    // v runs from the tip (0) to behind the hilt (1), as DoLine's texture coordinates do.
    // Derivatives are taken before any branch (uniform control flow); the glow ignores the
    // core's and a skin's glow takes the pixel's size. The core is sampled with the
    // derivatives of its coordinates before the rounded tip narrows them (the shaft's, where
    // the two are the same): in the tip the narrowing runs to a hundred times within a pixel,
    // and those derivatives picked the texture's smallest mip, a flat bright average of the
    // whole line, which drew a bright rim of pixels round the tip's end (players' reports
    // after the 2026.1021.1 release, every signature blade).
    // One skin lookup for both: the filter's coordinates are the line's without the tip.
    let line = core_line(input);
    let core_uv = core_coordinates_at(input, line, line.stretch);
    let filter_uv = core_coordinates_at(input, line, line.breath);
    let core_dx = dpdx(filter_uv);
    let core_dy = dpdy(filter_uv);
    let pixel = fwidth(input.blade.x);
    if input.kind == KIND_BLACK { discard; }
    if input.hilt <= STAR {
        return vec4(star_glow(input, pixel), 1.0);
    }
    // A Blade Lab design draws its core in its glow (the lab's blade is one shader).
    let design = lab_design(input);
    if input.hilt > 0.0 {
        if design != 0u { return vec4(lab_glow(input, pixel), 1.0); }
        if input.kind >= KIND_SKIN { return vec4(skin_glow(input, pixel), 1.0); }
        return vec4(glow_capsule(input) * input.color, 1.0);
    }
    // How far the core's `u` moves in a pixel, from the straight line's across scaled by the
    // stretch here: the derivatives of `core_uv` itself blow up on the tip's last row, where
    // the stretch runs to a hundred within one pixel, and the rounded tip's cut then left
    // half the line's width lit there, a bar across the tip.
    if design != 0u { discard; }
    let footprint = pixel / (2.0 * input.radius) * line.stretch;
    var texel = textureSampleGrad(core_texture, core_sampler, core_uv, core_dx, core_dy);
    let split = line.split;
    if split > 0.0 {
        // A glitch parts the core's colours: red one way, blue the other.
        let offset = vec2(split, 0.0);
        let red = textureSampleGrad(core_texture, core_sampler, core_uv + offset, core_dx, core_dy);
        let blue = textureSampleGrad(core_texture, core_sampler, core_uv - offset, core_dx, core_dy);
        let skin = skin_core(input, texel, core_uv, footprint);
        return vec4(
            skin_core(input, red, core_uv + offset, footprint).r,
            skin.g,
            skin_core(input, blue, core_uv - offset, footprint).b,
            1.0,
        );
    }
    if input.kind >= KIND_SKIN {
        return vec4(skin_core(input, texel, core_uv, footprint), 1.0);
    }
    if input.kind == KIND_NEUTRAL {
        // Neutral core: red = white-hot core, green = tinted fringe.
        return vec4(CORE_DRAWS * (vec3(texel.r) + input.color * texel.g), 1.0);
    }
    return vec4(CORE_DRAWS * texel.rgb * input.color, 1.0);
}

// JA+ black blades use alpha blending, not the additive retail/RGB pass.
// How many of stock's glow sprites cover a point of a blade (`fragment_black`).
// A little under stock's 2 / 0.65 (Dave's test, 28/10/2026: the full overlap read as
// too much black glow in game).
const BLACK_GLOW_OVERLAP: f32 = 2.2;

@fragment
fn fragment_black(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = core_coordinates(input);
    let dx = dpdx(uv);
    let dy = dpdy(uv);
    if input.hilt <= 0.0 {
        return textureSampleGrad(core_texture, core_sampler, uv, dx, dy);
    }
    // Smooth the sprite chain into a capsule, with the installed alpha profile.
    let along = input.blade.y / max(input.shaft, 0.0001) * input.length;
    let beyond = along - clamp(along, 0.0, input.length);
    let radius = chain_radius(input.radius, input.length, clamp(along / max(input.length, 0.0001), 0.0, 1.0));
    let glow_uv = vec2(0.5 + input.blade.x / (2.0 * radius), 0.5 + beyond / (2.0 * radius));
    if any(glow_uv < vec2(0.0)) || any(glow_uv > vec2(1.0)) { return vec4(0.0); }
    let glow = textureSampleLevel(glow_texture, saber_sampler, glow_uv, 0.0);
    // Stock's sprite chain (`RB_SurfaceSaberGlow`, rd-vanilla `tr_surface.cpp`) lays a
    // sprite of the blade's radius every 0.65 radii, so every point of the blade is under
    // about 2 / 0.65 of them, each blending its alpha over the last: the black blade's
    // halo is that much darker (thicker) than one sprite (Dave's test, 27/10/2026).
    return vec4(glow.rgb, 1.0 - pow(1.0 - glow.a, BLACK_GLOW_OVERLAP));
}

// --- Heat haze: drawn first into the effect layer, replacing its pixels ---------------------

// The scene as the effect layer holds it before any effect (effect_layer.rs `original`).
@group(3) @binding(0)
var haze_scene: texture_2d<f32>;
@group(3) @binding(1)
var haze_sampler: sampler;

// The scene behind a skin's glow bent by its heat: a rising, curling shift (at most
// `strength` world units, turned into pixels with the quad's own derivatives), strongest
// by the blade and above it, nothing at the reach's edge, so the quad's edge is the scene
// itself, unchanged.
@fragment
fn fragment_haze(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow): units across and along a pixel.
    let dx = dpdx(input.blade);
    let dy = dpdy(input.blade);
    let size = vec2<f32>(textureDimensions(haze_scene));
    let here = input.clip_position.xy;
    let skin = skin_at(input);
    if input.hilt <= FLASH + 0.5 {
        // A lighting blade's heat burst: a ring of bent air spreading from the emitter,
        // pushing the scene outward, weaker as it goes; nothing drawn off the ring.
        let b = skin.flash_burst;
        let left = input.color.b;
        let d = length(input.blade);
        let radius = b.y * (1.0 - left * left);
        let e = (d - radius) / max(b.z, 0.01);
        let ring = exp(-e * e) * left;
        if ring < 0.01 {
            discard;
        }
        let push = input.blade / max(d, 0.001) * b.x * ring;
        let moved = to_pixels(push, dx, dy, 0.02 * max(size.x, size.y));
        return vec4(textureSampleLevel(haze_scene, haze_sampler, (here + moved) / size, 0.0).rgb,
            1.0);
    }
    let h = skin.haze;
    let up = skin_up(input.down.xy);
    let at = skin_above(input.blade, input.shaft, up);
    let distance = length(at);
    let r = chain_radius(input.radius, input.length,
        clamp(input.blade.y / max(input.shaft, 0.0001), 0.0, 1.0)) * skin.swell.x;
    // Full by the glow, gone at its edge plus the reach; more above the blade (heat rises).
    let fall = 1.0 - smoothstep(0.3 * r, r + h.w, distance);
    let rising = mix(0.55, 1.0, smoothstep(-r, r, at.x));
    let time = input.animation.x;
    let seed = input.animation.y;
    let a = skin_rising(input.blade, up, h.y, h.z, 0.6, time, seed);
    let b = skin_rising(input.blade + vec2(17.3, -9.1), up, h.y, h.z, 0.6, time, seed);
    // Calming up the blade with its unrest, as far as the skin says.
    let calm = mix(1.0, skin_unrest(skin, skin_along(input)), skin.unrest_lean.z);
    let shift = (vec2(a, b) - 0.5) * 2.0 * h.x * fall * rising * calm;
    // Never more than a few dozen pixels, however close.
    let pixels = to_pixels(shift, dx, dy, 0.02 * max(size.x, size.y));
    let uv = (here + pixels) / size;
    return vec4(textureSampleLevel(haze_scene, haze_sampler, uv, 0.0).rgb, 1.0);
}

// `shift` in the quad's units as pixels (the inverse of its Jacobian `dx`, `dy`), at
// most `most` long.
fn to_pixels(shift: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, most: f32) -> vec2<f32> {
    let det = dx.x * dy.y - dx.y * dy.x;
    var pixels = vec2(0.0);
    if abs(det) > 1e-8 {
        pixels = vec2(shift.x * dy.y - shift.y * dy.x, -shift.x * dx.y + shift.y * dx.x) / det;
    }
    let len = length(pixels);
    if len > most {
        pixels *= most / len;
    }
    return pixels;
}

// The black holes' quads (the lens and the shade): a skin's glint instances when it has
// holes its wearer left on; every other instance is folded away.
@vertex
fn vertex_hole(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    var hole = output.kind >= KIND_SKIN && output.hilt <= STAR && output.hilt > FLASH + 0.5
        && (blade.off.x & OFF_HOLES) == 0u;
    if hole {
        hole = skins[skin_index(output)].hole_shape.x > 0.0;
    }
    if !hole {
        output.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
    }
    return output;
}

// A black hole's lens, drawn with the haze before any glow: the scene round it bent as
// a point mass bends light, what is behind it drawn into a ring (`lens` horizon radii
// past the horizon) and squeezed toward it, fading to the scene unbent at the quad's
// edge (its alpha), and weaker while it is behind the blade.
@fragment
fn fragment_lens(input: VertexOutput) -> @location(0) vec4<f32> {
    // Taken before any branch (uniform control flow).
    let dx = dpdx(input.blade);
    let dy = dpdy(input.blade);
    let size = vec2<f32>(textureDimensions(haze_scene));
    let skin = skin_at(input);
    let scale = glint_scale(STAR - input.hilt);
    let horizon = skin.hole_shape.x * scale;
    let reach = skin.hole_shape.w * scale;
    let ring = horizon * (1.0 + skin.hole_shape.y);
    let p = input.blade;
    let r = length(p);
    let fade = 1.0 - smoothstep(0.6 * reach, reach, r);
    let shift = -p / max(r, 0.0001) * (ring * ring / max(r, horizon)) * fade;
    let pixels = to_pixels(shift, dx, dy, 0.06 * max(size.x, size.y));
    let uv = (input.clip_position.xy + pixels) / size;
    let seen = textureSampleLevel(haze_scene, haze_sampler, uv, 0.0).rgb;
    return vec4(seen, fade * (1.0 - 0.6 * input.shaft));
}

// --- The seventh set (blade_skin_solids.rs): solid blades, blended after the glow -------

// The solid pass's quads: a skin's glow quad when it has ink or chrome on, for the blade
// itself (not an afterimage); every other instance is folded away.
@vertex
fn vertex_solid(@builtin(vertex_index) vertex_index: u32, blade: BladeInstance) -> VertexOutput {
    var output = blade_vertex(vertex_index, blade);
    var solid = output.kind >= KIND_SKIN && output.hilt > 0.0 && (blade.persona.x >> 24u) == 255u;
    if solid {
        let skin = skins[skin_index(output)];
        let ink = (blade.off.x & OFF_INK) == 0u
            && (skin.ink_color.w > 0.0 || skin.ink_shape.x > 0.0);
        let chrome = (blade.off.x & OFF_CHROME) == 0u && skin.chrome_color.w > 0.0;
        solid = ink || chrome || skin.design.w > 0.0;
    }
    if !solid {
        output.clip_position = vec4(0.0, 0.0, 0.0, 1.0);
    }
    return output;
}

// A liquid mirror's colour and coverage at `place` units along (unclamped) and `x` across:
// rippling, beads rolling up it, a round end; it reflects a made-up surrounding (a dark
// ground, a bright sky, a key highlight, a lit rim) in the light where the blade is.
fn solid_chrome(skin: Skin, input: VertexOutput, place: f32, x: f32, pixel: f32) -> vec4<f32> {
    let c = skin.chrome_color;
    let s = skin.chrome_shape;
    let time = input.animation.x;
    let seed = input.animation.y;
    let seeded = i32(seed * 4096.0);
    var radius = c.w * input.radius
        * (1.0 + s.x * sin(place * 1.2 - time * s.z * 0.25 + seed * 6.0));
    let span = input.length + 6.0;
    var swell = 0.0;
    for (var b = 0; b < min(i32(s.y), 8); b++) {
        let pace = s.z * (0.6 + 0.8 * skin_hash(vec2(b, 301 + seeded)));
        let at = fract(skin_hash(vec2(b, 307 + seeded)) + time * pace / span) * span - 3.0;
        swell = max(swell, exp(-pow((place - at) / 1.5, 2.0)));
    }
    radius *= 1.0 + 0.6 * swell;
    let beyond = max(place - input.length, 0.0);
    let reach = sqrt(max(radius * radius - beyond * beyond, 0.0));
    let inside = clamp((reach - abs(x)) / max(pixel, 0.0001) + 0.5, 0.0, 1.0)
        * clamp(place / max(pixel, 0.0001) + 0.5, 0.0, 1.0);
    let n = clamp(x / max(reach, 0.0001), -1.0, 1.0);
    let warp = n + 0.25 * sin(place * 0.7 - time * 1.5 + n * 2.0);
    var env = mix(vec3(0.05, 0.055, 0.07), vec3(0.8, 0.85, 0.95), smoothstep(-0.8, 0.9, warp));
    env += vec3(1.0) * exp(-pow((warp + 0.45) / 0.1, 2.0)) * 1.3;
    env += vec3(0.6, 0.7, 0.9) * pow(abs(n), 6.0) * 0.6;
    env *= c.rgb * (0.65 + 0.5 * persona_light(input));
    return vec4(env, inside);
}

// The solid pass: a skin's chrome, then its ink (a body, a glossy streak and an outline)
// over the blade's glow just drawn, alpha-blended; a chroma's ink body takes its colour.
@fragment
fn fragment_solid(input: VertexOutput) -> @location(0) vec4<f32> {
    return solid_colour(input);
}

// A signature's solid layer that stands wide of its blade (`@solidnear`): its depth is pulled well toward
// the camera, so its plane does not slice through the wearer or a wall beside the blade.
@fragment
fn fragment_solid_near(input: VertexOutput) -> GlowOut {
    var out: GlowOut;
    out.colour = solid_colour(input);
    out.depth = input.clip_position.z;
    let dist = length(input.ray);
    let pull = min(SIG_NEAR_PULL, max(dist - 8.0, 0.0));
    if pull > 0.0 {
        let clip = camera.view_projection * vec4(camera.position + input.ray * ((dist - pull) / dist), 1.0);
        if clip.w > 0.0001 {
            out.depth = clamp(clip.z / clip.w, 0.0, 1.0);
        }
    }
    return out;
}

fn solid_colour(input: VertexOutput) -> vec4<f32> {
    let pixel = fwidth(input.blade.x);
    if lab_design(input) != 0u {
        return lab_solid(input, pixel);
    }
    let skin = skin_at(input);
    let time = input.animation.x;
    let place = input.blade.y / max(input.shaft, 0.0001) * input.length;
    let x = input.blade.x;
    var out = vec4(0.0);
    if skin.chrome_color.w > 0.0 {
        out = solid_chrome(skin, input, place, x, pixel);
    }
    let ink = skin.ink_color;
    let shape = skin.ink_shape;
    if ink.w > 0.0 || shape.x > 0.0 {
        var radius = shape.y * input.radius;
        let beyond = place - input.length;
        // Signed distance to the body's edge (negative inside): a capsule, or drawn to a
        // point over its last `point` units; flat at the hilt.
        var d: f32;
        if shape.w > 0.0 {
            radius *= clamp((input.length - place) / shape.w, 0.0, 1.0);
            d = abs(x) - radius;
        } else {
            d = select(abs(x) - radius, length(vec2(x, beyond)) - radius, beyond > 0.0);
        }
        d = max(d, -place);
        let soft = max(pixel, 0.0001);
        let body = ink.w * clamp(-d / soft + 0.5, 0.0, 1.0);
        let ring = shape.x * clamp((0.5 * shape.z * input.radius - abs(d)) / soft + 0.5, 0.0, 1.0);
        let gloss = skin.ink_more.x * exp(-pow((x / max(radius, 0.0001) - 0.35) / 0.12, 2.0))
            * (0.5 + 0.5 * sin(place * 0.25 - time * 0.8));
        var colour = ink.rgb;
        if input.chroma != 0.0 {
            colour = skin_turn_hue(colour, input.chroma);
        }
        var rgb = mix(out.rgb, colour + vec3(gloss), body);
        var alpha = max(out.a, body);
        rgb = mix(rgb, vec3(0.0), ring);
        alpha = max(alpha, ring);
        out = vec4(rgb, alpha);
    }
    // A signature shader's solid layer (dark stone, scales, a horizon).
    if skin.design.x == f32(LAB_SIGNATURE) {
        sig_take(input);
        if SIG_OWN_EFFECTS {
            let view = sig_ray(input);
            labRayPrepare(view.o, view.d);
        }
        let place = sig_place(input);
        out = clamp(sig_solid(out, place, pixel), vec4(0.0), vec4(1.0));
        // A new blade's body keeps its pattern from third person and further: its contrast and colour gain with distance.
        if SIG_OWN_EFFECTS {
            let far_off = smoothstep(55.0, 240.0, length(input.ray));
            let level = dot(out.rgb, vec3(0.3, 0.5, 0.2));
            out = vec4(clamp(vec3(level) + (out.rgb - vec3(level)) * (1.0 + 0.5 * far_off), vec3(0.0), vec3(1.0)), out.a);
        }
        // Only on the blade there is (Creyon's round 13: World Shift's lava crust stayed at
        // full length while the blade went out): a signature places its solid parts on the
        // full length, so what lies past the tip the blade has now is cut.
        // Once the blade is all out the cut moves past its rounded tip (Creyon, 27/10/2026: the
        // Moon's black tip was half see-through, the fade starting a unit below the tip).
        let out_full = select(0.0, 1.0, input.axis.w > 0.0 && input.length >= input.axis.w);
        let grown = 1.0 - smoothstep(input.length - 1.0 + 1.2 * out_full,
            input.length + 0.5 + 0.4 * out_full, place.y);
        let kept_past = select(grown, 1.0, SIG_SOLID_PAST_TIP && input.length >= input.axis.w);
        out = vec4(out.rgb, out.a * sig_kept(input, place) * kept_past);
        if (input.persona.x >> 24u) != 255u {
            // An echo keeps its blade's solid parts only, faded (round 12).
            return vec4(sig_coloured(input, out.rgb), out.a * sig_edge(input) * persona_fade(input));
        }
        let ray = sig_ray(input);
        labRayPrepare(ray.o, ray.d);
        let shown = sig_full(input);
        let near = clamp(sig_vol_solid(ray.o, ray.d, pixel), vec4(0.0), vec4(1.0));
        let round = clamp(sig_orbit_solid(ray.o, ray.d, pixel), vec4(0.0), vec4(1.0));
        let v = vec4(mix(near.rgb, round.rgb, round.a), max(near.a, round.a)) * shown;
        out = vec4(mix(out.rgb, v.rgb, v.a), max(out.a, v.a));
        out = vec4(mix(sig_coloured(input, out.rgb), out.rgb, clamp(uOwn, 0.0, 1.0)),
            out.a * sig_edge(input));
    }
    return out;
}

// A black hole's horizon, drawn after the glow: black, over the blade's glow when the
// hole is in front of it; behind the blade (and close to its axis) the glow shows.
@fragment
fn fragment_shade(input: VertexOutput) -> @location(0) vec4<f32> {
    let pixel = fwidth(input.blade.x);
    let skin = skin_at(input);
    let horizon = skin.hole_shape.x * glint_scale(STAR - input.hilt);
    let disc = clamp((horizon - length(input.blade)) / max(pixel, 0.0001) + 0.5, 0.0, 1.0);
    return vec4(0.0, 0.0, 0.0, disc * (1.0 - input.shaft));
}

// --- Creyon's Blade Lab designs (blade_lab_designs.rs) ---------------------------------
//
// A skin `blade_lab_designs.rs` names is drawn as its design in Creyon's Blade Lab was:
// the lab's own blade shader in the lab's units (metres: a stock 40-unit blade is the
// lab's 1.08 m one), its particles in the glow quad (additive) or the solid pass
// (blended), and its solid blade in the solid pass. Its `design` lane: the design's
// number (0: none, the skin's file draws it), the glow's reach and the particles' room
// (metres), and whether it has a solid pass. The swing, clashes and ignition come with
// the instance (`motion`, saber_motion.rs).

// Metres a unit (1.08 / 40).
const LAB_METRE: f32 = 0.027;
const LAB_PEPPERMINT: u32 = 1u;
const LAB_BUBBLE: u32 = 2u;
const LAB_LAVALAMP: u32 = 3u;
const LAB_SKETCH: u32 = 4u;
const LAB_HOURGLASS: u32 = 5u;
const LAB_COMIC: u32 = 6u;
const LAB_QUICKSILVER: u32 = 7u;
const LAB_REACTOR: u32 = 8u;
const LAB_OBSIDIAN: u32 = 9u;
const LAB_BLEEDING: u32 = 10u;
const LAB_MONARCH: u32 = 11u;
const LAB_PHOENIX: u32 = 12u;
const LAB_SERPENT: u32 = 13u;
const LAB_QUANTUM: u32 = 14u;
// A skin drawn by a Blade Lab signature shader (blade_signatures.rs): its `design` lane's
// number. The skin's own passes draw it, each handing what it drew to the signature.
const LAB_SIGNATURE: u32 = 100u;

// Where a fragment is, in the lab's terms. `x` along the blade (metres from the hilt, as
// on the blade itself), `y` across, `big_l` the blade's length, `d` the distance to the
// blade's axis on screen; `p` the place on screen (along, across; metres) and `shaft`
// the blade's length there, `k` how much of its length shows; `up` the world's up on
// screen (along, across); `px` a pixel (metres).
struct LabPlace {
    x: f32,
    y: f32,
    big_l: f32,
    d: f32,
    t: f32,
    px: f32,
    p: vec2<f32>,
    shaft: f32,
    k: f32,
    up: vec2<f32>,
    swing: f32,
    clash: f32,
    age: f32,
    since_swing: f32,
    since_clash: f32,
    // A chroma's turn to its wearer's colour (0: the design's own colours).
    turn: f32,
    // How much narrower than the lab's it is drawn (`LAB_THIN`).
    thin: f32,
    // A blade gone in whose words float on (`saber_motion::BladeMotion::lingering`: its
    // swing lane is -1): only the words are drawn.
    lingering: bool,
    // The wearer's switched-off parts (`input.off.x`): the Clone Wars blade's own (round 19).
    off: u32,
}

// How far each design's blade starts behind its hilt end (metres): stock blades hide
// the hilt's mouth under their glow and hilt sprite, the lab's start at its emitter.
const LAB_START: f32 = 0.027;
// How much narrower each design is drawn than in the lab (its width divided by this),
// so every blade is a stock blade's thickness and fits its hilt; by number.
const LAB_THIN: array<f32, 15> = array<f32, 15>(
    1.0, 1.35, 1.25, 1.5, 1.0, 1.0, 1.35, 1.1, 1.35, 1.15, 1.1, 1.0, 1.5, 1.4, 1.0,
);
// How much of a design's light reaches the dynamic glow (the bloom): its white core
// bloomed into a blob that hid its effects from afar.
const LAB_BLOOM: f32 = 0.3;

fn lab_design(input: VertexOutput) -> u32 {
    if input.kind < KIND_SKIN {
        return 0u;
    }
    let n = u32(skins[skin_index(input)].design.x + 0.5);
    // A signature shader is drawn by the skin's own passes, with its hooks.
    if n == LAB_SIGNATURE {
        return 0u;
    }
    return n;
}

fn lab_place(input: VertexOutput, pixel: f32) -> LabPlace {
    var p: LabPlace;
    // The true place on the blade (`blade_place`): no foreshortening left to undo.
    let place = blade_place(input);
    p.k = 1.0;
    p.p = vec2(place.y * LAB_METRE + LAB_START, place.x * LAB_METRE);
    p.shaft = input.length * LAB_METRE + LAB_START;
    p.x = p.p.x / p.k;
    let n = min(u32(skins[skin_index(input)].design.x + 0.5), 14u);
    p.thin = LAB_THIN[n];
    p.y = p.p.y * p.thin;
    p.big_l = input.length * LAB_METRE + LAB_START;
    p.d = lab_seg(p.p.x, p.y, p.shaft);
    p.t = input.animation.x;
    p.px = max(pixel * LAB_METRE, 0.00001);
    let up = skin_up(input.down.xy);
    p.up = vec2(up.y, up.x);
    p.swing = clamp(input.motion.x, 0.0, 1.0);
    p.lingering = input.motion.x < -0.5;
    // The lab's clash fades at 2.2 a second.
    p.clash = max(1.0 - input.motion.y * 2.2, 0.0);
    // Its `bursts` option off (the motes bit): swings and clashes set nothing off.
    if (input.off.x & OFF_MOTES) != 0u {
        p.swing = 0.0;
        p.clash = 0.0;
    }
    p.age = input.motion.z;
    p.since_swing = input.motion.w;
    p.since_clash = input.motion.y;
    p.turn = input.chroma;
    p.off = input.off.x;
    return p;
}

// `c` turned to the wearer's colour, for a chroma's parts that take it.
fn lab_turn(c: vec3<f32>, p: LabPlace) -> vec3<f32> {
    if p.turn == 0.0 {
        return c;
    }
    return skin_turn_hue(c, p.turn);
}

// `p` as a copy of the blade turned `angle` (radians, on screen) about its hilt sees it.
fn lab_turned(p0: LabPlace, angle: f32) -> LabPlace {
    var p = p0;
    let c = cos(angle);
    let s = sin(angle);
    p.p = vec2(p0.p.x * c - p0.p.y * s, p0.p.x * s + p0.p.y * c);
    p.x = p.p.x / p.k;
    p.y = p.p.y * p.thin;
    p.d = lab_seg(p.p.x, p.y, p.shaft);
    return p;
}

fn lab_sstep(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn lab_hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453);
}

fn lab_hash2(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

fn lab_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    var f = fract(p);
    f = f * f * (3.0 - 2.0 * f);
    return mix(mix(lab_hash2(i), lab_hash2(i + vec2(1.0, 0.0)), f.x),
        mix(lab_hash2(i + vec2(0.0, 1.0)), lab_hash2(i + vec2(1.0)), f.x), f.y);
}

fn lab_fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var s = 0.0;
    var a = 0.5;
    for (var i = 0; i < 4; i++) {
        s += a * lab_noise(p);
        p *= 2.03;
        a *= 0.5;
    }
    return s;
}

fn lab_hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
    let m = h * 6.0 + vec3(0.0, 4.0, 2.0);
    let k = clamp(abs(m - 6.0 * floor(m / 6.0) - 3.0) - 1.0, vec3(0.0), vec3(1.0));
    return v * mix(vec3(1.0), k, s);
}

fn lab_seg(x: f32, y: f32, l: f32) -> f32 {
    return length(vec2(max(0.0, max(-x, x - l)), y));
}

// The lab's plain blade: a white-hot core and a glow round it.
fn lab_saber(g: vec3<f32>, c: vec3<f32>, d: f32, cr: f32, k: f32) -> vec3<f32> {
    let core = lab_sstep(cr, cr * 0.35, d);
    let glow = exp(-d * k) * 0.55 + exp(-d * k * 0.3) * 0.16;
    return c * core + g * glow * (1.0 - core * 0.6);
}

fn lab_peppermint(p: LabPlace) -> vec3<f32> {
    let r = 0.03;
    let body = lab_sstep(r, r * 0.82, p.d);
    let th = asin(clamp(p.y / r, -1.0, 1.0));
    let sh = cos(th);
    let s = fract(p.x * 5.5 + th / PI * 0.5 - p.t * 0.5);
    let stripe = lab_sstep(0.44, 0.5, s) - lab_sstep(0.94, 1.0, s);
    let col = mix(vec3(1.0, 0.96, 0.96), lab_turn(vec3(1.0, 0.05, 0.11), p), stripe);
    let spec = pow(max(0.0, 1.0 - abs(th + 0.55) / 0.3), 3.0) * 0.55;
    let glit = step(0.988, lab_hash2(floor(vec2(p.x * 60.0, th * 8.0)) + floor(p.t * 5.0))) * 1.4;
    let b = col * (0.4 + 0.6 * sh) * body + (spec + glit) * body;
    let halo = lab_turn(vec3(1.0, 0.32, 0.36), p) * (exp(-p.d * 20.0) * 0.42 + exp(-p.d * 6.0) * 0.11);
    return b + halo * (1.0 - body * 0.5);
}

fn lab_bubble(p: LabPlace) -> vec3<f32> {
    let r = 0.028;
    let body = lab_sstep(r, r * 0.9, p.d);
    let th = asin(clamp(p.y / r, -1.0, 1.0));
    let film = lab_fbm(vec2(p.x * 4.0 + p.t * 0.2, th * 1.6 - p.t * 0.35)) + 0.3 * sin(p.x * 9.0 - p.t * 0.8);
    let col = lab_hsv(film * 1.8 + p.t * 0.03 + p.turn, 0.6, 1.0);
    let fres = pow(abs(p.y) / r, 4.0);
    let line = exp(-abs(p.y) * 900.0) * 0.75 * step(0.0, p.x) * step(p.x, p.big_l);
    let hl = pow(max(0.0, 1.0 - abs(th - 0.7) / 0.2), 4.0) * body * 0.5;
    let halo = lab_hsv(film * 1.8 + 0.5 + p.t * 0.03 + p.turn, 0.4, 1.0) * (exp(-p.d * 22.0) * 0.28 + exp(-p.d * 7.0) * 0.08);
    return col * (0.12 + fres * 0.95) * body + vec3(line + hl) + halo;
}

fn lab_lavalamp(p: LabPlace) -> vec3<f32> {
    let r = 0.034;
    let in_glass = lab_sstep(r, r * 0.9, p.d) * step(0.0, p.x);
    var f = 0.0;
    for (var i = 0; i < 7; i++) {
        let fi = f32(i);
        let sp = 0.05 + 0.04 * lab_hash(fi * 3.1);
        let xi = fract(lab_hash(fi * 1.7) + p.t * sp) * (p.big_l + 0.12) - 0.06;
        let yi = sin(p.t * 0.6 + fi * 2.0) * 0.008;
        let rr = 0.017 + 0.009 * lab_hash(fi * 5.3);
        let q = vec2((p.x - xi) * 0.75, p.y - yi);
        f += rr * rr / (dot(q, q) + 1e-5);
    }
    let pool = lab_sstep(0.07, 0.0, p.x) + lab_sstep(p.big_l - 0.06, p.big_l, p.x);
    let m = lab_sstep(0.9, 1.25, f + pool * 1.3);
    let wax = mix(vec3(1.0, 0.32, 0.14), vec3(1.0, 0.78, 0.4), lab_sstep(1.3, 3.5, f + pool));
    let inside = mix(vec3(0.3, 0.05, 0.6) * 0.45, wax * 1.2, m) * in_glass;
    let rim = exp(-abs(abs(p.y) - r) * 700.0) * step(0.0, p.x) * step(p.x, p.big_l) * 0.3;
    let halo = mix(vec3<f32>(0.62, 0.15, 0.95), vec3<f32>(1.0, 0.4, 0.2), 0.35) * (exp(-p.d * 24.0) * 0.3 + exp(-p.d * 6.0) * 0.08);
    return lab_turn(inside + vec3(0.9, 0.8, 1.0) * rim + halo, p);
}

fn lab_sketch(p: LabPlace) -> vec3<f32> {
    let ts = floor(p.t * 8.0);
    let w1 = (lab_noise(vec2(p.x * 14.0, ts * 3.1)) - 0.5) * 0.008;
    let w2 = (lab_noise(vec2(p.x * 11.0 + 7.0, ts * 5.7)) - 0.5) * 0.008;
    let inn = step(0.0, p.x) * step(p.x, p.big_l);
    let taper = 1.0 - lab_sstep(p.big_l - 0.07, p.big_l, p.x);
    let rr = 0.022 * mix(0.08, 1.0, taper);
    // A line at least a pixel wide, as bright in all (the lab's are thinner than a pixel
    // far off).
    let l1 = exp(-abs(abs(p.y - w1) - rr) / max(1.0 / 1100.0, p.px * 0.5)) * min(1.0, 1.0 / (1100.0 * p.px * 0.5));
    let l2 = exp(-abs(abs(p.y - w2) - rr * 1.05) / max(1.0 / 1300.0, p.px * 0.5)) * 0.6 * min(1.0, 1.0 / (1300.0 * p.px * 0.5));
    let h = abs(fract((p.x * 0.7 + p.y) * 48.0 + lab_hash(ts)) - 0.5);
    let hatch = lab_sstep(0.08, 0.0, h) * step(abs(p.y), rr * 0.9) * 0.42;
    let grain = 0.72 + 0.28 * lab_noise(vec2(p.x * 400.0, p.y * 400.0 + ts));
    let ink = vec3(0.93, 0.95, 1.0);
    return ink * (l1 + l2 + hatch) * inn * grain + lab_turn(vec3(0.55, 0.65, 1.0), p) * (exp(-p.d * 18.0) * 0.15 + exp(-p.d * 5.0) * 0.05);
}

fn lab_hourglass(p: LabPlace) -> vec3<f32> {
    // Two glass bulbs as wide as a stock blade, pinched at the middle, closed in a dome
    // at the tip and seated in the hilt; the sand runs from the upper bulb down into the
    // lower one in 24 seconds, then the glass flips with a flash.
    let period = 24.0;
    let ph = fract(p.t / period);
    let xn = p.x / max(p.big_l, 0.001);
    let wide = 0.024;
    let waist = 0.0045;
    let f = abs(2.0 * xn - 1.0);
    var r = waist + (wide - waist) * lab_sstep(0.0, 0.5, f);
    // The dome over the tip.
    let dome = lab_sstep(0.93, 1.0, xn);
    r *= sqrt(max(1.0 - dome * dome, 0.0));
    let inn = step(0.0, p.x) * step(xn, 1.0);
    let in_glass = lab_sstep(r, r * 0.85, abs(p.y)) * inn;
    let rim = exp(-abs(abs(p.y) - r) / max(1.0 / 800.0, p.px * 0.5)) * inn * 0.45;
    let h = p.big_l * 0.5;
    let top_end = h + (1.0 - ph) * p.big_l * 0.42;
    let bottom_end = ph * p.big_l * 0.42;
    // A sand surface heaped a little in the middle.
    let heap = (1.0 - abs(p.y) / wide) * 0.012;
    let sand_top = step(h, p.x) * step(p.x, top_end - heap * 0.5);
    let sand_bottom = step(p.x, bottom_end + heap);
    let grain = 0.7 + 0.3 * lab_noise(vec2(p.x * 500.0, p.y * 500.0));
    let glint = step(0.97, lab_hash2(floor(vec2(p.x * 300.0, p.y * 300.0)) + floor(p.t * 3.0)));
    let stream = step(bottom_end, p.x) * step(p.x, h) * exp(-abs(p.y) / max(1.0 / 2500.0, p.px * 0.5))
        * (0.55 + 0.45 * lab_noise(vec2(p.x * 120.0 - p.t * 14.0, 0.0))) * step(ph, 0.985);
    let sand = lab_turn(vec3(1.0, 0.78, 0.35), p);
    let s = sand * (sand_top + sand_bottom) * in_glass * (grain * 1.1 + glint * 0.8) + sand * stream * 1.4;
    let glass = vec3(0.55, 0.85, 1.0) * (rim + in_glass * 0.06);
    // The glass's highlight, a thin line down one side.
    let shine = exp(-abs(p.y - r * 0.55) / max(0.0012, p.px * 0.5)) * in_glass * 0.25;
    let flip = exp(-ph * 30.0) + exp(-(1.0 - ph) * 60.0);
    let halo = mix(vec3<f32>(1.0, 0.7, 0.3), vec3<f32>(0.5, 0.8, 1.0), 0.3) * (exp(-p.d * 16.0) * 0.22 + exp(-p.d * 5.0) * 0.06) * (1.0 + flip * 2.0);
    return s + glass + vec3(shine) + halo + vec3(1.0, 0.9, 0.7) * flip * in_glass * 0.8;
}

// The comic blade's halftone glow and speed lines (its inked body is the solid pass's).
fn lab_comic(p: LabPlace) -> vec3<f32> {
    let c = lab_turn(vec3(0.2, 0.7, 1.0), p);
    let cell = 0.011;
    let q = vec2(p.x, p.y) / cell;
    let g = fract(q) - 0.5;
    let id = floor(q);
    let cd = lab_seg((id.x + 0.5) * cell, (id.y + 0.5) * cell, p.big_l);
    let rad = clamp(exp(-cd * 17.0) * 0.64, 0.0, 0.6);
    // Round 22 (Creyon: the dots reached below the hilt): only beside the blade, from its emitter
    // up, fading in just past it.
    let dots = lab_sstep(rad, rad - 0.1, length(g)) * step(0.03, p.d) * lab_sstep(0.01, 0.06, p.x);
    var sp = 0.0;
    for (var i = 0; i < 6; i++) {
        let fi = f32(i);
        let yy = (lab_hash(fi * 3.3) * 2.0 - 1.0) * 0.13;
        let ph = fract(lab_hash(fi) + p.t * 1.9);
        let xx = ph * p.big_l;
        sp += exp(-abs(p.y - yy) * 900.0) * lab_sstep(0.25, 0.0, abs(p.x - xx)) * step(0.045, abs(yy));
    }
    var light = c * dots * 0.95 + vec3(1.0) * sp * p.swing * 0.9;
    // Round 19 (Creyon): now and then the blade turns to a flickering blue hologram for a
    // second (its `holo flicker` part): the glow goes holo blue, scan lines run up it and
    // thin interference lines cross it.
    let holo = lab_comic_holo(p);
    if holo > 0.0 {
        let scan = 0.55 + 0.45 * step(0.5, fract(p.x / 0.009 - p.t * 2.5));
        let blue = vec3(0.35, 0.75, 1.0);
        var ticks = 0.0;
        for (var i = 0; i < 2; i++) {
            let at = fract(lab_hash(floor(p.t * 3.0) + f32(i) * 7.0)) * p.shaft;
            ticks += exp(-abs(p.x - at) / max(0.0015, p.px)) * lab_sstep(0.16, 0.05, abs(p.y));
        }
        let level = max(light.r, max(light.g, light.b));
        light = mix(light, blue * level * scan, holo) + blue * ticks * 0.8 * holo;
    }
    return light;
}

// How much of a hologram the Clone Wars blade is now (0 to 1): about one second in every
// nine, flickering on and off as it comes and goes; 0 with its `holo flicker` part off.
fn lab_comic_holo(p: LabPlace) -> f32 {
    if (p.off & OFF_BANDS) != 0u || p.lingering {
        return 0.0;
    }
    let c = fract(p.t / 9.0 + 0.37) * 9.0;
    let on = lab_sstep(7.6, 7.75, c) * (1.0 - lab_sstep(8.85, 9.0, c));
    let flick = select(1.0, 0.35, lab_hash(floor(p.t * 24.0)) < 0.3);
    return on * flick;
}

fn lab_quicksilver(p: LabPlace) -> vec3<f32> {
    let sh = lab_noise(vec2(p.x * 30.0 - p.t * 2.0, p.y * 80.0 + p.t));
    return vec3(0.75, 0.82, 0.95) * (exp(-p.d * 26.0) * 0.45 + exp(-p.d * 7.0) * 0.1) * (0.8 + 0.4 * sh);
}

// The reactor's cycle (seconds into its 14): calm until 8, heating to 11, critical to
// 12.6 (red, strobing, venting), then a flash and it cools back to blue.
fn lab_reactor_heat(t: f32) -> vec3<f32> {
    // Its 30-second cycle: calm until 16, heating over 6 seconds, critical for 4 (red,
    // strobing, venting), then a flash and it cools over 4.
    let c = fract(t / 30.0) * 30.0;
    let cooling = 1.0 - lab_sstep(26.0, 30.0, c);
    let heating = lab_sstep(16.0, 22.0, c) * cooling;
    let critical = lab_sstep(21.6, 22.4, c) * (1.0 - lab_sstep(25.6, 26.2, c));
    let flash = exp(-max(c - 26.0, 0.0) * 4.0) * step(26.0, c);
    return vec3(heating, critical, flash);
}

fn lab_reactor(p: LabPlace) -> vec3<f32> {
    let core = lab_reactor_core(p);
    let heat = lab_reactor_heat(p.t);
    // The containment field: two magnetic field lines winding round the plasma, faster
    // and brighter as it heats.
    var field = 0.0;
    for (var i = 0; i < 2; i++) {
        let fi = f32(i);
        let ph = p.x * 52.0 - p.t * (3.0 + 6.0 * heat.x) + fi * PI;
        let yc = 0.017 * sin(ph);
        let front = 0.5 + 0.5 * cos(ph);
        field += exp(-abs(p.y - yc) / max(0.0012, p.px * 0.6)) * (0.25 + 0.75 * front);
    }
    field *= step(0.02, p.x) * step(p.x, p.big_l - 0.02) * (0.35 + 0.65 * heat.x + 0.5 * heat.y);
    let fc = mix(lab_turn(vec3(0.5, 0.85, 1.0), p), vec3(1.0, 0.55, 0.25), heat.x);
    // Arcs jumping between neighbouring rings while it is hot.
    var arcs = 0.0;
    let n = 5.0;
    for (var k = 0; k < 5; k++) {
        let x0 = fract((f32(k) + p.t * 0.1 * n) / n) * p.big_l;
        let x1 = x0 + p.big_l / n;
        let cycle = floor(p.t * 7.0 + f32(k) * 1.7);
        let on = step(0.55 - 0.5 * heat.x, lab_hash(cycle * 3.1 + f32(k)));
        let side = select(-1.0, 1.0, lab_hash(cycle + f32(k) * 5.0) < 0.5);
        let u = clamp((p.x - x0) / max(x1 - x0, 0.001), 0.0, 1.0);
        let jag = (lab_noise(vec2(p.x * 300.0, cycle)) - 0.5) * 0.008;
        let yc = side * (0.02 + 0.006 * sin(u * PI)) + jag;
        arcs += exp(-abs(p.y - yc) / max(0.0009, p.px * 0.5)) * step(x0, p.x) * step(p.x, x1) * on;
    }
    arcs *= heat.x * step(p.x, p.big_l);
    // The emitter coil glowing at the hilt end.
    let coil = exp(-length(vec2(p.x, p.y) - vec2(0.03, 0.0)) * 60.0) * (0.6 + 0.4 * sin(p.t * 6.0));
    return core + fc * field * 0.55 + vec3(0.85, 0.9, 1.0) * arcs * 0.9 + mix(lab_turn(vec3(0.4, 0.8, 1.0), p), vec3(1.0, 0.5, 0.2), heat.x) * coil;
}

fn lab_reactor_core(p: LabPlace) -> vec3<f32> {
    let n = 5.0;
    let sp = 0.1;
    let heat = lab_reactor_heat(p.t);
    let strobe = heat.y * (0.5 + 0.5 * step(0.5, fract(p.t * 2.0)));
    // The plasma throbs faster as it heats, and swells while critical.
    let throb = 1.0 + (0.08 + 0.25 * heat.x) * sin(p.t * (5.0 + 22.0 * heat.x));
    let ph = p.x / max(p.big_l, 0.001) * n - p.t * sp * n;
    let fr = fract(ph);
    let r = (0.008 + 0.012 * pow(sin(PI * fr), 2.0)) * (1.0 + 0.35 * heat.y) * throb;
    let calm = lab_turn(vec3(0.3, 0.7, 1.0), p);
    let pc = mix(mix(calm, vec3(1.0, 0.45, 0.12), heat.x), vec3(1.0, 0.18, 0.06), heat.y);
    let dd = length(vec2(max(0.0, max(-p.x, p.x - p.big_l)), max(0.0, abs(p.y) - r * 0.45)));
    // A coloured core, not a white one, so it reads as plasma from afar.
    let plasma = lab_saber(pc * (1.0 + strobe * 0.6), mix(vec3(1.0), pc, 0.35), dd, r * 0.9, 30.0);
    var rings = 0.0;
    for (var k = 0; k < 5; k++) {
        let xr = fract((f32(k) + p.t * sp * n) / n) * p.big_l;
        let rr = 0.026;
        let ay = min(abs(p.y) / rr, 1.0);
        let arc = xr + 0.004 * sqrt(1.0 - ay * ay);
        rings += exp(-abs(p.x - arc) / max(1.0 / 700.0, p.px * 0.5 / p.k)) * (0.35 + 0.9 * pow(ay, 6.0)) * step(abs(p.y), rr);
    }
    let rc = mix(mix(vec3<f32>(0.6, 0.95, 1.0), vec3<f32>(1.0, 0.6, 0.3), heat.x), vec3<f32>(1.0, 0.3, 0.15), heat.y);
    let flash = vec3(1.0, 0.95, 0.9) * heat.z * (exp(-p.d * 30.0) * 1.5 + exp(-p.d * 8.0) * 0.4);
    return plasma + rc * rings * step(0.0, p.x) * step(p.x, p.big_l) * (1.0 + strobe) + flash;
}

// The obsidian blade's outline: where on its flat shape (`q`: across, along as a share
// of the length), and how far from its edge (metres).
const LAB_OBSIDIAN_HALF: f32 = 0.026;

fn lab_seg_to(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

fn lab_obsidian_edge(p: LabPlace) -> f32 {
    let w = LAB_OBSIDIAN_HALF;
    let l = max(p.big_l, 0.0001);
    // The shape's own coordinates are across in metres and along as a share of the
    // blade (the lab stretches it to the blade's length); its distances are in those.
    let q = vec2(p.y, p.x / l);
    return min(
        min(lab_seg_to(q, vec2(-w, 0.0), vec2(-w, 0.86)), lab_seg_to(q, vec2(-w, 0.86), vec2(w * 0.25, 1.0))),
        min(lab_seg_to(q, vec2(w * 0.25, 1.0), vec2(w, 0.93)), lab_seg_to(q, vec2(w, 0.93), vec2(w, 0.0))),
    );
}

// Whether `q` (across, along as a share) is inside the obsidian blade's shape.
fn lab_obsidian_inside(q: vec2<f32>) -> bool {
    let w = LAB_OBSIDIAN_HALF;
    if q.y < 0.0 || abs(q.x) > w {
        return false;
    }
    // Left edge to 0.86, then a slope to the point at (w/4, 1), right edge to 0.93 then a
    // slope to the point.
    let left_top = select(0.86 + (q.x + w) / (w * 1.25) * 0.14, 0.86, q.x < -w);
    let right_top = 0.93 + (w - q.x) / (w * 0.75) * 0.07;
    let top = select(right_top, left_top, q.x <= w * 0.25);
    return q.y <= top;
}

fn lab_obsidian(p: LabPlace) -> vec3<f32> {
    let e = lab_obsidian_edge(p);
    let cr = lab_noise(vec2(p.x / max(p.big_l, 0.0001) * 70.0 - p.t * 9.0, p.t * 23.0 + p.y * 40.0));
    let spike = step(0.86, cr);
    let rim = exp(-e / max(1.0 / 420.0, p.px * 0.6)) * (0.8 + 0.4 * cr) + exp(-e * 55.0) * 0.28 + exp(-e * 140.0) * spike * 0.9;
    let halo = vec3(0.85, 0.9, 1.0) * (exp(-p.d * 30.0) * 0.1 + exp(-p.d * 8.0) * 0.04);
    return vec3(0.9, 0.94, 1.0) * rim + halo;
}

fn lab_bleeding(p: LabPlace) -> vec3<f32> {
    let th = asin(clamp(p.y / 0.03, -1.0, 1.0));
    let b = 0.22 + 0.48 * (0.5 + 0.5 * sin(p.t * 0.4)) + p.clash * 0.35;
    let lvl = b * p.big_l;
    let mask = lab_sstep(lvl, lvl - 0.2, p.x);
    let q = vec2(p.x * 26.0, th * 3.0);
    let i = floor(q);
    let f = fract(q);
    var m1 = 8.0;
    var m2 = 8.0;
    for (var a = -1; a <= 1; a++) {
        for (var c = -1; c <= 1; c++) {
            let o = vec2(f32(a), f32(c));
            let pp = o + vec2(lab_hash2(i + o), lab_hash2(i + o + 17.0)) - f;
            let dd = dot(pp, pp);
            if dd < m1 {
                m2 = m1;
                m1 = dd;
            } else if dd < m2 {
                m2 = dd;
            }
        }
    }
    let crack = lab_sstep(0.13, 0.0, sqrt(m2) - sqrt(m1));
    let body = lab_sstep(0.026, 0.016, p.d);
    // A chroma's colour is the pure part's; the corruption stays red.
    let pure = select(lab_turn(vec3(1.0, 0.1, 0.12), p), vec3(0.8, 0.88, 1.0), p.turn == 0.0);
    let white = lab_saber(pure, vec3(1.0), p.d, 0.016, 22.0);
    let red = lab_saber(vec3(1.0, 0.08, 0.1), vec3(1.0, 0.8, 0.8), p.d, 0.016, 20.0);
    let c = mix(white, red, mask * 0.85);
    return mix(c, vec3(1.0, 0.03, 0.05) * 1.6, crack * body * mask);
}

fn lab_monarch(p: LabPlace) -> vec3<f32> {
    return lab_saber(lab_turn(vec3(1.0, 0.55, 0.12), p), vec3(1.0, 0.95, 0.82), p.d, 0.016, 18.0) * (0.95 + 0.05 * sin(p.t * 2.0));
}

fn lab_phoenix(p: LabPlace) -> vec3<f32> {
    let ash = 1.0 - lab_sstep(0.35, 0.95, p.age);
    let n = lab_fbm(vec2(p.x * 7.0 - p.t * 2.6, p.y * 28.0 + p.t * 0.7));
    let n2 = lab_fbm(vec2(p.x * 15.0 - p.t * 4.0, p.y * 55.0 - p.t));
    let r = (0.02 + 0.03 * n) * (1.0 - lab_sstep(p.big_l - 0.08, p.big_l + 0.06, p.x) * 0.6);
    let dd = length(vec2(max(0.0, max(-p.x, p.x - p.big_l - 0.04 * n)), p.y));
    let flame = lab_sstep(r + 0.02, r * 0.3, abs(p.y)) * step(0.0, p.x) * step(p.x, p.big_l + 0.05);
    let heat = clamp(1.0 - abs(p.y) / (r + 0.02), 0.0, 1.0) * (0.6 + 0.6 * n2);
    var col = mix(vec3(0.75, 0.05, 0.02), vec3(1.0, 0.45, 0.05), lab_sstep(0.1, 0.5, heat));
    col = mix(col, vec3(1.0, 0.9, 0.5), lab_sstep(0.5, 0.85, heat));
    col = mix(col, vec3(1.0), lab_sstep(0.85, 1.0, heat));
    let fire = col * flame * 1.1 + vec3(1.0, 0.4, 0.08) * (exp(-dd * 14.0) * 0.35 + exp(-dd * 5.0) * 0.1);
    let ashes = vec3(0.32, 0.3, 0.28) * lab_sstep(0.022, 0.0, p.d) * (0.4 + 0.6 * lab_noise(vec2(p.x * 80.0, p.y * 200.0)))
        + vec3(1.0, 0.3, 0.05) * step(0.78, lab_noise(vec2(p.x * 60.0, p.t * 3.0))) * lab_sstep(0.016, 0.0, p.d);
    return mix(fire, ashes, ash);
}

fn lab_serpent(p: LabPlace) -> vec3<f32> {
    let x = p.x;
    let y = p.y;
    let t = p.t;
    let bl = lab_saber(lab_turn(vec3(0.1, 1.0, 0.45), p), vec3(0.9, 1.0, 0.92), p.d, 0.014, 22.0);
    let amp = 0.034;
    let k = 26.0;
    let w = 2.2;
    let xh = p.big_l * 0.9 + sin(t * 1.1) * 0.01;
    // The coils travel up toward the head, the serpent climbing the blade.
    let ph = k * x + t * w;
    let yc = amp * sin(ph);
    let z = cos(ph);
    let slope = amp * k * cos(ph);
    let dy = abs(y - yc) / sqrt(1.0 + slope * slope);
    let tail = lab_sstep(0.02, 0.25, x);
    let thk = 0.0085 * mix(0.35, 1.0, tail);
    let body_on = step(0.02, x) * step(x, xh - 0.008);
    let body = lab_sstep(thk, thk * 0.5, dy) * body_on;
    let scales = 0.72 + 0.28 * sin(x * 260.0 - t * 2.0) * sin((y - yc) * 900.0);
    let vis = mix(0.28, 1.0, step(0.0, z));
    let gold = vec3(1.0, 0.78, 0.25);
    let bc = gold * body * scales * vis * 1.25 + gold * exp(-dy * 120.0) * 0.22 * body_on * vis;
    let hph = k * xh + t * w;
    let hy = amp * sin(hph);
    let hp = vec2(x - xh, y - hy);
    let jaw = lab_sstep(0.7, 1.0, 0.5 + 0.5 * sin(t * 1.7));
    let hq = hp / vec2(0.028, 0.0145);
    var head = lab_sstep(1.0, 0.75, length(hq - vec2(-0.1, 0.0)));
    let mouth = step(0.3, hq.x) * step(abs(hq.y), 0.5 * jaw);
    head *= 1.0 - mouth;
    let e = exp(-length(hp - vec2(0.005, 0.0058)) * 900.0) + exp(-length(hp - vec2(0.005, -0.0058)) * 900.0);
    let tongue = exp(-abs(hp.y - sin(t * 40.0) * 0.002) * 1600.0) * step(0.025, hp.x) * step(hp.x, 0.05) * step(0.5, jaw);
    return bl * (1.0 - body * step(0.0, z) * 0.6) + bc + gold * head * 1.3 + vec3(1.0, 0.08, 0.04) * e * 2.2 + vec3(1.0, 0.35, 0.4) * tongue;
}

// Quantum without its ghost copies: the blade itself, jittering while held still, with
// its interference fringes.
fn lab_quantum(p: LabPlace) -> vec3<f32> {
    let jit = (lab_hash(floor(p.t * 12.0)) - 0.5) * 0.004 * (1.0 - p.swing);
    let yy = p.y - jit;
    let dd = length(vec2(max(0.0, max(-p.p.x, p.p.x - p.shaft)), yy));
    let h = 0.5 + 0.5 * sin(p.x * 14.0 - p.t * 3.0);
    let g = lab_turn(mix(vec3(0.25, 0.85, 1.0), vec3(0.7, 0.35, 1.0), h), p);
    // A standing wave along it: bright antinodes, dim nodes drifting slowly.
    let wave = 0.75 + 0.25 * cos(p.x * 38.0 - sin(p.t * 0.7) * 3.0) * cos(p.t * 2.3);
    var c = lab_saber(g, vec3(0.92, 0.97, 1.0), dd, 0.015, 22.0) * wave;
    c *= 0.82 + 0.18 * cos(yy * 480.0 - p.t * 6.0 + p.x * 20.0);
    // Tunnelling: bright points that vanish and appear elsewhere on the blade.
    var blips = 0.0;
    for (var i = 0; i < 4; i++) {
        let fi = f32(i);
        let cycle = floor(p.t * 3.0 + lab_hash(fi * 3.7));
        let at = vec2(lab_hash(cycle * 1.31 + fi * 7.1) * p.shaft, (lab_hash(cycle * 2.7 + fi) - 0.5) * 0.02);
        blips += exp(-length(p.p - at) / max(0.004, p.px)) * step(0.4, lab_hash(cycle + fi * 11.0));
    }
    // The collapse: a flash running up the blade as a swing begins.
    let collapse = exp(-p.since_swing * 5.0) * exp(-abs(p.x / max(p.big_l, 0.001) - p.since_swing * 4.0) * 8.0)
        * exp(-dd * 40.0) * 2.0;
    return c + g * blips * 1.2 + vec3(1.0) * collapse;
}

// Quantum's ghost copies: two faint copies of the blade at other angles about its hilt,
// drifting round while it is held still and collapsing into it when it swings.
fn lab_quantum_ghosts(p: LabPlace) -> vec3<f32> {
    // Apart while it is held, close and flickering in superposition while it swings,
    // never quite gone.
    let held = mix(0.45, 1.0, 1.0 - clamp(p.swing * 2.0, 0.0, 1.0));
    var light = vec3(0.0);
    for (var k = 0; k < 2; k++) {
        let fk = f32(k);
        let side = select(-1.0, 1.0, k == 0);
        let angle = (0.1 + 0.04 * sin(p.t * 1.3 + fk)) * held * side * (0.6 + 0.4 * cos(p.t * 0.7 + fk * 2.4));
        let ghost = lab_turned(p, angle);
        let fade = 0.42 * held * (0.85 + 0.15 * sin(p.t * 9.0 + fk * 3.0));
        let e = 1.0 - lab_sstep(0.045, 0.098, max(ghost.d, abs(ghost.y)));
        light += lab_quantum(ghost) * fade * e;
    }
    return light;
}

fn lab_blade(n: u32, p: LabPlace) -> vec3<f32> {
    switch n {
        case 1u: { return lab_peppermint(p); }
        case 2u: { return lab_bubble(p); }
        case 3u: { return lab_lavalamp(p); }
        case 4u: { return lab_sketch(p); }
        case 5u: { return lab_hourglass(p); }
        case 6u: { return lab_comic(p); }
        case 7u: { return lab_quicksilver(p); }
        case 8u: { return lab_reactor(p); }
        case 9u: { return lab_obsidian(p); }
        case 10u: { return lab_bleeding(p); }
        case 11u: { return lab_monarch(p); }
        case 12u: { return lab_phoenix(p); }
        case 13u: { return lab_serpent(p); }
        case 14u: { return lab_quantum(p); }
        default: { return vec3(0.0); }
    }
}

// --- The lab's particles, each a slot reborn every `life` seconds at a new place ---------

// A slot's age and its life's number, `salt` the design's.
fn lab_cycle(i: f32, life: f32, t: f32, salt: f32) -> vec2<f32> {
    let ph = t / life + lab_hash(i * 7.31 + salt * 1.37);
    return vec2(fract(ph) * life, floor(ph));
}

// A slot's draw `j` for its life `cycle`, in [0, 1).
fn lab_rand(i: f32, cycle: f32, j: f32, salt: f32) -> f32 {
    return lab_hash(i * 12.9898 + fract(cycle * 0.0137) * 7853.1 + j * 37.719 + salt * 3.91);
}

// A place on the blade on screen: `u` of the way up, `radius` out at `angle` round it.
fn lab_on_blade(p: LabPlace, u: f32, radius: f32, angle: f32) -> vec2<f32> {
    return vec2(u * p.shaft, radius * cos(angle));
}

// A particle's sprite at `at` (along, across), `size` across (metres, as the lab's point
// size): the place in it (-1 to 1: across, along) and how much of its light a pixel-wide
// sprite keeps (a sprite never narrower than a pixel, its light spread over it).
fn lab_sprite(p: LabPlace, at: vec2<f32>, size: f32) -> vec3<f32> {
    let half = size * 0.5;
    let shown = max(half, p.px);
    let q = (p.p - at) / shown;
    return vec3(q.y, q.x, (half / shown) * (half / shown));
}

// The lab's additive sprite kinds: 0 a soft dot, 1 a spark, 2 a bubble, 4 a feather, 5 a
// four-pointed glint. `extra` a bubble's hue or a feather's turn.
fn lab_sprite_light(kind: u32, s: vec3<f32>, color: vec3<f32>, extra: f32) -> vec3<f32> {
    let q = s.xy;
    if abs(q.x) > 1.0 || abs(q.y) > 1.0 {
        return vec3(0.0);
    }
    let r = length(q);
    var c = color;
    var a = 0.0;
    switch kind {
        case 0u: { a = exp(-r * r * 4.0); }
        case 1u: {
            a = exp(-r * r * 9.0) * 1.4;
            c = mix(c, vec3(1.0), exp(-r * r * 30.0));
        }
        case 2u: {
            if r > 1.0 {
                return vec3(0.0);
            }
            let ring = exp(-abs(r - 0.82) * 13.0);
            let spot = exp(-length(q - vec2(-0.35, 0.35)) * 11.0);
            c = mix(lab_hsv(atan2(q.y, q.x) / TAU + extra, 0.55, 1.0), vec3(1.0), spot);
            a = ring * 0.75 + spot * 0.9;
        }
        case 4u: {
            let cs = cos(extra);
            let sn = sin(extra);
            let w = vec2(cs * q.x - sn * q.y, sn * q.x + cs * q.y);
            let f = length(w / vec2(0.28, 0.95));
            if f > 1.0 {
                return vec3(0.0);
            }
            a = (1.0 - f) * 1.4 + exp(-abs(w.x) * 40.0) * 0.4;
        }
        default: {
            a = max(exp(-abs(q.x) * 12.0) * exp(-abs(q.y) * 2.6), exp(-abs(q.y) * 12.0) * exp(-abs(q.x) * 2.6)) * 1.3;
        }
    }
    return c * a * s.z;
}

// The lab's fade: in over `fade_in` seconds, out with the square of its age's share.
fn lab_fade(age: f32, life: f32, fade_in: f32) -> f32 {
    let k = clamp(age / life, 0.0, 1.0);
    return min(1.0, age / fade_in) * (1.0 - k * k);
}

// The additive particles of design `n` at this fragment.
fn lab_motes(n: u32, p: LabPlace) -> vec3<f32> {
    var light = vec3(0.0);
    let t = p.t;
    let side = vec2(p.up.y, -p.up.x);
    switch n {
        case 1u: {
            // Sugar glints round the candy.
            for (var i = 0; i < 10; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 0.7, t, 1.0);
                let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 1.0), 0.034, lab_rand(fi, c.y, 1.0, 1.0) * TAU)
                    + p.up * mix(-0.02, 0.08, lab_rand(fi, c.y, 2.0, 1.0)) * c.x
                    + side * (lab_rand(fi, c.y, 3.0, 1.0) - 0.5) * 0.12 * c.x;
                let col = select(vec3<f32>(1.0, 0.6, 0.65), vec3<f32>(1.0), lab_rand(fi, c.y, 4.0, 1.0) < 0.5);
                let s = lab_sprite(p, at, mix(0.02, 0.03, lab_rand(fi, c.y, 5.0, 1.0)));
                light += lab_sprite_light(5u, s, col, 0.0) * lab_fade(c.x, 0.7, 0.06);
            }
        }
        case 2u: {
            // Bubbles blown off the film, rising, each popping into a puff of droplets.
            for (var i = 0; i < 18; i++) {
                let fi = f32(i);
                let life = 3.2;
                let c = lab_cycle(fi, life, t, 2.0);
                let pops = life * mix(0.6, 1.0, lab_rand(fi, c.y, 6.0, 2.0));
                let start = lab_on_blade(p, mix(0.05, 1.0, lab_rand(fi, c.y, 0.0, 2.0)), 0.03, lab_rand(fi, c.y, 1.0, 2.0) * TAU);
                let v0 = side * (lab_rand(fi, c.y, 2.0, 2.0) - 0.5) * 0.16 + p.up * mix(0.08, 0.2, lab_rand(fi, c.y, 3.0, 2.0));
                let a = min(c.x, pops);
                let at = start + v0 * (1.0 - exp(-0.4 * a)) / 0.4 + p.up * 0.015 * a * a;
                if c.x < pops {
                    // Blown up from nothing as it leaves the film.
                    let grown = lab_sstep(0.0, 0.5, c.x);
                    let s = lab_sprite(p, at, mix(0.05, 0.11, lab_rand(fi, c.y, 4.0, 2.0)) * mix(0.3, 1.0, grown));
                    light += lab_sprite_light(2u, s, vec3(1.0), lab_rand(fi, c.y, 5.0, 2.0) + p.turn) * 1.4 * min(1.0, c.x / 0.3);
                } else if c.x < pops + 0.22 {
                    let since = c.x - pops;
                    for (var j = 0; j < 7; j++) {
                        let fj = f32(j);
                        let dir = fj / 7.0 * TAU + lab_rand(fi, c.y, 7.0 + fj, 2.0);
                        let speed = mix(0.2, 0.5, lab_rand(fi, c.y, 15.0 + fj, 2.0));
                        let drop = at + vec2(cos(dir), sin(dir)) * speed * since;
                        light += lab_sprite_light(0u, lab_sprite(p, drop, 0.012), vec3(0.9, 0.95, 1.0), 0.0) * lab_fade(since, 0.22, 0.06);
                    }
                }
            }
        }
        case 3u: {
            // Warm specks drifting up off the lamp.
            for (var i = 0; i < 4; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 1.6, t, 3.0);
                let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 3.0), 0.04, lab_rand(fi, c.y, 1.0, 3.0) * TAU)
                    + p.up * mix(0.05, 0.12, lab_rand(fi, c.y, 2.0, 3.0)) * c.x;
                light += lab_sprite_light(0u, lab_sprite(p, at, 0.02), vec3(1.0, 0.45, 0.2), 0.0) * 0.6 * lab_fade(c.x, 1.6, 0.06);
            }
        }
        case 4u: {
            // Doodled stars round the drawing.
            for (var i = 0; i < 3; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 0.5, t, 4.0);
                let at = lab_on_blade(p, mix(0.1, 1.0, lab_rand(fi, c.y, 0.0, 4.0)), mix(0.06, 0.12, lab_rand(fi, c.y, 1.0, 4.0)),
                    lab_rand(fi, c.y, 2.0, 4.0) * TAU);
                let s = lab_sprite(p, at, mix(0.03, 0.05, lab_rand(fi, c.y, 3.0, 4.0)));
                light += lab_sprite_light(5u, s, vec3(0.9, 0.93, 1.0), 0.0) * lab_fade(c.x, 0.5, 0.001);
            }
        }
        case 5u: {
            // Sand spilling out of a hard swing and falling.
            if p.swing > 0.35 {
                for (var i = 0; i < 24; i++) {
                    let fi = f32(i);
                    let c = lab_cycle(fi, 0.85, t, 5.0);
                    if lab_rand(fi, c.y, 9.0, 5.0) < p.swing {
                        let at = lab_on_blade(p, mix(0.05, 0.95, lab_rand(fi, c.y, 0.0, 5.0)), 0.02, lab_rand(fi, c.y, 1.0, 5.0) * TAU)
                            + side * (lab_rand(fi, c.y, 2.0, 5.0) - 0.5) * 0.3 * c.x - p.up * 1.7 * c.x * c.x;
                        light += lab_sprite_light(0u, lab_sprite(p, at, 0.012), vec3(1.0, 0.8, 0.4), 0.0) * lab_fade(c.x, 0.85, 0.06);
                    }
                }
            }
        }
        case 8u: {
            // Vented sparks, a shower while it overheats.
            let ov = lab_reactor_heat(t).y > 0.5;
            for (var i = 0; i < 20; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 0.35, t, 8.0);
                if lab_rand(fi, c.y, 9.0, 8.0) < select(0.04, 1.0, ov) {
                    let start = lab_on_blade(p, mix(0.6, 1.0, lab_rand(fi, c.y, 0.0, 8.0)), 0.02, lab_rand(fi, c.y, 1.0, 8.0) * TAU);
                    let out = normalize(start + vec2(0.0, 0.0001));
                    let at = start + out * mix(0.6, 1.6, lab_rand(fi, c.y, 2.0, 8.0)) * c.x - p.up * c.x * c.x;
                    let col = select(vec3<f32>(0.5, 0.85, 1.0), vec3<f32>(1.0, 0.55, 0.2), ov);
                    light += lab_sprite_light(1u, lab_sprite(p, at, mix(0.01, 0.02, lab_rand(fi, c.y, 3.0, 8.0))), col, 0.0)
                        * lab_fade(c.x, 0.35, 0.06);
                }
            }
        }
        case 9u: {
            // Sparks spitting off the white edge.
            for (var i = 0; i < 3; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 0.18, t, 9.0);
                if lab_rand(fi, c.y, 9.0, 9.0) < 0.4 {
                    let s = select(-1.0, 1.0, lab_rand(fi, c.y, 0.0, 9.0) < 0.5);
                    let start = vec2(lab_rand(fi, c.y, 1.0, 9.0) * 0.85 * p.shaft, s * LAB_OBSIDIAN_HALF);
                    let at = start + vec2(0.0, s) * mix(0.4, 1.0, lab_rand(fi, c.y, 2.0, 9.0)) * c.x
                        + p.up * mix(-0.2, 0.4, lab_rand(fi, c.y, 3.0, 9.0)) * c.x;
                    light += lab_sprite_light(1u, lab_sprite(p, at, 0.012), vec3(0.9, 0.95, 1.0), 0.0) * lab_fade(c.x, 0.18, 0.02);
                }
            }
        }
        case 11u: {
            // Pollen floating round the butterflies.
            for (var i = 0; i < 8; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 2.0, t, 11.0);
                let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 11.0), mix(0.03, 0.1, lab_rand(fi, c.y, 1.0, 11.0)),
                    lab_rand(fi, c.y, 2.0, 11.0) * TAU)
                    + p.up * mix(0.02, 0.07, lab_rand(fi, c.y, 3.0, 11.0)) * c.x + side * (lab_rand(fi, c.y, 4.0, 11.0) - 0.5) * 0.06 * c.x;
                light += lab_sprite_light(0u, lab_sprite(p, at, 0.012), vec3(1.0, 0.85, 0.4), 0.0) * 0.7 * lab_fade(c.x, 2.0, 0.06);
            }
        }
        case 12u: {
            if p.age > 0.6 {
                // Flames licking up off the fire.
                for (var i = 0; i < 22; i++) {
                    let fi = f32(i);
                    let c = lab_cycle(fi, 0.85, t, 12.0);
                    let v = p.up * mix(0.3, 0.7, lab_rand(fi, c.y, 2.0, 12.0)) + side * (lab_rand(fi, c.y, 3.0, 12.0) - 0.5) * 0.2;
                    let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 12.0), 0.02, lab_rand(fi, c.y, 1.0, 12.0) * TAU)
                        + v * (1.0 - exp(-0.6 * c.x)) / 0.6;
                    let col = vec3(1.0, mix(0.35, 0.6, lab_rand(fi, c.y, 4.0, 12.0)), 0.08);
                    light += lab_sprite_light(1u, lab_sprite(p, at, mix(0.008, 0.016, lab_rand(fi, c.y, 5.0, 12.0))), col, 0.0)
                        * lab_fade(c.x, 0.85, 0.06);
                }
                // Burning feathers shed by the flame, more on a swing, greying as they fall.
                for (var i = 0; i < 12; i++) {
                    let fi = f32(i);
                    let c = lab_cycle(fi, 1.8, t, 112.0);
                    if lab_rand(fi, c.y, 9.0, 112.0) < (3.0 + p.swing * 28.0) / 6.7 {
                        let v = side * (lab_rand(fi, c.y, 2.0, 112.0) - 0.5) * 0.3 + p.up * mix(-0.05, 0.2, lab_rand(fi, c.y, 3.0, 112.0));
                        let at = lab_on_blade(p, mix(0.1, 1.0, lab_rand(fi, c.y, 0.0, 112.0)), 0.03, lab_rand(fi, c.y, 1.0, 112.0) * TAU)
                            + v * (1.0 - exp(-1.2 * c.x)) / 1.2 - p.up * 0.1 * c.x * c.x;
                        let k = c.x / 1.8;
                        let col = vec3(1.0 - 0.62 * k, 0.5 - 0.18 * k, 0.1 + 0.2 * k);
                        let turn = lab_rand(fi, c.y, 4.0, 112.0) * TAU + c.x * 1.5;
                        light += lab_sprite_light(4u, lab_sprite(p, at, mix(0.05, 0.08, lab_rand(fi, c.y, 5.0, 112.0))), col, turn)
                            * lab_fade(c.x, 1.8, 0.06);
                    }
                }
            } else {
                // Embers in the ash.
                for (var i = 0; i < 6; i++) {
                    let fi = f32(i);
                    let c = lab_cycle(fi, 0.6, t, 212.0);
                    let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 212.0), 0.015, lab_rand(fi, c.y, 1.0, 212.0) * TAU)
                        + p.up * mix(0.05, 0.2, lab_rand(fi, c.y, 2.0, 212.0)) * c.x;
                    light += lab_sprite_light(1u, lab_sprite(p, at, 0.008), vec3(1.0, 0.35, 0.05), 0.0) * lab_fade(c.x, 0.6, 0.06);
                }
            }
        }
        case 13u: {
            // Green motes rising off the blade.
            for (var i = 0; i < 4; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 1.2, t, 13.0);
                let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 13.0), 0.04, lab_rand(fi, c.y, 1.0, 13.0) * TAU) + p.up * 0.05 * c.x;
                light += lab_sprite_light(0u, lab_sprite(p, at, 0.012), vec3(0.4, 1.0, 0.6), 0.0) * 0.6 * lab_fade(c.x, 1.2, 0.06);
            }
        }
        case 14u: {
            // The probability cloud, gone while it swings.
            let shown = 1.0 - clamp(p.swing * 1.8, 0.0, 1.0);
            for (var i = 0; i < 14; i++) {
                let fi = f32(i);
                let c = lab_cycle(fi, 0.19, t, 14.0);
                if lab_rand(fi, c.y, 9.0, 14.0) < shown {
                    let out = abs(lab_rand(fi, c.y, 1.0, 14.0) + lab_rand(fi, c.y, 2.0, 14.0) - 1.0) * 2.0 * 0.06 + 0.015;
                    let at = lab_on_blade(p, lab_rand(fi, c.y, 0.0, 14.0), out, lab_rand(fi, c.y, 3.0, 14.0) * TAU);
                    let col = select(vec3<f32>(0.7, 0.4, 1.0), vec3<f32>(0.3, 0.85, 1.0), lab_rand(fi, c.y, 4.0, 14.0) < 0.5);
                    light += lab_sprite_light(0u, lab_sprite(p, at, mix(0.008, 0.016, lab_rand(fi, c.y, 5.0, 14.0))), col, 0.0)
                        * lab_fade(c.x, 0.19, 0.001);
                }
            }
        }
        default: {}
    }
    return light;
}

// A design's glow: its blade fading out at its reach, its particles fading at their
// room's edge; turned to its wearer's colour for a chroma, faded for the persona.
fn lab_glow(input: VertexOutput, pixel: f32) -> vec3<f32> {
    let design = skins[skin_index(input)].design;
    let n = u32(design.x + 0.5);
    // An afterimage draws nothing.
    if afterimage(input.persona.x, input.off.x) {
        return vec3(0.0);
    }
    let p = lab_place(input, pixel);
    if p.lingering {
        return vec3(0.0);
    }
    let reach = design.y;
    let e = 1.0 - lab_sstep(reach * 0.45, reach * 0.98, max(p.d, abs(p.y)));
    var light = vec3(0.0);
    if e > 0.0 {
        light = max(lab_blade(n, p), vec3(0.0)) * e;
    }
    // The emitter: light where the blade leaves the hilt, as a stock blade's hilt sprite
    // gives, so no gap shows between hilt and blade.
    let mouth = length(p.p - vec2(LAB_START * p.k, 0.0));
    let emitter = lab_turn(skins[skin_index(input)].design_color.rgb, p);
    light += emitter * (exp(-mouth / 0.012) * 0.9 + exp(-mouth / 0.035) * 0.22);
    // Quantum's ghost copies, an option its wearer may switch off.
    if n == LAB_QUANTUM && (input.off.x & OFF_COPIES) == 0u {
        light += max(lab_quantum_ghosts(p), vec3(0.0));
    }
    let room = design.z;
    // Its particles, unless its wearer switched them off (the `particles` option, the
    // arcs bit: blade_lab_designs.rs `Design::shown`).
    if room > 0.0 && p.d < room && (input.off.x & OFF_ARCS) == 0u {
        light += lab_motes(n, p) * (1.0 - lab_sstep(room * 0.75, room, p.d));
    }
    return light * persona_fade(input);
}

// --- The lab's solids and blended particles: the solid pass ------------------------------

// A blended particle (the lab's second set): 3 a butterfly (`extra` how far its wings
// are open), 6 a drop. Colour and coverage.
fn lab_sprite_cover(kind: u32, s: vec3<f32>, color: vec3<f32>, extra: f32) -> vec4<f32> {
    let q = s.xy;
    if abs(q.x) > 1.0 || abs(q.y) > 1.0 {
        return vec4(0.0);
    }
    if kind == 3u {
        let fl = max(extra, 0.12);
        let w = vec2(abs(q.x) / fl, q.y);
        let upper = length((w - vec2(0.5, 0.3)) / vec2(0.5, 0.42));
        let lower = length((w - vec2(0.36, -0.3)) / vec2(0.36, 0.3));
        let wing = min(upper, lower);
        let body = step(abs(q.x), 0.045) * step(abs(q.y), 0.62);
        if wing > 1.0 && body < 0.5 {
            return vec4(0.0);
        }
        let edge = lab_sstep(0.78, 0.9, wing);
        let vein = lab_sstep(0.9, 0.97, abs(sin(atan2(w.y, w.x - 0.05) * 7.0)));
        var c = mix(color, vec3(0.02), max(edge, vein * 0.8));
        let dots = step(0.86, wing) * step(0.6, fract(atan2(w.y, w.x) * 3.0));
        c = mix(c, vec3(1.0), dots * 0.8);
        if body > 0.5 {
            c = vec3(0.03);
        }
        return vec4(c, min(s.z * 4.0, 1.0));
    }
    let r = length(q);
    if r > 0.85 {
        return vec4(0.0);
    }
    var c = mix(color * 0.7, color * 1.25, lab_sstep(0.6, -0.4, q.y));
    c = mix(c, vec3(1.0), exp(-length(q - vec2(-0.28, 0.3)) * 14.0) * 0.8);
    return vec4(c, lab_sstep(0.85, 0.7, r) * min(s.z * 4.0, 1.0));
}

// `top` over `under`, both straight colour and coverage.
fn lab_over(under: vec4<f32>, top: vec4<f32>) -> vec4<f32> {
    let a = top.a + under.a * (1.0 - top.a);
    if a <= 0.0 {
        return vec4(0.0);
    }
    return vec4((top.rgb * top.a + under.rgb * under.a * (1.0 - top.a)) / a, a);
}

// Round 19 (Creyon): the Clone Wars blade's comic panel, its `comic panel` part: each swing
// leaves the comic's motion behind the blade for a moment, see-through (Creyon: no grey box): a
// patch of halftone dots in the blade's colour, bigger toward the blade, and thin speed lines
// streaking back along the swing, all fading softly at their edges.
fn lab_comic_panel(p: LabPlace, color: vec3<f32>) -> vec4<f32> {
    if (p.off & OFF_SPUTTER) != 0u || p.lingering {
        return vec4(0.0);
    }
    let age = p.since_swing - 0.04;
    if age < 0.0 || age > 0.9 {
        return vec4(0.0);
    }
    let born = floor((p.t - age) * 10.0);
    // Behind the blade, on the side away from the swing's word.
    let side = select(1.0, -1.0, lab_hash(born + 5.0) < 0.5);
    let fade = lab_sstep(0.0, 0.06, age) * (1.0 - lab_sstep(0.45, 0.9, age));
    // across: 0 at the blade to 1 at the patch's far edge; along: -1 to 1 over the blade's
    // upper part
    let across = p.p.y * side / 0.16;
    let along = (p.p.x - 0.62 * p.shaft) / (0.32 * p.shaft);
    if across < -0.1 || across > 1.2 || abs(along) > 1.1 {
        return vec4(0.0);
    }
    let soft_edge = lab_sstep(-0.1, 0.1, across) * (1.0 - lab_sstep(0.6, 1.15, across))
        * (1.0 - lab_sstep(0.6, 1.05, abs(along)));
    // halftone dots, shrinking away from the blade
    let cell = 0.011;
    let g = fract(p.p / cell) - 0.5;
    let radius = 0.42 * (1.0 - clamp(across, 0.0, 1.0)) + 0.05;
    let dots = clamp((radius - length(g)) * cell / max(p.px, 0.0001) + 0.5, 0.0, 1.0);
    // speed lines: thin streaks along the blade's length at their own spacings
    let lane = across * 9.0 + lab_hash(born + 3.0) * 3.0;
    let streak = lab_hash(floor(lane) + born * 1.7);
    let thin = 1.0 - lab_sstep(0.06, 0.14, abs(fract(lane) - 0.5) - 0.32);
    let length_on = step(0.45, streak) * lab_sstep(-1.0 + streak, -0.6 + streak, along)
        * (1.0 - lab_sstep(0.5 + 0.5 * streak - 0.2, 0.5 + 0.5 * streak, along));
    let lines = thin * length_on * lab_sstep(0.15, 0.3, across);
    let shown = max(dots * 0.55, lines * 0.7) * soft_edge * fade;
    let rgb = mix(color, mix(color, vec3(1.0), 0.6), lines);
    return vec4(rgb, shown);
}

// Round 19 (Creyon): the Clone Wars blade's action burst, its `action burst` part: on a swing
// or clash a ring of jagged impact lines in the blade's colour bursts out from the blade, away
// from the word (Creyon: no white bubble behind the words, which made them harder to read).
fn lab_comic_burst(p: LabPlace, at: vec2<f32>, age: f32, color: vec3<f32>, seed: f32, size: f32) -> vec4<f32> {
    if (p.off & OFF_GLITCH) != 0u {
        return vec4(0.0);
    }
    let life = 0.6;
    let k = age / life;
    if k < 0.0 || k >= 1.0 {
        return vec4(0.0);
    }
    let v = p.p - at;
    let r = length(v);
    let reach = size * (0.55 + 0.45 * lab_sstep(0.0, 0.35, k));
    if r > reach * 1.3 || r < size * 0.25 {
        return vec4(0.0);
    }
    let spikes = 12.0;
    let f = atan2(v.y, v.x) / TAU * spikes + seed * 3.7;
    let i = floor(f);
    // each line along its spike, from just off the blade out to its own length
    let off = abs(fract(f) - 0.5) / spikes * TAU * r;
    let len = reach * (0.7 + 0.3 * lab_hash(i + seed * 17.0));
    let inner = size * (0.3 + 0.35 * k);
    let along = lab_sstep(inner, inner + 0.01, r) * (1.0 - lab_sstep(len - 0.01, len, r));
    let soft = p.px;
    let w = 0.0045 * (1.0 - 0.5 * k);
    let ink = clamp((w + 0.003 - off) / soft + 0.5, 0.0, 1.0) * along;
    let fill = clamp((w - off) / soft + 0.5, 0.0, 1.0) * along;
    let rgb = mix(vec3(0.0), mix(color, vec3(1.0), 0.35), fill);
    return vec4(rgb, ink * (1.0 - lab_sstep(0.6, 1.0, k)));
}

// The comic font: five by six, a bit a cell from the top left; V M H S W O A K Z T R B !.
const LAB_FONT: array<u32, 13> = array<u32, 13>(
    145049137u, 588830577u, 588840497u, 520632382u, 599447089u, 488162862u, 589284910u,
    588553521u, 1041305887u, 138547359u, 580042287u, 521715247u, 134353028u,
);
// The words, four bits a letter (15 ends one): VMMMM! on lighting and switching off (Creyon's
// round 13); SHWOOM! VWOSH! ZZAKT! on a swing; KRAKK! SHZAK! BZZT! on a clash.
const LAB_WORDS: array<u32, 7> = array<u32, 7>(
    4290842896u, 4229256227u, 4290917696u, 4291393160u, 4291262119u, 4291258403u, 4294744203u,
);

fn lab_word_letter(word: u32, i: i32) -> u32 {
    if i < 0 || i > 7 {
        return 15u;
    }
    return (LAB_WORDS[word] >> (u32(i) * 4u)) & 15u;
}

fn lab_word_length(word: u32) -> i32 {
    var n = 0;
    for (var i = 0; i < 8; i++) {
        if lab_word_letter(word, i) == 15u {
            break;
        }
        n = i + 1;
    }
    return n;
}

// Whether the word's cell (column, row; six columns a letter, a gap last) is inked.
fn lab_word_cell(word: u32, cell: vec2<i32>) -> bool {
    if cell.y < 0 || cell.y > 5 || cell.x < 0 {
        return false;
    }
    let letter = lab_word_letter(word, cell.x / 6);
    let column = cell.x % 6;
    if letter == 15u || column == 5 {
        return false;
    }
    return ((LAB_FONT[letter] >> u32(cell.y * 5 + column)) & 1u) == 1u;
}

// A comic word `age` seconds old, born at `at` (along, across): yellow letters with a
// thick black outline and a thin line of the blade's colour round them, popping up,
// growing and rising as it fades (the lab's sprite).
fn lab_comic_word(p: LabPlace, word: u32, at: vec2<f32>, tilt: f32, age: f32, color: vec3<f32>) -> vec4<f32> {
    // Long enough to read: it pops up in a tenth of a second, holds, then fades over its
    // last third.
    let life = LAB_WORD_LIFE;
    let k = age / life;
    if k < 0.0 || k >= 1.0 {
        return vec4(0.0);
    }
    let size = select(0.66 + k * 0.06, age / 0.1 * 0.66, age < 0.1);
    let centre = at + p.up * 0.05 * age;
    // The word's frame: right and up on screen as the world's (across runs to the
    // screen's left).
    let right = vec2(-p.up.y, p.up.x);
    let v = p.p - centre;
    let cs = cos(tilt);
    let sn = sin(tilt);
    let local = vec2(dot(v, right), dot(v, p.up));
    let w = vec2(cs * local.x + sn * local.y, -sn * local.x + cs * local.y);
    let letters = lab_word_length(word);
    let columns = f32(letters * 6 - 1);
    let cell = size * 0.8 / columns;
    let g = vec2(w.x / cell + columns * 0.5, 3.0 - w.y / cell);
    if g.x < -2.5 || g.x > columns + 2.5 || g.y < -2.5 || g.y > 8.5 {
        return vec4(0.0);
    }
    let base = vec2<i32>(floor(g));
    var nearest = 4.0;
    for (var dy = -2; dy <= 2; dy++) {
        for (var dx = -2; dx <= 2; dx++) {
            let c = base + vec2(dx, dy);
            if lab_word_cell(word, c) {
                let q = abs(g - (vec2<f32>(c) + 0.5)) - vec2(0.42);
                let dist = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - 0.12;
                nearest = min(nearest, dist);
            }
        }
    }
    let soft = max(p.px / cell, 0.05);
    let ink = clamp((1.05 - nearest) / soft + 0.5, 0.0, 1.0);
    let fill = clamp((-nearest) / soft + 0.5, 0.0, 1.0);
    let stroke = clamp((0.16 - abs(nearest + 0.08)) / soft + 0.5, 0.0, 1.0);
    var rgb = mix(vec3(0.0), vec3(1.0, 0.886, 0.239), fill);
    // A line of the blade's colour round the letters, only where they are big enough
    // on screen for it not to drown the yellow.
    rgb = mix(rgb, color, stroke * fill * lab_sstep(0.25, 0.12, soft));
    return vec4(rgb, ink * (1.0 - lab_sstep(0.66, 1.0, k)));
}

// The comic words now showing: on ignition, while it goes out, the last swing and the last
// clash (Creyon's round 13: words for swings, lighting and switching off, not for running).
fn lab_comic_words(p: LabPlace, color: vec3<f32>) -> vec4<f32> {
    var out = vec4(0.0);
    // Lit: VMMMM! as it comes out.
    if p.age < LAB_WORD_LIFE {
        out = lab_over(out, lab_comic_word(p, 0u, vec2(0.45 * p.shaft, -0.14), 0.12, p.age, color));
    }
    // Going out: VMMMM! at once at full size, on the lingering whole-length blade only
    // (`lab_lingering`: it floats on after the blade has gone in, Creyon's round 14), held
    // for a second and more, then fading.
    if p.age >= SABER_GOING_OUT && p.lingering {
        let gone = p.age - SABER_GOING_OUT;
        let age = select(0.6, 1.2 + (gone - 1.2) * 1.5, gone > 1.2);
        out = lab_over(out, lab_comic_word(p, 0u, vec2(0.12 + 0.2 * p.shaft, 0.13), -0.12, age, color));
    }
    if p.lingering {
        return out;
    }
    let swung = p.since_swing - 0.08;
    if swung >= 0.0 && swung < LAB_WORD_LIFE {
        let born = floor((p.t - swung) * 10.0);
        let word = 1u + u32(lab_hash(born + 1.0) * 2.999);
        let side = select(-1.0, 1.0, lab_hash(born + 5.0) < 0.5);
        out = lab_over(out, lab_comic_word(p, word, vec2(0.7 * p.shaft, 0.17 * side), (lab_hash(born + 2.0) - 0.5) * 0.3, swung, color));
    }
    if p.since_clash < LAB_WORD_LIFE {
        let age = p.since_clash;
        let born = floor((p.t - age) * 10.0);
        let word = 4u + u32(lab_hash(born + 7.0) * 2.999);
        out = lab_over(out, lab_comic_word(p, word, vec2(0.6 * p.shaft, 0.0) + p.up * 0.14, (lab_hash(born + 8.0) - 0.5) * 0.3, age, color));
    }
    return out;
}

// The lit lane while a blade goes out (`saber_motion::GOING_OUT`).
const SABER_GOING_OUT: f32 = 2000.0;
// How long a comic word shows (seconds).
const LAB_WORD_LIFE: f32 = 1.8;

// The quicksilver blade: a mirror column wobbling like mercury, three beads swelling it
// as they roll up to the tip, and the drops they leave falling off it.
fn lab_quicksilver_body(p: LabPlace, light: f32) -> vec4<f32> {
    let v = p.x / max(p.big_l, 0.0001);
    if p.p.x < 0.0 || v > 1.0 {
        return vec4(0.0);
    }
    let t = p.t;
    // The side of the column this pixel sees (its angle round the axis).
    let side = clamp(p.y / 0.03, -1.0, 1.0);
    let angle = asin(side);
    var r = 0.022 * (1.0 + 0.1 * sin(v * 38.0 - t * 3.0 + sin(angle * 3.0 + t * 1.3) * 1.2));
    for (var b = 0; b < 3; b++) {
        let ph = fract(f32(b) * 0.33 + t * 0.16);
        let dv = (v - ph) / 0.03;
        r += 0.011 * exp(-dv * dv) * min(1.0, ph * 6.0);
    }
    if v > 0.955 {
        r *= sqrt(max(0.0, 1.0 - pow((v - 0.955) / 0.045, 2.0)));
    }
    let cover = clamp((r - abs(p.y)) / p.px + 0.5, 0.0, 1.0);
    if cover <= 0.0 {
        return vec4(0.0);
    }
    // A made-up room in the mirror: a dark floor, a bright sky, a hot highlight.
    let n = clamp(p.y / max(r, 0.0001), -1.0, 1.0);
    let warp = n + 0.25 * sin(v * 18.0 - t * 1.5 + n * 2.0);
    // The lab's mirror is lit by a bright room (its environment map, 2.6 times as
    // bright): a blue-grey floor, a pale sky.
    var env = mix(vec3(0.16, 0.18, 0.26), vec3(0.78, 0.84, 0.98), lab_sstep(-0.8, 0.9, warp));
    env += vec3(1.0) * exp(-pow((warp + 0.45) / 0.1, 2.0)) * 1.3;
    env += vec3(0.6, 0.7, 0.9) * pow(abs(n), 6.0) * 0.6;
    env *= vec3(0.95, 0.96, 1.0) * (0.75 + 0.4 * light);
    return vec4(min(env, vec3(1.0)), cover);
}

fn lab_quicksilver_drops(p: LabPlace) -> vec4<f32> {
    var out = vec4(0.0);
    for (var b = 0; b < 3; b++) {
        let fb = f32(b);
        let ph = f32(b) * 0.33 + p.t * 0.16;
        // Seconds since this bead reached the tip.
        let since = fract(ph) / 0.16;
        let drop = floor(ph);
        if since < 1.4 {
            let tip = vec2(p.shaft, 0.0);
            let v0 = vec2(p.up.y, -p.up.x) * (lab_rand(fb, drop, 0.0, 7.0) - 0.5) * 0.3
                + p.up * mix(-0.1, 0.2, lab_rand(fb, drop, 1.0, 7.0));
            let at = tip + v0 * since - p.up * 1.6 * since * since;
            let s = lab_sprite(p, at, 0.03);
            var c = lab_sprite_cover(6u, s, vec3(0.82, 0.86, 0.95), 0.0);
            c.a *= 1.0 - pow(since / 1.4, 2.0);
            out = lab_over(out, c);
        }
    }
    return out;
}

// The obsidian blade: flat black glass, its point toward the tip, glossy where the room
// shows in it.
fn lab_obsidian_body(p: LabPlace) -> vec4<f32> {
    let q = vec2(p.y, p.x / max(p.big_l, 0.0001));
    // Coverage from the edge's distance, a pixel soft.
    let e = lab_obsidian_edge(p);
    let inside = lab_obsidian_inside(q);
    let cover = select(clamp(0.5 - e / p.px, 0.0, 1.0), clamp(0.5 + e / p.px, 0.0, 1.0), inside);
    if cover <= 0.0 {
        return vec4(0.0);
    }
    let gloss = exp(-pow((q.x / LAB_OBSIDIAN_HALF + 0.3 - 0.4 * sin(p.t * 0.3 + q.y * 2.0)) / 0.18, 2.0));
    let bevel = exp(-e / 0.0025);
    let c = vec3(0.02, 0.02, 0.028) + vec3(0.3, 0.32, 0.38) * gloss * 0.6 + vec3(0.45, 0.48, 0.55) * bevel * 0.5;
    return vec4(c, cover);
}

// The bleeding blade's drops falling from below the corruption's line.
fn lab_bleeding_drops(p: LabPlace) -> vec4<f32> {
    var out = vec4(0.0);
    let level = min(0.22 + 0.48 * (0.5 + 0.5 * sin(p.t * 0.4)) + p.clash * 0.35, 1.0);
    let more = p.clash;
    for (var i = 0; i < 6; i++) {
        let fi = f32(i);
        let c = lab_cycle(fi, 1.1, p.t, 10.0);
        // Two drops a second, many more on a clash.
        if lab_rand(fi, c.y, 9.0, 10.0) < 0.33 + more {
            let at = lab_on_blade(p, mix(0.03, level, lab_rand(fi, c.y, 0.0, 10.0)), 0.018, lab_rand(fi, c.y, 1.0, 10.0) * TAU)
                - p.up * (0.05 * c.x + 1.1 * c.x * c.x);
            var d = lab_sprite_cover(6u, lab_sprite(p, at, 0.022), vec3(0.6, 0.02, 0.04), 0.0);
            d.a *= 1.0 - pow(c.x / 1.1, 2.0);
            out = lab_over(out, d);
        }
    }
    return out;
}

// The monarchs (Felucia's spores): seven sitting on the blade slowly opening their wings; a
// swing lifts them off it, and once it is still again they come back and land.
fn lab_butterflies(p: LabPlace) -> vec4<f32> {
    var out = vec4(0.0);
    // How far off they are: they lift off gently over the swing's first second rather than
    // jumping at once (Creyon's round 14: "move in a flow of the blade"), drifting home after
    // two seconds still.
    let rise = lab_sstep(0.0, 0.9, p.since_swing);
    let away = rise * select(1.0 - lab_sstep(2.2, 3.6, p.since_swing), 0.0, p.since_swing > 3.6);
    for (var i = 0; i < 7; i++) {
        let fi = f32(i);
        let u = 0.18 + fi * 0.11 + (lab_hash(fi * 3.7) - 0.5) * 0.06;
        let angle = lab_hash(fi * 5.1) * TAU;
        let ph = lab_hash(fi * 9.3) * 6.0;
        let perch = lab_on_blade(p, u, 0.028, angle);
        // Off the blade they keep with it, swaying in a slow stream along it and close by.
        let along = lab_on_blade(p, clamp(u + 0.05 * sin(p.t * 0.9 + ph), 0.1, 0.95), 0.028, angle);
        let flight = along
            + vec2(p.up.y, -p.up.x) * (sin(p.t * 1.6 + ph) * 0.07 + select(-0.06, 0.06, lab_hash(fi * 2.1) < 0.5))
            + p.up * (0.04 + 0.03 * sin(p.t * 1.2 + ph * 2.0));
        let at = mix(perch, flight, lab_sstep(0.0, 1.0, away));
        let open = select(0.35 + 0.3 * abs(sin(p.t * 1.4 + ph)), 0.15 + 0.85 * abs(sin(p.t * 24.0 + ph)), away > 0.05);
        let s = lab_sprite(p, at, 0.075);
        out = lab_over(out, lab_sprite_cover(3u, s, vec3(1.0, mix(0.42, 0.55, lab_hash(fi * 6.6)), 0.05), open));
    }
    return out;
}

// Round 20 (Creyon: the Clone Wars blade itself, its smoke and its stars were flat; the words
// stay as they are): the blade's toon body, the puffs and the spark stars in three dimensions,
// along the true view ray in the hilt's frame (`sig_ray`), in units.

// The ray's nearest approach to the blade's axis from 0 to `len`: (distance, along the axis,
// along the ray).
fn comic_axis(ray: SigRay, len: f32) -> vec3<f32> {
    let a = dot(ray.d.xz, ray.d.xz);
    var t = 0.0;
    if a > 0.000001 {
        t = max(-dot(ray.o.xz, ray.d.xz) / a, 0.0);
    }
    let y = clamp(ray.o.y + ray.d.y * t, 0.0, len);
    // the ray's nearest point to that place on the axis
    let c = vec3(0.0, y, 0.0);
    let tc = max(dot(c - ray.o, ray.d), 0.0);
    return vec3(length(ray.o + ray.d * tc - c), y, tc);
}

// The toon body: a capsule round the axis in its black outline, cel-shaded by its true normal,
// its clone markings and holo flicker on its surface.
fn comic_body3(p: LabPlace, ray: SigRay, len: f32, pixel: f32, color: vec3<f32>) -> vec4<f32> {
    let outer = 0.82;
    let inner = 0.52;
    let near = comic_axis(ray, len);
    let soft = max(pixel, 0.0001);
    let outline = clamp((outer - near.x) / soft + 0.5, 0.0, 1.0) * step(-0.2, near.y);
    if outline <= 0.0 {
        return vec4(0.0);
    }
    let fill = clamp((inner - near.x) / soft + 0.5, 0.0, 1.0);
    // the surface's normal where the ray meets the inner capsule
    let q = ray.o + ray.d * near.z;
    var radial = q - vec3(0.0, near.y, 0.0);
    let into = sqrt(max(inner * inner - near.x * near.x, 0.0));
    let n = normalize(radial - ray.d * into + vec3(0.0, 0.0001, 0.0));
    let light = normalize(vec3(-0.5, 0.6, 0.6) - ray.d * 0.8);
    let lit = select(0.72, 1.0, dot(n, light) > 0.1);
    var toon = min((color * 0.6 + 0.3) * lit + (color * 0.7 + 0.15), vec3(1.0));
    // clone markings over its surface, sliding up it
    if (p.off & OFF_SCAN) == 0u {
        let around = atan2(n.x, n.z) * inner * LAB_METRE;
        let x = near.y * LAB_METRE;
        let u = (x + around * 0.9 - p.t * 0.05) / 0.2;
        let band = fract(u);
        let blue = fract(floor(u) * 0.5) > 0.25;
        let paint = select(vec3(1.0, 0.52, 0.08), vec3(0.2, 0.42, 1.0), blue) * lit;
        let w = max(p.px / 0.2, 0.004);
        let stripe = lab_sstep(0.0, w, band) * (1.0 - lab_sstep(0.16, 0.16 + w, band));
        let pin = (lab_sstep(0.2, 0.2 + w, band) * (1.0 - lab_sstep(0.23, 0.23 + w, band)))
            + (lab_sstep(0.27, 0.27 + w, band) * (1.0 - lab_sstep(0.3, 0.3 + w, band)));
        let on = lab_sstep(1.1, 2.2, near.y) * (1.0 - lab_sstep(len - 2.2, len - 1.1, near.y));
        toon = mix(toon, paint, stripe * on);
        toon = mix(toon, vec3(1.0) * lit, pin * on);
    }
    var rgb = mix(vec3(0.0), toon, fill);
    var alpha = outline;
    let holo = lab_comic_holo(p);
    if holo > 0.0 {
        let scan = 0.6 + 0.4 * step(0.5, fract(near.y * LAB_METRE / 0.009 - p.t * 2.5));
        let blue = vec3(0.4, 0.8, 1.0) * scan;
        rgb = mix(rgb, mix(blue * 0.35, blue, fill), holo);
        alpha = mix(alpha, alpha * 0.55, holo);
    }
    return vec4(rgb, alpha);
}

// The cartoon smoke and spark stars in three dimensions: outlined puffs round the blade on a
// clash (and a few by the tip on a swing), drifting out and shrinking, and four-pointed stars
// popping up round it, each facing the camera where it is.
fn comic_puffs3(p: LabPlace, ray: SigRay, len: f32, pixel: f32) -> vec4<f32> {
    if (p.off & OFF_ARCS) != 0u || p.lingering {
        return vec4(0.0);
    }
    var out = vec4(0.0);
    let soft = max(pixel, 0.0001);
    let side = normalize(cross(ray.d, vec3(0.0, 1.0, 0.0)) + vec3(0.0001, 0.0, 0.0));
    let up = cross(side, ray.d);
    for (var i = 0; i < 7; i++) {
        let fi = f32(i);
        let clash = i < 4;
        let age = select(p.since_swing - 0.05, p.since_clash, clash);
        let life = select(0.9, 1.3, clash);
        if age < 0.0 || age > life {
            continue;
        }
        let born = floor((p.t - age) * 10.0);
        let k = age / life;
        let a = lab_hash(born * 1.3 + fi * 7.1);
        let from_y = select(len * 0.93, len * 0.6, clash);
        let reach = select(2.2 + 2.2 * lab_hash(fi * 3.3 + born), 3.3 + 3.0 * lab_hash(fi * 5.1 + born), clash);
        let r = 0.9 + reach * sqrt(k);
        let centre = vec3(r * sin(TAU * a), from_y + (lab_hash(fi + born * 3.0) - 0.5) * 3.0 + 1.8 * age,
            r * cos(TAU * a));
        let size = select(0.8, 1.25, clash) * (0.8 + 0.4 * lab_hash(fi + born * 2.0))
            * lab_sstep(0.0, 0.12, k) * (1.0 - 0.5 * k);
        let tc = max(dot(centre - ray.o, ray.d), 0.0);
        let off = ray.o + ray.d * tc - centre;
        let d = length(off) - size;
        let ink = clamp((0.15 - d) / soft + 0.5, 0.0, 1.0) * (1.0 - ray_behind_blade(ray, tc, len));
        if ink <= 0.0 {
            continue;
        }
        let fill = clamp(-d / soft + 0.5, 0.0, 1.0);
        let shade = select(0.86, 0.68, dot(off, up) < -size * 0.3);
        let rgb = mix(vec3(0.0), vec3(0.93, 0.94, 0.98) * shade, fill);
        out = lab_over(out, vec4(rgb, ink * (1.0 - lab_sstep(0.75, 1.0, k))));
    }
    for (var i = 0; i < 4; i++) {
        let fi = f32(i);
        let cyc = p.t * 0.9 + fi * 0.25;
        let born = floor(cyc);
        let k = fract(cyc);
        let y = (0.15 + 0.78 * lab_hash(born * 3.1 + fi * 1.7)) * len;
        let a = lab_hash(born * 5.3 + fi * 2.9);
        let r = 2.0 + 3.5 * lab_hash(born * 7.7 + fi);
        let centre = vec3(r * sin(TAU * a), y, r * cos(TAU * a));
        let size = 0.8 * (0.7 + 0.6 * lab_hash(born + fi * 9.0)) * sin(3.1416 * k);
        let tc = max(dot(centre - ray.o, ray.d), 0.0);
        let off = ray.o + ray.d * tc - centre;
        let q = abs(vec2(dot(off, side), dot(off, up))) / max(size, 0.0001);
        let star = sqrt(q.x) + sqrt(q.y) - 1.0;
        let edge = soft / max(size, 0.0001);
        let ink = clamp((0.25 - star) / max(edge * 2.0, 0.02) + 0.5, 0.0, 1.0)
            * (1.0 - ray_behind_blade(ray, tc, len));
        if ink <= 0.0 {
            continue;
        }
        let fill = clamp(-star / max(edge * 2.0, 0.02) + 0.5, 0.0, 1.0);
        out = lab_over(out, vec4(mix(vec3(0.0), vec3(1.0, 0.86, 0.2), fill), ink));
    }
    return out;
}

// A design's solid pass: its solid blade and its blended particles, colour and coverage.
fn lab_solid(input: VertexOutput, pixel: f32) -> vec4<f32> {
    let n = lab_design(input);
    let p = lab_place(input, pixel);
    var color = vec3(0.2, 0.7, 1.0);
    if input.chroma != 0.0 {
        color = skin_turn_hue(color, input.chroma);
    }
    var out = vec4(0.0);
    switch n {
        case 6u: {
            let ray = sig_ray(input);
            let len = input.length;
            out = lab_comic_panel(p, color);
            if !p.lingering {
                // the action bursts on the blade, away from the words
                let swung = p.since_swing - 0.04;
                let born = floor((p.t - swung) * 10.0);
                let side = select(-1.0, 1.0, lab_hash(born + 5.0) < 0.5);
                out = lab_over(out, lab_comic_burst(p, vec2(0.5 * p.shaft, -0.03 * side), swung, color, born, 0.16));
                out = lab_over(out, lab_comic_burst(p, vec2(0.42 * p.shaft, 0.0), p.since_clash, color, floor((p.t - p.since_clash) * 10.0) + 0.5, 0.2));
            }
            if !p.lingering {
                out = lab_over(out, comic_body3(p, ray, len, pixel, color));
            }
            out = lab_over(out, comic_puffs3(p, ray, len, pixel));
            out = lab_over(out, lab_comic_words(p, color));
        }
        case 7u: {
            out = lab_quicksilver_body(p, persona_light(input).g);
            out = lab_over(out, lab_quicksilver_drops(p));
        }
        case 9u: { out = lab_obsidian_body(p); }
        case 10u: { out = lab_bleeding_drops(p); }
        case 11u: { out = lab_butterflies(p); }
        default: {}
    }
    out.a *= persona_fade(input);
    return out;
}

// --- Blade Lab signature shaders (blade_signatures.rs) ----------------------------------
//
// A signature is the lab's own GLSL shader for one blade, translated to WGSL by
// scripts/blade_signatures.py: `sig_glow` takes the glow the skin's file drew (`g`) and
// returns the blade's light, `sig_core` the same for the core line, `sig_solid` a layer
// blended over the glow. `b` is the place on the blade in units (across, along: 0 at the
// hilt, `uLength` at the tip), `r` the stock glow's radius there, `px` a pixel in units.
// They read the lab's uniforms, set from the instance by `sig_take`. This module holds
// stand-ins that change nothing; each signature skin's pipelines are built from this
// module with the region between the markers replaced by the lab's helpers and its own
// functions (blade_signatures.rs `module_source`).

var<private> uTime: f32;
// Real time in seconds: a blade's cycles and moments keep their periods whatever its `@speed`.
var<private> uClock: f32;
var<private> uLength: f32;
var<private> uSwing: f32;
var<private> uClash: f32;
var<private> uIgn: f32;
var<private> uAge: f32;
// Seconds since the last swing began (Creyon's round 12: Felucia's spores stay off a while).
var<private> uSince: f32;
var<private> uSeed: f32;
var<private> uChroma: f32;
var<private> uRoom: f32;
// How far (units) from the blade's axis a signature's effects show in full: where its quad
// begins to fade them out (`sig_edge`), so the lab's pieces fade before it (`roomFade`).
var<private> uReach: f32;
// Whether this signature is a mythical blade's (Creyon's round 19: the mythicals must feel
// mythical): `Signature::module_source` sets it in a mythical's own module.
const SIG_MYTHICAL: bool = false;
// How fast a signature's own time runs (`@speed`, Signature::speed): 1 is real time.
const SIG_SPEED: f32 = 1.0;
// A round blade's effects keep their own colours under a chroma; only its body and glow take the
// wearer's (update 28, Creyon: Zeffo in a colour took all its effects with it).
const SIG_OWN_EFFECTS: bool = false;
// A signature whose solid layer stands wide past the point and closes itself there (`@solidpast`,
// Signature::solid_past; Angel's cloud bank): once the blade is all out, its solid parts are not cut
// along a straight line just past the tip (Creyon, 5/11/2026: up close the clouds' top was flat).
const SIG_SOLID_PAST_TIP: bool = false;
// How far (units) a `@solidnear` solid layer's depth is pulled toward the camera (`@nearpull`,
// Signature::near_pull): 14 keeps a wide plane from slicing the wearer or a wall; Angel's cloud bank
// pulls less, or it covered its wearer's head and face (Creyon, 5/11/2026).
const SIG_NEAR_PULL: f32 = 14.0;
// 1 while its wearer has its effects on (the `effects` option, the arcs bit), else 0: what a
// signature's own volume (`sig_vol`) keeps of everything but the blade itself.
var<private> uFx: f32;
// What a signature's solid layer sets where it keeps its own colours under a chroma (the
// Abyssal Eye's amber eye, Creyon's round 9): 0 takes the wearer's hue, 1 keeps its own.
var<private> uOwn: f32;
// Light a signature adds in its own colours whatever its wearer's colour (Creyon's round 12:
// Between Worlds' white and red duel stays white and red), added after the chroma's turn.
var<private> uKeep: vec3<f32>;
// Light a round blade's effect wants turned with the blade under a chroma, unlike its other effects
// (update 28: Byss's lightning).
var<private> uTurnFx: vec3<f32>;
// How much of what is behind them the effects in the air hide (a cloud, not light): `fragment_volume`.
var<private> uAirCover: f32;
// Seconds since its last clash, for what lasts longer than `uClash`.
var<private> uClashT: f32;
// A chroma's turn, and how far the blade's side in the picture lies toward the wearer's left (-1 to 1):
// one value for the whole blade, from where the camera stands, passing through 0 seen edge on.
var<private> uTurn: f32;
var<private> uBend: f32;
var<private> uSide: f32;
// Which of a signature's own parts its wearer left on (round 11: every effect can be
// switched off): bit k set when part k (`// @part` line k) is off, read by `part(k)`.
var<private> uParts: f32;
// The section bits a signature's parts ride on, part 0 to 5 (sections it never uses).
const SIG_PART_BITS: array<u32, 6> = array<u32, 6>(OFF_SPUTTER, OFF_GLITCH, OFF_SCAN, OFF_BANDS,
    OFF_SEGMENTS, OFF_FILM);

// The lab's ignition takes 0.42 s and its clash fades at 2.2 a second.
const SIG_IGNITION: f32 = 0.42;

fn sig_take(input: VertexOutput) {
    uTime = input.animation.x * SIG_SPEED;
    uClock = input.animation.x;
    uSeed = input.animation.y;
    // The full length when it is known (round 11): everything a signature places along the
    // blade stays where it is while the blade grows or shrinks.
    uLength = max(input.length, input.axis.w);
    // Only a real swing counts (Creyon's round 12: effects stirred while he ran or turned):
    // the arm's sway while running and the torso following the view stay under this.
    uSwing = smoothstep(0.45, 1.0, clamp(input.motion.x, 0.0, 1.0));
    uClash = max(1.0 - input.motion.y * 2.2, 0.0);
    uClashT = input.motion.y;
    uAge = input.motion.z;
    uSince = input.motion.w;
    uIgn = clamp(input.motion.z / SIG_IGNITION, 0.0, 1.0);
    // The options every signature has (blade_signatures.rs `Signature::shown`), on bits
    // a signature skin has no other use for: no swing and clash bursts (the pulse bit),
    // and its own parts (`SIG_PART_BITS`). Sol's own sections (arcs, motes, wisps, ...)
    // switch themselves.
    if (input.off.x & OFF_PULSE) != 0u {
        uSwing = 0.0;
        uClash = 0.0;
        uClashT = 1000.0;
        uSince = 1000.0;
    }
    var parts = 0u;
    for (var k = 0u; k < 6u; k++) {
        if (input.off.x & SIG_PART_BITS[k]) != 0u {
            parts |= 1u << k;
        }
    }
    uParts = f32(parts);
    uFx = 1.0;
    uOwn = 0.0;
    uTurnFx = vec3(0.0);
    uAirCover = 0.0;
    uTurn = input.chroma;
    {
        let ay = normalize(input.axis.xyz);
        var az = oct_unpack(input.down.zw);
        az -= ay * dot(az, ay);
        let n = cross(input.hilt_at, ay);
        uBend = clamp(dot(n, cross(ay, az)) / max(length(n) * max(length(az), 0.0001), 0.000001) * 2.5, -1.0, 1.0);
        // The same across the hilt's other axis: a curve set on this stays on one side of the wearer from every side.
        uSide = clamp(-dot(n, az) / max(length(n) * max(length(az), 0.0001), 0.000001) * 2.5, -1.0, 1.0);
    }
    uKeep = vec3(0.0);
    // The page turned a chroma's output afterwards; the game turns it after the hooks.
    uChroma = 0.0;
    uRoom = skins[skin_index(input)].design.z / LAB_METRE;
    let skin = skins[skin_index(input)];
    let room = max(skin_room(skin, input.off.x), max(skin.design.y, skin.design.z) / LAB_METRE);
    uReach = 0.6 * (max(chain_radius(input.radius, input.length, 0.0) * skin.swell.x, input.hilt) + room);
}

// What a signature keeps of what it draws at `b` (once: only the blade itself with its
// `effects` option off; since round 11 every part switches on its own).
fn sig_kept(input: VertexOutput, b: vec2<f32>) -> f32 {
    // Round 11: each effect switches on its own (`part`), the blade itself never.
    return 1.0;
}

// A chroma signature's colour on its wearer: every coloured part takes the wearer's hue
// outright (`hue`, turns from red), at least well saturated, and only its white-hot parts
// stay white, as a stock blade's core does; turning the colours round (`skin_turn_hue`)
// left the lab's many-coloured and pale blades looking hardly changed.
fn sig_recolour(c: vec3<f32>, hue: f32) -> vec3<f32> {
    let high = max(c.r, max(c.g, c.b));
    if high <= 0.0 {
        return c;
    }
    let low = min(c.r, min(c.g, c.b));
    let sat = (high - low) / high;
    let turned = skin_turn_hue(vec3(1.0, 0.0, 0.0), hue);
    let tint = turned / max(max(turned.r, max(turned.g, turned.b)), 0.0001);
    let white = smoothstep(0.7, 1.0, high) * (1.0 - smoothstep(0.08, 0.3, sat));
    return high * mix(vec3(1.0), tint, mix(max(sat, 0.85), sat, white));
}

// The colour a signature skin's pass ends with: its wearer's hue for a chroma.
fn sig_coloured(input: VertexOutput, c: vec3<f32>) -> vec3<f32> {
    let base = skins[skin_index(input)].design_color.w;
    if input.chroma == 0.0 || base < 0.0 {
        return c;
    }
    if SIG_OWN_EFFECTS {
        return sig_turned(c, base, fract(base + input.chroma), 0.0);
    }
    return sig_recolour(c, fract(base + input.chroma));
}

// The same for a round blade's air effects: a neighbour on the wheel, so they stand off the body.
fn sig_coloured_air(input: VertexOutput, c: vec3<f32>) -> vec3<f32> {
    let base = skins[skin_index(input)].design_color.w;
    if input.chroma == 0.0 || base < 0.0 {
        return c;
    }
    return sig_turned(c, base, fract(base + input.chroma), 0.13);
}

// A round blade under a chroma (Creyon, 3/11/2026: all one colour hid the effects, then a plain turn gave colours
// that fought): its main colour becomes the wearer's and its other colours stay near it, closer together than
// they were, so effects still stand apart in one family of colour. White stays white.
fn sig_turned(c: vec3<f32>, base: f32, hue: f32, accent: f32) -> vec3<f32> {
    let high = max(c.r, max(c.g, c.b));
    if high <= 0.0 {
        return c;
    }
    let low = min(c.r, min(c.g, c.b));
    let span = high - low;
    let sat = span / high;
    var h = 0.0;
    if span > 0.00001 {
        if high == c.r {
            h = fract(((c.g - c.b) / span) / 6.0);
        } else if high == c.g {
            h = ((c.b - c.r) / span + 2.0) / 6.0;
        } else {
            h = ((c.r - c.g) / span + 4.0) / 6.0;
        }
    }
    let away = fract(h - base + 0.5) - 0.5;
    // Greys and pale tones take the wearer's own hue; the white-hot keeps white
    let colour = smoothstep(0.03, 0.3, sat);
    // The wearer's colour is the body's exactly: 250 0 0 is red. What was another colour stays a close
    // neighbour of it; the air effects (`accent`) lean a little along the wheel, deeper in their dark tones.
    // Creyon, 6/11/2026: under a colour its effects still ran together, and colours opposite the main one
    // (Peridea's red and blue) folded onto it. Now the body is the wearer's colour; every other colour it was
    // designed with keeps its side of the wheel and stands a clear step off, more the further it was, up to a
    // triad (a third of the wheel), so none meets the body's; the air effects take an analogous colour a
    // sixth of the wheel off to their side. One family, every effect told apart.
    let side = select(-1.0, 1.0, away >= 0.0);
    let reach = abs(away);
    let apart = side * (0.1 + 0.55 * clamp(reach - 0.08, 0.0, 0.42)) * smoothstep(0.04, 0.12, reach);
    let depth = accent * (smoothstep(0.08, 0.85, high) - 0.5);
    var turned = fract(hue + (apart + depth) * colour);
    if accent > 0.0 {
        turned = fract(hue + side * (0.17 + 0.12 * clamp(reach - 0.1, 0.0, 0.4)) * colour + 0.04 * depth);
    }
    let k = vec3(1.0, 2.0 / 3.0, 1.0 / 3.0);
    let wheel = clamp(abs(fract(vec3(turned) + k) * 6.0 - vec3(3.0)) - vec3(1.0), vec3(0.0), vec3(1.0));
    let white = smoothstep(0.82, 1.0, high) * (1.0 - smoothstep(0.08, 0.3, sat));
    var keep = mix(max(sat, mix(0.94, 0.6, smoothstep(0.1, 0.3, abs(away)))), sat, white);
    if accent > 0.0 {
        keep = mix(max(sat, 0.55), sat, white);
    }
    return high * mix(vec3(1.0), wheel, keep);
}

// The view ray in the blade's own frame, units: `o` the camera, `d` the unit direction; y
// along the blade from its hilt, z along the direction across it fixed to the hilt
// (`frame`, Creyon's round 9: a frame turned toward the camera, as before, swung every
// effect round the blade whenever the view turned) and x across both, so a signature draws
// in three dimensions with its depth and its effects stay put as the camera moves. A blade
// without a hilt frame (thrown, the menu stage) takes a fixed world direction instead.
struct SigRay {
    o: vec3<f32>,
    d: vec3<f32>,
}

fn sig_ray(input: VertexOutput) -> SigRay {
    let ay = normalize(input.axis.xyz);
    var across = oct_unpack(input.down.zw);
    if input.axis.w <= 0.0 {
        across = select(vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0), abs(ay.z) > 0.9);
    }
    across -= ay * dot(across, ay);
    if dot(across, across) < 0.000001 {
        across = cross(ay, select(vec3(0.0, 0.0, 1.0), vec3(0.0, 1.0, 0.0), abs(ay.z) > 0.9));
    }
    let az = normalize(across);
    let ax = cross(ay, az);
    let cam = -input.hilt_at;
    let r = normalize(input.ray);
    var out: SigRay;
    out.o = vec3(dot(cam, ax), dot(cam, ay), dot(cam, az));
    out.d = vec3(dot(r, ax), dot(r, ay), dot(r, az));
    // A camera far off (the Collection's pictures look from 20 000 units away) starts its ray
    // near the blade instead, or the effects' sums lose their precision and smear across it
    // (Creyon's round 20: the pictures showed things the blades do not have); nothing is
    // drawn more than a hundred units before the blade.
    let ahead = dot(-out.o, out.d);
    out.o += out.d * max(ahead - 100.0, 0.0);
    return out;
}

// Before a signature's three-dimensional effects: they are placed on the blade's full
// length, not the length it has grown to, and show only once it is (nearly) all out, so
// nothing slides up the blade as it ignites or down as it is put away (Creyon's round 9).
// Returns how much of them shows.
fn sig_full(input: VertexOutput) -> f32 {
    let full = input.axis.w;
    if full <= input.length {
        return 1.0;
    }
    return smoothstep(0.85, 1.0, input.length / full);
}

// Sol's lightning and prominences (`arcs`) on a signature, held by the hilt (Creyon's round 12:
// drawn in the picture facing the camera, they kept their side of the screen whatever the view,
// so they swung round the blade with the camera when he turned or ran). They are drawn on two
// sheets through the blade's axis fixed to the hilt (`sig_ray`'s frame), each with its own
// strikes and seen from wherever the camera is, so they stay with the blade as everything held
// by the hilt does; a sheet seen edge on fades out and the other carries the picture.
fn sig_arcs(skin: Skin, input: VertexOutput, ray: SigRay, pixel: f32) -> vec3<f32> {
    let time = input.animation.x;
    let seed = input.animation.y;
    let r = chain_radius(input.radius, input.length, 0.5);
    let extent = skin_extent(skin, input);
    let faded = extent + max(skin.design.y, skin.design.z) / LAB_METRE;
    var total = 0.0;
    for (var sheet = 0; sheet < 2; sheet++) {
        let facing = select(abs(ray.d.z), abs(ray.d.x), sheet == 1);
        if facing < 0.05 {
            continue;
        }
        let t = select(-ray.o.z / ray.d.z, -ray.o.x / ray.d.x, sheet == 1);
        if t <= 0.0 {
            continue;
        }
        let p = ray.o + ray.d * t;
        let across = select(p.x, p.z, sheet == 1);
        let beyond = max(p.y - input.length, 0.0);
        var room = (1.0 - smoothstep(0.75, 1.0, abs(across) / faded))
            * (1.0 - smoothstep(0.75, 1.0, beyond / faded));
        // Round 19 (Creyon: the Eclipse's, Twin Suns' and Dyson Sphere's prominences crawled
        // up past the tip and were cut there): arcs that do not leap from the tip fade out
        // over the blade's last units, so none reaches past it.
        if skin.arc_place.w <= 0.0 {
            room *= 1.0 - smoothstep(input.length - 3.0, input.length - 0.4, p.y);
        }
        let arcs = skin_arcs(skin, p.y, across, r, input.length, extent, pixel / max(facing, 0.25),
            time, fract(seed + 0.37 * f32(sheet)));
        total += arcs * room * smoothstep(0.05, 0.35, facing);
    }
    return total * skin.arc_color.rgb * skin.arc_color.w;
}

// What every mythical blade has over its own effects, in its own light's colour (Creyon's round
// 19: "what can we do to make the mythical sabers feel more mythical"): it awakens when lit, a
// surge of light running up it from the hilt and bursting at the tip in a flare and a ring;
// it breathes, a slow wave of light rising up it every few seconds; and a clash rings out round
// it in a ring of its light (with its swing and clash bursts).
fn sig_mythic(input: VertexOutput, place: vec2<f32>, ray: SigRay, pixel: f32) -> vec3<f32> {
    let tone = skins[skin_index(input)].design_color.rgb;
    let c = tone / max(max(tone.r, max(tone.g, tone.b)), 0.001);
    let white = mix(c, vec3(1.0), 0.6);
    let len = uLength;
    let age = uAge;
    let near = exp(-pow(place.x / 1.8, 2.0)) * step(-0.5, place.y);
    var light = vec3(0.0);
    // the awakening: a surge up the blade, then a flare and a ring at the tip
    if age < 3.0 {
        let front = len * smoothstep(0.05, 1.1, age);
        let surge = exp(-pow((place.y - front) / 2.6, 2.0)) * (1.0 - smoothstep(1.05, 1.5, age))
            * smoothstep(0.05, 0.15, age);
        light += white * surge * near * 1.6;
        let burst = max(age - 1.1, 0.0);
        let flare = exp(-burst * 3.5) * step(1.1, age);
        let tip = length(vec2(place.x, place.y - len + 0.6));
        light += (white * exp(-pow(tip / 1.2, 2.0)) * 2.0 + c * exp(-tip / 3.5) * 0.6) * flare;
        // the ring, in three dimensions round the tip, out to well inside the room
        let ring = sig_ring_d(ray, len - 0.8, 0.6 + 0.8 * uReach * (1.0 - exp(-burst * 3.0)));
        light += (white * exp(-pow(ring / max(0.12, pixel), 2.0)) + c * exp(-pow(ring / 0.6, 2.0)) * 0.4)
            * flare * 1.2;
    }
    // its breath: a slow wave of light rising up it
    let beat = fract(uClock / 6.5 + uSeed * 0.37);
    let wave = beat * (len + 10.0) - 5.0;
    light += c * exp(-pow((place.y - wave) / 3.2, 2.0)) * near * 0.45 * smoothstep(1.4, 2.4, age);
    // a clash rings out round it (with its swing and clash bursts: `uClash`)
    let clash = clamp(uClash, 0.0, 1.0);
    if clash > 0.01 {
        let ring = sig_ring_d(ray, len * 0.55, 0.8 + 0.85 * uReach * (1.0 - clash * clash));
        light += (white * exp(-pow(ring / max(0.1, pixel), 2.0)) * 1.4 + c * exp(-pow(ring / 0.5, 2.0)) * 0.5)
            * clash;
    }
    return light;
}

// 1 along a signature's blade from just past the hilt's mouth, 0 behind it; 1 for other skins.
fn sig_from_mouth(skin: Skin, along: f32) -> f32 {
    return select(1.0, smoothstep(0.2, 1.0, along), skin.design.x == f32(LAB_SIGNATURE));
}

// How hidden by the blade's body a point `t` along `ray` is (0 seen, 1 hidden), the blade `len`
// units long (round 22, Creyon: nothing may be seen through the blade behind it).
// Round 23 (players' reports after the 2026.1021.1 release): it fades in and out softly instead
// of switching off when looking nearly along the blade, as `behindBlade` in the lab.
fn ray_behind_blade(ray: SigRay, t: f32, len: f32) -> f32 {
    let a = dot(ray.d.xz, ray.d.xz);
    let along_view = smoothstep(0.01, 0.04, a);
    if along_view <= 0.0 {
        return 0.0;
    }
    let ta = -dot(ray.o.xz, ray.d.xz) / a;
    let y = ray.o.y + ray.d.y * ta;
    let d = length(ray.o.xz + ray.d.xz * ta);
    let p = ray.o + ray.d * t;
    let outside = smoothstep(0.5, 0.9, length(p.xz));
    // Round 23: the body hiding narrows with the blade's rounded tip to its point, as
    // `behindBlade` in the lab: what passes behind the tip stays hidden, with no dark round end
    // past the tip seen end on.
    let into = clamp((y - (len - 1.4)) / 1.4, 0.0, 1.0);
    let w = sqrt(max(1.0 - into * into, 0.0));
    let on_blade = smoothstep(-0.8, -0.2, y) * (1.0 - smoothstep(len - 0.15, len + 0.05, y));
    return (1.0 - smoothstep(0.35 * w, 0.8 * w + 0.02, d)) * on_blade * outside
        * smoothstep(ta + 0.15, ta + 0.7, t) * along_view;
}

// A distance `d` as seen when `h` (`ray_behind_blade`) of it is hidden by the blade: the piece
// narrows and dims as it passes behind, as `hideD` in the lab (round 23: adding a thousand times
// `h` cut it at a hard edge along the blade's body, a box round the tip seen end on).
fn hide_d(d: f32, h: f32) -> f32 {
    return d / max(1.0 - h, 0.001) + h;
}

// The ray's distance to a ring of radius `radius` round the blade at `height` (the hilt's
// frame, `sig_ray`): the nearer of where it crosses the ring's plane and its cylinder.
fn sig_ring_d(ray: SigRay, height: f32, radius: f32) -> f32 {
    var best = 1000.0;
    if abs(ray.d.y) > 0.00001 {
        let t = (height - ray.o.y) / ray.d.y;
        if t > 0.0 {
            let h = ray.o + ray.d * t;
            best = hide_d(abs(length(h.xz) - radius), ray_behind_blade(ray, t, uLength));
        }
    }
    let a = dot(ray.d.xz, ray.d.xz);
    let b = dot(ray.o.xz, ray.d.xz);
    let c = dot(ray.o.xz, ray.o.xz) - radius * radius;
    let found = b * b - a * c;
    if found > 0.0 && a > 0.000001 {
        let q = sqrt(found);
        let t0 = (-b - q) / a;
        let t1 = (-b + q) / a;
        if t0 > 0.0 {
            best = min(best, hide_d(abs(ray.o.y + ray.d.y * t0 - height), ray_behind_blade(ray, t0, uLength)));
        }
        if t1 > 0.0 {
            best = min(best, hide_d(abs(ray.o.y + ray.d.y * t1 - height), ray_behind_blade(ray, t1, uLength)));
        }
    }
    return best;
}

// Fades a signature out before its quad's edges (the quad `blade_vertex` builds): nothing it
// draws ever stops at a straight edge.
fn sig_edge(input: VertexOutput) -> f32 {
    let skin = skins[skin_index(input)];
    let room = max(skin_room(skin, input.off.x), max(skin.design.y, skin.design.z) / LAB_METRE);
    let extent = max(chain_radius(input.radius, input.length, 0.0) * skin.swell.x, input.hilt) + room;
    let across = 1.0 - smoothstep(0.6 * extent, 0.95 * extent, abs(input.blade.x));
    let below = smoothstep(-0.95 * extent, -0.6 * extent, input.blade.y);
    let above = 1.0 - smoothstep(input.shaft + 0.6 * extent, input.shaft + 0.95 * extent, input.blade.y);
    return across * below * above;
}

// @signature begin
fn labRayPrepare(ro: vec3<f32>, rd: vec3<f32>) {
    // The lab keeps what of its blade hiding depends on the view ray alone (`lab.glsl`,
    // behindBlade); a module with no signature has nothing to keep.
}

fn sig_glow(g: vec3<f32>, b: vec2<f32>, r: f32, px: f32) -> vec3<f32> {
    return g;
}

fn sig_core(c: vec3<f32>, b: vec2<f32>, px: f32) -> vec3<f32> {
    return c;
}

fn sig_solid(o: vec4<f32>, b: vec2<f32>, px: f32) -> vec4<f32> {
    return o;
}

fn sig_vol(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec3<f32> {
    return vec3(0.0);
}

fn sig_vol_solid(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec4<f32> {
    return vec4(0.0);
}

fn sig_orbit(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec3<f32> {
    return vec3(0.0);
}

fn sig_orbit_solid(ro: vec3<f32>, rd: vec3<f32>, px: f32) -> vec4<f32> {
    return vec4(0.0);
}
// @signature end
