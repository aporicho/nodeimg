use super::Context;
use crate::control::ResizeEdge;
use crate::cursor::{resolve_cursor, CursorKind};
use crate::input::{PointerHitRequest, PointerHitResolver, PointerHitSnapshot};
use crate::renderer::Rect;
use crate::tree::{
    hit_test_with_animations, resize_hit_at_screen_point, HitChain, NodeId, TargetChain,
};

impl Context {
    pub(crate) fn hit_test(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(&self.tree, root, x, y, Some(&self.animations))
    }

    pub(crate) fn root(&self) -> Option<NodeId> {
        self.tree.root()
    }

    pub(crate) fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree.node_by_str(id)
    }

    pub(crate) fn node_exists(&self, id: &str) -> bool {
        self.node_id_by_name(id).is_some()
    }

    pub(crate) fn node_rect(&self, id: &str) -> Option<Rect> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.node_rect_by_node(node_id))
    }

    pub(crate) fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.tree.get(node_id).map(|node| node.rect)
    }

    pub(crate) fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub(crate) fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.node_id_by_name(id)
            .and_then(|node_id| self.tree.get(node_id))
            .map(|node| node.scroll_offset())
    }

    pub(crate) fn resize_hit_at_screen_point(
        &self,
        x: f32,
        y: f32,
    ) -> Option<(NodeId, ResizeEdge)> {
        let Some(root) = self.tree.root() else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                x,
                y,
                "query resize edge at screen point: root missing"
            );
            return None;
        };
        let hit = resize_hit_at_screen_point(&self.tree, root, x, y, Some(&self.animations));
        let root_node = self.tree.get(root).map(|node| node.id.to_string());
        let hit_node =
            hit.and_then(|hit| self.tree.get(hit.node_id).map(|node| node.id.to_string()));
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            x,
            y,
            root_node = root_node.as_deref(),
            hit_node = hit_node.as_deref(),
            edge = ?hit.map(|hit| hit.edge),
            "query resize edge at screen point"
        );
        hit.map(|hit| (hit.node_id, hit.edge))
    }

    pub(crate) fn pointer_hit_at(&self, x: f32, y: f32) -> PointerHitSnapshot {
        PointerHitResolver::new(&self.tree, Some(&self.animations))
            .snapshot_for_request(PointerHitRequest::new(x, y, true))
    }

    pub(crate) fn cursor_for_hit(&self, hit: &PointerHitSnapshot) -> CursorKind {
        let targets = TargetChain::from_hit_chain(&self.tree, hit.chain());
        resolve_cursor(hit.resize_hit().map(|(_, edge)| edge), &targets)
    }

    pub(crate) fn cursor_at(&self, x: f32, y: f32) -> CursorKind {
        let hit = self.pointer_hit_at(x, y);
        self.cursor_for_hit(&hit)
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn focused_control_id(&self) -> Option<&str> {
        self.node_name_for(self.focused_node())
    }

    pub(crate) fn hovered_node(&self) -> Option<NodeId> {
        self.interaction.hovered()
    }

    pub(crate) fn hovered_control_id(&self) -> Option<&str> {
        self.node_name_for(self.hovered_node())
    }

    pub(crate) fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub(crate) fn captured_control_id(&self) -> Option<&str> {
        self.node_name_for(self.captured_node())
    }

    fn node_name_for(&self, node_id: Option<NodeId>) -> Option<&str> {
        node_id.and_then(|id| self.node_name(id))
    }
}
