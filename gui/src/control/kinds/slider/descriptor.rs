use super::mount::mount_slider;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlInteractionSpec, ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Slider,
    matches: matches_control,
    min_height: default_control_height,
    layout_policy: None,
    mount,
    interaction_spec: Some(interaction_spec),
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Slider { .. })
}

fn interaction_spec(control: &ControlSpec) -> ControlInteractionSpec {
    let ControlSpec::Slider {
        value,
        min,
        max,
        step,
    } = control
    else {
        unreachable!("slider descriptor received non-slider control");
    };
    ControlInteractionSpec::slider(*value, *min, *max, *step)
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
    let ControlSpec::Slider {
        value, min, max, ..
    } = control
    else {
        unreachable!("slider descriptor received non-slider control");
    };
    mount_slider(
        cx,
        parent,
        id,
        *value,
        *min,
        *max,
        interaction,
        theme,
        metrics,
    )
}
