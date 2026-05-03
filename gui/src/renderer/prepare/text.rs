use crate::paint::TextStyle;

use super::super::affine::translate_uniform_scale;
use super::super::command::AffineTextRequest;
use super::super::pipeline::text::TextRequest;
use super::{DrawOp, PreparedFrame};

pub(super) fn push_text_op(req: &AffineTextRequest, frame: &mut PreparedFrame) {
    let Some(axis_aligned) = translate_uniform_scale(req.transform) else {
        frame.stats.affine_text_fallbacks += 1;
        frame.ops.push(DrawOp::AffineText(req.clone()));
        return;
    };

    let index = frame.text_requests.len();
    frame.text_requests.push(TextRequest {
        pos: req.transform.transform_point(req.pos),
        text: req.text.clone(),
        style: scale_text_style(req.style, axis_aligned.scale),
        bounds: req.bounds.map(|bounds| axis_aligned.rect(bounds)),
    });
    frame.ops.push(DrawOp::Text { index });
}

fn scale_text_style(mut style: TextStyle, scale: f32) -> TextStyle {
    style.size *= scale;
    style
}
