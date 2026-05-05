use super::mount::mount_group;
use crate::control::kinds::descriptor::{
    fill_layout_policy, stacked_min_height, text_line_height, ControlKindDescriptor,
};
use crate::control::{ControlKind, ControlLayoutPolicy, ControlMetrics, ControlSpec};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

pub(in crate::control::kinds) const DESCRIPTOR: ControlKindDescriptor = ControlKindDescriptor {
    kind: ControlKind::Group,
    matches: matches_control,
    min_height,
    layout_policy: Some(layout_policy),
    mount,
    interaction_spec: None,
};

fn matches_control(control: &ControlSpec) -> bool {
    matches!(control, ControlSpec::Group { .. })
}

fn min_height(control: &ControlSpec, theme: &Theme, metrics: ControlMetrics) -> f32 {
    let ControlSpec::Group { children, .. } = control else {
        unreachable!("group descriptor received non-group control");
    };
    let title = text_line_height(theme.text_style_title_sm());
    let child_heights = std::iter::once(title).chain(children.iter().map(|child| {
        crate::control::kinds::registry::control_min_height(child.spec(), theme, metrics)
    }));
    theme.components.group.padding * 2.0
        + stacked_min_height(child_heights, theme.components.group.gap)
}

fn layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    fill_layout_policy(ControlKind::Group, min_height(control, theme, metrics))
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
    let ControlSpec::Group { title, children } = control else {
        unreachable!("group descriptor received non-group control");
    };
    mount_group(cx, parent, id, title, children, theme, metrics)
}
