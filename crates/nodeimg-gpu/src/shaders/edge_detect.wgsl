@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    method: u32,   // 0=sobel, 1=laplacian
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

fn luminance(c: vec4<f32>) -> f32 {
    return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
}

fn sample_clamped(coord: vec2<i32>, dims: vec2<u32>) -> vec4<f32> {
    let cx = clamp(coord.x, 0, i32(dims.x) - 1);
    let cy = clamp(coord.y, 0, i32(dims.y) - 1);
    return textureLoad(input, vec2<i32>(cx, cy));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let pos = vec2<i32>(id.xy);

    if (params.method == 1u) {
        // Laplacian: [0,-1,0; -1,4,-1; 0,-1,0]
        let c  = luminance(sample_clamped(pos + vec2<i32>( 0,  0), dims));
        let t  = luminance(sample_clamped(pos + vec2<i32>( 0, -1), dims));
        let b  = luminance(sample_clamped(pos + vec2<i32>( 0,  1), dims));
        let l  = luminance(sample_clamped(pos + vec2<i32>(-1,  0), dims));
        let r  = luminance(sample_clamped(pos + vec2<i32>( 1,  0), dims));

        let val = abs(4.0 * c - t - b - l - r);
        let mag = clamp(val, 0.0, 1.0);
        textureStore(output, pos, vec4<f32>(mag, mag, mag, 1.0));
    } else {
        // Sobel
        let tl = luminance(sample_clamped(pos + vec2<i32>(-1, -1), dims));
        let tc = luminance(sample_clamped(pos + vec2<i32>( 0, -1), dims));
        let tr = luminance(sample_clamped(pos + vec2<i32>( 1, -1), dims));
        let ml = luminance(sample_clamped(pos + vec2<i32>(-1,  0), dims));
        let mr = luminance(sample_clamped(pos + vec2<i32>( 1,  0), dims));
        let bl = luminance(sample_clamped(pos + vec2<i32>(-1,  1), dims));
        let bc = luminance(sample_clamped(pos + vec2<i32>( 0,  1), dims));
        let br = luminance(sample_clamped(pos + vec2<i32>( 1,  1), dims));

        // Gx = [-1,0,1; -2,0,2; -1,0,1]
        let gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
        // Gy = [-1,-2,-1; 0,0,0; 1,2,1]
        let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;

        let mag = clamp(sqrt(gx * gx + gy * gy), 0.0, 1.0);
        textureStore(output, pos, vec4<f32>(mag, mag, mag, 1.0));
    }
}
