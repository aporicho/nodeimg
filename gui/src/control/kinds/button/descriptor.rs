use super::mount::mount_button;
use crate::control::kinds::descriptor::{default_control_height, ControlKindDescriptor};
use crate::control::{ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Button,
    matches: matches_control,
    min_height,
    layout_policy: None,
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Button { .. })
}

fn min_height(control: &ControlSpec, theme: &Theme, metrics: ControlMetrics) -> f32 {
    let ControlSpec::Button { .. } = control else {
        return default_control_height(control, theme, metrics);
    };
    theme.components.button.font_size + theme.components.button.padding_y * 2.0
}

fn mount(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    _interaction: crate::control::ControlInteractionSpec,
    theme: &Theme,
    _metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let ControlSpec::Button { label } = control else {
        unreachable!("button descriptor received non-button control");
    };
    mount_button(cx, parent, id, label, theme)
}
