use super::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::{self, BoxStyle, LayoutTree};
use crate::renderer::Rect;

impl LayoutTree for PanelTree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        match &self.get(node).unwrap().kind {
            NodeKind::Container { style, .. } | NodeKind::Leaf { style, .. } => style,
        }
    }

    fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.get(node).map(|n| n.children.clone()).unwrap_or_default()
    }

    fn set_rect(&mut self, node: NodeId, rect: Rect) {
        if let Some(n) = self.get_mut(node) {
            n.rect = rect;
        }
    }

    fn scroll_offset(&self, node: NodeId) -> f32 {
        self.get(node).map(|n| n.scroll_offset).unwrap_or(0.0)
    }

    fn set_content_height(&mut self, node: NodeId, height: f32) {
        if let Some(n) = self.get_mut(node) {
            n.content_height = height;
        }
    }
}

pub fn layout(tree: &mut PanelTree, root: NodeId, available: Rect) {
    layout::layout(tree, root, available);
}

/// 更新滚动偏移。delta 为正数向下滚。
pub fn scroll(tree: &mut PanelTree, node_id: NodeId, delta: f32) {
    if let Some(node) = tree.get_mut(node_id) {
        let max = (node.content_height - node.rect.h).max(0.0);
        node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
    }
}
