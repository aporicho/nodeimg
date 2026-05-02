use crate::animation::{visual_affine, AnimationStore};
use crate::geometry::Point;
use crate::tree::paint_space::PaintSpace;
use crate::tree::{NodeId, Tree};

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
