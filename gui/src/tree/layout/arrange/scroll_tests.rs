use super::arrange;
use super::test_support::{container, no_measure};
use crate::renderer::Rect;
use crate::tree::layout::{BoxStyle, Direction, Overflow, Position, Size};
use crate::tree::Tree;

#[test]
fn absolute_in_scroll_container() {
    let mut tree = Tree::new();
    let flow1_id = tree.insert(container(BoxStyle {
        height: Size::Fixed(200.0),
        ..Default::default()
    }));
    let flow2_id = tree.insert(container(BoxStyle {
        height: Size::Fixed(200.0),
        ..Default::default()
    }));
    let abs_id = tree.insert(container(BoxStyle {
        position: Position::absolute_xy(10.0, 10.0),
        width: Size::Fixed(100.0),
        height: Size::Fixed(999.0),
        ..Default::default()
    }));

    let mut root = container(BoxStyle {
        width: Size::Fixed(400.0),
        height: Size::Fixed(300.0),
        direction: Direction::Column,
        overflow: Overflow::Scroll,
        ..Default::default()
    });
    root.children = vec![flow1_id, flow2_id, abs_id];
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

    let root = tree.get(root_id).unwrap();
    assert_eq!(
        root.content_height(),
        400.0,
        "scroll content_height 应只累计 Flow 子节点"
    );
}
