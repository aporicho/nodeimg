@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    direction: u32,  // 0=horizontal, 1=vertical, 2=both
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    var src_x = i32(id.x);
    var src_y = i32(id.y);

    if (params.direction == 0u || params.direction == 2u) {
        src_x = i32(dims.x) - 1 - src_x;
    }
    if (params.direction == 1u || params.direction == 2u) {
        src_y = i32(dims.y) - 1 - src_y;
    }

    let color = textureLoad(input, vec2<i32>(src_x, src_y));
    textureStore(output, vec2<i32>(id.xy), color);
}
