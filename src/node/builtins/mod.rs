mod blend;
mod blur;
mod channel_merge;
mod checkerboard;
mod color_adjust;
mod color_balance;
pub mod curves;
mod crop;
mod denoise;
mod distort;
mod edge_detect;
mod emboss;
mod film_grain;
mod flip;
mod gradient;
mod histogram;
mod hue_saturation;
mod invert;
mod levels;
mod load_image;
mod lut_apply;
mod mask;
mod noise;
mod pixelate;
mod preview;
mod resize;
mod rotate;
mod save_image;
mod sharpen;
mod solid_color;
mod threshold;
mod vignette;

use crate::node::registry::NodeRegistry;

/// Register all built-in nodes.
pub fn register_all(registry: &mut NodeRegistry) {
    load_image::register(registry);
    color_adjust::register(registry);
    invert::register(registry);
    threshold::register(registry);
    levels::register(registry);
    hue_saturation::register(registry);
    color_balance::register(registry);
    lut_apply::register(registry);
    preview::register(registry);
    save_image::register(registry);
    solid_color::register(registry);
    gradient::register(registry);
    checkerboard::register(registry);
    noise::register(registry);
    resize::register(registry);
    crop::register(registry);
    rotate::register(registry);
    flip::register(registry);
    distort::register(registry);
    // Composite nodes
    blend::register(registry);
    mask::register(registry);
    channel_merge::register(registry);
    // Tool nodes
    histogram::register(registry);
    // Filter nodes
    blur::register(registry);
    pixelate::register(registry);
    vignette::register(registry);
    film_grain::register(registry);
    sharpen::register(registry);
    edge_detect::register(registry);
    emboss::register(registry);
    denoise::register(registry);
    curves::register(registry);
}
