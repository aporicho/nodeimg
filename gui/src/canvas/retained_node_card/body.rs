use super::node_factory::{container, leaf};
use super::style::ellipsis_text_layout;
use crate::canvas::node_spec::{NodeBodyRowSpec, NodeBodySpec};
use crate::canvas::node_style::NodeCardMetrics;
use crate::control::templates::mount_control;
use crate::control::{control_layout_policy, ControlHeight, ControlLayoutPolicy};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, LeafKind, Size};
use crate::tree::{NodeId, TreeNode};

pub(super) fn mount_node_body(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    body: &NodeBodySpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let body_node = cx.child(
        parent,
        container(
            body.id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fill,
                flex_grow: 1.0,
                gap: metrics.row_gap,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    for row in &body.rows {
        mount_body_row(cx, body_node, row, theme, metrics)?;
    }
    Ok(())
}

fn mount_body_row(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    row: &NodeBodyRowSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    match row {
        NodeBodyRowSpec::Summary { id, text_id, text } => {
            let row = cx.child(
                parent,
                body_row(
                    id,
                    theme,
                    metrics,
                    Align::Center,
                    Size::Fixed(metrics.param_row_height),
                    0.0,
                ),
            )?;
            cx.child(
                row,
                leaf(
                    text_id.clone(),
                    LeafKind::Text {
                        content: text.clone(),
                        style: theme.text_style_label_sm(),
                        layout: ellipsis_text_layout(),
                    },
                    BoxStyle {
                        width: Size::Fill,
                        height: Size::Auto,
                        flex_shrink: 1.0,
                        ..BoxStyle::default()
                    },
                ),
            )?;
        }
        NodeBodyRowSpec::Param {
            id,
            control_id,
            control,
        } => {
            let policy = control_layout_policy(control, theme, metrics.control);
            let row = cx.child(
                parent,
                body_row(
                    id,
                    theme,
                    metrics,
                    policy.row_align,
                    row_height(policy, metrics),
                    row_flex(policy),
                ),
            )?;
            mount_control(cx, row, control_id, control, theme, metrics.control, policy)?;
        }
    }
    Ok(())
}

fn body_row(
    id: &str,
    theme: &Theme,
    metrics: NodeCardMetrics,
    align_items: Align,
    height: Size,
    flex_grow: f32,
) -> TreeNode {
    container(
        id.to_string(),
        BoxStyle {
            width: Size::Fill,
            height,
            min_height: match height {
                Size::Fixed(value) => value,
                Size::Fill => metrics.param_row_height,
                Size::Auto => 0.0,
            },
            flex_grow,
            direction: Direction::Row,
            gap: metrics.param_label_gap,
            align_items,
            justify_content: crate::tree::layout::Justify::Start,
            ..BoxStyle::default()
        },
        Some(Decoration {
            background: Some(theme.colors.canvas_bg),
            border: None,
            radius: [metrics.row_radius; 4],
            shadow: None,
        }),
    )
}

fn row_height(policy: ControlLayoutPolicy, metrics: NodeCardMetrics) -> Size {
    match policy.height {
        ControlHeight::Fixed(height) => Size::Fixed(metrics.param_row_height.max(height)),
        ControlHeight::Fill { .. } => Size::Fill,
    }
}

fn row_flex(policy: ControlLayoutPolicy) -> f32 {
    policy.fills_parent_height().then_some(1.0).unwrap_or(0.0)
}
