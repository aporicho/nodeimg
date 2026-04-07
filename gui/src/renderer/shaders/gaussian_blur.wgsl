struct BlurParams {
    direction: vec2<f32>,  // (1,0) 水平 or (0,1) 垂直
    sigma: f32,
    tex_size: vec2<f32>,   // 纹理尺寸（像素）
}

@group(0) @binding(0) var input_tex: texture_2d<f32>;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<uniform> params: BlurParams;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let coords = vec2<i32>(gid.xy);
    let size = vec2<i32>(params.tex_size);

    if coords.x >= size.x || coords.y >= size.y {
        return;
    }

    let sigma = params.sigma;
    let radius = i32(ceil(sigma * 3.0));

    // 高斯核归一化累加
    var color = vec4<f32>(0.0);
    var weight_sum = 0.0;

    for (var i = -radius; i <= radius; i++) {
        let offset = vec2<i32>(params.direction * f32(i));
        let sample_coords = coords + offset;

        // 边界 clamp
        let clamped = clamp(sample_coords, vec2<i32>(0), size - vec2<i32>(1));

        let w = exp(-f32(i * i) / (2.0 * sigma * sigma));
        color += textureLoad(input_tex, clamped, 0) * w;
        weight_sum += w;
    }

    textureStore(output_tex, coords, color / weight_sum);
}
