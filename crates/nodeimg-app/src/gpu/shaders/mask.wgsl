@group(0) @binding(0) var image_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var mask_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    invert_mask: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(0) @binding(3) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(id.xy);
    let color = textureLoad(image_tex, coord);
    let mask_color = textureLoad(mask_tex, coord);

    // Use luminance of mask as alpha multiplier
    var mask_val = 0.2126 * mask_color.r + 0.7152 * mask_color.g + 0.0722 * mask_color.b;

    if (params.invert_mask > 0.5) {
        mask_val = 1.0 - mask_val;
    }

    let result = vec4<f32>(color.rgb, color.a * mask_val);
    textureStore(output, coord, result);
}
