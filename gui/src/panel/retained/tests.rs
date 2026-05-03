use std::borrow::Cow;

use super::data::{PanelContentTemplate, PanelFrameTemplateData};
use super::node_factory::{container, zero_rect};
use crate::control::{ControlNode, ControlRole};
use crate::gesture::Gesture;
use crate::panel::{PanelConfig, PanelId, PanelRuntime};
use crate::renderer::Rect;
use crate::template::{
    InstanceId, TemplateId, TemplatePayload, TemplateRegistry, PANEL_FRAME_TEMPLATE,
};
use crate::theme::light_theme;
use crate::tree::layout::{BoxStyle, Position, RelayoutBoundaryReason, Size};
use crate::tree::{RectMoveInvalidation, RepaintBoundaryReason, Tree};

#[test]
fn panel_frame_template_mounts_root_titlebar_content_and_toolbar_actions() {
    let registry = TemplateRegistry::with_builtin_templates();
    let mut tree = Tree::new();
    let parent = tree.insert(container(
        "__panel_root".to_string(),
        BoxStyle::default(),
        None,
    ));
    tree.set_root(parent);
    let theme = light_theme();

    registry
        .instantiate_payload(
            &mut tree,
            TemplateId::from(PANEL_FRAME_TEMPLATE),
            InstanceId::from("panel::toolbar"),
            parent,
            TemplatePayload::PanelFrame(PanelFrameTemplateData::new(
                PanelConfig {
                    id: PanelId::new("toolbar"),
                    title: Cow::Borrowed("Toolbar"),
                    default_rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 100.0,
                        h: 80.0,
                    },
                    min_size: [100.0, 80.0],
                    titlebar_visible: true,
                    draggable: true,
                    resizable: true,
                    closable: false,
                    initially_visible: true,
                },
                PanelRuntime {
                    rect: Rect {
                        x: 12.0,
                        y: 34.0,
                        w: 240.0,
                        h: 180.0,
                    },
                    min_size: [120.0, 90.0],
                    visible: true,
                    z_index: 42,
                    collapsed: false,
                },
                PanelContentTemplate::new(vec![
                    ControlNode::button("toolbar::add", "Add Image Demo"),
                    ControlNode::button("toolbar::run", "Run Image Demo"),
                ]),
                &theme,
            )),
        )
        .expect("mount panel frame");

    let root = tree.node_by_str("toolbar").expect("panel root");
    let root_node = tree.get(root).expect("panel root node");
    assert_eq!(root_node.rect, zero_rect());
    assert_eq!(root_node.style.position, Position::absolute_xy(12.0, 34.0));
    assert_eq!(root_node.style.width, Size::Fixed(240.0));
    assert_eq!(root_node.style.height, Size::Fixed(180.0));
    assert_eq!(root_node.style.min_width, 120.0);
    assert_eq!(root_node.style.min_height, 90.0);
    assert_eq!(root_node.style.z_index, 42);
    assert!(root_node.style.hittable);
    assert!(root_node.style.resizable);
    assert_eq!(root_node.props.semantic_role, Some(ControlRole::Panel));
    assert_eq!(
        root_node.layout_meta.boundary,
        Some(RelayoutBoundaryReason::Panel)
    );
    assert_eq!(
        root_node.paint_meta.boundary,
        Some(RepaintBoundaryReason::PanelFrame)
    );
    assert_eq!(
        root_node.mutation_meta.rect_move,
        RectMoveInvalidation::LayoutAndBoundaryPlacement
    );

    let titlebar = tree
        .node_by_str("toolbar::titlebar")
        .expect("panel titlebar");
    let titlebar_node = tree.get(titlebar).expect("titlebar node");
    assert_eq!(titlebar_node.props.owner_id.as_deref(), Some("toolbar"));
    assert!(titlebar_node.style.hittable);
    assert!(titlebar_node.style.draggable);
    assert!(titlebar_node.style.gestures.contains(&Gesture::Tap));
    assert!(titlebar_node.style.gestures.contains(&Gesture::Drag));

    tree.node_by_str("toolbar::content")
        .expect("panel content root");
    tree.node_by_str("toolbar::add")
        .expect("toolbar add button");
    tree.node_by_str("toolbar::run")
        .expect("toolbar run button");
}
