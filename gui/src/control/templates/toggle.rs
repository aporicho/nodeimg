use super::node_factory::{container, TreeNodeExt};
use crate::control::{ControlInteractionSpec, ControlMetrics, ControlRole};
use crate::gesture::Gesture;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Position, Size};
use crate::tree::NodeId;

pub(super) fn mount_toggle(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    checked: bool,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let width = metrics.control_height * 1.65;
    let height = metrics.control_height * 0.72;
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fixed(width),
                height: Size::Fixed(height),
                position: Position::relative(),
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(if checked {
                    theme.colors.accent
                } else {
                    theme.colors.border
                }),
                border: None,
                radius: [height * 0.5; 4],
                shadow: None,
            }),
        )
        .with_semantic_role(ControlRole::Toggle)
        .with_runtime_slot(ControlInteractionSpec::toggle(checked)),
    )?;
    let knob = height - 4.0;
    cx.child(
        root,
        container(
            format!("{id}::knob"),
            BoxStyle {
                position: Position::absolute_xy(
                    if checked { width - knob - 2.0 } else { 2.0 },
                    2.0,
                ),
                width: Size::Fixed(knob),
                height: Size::Fixed(knob),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: None,
                radius: [knob * 0.5; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}
