@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    radius: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let r = i32(clamp(params.radius, 0.0, 63.0));
    if (r == 0) {
        let c = textureLoad(input, vec2<i32>(id.xy));
        textureStore(output, vec2<i32>(id.xy), c);
        return;
    }

    let sigma = f32(r) / 3.0;
    let two_sigma_sq = 2.0 * sigma * sigma;

    var sum = vec4<f32>(0.0);
    var weight_sum: f32 = 0.0;

    for (var i: i32 = -r; i <= r; i = i + 1) {
        let sx = i32(id.x);
        let sy = clamp(i32(id.y) + i, 0, i32(dims.y) - 1);
        let w = exp(-f32(i * i) / two_sigma_sq);
        sum = sum + textureLoad(input, vec2<i32>(sx, sy)) * w;
        weight_sum = weight_sum + w;
    }

    let result = sum / weight_sum;
    textureStore(output, vec2<i32>(id.xy), result);
}
