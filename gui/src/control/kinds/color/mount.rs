use crate::control::mount::{container, mount_control_text};
use crate::control::ControlMetrics;
use crate::renderer::{Border, Color};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Size};
use crate::tree::NodeId;

pub(crate) fn mount_color_control(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    rgba: [f32; 4],
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.control_height),
                direction: Direction::Row,
                gap: metrics.control_height * 0.25,
                align_items: Align::Center,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        root,
        container(
            format!("{id}::swatch"),
            BoxStyle {
                width: Size::Fixed(metrics.control_height * 0.75),
                height: Size::Fixed(metrics.control_height * 0.75),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(Color {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: rgba[3],
                }),
                border: Some(Border {
                    width: 1.0,
                    color: theme.colors.border,
                }),
                radius: [theme.radii.sm; 4],
                shadow: None,
            }),
        ),
    )?;
    mount_control_text(
        cx,
        root,
        &format!("{id}::value"),
        &format!(
            "#{:02X}{:02X}{:02X}",
            (rgba[0].clamp(0.0, 1.0) * 255.0) as u8,
            (rgba[1].clamp(0.0, 1.0) * 255.0) as u8,
            (rgba[2].clamp(0.0, 1.0) * 255.0) as u8
        ),
        theme,
    )
}
