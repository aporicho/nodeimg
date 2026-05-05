use super::mount::mount_image;
use crate::control::kinds::descriptor::{fill_layout_policy, ControlKindDescriptor};
use crate::control::{ControlKind, ControlLayoutPolicy, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Image,
    matches: matches_control,
    min_height,
    layout_policy: Some(layout_policy),
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Image { .. })
}

fn min_height(control: &ControlSpec, _theme: &Theme, _metrics: ControlMetrics) -> f32 {
    let ControlSpec::Image { min_height, .. } = control else {
        unreachable!("image descriptor received non-image control");
    };
    *min_height
}

fn layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    fill_layout_policy(ControlKind::Image, min_height(control, theme, metrics))
}

fn mount(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    _interaction: crate::control::ControlInteractionSpec,
    _theme: &Theme,
    _metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let ControlSpec::Image {
        texture,
        image_style,
        min_height,
    } = control
    else {
        unreachable!("image descriptor received non-image control");
    };
    mount_image(cx, parent, id, *texture, *image_style, *min_height)
}
