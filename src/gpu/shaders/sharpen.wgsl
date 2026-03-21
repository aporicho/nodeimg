@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    amount: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

fn sample_clamped(coord: vec2<i32>, dims: vec2<u32>) -> vec4<f32> {
    let c = vec2<i32>(clamp(coord.x, 0, i32(dims.x) - 1), clamp(coord.y, 0, i32(dims.y) - 1));
    return textureLoad(input, c);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let pos = vec2<i32>(id.xy);

    // 3x3 sharpening kernel (unsharp mask style)
    // Center = 1 + 4*amount, neighbors = -amount
    let center = sample_clamped(pos, dims);
    let top    = sample_clamped(pos + vec2<i32>( 0, -1), dims);
    let bottom = sample_clamped(pos + vec2<i32>( 0,  1), dims);
    let left   = sample_clamped(pos + vec2<i32>(-1,  0), dims);
    let right  = sample_clamped(pos + vec2<i32>( 1,  0), dims);

    let a = params.amount;
    let sharpened = center * (1.0 + 4.0 * a) - (top + bottom + left + right) * a;

    let result = vec4<f32>(
        clamp(sharpened.r, 0.0, 1.0),
        clamp(sharpened.g, 0.0, 1.0),
        clamp(sharpened.b, 0.0, 1.0),
        center.a,
    );
    textureStore(output, vec2<i32>(id.xy), result);
}
