use super::{
    button::mount_button, color::mount_color_control, file_path::mount_file_path,
    group::mount_group, image::mount_image, label::mount_label, number::mount_number,
    read_only::mount_read_only, select::mount_select, slider::mount_slider, text::mount_text,
    text_area::mount_text_area, toggle::mount_toggle,
};
use crate::control::{
    ControlHeight, ControlInteractionSpec, ControlKind, ControlLayoutPolicy, ControlMetrics,
    ControlNode, ControlSpec,
};
use crate::renderer::TextStyle;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::Align;
use crate::tree::NodeId;

pub(crate) fn control_kind(control: &ControlSpec) -> ControlKind {
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

pub(crate) fn control_layout_policy(
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

pub(crate) fn control_min_height(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> f32 {
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
                    .map(|child| control_min_height(child.spec(), theme, metrics)),
            );
            theme.components.group.padding * 2.0
                + stacked_min_height(child_heights, theme.components.group.gap)
        }
        ControlSpec::TextArea { min_rows, .. } => text_area_min_height(*min_rows, theme, metrics),
        _ => metrics.control_height,
    }
}

pub(crate) fn control_list_min_height<'a>(
    controls: impl Iterator<Item = &'a ControlNode>,
    theme: &Theme,
    metrics: ControlMetrics,
    gap: f32,
) -> f32 {
    stacked_min_height(
        controls.map(|control| control_min_height(control.spec(), theme, metrics)),
        gap,
    )
}

pub(crate) fn mount_control_content(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    match control {
        ControlSpec::Button { label } => mount_button(cx, parent, id, label, theme),
        ControlSpec::Label { text, muted } => mount_label(cx, parent, id, text, theme, *muted),
        ControlSpec::Image {
            texture,
            image_style,
            min_height,
        } => mount_image(cx, parent, id, *texture, *image_style, *min_height),
        ControlSpec::Group { title, children } => {
            mount_group(cx, parent, id, title, children, theme, metrics)
        }
        ControlSpec::Text { value } => mount_text(cx, parent, id, value, theme, metrics),
        ControlSpec::TextArea { value, min_rows } => {
            mount_text_area(cx, parent, id, value, *min_rows, theme, metrics)
        }
        ControlSpec::ReadOnly { value } => mount_read_only(cx, parent, id, value, theme),
        ControlSpec::Number {
            value, precision, ..
        } => mount_number(cx, parent, id, *value, *precision, theme, metrics),
        ControlSpec::Slider {
            value, min, max, ..
        } => mount_slider(
            cx,
            parent,
            id,
            *value,
            *min,
            *max,
            control_interaction_spec(control),
            theme,
            metrics,
        ),
        ControlSpec::Toggle { checked } => mount_toggle(
            cx,
            parent,
            id,
            *checked,
            control_interaction_spec(control),
            theme,
            metrics,
        ),
        ControlSpec::Select { options, selected } => {
            mount_select(cx, parent, id, options, *selected, theme)
        }
        ControlSpec::Color { rgba } => mount_color_control(cx, parent, id, *rgba, theme, metrics),
        ControlSpec::FilePath { path, .. } => mount_file_path(cx, parent, id, path, theme),
    }
}

pub(crate) fn control_interaction_spec(control: &ControlSpec) -> ControlInteractionSpec {
    match control {
        ControlSpec::Slider {
            value,
            min,
            max,
            step,
        } => ControlInteractionSpec::slider(*value, *min, *max, *step),
        ControlSpec::Toggle { checked } => ControlInteractionSpec::toggle(*checked),
        _ => ControlInteractionSpec::None,
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
    fn registry_identifies_every_control_kind() {
        let controls = [
            (ControlSpec::button("Run"), ControlKind::Button),
            (ControlSpec::label("Label"), ControlKind::Label),
            (ControlSpec::read_only("Value"), ControlKind::ReadOnly),
            (ControlSpec::text("Text"), ControlKind::Text),
            (ControlSpec::text_area("Text", 2), ControlKind::TextArea),
            (
                ControlSpec::number(1.0, 0.0, 2.0, 1.0, 0),
                ControlKind::Number,
            ),
            (ControlSpec::slider(1.0, 0.0, 2.0, 1.0), ControlKind::Slider),
            (ControlSpec::toggle(true), ControlKind::Toggle),
            (
                ControlSpec::select(vec!["A".to_string()], 0),
                ControlKind::Select,
            ),
            (ControlSpec::color([1.0, 0.0, 0.0, 1.0]), ControlKind::Color),
            (
                ControlSpec::file_path("/tmp/a.png", vec!["png".to_string()]),
                ControlKind::FilePath,
            ),
            (
                ControlSpec::image(
                    crate::tree::layout::TextureHandle(1),
                    crate::renderer::ImageStyle::default(),
                ),
                ControlKind::Image,
            ),
            (ControlSpec::group("Group", Vec::new()), ControlKind::Group),
        ];

        for (spec, kind) in controls {
            assert_eq!(control_kind(&spec), kind);
        }
    }

    #[test]
    fn registry_exposes_interaction_specs_for_interactive_controls() {
        assert!(
            control_interaction_spec(&ControlSpec::slider(1.0, 0.0, 2.0, 1.0)).is_interactive()
        );
        assert!(control_interaction_spec(&ControlSpec::toggle(true)).is_interactive());
        assert!(!control_interaction_spec(&ControlSpec::label("Label")).is_interactive());
    }

    #[test]
    fn registry_preserves_group_min_height() {
        let theme = light_theme();
        let metrics = ControlMetrics::from_theme(&theme);
        let spec = ControlSpec::group("Group", vec![ControlNode::label("status", "Ready")]);

        assert!(control_min_height(&spec, &theme, metrics) > metrics.control_height);
    }
}
