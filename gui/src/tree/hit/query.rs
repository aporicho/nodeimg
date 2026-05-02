use super::chain::HitChain;
use super::leaf_shape::leaf_shape_hit;
use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::Point;
use crate::tree::layout::{BoxStyle, Overflow};
use crate::tree::node::{NodeId, NodeKind, TreeNode};
use crate::tree::paint_space::{NodePaintSpace, PaintSpace};
use crate::tree::Tree;

/// 公开入口：返回命中链。从 root 开始向下递归，返回的链从叶子到根。
/// 如果没命中返回 empty HitChain。
#[cfg(test)]
pub(crate) fn hit_test(tree: &Tree, root: NodeId, x: f32, y: f32) -> HitChain {
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
    if self_hit && is_hittable(&style) {
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
        NodeKind::Container => inside_bounds,
    }
}

fn is_hittable(style: &BoxStyle) -> bool {
    style.hittable || style.draggable || style.resizable || !style.gestures.is_empty()
}
