use super::test_support::{container_with_rect, decor, hittable_style};
use super::{hit_test, HitChain};
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Overflow};
use crate::tree::Tree;

#[test]
fn hit_visible_overflow_child_outside_parent_bounds() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 120.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                overflow: Overflow::Visible,
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        );
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 125.0, 15.0);
    let ids: Vec<_> = chain.iter().collect();
    assert_eq!(ids, vec![child_id, root_id]);
}

#[test]
fn hit_hidden_overflow_clips_child_outside_parent_bounds() {
    let chain = hit_overflow_child_outside_parent_bounds(Overflow::Hidden);

    assert!(chain.is_empty());
}

#[test]
fn hit_scroll_overflow_clips_child_outside_parent_bounds() {
    let chain = hit_overflow_child_outside_parent_bounds(Overflow::Scroll);

    assert!(chain.is_empty());
}

#[test]
fn hit_visible_overflow_does_not_hit_parent_outside_its_bounds() {
    let mut tree = Tree::new();
    let root = tree.insert(container_with_rect(
        BoxStyle {
            overflow: Overflow::Visible,
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

    let chain = hit_test(&tree, root, 125.0, 15.0);
    assert!(chain.is_empty());
}

#[test]
fn hit_visible_overflow_keeps_reverse_z_order() {
    let mut tree = Tree::new();
    let child1_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 120.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    let child2_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 120.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                overflow: Overflow::Visible,
                ..Default::default()
            },
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

    let chain = hit_test(&tree, root_id, 125.0, 15.0);
    assert_eq!(chain.leaf(), Some(child2_id));
}

fn hit_overflow_child_outside_parent_bounds(overflow: Overflow) -> HitChain {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 120.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                overflow,
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        );
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    hit_test(&tree, root_id, 125.0, 15.0)
}
