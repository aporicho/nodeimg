struct Viewport {
    size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> viewport: Viewport;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) pattern_pos: vec2<f32>,
    @location(2) spacing: f32,
    @location(3) radius: f32,
    @location(4) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pattern_pos: vec2<f32>,
    @location(1) spacing: f32,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let ndc_x = (in.position.x / viewport.size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.position.y / viewport.size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.pattern_pos = in.pattern_pos;
    out.spacing = in.spacing;
    out.radius = in.radius;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if in.spacing <= 0.0 || in.radius <= 0.0 {
        discard;
    }

    let cell = abs(fract(in.pattern_pos / in.spacing + vec2<f32>(0.5, 0.5)) - vec2<f32>(0.5, 0.5)) * in.spacing;
    let dist = length(cell);
    let alpha = 1.0 - smoothstep(in.radius, in.radius + 1.0, dist);
    if alpha < 0.001 {
        discard;
    }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
