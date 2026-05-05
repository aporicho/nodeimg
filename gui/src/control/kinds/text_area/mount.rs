use crate::control::mount::mount_text_field;
use crate::control::ControlMetrics;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;
use crate::tree::SemanticRole;

pub(crate) fn mount_text_area(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    value: &str,
    min_rows: usize,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    mount_text_field(
        cx,
        parent,
        id,
        value,
        true,
        min_rows,
        SemanticRole::TextArea,
        theme,
        metrics,
    )
}
