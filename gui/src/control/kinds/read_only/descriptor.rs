use super::mount::mount_read_only;
use crate::control::kinds::descriptor::{text_line_height, ControlKindDescriptor};
use crate::control::{ControlKind, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::ReadOnly,
    matches: matches_control,
    min_height,
    layout_policy: None,
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::ReadOnly { .. })
}

fn min_height(_control: &ControlSpec, theme: &Theme, _metrics: ControlMetrics) -> f32 {
    text_line_height(theme.text_style_label_sm())
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
    let ControlSpec::ReadOnly { value } = control else {
        unreachable!("read_only descriptor received non-read_only control");
    };
    mount_read_only(cx, parent, id, value, theme)
}
