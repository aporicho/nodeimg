use super::model::TextBoxMode;
use super::model::TextBoxSpec;
use super::preedit::preedit_layout;
use super::runtime::TextBoxRuntime;
use crate::renderer::{Point, Rect, TextMeasurer, TextStyle};
use crate::text::layout::TextLayoutPolicy;
use crate::text::TextLayoutCache;
use crate::theme::TextInputTheme;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TextBoxLayoutSync {
    pub(super) cache_hit: bool,
    pub(super) desired_height_changed: bool,
}

impl TextBoxRuntime {
    pub(super) fn sync_layout_with_style(
        &mut self,
        field_rect: Rect,
        value_rect: Option<Rect>,
        measurer: &mut TextMeasurer,
        spec: &TextBoxSpec,
        value_style: TextStyle,
        layout_cache: &mut TextLayoutCache,
    ) -> TextBoxLayoutSync {
        let previous_desired_height = self.desired_height;
        let mut sync = TextBoxLayoutSync::default();
        self.field_rect = field_rect;
        match spec.mode {
            TextBoxMode::SingleLine => {
                self.content_rect = Rect {
                    x: field_rect.x + spec.tokens.padding_x,
                    y: field_rect.y,
                    w: (field_rect.w - spec.tokens.padding_x * 2.0).max(1.0),
                    h: field_rect.h,
                };
                let (layout, cache_hit) = layout_cache.layout_text(
                    self.editor.text(),
                    TextLayoutPolicy::NoWrap,
                    self.content_rect.w,
                    measurer,
                    &value_style,
                );
                self.layout = layout;
                sync.cache_hit = cache_hit;
                self.text_origin = Point {
                    x: self.content_rect.x,
                    y: value_rect
                        .map(|rect| rect.y)
                        .unwrap_or(field_rect.y + (field_rect.h - self.layout.line_height) * 0.5),
                };
                self.min_height = field_rect.h;
                self.desired_height = field_rect.h;
                self.preedit_layout = self
                    .preedit
                    .as_ref()
                    .map(|preedit| preedit_layout(preedit, self, measurer, &value_style));
                self.clamp_scroll();
                self.ensure_cursor_visible();
            }
            TextBoxMode::MultiLine { min_rows } => {
                self.content_rect = Rect {
                    x: field_rect.x + spec.tokens.padding_x,
                    y: field_rect.y + spec.tokens.padding_y,
                    w: (field_rect.w - spec.tokens.padding_x * 2.0).max(1.0),
                    h: (field_rect.h - spec.tokens.padding_y * 2.0).max(1.0),
                };
                self.text_origin = Point {
                    x: self.content_rect.x,
                    y: self.content_rect.y,
                };
                let (layout, cache_hit) = layout_cache.layout_text(
                    self.editor.text(),
                    TextLayoutPolicy::Wrap,
                    self.content_rect.w,
                    measurer,
                    &value_style,
                );
                self.layout = layout;
                sync.cache_hit = cache_hit;
                self.min_height =
                    min_rows.max(1) as f32 * self.layout.line_height + spec.tokens.padding_y * 2.0;
                self.desired_height =
                    (self.layout.height + spec.tokens.padding_y * 2.0).max(self.min_height);
                self.scroll_x = 0.0;
                self.preedit_layout = None;
            }
        }
        sync.desired_height_changed = (self.desired_height - previous_desired_height).abs() > 0.5;
        sync
    }
}

pub(super) fn fallback_height(mode: TextBoxMode, tokens: TextInputTheme) -> f32 {
    match mode {
        TextBoxMode::SingleLine => tokens.field_height,
        TextBoxMode::MultiLine { min_rows } => {
            min_rows.max(1) as f32 * tokens.value_size * 1.2 + tokens.padding_y * 2.0
        }
    }
}

pub(super) fn min_height_for_mode(
    mode: TextBoxMode,
    line_height: f32,
    tokens: TextInputTheme,
) -> f32 {
    match mode {
        TextBoxMode::SingleLine => tokens.field_height,
        TextBoxMode::MultiLine { min_rows } => {
            min_rows.max(1) as f32 * line_height + tokens.padding_y * 2.0
        }
    }
}
