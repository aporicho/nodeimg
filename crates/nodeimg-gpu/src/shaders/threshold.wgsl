@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    threshold_val: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));

    // Luminance using Rec.709 coefficients (matching CPU)
    let lum = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;

    var v: f32;
    if (lum >= params.threshold_val) {
        v = 1.0;
    } else {
        v = 0.0;
    }

    textureStore(output, vec2<i32>(id.xy), vec4<f32>(v, v, v, color.a));
}
