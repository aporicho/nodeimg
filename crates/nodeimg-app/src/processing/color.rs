use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, Rgba};

/// Invert the colors of an image. Alpha is unchanged.
pub fn invert(img: &DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        out.put_pixel(
            x,
            y,
            Rgba([255 - pixel[0], 255 - pixel[1], 255 - pixel[2], pixel[3]]),
        );
    }
    DynamicImage::ImageRgba8(out)
}

/// Threshold: pixels with luminance >= threshold become white, others become black.
/// threshold_val is in 0..1 range.
pub fn threshold(img: &DynamicImage, threshold_val: f32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let v = if lum >= threshold_val { 255u8 } else { 0u8 };
        out.put_pixel(x, y, Rgba([v, v, v, pixel[3]]));
    }
    DynamicImage::ImageRgba8(out)
}

/// Levels adjustment.
/// in_black/in_white: input level range (0..1)
/// gamma: gamma correction factor
/// out_black/out_white: output level range (0..1)
pub fn levels(
    img: &DynamicImage,
    in_black: f32,
    in_white: f32,
    gamma: f32,
    out_black: f32,
    out_white: f32,
) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    let in_range = (in_white - in_black).max(1e-6);
    let gamma_inv = 1.0 / gamma.max(1e-6);

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let a = pixel[3];
        let mut channels = [0u8; 3];
        for (i, &ch) in pixel.0[..3].iter().enumerate() {
            let mut v = ch as f32 / 255.0;
            // Input map
            v = ((v - in_black) / in_range).clamp(0.0, 1.0);
            // Gamma
            v = v.powf(gamma_inv);
            // Output map
            v = out_black + v * (out_white - out_black);
            channels[i] = (v.clamp(0.0, 1.0) * 255.0) as u8;
        }
        out.put_pixel(x, y, Rgba([channels[0], channels[1], channels[2], a]));
    }
    DynamicImage::ImageRgba8(out)
}

// ---- HSL helpers ----

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;

    if delta < 1e-6 {
        return (0.0, 0.0, l);
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / delta).rem_euclid(6.0) * 60.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / delta + 2.0) * 60.0
    } else {
        ((r - g) / delta + 4.0) * 60.0
    };

    (h, s, l)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 1e-6 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;
    let r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h_norm);
    let b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);
    (r, g, b)
}

/// Adjust hue, saturation, and lightness.
/// hue_shift: degrees (-180..180 typical), saturation/lightness: additive (-1..1)
pub fn hsl_adjust(
    img: &DynamicImage,
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let a = pixel[3];

        let (h_val, s_val, l_val) = rgb_to_hsl(r, g, b);
        let new_h = (h_val + hue_shift).rem_euclid(360.0);
        let new_s = (s_val + saturation).clamp(0.0, 1.0);
        let new_l = (l_val + lightness).clamp(0.0, 1.0);
        let (nr, ng, nb) = hsl_to_rgb(new_h, new_s, new_l);

        out.put_pixel(
            x,
            y,
            Rgba([
                (nr.clamp(0.0, 1.0) * 255.0) as u8,
                (ng.clamp(0.0, 1.0) * 255.0) as u8,
                (nb.clamp(0.0, 1.0) * 255.0) as u8,
                a,
            ]),
        );
    }
    DynamicImage::ImageRgba8(out)
}

/// Color balance: adjust shadows, midtones, highlights separately.
/// Each array is [r_offset, g_offset, b_offset] in -1..1 range.
pub fn color_balance(
    img: &DynamicImage,
    shadows: [f32; 3],
    midtones: [f32; 3],
    highlights: [f32; 3],
) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let a = pixel[3];

        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let shadows_w = (1.0 - lum * 4.0).clamp(0.0, 1.0);
        let highlights_w = (lum * 4.0 - 3.0).clamp(0.0, 1.0);
        let midtones_w = 1.0 - shadows_w - highlights_w;

        let channels = [r, g, b];
        let mut result = [0u8; 3];
        for i in 0..3 {
            let v = channels[i]
                + shadows[i] * shadows_w
                + midtones[i] * midtones_w
                + highlights[i] * highlights_w;
            result[i] = (v.clamp(0.0, 1.0) * 255.0) as u8;
        }
        out.put_pixel(x, y, Rgba([result[0], result[1], result[2], a]));
    }
    DynamicImage::ImageRgba8(out)
}

