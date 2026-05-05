use super::node_factory::{container, leaf};
use super::text_leaf::ellipsis_text_layout;
use crate::control::ControlMetrics;
use crate::gesture::Gesture;
use crate::renderer::{Border, TextStyle};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{
    BoxStyle, Decoration, Edges, Inset, LeafKind, Overflow, Position, Size, TextLayout,
};
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(crate) fn mount_text_field(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    value: &str,
    multiline: bool,
    min_rows: usize,
    role: SemanticRole,
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
                height: if multiline {
                    Size::Fill
                } else {
                    Size::Fixed(field_tokens.field_height)
                },
                min_height: if multiline {
                    min_rows.max(1) as f32 * field_tokens.value_size * 1.2
                        + field_tokens.padding_y * 2.0
                } else {
                    field_tokens.field_height
                },
                flex_grow: multiline.then_some(1.0).unwrap_or(0.0),
                ..BoxStyle::default()
            },
            None,
        )
        .semantic_role(role),
    )?;
    let field = cx.child(
        root,
        container(
            format!("{id}::field"),
            BoxStyle {
                width: Size::Fill,
                height: if multiline {
                    Size::Fill
                } else {
                    Size::Fixed(field_tokens.field_height)
                },
                min_height: if multiline {
                    min_rows.max(1) as f32 * field_tokens.value_size * 1.2
                        + field_tokens.padding_y * 2.0
                } else {
                    field_tokens.field_height
                },
                padding: Edges::symmetric(field_tokens.padding_y, field_tokens.padding_x),
                position: Position::relative(),
                overflow: if multiline {
                    Overflow::Scroll
                } else {
                    Overflow::Hidden
                },
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
                content: value.to_string(),
                style: TextStyle::new(theme.colors.text, field_tokens.value_size)
                    .with_family(theme.text.body_family)
                    .with_line_height(theme.text.default_line_height),
                layout: if multiline {
                    TextLayout::default()
                } else {
                    ellipsis_text_layout()
                },
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
