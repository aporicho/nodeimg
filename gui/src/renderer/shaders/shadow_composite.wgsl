struct Viewport {
    size: vec2<f32>,
}

struct ShadowRect {
    rect: vec4<f32>,  // x, y, w, h（逻辑像素，含偏移）
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var<uniform> shadow_rect: ShadowRect;
@group(0) @binding(2) var shadow_tex: texture_2d<f32>;
@group(0) @binding(3) var shadow_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

const QUAD_UVS = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 1.0),
);

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = QUAD_UVS[vertex_index];

    let px = shadow_rect.rect.x + uv.x * shadow_rect.rect.z;
    let py = shadow_rect.rect.y + uv.y * shadow_rect.rect.w;

    let ndc_x = (px / viewport.size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / viewport.size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(shadow_tex, shadow_sampler, in.uv);
}
