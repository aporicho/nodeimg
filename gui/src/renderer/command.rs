use std::sync::Arc;

use super::curve::CurveRequest;
use super::quad::QuadRequest;
use super::shadow::ShadowRequest;
use super::text::TextRequest;
use super::types::Rect;

pub enum DrawCommand {
    Shadow(ShadowRequest),
    Rect(QuadRequest),
    Text(TextRequest),
    Image {
        rect: Rect,
        view: Arc<wgpu::TextureView>,
    },
    Curve(CurveRequest),
    PushClip {
        rect: Rect,
        radius: f32,
    },
    PopClip,
}
