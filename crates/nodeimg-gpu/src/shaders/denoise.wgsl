@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    strength: f32,
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

    let pos = vec2<i32>(id.xy);
    let center = textureLoad(input, pos);

    // Bilateral filter parameters
    let spatial_sigma = 2.0;
    // color_sigma = strength * 10 in 0–255 space → in 0–1 space divide by 255
    let color_sigma = params.strength * 10.0 / 255.0;

    let two_spatial_sq = 2.0 * spatial_sigma * spatial_sigma;
    let two_color_sq = 2.0 * color_sigma * color_sigma;

    var acc = vec3<f32>(0.0, 0.0, 0.0);
    var weight_sum: f32 = 0.0;

    // 5x5 window (radius 2)
    for (var dy: i32 = -2; dy <= 2; dy = dy + 1) {
        for (var dx: i32 = -2; dx <= 2; dx = dx + 1) {
            let sx = clamp(pos.x + dx, 0, i32(dims.x) - 1);
            let sy = clamp(pos.y + dy, 0, i32(dims.y) - 1);
            let p = textureLoad(input, vec2<i32>(sx, sy));

            // Spatial weight
            let dist2 = f32(dx * dx + dy * dy);
            let sw = exp(-dist2 / two_spatial_sq);

            // Color weight (in 0–1 range)
            let dr = p.r - center.r;
            let dg = p.g - center.g;
            let db = p.b - center.b;
            let color_diff2 = dr * dr + dg * dg + db * db;
            let cw = exp(-color_diff2 / two_color_sq);

            let w = sw * cw;
            acc = acc + vec3<f32>(p.r, p.g, p.b) * w;
            weight_sum = weight_sum + w;
        }
    }

    let result = acc / weight_sum;
    textureStore(output, pos, vec4<f32>(
        clamp(result.r, 0.0, 1.0),
        clamp(result.g, 0.0, 1.0),
        clamp(result.b, 0.0, 1.0),
        center.a
    ));
}
