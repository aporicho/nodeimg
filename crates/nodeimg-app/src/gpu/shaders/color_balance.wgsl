@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    shadows_r: f32,
    shadows_g: f32,
    shadows_b: f32,
    midtones_r: f32,
    midtones_g: f32,
    midtones_b: f32,
    highlights_r: f32,
    highlights_g: f32,
    highlights_b: f32,
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

    let lum = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;

    // Weights matching CPU implementation
    let shadows_w = clamp(1.0 - lum * 4.0, 0.0, 1.0);
    let highlights_w = clamp(lum * 4.0 - 3.0, 0.0, 1.0);
    let midtones_w = 1.0 - shadows_w - highlights_w;

    let r = color.r
        + params.shadows_r * shadows_w
        + params.midtones_r * midtones_w
        + params.highlights_r * highlights_w;
    let g = color.g
        + params.shadows_g * shadows_w
        + params.midtones_g * midtones_w
        + params.highlights_g * highlights_w;
    let b = color.b
        + params.shadows_b * shadows_w
        + params.midtones_b * midtones_w
        + params.highlights_b * highlights_w;

    let out_color = vec4<f32>(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), color.a);
    textureStore(output, vec2<i32>(id.xy), out_color);
}
