use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::controls::infra::layout::{self, BoxStyle, Direction, LayoutBox, Size};
use crate::renderer::Rect;

const BUTTON_HEIGHT: f32 = 40.0;
const SPACING: f32 = 8.0;

/// 用新布局引擎计算面板树中每个节点的 rect。
pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect) {
    let layout_tree = build_layout_box(tree, root);
    let result = layout::layout(&layout_tree, available);

    // 写回 rect
    for (id, rect) in &result.rects {
        if let Some(node_id) = find_node_by_id(tree, root, id) {
            if let Some(node) = tree.get_mut(node_id) {
                node.rect = *rect;
            }
        }
    }

    // 容器节点（Column）用 available rect
    if let Some(node) = tree.get_mut(root) {
        node.rect = available;
    }
}

/// PanelNode → LayoutBox
fn build_layout_box(tree: &PanelTree, node_id: NodeId) -> LayoutBox {
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
                children: children_ids.iter().map(|&id| build_layout_box(tree, id)).collect(),
            }
        }
        NodeKind::Button { .. } => {
            LayoutBox {
                id: Some(node.id),
                style: BoxStyle {
                    height: Size::Fixed(BUTTON_HEIGHT),
                    ..BoxStyle::default()
                },
                children: Vec::new(),
            }
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
