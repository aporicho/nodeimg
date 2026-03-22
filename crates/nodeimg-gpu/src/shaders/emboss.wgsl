@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    strength: f32,
    angle_rad: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

fn sample_clamped(coord: vec2<i32>, dims: vec2<u32>) -> vec4<f32> {
    let cx = clamp(coord.x, 0, i32(dims.x) - 1);
    let cy = clamp(coord.y, 0, i32(dims.y) - 1);
    return textureLoad(input, vec2<i32>(cx, cy));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let pos = vec2<i32>(id.xy);
    let s = params.strength;

    // Emboss kernel (top-left to bottom-right):
    // [-2s, -s,  0 ]
    // [-s,   1,  s ]
    // [ 0,   s,  2s]
    // Matches the CPU implementation exactly.

    let tl = sample_clamped(pos + vec2<i32>(-1, -1), dims);
    let tc = sample_clamped(pos + vec2<i32>( 0, -1), dims);
    let tr = sample_clamped(pos + vec2<i32>( 1, -1), dims);
    let ml = sample_clamped(pos + vec2<i32>(-1,  0), dims);
    let mc = sample_clamped(pos + vec2<i32>( 0,  0), dims);
    let mr = sample_clamped(pos + vec2<i32>( 1,  0), dims);
    let bl = sample_clamped(pos + vec2<i32>(-1,  1), dims);
    let bc = sample_clamped(pos + vec2<i32>( 0,  1), dims);
    let br = sample_clamped(pos + vec2<i32>( 1,  1), dims);

    // Apply kernel to RGB channels; the CPU code uses 0–255 range + 128 offset.
    // In GPU (0–1 range), the offset is 128/255 ≈ 0.502.
    let acc = tl * (-2.0 * s) + tc * (-s)
            + ml * (-s)       + mc * 1.0   + mr * s
                              + bc * s     + br * (2.0 * s);

    let offset = 128.0 / 255.0;
    let orig_alpha = mc.a;

    let result = vec4<f32>(
        clamp(acc.r + offset, 0.0, 1.0),
        clamp(acc.g + offset, 0.0, 1.0),
        clamp(acc.b + offset, 0.0, 1.0),
        orig_alpha
    );

    textureStore(output, pos, result);
}
