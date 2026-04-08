use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::renderer::Rect;

const BUTTON_HEIGHT: f32 = 40.0;
const SPACING: f32 = 8.0;

/// 测量 + 排列。自顶向下分配位置，写入每个节点的 rect。
pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect) {
    layout_node(tree, root, available);
}

fn layout_node(tree: &mut PanelTree, node_id: NodeId, available: Rect) {
    let (kind_is_column, children) = {
        let Some(node) = tree.get(node_id) else { return };
        let is_column = matches!(node.kind, NodeKind::Column);
        (is_column, node.children.clone())
    };

    // 设置自身 rect
    if let Some(node) = tree.get_mut(node_id) {
        node.rect = available;
    }

    if kind_is_column {
        // Column：子节点纵向排列
        let mut y = available.y;
        for child_id in children {
            let child_height = measure_height(tree, child_id);
            let child_rect = Rect {
                x: available.x,
                y,
                w: available.w,
                h: child_height,
            };
            layout_node(tree, child_id, child_rect);
            y += child_height + SPACING;
        }
    }
}

fn measure_height(tree: &PanelTree, node_id: NodeId) -> f32 {
    let Some(node) = tree.get(node_id) else { return 0.0 };
    match &node.kind {
        NodeKind::Button { .. } => BUTTON_HEIGHT,
        NodeKind::Column => {
            let children = &node.children;
            if children.is_empty() {
                return 0.0;
            }
            let total: f32 = children.iter().map(|&id| measure_height(tree, id)).sum();
            total + SPACING * (children.len() as f32 - 1.0)
        }
    }
}
