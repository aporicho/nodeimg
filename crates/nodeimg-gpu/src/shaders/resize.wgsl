@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    src_width: f32,
    src_height: f32,
    dst_width: f32,
    dst_height: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let out_dims = textureDimensions(output);
    if (id.x >= out_dims.x || id.y >= out_dims.y) {
        return;
    }

    // Map destination pixel to source coordinates
    let src_x = (f32(id.x) + 0.5) * params.src_width / params.dst_width - 0.5;
    let src_y = (f32(id.y) + 0.5) * params.src_height / params.dst_height - 0.5;

    // Bilinear interpolation
    let x0 = i32(floor(src_x));
    let y0 = i32(floor(src_y));
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let tx = src_x - floor(src_x);
    let ty = src_y - floor(src_y);

    let max_x = i32(params.src_width) - 1;
    let max_y = i32(params.src_height) - 1;

    let cx0 = clamp(x0, 0, max_x);
    let cy0 = clamp(y0, 0, max_y);
    let cx1 = clamp(x1, 0, max_x);
    let cy1 = clamp(y1, 0, max_y);

    let p00 = textureLoad(input, vec2<i32>(cx0, cy0));
    let p10 = textureLoad(input, vec2<i32>(cx1, cy0));
    let p01 = textureLoad(input, vec2<i32>(cx0, cy1));
    let p11 = textureLoad(input, vec2<i32>(cx1, cy1));

    let color = p00 * (1.0 - tx) * (1.0 - ty)
              + p10 * tx * (1.0 - ty)
              + p01 * (1.0 - tx) * ty
              + p11 * tx * ty;

    textureStore(output, vec2<i32>(id.xy), color);
}
