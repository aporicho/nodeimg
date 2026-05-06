use crate::renderer::TextStyle;
use crate::theme::{TextInputTheme, Theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoxMode {
    SingleLine,
    MultiLine { min_rows: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextBoxFont {
    Body,
    Mono,
}

pub fn format_number(value: f32, precision: usize) -> String {
    if precision == 0 {
        format!("{value:.0}")
    } else {
        let mut text = format!("{value:.precision$}");
        while text.contains('.') && text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    }
}

pub(crate) fn text_box_value_style(
    theme: &Theme,
    tokens: TextInputTheme,
    font: TextBoxFont,
) -> TextStyle {
    let base = match font {
        TextBoxFont::Body => theme.text_style_body_sm(),
        TextBoxFont::Mono => theme.text_style_mono_md(),
    };
    TextStyle {
        size: tokens.value_size,
        ..base
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TextBoxValueKind {
    Text,
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    },
    Color {
        rgba: [f32; 4],
    },
    FilePath,
}

#[derive(Clone, Debug)]
pub(crate) struct TextBoxSpec {
    pub(crate) external_text: String,
    pub(crate) mode: TextBoxMode,
    pub(crate) value_kind: TextBoxValueKind,
    pub(crate) tokens: TextInputTheme,
    pub(crate) font: TextBoxFont,
    pub(crate) disabled: bool,
}
