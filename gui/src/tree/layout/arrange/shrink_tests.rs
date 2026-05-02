use super::arrange;
use super::test_support::{auto_wrapper_with_fixed_child, container, no_measure};
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Direction, Size};
use crate::tree::Tree;

#[test]
fn row_shrinks_flexible_child_when_content_overflows() {
    let mut tree = Tree::new();
    let shrink_id = auto_wrapper_with_fixed_child(
        &mut tree,
        120.0,
        10.0,
        BoxStyle {
            flex_shrink: 1.0,
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let fixed_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(20.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![shrink_id, fixed_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
        &mut measure,
    );

    assert_eq!(tree.get(shrink_id).unwrap().rect.w, 60.0);
    assert_eq!(tree.get(fixed_id).unwrap().rect.w, 40.0);
}

#[test]
fn row_shrinks_fill_child_when_content_overflows() {
    let mut tree = Tree::new();
    let fill_id = auto_wrapper_with_fixed_child(
        &mut tree,
        120.0,
        10.0,
        BoxStyle {
            width: Size::Fill,
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let fixed_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(20.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![fill_id, fixed_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
        &mut measure,
    );

    assert_eq!(tree.get(fill_id).unwrap().rect.w, 60.0);
    assert_eq!(tree.get(fixed_id).unwrap().rect.x, 60.0);
}

#[test]
fn row_keeps_default_non_shrinking_child_width() {
    let mut tree = Tree::new();
    let first_id = auto_wrapper_with_fixed_child(
        &mut tree,
        120.0,
        10.0,
        BoxStyle {
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let second_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(20.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![first_id, second_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
        &mut measure,
    );

    assert_eq!(tree.get(first_id).unwrap().rect.w, 120.0);
    assert_eq!(tree.get(second_id).unwrap().rect.x, 120.0);
}

#[test]
fn row_distributes_shrink_by_weighted_base_size() {
    let mut tree = Tree::new();
    let first_id = auto_wrapper_with_fixed_child(
        &mut tree,
        100.0,
        10.0,
        BoxStyle {
            flex_shrink: 2.0,
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let second_id = auto_wrapper_with_fixed_child(
        &mut tree,
        100.0,
        10.0,
        BoxStyle {
            flex_shrink: 1.0,
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let mut root = container(BoxStyle {
        width: Size::Fixed(150.0),
        height: Size::Fixed(20.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![first_id, second_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 150.0,
            h: 20.0,
        },
        &mut measure,
    );

    assert!((tree.get(first_id).unwrap().rect.w - 66.66667).abs() < 0.001);
    assert!((tree.get(second_id).unwrap().rect.w - 83.33333).abs() < 0.001);
}

#[test]
fn row_shrink_respects_min_width() {
    let mut tree = Tree::new();
    let shrink_id = auto_wrapper_with_fixed_child(
        &mut tree,
        120.0,
        10.0,
        BoxStyle {
            flex_shrink: 1.0,
            min_width: 80.0,
            height: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let fixed_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(20.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![shrink_id, fixed_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 20.0,
        },
        &mut measure,
    );

    assert_eq!(tree.get(shrink_id).unwrap().rect.w, 80.0);
}

#[test]
fn column_shrinks_height_and_respects_min_height() {
    let mut tree = Tree::new();
    let shrink_id = auto_wrapper_with_fixed_child(
        &mut tree,
        10.0,
        120.0,
        BoxStyle {
            flex_shrink: 1.0,
            min_height: 70.0,
            width: Size::Fixed(10.0),
            ..Default::default()
        },
    );
    let fixed_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(10.0),
        height: Size::Fixed(40.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(20.0),
        height: Size::Fixed(100.0),
        direction: Direction::Column,
        ..Default::default()
    });
    root.children = vec![shrink_id, fixed_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 20.0,
            h: 100.0,
        },
        &mut measure,
    );

    assert_eq!(tree.get(shrink_id).unwrap().rect.h, 70.0);
}
