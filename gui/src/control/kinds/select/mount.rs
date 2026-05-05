use crate::control::mount::mount_control_text;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(crate) fn mount_select(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    options: &[String],
    selected: usize,
    theme: &Theme,
) -> Result<(), TemplateError> {
    let value = options.get(selected).cloned().unwrap_or_default();
    mount_control_text(cx, parent, id, &value, theme)
}
