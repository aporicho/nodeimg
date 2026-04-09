use crate::widget::node::{NodeId, NodeKind};
use super::tree::PanelTree;
use crate::widget::layout::{self, BoxStyle, LeafKind, LayoutTree};
use crate::renderer::Rect;

impl LayoutTree for PanelTree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        &self.get(node).unwrap().style
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

    fn text_content(&self, node: NodeId) -> Option<(&str, f32)> {
        match &self.get(node)?.kind {
            NodeKind::Leaf(LeafKind::Text { content, font_size, .. }) => Some((content, *font_size)),
            _ => None,
        }
    }
}

pub fn layout(
    tree: &mut PanelTree,
    root: NodeId,
    available: Rect,
    measure_text: &mut dyn FnMut(&str, f32) -> (f32, f32),
) {
    layout::layout(tree, root, available, measure_text);
}

/// 更新滚动偏移。delta 为正数向下滚。
pub fn scroll(tree: &mut PanelTree, node_id: NodeId, delta: f32) {
    if let Some(node) = tree.get_mut(node_id) {
        let max = (node.content_height - node.rect.h).max(0.0);
        node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
    }
}
