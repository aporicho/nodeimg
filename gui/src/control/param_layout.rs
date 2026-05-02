use super::ParamControlSpec;
use crate::theme::{ControlSize, Density, Theme};
use crate::tree::layout::Align;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamControlKind {
    ReadOnly,
    Text,
    TextArea,
    Number,
    Slider,
    Toggle,
    Select,
    Color,
    FilePath,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamControlHeight {
    Fixed(f32),
    Fill { min_height: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamControlLayoutPolicy {
    pub kind: ParamControlKind,
    pub height: ParamControlHeight,
    pub row_align: Align,
    pub wrapper_align: Align,
    pub affects_parent_height: bool,
}

impl ParamControlLayoutPolicy {
    pub fn min_height(self) -> f32 {
        match self.height {
            ParamControlHeight::Fixed(height) => height,
            ParamControlHeight::Fill { min_height } => min_height,
        }
    }

    pub fn fills_parent_height(self) -> bool {
        matches!(self.height, ParamControlHeight::Fill { .. })
    }
}

pub fn param_control_layout_policy(
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: ParamControlMetrics,
) -> ParamControlLayoutPolicy {
    let kind = param_control_kind(control);
    match control {
        ParamControlSpec::TextArea { min_rows, .. } => ParamControlLayoutPolicy {
            kind,
            height: ParamControlHeight::Fill {
                min_height: text_area_min_height(*min_rows, theme, metrics),
            },
            row_align: Align::Stretch,
            wrapper_align: Align::Stretch,
            affects_parent_height: true,
        },
        _ => ParamControlLayoutPolicy {
            kind,
            height: ParamControlHeight::Fixed(metrics.control_height),
            row_align: Align::Center,
            wrapper_align: Align::Center,
            affects_parent_height: false,
        },
    }
}

pub fn param_control_min_height(
    control: &ParamControlSpec,
    theme: &Theme,
    metrics: ParamControlMetrics,
) -> f32 {
    param_control_layout_policy(control, theme, metrics).min_height()
}

pub fn param_control_kind(control: &ParamControlSpec) -> ParamControlKind {
    match control {
        ParamControlSpec::ReadOnly { .. } => ParamControlKind::ReadOnly,
        ParamControlSpec::Text { .. } => ParamControlKind::Text,
        ParamControlSpec::TextArea { .. } => ParamControlKind::TextArea,
        ParamControlSpec::Number { .. } => ParamControlKind::Number,
        ParamControlSpec::Slider { .. } => ParamControlKind::Slider,
        ParamControlSpec::Toggle { .. } => ParamControlKind::Toggle,
        ParamControlSpec::Select { .. } => ParamControlKind::Select,
        ParamControlSpec::Color { .. } => ParamControlKind::Color,
        ParamControlSpec::FilePath { .. } => ParamControlKind::FilePath,
    }
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
    fn text_area_layout_policy_fills_and_stretches() {
        let theme = light_theme();
        let metrics = ParamControlMetrics::from_theme(&theme);
        let policy = param_control_layout_policy(
            &ParamControlSpec::TextArea {
                value: "line".to_string(),
                min_rows: 5,
            },
            &theme,
            metrics,
        );

        assert_eq!(policy.kind, ParamControlKind::TextArea);
        assert!(policy.fills_parent_height());
        assert_eq!(policy.row_align, Align::Stretch);
        assert_eq!(policy.wrapper_align, Align::Stretch);
        assert!(policy.affects_parent_height);
        assert!(policy.min_height() > metrics.control_height);
    }

    #[test]
    fn fixed_controls_keep_centered_policy() {
        let theme = light_theme();
        let metrics = ParamControlMetrics::from_theme(&theme);
        let policy = param_control_layout_policy(
            &ParamControlSpec::Slider {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            &theme,
            metrics,
        );

        assert_eq!(policy.kind, ParamControlKind::Slider);
        assert_eq!(
            policy.height,
            ParamControlHeight::Fixed(metrics.control_height)
        );
        assert_eq!(policy.row_align, Align::Center);
        assert_eq!(policy.wrapper_align, Align::Center);
        assert!(!policy.affects_parent_height);
    }
}
