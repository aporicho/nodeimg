use super::*;
use crate::geometry::ResizeEdge;
use crate::renderer::Rect;
use crate::shell::{AppEvent, MouseButton};
use crate::tree::layout::BoxStyle;
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

fn resizable_node(id: &'static str, rect: Rect) -> TreeNode {
    node(
        id,
        rect,
        BoxStyle {
            hittable: true,
            resizable: true,
            ..Default::default()
        },
    )
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

#[test]
fn pointer_snapshot_reuses_hit_chain_for_event_point() {
    let mut tree = Tree::new();
    let root = tree.insert(hittable_node("root", rect(0.0, 0.0, 100.0, 100.0)));
    tree.set_root(root);
    let resolver = PointerHitResolver::new(&tree, None);
    let request =
        PointerHitRequest::from_event(&AppEvent::MouseMove { x: 20.0, y: 30.0 }).expect("request");

    let snapshot = resolver.snapshot_for_request(request);

    assert!(snapshot.matches_point(20.0, 30.0));
    assert_eq!(snapshot.chain().leaf(), Some(root));
    assert!(snapshot.resize_hit().is_none());
}

#[test]
fn resolver_falls_back_when_snapshot_point_does_not_match() {
    let mut tree = Tree::new();
    let first = tree.insert(hittable_node("first", rect(0.0, 0.0, 40.0, 40.0)));
    let second = tree.insert(hittable_node("second", rect(50.0, 0.0, 40.0, 40.0)));
    let root = tree.insert(hittable_node("root", rect(0.0, 0.0, 100.0, 100.0)));
    tree.get_mut(root).expect("root").children = vec![first, second];
    tree.set_root(root);
    let resolver = PointerHitResolver::new(&tree, None);
    let snapshot = resolver.snapshot_for_request(PointerHitRequest::new(10.0, 10.0, false));

    let fallback = resolver.chain_at(Some(&snapshot), 60.0, 10.0);

    assert_eq!(snapshot.chain().leaf(), Some(first));
    assert_eq!(fallback.leaf(), Some(second));
}

#[test]
fn left_mouse_press_includes_resize_hit() {
    let mut tree = Tree::new();
    let root = tree.insert(resizable_node("panel", rect(0.0, 0.0, 100.0, 100.0)));
    tree.set_root(root);
    let resolver = PointerHitResolver::new(&tree, None);
    let request = PointerHitRequest::from_event(&AppEvent::MousePress {
        x: 100.0,
        y: 50.0,
        button: MouseButton::Left,
    })
    .expect("request");

    let snapshot = resolver.snapshot_for_request(request);

    assert_eq!(snapshot.resize_hit(), Some((root, ResizeEdge::Right)));
}

#[test]
fn non_left_mouse_press_does_not_create_pointer_request() {
    let event = AppEvent::MousePress {
        x: 100.0,
        y: 50.0,
        button: MouseButton::Right,
    };

    assert!(PointerHitRequest::from_event(&event).is_none());
}

#[test]
fn empty_tree_snapshot_uses_empty_hit_chain() {
    let tree = Tree::new();
    let resolver = PointerHitResolver::new(&tree, None);
    let snapshot = resolver.snapshot_for_request(PointerHitRequest::new(10.0, 10.0, true));

    assert!(snapshot.chain().is_empty());
    assert!(snapshot.resize_hit().is_none());
}
