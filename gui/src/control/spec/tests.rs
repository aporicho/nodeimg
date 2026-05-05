use super::{ControlNode, ControlSpec};
use crate::renderer::{ImageFit, ImageStyle};
use crate::tree::layout::TextureHandle;

#[test]
fn control_spec_constructors_cover_variants() {
    let image_style = ImageStyle {
        fit: ImageFit::Contain,
        ..ImageStyle::default()
    };

    assert!(matches!(
        ControlSpec::button("Run"),
        ControlSpec::Button { .. }
    ));
    assert!(matches!(
        ControlSpec::label("Status"),
        ControlSpec::Label { muted: false, .. }
    ));
    assert!(matches!(
        ControlSpec::muted_label("Status"),
        ControlSpec::Label { muted: true, .. }
    ));
    assert!(matches!(
        ControlSpec::image(TextureHandle(1), image_style),
        ControlSpec::Image {
            min_height: 128.0,
            ..
        }
    ));
    assert!(matches!(
        ControlSpec::group("Group", Vec::new()),
        ControlSpec::Group { .. }
    ));
    assert!(matches!(
        ControlSpec::read_only("Value"),
        ControlSpec::ReadOnly { .. }
    ));
    assert!(matches!(
        ControlSpec::text("Value"),
        ControlSpec::Text { .. }
    ));
    assert!(matches!(
        ControlSpec::text_area("Value", 3),
        ControlSpec::TextArea { min_rows: 3, .. }
    ));
    assert!(matches!(
        ControlSpec::number(1.0, 0.0, 10.0, 1.0, 2),
        ControlSpec::Number { precision: 2, .. }
    ));
    assert!(matches!(
        ControlSpec::slider(1.0, 0.0, 10.0, 1.0),
        ControlSpec::Slider { .. }
    ));
    assert!(matches!(
        ControlSpec::toggle(true),
        ControlSpec::Toggle { checked: true }
    ));
    assert!(matches!(
        ControlSpec::select(vec!["A".to_string()], 0),
        ControlSpec::Select { .. }
    ));
    assert!(matches!(
        ControlSpec::color([1.0, 0.0, 0.0, 1.0]),
        ControlSpec::Color { .. }
    ));
    assert!(matches!(
        ControlSpec::file_path("/tmp/a.png", vec!["png".to_string()]),
        ControlSpec::FilePath { .. }
    ));
}

#[test]
fn control_node_constructors_preserve_id_and_spec() {
    let node = ControlNode::slider("opacity", 0.5, 0.0, 1.0, 0.01);

    assert_eq!(node.id(), "opacity");
    assert!(matches!(node.spec(), ControlSpec::Slider { .. }));
}
