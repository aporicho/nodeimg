use image::{DynamicImage, ImageBuffer, Rgba};

/// Create a solid-color test image.
pub fn make_test_image(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> DynamicImage {
    let buf = ImageBuffer::from_pixel(w, h, Rgba([r, g, b, a]));
    DynamicImage::ImageRgba8(buf)
}

/// Compare two images pixel by pixel. Returns the max absolute difference
/// across all RGBA channels. Returns Err if dimensions mismatch.
pub fn max_pixel_diff(a: &DynamicImage, b: &DynamicImage) -> Result<u8, String> {
    if a.width() != b.width() || a.height() != b.height() {
        return Err(format!(
            "dimension mismatch: {}x{} vs {}x{}",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        ));
    }
    let a_rgba = a.to_rgba8();
    let b_rgba = b.to_rgba8();
    let max_diff = a_rgba
        .pixels()
        .zip(b_rgba.pixels())
        .flat_map(|(pa, pb)| pa.0.iter().zip(pb.0.iter()).map(|(&a, &b)| a.abs_diff(b)))
        .max()
        .unwrap_or(0);
    Ok(max_diff)
}

/// Assert two images are identical (max_pixel_diff == 0).
pub fn assert_images_equal(a: &DynamicImage, b: &DynamicImage) {
    let diff = max_pixel_diff(a, b).expect("images should have same dimensions");
    assert_eq!(diff, 0, "images differ by max {} levels", diff);
}

/// Assert two images are similar within a tolerance (for GPU float rounding).
pub fn assert_images_similar(a: &DynamicImage, b: &DynamicImage, tolerance: u8) {
    let diff = max_pixel_diff(a, b).expect("images should have same dimensions");
    assert!(
        diff <= tolerance,
        "images differ by max {} levels (tolerance: {})",
        diff,
        tolerance
    );
}
