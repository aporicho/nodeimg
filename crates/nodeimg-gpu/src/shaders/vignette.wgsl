@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    strength: f32,
    radius: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));

    // Normalized coordinates centered at (0.5, 0.5)
    let uv = vec2<f32>(f32(id.x) / f32(dims.x), f32(id.y) / f32(dims.y));
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);

    // Vignette factor: smoothstep from radius to edge
    let r = params.radius * 0.707; // 0.707 ~= sqrt(0.5), max corner distance
    let falloff = smoothstep(r, r + params.strength * 0.5, dist);
    let factor = 1.0 - falloff * params.strength;

    let result = vec4<f32>(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    );
    textureStore(output, vec2<i32>(id.xy), result);
}
