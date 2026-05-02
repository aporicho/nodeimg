use super::Context;
use crate::control::ResizeEdge;

impl Context {
    pub(crate) fn sync_canvas_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.sync_canvas_node_layouts(identities)
    }

    pub(crate) fn export_canvas_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        self.tree.export_canvas_node_layouts()
    }

    pub(crate) fn import_canvas_node_layouts(
        &mut self,
        layouts: &[crate::canvas::CanvasNodeLayout],
    ) {
        self.tree.import_canvas_node_layouts(layouts);
    }

    pub(crate) fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        self.tree.move_canvas_node_by(owner_id, dx, dy)
    }

    pub(crate) fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        self.tree.resize_canvas_node_by(owner_id, edge, dx, dy)
    }

    pub(crate) fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        self.tree
            .ensure_canvas_node_min_size(owner_id, min_width, min_height)
    }

    pub(crate) fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        self.tree.apply_canvas_node_sizing(owner_id, request)
    }

    pub(crate) fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        self.tree.canvas_port_group_view(owner_id, side)
    }

    pub(crate) fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        self.tree.toggle_canvas_port_group(owner_id, side)
    }

    pub(crate) fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        self.tree.select_canvas_node(owner_id)
    }

    pub(crate) fn clear_canvas_selection(&mut self) -> bool {
        self.tree.clear_canvas_selection()
    }

    pub(crate) fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.tree.is_canvas_node_selected(owner_id)
    }

    pub(crate) fn pending_canvas_connection(
        &self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.pending_canvas_connection()
    }

    pub(crate) fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.tree
            .begin_pending_canvas_connection(from_port_id, cursor_canvas)
    }

    pub(crate) fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        self.tree.update_pending_canvas_connection(cursor_canvas)
    }

    pub(crate) fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.tree.end_pending_canvas_connection()
    }

    pub(crate) fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.tree.cancel_pending_canvas_connection()
    }

    pub(crate) fn hovered_canvas_port_id(&self) -> Option<String> {
        self.tree.hovered_canvas_port_id()
    }

    pub(crate) fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        self.tree.set_hovered_canvas_port(port_id)
    }
}
