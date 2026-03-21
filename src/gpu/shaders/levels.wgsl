@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
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

    let in_range = max(params.in_white - params.in_black, 1e-6);
    let gamma_inv = 1.0 / max(params.gamma, 1e-6);

    // Process each channel
    let r_norm = clamp((color.r - params.in_black) / in_range, 0.0, 1.0);
    let g_norm = clamp((color.g - params.in_black) / in_range, 0.0, 1.0);
    let b_norm = clamp((color.b - params.in_black) / in_range, 0.0, 1.0);

    // Gamma correction
    let r_gamma = pow(r_norm, gamma_inv);
    let g_gamma = pow(g_norm, gamma_inv);
    let b_gamma = pow(b_norm, gamma_inv);

    // Output mapping
    let r_out = params.out_black + r_gamma * (params.out_white - params.out_black);
    let g_out = params.out_black + g_gamma * (params.out_white - params.out_black);
    let b_out = params.out_black + b_gamma * (params.out_white - params.out_black);

    let out_color = vec4<f32>(
        clamp(r_out, 0.0, 1.0),
        clamp(g_out, 0.0, 1.0),
        clamp(b_out, 0.0, 1.0),
        color.a
    );
    textureStore(output, vec2<i32>(id.xy), out_color);
}
