use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

/// Blend two images together using the given mode and opacity.
///
/// `layer` is resized to match `base` dimensions if they differ.
/// `opacity` is in 0.0..1.0 (0 = only base, 1 = full blend).
pub fn blend(base: &DynamicImage, layer: &DynamicImage, mode: &str, opacity: f32) -> DynamicImage {
    let (w, h) = base.dimensions();
    let base_rgba = base.to_rgba8();

    // Resize layer if needed
    let layer_resized;
    let layer_rgba = if layer.dimensions() != (w, h) {
        layer_resized = layer.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
        layer_resized.to_rgba8()
    } else {
        layer.to_rgba8()
    };

    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    for (x, y, base_pixel) in base_rgba.enumerate_pixels() {
        let layer_pixel = layer_rgba.get_pixel(x, y);

        let br = base_pixel[0] as f32 / 255.0;
        let bg = base_pixel[1] as f32 / 255.0;
        let bb = base_pixel[2] as f32 / 255.0;
        let ba = base_pixel[3];

        let lr = layer_pixel[0] as f32 / 255.0;
        let lg = layer_pixel[1] as f32 / 255.0;
        let lb = layer_pixel[2] as f32 / 255.0;

        let (blended_r, blended_g, blended_b) = match mode {
            "multiply" => (br * lr, bg * lg, bb * lb),
            "screen" => (
                1.0 - (1.0 - br) * (1.0 - lr),
                1.0 - (1.0 - bg) * (1.0 - lg),
                1.0 - (1.0 - bb) * (1.0 - lb),
            ),
            "overlay" => {
                let r = if br < 0.5 {
                    2.0 * br * lr
                } else {
                    1.0 - 2.0 * (1.0 - br) * (1.0 - lr)
                };
                let g = if bg < 0.5 {
                    2.0 * bg * lg
                } else {
                    1.0 - 2.0 * (1.0 - bg) * (1.0 - lg)
                };
                let b = if bb < 0.5 {
                    2.0 * bb * lb
                } else {
                    1.0 - 2.0 * (1.0 - bb) * (1.0 - lb)
                };
                (r, g, b)
            }
            "add" => (br + lr, bg + lg, bb + lb),
            "subtract" => (br - lr, bg - lg, bb - lb),
            "difference" => ((br - lr).abs(), (bg - lg).abs(), (bb - lb).abs()),
            // "normal" and default
            _ => (lr, lg, lb),
        };

        let r = (br * (1.0 - opacity) + blended_r * opacity).clamp(0.0, 1.0);
        let g = (bg * (1.0 - opacity) + blended_g * opacity).clamp(0.0, 1.0);
        let b = (bb * (1.0 - opacity) + blended_b * opacity).clamp(0.0, 1.0);

        out.put_pixel(
            x,
            y,
            Rgba([(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8, ba]),
        );
    }

    DynamicImage::ImageRgba8(out)
}

/// Apply a mask image to set the alpha channel of `img`.
///
/// The mask's luminance determines alpha. If `invert_mask` is true, alpha = 255 - luminance.
/// Mask is resized to match `img` dimensions if needed.
pub fn mask_apply(img: &DynamicImage, mask: &DynamicImage, invert_mask: bool) -> DynamicImage {
    let (w, h) = img.dimensions();
    let img_rgba = img.to_rgba8();

    // Resize mask if needed
    let mask_resized;
    let mask_rgba = if mask.dimensions() != (w, h) {
        mask_resized = mask.resize_exact(w, h, image::imageops::FilterType::Lanczos3);
        mask_resized.to_rgba8()
    } else {
        mask.to_rgba8()
    };

    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    for (x, y, img_pixel) in img_rgba.enumerate_pixels() {
        let mask_pixel = mask_rgba.get_pixel(x, y);

        let mr = mask_pixel[0] as f32 / 255.0;
        let mg = mask_pixel[1] as f32 / 255.0;
        let mb = mask_pixel[2] as f32 / 255.0;
        let luminance = (0.2126 * mr + 0.7152 * mg + 0.0722 * mb).clamp(0.0, 1.0);
        let mask_alpha = (luminance * 255.0) as u8;

        let alpha = if invert_mask {
            255 - mask_alpha
        } else {
            mask_alpha
        };

        out.put_pixel(
            x,
            y,
            Rgba([img_pixel[0], img_pixel[1], img_pixel[2], alpha]),
        );
    }

    DynamicImage::ImageRgba8(out)
}

/// Merge separate channel images into a single RGBA image.
///
/// Each channel uses its red channel value (or luminance for multi-channel inputs).
/// If a channel is None it defaults to 255. Returns None only if all channels are None.
pub fn channel_merge(
    red: Option<&DynamicImage>,
    green: Option<&DynamicImage>,
    blue: Option<&DynamicImage>,
    alpha: Option<&DynamicImage>,
) -> Option<DynamicImage> {
    // Find dimensions from the first non-None channel
    let (w, h) = red
        .or(green)
        .or(blue)
        .or(alpha)
        .map(|img| img.dimensions())?;

    let resize = |img: &DynamicImage| -> image::RgbaImage {
        if img.dimensions() != (w, h) {
            img.resize_exact(w, h, image::imageops::FilterType::Lanczos3)
                .to_rgba8()
        } else {
            img.to_rgba8()
        }
    };

    let red_buf = red.map(resize);
    let green_buf = green.map(resize);
    let blue_buf = blue.map(resize);
    let alpha_buf = alpha.map(resize);

    let channel_value = |buf: &Option<image::RgbaImage>, x: u32, y: u32| -> u8 {
        match buf {
            Some(img) => {
                let p = img.get_pixel(x, y);
                // Use red channel (first channel) to represent the mask value
                p[0]
            }
            None => 255,
        }
    };

    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = channel_value(&red_buf, x, y);
            let g = channel_value(&green_buf, x, y);
            let b = channel_value(&blue_buf, x, y);
            let a = channel_value(&alpha_buf, x, y);
            out.put_pixel(x, y, Rgba([r, g, b, a]));
        }
    }

    Some(DynamicImage::ImageRgba8(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn make_test_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        let buf = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, a]));
        DynamicImage::ImageRgba8(buf)
    }

    #[test]
    fn test_blend_normal_opacity_1_equals_layer() {
        let base = make_test_image(2, 2, 100, 100, 100, 255);
        let layer = make_test_image(2, 2, 200, 50, 150, 255);
        let result = blend(&base, &layer, "normal", 1.0).to_rgba8();
        for p in result.pixels() {
            assert_eq!(p[0], 200, "R should equal layer R");
            assert_eq!(p[1], 50, "G should equal layer G");
            assert_eq!(p[2], 150, "B should equal layer B");
        }
    }

    #[test]
    fn test_blend_normal_opacity_0_equals_base() {
        let base = make_test_image(2, 2, 100, 100, 100, 255);
        let layer = make_test_image(2, 2, 200, 50, 150, 255);
        let result = blend(&base, &layer, "normal", 0.0).to_rgba8();
        for p in result.pixels() {
            assert_eq!(p[0], 100, "R should equal base R");
            assert_eq!(p[1], 100, "G should equal base G");
            assert_eq!(p[2], 100, "B should equal base B");
        }
    }

    #[test]
    fn test_blend_multiply() {
        // 128/255 * 128/255 ≈ 0.2518, * 255 ≈ 64
        let base = make_test_image(1, 1, 128, 128, 128, 255);
        let layer = make_test_image(1, 1, 128, 128, 128, 255);
        let result = blend(&base, &layer, "multiply", 1.0).to_rgba8();
        let p = result.get_pixel(0, 0);
        // 0.5020 * 0.5020 = 0.2520, * 255 = 64.26 ≈ 64
        assert!((p[0] as i16 - 64).abs() <= 2, "multiply R: got {}", p[0]);
    }

    #[test]
    fn test_blend_screen() {
        // 1 - (1-0.5)*(1-0.5) = 1 - 0.25 = 0.75, * 255 ≈ 191
        let base = make_test_image(1, 1, 128, 128, 128, 255);
        let layer = make_test_image(1, 1, 128, 128, 128, 255);
        let result = blend(&base, &layer, "screen", 1.0).to_rgba8();
        let p = result.get_pixel(0, 0);
        assert!((p[0] as i16 - 191).abs() <= 2, "screen R: got {}", p[0]);
    }

    #[test]
    fn test_mask_white_preserves_alpha() {
        let img = make_test_image(2, 2, 100, 100, 100, 200);
        let mask = make_test_image(2, 2, 255, 255, 255, 255);
        let result = mask_apply(&img, &mask, false).to_rgba8();
        for p in result.pixels() {
            assert_eq!(p[3], 255, "white mask → alpha=255");
        }
    }

    #[test]
    fn test_mask_black_zeroes_alpha() {
        let img = make_test_image(2, 2, 100, 100, 100, 255);
        let mask = make_test_image(2, 2, 0, 0, 0, 255);
        let result = mask_apply(&img, &mask, false).to_rgba8();
        for p in result.pixels() {
            assert_eq!(p[3], 0, "black mask → alpha=0");
        }
    }

    #[test]
    fn test_mask_invert() {
        let img = make_test_image(2, 2, 100, 100, 100, 255);
        let mask = make_test_image(2, 2, 255, 255, 255, 255);
        let result = mask_apply(&img, &mask, true).to_rgba8();
        for p in result.pixels() {
            assert_eq!(p[3], 0, "inverted white mask → alpha=0");
        }
    }

    #[test]
    fn test_channel_merge_all_white() {
        let white = make_test_image(2, 2, 255, 255, 255, 255);
        let result = channel_merge(Some(&white), Some(&white), Some(&white), Some(&white))
            .expect("should produce image");
        let rgba = result.to_rgba8();
        for p in rgba.pixels() {
            assert_eq!(p[0], 255);
            assert_eq!(p[1], 255);
            assert_eq!(p[2], 255);
            assert_eq!(p[3], 255);
        }
    }

    #[test]
    fn test_channel_merge_all_none_returns_none() {
        assert!(channel_merge(None, None, None, None).is_none());
    }

    #[test]
    fn test_channel_merge_defaults_to_255() {
        let red_ch = make_test_image(2, 2, 100, 100, 100, 255);
        // Only provide red channel; green, blue, alpha should default to 255
        let result = channel_merge(Some(&red_ch), None, None, None).expect("should produce image");
        let rgba = result.to_rgba8();
        for p in rgba.pixels() {
            assert_eq!(p[0], 100, "red channel from input");
            assert_eq!(p[1], 255, "green defaults to 255");
            assert_eq!(p[2], 255, "blue defaults to 255");
            assert_eq!(p[3], 255, "alpha defaults to 255");
        }
    }
}
