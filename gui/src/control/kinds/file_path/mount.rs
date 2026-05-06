use crate::control::mount::mount_text_field;
use crate::control::ControlMetrics;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(crate) fn mount_file_path(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    path: &str,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    mount_text_field(
        cx,
        parent,
        id,
        path,
        false,
        1,
        SemanticRole::TextInput,
        theme,
        metrics,
    )
}
