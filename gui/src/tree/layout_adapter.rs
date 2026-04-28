use super::layout::{BoxStyle, LayoutTree, LeafKind};
use super::node::{NodeId, NodeKind};
use super::tree::Tree;
use super::Revision;
use crate::renderer::Rect;

impl LayoutTree for Tree {
    type NodeId = NodeId;

    fn style(&self, node: NodeId) -> &BoxStyle {
        &self.get(node).unwrap().style
    }

    fn explicit_rect(&self, node: NodeId) -> Option<Rect> {
        self.get(node).and_then(|node| {
            (node.layout_meta.explicit_rect_revision != Revision::ZERO).then_some(node.rect)
        })
    }

    fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.get(node)
            .map(|n| n.children.clone())
            .unwrap_or_default()
    }

    fn set_rect(&mut self, node: NodeId, rect: Rect) {
        self.record_layout_node_visited();
        if let Some(n) = self.get_mut(node) {
            n.rect = rect;
        }
    }

    fn scroll_offset(&self, node: NodeId) -> f32 {
        self.get(node).map(|n| n.scroll_offset()).unwrap_or(0.0)
    }

    fn set_content_height(&mut self, node: NodeId, height: f32) {
        if let Some(n) = self.get_mut(node) {
            n.set_content_height(height);
        }
    }

    fn text_content(&self, node: NodeId) -> Option<(&str, &crate::renderer::TextStyle)> {
        match &self.get(node)?.kind {
            NodeKind::Leaf(LeafKind::Text { content, style, .. }) => Some((content, style)),
            _ => None,
        }
    }
}
