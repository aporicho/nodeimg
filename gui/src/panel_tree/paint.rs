use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::renderer::Renderer;

pub fn paint(tree: &PanelTree, root: NodeId, renderer: &mut Renderer) {
    paint_node(tree, root, renderer);
}

fn paint_node(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) {
    let Some(node) = tree.get(node_id) else { return };

    match &node.kind {
        NodeKind::Container { .. } => {}
        NodeKind::Widget(w) => {
            w.paint(node.rect, renderer);
        }
    }

    let children: Vec<NodeId> = tree
        .get(node_id)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    for child_id in children {
        paint_node(tree, child_id, renderer);
    }
}

pub fn hit_test(tree: &PanelTree, root: NodeId, x: f32, y: f32) -> Option<&'static str> {
    hit_test_node(tree, root, x, y)
}

fn hit_test_node(tree: &PanelTree, node_id: NodeId, x: f32, y: f32) -> Option<&'static str> {
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

    match &node.kind {
        NodeKind::Widget(w) => {
            if w.hit_test(node.rect, x, y) {
                return Some(node.id);
            }
        }
        _ => {}
    }

    None
}
