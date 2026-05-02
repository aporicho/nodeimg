use super::order::HitOrderCache;
use crate::renderer::Rect;
use crate::tree::layout::BoxStyle;
use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
use crate::tree::{NodeProps, RuntimeSlots, StableId, Tree};

fn container(id: &'static str, z_index: i32) -> TreeNode {
    TreeNode {
        id: StableId::from(id),
        props: NodeProps::default(),
        style: BoxStyle {
            z_index,
            ..Default::default()
        },
        decoration: None,
        kind: NodeKind::Container,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        children: Vec::new(),
        local_runtime: NodeLocalRuntime::default(),
        layout_meta: Default::default(),
        paint_meta: Default::default(),
        mutation_meta: Default::default(),
        runtime_slots: RuntimeSlots::default(),
    }
}

#[test]
fn hit_order_cache_reuses_order_until_child_revision_changes() {
    let mut tree = Tree::new();
    let first = tree.insert(container("first", 1));
    let second = tree.insert(container("second", 2));
    let parent = tree.insert(container("parent", 0));
    tree.set_children(parent, vec![first, second]);
    let mut cache = HitOrderCache::default();

    let (first_order, first_hit) = cache.children_in_hit_order(&tree, parent, &[first, second]);
    let (second_order, second_hit) = cache.children_in_hit_order(&tree, parent, &[first, second]);

    assert!(!first_hit);
    assert!(second_hit);
    assert_eq!(first_order, vec![second, first]);
    assert_eq!(second_order, first_order);
}
