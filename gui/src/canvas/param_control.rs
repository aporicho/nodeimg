use crate::theme::{ControlSize, Density, Theme};
use crate::tree::layout::{Align, Justify, Size, TextAlign, TextOverflow};
use crate::tree::Desc;
use crate::ui::{self, StyleBuilder};
use crate::widget::atoms::color_swatch::ColorSwatchProps;
use crate::widget::atoms::dropdown::DropdownProps;
use crate::widget::atoms::label::LabelVariant;
use crate::widget::atoms::number_input::NumberInputProps;
use crate::widget::atoms::path_input::PathInputProps;
use crate::widget::atoms::slider::SliderProps;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::atoms::toggle::ToggleProps;
use crate::widget::atoms::truncated_text::TruncatedTextProps;
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum CanvasNodeParamControl {
    ReadOnly {
        value: String,
    },
    Text {
        value: String,
    },
    Number {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        precision: usize,
    },
    Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
    },
    Toggle {
        checked: bool,
    },
    Select {
        options: Vec<String>,
        selected: usize,
    },
    Color {
        rgba: [f32; 4],
    },
    FilePath {
        path: String,
        extensions: Vec<String>,
    },
}

impl Default for CanvasNodeParamControl {
    fn default() -> Self {
        Self::ReadOnly {
            value: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasParamControlMetrics {
    pub control_height: f32,
    pub control_width: f32,
    pub size: ControlSize,
    pub density: Density,
}

impl CanvasParamControlMetrics {
    pub fn from_theme(_theme: &Theme) -> Self {
        Self {
            control_height: 24.0,
            control_width: 128.0,
            size: ControlSize::Small,
            density: Density::Compact,
        }
    }
}

pub fn param_control(
    id: impl Into<Cow<'static, str>>,
    control: &CanvasNodeParamControl,
    _theme: &Theme,
    metrics: CanvasParamControlMetrics,
) -> Desc {
    let id = id.into();
    let base = id.to_string();
    let child_id = format!("{base}::widget");
    let child = match control {
        CanvasNodeParamControl::ReadOnly { value } => ui::widget(
            child_id,
            TruncatedTextProps {
                text: Cow::Owned(value.clone()),
                variant: LabelVariant::Caption,
                muted: true,
                overflow: TextOverflow::Ellipsis,
                align: TextAlign::Start,
                width: Size::Fill,
            },
        )
        .build(),
        CanvasNodeParamControl::Text { value } => ui::widget(
            child_id,
            TextInputProps {
                label: None,
                value: Cow::Owned(value.clone()),
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::Number {
            value,
            min,
            max,
            step,
            precision,
        } => ui::widget(
            child_id,
            NumberInputProps {
                label: None,
                value: *value,
                min: *min,
                max: *max,
                step: *step,
                precision: *precision,
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::Slider {
            value,
            min,
            max,
            step,
        } => ui::widget(
            child_id,
            SliderProps {
                label: None,
                min: *min,
                max: *max,
                step: *step,
                value: *value,
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::Toggle { checked } => ui::widget(
            child_id,
            ToggleProps {
                label: None,
                value: *checked,
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::Select { options, selected } => ui::widget(
            child_id,
            DropdownProps {
                label: None,
                options: options.iter().cloned().map(Cow::Owned).collect(),
                selected: *selected,
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::Color { rgba } => ui::widget(
            child_id,
            ColorSwatchProps {
                rgba: *rgba,
                label: None,
                show_value: true,
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
        CanvasNodeParamControl::FilePath { path, extensions } => ui::widget(
            child_id,
            PathInputProps {
                label: None,
                value: Cow::Owned(path.clone()),
                extensions: extensions.iter().cloned().map(Cow::Owned).collect(),
                disabled: false,
                size: metrics.size,
                density: metrics.density,
            },
        )
        .build(),
    };

    ui::row(id)
        .fixed_width(metrics.control_width)
        .fixed_height(metrics.control_height)
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .child(child)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn slider_param_control_adapts_to_widget() {
        let theme = light_theme();
        let metrics = CanvasParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &CanvasNodeParamControl::Slider {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            &theme,
            metrics,
        ) else {
            panic!("slider control should build a wrapper container");
        };

        assert_eq!(children[0].id(), "control::widget");
        match &children[0] {
            Desc::Widget(widget) => assert_eq!(widget.props().widget_type(), "Slider"),
            _ => panic!("expected slider widget"),
        }
    }

    #[test]
    fn color_param_control_adapts_to_widget() {
        let theme = light_theme();
        let metrics = CanvasParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &CanvasNodeParamControl::Color {
                rgba: [1.0, 0.5, 0.0, 1.0],
            },
            &theme,
            metrics,
        ) else {
            panic!("color control should build a wrapper container");
        };

        match &children[0] {
            Desc::Widget(widget) => assert_eq!(widget.props().widget_type(), "ColorSwatch"),
            _ => panic!("expected color swatch widget"),
        }
    }
}
