use super::body::mount_node_body;
use super::card::mount_card;
use super::header::mount_node_header;
use super::node_factory::{container, TreeNodeExt};
use super::ports::mount_pin_column;
use crate::canvas::node_spec::NodeRenderSpec;
use crate::canvas::CanvasPortSide;
use crate::gesture::Gesture;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Direction, Overflow, Position, RelayoutBoundaryReason, Size,
};
use crate::tree::{NodeId, RectMoveInvalidation, RepaintBoundaryReason};

pub(super) fn mount_node_card(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    spec: &NodeRenderSpec,
    theme: &Theme,
) -> Result<NodeId, TemplateError> {
    let metrics = spec.metrics;
    let root = cx.child(
        parent,
        container(
            spec.id.clone(),
            BoxStyle {
                position: Position::absolute_xy(spec.layout.rect.x, spec.layout.rect.y),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.column_gap,
                align_items: Align::Center,
                justify_content: crate::tree::layout::Justify::Start,
                overflow: Overflow::Visible,
                z_index: spec.layout.z_index,
                hittable: true,
                draggable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            None,
        )
        .with_layout_boundary(RelayoutBoundaryReason::CanvasNodeCard)
        .with_paint_boundary(RepaintBoundaryReason::CanvasNodeCard)
        .with_rect_move_invalidation(RectMoveInvalidation::BoundaryPlacement),
    )?;

    mount_pin_column(
        cx,
        root,
        &spec.input_column_id,
        &spec.input_trigger,
        CanvasPortSide::Input,
        &spec.inputs,
        theme,
        metrics,
    )?;
    let card = mount_card(cx, root, spec, theme, metrics)?;
    mount_node_body(cx, card, &spec.body, theme, metrics)?;
    mount_node_header(cx, card, &spec.header, theme, metrics)?;
    mount_pin_column(
        cx,
        root,
        &spec.output_column_id,
        &spec.output_trigger,
        CanvasPortSide::Output,
        &spec.outputs,
        theme,
        metrics,
    )?;

    Ok(root)
}
