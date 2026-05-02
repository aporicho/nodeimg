use crate::control::{TextBoxFont, TextBoxMode};
use crate::theme::TextInputTheme;

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
