use super::node_factory::leaf;
use super::text::{ellipsis_text_layout, label_style};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::LeafKind;
use crate::tree::NodeId;

pub(super) fn mount_label(
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
