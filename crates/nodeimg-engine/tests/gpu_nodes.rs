mod helpers;

use std::collections::HashMap;
use std::sync::Arc;

use nodeimg_gpu::test_utils::try_create_headless_context;
use nodeimg_gpu::GpuContext;
use nodeimg_processing::test_helpers::make_test_image;
use nodeimg_types::value::Value;

fn gpu() -> Option<Arc<GpuContext>> {
    let ctx = try_create_headless_context();
    if ctx.is_none() {
        eprintln!("SKIPPED: no GPU adapter available");
    }
    ctx
}

fn assert_gpu_image(result: &HashMap<String, Value>, key: &str, w: u32, h: u32) {
    match result.get(key) {
        Some(Value::GpuImage(tex)) => {
            assert_eq!(tex.width, w, "expected width {}, got {}", w, tex.width);
            assert_eq!(tex.height, h, "expected height {}, got {}", h, tex.height);
        }
        other => panic!("expected GpuImage for '{}', got {:?}", key, other),
    }
}

// =====================================================================
// Generator nodes (no inputs)
// =====================================================================

#[test]
fn test_solid_color() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_node_test(
        "solid_color",
        HashMap::from([
            ("color".into(), Value::Color([1.0, 0.0, 0.0, 1.0])),
            ("width".into(), Value::Int(64)),
            ("height".into(), Value::Int(64)),
        ]),
        HashMap::new(),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 64, 64);

    // Pixel verification: should be red
    if let Some(Value::GpuImage(tex)) = result.get("image") {
        let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
        let (r, g, b) = (pixels[0], pixels[1], pixels[2]);
        assert!(r >= 254, "red channel should be ~255, got {}", r);
        assert!(g <= 1, "green channel should be ~0, got {}", g);
        assert!(b <= 1, "blue channel should be ~0, got {}", b);
    }
}

#[test]
fn test_gradient() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_node_test(
        "gradient",
        HashMap::from([
            ("color_start".into(), Value::Color([0.0, 0.0, 0.0, 1.0])),
            ("color_end".into(), Value::Color([1.0, 1.0, 1.0, 1.0])),
            ("direction".into(), Value::String("horizontal".into())),
            ("width".into(), Value::Int(64)),
            ("height".into(), Value::Int(64)),
        ]),
        HashMap::new(),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 64, 64);
}

#[test]
fn test_checkerboard() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_node_test(
        "checkerboard",
        HashMap::from([
            ("color_a".into(), Value::Color([1.0, 1.0, 1.0, 1.0])),
            ("color_b".into(), Value::Color([0.0, 0.0, 0.0, 1.0])),
            ("size".into(), Value::Int(8)),
            ("width".into(), Value::Int(64)),
            ("height".into(), Value::Int(64)),
        ]),
        HashMap::new(),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 64, 64);
}

#[test]
fn test_noise() {
    let Some(ctx) = gpu() else { return };
    let result = helpers::run_node_test(
        "noise",
        HashMap::from([
            ("seed".into(), Value::Int(42)),
            ("scale".into(), Value::Float(1.0)),
            ("width".into(), Value::Int(64)),
            ("height".into(), Value::Int(64)),
        ]),
        HashMap::new(),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 64, 64);
}

// =====================================================================
// Single-input color nodes
// =====================================================================

#[test]
fn test_invert() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "invert",
        HashMap::new(),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);

    // Pixel verification: red -> cyan (0, 255, 255)
    if let Some(Value::GpuImage(tex)) = result.get("image") {
        let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
        let (r, g, b) = (pixels[0], pixels[1], pixels[2]);
        assert!(r <= 1, "inverted red should be ~0, got {}", r);
        assert!(g >= 254, "inverted green should be ~255, got {}", g);
        assert!(b >= 254, "inverted blue should be ~255, got {}", b);
    }
}

