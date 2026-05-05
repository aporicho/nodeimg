use super::data::PanelFrameTemplateData;
use super::node_factory::{container, leaf};
use crate::template::{TemplateError, TemplateMountCx};
use crate::tree::layout::Gesture;
use crate::tree::layout::{
    Align, BoxStyle, Decoration, Direction, Edges, LeafKind, Size, TextLayout, TextOverflow,
};
use crate::tree::NodeId;

pub(in crate::panel::retained) fn mount_titlebar(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<(), TemplateError> {
    let id = data.config.id.as_str();
    let panel = data.theme.components.panel;
    let visual = data.theme.panel_visual();
    let titlebar = cx.child(
        parent,
        container(
            format!("{id}::titlebar"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fixed(panel.title_bar_height),
                padding: Edges::symmetric(panel.title_padding_y, panel.title_padding_x),
                direction: Direction::Row,
                align_items: Align::Center,
                hittable: true,
                draggable: data.config.draggable,
                gestures: vec![Gesture::Tap, Gesture::Drag],
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.titlebar_background),
                border: None,
                radius: [panel.radius, panel.radius, 0.0, 0.0],
                shadow: None,
            }),
        )
        .owner(id.to_string()),
    )?;
    cx.child(
        titlebar,
        leaf(
            format!("{id}::title"),
            LeafKind::Text {
                content: data.config.title.to_string(),
                style: crate::renderer::TextStyle::new(visual.title_text, panel.title_font_size)
                    .with_family(data.theme.text.body_family)
                    .with_line_height(data.theme.text.default_line_height),
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

fn ellipsis_text_layout() -> TextLayout {
    TextLayout {
        overflow: TextOverflow::Ellipsis,
        ..TextLayout::default()
    }
}
