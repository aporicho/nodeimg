use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

/// Resize an image to the given dimensions using the specified filter method.
pub fn resize(img: &DynamicImage, width: u32, height: u32, method: &str) -> DynamicImage {
    let filter = match method {
        "nearest" => image::imageops::FilterType::Nearest,
        "bilinear" => image::imageops::FilterType::Triangle,
        "bicubic" => image::imageops::FilterType::CatmullRom,
        "lanczos3" => image::imageops::FilterType::Lanczos3,
        _ => image::imageops::FilterType::Triangle,
    };
    img.resize_exact(width, height, filter)
}

/// Crop an image to the given region.  x+width and y+height are clamped to the
/// image bounds so no out-of-bounds access can occur.
pub fn crop(img: &DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    let (img_w, img_h) = img.dimensions();
    let x = x.min(img_w.saturating_sub(1));
    let y = y.min(img_h.saturating_sub(1));
    let width = width.min(img_w - x);
    let height = height.min(img_h - y);
    img.crop_imm(x, y, width, height)
}

/// Rotate an image by `angle` degrees.
///
/// 90° multiples use the fast lossless paths from the `image` crate.  For
/// arbitrary angles an RGBA canvas is created and filled via inverse-mapping
/// with bilinear interpolation; regions outside the source are filled with
/// `fill_color`.
pub fn rotate(img: &DynamicImage, angle: f32, fill_color: [f32; 4]) -> DynamicImage {
    // Normalise to [0, 360)
    let angle = ((angle % 360.0) + 360.0) % 360.0;

    // Fast paths for cardinal angles (within 0.5° tolerance)
    if (angle - 0.0).abs() < 0.5 || (angle - 360.0).abs() < 0.5 {
        return img.clone();
    }
    if (angle - 90.0).abs() < 0.5 {
        return img.rotate90();
    }
    if (angle - 180.0).abs() < 0.5 {
        return img.rotate180();
    }
    if (angle - 270.0).abs() < 0.5 {
        return img.rotate270();
    }

    // Arbitrary angle via inverse mapping
    let (src_w, src_h) = img.dimensions();
    let rgba = img.to_rgba8();

    let fill = [
        (fill_color[0] * 255.0) as u8,
        (fill_color[1] * 255.0) as u8,
        (fill_color[2] * 255.0) as u8,
        (fill_color[3] * 255.0) as u8,
    ];

    let rad = -angle.to_radians(); // negative → rotate src CCW == rotate canvas CW
    let cos = rad.cos();
    let sin = rad.sin();

    let cx_src = (src_w as f32 - 1.0) / 2.0;
    let cy_src = (src_h as f32 - 1.0) / 2.0;
    let cx_dst = (src_w as f32 - 1.0) / 2.0;
    let cy_dst = (src_h as f32 - 1.0) / 2.0;

    let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(src_w, src_h, Rgba(fill));

    for dy in 0..src_h {
        for dx in 0..src_w {
            // Map destination pixel back to source space (inverse rotation)
            let fx = (dx as f32 - cx_dst) * cos - (dy as f32 - cy_dst) * sin + cx_src;
            let fy = (dx as f32 - cx_dst) * sin + (dy as f32 - cy_dst) * cos + cy_src;

            if fx < 0.0 || fy < 0.0 || fx >= src_w as f32 - 1.0 || fy >= src_h as f32 - 1.0 {
                continue; // leave fill
            }

            // Bilinear interpolation
            let x0 = fx.floor() as u32;
            let y0 = fy.floor() as u32;
            let x1 = (x0 + 1).min(src_w - 1);
            let y1 = (y0 + 1).min(src_h - 1);
            let tx = fx - x0 as f32;
            let ty = fy - y0 as f32;

            let p00 = rgba.get_pixel(x0, y0).0;
            let p10 = rgba.get_pixel(x1, y0).0;
            let p01 = rgba.get_pixel(x0, y1).0;
            let p11 = rgba.get_pixel(x1, y1).0;

            let mut out = [0u8; 4];
            for i in 0..4 {
                let v = p00[i] as f32 * (1.0 - tx) * (1.0 - ty)
                    + p10[i] as f32 * tx * (1.0 - ty)
                    + p01[i] as f32 * (1.0 - tx) * ty
                    + p11[i] as f32 * tx * ty;
                out[i] = v.round() as u8;
            }
            buf.put_pixel(dx, dy, Rgba(out));
        }
    }

    DynamicImage::ImageRgba8(buf)
}

/// Flip an image horizontally, vertically, or both.
pub fn flip(img: &DynamicImage, direction: &str) -> DynamicImage {
    match direction {
        "horizontal" => img.fliph(),
        "vertical" => img.flipv(),
        "both" => img.fliph().flipv(),
        _ => img.fliph(),
    }
}

