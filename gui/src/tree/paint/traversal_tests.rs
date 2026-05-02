use super::test_support::{
    container_node, leaf_node, only_clip, only_command, paint_tree, point, rect,
};
use crate::geometry::TransformSpec;
use crate::paint::{
    Border, ClipShape, Color, PaintCommand, PathData, RectPaint, RectStyle, Stroke,
};
use crate::tree::layout::{LeafKind, Overflow};
use crate::tree::Tree;

#[test]
fn overflow_hidden_records_local_clip_around_children() {
    let mut tree = Tree::new();
    let child = tree.insert(leaf_node(
        "child",
        LeafKind::Line {
            start: point(0.0, 0.0),
            end: point(10.0, 0.0),
            stroke: Stroke::new(1.0, Color::WHITE),
        },
        rect(0.0, 0.0, 10.0, 10.0),
    ));
    let mut root_node = container_node("root", rect(1.0, 2.0, 30.0, 40.0), vec![child]);
    root_node.style.overflow = Overflow::Hidden;
    let root = tree.insert(root_node);
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let clip = only_clip(&list);

    assert!(matches!(
        clip.shape,
        ClipShape::RoundedRect {
            rect: clip_rect,
            radius: [0.0, 0.0, 0.0, 0.0],
        } if clip_rect == rect(0.0, 0.0, 30.0, 40.0)
    ));
    assert_eq!(
        clip.transform.transform_point(point(0.0, 0.0)),
        point(1.0, 2.0)
    );
    assert_eq!(list.commands[0].clips, vec![clip.id]);
}

#[test]
fn transform_rotate_records_affine_without_screen_space_points() {
    let mut tree = Tree::new();
    let child = tree.insert(leaf_node(
        "child",
        LeafKind::Line {
            start: point(0.0, 0.0),
            end: point(10.0, 0.0),
            stroke: Stroke::new(1.0, Color::WHITE),
        },
        rect(10.0, 10.0, 10.0, 10.0),
    ));
    let mut root_node = container_node("root", rect(0.0, 0.0, 100.0, 100.0), vec![child]);
    root_node.style.transform = Some(TransformSpec::translate_scale_rotate(
        [0.0, 0.0],
        1.0,
        std::f32::consts::FRAC_PI_2,
    ));
    let root = tree.insert(root_node);
    tree.set_root(root);

    let list = paint_tree(&tree, root);
    let command = only_command(&list);

    assert!(matches!(
        &command.command,
        PaintCommand::Path(path)
            if path.data == PathData::line(point(0.0, 0.0), point(10.0, 0.0))
    ));
    assert_eq!(
        command.transform.transform_point(point(0.0, 0.0)),
        point(-10.0, 10.0)
    );
}

#[test]
fn decoration_records_local_rect_command() {
    let mut tree = Tree::new();
    let mut root_node = container_node("root", rect(10.0, 20.0, 30.0, 40.0), Vec::new());
    root_node.decoration = Some(crate::tree::layout::Decoration {
        background: Some(Color::BLACK),
        border: Some(Border {
            width: 2.0,
            color: Color::WHITE,
        }),
        radius: [3.0; 4],
        shadow: None,
    });
    let root = tree.insert(root_node);
    tree.set_root(root);

    let list = paint_tree(&tree, root);

    assert_eq!(
        only_command(&list).command,
        PaintCommand::Rect(RectPaint {
            rect: rect(0.0, 0.0, 30.0, 40.0),
            style: RectStyle {
                color: Color::BLACK,
                border: Some(Border {
                    width: 2.0,
                    color: Color::WHITE,
                }),
                radius: [3.0; 4],
                shadow: None,
            },
        })
    );
}
