use super::data::DropdownOverlayTemplateData;
use super::dropdown::mount_dropdown_group;
use super::node_factory::container;
use crate::overlay::OverlayContent;
use crate::template::{TemplateError, TemplateMountCx};
use crate::tree::layout::{BoxStyle, Overflow, Position, Size};
use crate::tree::{NodeId, RepaintBoundaryReason};

pub(in crate::overlay::retained) fn mount_dropdown_overlay(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    data: &DropdownOverlayTemplateData,
) -> Result<NodeId, TemplateError> {
    let OverlayContent::DropdownOptions(content) = &data.content;
    let root = cx.child(
        parent,
        container(
            format!("__overlay::{}", data.overlay_id),
            BoxStyle {
                position: Position::absolute_xy(data.x, data.y),
                width: data.width.map(Size::Fixed).unwrap_or(Size::Auto),
                height: Size::Auto,
                overflow: Overflow::Visible,
                hittable: true,
                z_index: 20_000,
                ..BoxStyle::default()
            },
            None,
        )
        .paint_boundary(RepaintBoundaryReason::Explicit),
    )?;
    mount_dropdown_group(cx, root, content, &data.theme)?;
    Ok(root)
}
