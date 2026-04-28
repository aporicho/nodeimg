mod affine;
mod buffer;
pub mod cache_lifecycle;
mod command;
mod dispatch;
mod display_backend;
mod display_resources;
mod geometry_cache;
mod image;
mod offscreen;
mod path;
mod path_geometry;
mod pipeline;
mod prepare;
mod scene_prepare;
pub(crate) mod svg;
pub(crate) mod text_measurer;
mod texture;
mod types;
mod upload_arena;
mod vector_tessellator;

pub mod style;

pub use core::Renderer;
pub(crate) use display_resources::RegistryDisplayResources;
pub use image::{
    resolve_image_draw, ImageFilter, ImageFit, ImageOpacity, ImageSourceRect, ImageStyle,
    ResolvedImageDraw, TextureSize,
};
pub use path::{PathCommand, PathData, PathRequest, PathStyle};
pub use scene_prepare::RendererPrepareStats;
pub use style::{
    Border, Fill, FillRule, LineCap, LineJoin, RectStyle, Shadow, Stroke, TextFamily, TextStyle,
    TextWeight,
};
pub use text_measurer::TextMeasurer;
pub(crate) use texture::TextureResource;
pub use types::{Color, Point, Rect};

mod core;

#[cfg(test)]
pub(crate) mod test_support;
