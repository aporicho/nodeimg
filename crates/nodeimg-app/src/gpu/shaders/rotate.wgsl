@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    angle_rad: f32,
    fill_r: f32,
    fill_g: f32,
    fill_b: f32,
    fill_a: f32,
    center_x: f32,
    center_y: f32,
    _pad: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let fill = vec4<f32>(params.fill_r, params.fill_g, params.fill_b, params.fill_a);

    let cos_a = cos(params.angle_rad);
    let sin_a = sin(params.angle_rad);

    // Map destination pixel back to source space (inverse rotation)
    let dx = f32(id.x) - params.center_x;
    let dy = f32(id.y) - params.center_y;

    let src_x = dx * cos_a - dy * sin_a + params.center_x;
    let src_y = dx * sin_a + dy * cos_a + params.center_y;

    // Check bounds
    if (src_x < 0.0 || src_y < 0.0 || src_x >= f32(dims.x) - 1.0 || src_y >= f32(dims.y) - 1.0) {
        textureStore(output, vec2<i32>(id.xy), fill);
        return;
    }

    // Bilinear interpolation
    let x0 = i32(floor(src_x));
    let y0 = i32(floor(src_y));
    let x1 = min(x0 + 1, i32(dims.x) - 1);
    let y1 = min(y0 + 1, i32(dims.y) - 1);
    let tx = src_x - floor(src_x);
    let ty = src_y - floor(src_y);

    let p00 = textureLoad(input, vec2<i32>(x0, y0));
    let p10 = textureLoad(input, vec2<i32>(x1, y0));
    let p01 = textureLoad(input, vec2<i32>(x0, y1));
    let p11 = textureLoad(input, vec2<i32>(x1, y1));

    let color = p00 * (1.0 - tx) * (1.0 - ty)
              + p10 * tx * (1.0 - ty)
              + p01 * (1.0 - tx) * ty
              + p11 * tx * ty;

    textureStore(output, vec2<i32>(id.xy), color);
}
