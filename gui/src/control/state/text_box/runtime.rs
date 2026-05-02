use super::layout::min_height_for_mode;
use super::model::{TextBoxSpec, TextBoxValueKind};
use super::preedit::{PreeditLayout, PreeditState};
use crate::control::{TextBoxFont, TextBoxMode};
use crate::renderer::{Point, Rect};
use crate::text::layout::TextLayoutResult;
use crate::text::TextEditState;
use crate::theme::TextInputTheme;

pub(crate) struct TextBoxRuntime {
    pub(super) editor: TextEditState,
    pub(super) last_external_text: String,
    pub(super) mode: TextBoxMode,
    pub(super) value_kind: TextBoxValueKind,
    pub(super) tokens: TextInputTheme,
    pub(super) font: TextBoxFont,
    pub(super) disabled: bool,
    pub(super) field_rect: Rect,
    pub(super) content_rect: Rect,
    pub(super) text_origin: Point,
    pub(super) layout: TextLayoutResult,
    pub(super) min_height: f32,
    pub(super) desired_height: f32,
    pub(super) scroll_x: f32,
    pub(super) preedit: Option<PreeditState>,
    pub(super) preedit_layout: Option<PreeditLayout>,
}

impl TextBoxRuntime {
    pub(super) fn new(spec: &TextBoxSpec) -> Self {
        let line_height = spec.tokens.value_size * 1.2;
        let min_height = min_height_for_mode(spec.mode, line_height, spec.tokens);
        Self {
            editor: TextEditState::new(&spec.external_text),
            last_external_text: spec.external_text.clone(),
            mode: spec.mode,
            value_kind: spec.value_kind,
            tokens: spec.tokens,
            font: spec.font,
            disabled: spec.disabled,
            field_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: min_height,
            },
            content_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: min_height,
            },
            text_origin: Point { x: 0.0, y: 0.0 },
            layout: TextLayoutResult {
                lines: Vec::new(),
                line_height,
                width: 1.0,
                height: line_height,
            },
            min_height,
            desired_height: min_height,
            scroll_x: 0.0,
            preedit: None,
            preedit_layout: None,
        }
    }

    pub(super) fn sync_spec(&mut self, spec: &TextBoxSpec) {
        self.mode = spec.mode;
        self.value_kind = spec.value_kind;
        self.tokens = spec.tokens;
        self.font = spec.font;
        self.disabled = spec.disabled;
        if self.is_multiline() {
            self.scroll_x = 0.0;
        }
    }

    pub(crate) fn value_kind(&self) -> TextBoxValueKind {
        self.value_kind
    }

    pub(crate) fn is_multiline(&self) -> bool {
        matches!(self.mode, TextBoxMode::MultiLine { .. })
    }

    pub(crate) fn is_number(&self) -> bool {
        matches!(self.value_kind, TextBoxValueKind::Number { .. })
    }

    pub(crate) fn external_text(&self) -> &str {
        &self.last_external_text
    }

    pub(crate) fn editor(&self) -> &TextEditState {
        &self.editor
    }

    pub(crate) fn editor_mut(&mut self) -> &mut TextEditState {
        &mut self.editor
    }

    pub(crate) fn sync_external_text(&mut self, text: &str, allow_override: bool) -> bool {
        if text == self.last_external_text {
            return false;
        }

        let current_matches_external = self.editor.text() == self.last_external_text;
        if allow_override || current_matches_external || self.editor.text() == text {
            self.clear_preedit();
            self.editor.set_text(text);
        }
        self.last_external_text = text.to_string();
        true
    }

    pub(crate) fn revert_to_external(&mut self) {
        let external = self.last_external_text.clone();
        self.clear_preedit();
        self.editor.set_text(&external);
    }

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

    pub(super) fn caret_offset(&self, byte_index: usize) -> f32 {
        self.layout
            .line_for_index(byte_index)
            .map(|line| line.x_for_index(byte_index))
            .unwrap_or(0.0)
    }

    pub(super) fn visible_content_x(&self, content_x: f32) -> f32 {
        if self.is_multiline() {
            self.text_origin.x + content_x
        } else {
            self.text_origin.x + content_x - self.scroll_x
        }
    }

    pub(super) fn ensure_cursor_visible(&mut self) {
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

    pub(super) fn clamp_scroll(&mut self) {
        if self.is_multiline() {
            self.scroll_x = 0.0;
            return;
        }
        let max_scroll = (self.active_content_width() - self.content_rect.w).max(0.0);
        self.scroll_x = self.scroll_x.clamp(0.0, max_scroll);
    }
}