// ---- LUT (3D lookup table) ----

/// A 3D LUT with `size^3` entries.
pub struct Lut3D {
    pub size: usize,
    pub data: Vec<[f32; 3]>, // size^3 entries, in B-G-R innermost order (cube file order)
}

/// Parse a .cube LUT file format.
pub fn parse_cube_lut(content: &str) -> Result<Lut3D, String> {
    let mut size: Option<usize> = None;
    let mut data: Vec<[f32; 3]> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("TITLE")
            || trimmed.starts_with("DOMAIN_MIN")
            || trimmed.starts_with("DOMAIN_MAX")
        {
            continue;
        }

        if trimmed.starts_with("LUT_3D_SIZE") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 2 {
                return Err("Invalid LUT_3D_SIZE line".into());
            }
            size = Some(
                parts[1]
                    .parse::<usize>()
                    .map_err(|e| format!("Invalid LUT size: {e}"))?,
            );
            continue;
        }

        // Try to parse as RGB float triple
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            let r = parts[0]
                .parse::<f32>()
                .map_err(|e| format!("Invalid float: {e}"))?;
            let g = parts[1]
                .parse::<f32>()
                .map_err(|e| format!("Invalid float: {e}"))?;
            let b = parts[2]
                .parse::<f32>()
                .map_err(|e| format!("Invalid float: {e}"))?;
            data.push([r, g, b]);
        }
    }

    let size = size.ok_or("Missing LUT_3D_SIZE")?;
    let expected = size * size * size;
    if data.len() != expected {
        return Err(format!(
            "Expected {} LUT entries, got {}",
            expected,
            data.len()
        ));
    }

    Ok(Lut3D { size, data })
}

/// Apply a 3D LUT to an image using trilinear interpolation.
/// intensity: blend factor (0=no change, 1=full LUT).
pub fn lut_apply(img: &DynamicImage, lut: &Lut3D, intensity: f32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();
    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);
    let size = lut.size;
    let scale = (size - 1) as f32;

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let r_in = pixel[0] as f32 / 255.0;
        let g_in = pixel[1] as f32 / 255.0;
        let b_in = pixel[2] as f32 / 255.0;
        let a = pixel[3];

        // Scale to LUT coordinates
        let rf = r_in * scale;
        let gf = g_in * scale;
        let bf = b_in * scale;

        let r0 = rf.floor() as usize;
        let g0 = gf.floor() as usize;
        let b0 = bf.floor() as usize;
        let r1 = (r0 + 1).min(size - 1);
        let g1 = (g0 + 1).min(size - 1);
        let b1 = (b0 + 1).min(size - 1);

        let dr = rf - r0 as f32;
        let dg = gf - g0 as f32;
        let db = bf - b0 as f32;

        // Cube file order: B changes fastest, then G, then R
        // index = r * size*size + g * size + b
        let idx = |ri: usize, gi: usize, bi: usize| ri * size * size + gi * size + bi;

        let c000 = lut.data[idx(r0, g0, b0)];
        let c001 = lut.data[idx(r0, g0, b1)];
        let c010 = lut.data[idx(r0, g1, b0)];
        let c011 = lut.data[idx(r0, g1, b1)];
        let c100 = lut.data[idx(r1, g0, b0)];
        let c101 = lut.data[idx(r1, g0, b1)];
        let c110 = lut.data[idx(r1, g1, b0)];
        let c111 = lut.data[idx(r1, g1, b1)];

        let mut lut_rgb = [0.0f32; 3];
        for i in 0..3 {
            lut_rgb[i] = c000[i] * (1.0 - dr) * (1.0 - dg) * (1.0 - db)
                + c001[i] * (1.0 - dr) * (1.0 - dg) * db
                + c010[i] * (1.0 - dr) * dg * (1.0 - db)
                + c011[i] * (1.0 - dr) * dg * db
                + c100[i] * dr * (1.0 - dg) * (1.0 - db)
                + c101[i] * dr * (1.0 - dg) * db
                + c110[i] * dr * dg * (1.0 - db)
                + c111[i] * dr * dg * db;
        }

        let inp = [r_in, g_in, b_in];
        let out_r = (inp[0] + (lut_rgb[0] - inp[0]) * intensity).clamp(0.0, 1.0);
        let out_g = (inp[1] + (lut_rgb[1] - inp[1]) * intensity).clamp(0.0, 1.0);
        let out_b = (inp[2] + (lut_rgb[2] - inp[2]) * intensity).clamp(0.0, 1.0);

        out.put_pixel(
            x,
            y,
            Rgba([
                (out_r * 255.0) as u8,
                (out_g * 255.0) as u8,
                (out_b * 255.0) as u8,
                a,
            ]),
        );
    }
    DynamicImage::ImageRgba8(out)
}

