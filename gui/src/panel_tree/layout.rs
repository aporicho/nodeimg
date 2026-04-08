use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::controls::atoms::button::{ButtonProps, button_layout};
use crate::controls::infra::layout::{self, BoxStyle, Direction, LayoutBox};
use crate::renderer::{Rect, Renderer};

const SPACING: f32 = 8.0;

pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect, renderer: &mut Renderer) {
    let layout_tree = build_layout_box(tree, root, renderer);
    let result = layout::layout(&layout_tree, available);

    for (id, rect) in &result.rects {
        if let Some(node_id) = find_node_by_id(tree, root, id) {
            if let Some(node) = tree.get_mut(node_id) {
                node.rect = *rect;
            }
        }
    }

    if let Some(node) = tree.get_mut(root) {
        node.rect = available;
    }
}

fn build_layout_box(tree: &PanelTree, node_id: NodeId, renderer: &mut Renderer) -> LayoutBox {
    let Some(node) = tree.get(node_id) else {
        return LayoutBox {
            id: None,
            style: BoxStyle::default(),
            children: Vec::new(),
        };
    };

    match &node.kind {
        NodeKind::Column => {
            let children_ids = node.children.clone();
            LayoutBox {
                id: None,
                style: BoxStyle {
                    direction: Direction::Column,
                    gap: SPACING,
                    ..BoxStyle::default()
                },
                children: children_ids
                    .iter()
                    .map(|&id| build_layout_box(tree, id, renderer))
                    .collect(),
            }
        }
        NodeKind::Button { label, color } => {
            let props = ButtonProps {
                id: node.id,
                label,
                color: *color,
            };
            button_layout(&props, renderer)
        }
    }
}

fn find_node_by_id(tree: &PanelTree, root: NodeId, target_id: &str) -> Option<NodeId> {
    let Some(node) = tree.get(root) else { return None };
    if node.id == target_id {
        return Some(root);
    }
    for &child_id in &node.children {
        if let Some(found) = find_node_by_id(tree, child_id, target_id) {
            return Some(found);
        }
    }
    None
}
