use std::sync::Arc;

use super::pipeline::circle::CircleRequest;
use super::pipeline::curve::CurveRequest;
use super::pipeline::quad::QuadRequest;
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::text::TextRequest;
use super::types::Rect;

pub enum DrawCommand {
    Shadow(ShadowRequest),
    Rect(QuadRequest),
    Circle(CircleRequest),
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
