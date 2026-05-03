use super::node_factory::{container, leaf};
use super::option::mount_option;
use super::text::{ellipsis_text_layout, label_style};
use crate::overlay::DropdownOverlayContent;
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Direction, Edges, LeafKind, Overflow, Size};
use crate::tree::NodeId;

const POPUP_HEIGHT: f32 = 140.0;

pub(in crate::overlay::retained) fn mount_dropdown_group(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let group_id = format!("{}::popup_group", content.dropdown_id);
    let tokens = theme.components.group;
    let group = cx.child(
        parent,
        container(
            group_id.clone(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::all(tokens.padding),
                gap: tokens.title_gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(tokens.background),
                border: Some(Border {
                    width: tokens.border_width,
                    color: tokens.border,
                }),
                radius: [tokens.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    mount_group_title(cx, group, &group_id, content.title.as_ref(), theme)?;
    let content_root = cx.child(
        group,
        container(
            format!("{group_id}::content"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                gap: tokens.gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    mount_hint(cx, content_root, content, theme)?;
    mount_option_list(cx, content_root, content, theme)?;
    Ok(())
}

fn mount_group_title(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    group_id: &str,
    title: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let tokens = theme.components.group;
    let titlebar = cx.child(
        parent,
        container(
            format!("{group_id}::titlebar"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::symmetric(tokens.title_padding_y, tokens.title_padding_x),
                direction: Direction::Row,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let mut style = theme.text_style_label_sm();
    style.color = tokens.title_text;
    style.size = tokens.title_font_size;
    cx.child(
        titlebar,
        leaf(
            format!("{group_id}::title"),
            LeafKind::Text {
                content: title.to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}

fn mount_hint(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let mut style = theme.text_style_label_sm();
    style.color = theme.colors.text_muted;
    cx.child(
        parent,
        leaf(
            format!("{}::popup_hint", content.dropdown_id),
            LeafKind::Text {
                content: "Use mouse or Up/Down + Enter".to_string(),
                style,
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    Ok(())
}

fn mount_option_list(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    content: &DropdownOverlayContent,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let list_id = format!("{}::popup_list", content.dropdown_id);
    let scroll = cx.child(
        parent,
        container(
            format!("{list_id}::scroll"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(POPUP_HEIGHT),
                overflow: Overflow::Scroll,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    let items = cx.child(
        scroll,
        container(
            format!("{list_id}::items"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                gap: theme.components.list_view.item_gap,
                direction: Direction::Column,
                overflow: Overflow::Visible,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    for (index, option) in content.options.iter().enumerate() {
        mount_option(cx, items, content, index, option.as_ref(), theme)?;
    }
    Ok(())
}
