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
