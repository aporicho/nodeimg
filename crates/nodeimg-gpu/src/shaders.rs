//! WGSL shader source constants for GPU-accelerated image processing nodes.
//!
//! Each constant contains the complete shader source as a `&str`, loaded at
//! compile time via `include_str!`. Naming convention: filename without
//! extension, uppercased, hyphens replaced by underscores.

pub const BLEND: &str = include_str!("shaders/blend.wgsl");
pub const BOX_BLUR_H: &str = include_str!("shaders/box_blur_h.wgsl");
pub const BOX_BLUR_V: &str = include_str!("shaders/box_blur_v.wgsl");
pub const CHANNEL_MERGE: &str = include_str!("shaders/channel_merge.wgsl");
pub const CHECKERBOARD: &str = include_str!("shaders/checkerboard.wgsl");
pub const COLOR_ADJUST: &str = include_str!("shaders/color_adjust.wgsl");
pub const COLOR_BALANCE: &str = include_str!("shaders/color_balance.wgsl");
pub const CROP: &str = include_str!("shaders/crop.wgsl");
pub const DENOISE: &str = include_str!("shaders/denoise.wgsl");
pub const DISTORT: &str = include_str!("shaders/distort.wgsl");
pub const EDGE_DETECT: &str = include_str!("shaders/edge_detect.wgsl");
pub const EMBOSS: &str = include_str!("shaders/emboss.wgsl");
pub const FILM_GRAIN: &str = include_str!("shaders/film_grain.wgsl");
pub const FLIP: &str = include_str!("shaders/flip.wgsl");
pub const GAUSSIAN_BLUR_H: &str = include_str!("shaders/gaussian_blur_h.wgsl");
pub const GAUSSIAN_BLUR_V: &str = include_str!("shaders/gaussian_blur_v.wgsl");
pub const GRADIENT: &str = include_str!("shaders/gradient.wgsl");
pub const HUE_SATURATION: &str = include_str!("shaders/hue_saturation.wgsl");
pub const INVERT: &str = include_str!("shaders/invert.wgsl");
pub const LEVELS: &str = include_str!("shaders/levels.wgsl");
pub const LUT_APPLY: &str = include_str!("shaders/lut_apply.wgsl");
pub const MASK: &str = include_str!("shaders/mask.wgsl");
pub const NOISE: &str = include_str!("shaders/noise.wgsl");
pub const PIXELATE: &str = include_str!("shaders/pixelate.wgsl");
pub const RESIZE: &str = include_str!("shaders/resize.wgsl");
pub const ROTATE: &str = include_str!("shaders/rotate.wgsl");
pub const SHARPEN: &str = include_str!("shaders/sharpen.wgsl");
pub const SOLID_COLOR: &str = include_str!("shaders/solid_color.wgsl");
pub const THRESHOLD: &str = include_str!("shaders/threshold.wgsl");
pub const VIGNETTE: &str = include_str!("shaders/vignette.wgsl");
