use super::hit_shape::leaf_shape_hit;
use super::layout::{BoxStyle, Decoration, Overflow};
use super::node::{NodeId, NodeKind, TreeNode};
use super::paint_space::{NodePaintSpace, PaintSpace};
use super::tree::Tree;
use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::Point;

/// 从命中的叶子到根的节点链。
/// 链上 [0] 是最深的命中节点（叶子），[len-1] 是 root。
#[derive(Debug, Clone)]
pub struct HitChain {
    nodes: Vec<NodeId>,
}

impl HitChain {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        Self { nodes }
    }
    pub fn empty() -> Self {
        Self { nodes: Vec::new() }
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 最深的命中节点（叶子）
    pub fn leaf(&self) -> Option<NodeId> {
        self.nodes.first().copied()
    }
    /// 最外层的命中节点（root）
    pub fn root(&self) -> Option<NodeId> {
        self.nodes.last().copied()
    }

    /// 从叶子到根顺序遍历
    pub fn iter(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.iter().copied()
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains(&id)
    }
}

/// 公开入口：返回命中链。从 root 开始向下递归，返回的链从叶子到根。
/// 如果没命中返回 empty HitChain。
#[cfg(test)]
pub fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain {
    hit_test_with_animations(tree, root, x, y, None)
}

pub fn hit_test_with_animations(
    tree: &Tree,
    root: NodeId,
    x: f32,
    y: f32,
    animations: Option<&AnimationStore>,
) -> HitChain {
    let mut nodes = Vec::new();
    hit_recursive(
        tree,
        root,
        Point { x, y },
        PaintSpace::root(),
        animations,
        &mut nodes,
    );
    HitChain::new(nodes)
}

pub(crate) fn screen_to_node_layout_point(
    tree: &Tree,
    root: NodeId,
    target: NodeId,
    x: f32,
    y: f32,
    animations: Option<&AnimationStore>,
) -> Option<Point> {
    screen_to_node_layout_point_recursive(
        tree,
        root,
        target,
        Point { x, y },
        PaintSpace::root(),
        animations,
    )
}

fn screen_to_node_layout_point_recursive(
    tree: &Tree,
    node_id: NodeId,
    target: NodeId,
    screen: Point,
    current_space: PaintSpace,
    animations: Option<&AnimationStore>,
) -> Option<Point> {
    let node = tree.get(node_id)?;
    let visual = animations.and_then(|store| store.visual_for(node.id.as_ref()));
    let current_space = visual
        .and_then(|visual| visual_affine(node.rect, visual))
        .map(|transform| current_space.transformed(transform))
        .unwrap_or(current_space);

    if node_id == target {
        return current_space
            .to_screen
            .inverse()
            .map(|inverse| inverse.transform_point(screen));
    }

    let node_space = current_space.node_space(node.rect, node.style.transform);
    let child_space = if node_space.children_are_local {
        node_space.child_space()
    } else {
        current_space
    };

    for child_id in &node.children {
        if let Some(point) = screen_to_node_layout_point_recursive(
            tree,
            *child_id,
            target,
            screen,
            child_space,
            animations,
        ) {
            return Some(point);
        }
    }

    None
}

fn hit_recursive(
    tree: &Tree,
    node_id: NodeId,
    screen: Point,
    current_space: PaintSpace,
    animations: Option<&AnimationStore>,
    chain: &mut Vec<NodeId>,
) -> bool {
    let Some(node) = tree.get(node_id) else {
        return false;
    };

    let visual = animations.and_then(|store| store.visual_for(node.id.as_ref()));
    let current_space = visual
        .and_then(|visual| visual_affine(node.rect, visual))
        .map(|transform| current_space.transformed(transform))
        .unwrap_or(current_space);
    let node_space = current_space.node_space(node.rect, node.style.transform);
    let Some(local_point) = current_space.point_to_local(node.rect, screen) else {
        return false;
    };

    let inside_bounds = node_space.contains_local_bounds(local_point);
    if !inside_bounds && node.style.overflow != Overflow::Visible {
        return false;
    }

    let children = tree.children_in_hit_order_cached(node_id, &node.children);
    let style = node.style.clone();
    let decoration = node.decoration.clone();
    let child_space = if node_space.children_are_local {
        node_space.child_space()
    } else {
        current_space
    };

    for child_id in children {
        if hit_recursive(tree, child_id, screen, child_space, animations, chain) {
            chain.push(node_id);
            return true;
        }
    }

    let self_hit = node_self_hit(
        tree,
        node,
        local_point,
        node_space,
        current_space,
        inside_bounds,
    );
    if self_hit && is_hittable(&style, &decoration) {
        chain.push(node_id);
        return true;
    }

    false
}

