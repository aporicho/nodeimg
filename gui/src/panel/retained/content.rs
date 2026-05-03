use super::controls::{mount_button, mount_label};
use super::data::{PanelContentTemplate, PanelFrameTemplateData};
use super::node_factory::{container, leaf};
use super::text::{ellipsis_text_layout, label_style};
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, Decoration, Edges, LeafKind, Size};
use crate::tree::NodeId;

pub(in crate::panel::retained) fn mount_panel_content(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<(), TemplateError> {
    match &data.content {
        PanelContentTemplate::Toolbar {
            add_graph_id,
            run_graph_id,
        } => {
            mount_button(cx, parent, add_graph_id, "Add Image Demo", &data.theme)?;
            mount_button(cx, parent, run_graph_id, "Run Image Demo", &data.theme)?;
        }
        PanelContentTemplate::Preview {
            image_id,
            texture,
            image_style,
        } => {
            cx.child(
                parent,
                leaf(
                    image_id.clone(),
                    LeafKind::Image {
                        texture: *texture,
                        style: *image_style,
                    },
                    BoxStyle {
                        width: Size::Fill,
                        height: Size::Fill,
                        min_height: 128.0,
                        ..BoxStyle::default()
                    },
                ),
            )?;
        }
        PanelContentTemplate::Engine {
            group_id,
            status,
            catalog,
            last_action,
        } => mount_engine_group(
            cx,
            parent,
            group_id,
            status,
            catalog,
            last_action,
            &data.theme,
        )?,
    }
    Ok(())
}

fn mount_engine_group(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    status: &str,
    catalog: &str,
    last_action: &str,
    theme: &Theme,
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
                content: "Runtime".to_string(),
                style: theme.text_style_title_sm(),
                layout: ellipsis_text_layout(),
            },
            label_style(),
        ),
    )?;
    mount_label(cx, group, "engine_status", status, theme, false)?;
    mount_label(cx, group, "engine_catalog", catalog, theme, true)?;
    mount_label(cx, group, "engine_last_action", last_action, theme, true)?;
    Ok(())
}
