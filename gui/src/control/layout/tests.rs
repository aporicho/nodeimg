use super::*;
use crate::control::{ControlKind, ControlNode, ControlSpec};
use crate::theme::light_theme;
use crate::tree::layout::Align;

#[test]
fn text_area_layout_policy_fills_and_stretches() {
    let theme = light_theme();
    let metrics = ControlMetrics::from_theme(&theme);
    let policy = control_layout_policy(&ControlSpec::text_area("line", 5), &theme, metrics);

    assert_eq!(policy.kind, ControlKind::TextArea);
    assert!(policy.fills_parent_height());
    assert_eq!(policy.row_align, Align::Stretch);
    assert_eq!(policy.wrapper_align, Align::Stretch);
    assert!(policy.affects_parent_height);
    assert!(policy.min_height() > metrics.control_height);
}

#[test]
fn fixed_controls_keep_centered_policy() {
    let theme = light_theme();
    let metrics = ControlMetrics::from_theme(&theme);
    let policy = control_layout_policy(&ControlSpec::slider(0.5, 0.0, 1.0, 0.01), &theme, metrics);

    assert_eq!(policy.kind, ControlKind::Slider);
    assert_eq!(policy.height, ControlHeight::Fixed(metrics.control_height));
    assert_eq!(policy.row_align, Align::Center);
    assert_eq!(policy.wrapper_align, Align::Center);
    assert!(!policy.affects_parent_height);
}

#[test]
fn control_list_min_height_counts_groups_and_gaps() {
    let theme = light_theme();
    let metrics = ControlMetrics::from_theme(&theme);
    let controls = [
        ControlNode::button("run", "Run"),
        ControlNode::group(
            "runtime",
            "Runtime",
            vec![ControlNode::label("status", "Idle")],
        ),
    ];

    assert!(control_list_min_height(controls.iter(), &theme, metrics, theme.spacing.sm) > 0.0);
}