fn node_self_hit(
    tree: &Tree,
    node: &TreeNode,
    local_point: Point,
    node_space: NodePaintSpace,
    current_space: PaintSpace,
    inside_bounds: bool,
) -> bool {
    match &node.kind {
        NodeKind::Leaf(leaf) => leaf_shape_hit(tree, leaf, local_point, node_space, current_space)
            .unwrap_or(inside_bounds),
        NodeKind::Container | NodeKind::Widget(_) => inside_bounds,
    }
}

/// 混合判据：hittable 强制覆盖优先，否则交互能力、gestures 或 decoration 可命中。
fn is_hittable(style: &BoxStyle, decoration: &Option<Decoration>) -> bool {
    match style.hittable {
        Some(h) => h,
        None => {
            style.draggable || style.resizable || !style.gestures.is_empty() || decoration.is_some()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::{AnimationProps, AnimationStore, Ease};
    use crate::geometry::TransformSpec;
    use crate::renderer::{Color, Rect, Stroke};
    use crate::tree::layout::{BoxStyle, Decoration, LeafKind};
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::{NodeProps, RuntimeSlots};
    use std::borrow::Cow;
    use std::time::{Duration, Instant};

    fn node_with_rect(
        id: &'static str,
        style: BoxStyle,
        decoration: Option<Decoration>,
        kind: NodeKind,
        rect: Rect,
    ) -> TreeNode {
        TreeNode {
            id: Cow::Borrowed(id).into(),
            props: NodeProps::default(),
            style,
            decoration,
            kind,
            rect,
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    /// 构造一个设定好 rect 的 Container 节点
    fn container_with_rect(
        style: BoxStyle,
        decoration: Option<Decoration>,
        rect: Rect,
    ) -> TreeNode {
        node_with_rect("test", style, decoration, NodeKind::Container, rect)
    }

    fn leaf_with_rect(id: &'static str, leaf: LeafKind, style: BoxStyle, rect: Rect) -> TreeNode {
        node_with_rect(id, style, None, NodeKind::Leaf(leaf), rect)
    }

    /// 默认 decoration（background 让节点可命中）
    fn decor() -> Decoration {
        Decoration {
            background: Some(Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            }),
            border: None,
            radius: [0.0; 4],
            shadow: None,
        }
    }

    #[test]
    fn hit_empty_tree() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty(), "没 decoration 没 gestures 应该不可命中");
    }

    #[test]
    fn hit_root_only() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.leaf(), Some(root));
        assert_eq!(chain.root(), Some(root));
    }

    #[test]
    fn hit_leaf_nested() {
        let mut tree = Tree::new();
        let leaf_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 20.0,
                y: 20.0,
                w: 30.0,
                h: 30.0,
            },
        ));
        let middle = {
            let mut n = container_with_rect(
                BoxStyle::default(),
                Some(decor()),
                Rect {
                    x: 10.0,
                    y: 10.0,
                    w: 80.0,
                    h: 80.0,
                },
            );
            n.children = vec![leaf_id];
            n
        };
        let middle_id = tree.insert(middle);
        let root = {
            let mut n = container_with_rect(
                BoxStyle::default(),
                Some(decor()),
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 100.0,
                    h: 100.0,
                },
            );
            n.children = vec![middle_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 30.0, 30.0);
        assert_eq!(chain.len(), 3);
        let ids: Vec<_> = chain.iter().collect();
        assert_eq!(ids, vec![leaf_id, middle_id, root_id]);
    }

    #[test]
    fn hit_reverse_z_order() {
        let mut tree = Tree::new();
        let child1_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 50.0,
                h: 50.0,
            },
        ));
        let child2_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 50.0,
                h: 50.0,
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
            n.children = vec![child1_id, child2_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 30.0, 30.0);
        assert_eq!(chain.leaf(), Some(child2_id), "后添加的 child2 应该先命中");
    }

    #[test]
    fn hit_prefers_higher_z_index_over_later_source_order() {
        let mut tree = Tree::new();
        let lower_id = tree.insert(container_with_rect(
            BoxStyle {
                z_index: 0,
                ..Default::default()
            },
            Some(decor()),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 50.0,
                h: 50.0,
            },
        ));
        let higher_id = tree.insert(container_with_rect(
            BoxStyle {
                z_index: 10,
                ..Default::default()
            },
            Some(decor()),
            Rect {
                x: 10.0,
                y: 10.0,
                w: 50.0,
                h: 50.0,
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
            n.children = vec![higher_id, lower_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 30.0, 30.0);

        assert_eq!(chain.leaf(), Some(higher_id));
    }

    #[test]
    fn hit_hittable_force_false() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                hittable: Some(false),
                ..Default::default()
            },
            Some(decor()),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty(), "hittable=Some(false) 应强制不命中");
    }

    #[test]
    fn hit_hittable_force_true() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                hittable: Some(true),
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain.leaf(), Some(root));
    }

    #[test]
    fn hit_gestures_makes_hittable() {
        use crate::gesture::Gesture;
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                gestures: vec![Gesture::Tap],
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.leaf(), Some(root), "有 gestures 应该可命中");
    }

    #[test]
    fn hit_interaction_capabilities_make_hittable() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                draggable: true,
                resizable: true,
                ..Default::default()
            },
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert_eq!(chain.leaf(), Some(root), "交互能力应该可命中");
    }

    #[test]
    fn hit_neither_gestures_nor_decoration() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            None,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 50.0, 50.0);
        assert!(chain.is_empty());
    }

    #[test]
    fn hit_outside_bounds() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 200.0, 200.0);
        assert!(chain.is_empty());
    }

    #[test]
    fn hit_visible_overflow_child_outside_parent_bounds() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 120.0,
                y: 10.0,
                w: 20.0,
                h: 20.0,
            },
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    overflow: Overflow::Visible,
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

        let chain = hit_test(&tree, root_id, 125.0, 15.0);
        let ids: Vec<_> = chain.iter().collect();
        assert_eq!(ids, vec![child_id, root_id]);
    }

    #[test]
    fn hit_hidden_overflow_clips_child_outside_parent_bounds() {
        let chain = hit_overflow_child_outside_parent_bounds(Overflow::Hidden);

        assert!(chain.is_empty());
    }

    #[test]
    fn hit_scroll_overflow_clips_child_outside_parent_bounds() {
        let chain = hit_overflow_child_outside_parent_bounds(Overflow::Scroll);

        assert!(chain.is_empty());
    }

    #[test]
    fn hit_visible_overflow_does_not_hit_parent_outside_its_bounds() {
        let mut tree = Tree::new();
        let root = tree.insert(container_with_rect(
            BoxStyle {
                overflow: Overflow::Visible,
                ..Default::default()
            },
            Some(decor()),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 100.0,
            },
        ));
        tree.set_root(root);

        let chain = hit_test(&tree, root, 125.0, 15.0);
        assert!(chain.is_empty());
    }

    #[test]
    fn hit_visible_overflow_keeps_reverse_z_order() {
        let mut tree = Tree::new();
        let child1_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 120.0,
                y: 10.0,
                w: 20.0,
                h: 20.0,
            },
        ));
        let child2_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 120.0,
                y: 10.0,
                w: 20.0,
                h: 20.0,
            },
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    overflow: Overflow::Visible,
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
            n.children = vec![child1_id, child2_id];
            n
        };
        let root_id = tree.insert(root);
        tree.set_root(root_id);

        let chain = hit_test(&tree, root_id, 125.0, 15.0);
        assert_eq!(chain.leaf(), Some(child2_id));
    }

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
                hittable: Some(true),
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
                hittable: Some(true),
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
                hittable: Some(true),
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
                hittable: Some(true),
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
                hittable: Some(true),
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
                hittable: Some(true),
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

    #[test]
    fn hit_transform_identity() {
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
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
            BoxStyle::default(),
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
            BoxStyle::default(),
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
            BoxStyle::default(),
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
            BoxStyle::default(),
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
            BoxStyle::default(),
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
            BoxStyle::default(),
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

    fn hit_overflow_child_outside_parent_bounds(overflow: Overflow) -> HitChain {
        let mut tree = Tree::new();
        let child_id = tree.insert(container_with_rect(
            BoxStyle::default(),
            Some(decor()),
            Rect {
                x: 120.0,
                y: 10.0,
                w: 20.0,
                h: 20.0,
            },
        ));
        let root = {
            let mut n = container_with_rect(
                BoxStyle {
                    overflow,
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

        hit_test(&tree, root_id, 125.0, 15.0)
    }
}
