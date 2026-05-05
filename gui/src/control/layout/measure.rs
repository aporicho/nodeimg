use super::metrics::ControlMetrics;
use super::policy::ControlLayoutPolicy;
use crate::control::{ControlKind, ControlNode, ControlSpec};
use crate::theme::Theme;

pub fn control_layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    crate::control::kinds::registry::control_layout_policy(control, theme, metrics)
}

pub fn control_min_height(control: &ControlSpec, theme: &Theme, metrics: ControlMetrics) -> f32 {
    crate::control::kinds::registry::control_min_height(control, theme, metrics)
}

pub fn control_node_min_height(node: &ControlNode, theme: &Theme, metrics: ControlMetrics) -> f32 {
    control_min_height(node.spec(), theme, metrics)
}

pub fn control_list_min_height<'a>(
    controls: impl Iterator<Item = &'a ControlNode>,
    theme: &Theme,
    metrics: ControlMetrics,
    gap: f32,
) -> f32 {
    crate::control::kinds::registry::control_list_min_height(controls, theme, metrics, gap)
}

pub fn control_kind(control: &ControlSpec) -> ControlKind {
    crate::control::kinds::registry::control_kind(control)
}
