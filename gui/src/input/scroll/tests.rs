use super::*;
use crate::input::{PointerHitRequest, PointerHitResolver};
use crate::renderer::Rect;
use crate::shell::AppEvent;
use crate::tree::layout::{BoxStyle, Overflow};
use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

fn node(id: &'static str, rect: Rect, style: BoxStyle) -> TreeNode {
    TreeNodeBuilder::container(id, style).rect(rect).build()
}

fn hittable_node(id: &'static str, rect: Rect) -> TreeNode {
    node(
        id,
        rect,
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
    )
}

fn scroll_node(id: &'static str, rect: Rect) -> TreeNode {
    node(
        id,
        rect,
        BoxStyle {
            hittable: true,
            overflow: Overflow::Scroll,
            ..Default::default()
        },
    )
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

#[test]
fn line_scroll_request_converts_lines_to_pixels() {
    let request = ScrollRequest::from_event(&AppEvent::ScrollLine {
        x: 10.0,
        y: 20.0,
        delta_x: 0.0,
        delta_y: 2.0,
    })
    .expect("scroll request");

    assert_eq!(request.x(), 10.0);
    assert_eq!(request.y(), 20.0);
    assert_eq!(request.delta_y(), -64.0);
}

#[test]
fn pixel_scroll_request_preserves_pixel_delta() {
    let request = ScrollRequest::from_event(&AppEvent::ScrollPixel {
        x: 10.0,
        y: 20.0,
        delta_x: 0.0,
        delta_y: -7.0,
    })
    .expect("scroll request");

    assert_eq!(request.delta_y(), 7.0);
}

#[test]
fn non_scroll_event_does_not_create_scroll_request() {
    assert!(ScrollRequest::from_event(&AppEvent::MouseMove { x: 1.0, y: 2.0 }).is_none());
}

#[test]
fn scroll_target_uses_matching_pointer_snapshot() {
    let mut tree = Tree::new();
    let scroll = tree.insert(scroll_node("scroll", rect(0.0, 0.0, 100.0, 100.0)));
    tree.set_root(scroll);
    let snapshot = PointerHitResolver::new(&tree, None)
        .snapshot_for_request(PointerHitRequest::new(20.0, 20.0, false));
    let request = ScrollRequest::new(20.0, 20.0, 5.0);

    let target =
        ScrollTargetResolver::new(&tree, None).target_for_request(Some(&snapshot), request);

    assert_eq!(target, Some(scroll));
}

#[test]
fn scroll_target_falls_back_when_snapshot_point_does_not_match() {
    let mut tree = Tree::new();
    let first = tree.insert(scroll_node("first", rect(0.0, 0.0, 40.0, 40.0)));
    let second = tree.insert(scroll_node("second", rect(50.0, 0.0, 40.0, 40.0)));
    let mut root = hittable_node("root", rect(0.0, 0.0, 100.0, 100.0));
    root.children = vec![first, second];
    let root = tree.insert(root);
    tree.set_root(root);
    let snapshot = PointerHitResolver::new(&tree, None)
        .snapshot_for_request(PointerHitRequest::new(10.0, 10.0, false));
    let request = ScrollRequest::new(60.0, 10.0, 5.0);

    let target =
        ScrollTargetResolver::new(&tree, None).target_for_request(Some(&snapshot), request);

    assert_eq!(target, Some(second));
}

#[test]
fn scroll_target_finds_scroll_ancestor_from_leaf_hit() {
    let mut tree = Tree::new();
    let leaf = tree.insert(hittable_node("leaf", rect(20.0, 20.0, 20.0, 20.0)));
    let mut scroll = scroll_node("scroll", rect(0.0, 0.0, 100.0, 100.0));
    scroll.children = vec![leaf];
    let scroll = tree.insert(scroll);
    tree.set_root(scroll);
    let request = ScrollRequest::new(25.0, 25.0, 5.0);

    let target = ScrollTargetResolver::new(&tree, None).target_for_request(None, request);

    assert_eq!(target, Some(scroll));
}
