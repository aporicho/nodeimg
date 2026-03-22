@group(0) @binding(0) var red_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var green_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(2) var blue_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(3) var alpha_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(4) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    // Bitmask: bit 0 = has_red, bit 1 = has_green, bit 2 = has_blue, bit 3 = has_alpha
    channel_mask: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
@group(0) @binding(5) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(id.xy);
    let mask = i32(params.channel_mask);

    // Default: black with full alpha
    var r: f32 = 0.0;
    var g: f32 = 0.0;
    var b: f32 = 0.0;
    var a: f32 = 1.0;

    if ((mask & 1) != 0) {
        let c = textureLoad(red_tex, coord);
        // Use luminance as the channel value
        r = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    }
    if ((mask & 2) != 0) {
        let c = textureLoad(green_tex, coord);
        g = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    }
    if ((mask & 4) != 0) {
        let c = textureLoad(blue_tex, coord);
        b = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    }
    if ((mask & 8) != 0) {
        let c = textureLoad(alpha_tex, coord);
        a = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    }

    textureStore(output, coord, vec4<f32>(r, g, b, a));
}
