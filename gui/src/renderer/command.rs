use std::sync::Arc;

use crate::geometry::Affine2D;
use crate::icon::IconStyle;
use crate::paint::{CirclePaint, ClipShape, GridPaint, PathData, RectStyle, Shadow, TextStyle};

use super::image::{ImageStyle, TextureSize};
use super::path::PathStyle;
use super::svg::SvgSource;
use super::types::Rect;

#[derive(Clone)]
pub(super) enum BackendCommand {
    Shadow(AffineShadowRequest),
    Rect(AffineRectRequest),
    Circle(AffineCircleRequest),
    Grid(AffineGridRequest),
    Text(AffineTextRequest),
    Image(AffineImageRequest),
    SvgRaster(AffineSvgRasterRequest),
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

#[derive(Clone, Copy)]
pub(super) struct AffineGridRequest {
    pub paint: GridPaint,
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

#[derive(Clone)]
pub(super) struct AffineTextRequest {
    pub pos: crate::geometry::Point,
    pub text: String,
    pub style: TextStyle,
    pub bounds: Option<Rect>,
    pub transform: Affine2D,
}

#[derive(Clone, Copy)]
pub(super) struct AffineShadowRequest {
    pub rect: Rect,
    pub radius: [f32; 4],
    pub shadow: Shadow,
    pub transform: Affine2D,
}

#[derive(Clone)]
pub(super) struct AffineSvgRasterRequest {
    pub rect: Rect,
    pub source: SvgSource,
    pub style: IconStyle,
    pub transform: Affine2D,
}
