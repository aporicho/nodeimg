use super::node::NodeId;
use super::tree::Tree;

pub fn hit_test<'a>(tree: &'a Tree, root: NodeId, x: f32, y: f32) -> Option<&'a str> {
    hit_test_node(tree, root, x, y)
}

fn hit_test_node<'a>(tree: &'a Tree, node_id: NodeId, x: f32, y: f32) -> Option<&'a str> {
    let Some(node) = tree.get(node_id) else { return None };

    let r = &node.rect;
    if x < r.x || x > r.x + r.w || y < r.y || y > r.y + r.h {
        return None;
    }

    for &child_id in &node.children {
        if let Some(id) = hit_test_node(tree, child_id, x, y) {
            return Some(id);
        }
    }

    if node.decoration.is_some() {
        Some(node.id.as_ref())
    } else {
        None
    }
}
