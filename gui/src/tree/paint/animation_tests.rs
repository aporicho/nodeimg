use super::test_support::{
    container_node, leaf_node, only_command, paint_tree_with_animations, point, rect,
};
use crate::animation::{AnimationProps, AnimationStore, Ease};
use crate::paint::{Color, PaintCommand};
use crate::tree::layout::LeafKind;
use crate::tree::Tree;
use std::time::{Duration, Instant};

#[test]
fn animated_node_records_layer_with_opacity_and_transform() {
    let mut tree = Tree::new();
    let child = tree.insert(leaf_node(
        "child",
        LeafKind::Circle {
            radius: 4.0,
            fill: Some(Color::WHITE),
            stroke: None,
        },
        rect(10.0, 20.0, 8.0, 8.0),
    ));
    let root = tree.insert(container_node(
        "root",
        rect(0.0, 0.0, 100.0, 100.0),
        vec![child],
    ));
    tree.set_root(root);

    let mut animations = AnimationStore::new();
    let now = Instant::now();
    animations
        .animate("child")
        .to(AnimationProps::new().opacity(0.5).translate([12.0, 0.0]))
        .duration_ms(100)
        .ease(Ease::Linear)
        .play_at(now);
    animations.tick(now + Duration::from_millis(100));

    let list = paint_tree_with_animations(&tree, root, &animations);
    let command = only_command(&list);

    match &command.command {
        PaintCommand::Layer(layer) => {
            assert_eq!(layer.opacity, 0.5);
            assert_eq!(layer.content.commands.len(), 1);
        }
        other => panic!("expected animated layer, got {other:?}"),
    }
    assert_eq!(
        command.transform.transform_point(point(10.0, 20.0)),
        point(22.0, 20.0)
    );
}
