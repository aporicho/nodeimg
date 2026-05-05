use super::node_factory::{container, leaf};
use super::text::ellipsis_text_layout;
use crate::icon::{names, IconSpec};
use crate::interaction::ControlVisualState;
use crate::overlay::DropdownOverlayContent;
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::Gesture;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, Justify, LeafKind, Overflow, Size,
};
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(in crate::overlay::retained) fn mount_option(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    index: usize,
    label: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let metrics = theme.control_metrics(content.size, content.density);
    let highlighted = index == content.highlighted;
    let selected = index == content.selected;
    let visual_state = if highlighted {
        ControlVisualState::Focused
    } else {
        ControlVisualState::Normal
    };
    let visual = theme.button_visual(visual_state);
    let marker_color = if selected {
        theme.colors.accent
    } else {
        theme.colors.text_muted
    };
    let option_id = format!("__dropdown_option::{}::{}", content.dropdown_id, index);
    let option_root = cx.child(
        parent,
        container(
            option_id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(metrics.height),
                padding: Edges::symmetric(metrics.padding_y, metrics.padding_x),
                gap: metrics.gap,
                direction: Direction::Row,
                align_items: Align::Center,
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.background),
                border: visual.border.map(|color| Border {
                    width: metrics.border_width,
                    color,
                }),
                radius: [metrics.radius; 4],
                shadow: None,
            }),
        )
        .semantic_role(SemanticRole::Button),
    )?;
    let marker = cx.child(
        option_root,
        container(
            format!("{option_id}::marker"),
            BoxStyle {
                width: Size::Fixed(metrics.icon_size),
                height: Size::Fixed(metrics.icon_size),
                align_items: Align::Center,
                justify_content: Justify::Center,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let marker_icon = if selected {
        Some(names::CHECK)
    } else if highlighted {
        Some(names::NAV_ARROW_RIGHT)
    } else {
        None
    };
    if let Some(icon) = marker_icon {
        cx.child(
            marker,
            leaf(
                format!("{option_id}::marker_icon"),
                LeafKind::Icon {
                    spec: IconSpec::new(icon, marker_color),
                },
                BoxStyle {
                    width: Size::Fixed(metrics.icon_size),
                    height: Size::Fixed(metrics.icon_size),
                    ..BoxStyle::default()
                },
            ),
        )?;
    }
    let mut text_style = theme.text_style_body_sm();
    text_style.color = visual.text;
    text_style.size = metrics.font_size;
    cx.child(
        option_root,
        leaf(
            format!("{option_id}::label"),
            LeafKind::Text {
                content: label.to_string(),
                style: text_style,
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
    Ok(())
}
