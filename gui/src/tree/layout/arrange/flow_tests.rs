use super::test_support::{arrange_root, auto_wrapper_with_fixed_child, container};
use crate::tree::layout::{Align, BoxStyle, Direction, Inset, Position, Size};
use crate::tree::Tree;

#[test]
fn relative_child_offsets_rect_but_keeps_flow_slot() {
    let mut tree = Tree::new();
    let relative_id = tree.insert(container(BoxStyle {
        position: Position::relative_inset(Inset::xy(10.0, 5.0)),
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let sibling_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(40.0),
        height: Size::Fixed(10.0),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(50.0),
        direction: Direction::Column,
        ..Default::default()
    });
    root.children = vec![relative_id, sibling_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 100.0, 50.0);

    let relative = tree.get(relative_id).unwrap();
    let sibling = tree.get(sibling_id).unwrap();
    assert_eq!(relative.rect.x, 10.0);
    assert_eq!(relative.rect.y, 5.0);
    assert_eq!(sibling.rect.y, 10.0);
}

#[test]
fn align_self_stretches_child_when_parent_centers_items() {
    let mut tree = Tree::new();
    let child_id = auto_wrapper_with_fixed_child(
        &mut tree,
        20.0,
        10.0,
        BoxStyle {
            width: Size::Fixed(20.0),
            align_self: Some(Align::Stretch),
            ..Default::default()
        },
    );
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(60.0),
        direction: Direction::Row,
        align_items: Align::Center,
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 100.0, 60.0);

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.y, 0.0);
    assert_eq!(child.rect.h, 60.0);
}

#[test]
fn align_self_start_overrides_parent_stretch() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container(BoxStyle {
        width: Size::Fixed(20.0),
        height: Size::Fixed(10.0),
        align_self: Some(Align::Start),
        ..Default::default()
    }));
    let mut root = container(BoxStyle {
        width: Size::Fixed(100.0),
        height: Size::Fixed(60.0),
        direction: Direction::Row,
        align_items: Align::Stretch,
        ..Default::default()
    });
    root.children = vec![child_id];
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    arrange_root(&mut tree, root_id, 100.0, 60.0);

    let child = tree.get(child_id).unwrap();
    assert_eq!(child.rect.y, 0.0);
    assert_eq!(child.rect.h, 10.0);
}
