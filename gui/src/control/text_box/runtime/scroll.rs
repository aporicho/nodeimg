use super::model::TextBoxRuntime;

impl TextBoxRuntime {
    pub(in crate::control::text_box) fn visible_content_x(&self, content_x: f32) -> f32 {
        if self.is_multiline() {
            self.text_origin.x + content_x
        } else {
            self.text_origin.x + content_x - self.scroll_x
        }
    }

    pub(in crate::control::text_box) fn ensure_cursor_visible(&mut self) {
        if self.is_multiline() {
            return;
        }
        let view_width = self.content_rect.w.max(1.0);
        let caret_x = self
            .preedit_layout
            .map(|layout| layout.caret_x)
            .unwrap_or_else(|| self.caret_offset(self.editor.cursor()));
        if caret_x < self.scroll_x {
            self.scroll_x = caret_x;
        } else if caret_x > self.scroll_x + view_width {
            self.scroll_x = (caret_x - view_width).max(0.0);
        }
        self.clamp_scroll();
    }

    fn active_content_width(&self) -> f32 {
        let text_width = self
            .layout
            .lines
            .first()
            .map(|line| line.width)
            .unwrap_or(0.0);
        let Some(preedit) = &self.preedit else {
            return text_width;
        };
        let start_x = self.caret_offset(preedit.range.0);
        let end_x = self.caret_offset(preedit.range.1);
        start_x + self.preedit_width().unwrap_or(0.0) + (text_width - end_x)
    }

    pub(in crate::control::text_box) fn clamp_scroll(&mut self) {
        if self.is_multiline() {
            self.scroll_x = 0.0;
            return;
        }
        let max_scroll = (self.active_content_width() - self.content_rect.w).max(0.0);
        self.scroll_x = self.scroll_x.clamp(0.0, max_scroll);
    }
}
