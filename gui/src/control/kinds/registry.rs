use super::{
    button, color,
    descriptor::{fixed_layout_policy, ControlKindDescriptor},
    file_path, group, image, label, number, read_only, select, slider, text, text_area, toggle,
};
use crate::control::{
    ControlInteractionSpec, ControlKind, ControlLayoutPolicy, ControlMetrics, ControlNode,
    ControlSpec,
};
use crate::template::{TemplateError, TemplateMountCx};
use crate::theme::Theme;
use crate::tree::NodeId;

const CONTROL_DESCRIPTORS: &[&ControlKindDescriptor] = &[
    &button::DESCRIPTOR,
    &label::DESCRIPTOR,
    &image::DESCRIPTOR,
    &group::DESCRIPTOR,
    &read_only::DESCRIPTOR,
    &text::DESCRIPTOR,
    &text_area::DESCRIPTOR,
    &number::DESCRIPTOR,
    &slider::DESCRIPTOR,
    &toggle::DESCRIPTOR,
    &select::DESCRIPTOR,
    &color::DESCRIPTOR,
    &file_path::DESCRIPTOR,
];

pub(crate) fn control_kind(control: &ControlSpec) -> ControlKind {
    descriptor_for(control).kind
}

pub(crate) fn control_layout_policy(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> ControlLayoutPolicy {
    let descriptor = descriptor_for(control);
    descriptor
        .layout_policy
        .map(|layout_policy| layout_policy(control, theme, metrics))
        .unwrap_or_else(|| {
            fixed_layout_policy(
                descriptor.kind,
                (descriptor.min_height)(control, theme, metrics),
            )
        })
}

pub(crate) fn control_min_height(
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> f32 {
    let descriptor = descriptor_for(control);
    (descriptor.min_height)(control, theme, metrics)
}

pub(crate) fn control_list_min_height<'a>(
    controls: impl Iterator<Item = &'a ControlNode>,
    theme: &Theme,
    metrics: ControlMetrics,
    gap: f32,
) -> f32 {
    super::descriptor::stacked_min_height(
        controls.map(|control| control_min_height(control.spec(), theme, metrics)),
        gap,
    )
}

pub(crate) fn mount_control_content(
    cx: &mut TemplateMountCx<'_>,
    parent: NodeId,
    id: &str,
    control: &ControlSpec,
    theme: &Theme,
    metrics: ControlMetrics,
) -> Result<(), TemplateError> {
    let descriptor = descriptor_for(control);
    let interaction_spec = control_interaction_spec(control);
    (descriptor.mount)(cx, parent, id, control, interaction_spec, theme, metrics)
}

pub(crate) fn control_interaction_spec(control: &ControlSpec) -> ControlInteractionSpec {
    descriptor_for(control)
        .interaction_spec
        .map(|interaction_spec| interaction_spec(control))
        .unwrap_or(ControlInteractionSpec::None)
}

fn descriptor_for(control: &ControlSpec) -> &'static ControlKindDescriptor {
    CONTROL_DESCRIPTORS
        .iter()
        .copied()
        .find(|descriptor| (descriptor.matches)(control))
        .expect("every ControlSpec variant must have a ControlKindDescriptor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn registry_identifies_every_control_kind() {
        let controls = [
            (ControlSpec::button("Run"), ControlKind::Button),
            (ControlSpec::label("Label"), ControlKind::Label),
            (ControlSpec::read_only("Value"), ControlKind::ReadOnly),
            (ControlSpec::text("Text"), ControlKind::Text),
            (ControlSpec::text_area("Text", 2), ControlKind::TextArea),
            (
                ControlSpec::number(1.0, 0.0, 2.0, 1.0, 0),
                ControlKind::Number,
            ),
            (ControlSpec::slider(1.0, 0.0, 2.0, 1.0), ControlKind::Slider),
            (ControlSpec::toggle(true), ControlKind::Toggle),
            (
                ControlSpec::select(vec!["A".to_string()], 0),
                ControlKind::Select,
            ),
            (ControlSpec::color([1.0, 0.0, 0.0, 1.0]), ControlKind::Color),
            (
                ControlSpec::file_path("/tmp/a.png", vec!["png".to_string()]),
                ControlKind::FilePath,
            ),
            (
                ControlSpec::image(
                    crate::tree::layout::TextureHandle(1),
                    crate::renderer::ImageStyle::default(),
                ),
                ControlKind::Image,
            ),
            (ControlSpec::group("Group", Vec::new()), ControlKind::Group),
        ];

        for (spec, kind) in controls {
            assert_eq!(control_kind(&spec), kind);
            assert_eq!(descriptor_for(&spec).kind, kind);
        }
    }

    #[test]
    fn registry_exposes_interaction_specs_for_interactive_controls() {
        assert!(
            control_interaction_spec(&ControlSpec::slider(1.0, 0.0, 2.0, 1.0)).is_interactive()
        );
        assert!(control_interaction_spec(&ControlSpec::toggle(true)).is_interactive());
        assert!(
            control_interaction_spec(&ControlSpec::select(vec!["A".to_string()], 0))
                .is_interactive()
        );
        assert!(!control_interaction_spec(&ControlSpec::label("Label")).is_interactive());
    }

    #[test]
    fn registry_preserves_group_min_height() {
        let theme = light_theme();
        let metrics = ControlMetrics::from_theme(&theme);
        let spec = ControlSpec::group("Group", vec![ControlNode::label("status", "Ready")]);

        assert!(control_min_height(&spec, &theme, metrics) > metrics.control_height);
    }
}
