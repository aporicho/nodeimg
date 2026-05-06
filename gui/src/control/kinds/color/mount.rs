use crate::control::mount::{container, ellipsis_text_layout, leaf};
use crate::control::{format_color_hex, ControlMetrics};
use crate::renderer::{Border, Color, TextStyle};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, Gesture, Inset, LeafKind, Overflow, Position,
    Size,
};
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(crate) fn mount_color_control(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    rgba: [f32; 4],
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let field_tokens = theme.text_field_metrics(metrics.size, metrics.density);
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
        )
        .semantic_role(SemanticRole::TextInput),
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
    let field = cx.child(
        root,
        container(
            format!("{id}::field"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(field_tokens.field_height),
                padding: Edges::symmetric(field_tokens.padding_y, field_tokens.padding_x),
                position: Position::relative(),
                overflow: Overflow::Hidden,
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: field_tokens.border_width,
                    color: theme.colors.border,
                }),
                radius: [field_tokens.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        field,
        container(
            format!("{id}::selection"),
            BoxStyle {
                position: Position::absolute_inset(Inset::ZERO),
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    cx.child(
        field,
        leaf(
            format!("{id}::value"),
            LeafKind::Text {
                content: format_color_hex(rgba),
                style: TextStyle::new(theme.colors.text, field_tokens.value_size)
                    .with_family(theme.text.mono_family)
                    .with_line_height(theme.text.default_line_height),
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
    cx.child(
        field,
        container(
            format!("{id}::caret"),
            BoxStyle {
                position: Position::absolute_xy(0.0, 0.0),
                width: Size::Fixed(1.5),
                height: Size::Fixed(field_tokens.value_size * 1.2),
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.caret),
                border: None,
                radius: [1.0; 4],
                shadow: None,
            }),
        ),
    )?;
    Ok(())
}
