// Smoke ribbons from blade skins' emitters (`saber_ribbons.rs`): each vertex is a
// point of a strand pushed out to one side, facing the camera; the strand is soft
// across, its edges fading over `softness` of its half-width, and torn along by noise
// that rides with its points and turns as they age.

struct Camera {
    view_projection: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    // -1 to 1 across the strand, and the noise's place along it and in time.
    @location(1) across: f32,
    @location(2) along: f32,
    @location(3) age: f32,
    @location(4) @interpolate(flat) detail: f32,
    @location(5) @interpolate(flat) softness: f32,
}

@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) side: f32,
    @location(2) tangent: vec3<f32>,
    @location(3) along: f32,
    @location(4) color: vec4<f32>,
    @location(5) look: vec4<f32>,
) -> VertexOutput {
    // Widened across the strand's way, facing the camera; a strand pointing at the
    // camera keeps its last good side.
    let to_camera = camera.position - position;
    var across = cross(tangent, to_camera);
    if dot(across, across) < 1e-8 {
        across = cross(vec3(0.0, 0.0, 1.0), to_camera);
    }
    let world = position + normalize(across) * side;
    var output: VertexOutput;
    output.clip_position = camera.view_projection * vec4(world, 1.0);
    output.color = color;
    output.across = sign(side);
    output.along = along;
    output.age = look.z;
    output.detail = look.x;
    output.softness = look.y;
    return output;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let cell = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(cell), hash(cell + vec2(1.0, 0.0)), u.x),
        mix(hash(cell + vec2(0.0, 1.0)), hash(cell + vec2(1.0, 1.0)), u.x),
        u.y,
    );
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let edge = 1.0 - abs(input.across);
    let soft = smoothstep(0.0, max(input.softness, 0.01), edge);
    // Torn along by two octaves of noise, across a little too, turning with age.
    let p = vec2(input.along * input.detail, input.across * 1.5 + input.age * 0.6);
    let torn = noise(p) * 0.65 + noise(p * 2.3 + vec2(7.1, 3.3)) * 0.35;
    let tearing = clamp(input.detail, 0.0, 1.0);
    let body = mix(1.0, smoothstep(0.25, 0.85, torn), tearing);
    let alpha = input.color.a * soft * body;
    if alpha <= 0.002 {
        discard;
    }
    return vec4(input.color.rgb, alpha);
}
