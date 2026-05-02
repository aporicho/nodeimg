use super::node_factory::{container, leaf};
use super::style::ellipsis_text_layout;
use crate::canvas::node_spec::NodeHeaderSpec;
use crate::canvas::node_style::NodeCardMetrics;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Align, BoxStyle, Direction, LeafKind, Position, Size};
use crate::tree::NodeId;

pub(super) fn mount_node_header(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    header: &NodeHeaderSpec,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let row = cx.child(
        parent,
        container(
            header.row_id.clone(),
            BoxStyle {
                position: Position::absolute_xy(0.0, -metrics.title_lift),
                width: Size::Auto,
                height: Size::Auto,
                direction: Direction::Row,
                gap: metrics.title_label_gap,
                align_items: Align::Center,
                hittable: false,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        row,
        leaf(
            header.dot_id.clone(),
            LeafKind::Circle {
                radius: metrics.title_dot_diameter * 0.5,
                fill: Some(header.category_color),
                stroke: None,
            },
            BoxStyle {
                width: Size::Fixed(metrics.title_dot_diameter),
                height: Size::Fixed(metrics.title_dot_diameter),
                ..BoxStyle::default()
            },
        ),
    )?;
    cx.child(
        row,
        leaf(
            header.text_id.clone(),
            LeafKind::Text {
                content: header.title.clone(),
                style: theme.text_style_label_sm(),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Fixed(metrics.title_label_width),
                height: Size::Auto,
                flex_shrink: 1.0,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}
