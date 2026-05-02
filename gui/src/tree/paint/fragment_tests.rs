use super::build_paint_fragment;
use super::test_support::{container_node, leaf_node, paint_cx, point, rect};
use crate::geometry::TransformSpec;
use crate::paint::{Color, Stroke};
use crate::theme::Theme;
use crate::tree::layout::LeafKind;
use crate::tree::{RepaintBoundaryId, RepaintBoundaryReason, Tree};

#[test]
fn fragment_records_boundary_local_coordinates() {
    let stroke = Stroke::new(2.0, Color::WHITE);
    let mut tree = Tree::new();
    let root = tree.insert(leaf_node(
        "leaf",
        LeafKind::Line {
            start: point(1.0, 2.0),
            end: point(3.0, 4.0),
            stroke,
        },
        rect(10.0, 20.0, 100.0, 50.0),
    ));
    tree.set_root(root);

    let fragment = build_paint_fragment(
        &tree,
        RepaintBoundaryId(root),
        paint_cx(&Theme::default()),
        |_, _| (0.0, 0.0),
    )
    .expect("fragment");

    assert_eq!(fragment.commands.len(), 1);
    assert_eq!(
        fragment.commands[0]
            .transform
            .transform_point(point(0.0, 0.0)),
        point(0.0, 0.0)
    );
}

#[test]
fn fragment_records_child_repaint_boundary_ref_without_painting_child() {
    let stroke = Stroke::new(2.0, Color::WHITE);
    let mut tree = Tree::new();
    let child = tree.insert(leaf_node(
        "child",
        LeafKind::Line {
            start: point(1.0, 2.0),
            end: point(3.0, 4.0),
            stroke,
        },
        rect(20.0, 30.0, 20.0, 20.0),
    ));
    tree.set_repaint_boundary(child, RepaintBoundaryReason::CanvasNodeCard);
    let root = tree.insert(container_node(
        "root",
        rect(10.0, 10.0, 100.0, 80.0),
        vec![child],
    ));
    tree.set_root(root);

    let fragment = build_paint_fragment(
        &tree,
        RepaintBoundaryId(root),
        paint_cx(&Theme::default()),
        |_, _| (0.0, 0.0),
    )
    .expect("fragment");

    assert!(fragment.commands.is_empty());
    assert_eq!(fragment.child_boundaries.len(), 1);
    assert_eq!(
        fragment.child_boundaries[0].boundary,
        RepaintBoundaryId(child)
    );
    assert_eq!(
        fragment.child_boundaries[0]
            .local_transform
            .transform_point(point(0.0, 0.0)),
        point(10.0, 20.0)
    );
}

#[test]
fn fragment_child_boundary_ref_includes_ancestor_transform() {
    let stroke = Stroke::new(2.0, Color::WHITE);
    let mut tree = Tree::new();
    let child = tree.insert(leaf_node(
        "child",
        LeafKind::Line {
            start: point(1.0, 2.0),
            end: point(3.0, 4.0),
            stroke,
        },
        rect(20.0, 30.0, 20.0, 20.0),
    ));
    tree.set_repaint_boundary(child, RepaintBoundaryReason::CanvasNodeCard);
    let mut canvas = container_node("canvas", rect(10.0, 10.0, 100.0, 80.0), vec![child]);
    canvas.style.transform = Some(TransformSpec::translate_scale([100.0, 50.0], 2.0));
    let canvas = tree.insert(canvas);
    tree.set_root(canvas);

    let fragment = build_paint_fragment(
        &tree,
        RepaintBoundaryId(canvas),
        paint_cx(&Theme::default()),
        |_, _| (0.0, 0.0),
    )
    .expect("fragment");

    assert_eq!(fragment.child_boundaries.len(), 1);
    assert_eq!(
        fragment.child_boundaries[0]
            .local_transform
            .transform_point(point(0.0, 0.0)),
        point(140.0, 110.0)
    );
}
