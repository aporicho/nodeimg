use super::node_factory::leaf;
use crate::renderer::ImageStyle;
use crate::template::{TemplateError, TemplateMountCx};
use crate::tree::layout::{BoxStyle, LeafKind, Size, TextureHandle};
use crate::tree::NodeId;

pub(super) fn mount_image(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    texture: TextureHandle,
    image_style: ImageStyle,
    min_height: f32,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Image {
                texture,
                style: image_style,
            },
            BoxStyle {
                width: Size::Fill,
                height: Size::Fill,
                min_height,
                ..BoxStyle::default()
            },
        ),
    )?;
    Ok(())
}
