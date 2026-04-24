use std::sync::Arc;

use crate::geometry::Affine2D;
use crate::paint::{CirclePaint, ClipShape, PathData, RectStyle};

use super::image::{ImageStyle, TextureSize};
use super::path::PathStyle;
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::text::TextRequest;
use super::svg::SvgRasterDraw;
use super::types::Rect;

pub(super) enum BackendCommand {
    Shadow(ShadowRequest),
    Rect(AffineRectRequest),
    Circle(AffineCircleRequest),
    Text(TextRequest),
    Image(AffineImageRequest),
    SvgRaster(SvgRasterDraw),
    Path(AffinePathRequest),
    PushClip(AffineClipRequest),
    PopClip,
}

#[derive(Clone)]
pub(super) struct AffineRectRequest {
    pub rect: Rect,
    pub style: RectStyle,
    pub transform: Affine2D,
}

#[derive(Clone)]
pub(super) struct AffinePathRequest {
    pub data: PathData,
    pub style: PathStyle,
    pub transform: Affine2D,
}

#[derive(Clone, Copy)]
pub(super) struct AffineCircleRequest {
    pub paint: CirclePaint,
    pub transform: Affine2D,
}

#[derive(Clone)]
pub(super) struct AffineImageRequest {
    pub rect: Rect,
    pub transform: Affine2D,
    pub view: Arc<wgpu::TextureView>,
    pub size: TextureSize,
    pub style: ImageStyle,
}

#[derive(Clone)]
pub(super) struct AffineClipRequest {
    pub shape: ClipShape,
    pub transform: Affine2D,
}
