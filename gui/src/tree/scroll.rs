use super::node::NodeId;
use super::tree::Tree;

impl Tree {
    /// 更新滚动偏移。delta 为正数向下滚。
    pub fn scroll(&mut self, node_id: NodeId, delta: f32) {
        if let Some(node) = self.get_mut(node_id) {
            let max = (node.content_height - node.rect.h).max(0.0);
            node.scroll_offset = (node.scroll_offset + delta).clamp(0.0, max);
        }
    }
}
