use super::resize::resize_hit_at_screen_point;
use crate::control::ResizeEdge;
use crate::geometry::TransformSpec;
use crate::renderer::{Color, Rect};
use crate::tree::layout::{BoxStyle, Decoration, Overflow};
use crate::tree::{Tree, TreeNode, TreeNodeBuilder};

fn container(id: &'static str, style: BoxStyle, rect: Rect) -> TreeNode {
    TreeNodeBuilder::container(id, style).rect(rect).build()
}

fn resizable_style() -> BoxStyle {
    BoxStyle {
        resizable: true,
        ..Default::default()
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn rounded_decoration(radius: [f32; 4]) -> Decoration {
    Decoration {
        background: Some(Color::WHITE),
        border: None,
        radius,
        shadow: None,
    }
}

#[test]
fn resize_hit_uses_transformed_parent_space() {
    let mut tree = Tree::new();
    let card_id = tree.insert(container(
        "card",
        resizable_style(),
        rect(20.0, 30.0, 100.0, 40.0),
    ));
    let mut canvas = container(
        "canvas",
        BoxStyle {
            overflow: Overflow::Visible,
            transform: Some(TransformSpec::translate_scale([100.0, -40.0], 2.0)),
            ..Default::default()
        },
        rect(0.0, 0.0, 300.0, 200.0),
    );
    canvas.children = vec![card_id];
    let canvas_id = tree.insert(canvas);
    let mut root = container("root", BoxStyle::default(), rect(0.0, 0.0, 500.0, 400.0));
    root.children = vec![canvas_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let hit = resize_hit_at_screen_point(&tree, root_id, 340.0, 60.0, None).expect("resize hit");

    assert_eq!(hit.node_id, card_id);
    assert_eq!(hit.edge, ResizeEdge::Right);
}

#[test]
fn resize_hit_respects_rounded_container_shape() {
    let mut tree = Tree::new();
    let mut card = container("card", resizable_style(), rect(0.0, 0.0, 100.0, 100.0));
    card.decoration = Some(rounded_decoration([40.0; 4]));
    let card_id = tree.insert(card);
    tree.set_root(card_id);

    assert_eq!(
        resize_hit_at_screen_point(&tree, card_id, 100.0, 100.0, None),
        None
    );

    let hit = resize_hit_at_screen_point(&tree, card_id, 88.0, 88.0, None).expect("arc resize hit");
    assert_eq!(hit.node_id, card_id);
    assert_eq!(hit.edge, ResizeEdge::BottomRight);
}

#[test]
fn resize_hit_includes_edge_band_outside_container_bounds() {
    let mut tree = Tree::new();
    let card_id = tree.insert(container(
        "card",
        resizable_style(),
        rect(0.0, 0.0, 100.0, 100.0),
    ));
    tree.set_root(card_id);

    let hit = resize_hit_at_screen_point(&tree, card_id, 106.0, 50.0, None)
        .expect("outside edge band should resize");

    assert_eq!(hit.node_id, card_id);
    assert_eq!(hit.edge, ResizeEdge::Right);
}

#[test]
fn resize_hit_prefers_topmost_overlapping_child() {
    let mut tree = Tree::new();
    let lower_id = tree.insert(container(
        "lower",
        BoxStyle {
            z_index: 0,
            ..resizable_style()
        },
        rect(0.0, 0.0, 100.0, 100.0),
    ));
    let upper_id = tree.insert(container(
        "upper",
        BoxStyle {
            z_index: 10,
            ..resizable_style()
        },
        rect(0.0, 0.0, 100.0, 100.0),
    ));
    let mut root = container("root", BoxStyle::default(), rect(0.0, 0.0, 200.0, 200.0));
    root.children = vec![lower_id, upper_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let hit = resize_hit_at_screen_point(&tree, root_id, 100.0, 50.0, None).expect("resize hit");

    assert_eq!(hit.node_id, upper_id);
    assert_eq!(hit.edge, ResizeEdge::Right);
}
