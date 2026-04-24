struct Viewport {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) modulate: vec4<f32>,
}

struct ImageInstance {
    @location(0) p0p1: vec4<f32>,
    @location(1) p2p3: vec4<f32>,
    @location(2) uv_rect: vec4<f32>,  // x, y, w, h in normalized texture space
    @location(3) modulate: vec4<f32>,
}

const QUAD_CORNER_INDICES = array<u32, 6>(
    0u,
    1u,
    3u,
    1u,
    2u,
    3u,
);

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
    let quad_uv = QUAD_UVS[vertex_index];
    let uv = instance.uv_rect.xy + quad_uv * instance.uv_rect.zw;

    let positions = array<vec2<f32>, 4>(
        instance.p0p1.xy,
        instance.p0p1.zw,
        instance.p2p3.xy,
        instance.p2p3.zw,
    );
    let point = positions[QUAD_CORNER_INDICES[vertex_index]];

    let ndc_x = (point.x / viewport.size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (point.y / viewport.size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = uv;
    out.modulate = instance.modulate;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.uv) * in.modulate;
}
