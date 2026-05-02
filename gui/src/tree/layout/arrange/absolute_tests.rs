use super::arrange;
use super::test_support::{arrange_root, container, no_measure};
use crate::renderer::Rect;
use crate::tree::layout::{Align, BoxStyle, Direction, Edges, Inset, Position, Size};
use crate::tree::Tree;

#[test]
fn absolute_simple() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(100.0, 50.0),
        width: Size::Fixed(100.0),
        height: Size::Fixed(80.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(800.0),
        height: Size::Fixed(600.0),
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
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
        &mut measure,
    );

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.x, 100.0);
    assert_eq!(child.rect.y, 50.0);
    assert_eq!(child.rect.w, 100.0);
    assert_eq!(child.rect.h, 80.0);
}

#[test]
fn absolute_left_right_auto_width_stretches_between_insets() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_inset(Inset {
            top: Some(10.0),
            right: Some(25.0),
            bottom: None,
            left: Some(15.0),
        }),
        width: Size::Auto,
        height: Size::Fixed(30.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(200.0),
        height: Size::Fixed(100.0),
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 200.0, 100.0);

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.x, 15.0);
    assert_eq!(child.rect.y, 10.0);
    assert_eq!(child.rect.w, 160.0);
    assert_eq!(child.rect.h, 30.0);
}

#[test]
fn absolute_right_bottom_positions_from_containing_block_end() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_inset(Inset {
            top: None,
            right: Some(25.0),
            bottom: Some(10.0),
            left: None,
        }),
        width: Size::Fixed(50.0),
        height: Size::Fixed(30.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(200.0),
        height: Size::Fixed(100.0),
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 200.0, 100.0);

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.x, 125.0);
    assert_eq!(child.rect.y, 60.0);
    assert_eq!(child.rect.w, 50.0);
    assert_eq!(child.rect.h, 30.0);
}

#[test]
fn absolute_descendant_uses_nearest_relative_containing_block() {
    let mut tree = Tree::new();
    let abs_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(10.0, 15.0),
        width: Size::Fixed(20.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut static_wrapper = container(BoxStyle {
        width: Size::Fixed(80.0),
        height: Size::Fixed(50.0),
        padding: Edges::all(5.0),
        ..Default::default()
    });
    static_wrapper.children = vec![abs_id];
    let static_wrapper_id = tree.insert(static_wrapper);
    let mut relative_wrapper = container(BoxStyle {
        position: Position::relative(),
        width: Size::Fixed(100.0),
        height: Size::Fixed(70.0),
        padding: Edges::all(20.0),
        ..Default::default()
    });
    relative_wrapper.children = vec![static_wrapper_id];
    let relative_wrapper_id = tree.insert(relative_wrapper);
    let mut root = container(BoxStyle {
        width: Size::Fixed(200.0),
        height: Size::Fixed(120.0),
        ..Default::default()
    });
    root.children = vec![relative_wrapper_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 200.0, 120.0);

    let abs = tree.get(abs_id).unwrap();
    assert_eq!(abs.rect.x, 30.0);
    assert_eq!(abs.rect.y, 35.0);
}

#[test]
fn absolute_with_padding() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(100.0, 50.0),
        width: Size::Fixed(100.0),
        height: Size::Fixed(80.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        position: Position::relative(),
        width: Size::Fixed(800.0),
        height: Size::Fixed(600.0),
        padding: Edges::all(20.0),
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
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
        &mut measure,
    );

    let child = tree.get(child_id).unwrap();
    assert_eq!(
        child.rect.x, 120.0,
        "应该是 padding.left (20) + abs.x (100)"
    );
    assert_eq!(child.rect.y, 70.0, "应该是 padding.top (20) + abs.y (50)");
}

#[test]
fn absolute_does_not_take_flex_space() {
    let mut tree = Tree::new();

    let flex1_id = tree.insert(container(BoxStyle {
        flex_grow: 1.0,
        height: Size::Fixed(100.0),
        ..Default::default()
    }));
    let flex2_id = tree.insert(container(BoxStyle {
        flex_grow: 1.0,
        height: Size::Fixed(100.0),
        ..Default::default()
    }));
    let abs_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(200.0, 200.0),
        width: Size::Fixed(50.0),
        height: Size::Fixed(50.0),
        ..Default::default()
    }));

    let mut root = container(BoxStyle {
        width: Size::Fixed(800.0),
        height: Size::Fixed(600.0),
        direction: Direction::Row,
        ..Default::default()
    });
    root.children = vec![flex1_id, flex2_id, abs_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let mut measure = no_measure;
    arrange(
        &mut tree,
        root_id,
        Rect {
            x: 0.0,
            y: 0.0,
            w: 800.0,
            h: 600.0,
        },
        &mut measure,
    );

    let flex1 = tree.get(flex1_id).unwrap();
    let flex2 = tree.get(flex2_id).unwrap();
    assert_eq!(flex1.rect.w, 400.0, "Flex1 应占一半");
    assert_eq!(flex2.rect.w, 400.0, "Flex2 应占一半");
    assert_eq!(flex1.rect.x, 0.0);
    assert_eq!(flex2.rect.x, 400.0);

    let abs = tree.get(abs_id).unwrap();
    assert_eq!(abs.rect.x, 200.0);
    assert_eq!(abs.rect.y, 200.0);
}

#[test]
fn absolute_child_does_not_contribute_to_auto_parent_size() {
    let mut tree = Tree::new();
    let flow_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(30.0),
        height: Size::Fixed(20.0),
        ..Default::default()
    }));
    let abs_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(0.0, 0.0),
        width: Size::Fixed(200.0),
        height: Size::Fixed(100.0),
        ..Default::default()
    }));
    let mut auto_parent = container(BoxStyle {
        width: Size::Auto,
        height: Size::Auto,
        direction: Direction::Row,
        ..Default::default()
    });
    auto_parent.children = vec![flow_id, abs_id];
    let auto_parent_id = tree.insert(auto_parent);
    let sibling_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(10.0),
        height: Size::Fixed(20.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(300.0),
        height: Size::Fixed(100.0),
        direction: Direction::Row,
        align_items: Align::Start,
        ..Default::default()
    });
    root.children = vec![auto_parent_id, sibling_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 300.0, 100.0);

    let auto_parent = tree.get(auto_parent_id).unwrap();
    let sibling = tree.get(sibling_id).unwrap();
    assert_eq!(auto_parent.rect.w, 30.0);
    assert_eq!(auto_parent.rect.h, 20.0);
    assert_eq!(sibling.rect.x, 30.0);
}

#[test]
fn absolute_fixed_size() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(50.0, 50.0),
        width: Size::Fixed(200.0),
        height: Size::Fixed(150.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(400.0),
        height: Size::Fixed(300.0),
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
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 300.0,
        },
        &mut measure,
    );

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.x, 50.0);
    assert_eq!(child.rect.y, 50.0);
    assert_eq!(child.rect.w, 200.0);
    assert_eq!(child.rect.h, 150.0);
}
