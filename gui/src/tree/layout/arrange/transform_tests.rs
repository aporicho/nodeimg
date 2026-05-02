use super::arrange;
use super::test_support::{container, no_measure};
use crate::geometry::TransformSpec;
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Edges, Position, Size};
use crate::tree::Tree;

#[test]
fn transform_node_children_in_local_space() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(50.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(400.0),
        height: Size::Fixed(300.0),
        transform: Some(TransformSpec::translate_scale([10.0, 20.0], 2.0)),
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 50.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        },
        &mut measure,
    );

    let child = tree.get(child_id).unwrap();
    assert_eq!(
        child.rect.x, 0.0,
        "TransformSpec 父的 Flow 子应从 local origin (0,0) 开始"
    );
    assert_eq!(child.rect.y, 0.0);
    assert_eq!(child.rect.w, 100.0);
    assert_eq!(child.rect.h, 50.0);
}

#[test]
fn transform_node_absolute_children_use_local_content_box() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(10.0, 15.0),
        width: Size::Fixed(100.0),
        height: Size::Fixed(50.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(400.0),
        height: Size::Fixed(300.0),
        padding: Edges::all(20.0),
        transform: Some(TransformSpec::translate_scale([10.0, 20.0], 2.0)),
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 50.0,
            y: 60.0,
            w: 400.0,
            h: 300.0,
        },
        &mut measure,
    );

    let child = tree.get(child_id).unwrap();
    assert_eq!(
        child.rect.x, 30.0,
        "TransformSpec 父的 Absolute 子应从 local padding + abs.x 开始"
    );
    assert_eq!(child.rect.y, 35.0);
    assert_eq!(child.rect.w, 100.0);
    assert_eq!(child.rect.h, 50.0);
}
