use super::button::mount_button;
use super::color::mount_color_control;
use super::group::mount_group;
use super::image::mount_image;
use super::label::mount_label;
use super::slider::mount_slider;
use super::text::mount_control_text;
use super::text_field::mount_text_field;
use super::toggle::mount_toggle;
use crate::control::{
    ControlHeight, ControlLayoutPolicy, ControlMetrics, ControlNode, ControlRole, ControlSpec,
};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Direction, Size};
use crate::tree::NodeId;

use super::node_factory::container;

pub(crate) fn mount_control(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
    policy: ControlLayoutPolicy,
) -> Result<(), TemplateError> {
    let wrapper = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fixed(metrics.control_width),
                height: match policy.height {
                    ControlHeight::Fixed(height) => Size::Fixed(height),
                    ControlHeight::Fill { .. } => Size::Fill,
                },
                min_height: policy.min_height(),
                flex_grow: control_flex(policy),
                direction: Direction::Row,
                align_items: policy.wrapper_align,
                justify_content: crate::tree::layout::Justify::Start,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let child_id = format!("{id}::content");
    mount_control_content(cx, wrapper, &child_id, control, theme, metrics)
}

pub(crate) fn mount_control_node(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    node: &ControlNode,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    mount_control_content(cx, parent, node.id(), node.spec(), theme, metrics)
}

fn mount_control_content(
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
        ControlSpec::Text { value } => mount_text_field(
            cx,
            parent,
            id,
            value,
            false,
            1,
            ControlRole::TextInput,
            theme,
            metrics,
        ),
        ControlSpec::TextArea { value, min_rows } => mount_text_field(
            cx,
            parent,
            id,
            value,
            true,
            *min_rows,
            ControlRole::TextArea,
            theme,
            metrics,
        ),
        ControlSpec::ReadOnly { value } => mount_control_text(cx, parent, id, value, theme),
        ControlSpec::Number {
            value, precision, ..
        } => mount_text_field(
            cx,
            parent,
            id,
            &format!("{value:.precision$}"),
            false,
            1,
            ControlRole::NumberInput,
            theme,
            metrics,
        ),
        ControlSpec::Slider {
            value, min, max, ..
        } => mount_slider(cx, parent, id, *value, *min, *max, theme, metrics),
        ControlSpec::Toggle { checked } => mount_toggle(cx, parent, id, *checked, theme, metrics),
        ControlSpec::Select { options, selected } => {
            let value = options.get(*selected).cloned().unwrap_or_default();
            mount_control_text(cx, parent, id, &value, theme)
        }
        ControlSpec::Color { rgba } => mount_color_control(cx, parent, id, *rgba, theme, metrics),
        ControlSpec::FilePath { path, .. } => mount_control_text(cx, parent, id, path, theme),
    }
}

fn control_flex(policy: ControlLayoutPolicy) -> f32 {
    policy.fills_parent_height().then_some(1.0).unwrap_or(0.0)
}
