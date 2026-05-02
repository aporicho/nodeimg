use super::hit_test;
use super::test_support::{container_with_rect, leaf_with_rect};
use crate::geometry::{Point, TransformSpec};
use crate::renderer::{Color, Rect, Stroke};
use crate::tree::layout::{BoxStyle, LeafKind, Overflow};
use crate::tree::Tree;
use std::borrow::Cow;

#[test]
fn hit_circle_leaf_uses_shape_instead_of_bounds() {
    let mut tree = Tree::new();
    let circle_id = tree.insert(leaf_with_rect(
        "circle",
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
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
        n.children = vec![circle_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert_eq!(hit_test(&tree, root_id, 5.0, 5.0).leaf(), Some(circle_id));
    assert!(hit_test(&tree, root_id, 0.0, 0.0).is_empty());
}

#[test]
fn hit_line_leaf_uses_stroke_distance_instead_of_bounds() {
    let mut tree = Tree::new();
    let line_id = tree.insert(leaf_with_rect(
        "line",
        LeafKind::Line {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 20.0, y: 0.0 },
            stroke: Stroke::new(2.0, Color::WHITE),
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 20.0,
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
        n.children = vec![line_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert_eq!(hit_test(&tree, root_id, 10.0, 0.5).leaf(), Some(line_id));
    assert!(hit_test(&tree, root_id, 10.0, 10.0).is_empty());
}

#[test]
fn hit_line_leaf_respects_affine_transform() {
    let mut tree = Tree::new();
    let line_id = tree.insert(leaf_with_rect(
        "line",
        LeafKind::Line {
            start: Point { x: 0.0, y: 0.0 },
            end: Point { x: 20.0, y: 0.0 },
            stroke: Stroke::new(2.0, Color::WHITE),
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 4.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale_rotate(
                    [0.0, 0.0],
                    1.0,
                    std::f32::consts::FRAC_PI_2,
                )),
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
        n.children = vec![line_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert_eq!(hit_test(&tree, root_id, -10.5, 20.0).leaf(), Some(line_id));
    assert!(hit_test(&tree, root_id, 20.0, 10.5).is_empty());
}

#[test]
fn hit_shape_leaf_is_clipped_by_hidden_overflow_parent() {
    let mut tree = Tree::new();
    let circle_id = tree.insert(leaf_with_rect(
        "circle",
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 30.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                overflow: Overflow::Hidden,
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 20.0,
                h: 20.0,
            },
        );
        n.children = vec![circle_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert!(hit_test(&tree, root_id, 35.0, 5.0).is_empty());
}

#[test]
fn hit_connection_leaf_uses_resolved_path_even_with_zero_bounds() {
    let mut tree = Tree::new();
    let from_id = tree.insert(leaf_with_rect(
        "from",
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        BoxStyle::default(),
        Rect {
            x: 10.0,
            y: 50.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let to_id = tree.insert(leaf_with_rect(
        "to",
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        BoxStyle::default(),
        Rect {
            x: 90.0,
            y: 50.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let connection_id = tree.insert(leaf_with_rect(
        "connection",
        LeafKind::Connection {
            from_port: Cow::Borrowed("from"),
            to_port: Cow::Borrowed("to"),
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 120.0,
                h: 120.0,
            },
        );
        n.children = vec![from_id, to_id, connection_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert_eq!(
        hit_test(&tree, root_id, 55.0, 55.0).leaf(),
        Some(connection_id)
    );
    assert!(hit_test(&tree, root_id, 55.0, 60.0).is_empty());
}

#[test]
fn hit_pending_connection_leaf_uses_cursor_canvas_path() {
    let mut tree = Tree::new();
    let from_id = tree.insert(leaf_with_rect(
        "from",
        LeafKind::Circle {
            radius: 5.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        BoxStyle::default(),
        Rect {
            x: 10.0,
            y: 50.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let pending_id = tree.insert(leaf_with_rect(
        "pending",
        LeafKind::PendingConnection {
            from_port: Cow::Borrowed("from"),
            cursor_canvas: Point { x: 95.0, y: 55.0 },
        },
        BoxStyle {
            hittable: true,
            ..Default::default()
        },
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 120.0,
                h: 120.0,
            },
        );
        n.children = vec![from_id, pending_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    assert_eq!(
        hit_test(&tree, root_id, 55.0, 55.0).leaf(),
        Some(pending_id)
    );
    assert!(hit_test(&tree, root_id, 55.0, 60.0).is_empty());
}
