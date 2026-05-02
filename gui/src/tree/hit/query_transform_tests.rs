use super::test_support::{container_with_rect, decor, hittable_style};
use super::{hit_test, hit_test_with_animations, screen_to_node_layout_point};
use crate::animation::{AnimationProps, AnimationStore, Ease};
use crate::geometry::TransformSpec;
use crate::renderer::Rect;
use crate::tree::layout::BoxStyle;
use crate::tree::Tree;
use std::time::{Duration, Instant};

#[test]
fn hit_transform_identity() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 30.0,
            h: 30.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale([0.0, 0.0], 1.0)),
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
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 15.0, 15.0);
    assert_eq!(chain.leaf(), Some(child_id));
}

#[test]
fn hit_transform_translate() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale([50.0, 50.0], 1.0)),
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 200.0,
            },
        );
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    // 屏幕 (70, 70) → 逆变换 (70-50, 70-50) = (20, 20) → 命中 child local (10,10,20,20)
    let chain = hit_test(&tree, root_id, 70.0, 70.0);
    assert_eq!(chain.leaf(), Some(child_id));
}

#[test]
fn hit_transform_scale() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale([0.0, 0.0], 2.0)),
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 200.0,
            },
        );
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    // 屏幕 (30, 30) → 逆变换 (30/2, 30/2) = (15, 15) → 命中 child local (10,10,10,10)
    let chain = hit_test(&tree, root_id, 30.0, 30.0);
    assert_eq!(chain.leaf(), Some(child_id));
}

#[test]
fn screen_to_node_layout_point_inverts_transformed_parent_space() {
    let mut tree = Tree::new();
    let field_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 20.0,
            y: 30.0,
            w: 100.0,
            h: 40.0,
        },
    ));
    let canvas_id = {
        let mut node = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale([100.0, -40.0], 2.0)),
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 300.0,
                h: 200.0,
            },
        );
        node.children = vec![field_id];
        tree.insert(node)
    };
    let root_id = {
        let mut node = container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 500.0,
                h: 400.0,
            },
        );
        node.children = vec![canvas_id];
        tree.insert(node)
    };
    tree.set_root(root_id);

    let point = screen_to_node_layout_point(&tree, root_id, field_id, 150.0, 30.0, None)
        .expect("screen point should map to target layout space");

    assert!((point.x - 25.0).abs() < 0.0001, "point={point:?}");
    assert!((point.y - 35.0).abs() < 0.0001, "point={point:?}");
}

#[test]
fn hit_uses_animation_transform_for_whole_node() {
    let mut tree = Tree::new();
    let node_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
        },
    ));
    tree.set_root(node_id);

    let mut animations = AnimationStore::new();
    let now = Instant::now();
    animations
        .animate("test")
        .to(AnimationProps::new().translate([40.0, 0.0]))
        .duration_ms(100)
        .ease(Ease::Linear)
        .play_at(now);
    animations.tick(now + Duration::from_millis(100));

    let chain = hit_test_with_animations(&tree, node_id, 55.0, 15.0, Some(&animations));

    assert_eq!(chain.leaf(), Some(node_id));
    assert!(hit_test_with_animations(&tree, node_id, 15.0, 15.0, Some(&animations)).is_empty());
}

#[test]
fn hit_transform_rotate_uses_affine_inverse() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 10.0,
            h: 10.0,
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
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let rotated_hit = hit_test(&tree, root_id, -15.0, 15.0);
    assert_eq!(rotated_hit.leaf(), Some(child_id));

    let unrotated_point = hit_test(&tree, root_id, 15.0, 15.0);
    assert!(unrotated_point.is_empty());
}

#[test]
fn hit_transform_zero_scale_is_not_hittable() {
    let mut tree = Tree::new();
    let child_id = tree.insert(container_with_rect(
        hittable_style(),
        Some(decor()),
        Rect {
            x: 10.0,
            y: 10.0,
            w: 10.0,
            h: 10.0,
        },
    ));
    let root = {
        let mut n = container_with_rect(
            BoxStyle {
                transform: Some(TransformSpec::translate_scale([0.0, 0.0], 0.0)),
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
        n.children = vec![child_id];
        n
    };
    let root_id = tree.insert(root);
    tree.set_root(root_id);

    let chain = hit_test(&tree, root_id, 10.0, 10.0);
    assert!(chain.is_empty());
}
