use std::borrow::Cow;

use super::data::DropdownOverlayTemplateData;
use super::node_factory::{container, zero_rect};
use crate::icon::names;
use crate::overlay::{DropdownOverlayContent, OverlayContent};
use crate::template::{
    InstanceId, TemplateId, TemplatePayload, TemplateRegistry, DROPDOWN_OVERLAY_TEMPLATE,
};
use crate::theme::{light_theme, ControlSize, Density};
use crate::tree::layout::{BoxStyle, LeafKind, Position, Size};
use crate::tree::{NodeKind, RepaintBoundaryReason, Tree};

#[test]
fn dropdown_overlay_template_mounts_options_and_marker_icons() {
    let registry = TemplateRegistry::with_builtin_templates();
    let mut tree = Tree::new();
    let parent = tree.insert(container(
        "__overlay_root".to_string(),
        BoxStyle::default(),
        None,
    ));
    tree.set_root(parent);
    let theme = light_theme();

    registry
        .instantiate_payload(
            &mut tree,
            TemplateId::from(DROPDOWN_OVERLAY_TEMPLATE),
            InstanceId::from("dropdown::blend"),
            parent,
            TemplatePayload::DropdownOverlay(DropdownOverlayTemplateData::new(
                "dropdown::blend".to_string(),
                12.0,
                34.0,
                Some(180.0),
                OverlayContent::DropdownOptions(DropdownOverlayContent {
                    dropdown_id: "blend".to_string(),
                    title: Cow::Borrowed("Mode"),
                    options: vec![Cow::Borrowed("Normal"), Cow::Borrowed("Multiply")],
                    selected: 0,
                    highlighted: 1,
                    size: ControlSize::Small,
                    density: Density::Compact,
                }),
                &theme,
            )),
        )
        .expect("mount dropdown overlay");

    let root = tree
        .node_by_str("__overlay::dropdown::blend")
        .expect("overlay root");
    let root_node = tree.get(root).expect("root node");
    assert_eq!(root_node.rect, zero_rect());
    assert_eq!(root_node.style.position, Position::absolute_xy(12.0, 34.0));
    assert_eq!(root_node.style.width, Size::Fixed(180.0));
    assert!(root_node.style.hittable);
    assert_eq!(root_node.style.z_index, 20_000);
    assert_eq!(
        root_node.paint_meta.boundary,
        Some(RepaintBoundaryReason::Explicit)
    );

    let first = tree
        .node_by_str("__dropdown_option::blend::0::marker_icon")
        .expect("selected marker icon");
    let second = tree
        .node_by_str("__dropdown_option::blend::1::marker_icon")
        .expect("highlighted marker icon");
    assert!(matches!(
        &tree.get(first).expect("first marker").kind,
        NodeKind::Leaf(LeafKind::Icon { spec }) if spec.id.as_str() == names::CHECK.as_str()
    ));
    assert!(matches!(
        &tree.get(second).expect("second marker").kind,
        NodeKind::Leaf(LeafKind::Icon { spec }) if spec.id.as_str() == names::NAV_ARROW_RIGHT.as_str()
    ));
}
