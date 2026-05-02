use super::super::{Context, HitChain, PointerHitQueryResult};
use crate::control::ResizeEdge;
use crate::cursor::CursorKind;
use crate::renderer::Rect;
use crate::tree::NodeId;

pub struct QueryApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl QueryApi<'_> {
    pub fn root(&self) -> Option<NodeId> {
        self.ctx.root()
    }

    pub fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.ctx.node_id_by_name(id)
    }

    pub fn node_exists(&self, id: &str) -> bool {
        self.ctx.node_exists(id)
    }

    pub fn node_rect(&self, id: &str) -> Option<Rect> {
        self.ctx.node_rect(id)
    }

    pub fn node_rect_by_node(&self, node_id: NodeId) -> Option<Rect> {
        self.ctx.node_rect_by_node(node_id)
    }

    pub fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.ctx.node_name(node_id)
    }

    pub fn node_scroll_offset(&self, id: &str) -> Option<f32> {
        self.ctx.node_scroll_offset(id)
    }

    pub fn hit_test(&self, x: f32, y: f32) -> HitChain {
        self.ctx.hit_test(x, y)
    }

    pub fn resize_hit_at_screen_point(&self, x: f32, y: f32) -> Option<(NodeId, ResizeEdge)> {
        self.ctx.resize_hit_at_screen_point(x, y)
    }

    pub fn pointer_hit_at(&self, x: f32, y: f32) -> PointerHitQueryResult {
        self.ctx.pointer_hit_at(x, y)
    }

    pub fn cursor_for_hit(&self, hit: &PointerHitQueryResult) -> CursorKind {
        self.ctx.cursor_for_hit(hit)
    }

    pub fn cursor_at(&self, x: f32, y: f32) -> CursorKind {
        self.ctx.cursor_at(x, y)
    }

    pub fn focused_node(&self) -> Option<NodeId> {
        self.ctx.focused_node()
    }

    pub fn focused_control_id(&self) -> Option<&str> {
        self.ctx.focused_control_id()
    }

    pub fn hovered_node(&self) -> Option<NodeId> {
        self.ctx.hovered_node()
    }

    pub fn hovered_control_id(&self) -> Option<&str> {
        self.ctx.hovered_control_id()
    }

    pub fn captured_node(&self) -> Option<NodeId> {
        self.ctx.captured_node()
    }

    pub fn captured_control_id(&self) -> Option<&str> {
        self.ctx.captured_control_id()
    }
}