/// Adjust brightness, contrast, and saturation of an image.
/// - brightness: -1.0 to 1.0 (0 = no change)
/// - contrast: -1.0 to 1.0 (0 = no change)
/// - saturation: -1.0 to 1.0 (0 = no change)
pub fn color_adjust(
    img: &DynamicImage,
    brightness: f32,
    contrast: f32,
    saturation: f32,
) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();

    let mut out = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(w, h);

    // Contrast factor: map [-1, 1] to [0.5, 2.0]
    let contrast_factor = if contrast >= 0.0 {
        1.0 + contrast
    } else {
        1.0 + contrast * 0.5
    };

    // Saturation factor: map [-1, 1] to [0, 2]
    let sat_factor = 1.0 + saturation;

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let r = pixel[0] as f32 / 255.0;
        let g = pixel[1] as f32 / 255.0;
        let b = pixel[2] as f32 / 255.0;
        let a = pixel[3];

        // Brightness
        let r = r + brightness;
        let g = g + brightness;
        let b = b + brightness;

        // Contrast (around 0.5 midpoint)
        let r = (r - 0.5) * contrast_factor + 0.5;
        let g = (g - 0.5) * contrast_factor + 0.5;
        let b = (b - 0.5) * contrast_factor + 0.5;

        // Saturation (desaturate toward luminance)
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        let r = lum + (r - lum) * sat_factor;
        let g = lum + (g - lum) * sat_factor;
        let b = lum + (b - lum) * sat_factor;

        // Clamp and convert back
        let r = (r.clamp(0.0, 1.0) * 255.0) as u8;
        let g = (g.clamp(0.0, 1.0) * 255.0) as u8;
        let b = (b.clamp(0.0, 1.0) * 255.0) as u8;

        out.put_pixel(x, y, Rgba([r, g, b, a]));
    }

    DynamicImage::ImageRgba8(out)
}

/// Compute histogram: returns a Vec of channels, each a Vec<u32> of 256 bins.
/// channel: "rgb" → 3 arrays (R,G,B), "red"/"green"/"blue" → 1 array, "luminance" → 1 array
pub fn compute_histogram(img: &DynamicImage, channel: &str) -> Vec<Vec<u32>> {
    let rgba = img.to_rgba8();

    match channel {
        "red" => {
            let mut bins = vec![0u32; 256];
            for p in rgba.pixels() {
                bins[p[0] as usize] += 1;
            }
            vec![bins]
        }
        "green" => {
            let mut bins = vec![0u32; 256];
            for p in rgba.pixels() {
                bins[p[1] as usize] += 1;
            }
            vec![bins]
        }
        "blue" => {
            let mut bins = vec![0u32; 256];
            for p in rgba.pixels() {
                bins[p[2] as usize] += 1;
            }
            vec![bins]
        }
        "luminance" => {
            let mut bins = vec![0u32; 256];
            for p in rgba.pixels() {
                let r = p[0] as f32 / 255.0;
                let g = p[1] as f32 / 255.0;
                let b = p[2] as f32 / 255.0;
                let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                let idx = (lum.clamp(0.0, 1.0) * 255.0) as usize;
                bins[idx] += 1;
            }
            vec![bins]
        }
        // "rgb" and default
        _ => {
            let mut r_bins = vec![0u32; 256];
            let mut g_bins = vec![0u32; 256];
            let mut b_bins = vec![0u32; 256];
            for p in rgba.pixels() {
                r_bins[p[0] as usize] += 1;
                g_bins[p[1] as usize] += 1;
                b_bins[p[2] as usize] += 1;
            }
            vec![r_bins, g_bins, b_bins]
        }
    }
}

