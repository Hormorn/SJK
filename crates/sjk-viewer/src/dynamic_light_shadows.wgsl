// Point-light shadow tiles (`dynamic_light_shadows.rs`): every texel of every light's tile,
// traced against the static triangles the lamps' visibility uses (`lamp_geometry.wgsl`,
// group 0). Glass, grates and sky let light through, as for the lamps.
struct TracedLight { origin_radius: vec4<f32>, color: vec4<f32> };
// The head of the scene's point-light block (`point_lights.wgsl`): the lights and their count.
struct TracedLights { lights: array<TracedLight, 32>, metadata: vec4<u32> };
@group(1) @binding(0) var<uniform> traced: TracedLights;
// The header (lights with a tile), then the tiles, two texels to a word: copied whole into
// the block after its CPU part.
@group(1) @binding(1) var<storage, read_write> shadow_tiles: array<u32>;

// One texel: the 255th of the reach where the ray stops, rounded down (the receivers round
// up, so a stored hit is never nearer than the real one), and what it stopped on.
fn point_light_trace_texel(source: vec4<f32>, reach: f32, texel: u32) -> u32 {
    let pixel = vec2<i32>(vec2(texel % POINT_TILE_EDGE, texel / POINT_TILE_EDGE));
    let direction = point_light_texel_direction(pixel);
    let hit = gi_trace_through(source.xyz + direction*POINT_TILE_START, direction,
        reach - POINT_TILE_START, true);
    if !hit.hit { return 255u; }
    let level = min(u32(max((hit.distance + POINT_TILE_START)/reach, 0.0)*255.0), 254u);
    return level | (point_light_normal_code(hit.normal) << 8u);
}

var<workgroup> traced_texels: array<u32, 64>;

// One texel of light `id.y`'s tile per invocation, then even lanes pack their pair into a
// word (a tile's 784 texels and a group's 64 lanes are even, so a pair never straddles two
// groups). The trace is bound by a ray's latency rather than by how many rays there are, so
// one ray each rather than two in a row takes about 40 % off (0.38 -> 0.23 ms at 4K).
@compute @workgroup_size(64) fn trace(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(local_invocation_index) lane: u32,
) {
    let count = min(traced.metadata.x, 32u);
    if all(id == vec3(0u)) {
        shadow_tiles[0] = count;
        shadow_tiles[1] = 0u;
        shadow_tiles[2] = 0u;
        shadow_tiles[3] = 0u;
    }
    let texel = id.x;
    // One light per row of groups: `live` is uniform in a group, so all of it reaches the
    // barrier or none does.
    let light = id.y;
    let live = light < count && texel < POINT_TILE_TEXELS;
    var value = 255u;
    if live {
        let source = traced.lights[light].origin_radius;
        value = point_light_trace_texel(source, point_light_reach(source.w), texel);
    }
    traced_texels[lane] = value;
    workgroupBarrier();
    if live && (lane & 1u) == 0u {
        shadow_tiles[4u + light*(POINT_TILE_TEXELS/2u) + texel/2u] =
            traced_texels[lane] | (traced_texels[lane + 1u] << 16u);
    }
}
