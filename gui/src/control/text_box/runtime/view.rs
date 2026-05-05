use super::model::TextBoxRuntime;
use crate::renderer::{Point, Rect};
use crate::text::layout::TextLayoutResult;

impl TextBoxRuntime {
    pub(crate) fn current_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.field_rect.h]
    }

    pub(crate) fn min_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.min_height]
    }

    pub(crate) fn desired_size(&self) -> [f32; 2] {
        [self.field_rect.w, self.desired_height]
    }

    pub(crate) fn clip_rect(&self) -> Rect {
        self.content_rect
    }

    pub(crate) fn text_draw_origin(&self) -> Point {
        Point {
            x: self.visible_content_x(0.0),
            y: self.text_origin.y,
        }
    }

    pub(crate) fn layout(&self) -> &TextLayoutResult {
        &self.layout
    }

    pub(crate) fn visible_line_origin(&self, line_y: f32) -> Point {
        Point {
            x: self.visible_content_x(0.0),
            y: self.text_origin.y + line_y,
        }
    }
}
