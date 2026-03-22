use image::{DynamicImage, ImageBuffer, Luma, Rgba};
#[cfg(test)]
use image::GenericImageView;

/// Generate a solid color image. color is [r, g, b, a] each in 0.0..1.0.
pub fn solid_color(width: u32, height: u32, color: [f32; 4]) -> DynamicImage {
    let r = (color[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (color[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (color[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = (color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
    let buf = ImageBuffer::from_pixel(width, height, Rgba([r, g, b, a]));
    DynamicImage::ImageRgba8(buf)
}

/// Generate a gradient image between two colors.
/// direction: "horizontal" | "vertical" | "diagonal"
pub fn gradient(
    width: u32,
    height: u32,
    color_start: [f32; 4],
    color_end: [f32; 4],
    direction: &str,
) -> DynamicImage {
    let buf = ImageBuffer::from_fn(width, height, |x, y| {
        let t = match direction {
            "vertical" => {
                if height <= 1 {
                    0.0f32
                } else {
                    y as f32 / (height - 1) as f32
                }
            }
            "diagonal" => {
                let tx = if width <= 1 {
                    0.0f32
                } else {
                    x as f32 / (width - 1) as f32
                };
                let ty = if height <= 1 {
                    0.0f32
                } else {
                    y as f32 / (height - 1) as f32
                };
                (tx + ty) / 2.0
            }
            // "horizontal" is the default
            _ => {
                if width <= 1 {
                    0.0f32
                } else {
                    x as f32 / (width - 1) as f32
                }
            }
        };

        let lerp = |a: f32, b: f32, t: f32| -> u8 {
            ((a + (b - a) * t).clamp(0.0, 1.0) * 255.0).round() as u8
        };

        Rgba([
            lerp(color_start[0], color_end[0], t),
            lerp(color_start[1], color_end[1], t),
            lerp(color_start[2], color_end[2], t),
            lerp(color_start[3], color_end[3], t),
        ])
    });
    DynamicImage::ImageRgba8(buf)
}

/// Generate a checkerboard pattern image.
pub fn checkerboard(
    width: u32,
    height: u32,
    color_a: [f32; 4],
    color_b: [f32; 4],
    cell_size: u32,
) -> DynamicImage {
    let cell = cell_size.max(1);
    let to_rgba = |c: [f32; 4]| -> Rgba<u8> {
        Rgba([
            (c[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (c[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (c[2].clamp(0.0, 1.0) * 255.0).round() as u8,
            (c[3].clamp(0.0, 1.0) * 255.0).round() as u8,
        ])
    };
    let px_a = to_rgba(color_a);
    let px_b = to_rgba(color_b);

    let buf = ImageBuffer::from_fn(width, height, |x, y| {
        if (x / cell + y / cell).is_multiple_of(2) {
            px_a
        } else {
            px_b
        }
    });
    DynamicImage::ImageRgba8(buf)
}

/// Generate a classic 2D Perlin noise image (grayscale).
/// seed: seed for permutation table. scale: noise frequency scale.
/// Output: grayscale image with noise values mapped from (-1..1) to (0..255).
pub fn perlin_noise(width: u32, height: u32, seed: i32, scale: f32) -> DynamicImage {
    let perm = build_permutation(seed);

    let buf = ImageBuffer::from_fn(width, height, |x, y| {
        let nx = x as f32 / width as f32 * scale;
        let ny = y as f32 / height as f32 * scale;
        let n = noise2d(&perm, nx, ny);
        // map -1..1 to 0..255
        let v = ((n * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        Luma([v])
    });
    DynamicImage::ImageLuma8(buf)
}

// --- Perlin noise internals ---

fn build_permutation(seed: i32) -> [u8; 512] {
    let mut p: [u8; 256] = [0u8; 256];
    for (i, v) in p.iter_mut().enumerate() {
        *v = i as u8;
    }
    // Simple LCG shuffle seeded by seed
    let mut rng = seed.wrapping_mul(1664525).wrapping_add(1013904223) as u32;
    for i in (1..256usize).rev() {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let j = (rng >> 16) as usize % (i + 1);
        p.swap(i, j);
    }
    let mut perm = [0u8; 512];
    for i in 0..512 {
        perm[i] = p[i & 255];
    }
    perm
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

fn grad2(hash: u8, x: f32, y: f32) -> f32 {
    match hash & 3 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

fn noise2d(perm: &[u8; 512], x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32 & 255;
    let yi = y.floor() as i32 & 255;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = fade(xf);
    let v = fade(yf);

    let aa = perm[perm[xi as usize] as usize + yi as usize];
    let ab = perm[perm[xi as usize] as usize + yi as usize + 1];
    let ba = perm[perm[xi as usize + 1] as usize + yi as usize];
    let bb = perm[perm[xi as usize + 1] as usize + yi as usize + 1];

    let g_aa = grad2(aa, xf, yf);
    let g_ba = grad2(ba, xf - 1.0, yf);
    let g_ab = grad2(ab, xf, yf - 1.0);
    let g_bb = grad2(bb, xf - 1.0, yf - 1.0);

    lerp(lerp(g_aa, g_ba, u), lerp(g_ab, g_bb, u), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn make_test_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        let buf = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, a]));
        DynamicImage::ImageRgba8(buf)
    }

    #[test]
    fn test_solid_color_dimensions() {
        let img = solid_color(64, 32, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_solid_color_pixel_value() {
        let img = solid_color(4, 4, [1.0, 0.5, 0.0, 1.0]);
        let pixel = img.get_pixel(0, 0);
        // r=255, g≈128, b=0, a=255
        assert_eq!(pixel[0], 255);
        assert!((pixel[1] as i32 - 128).abs() <= 1);
        assert_eq!(pixel[2], 0);
        assert_eq!(pixel[3], 255);
    }

    #[test]
    fn test_solid_color_uses_make_test_image() {
        let _base = make_test_image(4, 4, 255, 0, 0, 255);
        let img = solid_color(4, 4, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);
    }

    #[test]
    fn test_gradient_horizontal_dimensions() {
        let img = gradient(
            64,
            32,
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            "horizontal",
        );
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn test_gradient_horizontal_endpoints() {
        let img = gradient(
            64,
            4,
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            "horizontal",
        );
        // left edge should be black
        let left = img.get_pixel(0, 0);
        assert_eq!(left[0], 0);
        // right edge should be white
        let right = img.get_pixel(63, 0);
        assert_eq!(right[0], 255);
    }

    #[test]
    fn test_gradient_vertical_endpoints() {
        let img = gradient(
            4,
            64,
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            "vertical",
        );
        let top = img.get_pixel(0, 0);
        assert_eq!(top[0], 0);
        let bottom = img.get_pixel(0, 63);
        assert_eq!(bottom[0], 255);
    }

    #[test]
    fn test_gradient_diagonal() {
        let img = gradient(4, 4, [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0], "diagonal");
        // top-left corner should be darkest
        let tl = img.get_pixel(0, 0);
        let br = img.get_pixel(3, 3);
        assert!(br[0] > tl[0]);
    }

    #[test]
    fn test_checkerboard_dimensions() {
        let img = checkerboard(64, 64, [1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0], 16);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }

    #[test]
    fn test_checkerboard_pattern() {
        // With cell_size=8, pixel (0,0) is in cell (0,0) => color_a (white)
        // pixel (8,0) is in cell (1,0) => (1+0)%2=1 => color_b (black)
        let img = checkerboard(32, 32, [1.0, 1.0, 1.0, 1.0], [0.0, 0.0, 0.0, 1.0], 8);
        let p00 = img.get_pixel(0, 0);
        assert_eq!(p00[0], 255); // white
        let p80 = img.get_pixel(8, 0);
        assert_eq!(p80[0], 0); // black
    }

    #[test]
    fn test_perlin_noise_dimensions() {
        let img = perlin_noise(64, 64, 42, 4.0);
        assert_eq!(img.width(), 64);
        assert_eq!(img.height(), 64);
    }

    #[test]
    fn test_perlin_noise_is_grayscale() {
        let img = perlin_noise(16, 16, 0, 1.0);
        // DynamicImage::ImageLuma8 has 1 channel pixels
        assert!(matches!(img, DynamicImage::ImageLuma8(_)));
    }

    #[test]
    fn test_perlin_noise_different_seeds_differ() {
        let img1 = perlin_noise(32, 32, 0, 4.0);
        let img2 = perlin_noise(32, 32, 12345, 4.0);
        // They should produce different output
        let p1 = img1.get_pixel(10, 10);
        let p2 = img2.get_pixel(10, 10);
        // Not guaranteed to differ at every pixel, but very likely at some
        let any_diff = (0..32u32)
            .flat_map(|x| (0..32u32).map(move |y| (x, y)))
            .any(|(x, y)| img1.get_pixel(x, y) != img2.get_pixel(x, y));
        assert!(any_diff, "different seeds produced identical noise images");
        let _ = (p1, p2);
    }
}
