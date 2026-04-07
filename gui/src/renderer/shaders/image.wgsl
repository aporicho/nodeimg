struct Viewport {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct ImageInstance {
    @location(0) rect: vec4<f32>,  // x, y, w, h
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
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: ImageInstance,
) -> VertexOutput {
    let uv = QUAD_UVS[vertex_index];

    let px = instance.rect.x + uv.x * instance.rect.z;
    let py = instance.rect.y + uv.y * instance.rect.w;

    let ndc_x = (px / viewport.size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / viewport.size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.uv);
}
