use super::{ControlNode, ControlSpec};
use crate::renderer::TextStyle;
use crate::theme::{ControlSize, Density, Theme};
use crate::tree::layout::Align;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    pub control_height: f32,
    pub control_width: f32,
    pub size: ControlSize,
    pub density: Density,
}

impl ControlMetrics {
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
pub enum ControlKind {
    Button,
    Label,
    Image,
    Group,
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
pub enum ControlHeight {
    Fixed(f32),
    Fill { min_height: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlLayoutPolicy {
    pub kind: ControlKind,
    pub height: ControlHeight,
    pub row_align: Align,
    pub wrapper_align: Align,
    pub affects_parent_height: bool,
}

impl ControlLayoutPolicy {
    pub fn min_height(self) -> f32 {
        match self.height {
            ControlHeight::Fixed(height) => height,
            ControlHeight::Fill { min_height } => min_height,
        }
    }

    pub fn fills_parent_height(self) -> bool {
        matches!(self.height, ControlHeight::Fill { .. })
    }
}

pub fn control_layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    let kind = control_kind(control);
    match control {
        ControlSpec::Image { min_height, .. } => ControlLayoutPolicy {
            kind,
            height: ControlHeight::Fill {
                min_height: *min_height,
            },
            row_align: Align::Stretch,
            wrapper_align: Align::Stretch,
            affects_parent_height: true,
        },
        ControlSpec::Group { .. } => ControlLayoutPolicy {
            kind,
            height: ControlHeight::Fill {
                min_height: control_min_height(control, theme, metrics),
            },
            row_align: Align::Stretch,
            wrapper_align: Align::Stretch,
            affects_parent_height: true,
        },
        ControlSpec::TextArea { min_rows, .. } => ControlLayoutPolicy {
            kind,
            height: ControlHeight::Fill {
                min_height: text_area_min_height(*min_rows, theme, metrics),
            },
            row_align: Align::Stretch,
            wrapper_align: Align::Stretch,
            affects_parent_height: true,
        },
        _ => ControlLayoutPolicy {
            kind,
            height: ControlHeight::Fixed(control_min_height(control, theme, metrics)),
            row_align: Align::Center,
            wrapper_align: Align::Center,
            affects_parent_height: false,
        },
    }
}

pub fn control_min_height(control: &ControlSpec, theme: &Theme, metrics: ControlMetrics) -> f32 {
    match control {
        ControlSpec::Button { .. } => {
            theme.components.button.font_size + theme.components.button.padding_y * 2.0
        }
        ControlSpec::Label { .. } | ControlSpec::ReadOnly { .. } => {
            text_line_height(theme.text_style_label_sm())
        }
        ControlSpec::Image { min_height, .. } => *min_height,
        ControlSpec::Group { children, .. } => {
            let title = text_line_height(theme.text_style_title_sm());
            let child_heights = std::iter::once(title).chain(
                children
                    .iter()
                    .map(|child| control_node_min_height(child, theme, metrics)),
            );
            theme.components.group.padding * 2.0
                + stacked_min_height(child_heights, theme.components.group.gap)
        }
        ControlSpec::TextArea { min_rows, .. } => text_area_min_height(*min_rows, theme, metrics),
        _ => metrics.control_height,
    }
}

pub fn control_node_min_height(node: &ControlNode, theme: &Theme, metrics: ControlMetrics) -> f32 {
    control_min_height(node.spec(), theme, metrics)
}

pub fn control_list_min_height<'a>(
    controls: impl Iterator<Item = &'a ControlNode>,
    theme: &Theme,
    metrics: ControlMetrics,
    gap: f32,
) -> f32 {
    stacked_min_height(
        controls.map(|control| control_node_min_height(control, theme, metrics)),
        gap,
    )
}

pub fn control_kind(control: &ControlSpec) -> ControlKind {
    match control {
        ControlSpec::Button { .. } => ControlKind::Button,
        ControlSpec::Label { .. } => ControlKind::Label,
        ControlSpec::Image { .. } => ControlKind::Image,
        ControlSpec::Group { .. } => ControlKind::Group,
        ControlSpec::ReadOnly { .. } => ControlKind::ReadOnly,
        ControlSpec::Text { .. } => ControlKind::Text,
        ControlSpec::TextArea { .. } => ControlKind::TextArea,
        ControlSpec::Number { .. } => ControlKind::Number,
        ControlSpec::Slider { .. } => ControlKind::Slider,
        ControlSpec::Toggle { .. } => ControlKind::Toggle,
        ControlSpec::Select { .. } => ControlKind::Select,
        ControlSpec::Color { .. } => ControlKind::Color,
        ControlSpec::FilePath { .. } => ControlKind::FilePath,
    }
}

fn stacked_min_height(heights: impl Iterator<Item = f32>, gap: f32) -> f32 {
    let mut count = 0usize;
    let mut total = 0.0;
    for height in heights {
        count += 1;
        total += height;
    }
    if count > 1 {
        total += gap * (count as f32 - 1.0);
    }
    total
}

fn text_line_height(style: TextStyle) -> f32 {
    style.size * style.line_height
}

fn text_area_min_height(min_rows: usize, theme: &Theme, metrics: ControlMetrics) -> f32 {
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
        let metrics = ControlMetrics::from_theme(&theme);
        let policy = control_layout_policy(
            &ControlSpec::TextArea {
                value: "line".to_string(),
                min_rows: 5,
            },
            &theme,
            metrics,
        );

        assert_eq!(policy.kind, ControlKind::TextArea);
        assert!(policy.fills_parent_height());
        assert_eq!(policy.row_align, Align::Stretch);
        assert_eq!(policy.wrapper_align, Align::Stretch);
        assert!(policy.affects_parent_height);
        assert!(policy.min_height() > metrics.control_height);
    }

    #[test]
    fn fixed_controls_keep_centered_policy() {
        let theme = light_theme();
        let metrics = ControlMetrics::from_theme(&theme);
        let policy = control_layout_policy(
            &ControlSpec::Slider {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            },
            &theme,
            metrics,
        );

        assert_eq!(policy.kind, ControlKind::Slider);
        assert_eq!(policy.height, ControlHeight::Fixed(metrics.control_height));
        assert_eq!(policy.row_align, Align::Center);
        assert_eq!(policy.wrapper_align, Align::Center);
        assert!(!policy.affects_parent_height);
    }

    #[test]
    fn control_list_min_height_counts_groups_and_gaps() {
        let theme = light_theme();
        let metrics = ControlMetrics::from_theme(&theme);
        let controls = [
            ControlNode::button("run", "Run"),
            ControlNode::group(
                "runtime",
                "Runtime",
                vec![ControlNode::label("status", "Idle")],
            ),
        ];

        assert!(control_list_min_height(controls.iter(), &theme, metrics, theme.spacing.sm) > 0.0);
    }
}
