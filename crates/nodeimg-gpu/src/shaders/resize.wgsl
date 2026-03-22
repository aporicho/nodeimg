@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    src_width: f32,
    src_height: f32,
    dst_width: f32,
    dst_height: f32,
    method: u32,    // 0=nearest, 1=bilinear, 2=bicubic, 3=lanczos3
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}
@group(0) @binding(2) var<uniform> params: Params;

const PI: f32 = 3.14159265358979;

// Clamp-load a texel from input, clamping coordinates to valid range.
fn load_clamped(x: i32, y: i32) -> vec4<f32> {
    let max_x = i32(params.src_width) - 1;
    let max_y = i32(params.src_height) - 1;
    return textureLoad(input, vec2<i32>(clamp(x, 0, max_x), clamp(y, 0, max_y)));
}

// Catmull-Rom cubic weight function.
// w(t) = 1.5|t|^3 - 2.5|t|^2 + 1            for |t| <= 1
// w(t) = -0.5|t|^3 + 2.5|t|^2 - 4|t| + 2    for 1 < |t| < 2
// w(t) = 0                                     for |t| >= 2
fn cubic_weight(t: f32) -> f32 {
    let at = abs(t);
    if (at <= 1.0) {
        return 1.5 * at * at * at - 2.5 * at * at + 1.0;
    } else if (at < 2.0) {
        return -0.5 * at * at * at + 2.5 * at * at - 4.0 * at + 2.0;
    }
    return 0.0;
}

// sinc(x) = sin(pi*x) / (pi*x), sinc(0) = 1
fn sinc(x: f32) -> f32 {
    if (abs(x) < 1e-7) {
        return 1.0;
    }
    let px = PI * x;
    return sin(px) / px;
}

// Lanczos3 weight function: sinc(x) * sinc(x/3) for |x| < 3, else 0
fn lanczos3_weight(x: f32) -> f32 {
    let ax = abs(x);
    if (ax >= 3.0) {
        return 0.0;
    }
    return sinc(ax) * sinc(ax / 3.0);
}

// Nearest-neighbor interpolation
fn nearest(src_x: f32, src_y: f32) -> vec4<f32> {
    let ix = i32(round(src_x));
    let iy = i32(round(src_y));
    return load_clamped(ix, iy);
}

// Bilinear interpolation
fn bilinear(src_x: f32, src_y: f32) -> vec4<f32> {
    let x0 = i32(floor(src_x));
    let y0 = i32(floor(src_y));
    let tx = src_x - floor(src_x);
    let ty = src_y - floor(src_y);

    let p00 = load_clamped(x0, y0);
    let p10 = load_clamped(x0 + 1, y0);
    let p01 = load_clamped(x0, y0 + 1);
    let p11 = load_clamped(x0 + 1, y0 + 1);

    return p00 * (1.0 - tx) * (1.0 - ty)
         + p10 * tx * (1.0 - ty)
         + p01 * (1.0 - tx) * ty
         + p11 * tx * ty;
}

// Bicubic (Catmull-Rom) interpolation — 4x4 kernel
fn bicubic(src_x: f32, src_y: f32) -> vec4<f32> {
    let ix = i32(floor(src_x));
    let iy = i32(floor(src_y));
    let fx = src_x - floor(src_x);
    let fy = src_y - floor(src_y);

    var color = vec4<f32>(0.0);

    // 4x4 kernel: offsets -1, 0, 1, 2 relative to the integer position
    for (var j = -1; j <= 2; j = j + 1) {
        let wy = cubic_weight(f32(j) - fy);
        for (var i = -1; i <= 2; i = i + 1) {
            let wx = cubic_weight(f32(i) - fx);
            let sample = load_clamped(ix + i, iy + j);
            color = color + sample * wx * wy;
        }
    }

    return clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
}

// Lanczos3 interpolation — 6x6 kernel
fn lanczos3(src_x: f32, src_y: f32) -> vec4<f32> {
    let ix = i32(floor(src_x));
    let iy = i32(floor(src_y));
    let fx = src_x - floor(src_x);
    let fy = src_y - floor(src_y);

    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;

    // 6x6 kernel: offsets -2, -1, 0, 1, 2, 3 relative to the integer position
    for (var j = -2; j <= 3; j = j + 1) {
        let wy = lanczos3_weight(f32(j) - fy);
        for (var i = -2; i <= 3; i = i + 1) {
            let wx = lanczos3_weight(f32(i) - fx);
            let w = wx * wy;
            let sample = load_clamped(ix + i, iy + j);
            color = color + sample * w;
            weight_sum = weight_sum + w;
        }
    }

    // Normalize to ensure weights sum to 1
    if (weight_sum > 0.0) {
        color = color / weight_sum;
    }

    return clamp(color, vec4<f32>(0.0), vec4<f32>(1.0));
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let out_dims = textureDimensions(output);
    if (id.x >= out_dims.x || id.y >= out_dims.y) {
        return;
    }

    // Map destination pixel to source coordinates
    let src_x = (f32(id.x) + 0.5) * params.src_width / params.dst_width - 0.5;
    let src_y = (f32(id.y) + 0.5) * params.src_height / params.dst_height - 0.5;

    var color: vec4<f32>;

    if (params.method == 0u) {
        color = nearest(src_x, src_y);
    } else if (params.method == 2u) {
        color = bicubic(src_x, src_y);
    } else if (params.method == 3u) {
        color = lanczos3(src_x, src_y);
    } else {
        // Default: bilinear (method == 1 or any unrecognized value)
        color = bilinear(src_x, src_y);
    }

    textureStore(output, vec2<i32>(id.xy), color);
}
