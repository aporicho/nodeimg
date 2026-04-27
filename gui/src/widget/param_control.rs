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
use crate::widget::atoms::text_area::TextAreaProps;
use crate::widget::atoms::text_input::TextInputProps;
use crate::widget::atoms::toggle::ToggleProps;
use crate::widget::atoms::truncated_text::TruncatedTextProps;
use crate::widget::mapping::ParamControlSpec;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamControlMetrics {
    pub control_height: f32,
    pub control_width: f32,
    pub size: ControlSize,
    pub density: Density,
}

impl ParamControlMetrics {
    pub fn from_theme(_theme: &Theme) -> Self {
        Self {
            control_height: 24.0,
            control_width: 128.0,
            size: ControlSize::Small,
            density: Density::Compact,
        }
    }
}

pub fn param_control_min_height(
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: ParamControlMetrics,
) -> f32 {
    match control {
        ParamControlSpec::TextArea { min_rows, .. } => {
            text_area_min_height(*min_rows, theme, metrics)
        }
        _ => metrics.control_height,
    }
}

pub fn param_control(
    id: impl Into<Cow<'static, str>>,
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: ParamControlMetrics,
) -> Desc {
    let id = id.into();
    let base = id.to_string();
    let child_id = format!("{base}::widget");
    let child = match control {
        ParamControlSpec::ReadOnly { value } => ui::widget(
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
        ParamControlSpec::Text { value } => ui::widget(
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
        ParamControlSpec::TextArea { value, min_rows } => ui::widget(
            child_id,
            TextAreaProps {
                label: None,
                value: Cow::Owned(value.clone()),
                disabled: false,
                size: metrics.size,
                density: metrics.density,
                min_rows: *min_rows,
            },
        )
        .build(),
        ParamControlSpec::Number {
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
        ParamControlSpec::Slider {
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
        ParamControlSpec::Toggle { checked } => ui::widget(
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
        ParamControlSpec::Select { options, selected } => ui::widget(
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
        ParamControlSpec::Color { rgba } => ui::widget(
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
        ParamControlSpec::FilePath { path, extensions } => ui::widget(
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

    let mut wrapper = ui::row(id)
        .fixed_width(metrics.control_width)
        .justify_content(Justify::Start)
        .align_items(Align::Center)
        .child(child);
    wrapper = if matches!(control, ParamControlSpec::TextArea { .. }) {
        let min_height = param_control_min_height(control, theme, metrics);
        wrapper.fill_height().min_height(min_height).flex_grow(1.0)
    } else {
        wrapper.fixed_height(metrics.control_height)
    };
    wrapper.build()
}

fn text_area_min_height(min_rows: usize, theme: &Theme, metrics: ParamControlMetrics) -> f32 {
    let tokens = theme.text_field_metrics(metrics.size, metrics.density);
    let line_height = tokens.value_size * 1.2;
    min_rows.max(1) as f32 * line_height + tokens.padding_y * 2.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn slider_param_control_adapts_to_widget() {
        let theme = light_theme();
        let metrics = ParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &ParamControlSpec::Slider {
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
        let metrics = ParamControlMetrics::from_theme(&theme);

        let Desc::Container { children, .. } = param_control(
            "control",
            &ParamControlSpec::Color {
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

    #[test]
    fn text_area_param_control_fills_available_height_from_min_rows() {
        let theme = light_theme();
        let metrics = ParamControlMetrics::from_theme(&theme);

        let Desc::Container {
            style, children, ..
        } = param_control(
            "control",
            &ParamControlSpec::TextArea {
                value: "line".to_string(),
                min_rows: 5,
            },
            &theme,
            metrics,
        )
        else {
            panic!("text area control should build a wrapper container");
        };

        assert_eq!(style.width, Size::Fixed(metrics.control_width));
        assert_eq!(style.height, Size::Fill);
        assert_eq!(style.flex_grow, 1.0);
        assert!(style.min_height > metrics.control_height);
        match &children[0] {
            Desc::Widget(widget) => assert_eq!(widget.props().widget_type(), "TextArea"),
            _ => panic!("expected text area widget"),
        }
    }
}
