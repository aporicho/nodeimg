@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    amount: f32,
    size: f32,
    seed: f32,
    _pad: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

// Hash function for pseudo-random noise
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));

    // Scale coordinates by grain size
    let inv_size = 1.0 / max(params.size, 0.5);
    let coord = vec2<f32>(f32(id.x) * inv_size, f32(id.y) * inv_size);

    // Generate noise with seed
    let noise = hash(coord + vec2<f32>(params.seed, params.seed * 1.7)) * 2.0 - 1.0;
    let grain = noise * params.amount;

    let result = vec4<f32>(
        clamp(color.r + grain, 0.0, 1.0),
        clamp(color.g + grain, 0.0, 1.0),
        clamp(color.b + grain, 0.0, 1.0),
        color.a,
    );
    textureStore(output, vec2<i32>(id.xy), result);
}
