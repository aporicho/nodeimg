use crate::control::{
    ControlHeight, ControlInteractionSpec, ControlKind, ControlLayoutPolicy, ControlMetrics,
    ControlSpec,
};
use crate::renderer::TextStyle;
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::layout::Align;
use crate::tree::NodeId;

pub(crate) type ControlMatchesFn = fn(&ControlSpec) -> bool;
pub(crate) type ControlMinHeightFn = fn(&ControlSpec, &Theme, ControlMetrics) -> f32;
pub(crate) type ControlLayoutPolicyFn =
    fn(&ControlSpec, &Theme, ControlMetrics) -> ControlLayoutPolicy;
pub(crate) type ControlMountFn = for<'a> fn(
    &mut TemplateMountCx<'a>,
    NodeId,
    &str,
    &ControlSpec,
    ControlInteractionSpec,
    &Theme,
    ControlMetrics,
) -> Result<(), TemplateError>;
pub(crate) type ControlInteractionSpecFn = fn(&ControlSpec) -> ControlInteractionSpec;

pub(crate) struct ControlKindDescriptor {
    pub(crate) kind: ControlKind,
    pub(crate) matches: ControlMatchesFn,
    pub(crate) min_height: ControlMinHeightFn,
    pub(crate) layout_policy: Option<ControlLayoutPolicyFn>,
    pub(crate) mount: ControlMountFn,
    pub(crate) interaction_spec: Option<ControlInteractionSpecFn>,
}

pub(crate) fn fixed_layout_policy(kind: ControlKind, min_height: f32) -> ControlLayoutPolicy {
    ControlLayoutPolicy {
        kind,
        height: ControlHeight::Fixed(min_height),
        row_align: Align::Center,
        wrapper_align: Align::Center,
        affects_parent_height: false,
    }
}

pub(crate) fn fill_layout_policy(kind: ControlKind, min_height: f32) -> ControlLayoutPolicy {
    ControlLayoutPolicy {
        kind,
        height: ControlHeight::Fill { min_height },
        row_align: Align::Stretch,
        wrapper_align: Align::Stretch,
        affects_parent_height: true,
    }
}

pub(crate) fn default_control_height(
    _control: &ControlSpec,
    _theme: &Theme,
    metrics: ControlMetrics,
) -> f32 {
    metrics.control_height
}

pub(crate) fn text_line_height(style: TextStyle) -> f32 {
    style.size * style.line_height
}

pub(crate) fn stacked_min_height(heights: impl Iterator<Item = f32>, gap: f32) -> f32 {
    let mut count = 0usize;
    let mut total = 0.0;
    for height in heights {
        count += 1;
        total += height;
    }
    if count > 1 {
        total += gap * (count as f32 - 1.0);
    }
    total
}

pub(crate) fn text_area_min_height(min_rows: usize, theme: &Theme, metrics: ControlMetrics) -> f32 {
    let tokens = theme.text_field_metrics(metrics.size, metrics.density);
    let line_height = tokens.value_size * 1.2;
    min_rows.max(1) as f32 * line_height + tokens.padding_y * 2.0
}
