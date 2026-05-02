use super::super::node_factory::leaf;
use super::super::style::ellipsis_text_layout;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{BoxStyle, LeafKind, Size};
use crate::tree::NodeId;

pub(super) fn mount_control_text(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    text: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Text {
                content: text.to_string(),
                style: theme.text_style_label_sm(),
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
