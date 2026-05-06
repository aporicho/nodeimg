use super::mount::mount_select;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlInteractionSpec, ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Select,
    matches: matches_control,
    min_height: default_control_height,
    layout_policy: None,
    mount,
    interaction_spec: Some(interaction_spec),
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Select { .. })
}

fn interaction_spec(control: &ControlSpec) -> ControlInteractionSpec {
    let ControlSpec::Select { options, selected } = control else {
        unreachable!("select descriptor received non-select control");
    };
    ControlInteractionSpec::select(*selected, options.len())
}

fn mount(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    interaction: crate::control::ControlInteractionSpec,
    theme: &Theme,
    _metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let ControlSpec::Select { options, selected } = control else {
        unreachable!("select descriptor received non-select control");
    };
    mount_select(cx, parent, id, options, *selected, interaction, theme)
}
