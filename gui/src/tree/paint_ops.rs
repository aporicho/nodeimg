use crate::renderer::{Color, PathData, PathStyle, Point, Rect, RectStyle, TextStyle};

use super::layout::TextureHandle;
use super::paint_target::PaintTarget;

#[derive(Debug, Clone, PartialEq)]
pub enum PaintOp {
    Rect {
        rect: Rect,
        style: RectStyle,
    },
    Text {
        pos: Point,
        text: String,
        style: TextStyle,
        bounds: Option<Rect>,
    },
    Image {
        rect: Rect,
        texture: TextureHandle,
    },
    Circle {
        center: Point,
        radius: f32,
        color: Color,
    },
    Path {
        data: PathData,
        style: PathStyle,
    },
    PushClip {
        rect: Rect,
        radius: f32,
    },
    PopClip,
}

pub struct RecordingPaintTarget {
    ops: Vec<PaintOp>,
    measure: Box<dyn FnMut(&str, &TextStyle) -> (f32, f32)>,
}

impl RecordingPaintTarget {
    pub fn new() -> Self {
        Self::with_measure(|text, style| {
            let width = text.chars().count() as f32 * style.size * 0.5;
            (width, style.size * style.line_height)
        })
    }

    pub fn with_measure(measure: impl FnMut(&str, &TextStyle) -> (f32, f32) + 'static) -> Self {
        Self {
            ops: Vec::new(),
            measure: Box::new(measure),
        }
    }

    pub fn ops(&self) -> &[PaintOp] {
        &self.ops
    }

    pub fn into_ops(self) -> Vec<PaintOp> {
        self.ops
    }
}

impl Default for RecordingPaintTarget {
    fn default() -> Self {
        Self::new()
    }
}

impl PaintTarget for RecordingPaintTarget {
    fn draw_rect(&mut self, rect: Rect, style: RectStyle) {
        self.ops.push(PaintOp::Rect { rect, style });
    }

    fn draw_text(&mut self, pos: Point, text: &str, style: TextStyle) {
        self.ops.push(PaintOp::Text {
            pos,
            text: text.to_string(),
            style,
            bounds: None,
        });
    }

    fn draw_text_clipped(&mut self, pos: Point, text: &str, style: TextStyle, bounds: Rect) {
        self.ops.push(PaintOp::Text {
            pos,
            text: text.to_string(),
            style,
            bounds: Some(bounds),
        });
    }

    fn draw_image(&mut self, rect: Rect, texture: TextureHandle) {
        self.ops.push(PaintOp::Image { rect, texture });
    }

    fn draw_circle(&mut self, center: Point, radius: f32, color: Color) {
        self.ops.push(PaintOp::Circle {
            center,
            radius,
            color,
        });
    }

    fn draw_path(&mut self, data: PathData, style: PathStyle) {
        self.ops.push(PaintOp::Path { data, style });
    }

    fn push_clip(&mut self, rect: Rect, radius: f32) {
        self.ops.push(PaintOp::PushClip { rect, radius });
    }

    fn pop_clip(&mut self) {
        self.ops.push(PaintOp::PopClip);
    }

    fn measure_text(&mut self, text: &str, style: &TextStyle) -> (f32, f32) {
        (self.measure)(text, style)
    }
}
