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

    var sum = vec4<f32>(0.0);
    let count = f32(2 * r + 1);

    for (var i: i32 = -r; i <= r; i = i + 1) {
        let sx = clamp(i32(id.x) + i, 0, i32(dims.x) - 1);
        sum = sum + textureLoad(input, vec2<i32>(sx, i32(id.y)));
    }

    let result = sum / count;
    textureStore(output, vec2<i32>(id.xy), result);
}
