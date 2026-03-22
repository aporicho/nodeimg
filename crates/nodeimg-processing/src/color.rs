use image::{DynamicImage, ImageBuffer, Rgb};
#[cfg(test)]
use image::Rgba;

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
