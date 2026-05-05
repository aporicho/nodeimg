use super::content::mount_panel_content;
use super::data::PanelFrameTemplateData;
use super::node_factory::container;
use super::titlebar::mount_titlebar;
use crate::renderer::Border;
use crate::template::{TemplateError, TemplateMountCx};
use crate::tree::layout::{
    BoxStyle, Decoration, Edges, Overflow, Position, RelayoutBoundaryReason, Size,
};
use crate::tree::SemanticRole;
use crate::tree::{NodeId, RectMoveInvalidation, RepaintBoundaryReason};

pub(in crate::panel::retained) fn mount_panel(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &PanelFrameTemplateData,
) -> Result<NodeId, TemplateError> {
    let id = data.config.id.as_str();
    let theme = &data.theme;
    let panel = theme.components.panel;
    let visual = theme.panel_visual();
    let root = cx.child(
        parent,
        container(
            id.to_string(),
            BoxStyle {
                position: Position::absolute_xy(data.runtime.rect.x, data.runtime.rect.y),
                width: Size::Fixed(data.runtime.rect.w),
                height: Size::Fixed(data.runtime.rect.h),
                min_width: data.runtime.min_size[0],
                min_height: data.runtime.min_size[1],
                z_index: data.runtime.z_index,
                overflow: Overflow::Hidden,
                hittable: true,
                resizable: data.config.resizable,
                ..BoxStyle::default()
            },
            Some(Decoration {
                background: Some(visual.frame_background),
                border: Some(Border {
                    width: panel.border_width,
                    color: visual.frame_border,
                }),
                radius: [panel.radius; 4],
                shadow: None,
            }),
        )
        .semantic_role(SemanticRole::Panel)
        .layout_boundary(RelayoutBoundaryReason::Panel)
        .paint_boundary(RepaintBoundaryReason::PanelFrame)
        .rect_move_invalidation(RectMoveInvalidation::LayoutAndBoundaryPlacement),
    )?;

    if data.config.titlebar_visible {
        mount_titlebar(cx, root, data)?;
    }
    let content = cx.child(
        root,
        container(
            format!("{id}::content"),
            BoxStyle {
                width: Size::Fill,
                height: Size::Fill,
                padding: Edges::all(panel.content_padding),
                gap: theme.spacing.sm,
                flex_grow: 1.0,
                overflow: Overflow::Hidden,
                ..BoxStyle::default()
            },
            None,
        ),
    )?;
    mount_panel_content(cx, content, data)?;
    Ok(root)
}
