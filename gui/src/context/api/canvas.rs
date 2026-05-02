use super::super::Context;
use crate::control::ResizeEdge;

pub struct CanvasApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl CanvasApi<'_> {
    pub fn export_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.export_canvas_node_layouts()
    }

    pub fn port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.ctx.canvas_port_group_view(owner_id, side)
    }

    pub fn is_node_selected(&self, owner_id: &str) -> bool {
        self.ctx.is_canvas_node_selected(owner_id)
    }

    pub fn pending_connection(&self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ctx.pending_canvas_connection()
    }

    pub fn hovered_port_id(&self) -> Option<String> {
        self.ctx.hovered_canvas_port_id()
    }
}

pub struct CanvasMutApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl CanvasMutApi<'_> {
    pub fn sync_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.sync_canvas_node_layouts(identities)
    }

    pub fn export_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.ctx.export_canvas_node_layouts()
    }

    pub fn import_node_layouts(&mut self, layouts: &[crate::canvas::CanvasNodeLayout]) {
        self.ctx.import_canvas_node_layouts(layouts);
    }

    pub fn move_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.ctx.move_canvas_node_by(owner_id, dx, dy)
    }

    pub fn resize_node_by(&mut self, owner_id: &str, edge: ResizeEdge, dx: f32, dy: f32) -> bool {
        self.ctx.resize_canvas_node_by(owner_id, edge, dx, dy)
    }

    pub fn ensure_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        self.ctx
            .ensure_canvas_node_min_size(owner_id, min_width, min_height)
    }

    pub fn apply_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        self.ctx.apply_canvas_node_sizing(owner_id, request)
    }

    pub fn toggle_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.ctx.toggle_canvas_port_group(owner_id, side)
    }

    pub fn select_node(&mut self, owner_id: &str) -> bool {
        self.ctx.select_canvas_node(owner_id)
    }

    pub fn clear_selection(&mut self) -> bool {
        self.ctx.clear_canvas_selection()
    }

    pub fn begin_pending_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.ctx
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub fn update_pending_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.ctx.update_pending_canvas_connection(cursor_canvas)
    }

    pub fn end_pending_connection(&mut self) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ctx.end_pending_canvas_connection()
    }

    pub fn cancel_pending_connection(&mut self) -> bool {
        self.ctx.cancel_pending_canvas_connection()
    }

    pub fn set_hovered_port(&mut self, port_id: Option<&str>) -> bool {
        self.ctx.set_hovered_canvas_port(port_id)
    }
}
