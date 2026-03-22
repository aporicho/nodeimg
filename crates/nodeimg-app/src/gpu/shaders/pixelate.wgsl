@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    block_size: f32,
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

    let bs = max(i32(params.block_size), 1);

    // Find the top-left corner of the block this pixel belongs to
    let bx = (i32(id.x) / bs) * bs;
    let by = (i32(id.y) / bs) * bs;

    // Average all pixels in the block
    var sum = vec4<f32>(0.0);
    var count: f32 = 0.0;

    let max_x = min(bx + bs, i32(dims.x));
    let max_y = min(by + bs, i32(dims.y));

    for (var y: i32 = by; y < max_y; y = y + 1) {
        for (var x: i32 = bx; x < max_x; x = x + 1) {
            sum = sum + textureLoad(input, vec2<i32>(x, y));
            count = count + 1.0;
        }
    }

    let avg = sum / count;
    textureStore(output, vec2<i32>(id.xy), avg);
}
