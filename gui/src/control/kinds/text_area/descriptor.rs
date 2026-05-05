use super::mount::mount_text_area;
use crate::control::kinds::descriptor::{
    fill_layout_policy, text_area_min_height, ControlKindDescriptor,
};
use crate::control::{ControlKind, ControlLayoutPolicy, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::TextArea,
    matches: matches_control,
    min_height,
    layout_policy: Some(layout_policy),
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::TextArea { .. })
}

fn min_height(control: &ControlSpec, theme: &Theme, metrics: ControlMetrics) -> f32 {
    let ControlSpec::TextArea { min_rows, .. } = control else {
        unreachable!("text_area descriptor received non-text_area control");
    };
    text_area_min_height(*min_rows, theme, metrics)
}

fn layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    fill_layout_policy(ControlKind::TextArea, min_height(control, theme, metrics))
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
    let ControlSpec::TextArea { value, min_rows } = control else {
        unreachable!("text_area descriptor received non-text_area control");
    };
    mount_text_area(cx, parent, id, value, *min_rows, theme, metrics)
}
