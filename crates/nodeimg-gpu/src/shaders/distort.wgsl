@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    distort_type: u32,  // 0=barrel, 1=pincushion, 2=swirl
    strength: f32,
    center_x: f32,
    center_y: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let cx = params.center_x;
    let cy = params.center_y;
    let r_max = sqrt(cx * cx + cy * cy);

    // Normalise to [-1, 1]
    let nx = (f32(id.x) - cx) / r_max;
    let ny = (f32(id.y) - cy) / r_max;

    var src_nx: f32;
    var src_ny: f32;

    if (params.distort_type == 0u) {
        // Barrel
        let r2 = nx * nx + ny * ny;
        let scale = 1.0 + params.strength * r2;
        if (abs(scale) < 1e-6) {
            textureStore(output, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 0.0));
            return;
        }
        src_nx = nx / scale;
        src_ny = ny / scale;
    } else if (params.distort_type == 1u) {
        // Pincushion
        let r2 = nx * nx + ny * ny;
        let scale = 1.0 - params.strength * r2;
        if (abs(scale) < 1e-6) {
            textureStore(output, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 0.0));
            return;
        }
        src_nx = nx / scale;
        src_ny = ny / scale;
    } else {
        // Swirl
        let r = sqrt(nx * nx + ny * ny);
        let theta = atan2(ny, nx);
        let twist = params.strength * (1.0 - r);
        let src_theta = theta - twist;
        src_nx = r * cos(src_theta);
        src_ny = r * sin(src_theta);
    }

    // Back to pixel coordinates
    let fx = src_nx * r_max + cx;
    let fy = src_ny * r_max + cy;

    let w = f32(dims.x);
    let h = f32(dims.y);

    if (fx < 0.0 || fy < 0.0 || fx > w - 1.0 || fy > h - 1.0) {
        textureStore(output, vec2<i32>(id.xy), vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return;
    }

    // Bilinear interpolation
    let x0 = i32(floor(fx));
    let y0 = i32(floor(fy));
    let x1 = min(x0 + 1, i32(dims.x) - 1);
    let y1 = min(y0 + 1, i32(dims.y) - 1);
    let tx = fx - floor(fx);
    let ty = fy - floor(fy);

    let p00 = textureLoad(input, vec2<i32>(x0, y0));
    let p10 = textureLoad(input, vec2<i32>(x1, y0));
    let p01 = textureLoad(input, vec2<i32>(x0, y1));
    let p11 = textureLoad(input, vec2<i32>(x1, y1));

    let color = p00 * (1.0 - tx) * (1.0 - ty)
              + p10 * tx * (1.0 - ty)
              + p01 * (1.0 - tx) * ty
              + p11 * tx * ty;

    textureStore(output, vec2<i32>(id.xy), color);
}
