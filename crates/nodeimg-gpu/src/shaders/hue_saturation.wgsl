@group(0) @binding(0) var input: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(1) var output: texture_storage_2d<rgba8unorm, write>;

struct Params {
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
    _pad: f32,
}
@group(0) @binding(2) var<uniform> params: Params;

fn hue_to_rgb(p: f32, q: f32, t_in: f32) -> f32 {
    var t = t_in;
    if (t < 0.0) {
        t = t + 1.0;
    }
    if (t > 1.0) {
        t = t - 1.0;
    }
    if (t < 1.0 / 6.0) {
        return p + (q - p) * 6.0 * t;
    }
    if (t < 1.0 / 2.0) {
        return q;
    }
    if (t < 2.0 / 3.0) {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    return p;
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> vec3<f32> {
    let max_c = max(r, max(g, b));
    let min_c = min(r, min(g, b));
    let delta = max_c - min_c;
    let l = (max_c + min_c) / 2.0;

    if (delta < 1e-6) {
        return vec3<f32>(0.0, 0.0, l);
    }

    var s: f32;
    if (l < 0.5) {
        s = delta / (max_c + min_c);
    } else {
        s = delta / (2.0 - max_c - min_c);
    }

    var h: f32;
    if (abs(max_c - r) < 1e-6) {
        // WGSL doesn't have rem_euclid, do manual modulo
        h = ((g - b) / delta);
        // equivalent of rem_euclid(6.0)
        h = h - floor(h / 6.0) * 6.0;
        h = h * 60.0;
    } else if (abs(max_c - g) < 1e-6) {
        h = ((b - r) / delta + 2.0) * 60.0;
    } else {
        h = ((r - g) / delta + 4.0) * 60.0;
    }

    return vec3<f32>(h, s, l);
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> vec3<f32> {
    if (s < 1e-6) {
        return vec3<f32>(l, l, l);
    }

    var q: f32;
    if (l < 0.5) {
        q = l * (1.0 + s);
    } else {
        q = l + s - l * s;
    }
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;

    let r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h_norm);
    let b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);
    return vec3<f32>(r, g, b);
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let dims = textureDimensions(input);
    if (id.x >= dims.x || id.y >= dims.y) {
        return;
    }

    let color = textureLoad(input, vec2<i32>(id.xy));

    let hsl = rgb_to_hsl(color.r, color.g, color.b);

    // Adjust hue (rem_euclid 360)
    var new_h = hsl.x + params.hue_shift;
    new_h = new_h - floor(new_h / 360.0) * 360.0;

    let new_s = clamp(hsl.y + params.saturation, 0.0, 1.0);
    let new_l = clamp(hsl.z + params.lightness, 0.0, 1.0);

    let rgb = hsl_to_rgb(new_h, new_s, new_l);

    let out_color = vec4<f32>(
        clamp(rgb.x, 0.0, 1.0),
        clamp(rgb.y, 0.0, 1.0),
        clamp(rgb.z, 0.0, 1.0),
        color.a
    );
    textureStore(output, vec2<i32>(id.xy), out_color);
}
