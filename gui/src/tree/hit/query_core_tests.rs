use super::hit_test;
use super::test_support::{container_with_rect, decor, hittable_style};
use crate::renderer::Rect;
use crate::tree::layout::BoxStyle;
use crate::tree::Tree;

#[test]
fn hit_empty_tree() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle::default(),
        None,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert!(chain.is_empty(), "未声明交互能力的节点应该不可命中");
}

#[test]
fn hit_root_only() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain.leaf(), Some(root));
    assert_eq!(chain.root(), Some(root));
}

#[test]
fn hit_leaf_nested() {
    let mut tree = Tree::new();
    let leaf_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 20.0,
            y: 20.0,
            w: 30.0,
            h: 30.0,
        },
    ));
    let middle = {
        let mut n = container_with_rect(
            hittable_style(),
            Some(decor()),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 80.0,
                h: 80.0,
            },
        );
        n.children = vec![leaf_id];
        n
    };
    let middle_id = tree.insert(middle);
    let root = {
        let mut n = container_with_rect(
            hittable_style(),
            Some(decor()),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        );
        n.children = vec![middle_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 30.0, 30.0);
    assert_eq!(chain.len(), 3);
    let ids: Vec<_> = chain.iter().collect();
    assert_eq!(ids, vec![leaf_id, middle_id, root_id]);
}

#[test]
fn hit_reverse_z_order() {
    let mut tree = Tree::new();
    let child1_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 50.0,
        },
    ));
    let child2_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 50.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        );
        n.children = vec![child1_id, child2_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 30.0, 30.0);
    assert_eq!(chain.leaf(), Some(child2_id), "后添加的 child2 应该先命中");
}

#[test]
fn hit_prefers_higher_z_index_over_later_source_order() {
    let mut tree = Tree::new();
    let lower_id = tree.insert(container_with_rect(
        BoxStyle {
            hittable: true,
            z_index: 0,
            ..Default::default()
        },
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 50.0,
        },
    ));
    let higher_id = tree.insert(container_with_rect(
        BoxStyle {
            hittable: true,
            z_index: 10,
            ..Default::default()
        },
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 50.0,
            h: 50.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        );
        n.children = vec![higher_id, lower_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 30.0, 30.0);

    assert_eq!(chain.leaf(), Some(higher_id));
}

#[test]
fn hit_hittable_explicit_false() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle {
            hittable: false,
            ..Default::default()
        },
        Some(decor()),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert!(chain.is_empty(), "hittable=false 应不可命中");
}

#[test]
fn hit_hittable_explicit_true() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        None,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert_eq!(chain.len(), 1);
    assert_eq!(chain.leaf(), Some(root));
}

#[test]
fn hit_gestures_makes_hittable() {
    use crate::gesture::Gesture;
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle {
            gestures: vec![Gesture::Tap],
            ..Default::default()
        },
        None,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert_eq!(chain.leaf(), Some(root), "有 gestures 应该可命中");
}

#[test]
fn hit_interaction_capabilities_make_hittable() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle {
            draggable: true,
            resizable: true,
            ..Default::default()
        },
        None,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert_eq!(chain.leaf(), Some(root), "交互能力应该可命中");
}

#[test]
fn hit_neither_gestures_nor_decoration() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle::default(),
        None,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 50.0, 50.0);
    assert!(chain.is_empty());
}

#[test]
fn hit_outside_bounds() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
    ));
    tree.set_root(root);

    let chain = hit_test(&tree, root, 200.0, 200.0);
    assert!(chain.is_empty());
}
