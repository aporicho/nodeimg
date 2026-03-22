@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    color_a: vec4<f32>,
    color_b: vec4<f32>,
    size: f32,
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

    let cell_size = u32(max(params.size, 1.0));
    let cx = id.x / cell_size;
    let cy = id.y / cell_size;
    let is_even = ((cx + cy) % 2u) == 0u;

    var color: vec4<f32>;
    if (is_even) {
        color = params.color_a;
    } else {
        color = params.color_b;
    }

    textureStore(output, vec2<i32>(id.xy), color);
}
