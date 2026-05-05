use super::model::TextBoxRuntime;
use crate::renderer::{Point, Rect};

impl TextBoxRuntime {
    pub(crate) fn caret_rect(&self) -> Rect {
        let caret_x = self
            .preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()));
        let point = if self.is_multiline() {
            self.layout.caret_point(self.editor.cursor())
        } else {
            Point { x: caret_x, y: 0.0 }
        };
        Rect {
            x: self.visible_content_x(point.x),
            y: self.text_origin.y + point.y,
            w: 1.0,
            h: self.layout.line_height.max(1.0),
        }
    }

    pub(crate) fn selection_rects(&self) -> Vec<Rect> {
        if self.preedit.is_some() {
            return Vec::new();
        }
        let Some((start, end)) = self.editor.selection_range() else {
            return Vec::new();
        };
        self.layout
            .selection_rects(start, end)
            .into_iter()
            .map(|rect| Rect {
                x: self.visible_content_x(rect.x),
                y: self.text_origin.y + rect.y,
                w: rect.w,
                h: rect.h,
            })
            .collect()
    }

    pub(crate) fn set_caret_from_point(&mut self, x: f32, y: f32) {
        let index = self.nearest_index_for_point(x, y);
        self.editor.move_to(index);
        self.ensure_cursor_visible();
    }

    pub(crate) fn select_to_point(&mut self, x: f32, y: f32) {
        let index = self.nearest_index_for_point(x, y);
        self.editor.select_to(index);
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_left(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_left();
        } else {
            self.editor.move_left();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_right(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_right();
        } else {
            self.editor.move_right();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_home(&mut self, extend_selection: bool) {
        if extend_selection {
            self.editor.select_to(0);
        } else {
            self.editor.move_home();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_end(&mut self, extend_selection: bool) {
        let text_end = self.editor.text().len();
        if extend_selection {
            self.editor.select_to(text_end);
        } else {
            self.editor.move_end();
        }
        self.ensure_cursor_visible();
    }

    pub(crate) fn move_vertical(&mut self, direction: i32, extend_selection: bool) {
        if !self.is_multiline() {
            return;
        }
        let cursor = self.editor.cursor();
        let Some(line_index) = self
            .layout
            .lines
            .iter()
            .position(|line| cursor >= line.start && cursor <= line.end)
        else {
            return;
        };
        let target = (line_index as i32 + direction)
            .clamp(0, self.layout.lines.len().saturating_sub(1) as i32)
            as usize;
        if target == line_index {
            return;
        }
        let x = self.layout.caret_point(cursor).x;
        let y = self.layout.lines[target].y;
        let index = self.layout.nearest_index(x, y);
        if extend_selection {
            self.editor.select_to(index);
        } else {
            self.editor.move_to(index);
        }
    }

    fn nearest_index_for_point(&self, x: f32, y: f32) -> usize {
        let local_x = if self.is_multiline() {
            x - self.text_origin.x
        } else {
            x - self.text_origin.x + self.scroll_x
        };
        let local_y = y - self.text_origin.y;
        self.layout
            .nearest_index(local_x.max(0.0), local_y.max(0.0))
    }

    pub(in crate::control::text_box) fn caret_offset(&self, byte_index: usize) -> f32 {
        self.layout
            .line_for_index(byte_index)
            .map(|line| line.x_for_index(byte_index))
            .unwrap_or(0.0)
    }
}
