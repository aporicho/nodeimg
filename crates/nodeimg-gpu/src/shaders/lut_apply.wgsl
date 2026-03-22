@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    lut_size: f32,
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

// LUT data: size^3 entries, each as vec4(r, g, b, 0)
@group(0) @binding(3) var<storage, read> lut_data: array<vec4<f32>>;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));
    let size = i32(params.lut_size);
    let scale = params.lut_size - 1.0;

    // Scale to LUT coordinates
    let rf = color.r * scale;
    let gf = color.g * scale;
    let bf = color.b * scale;

    let r0 = i32(floor(rf));
    let g0 = i32(floor(gf));
    let b0 = i32(floor(bf));
    let r1 = min(r0 + 1, size - 1);
    let g1 = min(g0 + 1, size - 1);
    let b1 = min(b0 + 1, size - 1);

    let dr = rf - f32(r0);
    let dg = gf - f32(g0);
    let db = bf - f32(b0);

    // Cube file order: index = r * size*size + g * size + b
    let c000 = lut_data[r0 * size * size + g0 * size + b0].rgb;
    let c001 = lut_data[r0 * size * size + g0 * size + b1].rgb;
    let c010 = lut_data[r0 * size * size + g1 * size + b0].rgb;
    let c011 = lut_data[r0 * size * size + g1 * size + b1].rgb;
    let c100 = lut_data[r1 * size * size + g0 * size + b0].rgb;
    let c101 = lut_data[r1 * size * size + g0 * size + b1].rgb;
    let c110 = lut_data[r1 * size * size + g1 * size + b0].rgb;
    let c111 = lut_data[r1 * size * size + g1 * size + b1].rgb;

    // Trilinear interpolation
    let lut_rgb = c000 * (1.0 - dr) * (1.0 - dg) * (1.0 - db)
        + c001 * (1.0 - dr) * (1.0 - dg) * db
        + c010 * (1.0 - dr) * dg * (1.0 - db)
        + c011 * (1.0 - dr) * dg * db
        + c100 * dr * (1.0 - dg) * (1.0 - db)
        + c101 * dr * (1.0 - dg) * db
        + c110 * dr * dg * (1.0 - db)
        + c111 * dr * dg * db;

    // Blend with original based on intensity
    let result = mix(color.rgb, lut_rgb, params.intensity);
    textureStore(output, vec2<i32>(id.xy), vec4<f32>(clamp(result, vec3(0.0), vec3(1.0)), color.a));
}
