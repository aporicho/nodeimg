use super::TreeNodeBuilder;
use crate::renderer::Rect;
use crate::renderer::{Color, TextStyle};
use crate::tree::layout::{BoxStyle, LeafKind, RelayoutBoundaryReason, TextLayout};
use crate::tree::SemanticRole;
use crate::tree::{RectMoveInvalidation, RepaintBoundaryReason};

#[test]
fn container_applies_tree_node_defaults() {
    let node = TreeNodeBuilder::container("root", BoxStyle::default()).build();

    assert_eq!(node.id.as_str(), "root");
    assert!(node.props.enabled);
    assert!(node.decoration.is_none());
    assert_eq!(
        node.rect,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    );
    assert!(node.children.is_empty());
    assert!(node.local_runtime.visible);
    assert!(node.layout_meta.boundary.is_none());
    assert!(node.paint_meta.boundary.is_none());
    assert_eq!(node.mutation_meta.rect_move, RectMoveInvalidation::Layout);
    assert!(node.runtime_slots.is_empty());
}

#[test]
fn builder_sets_standard_metadata() {
    let node = TreeNodeBuilder::leaf(
        "label",
        LeafKind::Text {
            content: "Label".to_string(),
            style: TextStyle::new(Color::BLACK, 12.0),
            layout: TextLayout::default(),
        },
        BoxStyle::default(),
    )
    .semantic_role(SemanticRole::Button)
    .owner("owner")
    .layout_boundary(RelayoutBoundaryReason::Root)
    .paint_boundary(RepaintBoundaryReason::Root)
    .rect_move_invalidation(RectMoveInvalidation::Repaint)
    .build();

    assert_eq!(node.props.semantic_role, Some(SemanticRole::Button));
    assert_eq!(node.props.owner_id.as_deref(), Some("owner"));
    assert_eq!(
        node.layout_meta.boundary,
        Some(RelayoutBoundaryReason::Root)
    );
    assert_eq!(node.paint_meta.boundary, Some(RepaintBoundaryReason::Root));
    assert_eq!(node.mutation_meta.rect_move, RectMoveInvalidation::Repaint);
}
