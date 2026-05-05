use super::Context;
use crate::geometry::ResizeEdge;

impl Context {
    pub(crate) fn sync_canvas_node_layouts(
        &mut self,
        identities: &[crate::canvas::CanvasNodeIdentity],
    ) -> Vec<crate::canvas::CanvasNodeLayout> {
        crate::canvas::runtime::sync_node_layouts(&mut self.tree, identities)
    }

    pub(crate) fn export_canvas_node_layouts(&self) -> Vec<crate::canvas::CanvasNodeLayout> {
        crate::canvas::runtime::export_node_layouts(&self.tree)
    }

    pub(crate) fn import_canvas_node_layouts(
        &mut self,
        layouts: &[crate::canvas::CanvasNodeLayout],
    ) {
        crate::canvas::runtime::import_node_layouts(&mut self.tree, layouts);
    }

    pub(crate) fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        crate::canvas::runtime::move_node_by(&mut self.tree, owner_id, dx, dy)
    }

    pub(crate) fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        crate::canvas::runtime::resize_node_by(&mut self.tree, owner_id, edge, dx, dy)
    }

    pub(crate) fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        crate::canvas::runtime::ensure_node_min_size(
            &mut self.tree,
            owner_id,
            min_width,
            min_height,
        )
    }

    pub(crate) fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        crate::canvas::runtime::apply_node_sizing(&mut self.tree, owner_id, request)
    }

    pub(crate) fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> crate::canvas::CanvasPortGroupView {
        crate::canvas::runtime::port_group_view(&self.tree, owner_id, side)
    }

    pub(crate) fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: crate::canvas::CanvasPortSide,
    ) -> bool {
        crate::canvas::runtime::toggle_port_group(&mut self.tree, owner_id, side)
    }

    pub(crate) fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        crate::canvas::runtime::select_node(&mut self.tree, owner_id)
    }

    pub(crate) fn clear_canvas_selection(&mut self) -> bool {
        crate::canvas::runtime::clear_selection(&mut self.tree)
    }

    pub(crate) fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        crate::canvas::runtime::is_node_selected(&self.tree, owner_id)
    }

    pub(crate) fn pending_canvas_connection(
        &self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        crate::canvas::runtime::pending_connection(&self.tree)
    }

    pub(crate) fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        crate::canvas::runtime::begin_pending_connection(
            &mut self.tree,
            from_port_id,
            cursor_canvas,
        )
    }

    pub(crate) fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        crate::canvas::runtime::update_pending_connection(&mut self.tree, cursor_canvas)
    }

    pub(crate) fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        crate::canvas::runtime::end_pending_connection(&mut self.tree)
    }

    pub(crate) fn cancel_pending_canvas_connection(&mut self) -> bool {
        crate::canvas::runtime::cancel_pending_connection(&mut self.tree)
    }

    pub(crate) fn hovered_canvas_port_id(&self) -> Option<String> {
        crate::canvas::runtime::hovered_port_id(&self.tree)
    }

    pub(crate) fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        crate::canvas::runtime::set_hovered_port(&mut self.tree, port_id)
    }
}
