use super::{param_control_layout_policy, ParamControlHeight, ParamControlMetrics};
use crate::theme::Theme;
use crate::tree::layout::{Justify, Size, TextAlign, TextOverflow};
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

pub fn param_control(
    id: impl Into<Cow<'static, str>>,
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: ParamControlMetrics,
) -> Desc {
    let id = id.into();
    let base = id.to_string();
    let child_id = format!("{base}::widget");
    let child = control_widget(child_id, control, metrics);
    let policy = param_control_layout_policy(control, theme, metrics);

    let wrapper = ui::row(id)
        .fixed_width(metrics.control_width)
        .justify_content(Justify::Start)
        .align_items(policy.wrapper_align)
        .child(child);

    match policy.height {
        ParamControlHeight::Fixed(height) => wrapper.fixed_height(height),
        ParamControlHeight::Fill { min_height } => {
            wrapper.fill_height().min_height(min_height).flex_grow(1.0)
        }
    }
    .build()
}

fn control_widget(
    child_id: String,
    control: &ParamControlSpec,
    metrics: ParamControlMetrics,
) -> Desc {
    match control {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;
    use crate::tree::layout::{Align, Size};

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
    fn text_area_param_control_fills_available_height_from_policy() {
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
        assert_eq!(style.align_items, Align::Stretch);
        assert!(style.min_height > metrics.control_height);
        match &children[0] {
            Desc::Widget(widget) => assert_eq!(widget.props().widget_type(), "TextArea"),
            _ => panic!("expected text area widget"),
        }
    }
}