/// Apply a lens distortion effect to an image.
///
/// All effects use inverse mapping + bilinear interpolation.  Pixels that map
/// outside the source image become transparent.
pub fn distort(img: &DynamicImage, distort_type: &str, strength: f32) -> DynamicImage {
    let (w, h) = img.dimensions();
    let rgba = img.to_rgba8();

    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    // Normalisation radius (half diagonal)
    let r_max = (cx * cx + cy * cy).sqrt();

    let mut buf: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(w, h, Rgba([0u8, 0, 0, 0]));

    for dy in 0..h {
        for dx in 0..w {
            let nx = (dx as f32 - cx) / r_max; // normalised [-1, 1]
            let ny = (dy as f32 - cy) / r_max;

            let (src_nx, src_ny) = match distort_type {
                "barrel" => {
                    // r' = r * (1 + k * r²), k = strength  →  inverse: solve for r from r'
                    // Approximate inverse: r_src ≈ r_dst / (1 + k * r_dst²)
                    let r2 = nx * nx + ny * ny;
                    let scale = 1.0 + strength * r2;
                    if scale.abs() < 1e-6 {
                        continue;
                    }
                    (nx / scale, ny / scale)
                }
                "pincushion" => {
                    let r2 = nx * nx + ny * ny;
                    let scale = 1.0 - strength * r2;
                    if scale.abs() < 1e-6 {
                        continue;
                    }
                    (nx / scale, ny / scale)
                }
                "swirl" => {
                    let r = (nx * nx + ny * ny).sqrt();
                    let theta = ny.atan2(nx);
                    let twist = strength * (1.0 - r / 1.0); // r/R, R=1 in normalised space
                    let src_theta = theta - twist;
                    (r * src_theta.cos(), r * src_theta.sin())
                }
                _ => (nx, ny),
            };

            // Back to pixel coordinates
            let fx = src_nx * r_max + cx;
            let fy = src_ny * r_max + cy;

            if fx < 0.0 || fy < 0.0 || fx > w as f32 - 1.0 || fy > h as f32 - 1.0 {
                continue; // transparent
            }

            let x0 = fx.floor() as u32;
            let y0 = fy.floor() as u32;
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);
            let tx = fx - x0 as f32;
            let ty = fy - y0 as f32;

            let p00 = rgba.get_pixel(x0, y0).0;
            let p10 = rgba.get_pixel(x1, y0).0;
            let p01 = rgba.get_pixel(x0, y1).0;
            let p11 = rgba.get_pixel(x1, y1).0;

            let mut out = [0u8; 4];
            for i in 0..4 {
                let v = p00[i] as f32 * (1.0 - tx) * (1.0 - ty)
                    + p10[i] as f32 * tx * (1.0 - ty)
                    + p01[i] as f32 * (1.0 - tx) * ty
                    + p11[i] as f32 * tx * ty;
                out[i] = v.round() as u8;
            }
            buf.put_pixel(dx, dy, Rgba(out));
        }
    }

    DynamicImage::ImageRgba8(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn make_test_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
        let buf = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, a]));
        DynamicImage::ImageRgba8(buf)
    }

    #[test]
    fn test_resize_dimensions() {
        let img = make_test_image(4, 4, 255, 0, 0, 255);
        let out = resize(&img, 2, 2, "bilinear");
        assert_eq!(out.dimensions(), (2, 2));
    }

    #[test]
    fn test_crop_dimensions() {
        let img = make_test_image(4, 4, 0, 255, 0, 255);
        let out = crop(&img, 1, 1, 2, 2);
        assert_eq!(out.dimensions(), (2, 2));
    }

    #[test]
    fn test_rotate_zero_same_size() {
        let img = make_test_image(4, 4, 128, 128, 128, 255);
        let out = rotate(&img, 0.0, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(out.dimensions(), img.dimensions());
    }

    #[test]
    fn test_rotate_90_swaps_dims() {
        let img = make_test_image(4, 2, 128, 128, 128, 255);
        let out = rotate(&img, 90.0, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(out.dimensions(), (2, 4));
    }

    #[test]
    fn test_flip_double_horizontal_identity() {
        let img = make_test_image(4, 4, 100, 150, 200, 255);
        // Paint a non-uniform pattern so we can distinguish pixels
        let mut buf = img.to_rgba8();
        buf.put_pixel(0, 0, Rgba([1, 2, 3, 255]));
        buf.put_pixel(3, 0, Rgba([10, 20, 30, 255]));
        let src = DynamicImage::ImageRgba8(buf);

        let flipped_twice = flip(&flip(&src, "horizontal"), "horizontal");
        assert_eq!(
            src.to_rgba8().into_raw(),
            flipped_twice.to_rgba8().into_raw()
        );
    }

    #[test]
    fn test_distort_zero_strength_no_change() {
        let img = make_test_image(8, 8, 200, 100, 50, 255);
        let out = distort(&img, "barrel", 0.0);
        // With strength=0 the mapping is identity; every pixel should match.
        assert_eq!(out.dimensions(), img.dimensions());
        assert_eq!(img.to_rgba8().into_raw(), out.to_rgba8().into_raw());
    }
}
