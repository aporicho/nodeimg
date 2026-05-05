use super::mount::mount_text;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Text,
    matches: matches_control,
    min_height: default_control_height,
    layout_policy: None,
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Text { .. })
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
    let ControlSpec::Text { value } = control else {
        unreachable!("text descriptor received non-text control");
    };
    mount_text(cx, parent, id, value, theme, metrics)
}
