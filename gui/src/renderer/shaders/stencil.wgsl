struct Viewport {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let ndc_x = (in.position.x / viewport.size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.position.y / viewport.size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0);
}
