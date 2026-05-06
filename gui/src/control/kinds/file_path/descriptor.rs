use super::mount::mount_file_path;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::FilePath,
    matches: matches_control,
    min_height: default_control_height,
    layout_policy: None,
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::FilePath { .. })
}

fn mount(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    _interaction: crate::control::ControlInteractionSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let ControlSpec::FilePath { path, .. } = control else {
        unreachable!("file_path descriptor received non-file_path control");
    };
    mount_file_path(cx, parent, id, path, theme, metrics)
}