/// Render histogram data as a 256x128 image.
///
/// Background: dark gray (32,32,32).
/// For "rgb": draws R in red, G in green, B in blue.
/// For "red"/"green"/"blue": draws in the corresponding color.
/// For "luminance": draws in white.
/// Bars are normalized to the maximum bin value.
pub fn render_histogram_image(histograms: &[Vec<u32>], channel: &str) -> DynamicImage {
    let width = 256u32;
    let height = 128u32;

    let mut out = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_pixel(width, height, Rgb([32, 32, 32]));

    if histograms.is_empty() {
        return DynamicImage::ImageRgb8(out);
    }

    // Find global max across all channel histograms for normalization
    let max_val = histograms
        .iter()
        .flat_map(|h| h.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);

    let channel_colors: Vec<[u8; 3]> = match channel {
        "red" => vec![[255, 64, 64]],
        "green" => vec![[64, 255, 64]],
        "blue" => vec![[64, 64, 255]],
        "luminance" => vec![[255, 255, 255]],
        // "rgb" and default: R, G, B
        _ => vec![[255, 64, 64], [64, 255, 64], [64, 64, 255]],
    };

    // Draw each channel's histogram as vertical bars (semi-transparent by averaging)
    for (ch_idx, bins) in histograms.iter().enumerate() {
        let color = channel_colors
            .get(ch_idx)
            .copied()
            .unwrap_or([255, 255, 255]);

        for (x, &count) in bins.iter().enumerate() {
            let normalized = count as f32 / max_val as f32;
            let bar_height = (normalized * height as f32).round() as u32;

            for row in 0..bar_height {
                let y = height - 1 - row;
                let current = out.get_pixel(x as u32, y);
                // Blend: average existing pixel with new color (additive clamped)
                let new_r = (current[0] as u16 + color[0] as u16).min(255) as u8;
                let new_g = (current[1] as u16 + color[1] as u16).min(255) as u8;
                let new_b = (current[2] as u16 + color[2] as u16).min(255) as u8;
                out.put_pixel(x as u32, y, Rgb([new_r, new_g, new_b]));
            }
        }
    }

    DynamicImage::ImageRgb8(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn make_test_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        let buf = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, a]));
        DynamicImage::ImageRgba8(buf)
    }

    #[test]
    fn identity_no_change() {
        let img = make_test_image(4, 4, 128, 64, 32, 255);
        let out = color_adjust(&img, 0.0, 0.0, 0.0);
        let orig = img.to_rgba8();
        let result = out.to_rgba8();
        for (p_orig, p_out) in orig.pixels().zip(result.pixels()) {
            // Allow ±1 rounding error from f32 arithmetic
            for ch in 0..3 {
                let diff = (p_orig[ch] as i16 - p_out[ch] as i16).unsigned_abs();
                assert!(
                    diff <= 1,
                    "channel {ch}: expected ~{}, got {}",
                    p_orig[ch],
                    p_out[ch]
                );
            }
            assert_eq!(p_orig[3], p_out[3], "alpha must be preserved exactly");
        }
    }

    #[test]
    fn test_invert_pixel() {
        let img = make_test_image(1, 1, 100, 150, 200, 255);
        let out = invert(&img).to_rgba8();
        let p = out.get_pixel(0, 0);
        assert_eq!(p[0], 155, "R: 255-100=155");
        assert_eq!(p[1], 105, "G: 255-150=105");
        assert_eq!(p[2], 55, "B: 255-200=55");
        assert_eq!(p[3], 255, "alpha unchanged");
    }

    #[test]
    fn test_invert_alpha_preserved() {
        let img = make_test_image(2, 2, 0, 0, 0, 128);
        let out = invert(&img).to_rgba8();
        for p in out.pixels() {
            assert_eq!(p[0], 255);
            assert_eq!(p[3], 128, "alpha must be preserved");
        }
    }

    #[test]
    fn test_threshold_bright() {
        // bright pixel → white
        let img = make_test_image(1, 1, 200, 200, 200, 255);
        let out = threshold(&img, 0.5).to_rgba8();
        let p = out.get_pixel(0, 0);
        assert_eq!(p[0], 255);
        assert_eq!(p[1], 255);
        assert_eq!(p[2], 255);
    }

    #[test]
    fn test_threshold_dark() {
        // dark pixel → black
        let img = make_test_image(1, 1, 10, 10, 10, 255);
        let out = threshold(&img, 0.5).to_rgba8();
        let p = out.get_pixel(0, 0);
        assert_eq!(p[0], 0);
        assert_eq!(p[1], 0);
        assert_eq!(p[2], 0);
    }

    #[test]
    fn test_levels_identity() {
        let img = make_test_image(4, 4, 128, 64, 200, 255);
        let out = levels(&img, 0.0, 1.0, 1.0, 0.0, 1.0);
        let orig = img.to_rgba8();
        let result = out.to_rgba8();
        for (p_orig, p_out) in orig.pixels().zip(result.pixels()) {
            for ch in 0..3 {
                let diff = (p_orig[ch] as i16 - p_out[ch] as i16).unsigned_abs();
                assert!(
                    diff <= 1,
                    "channel {ch}: expected ~{}, got {}",
                    p_orig[ch],
                    p_out[ch]
                );
            }
            assert_eq!(p_orig[3], p_out[3]);
        }
    }

    #[test]
    fn test_rgb_to_hsl_red() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        assert!((h - 0.0).abs() < 1.0, "hue of red should be ~0, got {h}");
        assert!((s - 1.0).abs() < 0.01, "sat of red should be 1, got {s}");
        assert!(
            (l - 0.5).abs() < 0.01,
            "lightness of red should be 0.5, got {l}"
        );
    }

    #[test]
    fn test_rgb_to_hsl_gray() {
        let (_, s, l) = rgb_to_hsl(0.5, 0.5, 0.5);
        assert!(s < 0.01, "sat of gray should be 0, got {s}");
        assert!(
            (l - 0.5).abs() < 0.01,
            "lightness of gray should be 0.5, got {l}"
        );
    }

    #[test]
    fn test_hsl_roundtrip() {
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 1.0).abs() < 0.01, "r roundtrip, got {r}");
        assert!(g.abs() < 0.01, "g roundtrip, got {g}");
        assert!(b.abs() < 0.01, "b roundtrip, got {b}");
    }

    #[test]
    fn test_hsl_adjust_identity() {
        let img = make_test_image(4, 4, 128, 64, 200, 255);
        let out = hsl_adjust(&img, 0.0, 0.0, 0.0);
        let orig = img.to_rgba8();
        let result = out.to_rgba8();
        for (p_orig, p_out) in orig.pixels().zip(result.pixels()) {
            for ch in 0..3 {
                let diff = (p_orig[ch] as i16 - p_out[ch] as i16).unsigned_abs();
                assert!(
                    diff <= 2,
                    "channel {ch}: expected ~{}, got {}",
                    p_orig[ch],
                    p_out[ch]
                );
            }
            assert_eq!(p_orig[3], p_out[3]);
        }
    }

    #[test]
    fn test_color_balance_identity() {
        let img = make_test_image(4, 4, 128, 64, 200, 255);
        let out = color_balance(&img, [0.0; 3], [0.0; 3], [0.0; 3]);
        let orig = img.to_rgba8();
        let result = out.to_rgba8();
        for (p_orig, p_out) in orig.pixels().zip(result.pixels()) {
            for ch in 0..3 {
                let diff = (p_orig[ch] as i16 - p_out[ch] as i16).unsigned_abs();
                assert!(
                    diff <= 1,
                    "channel {ch}: expected ~{}, got {}",
                    p_orig[ch],
                    p_out[ch]
                );
            }
            assert_eq!(p_orig[3], p_out[3]);
        }
    }

    #[test]
    fn test_lut_identity() {
        // Build an identity LUT of size 2
        let size = 2usize;
        let mut data = Vec::with_capacity(size * size * size);
        for r in 0..size {
            for g in 0..size {
                for b in 0..size {
                    data.push([
                        r as f32 / (size - 1) as f32,
                        g as f32 / (size - 1) as f32,
                        b as f32 / (size - 1) as f32,
                    ]);
                }
            }
        }
        let lut = Lut3D { size, data };

        let img = make_test_image(2, 2, 0, 128, 255, 255);
        let out = lut_apply(&img, &lut, 1.0);
        let orig = img.to_rgba8();
        let result = out.to_rgba8();
        for (p_orig, p_out) in orig.pixels().zip(result.pixels()) {
            for ch in 0..3 {
                let diff = (p_orig[ch] as i16 - p_out[ch] as i16).unsigned_abs();
                assert!(
                    diff <= 2,
                    "channel {ch}: expected ~{}, got {}",
                    p_orig[ch],
                    p_out[ch]
                );
            }
        }
    }

    #[test]
    fn test_parse_cube_lut() {
        let cube = "# test\nTITLE \"test\"\nDOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\nLUT_3D_SIZE 2\n0.0 0.0 0.0\n0.0 0.0 1.0\n0.0 1.0 0.0\n0.0 1.0 1.0\n1.0 0.0 0.0\n1.0 0.0 1.0\n1.0 1.0 0.0\n1.0 1.0 1.0\n";
        let lut = parse_cube_lut(cube).expect("parse failed");
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);
    }

    #[test]
    fn test_histogram_solid_gray_rgb() {
        // 4x4 solid (128,128,128) → each of R, G, B has 16 at bin 128
        let img = make_test_image(4, 4, 128, 128, 128, 255);
        let hists = compute_histogram(&img, "rgb");
        assert_eq!(hists.len(), 3, "rgb should return 3 channels");
        for (ch, hist) in hists.iter().enumerate() {
            assert_eq!(
                hist[128], 16,
                "channel {ch}: expected 16 at bin 128, got {}",
                hist[128]
            );
            let total: u32 = hist.iter().sum();
            assert_eq!(total, 16, "channel {ch}: total pixel count should be 16");
        }
    }

    #[test]
    fn test_histogram_single_channel() {
        let img = make_test_image(4, 4, 200, 100, 50, 255);
        let r_hist = compute_histogram(&img, "red");
        assert_eq!(r_hist.len(), 1);
        assert_eq!(r_hist[0][200], 16, "red channel bin 200 should have 16");

        let g_hist = compute_histogram(&img, "green");
        assert_eq!(g_hist[0][100], 16, "green channel bin 100 should have 16");

        let b_hist = compute_histogram(&img, "blue");
        assert_eq!(b_hist[0][50], 16, "blue channel bin 50 should have 16");
    }

    #[test]
    fn test_histogram_luminance() {
        let img = make_test_image(4, 4, 128, 128, 128, 255);
        let lum_hist = compute_histogram(&img, "luminance");
        assert_eq!(lum_hist.len(), 1, "luminance returns 1 channel");
        let total: u32 = lum_hist[0].iter().sum();
        assert_eq!(total, 16, "total pixel count should be 16");
    }

    #[test]
    fn test_render_histogram_image_dimensions() {
        let img = make_test_image(4, 4, 128, 64, 200, 255);
        let hists = compute_histogram(&img, "rgb");
        let rendered = render_histogram_image(&hists, "rgb");
        assert_eq!(rendered.width(), 256);
        assert_eq!(rendered.height(), 128);
    }

    #[test]
    fn test_render_histogram_image_empty() {
        let rendered = render_histogram_image(&[], "rgb");
        assert_eq!(rendered.width(), 256);
        assert_eq!(rendered.height(), 128);
    }
}
