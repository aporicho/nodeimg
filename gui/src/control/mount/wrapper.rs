use crate::control::{
    ControlHeight, ControlLayoutPolicy, ControlMetrics, ControlNode, ControlSpec,
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
    crate::control::kinds::registry::mount_control_content(
        cx, wrapper, &child_id, control, theme, metrics,
    )
}

pub(crate) fn mount_control_node(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    node: &ControlNode,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    crate::control::kinds::registry::mount_control_content(
        cx,
        parent,
        node.id(),
        node.spec(),
        theme,
        metrics,
    )
}

fn control_flex(policy: ControlLayoutPolicy) -> f32 {
    policy.fills_parent_height().then_some(1.0).unwrap_or(0.0)
}
