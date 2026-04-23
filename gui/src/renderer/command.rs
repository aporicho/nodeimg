use std::sync::Arc;

use super::image::{ImageStyle, TextureSize};
use super::path::PathRequest;
use super::pipeline::circle::CircleRequest;
use super::pipeline::quad::QuadRequest;
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::text::TextRequest;
use super::svg::SvgRasterDraw;
use super::types::Rect;

pub(super) enum BackendCommand {
    Shadow(ShadowRequest),
    Rect(QuadRequest),
    Circle(CircleRequest),
    Text(TextRequest),
    Image {
        rect: Rect,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
        style: ImageStyle,
    },
    SvgRaster(SvgRasterDraw),
    Path(PathRequest),
    PushClip {
        rect: Rect,
        radius: f32,
    },
    PopClip,
}
