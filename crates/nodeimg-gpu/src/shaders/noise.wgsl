@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    seed: f32,
    scale: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

// Hash-based pseudo-random gradient generation (Perlin noise approximation)
fn hash2(p: vec2<f32>) -> vec2<f32> {
    var q = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3))
    );
    return -1.0 + 2.0 * fract(sin(q) * 43758.5453123);
}

fn perlin(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);

    // Quintic interpolation curve
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);

    let g00 = dot(hash2(i + vec2<f32>(0.0, 0.0)), f - vec2<f32>(0.0, 0.0));
    let g10 = dot(hash2(i + vec2<f32>(1.0, 0.0)), f - vec2<f32>(1.0, 0.0));
    let g01 = dot(hash2(i + vec2<f32>(0.0, 1.0)), f - vec2<f32>(0.0, 1.0));
    let g11 = dot(hash2(i + vec2<f32>(1.0, 1.0)), f - vec2<f32>(1.0, 1.0));

    let x0 = mix(g00, g10, u.x);
    let x1 = mix(g01, g11, u.x);

    return mix(x0, x1, u.y);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let uv = vec2<f32>(f32(id.x), f32(id.y));
    let frequency = params.scale * 0.02;
    let p = uv * frequency + vec2<f32>(params.seed * 17.0, params.seed * 31.0);

    // Multi-octave noise for more natural appearance
    var value = 0.0;
    value += perlin(p) * 0.5;
    value += perlin(p * 2.0) * 0.25;
    value += perlin(p * 4.0) * 0.125;
    value += perlin(p * 8.0) * 0.0625;

    // Normalize from [-1,1] to [0,1]
    value = value * 0.5 + 0.5;
    value = clamp(value, 0.0, 1.0);

    let color = vec4<f32>(value, value, value, 1.0);
    textureStore(output, vec2<i32>(id.xy), color);
}
