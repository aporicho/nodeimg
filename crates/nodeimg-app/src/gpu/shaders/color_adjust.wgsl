@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    brightness: f32,
    contrast: f32,
    saturation: f32,
    _pad: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));

    // Brightness: add offset
    var r = color.r + params.brightness;
    var g = color.g + params.brightness;
    var b = color.b + params.brightness;

    // Contrast factor: map [-1, 1] to [0.5, 2.0] same as CPU
    var contrast_factor: f32;
    if (params.contrast >= 0.0) {
        contrast_factor = 1.0 + params.contrast;
    } else {
        contrast_factor = 1.0 + params.contrast * 0.5;
    }

    // Contrast: around 0.5 midpoint
    r = (r - 0.5) * contrast_factor + 0.5;
    g = (g - 0.5) * contrast_factor + 0.5;
    b = (b - 0.5) * contrast_factor + 0.5;

    // Saturation factor: map [-1, 1] to [0, 2]
    let sat_factor = 1.0 + params.saturation;
    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    r = lum + (r - lum) * sat_factor;
    g = lum + (g - lum) * sat_factor;
    b = lum + (b - lum) * sat_factor;

    // Clamp and write
    let out_color = vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), color.a);
    textureStore(output, vec2<i32>(id.xy), out_color);
}
