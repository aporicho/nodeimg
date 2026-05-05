use crate::geometry::Point;

use super::node::NodeId;
use super::paint_helpers::rect_center;
use super::paint_space::PaintSpace;
use super::stacking::children_in_paint_order;
use super::Tree;

pub(crate) fn node_screen_center(tree: &Tree, id: &str) -> Option<Point> {
    let target = tree.node_by_str(id)?;
    node_screen_center_recursive(tree, tree.root()?, target, PaintSpace::root())
}

fn node_screen_center_recursive(
    tree: &Tree,
    node_id: NodeId,
    target: NodeId,
    current_space: PaintSpace,
) -> Option<Point> {
    let node = tree.get(node_id)?;
    let node_space = current_space.node_space(node.rect, node.style.transform);

    if node_id == target {
        return Some(
            node_space
                .local_to_screen
                .transform_point(rect_center(node_space.local_rect)),
        );
    }

    let child_space = if node_space.children_are_local {
        node_space.child_space()
    } else {
        current_space
    };
    for child_id in children_in_paint_order(tree, &node.children) {
        if let Some(point) = node_screen_center_recursive(tree, child_id, target, child_space) {
            return Some(point);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Rect, TransformSpec};
    use crate::tree::{TreeNode, TreeNodeBuilder};

    const EPS: f32 = 1e-5;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    fn node(id: &'static str, rect: Rect, children: Vec<NodeId>) -> TreeNode {
        let mut node = TreeNodeBuilder::container(id, Default::default())
            .rect(rect)
            .build();
        node.children = children;
        node
    }

    #[test]
    fn resolves_node_center_in_screen_space() {
        let mut tree = Tree::new();
        let child = tree.insert(node("target", rect(10.0, 20.0, 8.0, 12.0), vec![]));
        let root = tree.insert(node("root", rect(100.0, 200.0, 300.0, 400.0), vec![child]));
        tree.set_root(root);

        assert_eq!(
            node_screen_center(&tree, "target"),
            Some(Point { x: 14.0, y: 26.0 })
        );
    }

    #[test]
    fn resolves_node_center_under_affine_transform() {
        let mut tree = Tree::new();
        let child = tree.insert(node("target", rect(10.0, 0.0, 4.0, 4.0), vec![]));
        let mut parent = node("parent", rect(20.0, 30.0, 40.0, 40.0), vec![child]);
        parent.style.transform = Some(TransformSpec::translate_scale_rotate(
            [0.0, 0.0],
            2.0,
            std::f32::consts::FRAC_PI_2,
        ));
        let parent = tree.insert(parent);
        let root = tree.insert(node("root", rect(0.0, 0.0, 100.0, 100.0), vec![parent]));
        tree.set_root(root);

        let center = node_screen_center(&tree, "target").expect("target center");

        assert_point_near(center, Point { x: 16.0, y: 54.0 });
    }

    #[test]
    fn returns_none_for_missing_node() {
        let mut tree = Tree::new();
        let root = tree.insert(node("root", rect(0.0, 0.0, 100.0, 100.0), vec![]));
        tree.set_root(root);

        assert_eq!(node_screen_center(&tree, "missing"), None);
    }
}