#[test]
fn test_color_adjust() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "color_adjust",
        HashMap::from([
            ("brightness".into(), Value::Float(0.1)),
            ("saturation".into(), Value::Float(0.0)),
            ("contrast".into(), Value::Float(0.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_threshold() {
    let Some(ctx) = gpu() else { return };
    // Bright gray (200/255 = ~0.78) > threshold 0.5 -> white
    let img = make_test_image(32, 32, 200, 200, 200, 255);
    let result = helpers::run_node_test(
        "threshold",
        HashMap::from([("threshold".into(), Value::Float(0.5))]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);

    // Pixel verification: should be white
    if let Some(Value::GpuImage(tex)) = result.get("image") {
        let pixels = tex.download_rgba(&ctx.device, &ctx.queue);
        let (r, g, b) = (pixels[0], pixels[1], pixels[2]);
        assert!(r >= 254, "threshold output R should be ~255, got {}", r);
        assert!(g >= 254, "threshold output G should be ~255, got {}", g);
        assert!(b >= 254, "threshold output B should be ~255, got {}", b);
    }
}

#[test]
fn test_levels() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "levels",
        HashMap::from([
            ("in_black".into(), Value::Float(0.0)),
            ("in_white".into(), Value::Float(1.0)),
            ("gamma".into(), Value::Float(1.0)),
            ("out_black".into(), Value::Float(0.0)),
            ("out_white".into(), Value::Float(1.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_hue_saturation() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "hue_saturation",
        HashMap::from([
            ("hue".into(), Value::Float(0.0)),
            ("saturation".into(), Value::Float(0.0)),
            ("lightness".into(), Value::Float(0.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_color_balance() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "color_balance",
        HashMap::from([
            ("shadows_r".into(), Value::Float(0.0)),
            ("shadows_g".into(), Value::Float(0.0)),
            ("shadows_b".into(), Value::Float(0.0)),
            ("midtones_r".into(), Value::Float(0.1)),
            ("midtones_g".into(), Value::Float(0.0)),
            ("midtones_b".into(), Value::Float(0.0)),
            ("highlights_r".into(), Value::Float(0.0)),
            ("highlights_g".into(), Value::Float(0.0)),
            ("highlights_b".into(), Value::Float(0.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

// =====================================================================
// Single-input filter nodes
// =====================================================================

#[test]
fn test_blur() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "blur",
        HashMap::from([
            ("radius".into(), Value::Float(2.0)),
            ("method".into(), Value::String("gaussian".into())),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_sharpen() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "sharpen",
        HashMap::from([
            ("amount".into(), Value::Float(1.0)),
            ("radius".into(), Value::Float(1.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_edge_detect() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "edge_detect",
        HashMap::from([("method".into(), Value::String("sobel".into()))]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_emboss() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "emboss",
        HashMap::from([
            ("strength".into(), Value::Float(1.0)),
            ("angle".into(), Value::Float(135.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_pixelate() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "pixelate",
        HashMap::from([("block_size".into(), Value::Int(4))]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_vignette() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 128, 64, 255);
    let result = helpers::run_node_test(
        "vignette",
        HashMap::from([
            ("strength".into(), Value::Float(0.5)),
            ("radius".into(), Value::Float(1.0)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_film_grain() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "film_grain",
        HashMap::from([
            ("amount".into(), Value::Float(0.3)),
            ("size".into(), Value::Float(1.0)),
            ("seed".into(), Value::Int(42)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_denoise() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "denoise",
        HashMap::from([("strength".into(), Value::Float(10.0))]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

// =====================================================================
// Single-input transform nodes
// =====================================================================

#[test]
fn test_resize() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(64, 64, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "resize",
        HashMap::from([
            ("width".into(), Value::Int(32)),
            ("height".into(), Value::Int(32)),
            ("method".into(), Value::String("bilinear".into())),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    // Pixel verification: 64x64 -> 32x32
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_crop() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(64, 64, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "crop",
        HashMap::from([
            ("x".into(), Value::Int(0)),
            ("y".into(), Value::Int(0)),
            ("width".into(), Value::Int(32)),
            ("height".into(), Value::Int(32)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_rotate() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "rotate",
        HashMap::from([
            ("angle".into(), Value::Float(90.0)),
            ("fill".into(), Value::Color([0.0, 0.0, 0.0, 0.0])),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_flip() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "flip",
        HashMap::from([("direction".into(), Value::String("horizontal".into()))]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_distort() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 128, 128, 128, 255);
    let result = helpers::run_node_test(
        "distort",
        HashMap::from([
            ("type".into(), Value::String("barrel".into())),
            ("strength".into(), Value::Float(0.5)),
        ]),
        HashMap::from([("image".into(), img)]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

// =====================================================================
// Multi-input nodes
// =====================================================================

#[test]
fn test_blend() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let result = helpers::run_node_test(
        "blend",
        HashMap::from([
            ("mode".into(), Value::String("normal".into())),
            ("opacity".into(), Value::Float(1.0)),
        ]),
        HashMap::from([
            ("base".into(), img.clone()),
            ("layer".into(), img),
        ]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_mask() {
    let Some(ctx) = gpu() else { return };
    let img = make_test_image(32, 32, 255, 0, 0, 255);
    let mask_img = make_test_image(32, 32, 255, 255, 255, 255);
    let result = helpers::run_node_test(
        "mask",
        HashMap::from([("invert".into(), Value::Boolean(false))]),
        HashMap::from([
            ("image".into(), img),
            ("mask".into(), mask_img),
        ]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}

#[test]
fn test_channel_merge() {
    let Some(ctx) = gpu() else { return };
    let red_ch = make_test_image(32, 32, 255, 255, 255, 255);
    let green_ch = make_test_image(32, 32, 128, 128, 128, 255);
    let blue_ch = make_test_image(32, 32, 0, 0, 0, 255);
    let alpha_ch = make_test_image(32, 32, 255, 255, 255, 255);
    let result = helpers::run_node_test(
        "channel_merge",
        HashMap::new(),
        HashMap::from([
            ("red".into(), red_ch),
            ("green".into(), green_ch),
            ("blue".into(), blue_ch),
            ("alpha".into(), alpha_ch),
        ]),
        Some(&ctx),
    );
    assert_gpu_image(&result, "image", 32, 32);
}
