use super::runtime::TextBoxRuntime;
use crate::renderer::{Point, Rect, TextMeasurer, TextStyle};

#[derive(Clone, Debug)]
pub(super) struct PreeditState {
    pub(super) text: String,
    pub(super) range: (usize, usize),
    pub(super) caret: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct PreeditLayout {
    pub(super) start_x: f32,
    pub(super) width: f32,
    pub(super) caret_x: f32,
}

impl TextBoxRuntime {
    pub(crate) fn clear_preedit(&mut self) {
        self.preedit = None;
        self.preedit_layout = None;
        self.clamp_scroll();
    }

    pub(crate) fn has_preedit(&self) -> bool {
        self.preedit.is_some()
    }

    pub(crate) fn preedit_text(&self) -> Option<&str> {
        self.preedit.as_ref().map(|preedit| preedit.text.as_str())
    }

    pub(crate) fn preedit_range(&self) -> Option<(usize, usize)> {
        self.preedit.as_ref().map(|preedit| preedit.range)
    }

    pub(crate) fn set_preedit(&mut self, text: &str, caret: Option<(usize, usize)>) {
        if text.is_empty() {
            self.clear_preedit();
            return;
        }

        let range = self
            .preedit
            .as_ref()
            .map(|preedit| preedit.range)
            .or_else(|| self.editor.selection_range())
            .unwrap_or((self.editor.cursor(), self.editor.cursor()));
        self.preedit = Some(PreeditState {
            text: text.to_string(),
            range,
            caret,
        });
    }

    pub(crate) fn preedit_start_x(&self) -> Option<f32> {
        self.preedit_layout
            .map(|layout| self.visible_content_x(layout.start_x))
    }

    pub(crate) fn preedit_width(&self) -> Option<f32> {
        self.preedit_layout.map(|layout| layout.width)
    }

    pub(crate) fn preedit_underline_rect(&self) -> Option<Rect> {
        let layout = self.preedit_layout?;
        Some(Rect {
            x: self.visible_content_x(layout.start_x),
            y: self.text_origin.y + self.layout.line_height + 1.0,
            w: layout.width.max(1.0),
            h: 1.0,
        })
    }

    pub(crate) fn preedit_origin_and_text(&self) -> Option<(Point, &str)> {
        let preedit = self.preedit.as_ref()?;
        let start = clamp_text_index(self.editor.text(), preedit.range.0);
        let point = self.layout.caret_point(start);
        let caret_text = preedit
            .caret
            .map(|(_, end)| &preedit.text[..clamp_text_index(&preedit.text, end)])
            .unwrap_or(preedit.text.as_str());
        let caret_offset = self.layout_width_for_text(caret_text);
        Some((
            Point {
                x: self.visible_content_x(point.x + caret_offset),
                y: self.text_origin.y + point.y,
            },
            preedit.text.as_str(),
        ))
    }

    fn layout_width_for_text(&self, text: &str) -> f32 {
        let chars = text.chars().count() as f32;
        chars * self.layout.line_height * 0.5
    }
}

pub(super) fn preedit_layout(
    preedit: &PreeditState,
    runtime: &TextBoxRuntime,
    measurer: &mut TextMeasurer,
    style: &TextStyle,
) -> PreeditLayout {
    let start = clamp_text_index(runtime.editor.text(), preedit.range.0);
    let start_x = runtime.caret_offset(start);
    let width = measurer.measure_with_style(&preedit.text, style).0;
    let caret_byte = preedit
        .caret
        .map(|(_, end)| clamp_text_index(&preedit.text, end))
        .unwrap_or(preedit.text.len());
    let caret_prefix = &preedit.text[..caret_byte];
    let caret_x = start_x + measurer.measure_with_style(caret_prefix, style).0;

    PreeditLayout {
        start_x,
        width,
        caret_x,
    }
}

fn clamp_text_index(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}
