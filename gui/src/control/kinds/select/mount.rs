use crate::control::mount::{ellipsis_text_layout, label_style, leaf};
use crate::control::ControlInteractionSpec;
use crate::renderer::TextStyle;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::{Gesture, LeafKind};
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(crate) fn mount_select(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    options: &[String],
    selected: usize,
    interaction: ControlInteractionSpec,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let value = options.get(selected).cloned().unwrap_or_default();
    cx.child(
        parent,
        leaf(
            id.to_string(),
            LeafKind::Text {
                content: value,
                style: TextStyle {
                    color: theme.colors.text,
                    ..theme.text_style_label_sm()
                },
                layout: ellipsis_text_layout(),
            },
            crate::tree::layout::BoxStyle {
                hittable: true,
                gestures: vec![Gesture::Tap],
                ..label_style()
            },
        )
        .semantic_role(SemanticRole::Dropdown)
        .runtime_slot(interaction),
    )?;
    Ok(())
}
