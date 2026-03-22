@group(0) @binding(0) var base_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var layer_tex: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(2) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    // mode: 0=normal, 1=multiply, 2=screen, 3=overlay, 4=add, 5=subtract, 6=difference
    mode: f32,
    opacity: f32,
    _pad1: f32,
    _pad2: f32,
}
@group(0) @binding(3) var<uniform> params: Params;

fn overlay_channel(base: f32, layer: f32) -> f32 {
    if (base < 0.5) {
        return 2.0 * base * layer;
    } else {
        return 1.0 - 2.0 * (1.0 - base) * (1.0 - layer);
    }
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(output);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let coord = vec2<i32>(id.xy);
    let base = textureLoad(base_tex, coord);
    let layer = textureLoad(layer_tex, coord);

    let mode = i32(params.mode);

    var blended: vec3<f32>;

    if (mode == 1) {
        // Multiply
        blended = base.rgb * layer.rgb;
    } else if (mode == 2) {
        // Screen
        blended = 1.0 - (1.0 - base.rgb) * (1.0 - layer.rgb);
    } else if (mode == 3) {
        // Overlay
        blended = vec3<f32>(
            overlay_channel(base.r, layer.r),
            overlay_channel(base.g, layer.g),
            overlay_channel(base.b, layer.b),
        );
    } else if (mode == 4) {
        // Add
        blended = base.rgb + layer.rgb;
    } else if (mode == 5) {
        // Subtract
        blended = base.rgb - layer.rgb;
    } else if (mode == 6) {
        // Difference
        blended = abs(base.rgb - layer.rgb);
    } else {
        // Normal (mode 0)
        blended = layer.rgb;
    }

    blended = clamp(blended, vec3<f32>(0.0), vec3<f32>(1.0));

    // Apply opacity: mix base with blended result
    let result_rgb = mix(base.rgb, blended, params.opacity);
    let result_a = mix(base.a, layer.a, params.opacity);

    textureStore(output, coord, vec4<f32>(result_rgb, result_a));
}
