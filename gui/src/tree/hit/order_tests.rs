use super::order::HitOrderCache;
use crate::tree::layout::BoxStyle;
use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

fn container(id: &'static str, z_index: i32) -> TreeNode {
    TreeNodeBuilder::container(
        id,
        BoxStyle {
            z_index,
            ..Default::default()
        },
    )
    .build()
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
