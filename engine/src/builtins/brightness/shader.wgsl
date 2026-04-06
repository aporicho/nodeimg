@group(0) @binding(0) var input_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

struct Params {
    brightness: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let dims = textureDimensions(input_tex);
    if gid.x >= dims.x || gid.y >= dims.y { return; }
    let coord = vec2<i32>(i32(gid.x), i32(gid.y));
    var color = textureLoad(input_tex, coord);
    color = vec4<f32>(
        clamp(color.r + params.brightness, 0.0, 1.0),
        clamp(color.g + params.brightness, 0.0, 1.0),
        clamp(color.b + params.brightness, 0.0, 1.0),
        color.a,
    );
    textureStore(output_tex, coord, color);
}
