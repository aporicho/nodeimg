use gui::canvas::camera::Camera;
use gui::canvas::canvas_node_event_owner_id;
use gui::context::Context;
use gui::geometry::ResizeEdge;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasNodeResizeSession {
    owner_id: String,
    edge: ResizeEdge,
    last_canvas_x: f32,
    last_canvas_y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CanvasNodeResizeController {
    active: Option<CanvasNodeResizeSession>,
}

impl CanvasNodeResizeController {
    pub(crate) fn start(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_event_owner_id(stable_id) else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                edge = ?edge,
                x,
                y,
                "ignore canvas node resize start: owner id not found"
            );
            return false;
        };
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        self.active = Some(CanvasNodeResizeSession {
            owner_id: owner_id.to_string(),
            edge,
            last_canvas_x: canvas_x,
            last_canvas_y: canvas_y,
        });
        let resized = gui.canvas_mut().resize_node_by(owner_id, edge, 0.0, 0.0);
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            stable_id,
            owner_id,
            edge = ?edge,
            screen_x = x,
            screen_y = y,
            canvas_x,
            canvas_y,
            resized,
            "start canvas node resize"
        );
        resized
    }

    pub(crate) fn resize(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_event_owner_id(stable_id) else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                edge = ?edge,
                x,
                y,
                "ignore canvas node resize move: owner id not found"
            );
            return false;
        };
        let Some(active) = self.active.as_mut() else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                owner_id,
                edge = ?edge,
                x,
                y,
                "resize move without active session; starting session"
            );
            return self.start(gui, camera, stable_id, edge, x, y);
        };
        if active.owner_id != owner_id || active.edge != edge {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                owner_id,
                active_owner_id = %active.owner_id,
                edge = ?edge,
                active_edge = ?active.edge,
                "ignore canvas node resize move: active session mismatch"
            );
            return false;
        }

        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        let prev_canvas_x = active.last_canvas_x;
        let prev_canvas_y = active.last_canvas_y;
        let dx = canvas_x - active.last_canvas_x;
        let dy = canvas_y - active.last_canvas_y;
        active.last_canvas_x = canvas_x;
        active.last_canvas_y = canvas_y;
        let resized = gui.canvas_mut().resize_node_by(owner_id, edge, dx, dy);
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            stable_id,
            owner_id,
            edge = ?edge,
            screen_x = x,
            screen_y = y,
            canvas_x,
            canvas_y,
            prev_canvas_x,
            prev_canvas_y,
            dx,
            dy,
            resized,
            "move canvas node resize"
        );
        resized
    }

    pub(crate) fn end(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        edge: ResizeEdge,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_event_owner_id(stable_id) else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                edge = ?edge,
                x,
                y,
                "ignore canvas node resize end: owner id not found"
            );
            return false;
        };
        let Some(active) = self.active.as_mut() else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                owner_id,
                edge = ?edge,
                x,
                y,
                "ignore canvas node resize end: no active session"
            );
            return false;
        };
        if active.owner_id != owner_id || active.edge != edge {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                stable_id,
                owner_id,
                active_owner_id = %active.owner_id,
                edge = ?edge,
                active_edge = ?active.edge,
                "ignore canvas node resize end: active session mismatch"
            );
            return false;
        }

        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        let prev_canvas_x = active.last_canvas_x;
        let prev_canvas_y = active.last_canvas_y;
        let dx = canvas_x - prev_canvas_x;
        let dy = canvas_y - prev_canvas_y;
        let resized = gui.canvas_mut().resize_node_by(owner_id, edge, dx, dy);
        self.active = None;
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            stable_id,
            owner_id,
            edge = ?edge,
            screen_x = x,
            screen_y = y,
            canvas_x,
            canvas_y,
            prev_canvas_x,
            prev_canvas_y,
            dx,
            dy,
            resized,
            "end canvas node resize"
        );
        resized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gui::canvas::CanvasNodeIdentity;
    use gui::renderer::Rect;

    #[test]
    fn resize_changes_canvas_node_in_canvas_space() {
        let mut gui = Context::new();
        gui.canvas_mut().sync_node_layouts(&[CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 304.0,
                h: 120.0,
            },
        }]);
        let mut camera = Camera::new();
        camera.zoom = 2.0;
        let mut resize = CanvasNodeResizeController::default();

        assert!(resize.start(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            100.0,
            100.0,
        ));
        assert!(resize.resize(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            140.0,
            160.0,
        ));
        assert!(resize.end(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            160.0,
            180.0,
        ));

        let layout = gui.canvas().export_node_layouts().remove(0);
        assert_eq!(layout.rect.w, 334.0);
        assert_eq!(layout.rect.h, 172.0);
    }

    #[test]
    fn resize_can_shrink_canvas_node_until_min_size() {
        let mut gui = Context::new();
        gui.canvas_mut().sync_node_layouts(&[CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 420.0,
                h: 220.0,
            },
        }]);
        let camera = Camera::new();
        let mut resize = CanvasNodeResizeController::default();

        assert!(resize.start(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            200.0,
            200.0,
        ));
        assert!(resize.resize(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            160.0,
            150.0,
        ));
        let layout = gui.canvas().export_node_layouts().remove(0);
        assert_eq!(layout.rect.w, 380.0);
        assert_eq!(layout.rect.h, 170.0);

        assert!(resize.resize(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1::card",
            ResizeEdge::BottomRight,
            -200.0,
            -100.0,
        ));
        let layout = gui.canvas().export_node_layouts().remove(0);
        assert_eq!(layout.rect.w, 304.0);
        assert_eq!(layout.rect.h, 132.0);
    }

    #[test]
    fn resize_ignores_non_canvas_node_ids() {
        let mut gui = Context::new();
        let camera = Camera::new();
        let mut resize = CanvasNodeResizeController::default();

        assert!(!resize.start(&mut gui, &camera, "slider", ResizeEdge::Right, 0.0, 0.0,));
        assert!(!resize.resize(&mut gui, &camera, "slider", ResizeEdge::Right, 10.0, 10.0,));
        assert!(!resize.end(&mut gui, &camera, "slider", ResizeEdge::Right, 10.0, 10.0,));
    }
}
