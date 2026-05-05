use super::mount::mount_toggle;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlInteractionSpec, ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Toggle,
    matches: matches_control,
    min_height: default_control_height,
    layout_policy: None,
    mount,
    interaction_spec: Some(interaction_spec),
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Toggle { .. })
}

fn interaction_spec(control: &ControlSpec) -> ControlInteractionSpec {
    let ControlSpec::Toggle { checked } = control else {
        unreachable!("toggle descriptor received non-toggle control");
    };
    ControlInteractionSpec::toggle(*checked)
}

fn mount(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    interaction: crate::control::ControlInteractionSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let ControlSpec::Toggle { checked } = control else {
        unreachable!("toggle descriptor received non-toggle control");
    };
    mount_toggle(cx, parent, id, *checked, interaction, theme, metrics)
}
