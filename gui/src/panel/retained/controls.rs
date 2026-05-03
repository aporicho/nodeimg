use super::node_factory::{container, leaf, TreeNodeExt};
use super::text::{ellipsis_text_layout, label_style};
use crate::control::ControlRole;
use crate::gesture::Gesture;
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Size};
use crate::tree::NodeId;

pub(in crate::panel::retained) fn mount_button(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    label: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let visual = theme.button_visual(crate::interaction::ControlVisualState::Normal);
    let button = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(
                    theme.components.button.font_size + theme.components.button.padding_y * 2.0,
                ),
                padding: Edges::symmetric(
                    theme.components.button.padding_y,
                    theme.components.button.padding_x,
                ),
                direction: Direction::Row,
                align_items: Align::Center,
                justify_content: Justify::Center,
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.background),
                border: visual.border.map(|color| Border {
                    width: theme.components.button.border_width,
                    color,
                }),
                radius: [theme.components.button.radius; 4],
                shadow: None,
            }),
        )
        .with_semantic_role(ControlRole::Button),
    )?;
    cx.child(
        button,
        leaf(
            format!("{id}::label"),
            LeafKind::Text {
                content: label.to_string(),
                style: crate::renderer::TextStyle::new(
                    visual.text,
                    theme.components.button.font_size,
                )
                .with_family(theme.text.body_family)
                .with_line_height(theme.text.default_line_height),
                layout: ellipsis_text_layout(),
            },
            BoxStyle {
                width: Size::Auto,
                height: Size::Auto,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}

pub(in crate::panel::retained) fn mount_label(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    text: &str,
    theme: &Theme,
    muted: bool,
) -> Result<(), TemplateError> {
    let mut style = theme.text_style_label_sm();
    style.color = if muted {
        theme.colors.text_muted
    } else {
        theme.colors.text
    };
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Text {
                content: text.to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}
