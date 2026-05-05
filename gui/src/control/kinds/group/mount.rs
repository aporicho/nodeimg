use crate::control::mount::{
    container, ellipsis_text_layout, label_style, leaf, mount_control_node,
};
use crate::control::{ControlMetrics, ControlNode};
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Edges, LeafKind, Size};
use crate::tree::NodeId;

pub(crate) fn mount_group(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    title: &str,
    children: &[ControlNode],
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let group = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                width: Size::Fill,
                height: Size::Auto,
                padding: Edges::all(theme.components.group.padding),
                gap: theme.components.group.gap,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(theme.colors.surface),
                border: Some(Border {
                    width: theme.components.group.border_width,
                    color: theme.colors.border,
                }),
                radius: [theme.components.group.radius; 4],
                shadow: None,
            }),
        ),
    )?;
    cx.child(
        group,
        leaf(
            format!("{id}::title"),
            LeafKind::Text {
                content: title.to_string(),
                style: theme.text_style_title_sm(),
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    for child in children {
        mount_control_node(cx, group, child, theme, metrics)?;
    }
    Ok(())
}
