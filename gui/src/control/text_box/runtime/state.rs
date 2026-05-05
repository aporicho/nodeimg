use super::model::TextBoxRuntime;
use crate::control::text_box::layout::min_height_for_mode;
use crate::control::text_box::model::{TextBoxMode, TextBoxSpec, TextBoxValueKind};
use crate::renderer::{Point, Rect};
use crate::text::layout::TextLayoutResult;
use crate::text::TextEditState;

impl TextBoxRuntime {
    pub(in crate::control::text_box) fn new(spec: &TextBoxSpec) -> Self {
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

    pub(in crate::control::text_box) fn sync_spec(&mut self, spec: &TextBoxSpec) {
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
}
