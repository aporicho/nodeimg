use super::node_factory::{container, leaf};
use super::style::{ellipsis_text_layout, port_border_color, port_state_color};
use crate::canvas::node_spec::{NodePortGroupTriggerSpec, NodePortSpec};
use crate::canvas::node_style::NodeCardMetrics;
use crate::canvas::{CanvasPortConnectionState, CanvasPortSide};
use crate::gesture::Gesture;
use crate::icon::{names, IconSpec};
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, LeafKind, Overflow, Position, Size,
};
use crate::tree::NodeId;

pub(super) fn mount_pin_column(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    trigger: &NodePortGroupTriggerSpec,
    side: CanvasPortSide,
    ports: &[NodePortSpec],
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let column = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                position: Position::relative(),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Column,
                gap: metrics.pin_row_gap,
                align_items: Align::Start,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                hittable: false,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;

    for port in ports {
        mount_pin_row(cx, column, side, port, theme, metrics)?;
    }
    if !ports.is_empty() {
        mount_port_group_trigger(cx, column, trigger, theme, metrics)?;
    }
    Ok(())
}

fn mount_port_group_trigger(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    trigger: &NodePortGroupTriggerSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let diameter = metrics.port_group_trigger_diameter;
    let (x, y) = port_group_trigger_offset(trigger.side, metrics);
    let border_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.border
    };
    let icon_color = if trigger.open {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };
    let trigger_node = cx.child(
        parent,
        container(
            trigger.id.clone(),
            BoxStyle {
                position: Position::absolute_xy(x, y),
                width: Size::Fixed(diameter),
                height: Size::Fixed(diameter),
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Center,
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: if trigger.open { 2.0 } else { 1.5 },
                    color: border_color,
                }),
                radius: [diameter * 0.5; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        trigger_node,
        leaf(
            format!("{}::icon", trigger.id),
            LeafKind::Icon {
                spec: IconSpec::new(names::PLUS, icon_color),
            },
            BoxStyle {
                width: Size::Fixed(metrics.port_group_trigger_icon_size),
                height: Size::Fixed(metrics.port_group_trigger_icon_size),
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn port_group_trigger_offset(side: CanvasPortSide, metrics: NodeCardMetrics) -> (f32, f32) {
    let diameter = metrics.port_group_trigger_diameter;
    let dot_center_offset = (metrics.pin_dot_diameter - diameter) * 0.5;
    let x = match side {
        CanvasPortSide::Input => dot_center_offset,
        CanvasPortSide::Output => {
            metrics.pin_label_width + metrics.pin_label_gap + dot_center_offset
        }
    };
    let y = -diameter - metrics.pin_row_gap;
    (x, y)
}

fn mount_pin_row(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    side: CanvasPortSide,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let row = cx.child(
        parent,
        container(
            port.row_id.clone(),
            BoxStyle {
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.pin_label_gap,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Start,
                hittable: true,
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    if side == CanvasPortSide::Output {
        mount_pin_label(cx, row, port, theme, metrics)?;
        mount_pin_dot(cx, row, port, theme, metrics)?;
    } else {
        mount_pin_dot(cx, row, port, theme, metrics)?;
        mount_pin_label(cx, row, port, theme, metrics)?;
    }
    Ok(())
}

fn mount_pin_dot(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            port.dot_id.clone(),
            LeafKind::Circle {
                radius: metrics.pin_dot_diameter * 0.5,
                fill: Some(port_state_color(port, theme)),
                stroke: Some(Border {
                    width: match port.connection_state {
                        CanvasPortConnectionState::Idle => 1.0,
                        CanvasPortConnectionState::Source
                        | CanvasPortConnectionState::CompatibleTarget
                        | CanvasPortConnectionState::IncompatibleTarget
                        | CanvasPortConnectionState::DropTarget
                        | CanvasPortConnectionState::RejectedDropTarget => 2.0,
                    },
                    color: port_border_color(port.connection_state, theme),
                }),
            },
            BoxStyle {
                width: Size::Fixed(metrics.pin_dot_diameter),
                height: Size::Fixed(metrics.pin_dot_diameter),
                hittable: true,
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

fn mount_pin_label(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    port: &NodePortSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            port.label_id.clone(),
            LeafKind::Text {
                content: port.name.clone(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fixed(metrics.pin_label_width),
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}
