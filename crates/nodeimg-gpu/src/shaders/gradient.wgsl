@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    color_start: vec4<f32>,
    color_end: vec4<f32>,
    // direction: 0 = horizontal, 1 = vertical, 2 = diagonal
    direction: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    // Reference input to prevent binding elimination by WGSL compiler
    _ = textureLoad(input, vec2<i32>(0, 0));

    let fx = f32(id.x) / f32(max(dims.x - 1u, 1u));
    let fy = f32(id.y) / f32(max(dims.y - 1u, 1u));

    var t: f32;
    let dir = i32(params.direction);
    if (dir == 1) {
        // vertical
        t = fy;
    } else if (dir == 2) {
        // diagonal
        t = (fx + fy) * 0.5;
    } else {
        // horizontal (default)
        t = fx;
    }

    let color = mix(params.color_start, params.color_end, t);
    textureStore(output, vec2<i32>(id.xy), color);
}
