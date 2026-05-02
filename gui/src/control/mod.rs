mod mapping;
pub(crate) mod painter;
mod param_layout;
mod resize_edge;
pub(crate) mod state;
pub(crate) mod systems;

use crate::renderer::TextStyle;
use crate::theme::{TextInputTheme, Theme};

pub use mapping::{ParamControlMap, ParamControlSpec};
pub use param_layout::{
    param_control_kind, param_control_layout_policy, param_control_min_height, ParamControlHeight,
    ParamControlKind, ParamControlLayoutPolicy, ParamControlMetrics,
};
#[cfg(test)]
pub(crate) use resize_edge::detect_resize_edge;
pub use resize_edge::{ResizeEdge, DEFAULT_RESIZE_EDGE_THRESHOLD};

#[derive(Clone, Debug, PartialEq)]
pub struct ControlIntrinsic {
    pub control_id: String,
    pub current_size: [f32; 2],
    pub min_size: [f32; 2],
    pub desired_size: [f32; 2],
    pub affects_parent_width: bool,
    pub affects_parent_height: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ControlRole {
    #[default]
    Generic,
    Button,
    Checkbox,
    Collapsible,
    Dropdown,
    NumberInput,
    Panel,
    Radio,
    Slider,
    TextArea,
    TextInput,
    Toggle,
}

impl ControlRole {
    pub fn is_focusable(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::Checkbox
                | Self::Collapsible
                | Self::Dropdown
                | Self::NumberInput
                | Self::Radio
                | Self::Slider
                | Self::TextArea
                | Self::TextInput
                | Self::Toggle
        )
    }

    pub fn is_panel(self) -> bool {
        matches!(self, Self::Panel)
    }

    pub fn is_text_input(self) -> bool {
        matches!(self, Self::TextInput)
    }

    pub fn is_text_area(self) -> bool {
        matches!(self, Self::TextArea)
    }
}

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
