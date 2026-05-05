use crate::control::mount::mount_control_text;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(crate) fn mount_read_only(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    value: &str,
    theme: &Theme,
) -> Result<(), TemplateError> {
    mount_control_text(cx, parent, id, value, theme)
}
