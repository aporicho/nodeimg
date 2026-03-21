@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    offset_x: f32,
    offset_y: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let out_dims = textureDimensions(output);
    if (id.x >= out_dims.x || id.y >= out_dims.y) {
        return;
    }

    let src_x = i32(id.x) + i32(params.offset_x);
    let src_y = i32(id.y) + i32(params.offset_y);

    let color = textureLoad(input, vec2<i32>(src_x, src_y));
    textureStore(output, vec2<i32>(id.xy), color);
}
