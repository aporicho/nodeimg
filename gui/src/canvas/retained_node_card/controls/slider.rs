use super::super::node_factory::container;
use crate::canvas::node_style::NodeCardMetrics;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Position, Size};
use crate::tree::NodeId;

pub(super) fn mount_slider(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    value: f32,
    min: f32,
    max: f32,
    theme: &Theme,
    metrics: NodeCardMetrics,
) -> Result<(), TemplateError> {
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.control.control_height),
                direction: Direction::Row,
                align_items: Align::Center,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let track = cx.child(
        root,
        container(
            format!("{id}::track"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(4.0),
                position: Position::relative(),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.border),
                border: None,
                radius: [2.0; 4],
                shadow: None,
            }),
        ),
    )?;
    let range = (max - min).abs().max(f32::EPSILON);
    let t = ((value - min) / range).clamp(0.0, 1.0);
    cx.child(
        track,
        container(
            format!("{id}::thumb"),
            BoxStyle {
                position: Position::absolute_xy(t * (metrics.control.control_width - 10.0), -3.0),
                width: Size::Fixed(10.0),
                height: Size::Fixed(10.0),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.accent),
                border: None,
                radius: [5.0; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}
