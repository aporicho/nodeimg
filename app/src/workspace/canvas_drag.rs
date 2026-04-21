use gui::canvas::camera::Camera;
use gui::canvas::canvas_node_owner_id;
use gui::context::Context;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasNodeDragSession {
    owner_id: String,
    last_canvas_x: f32,
    last_canvas_y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CanvasNodeDragController {
    active: Option<CanvasNodeDragSession>,
}

impl CanvasNodeDragController {
    pub(crate) fn start(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_owner_id(stable_id) else {
            return false;
        };
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        self.active = Some(CanvasNodeDragSession {
            owner_id: owner_id.to_string(),
            last_canvas_x: canvas_x,
            last_canvas_y: canvas_y,
        });
        gui.move_canvas_node_by(owner_id, 0.0, 0.0)
    }

    pub(crate) fn drag(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_owner_id(stable_id) else {
            return false;
        };
        let Some(active) = self.active.as_mut() else {
            return self.start(gui, camera, stable_id, x, y);
        };
        if active.owner_id != owner_id {
            return false;
        }

        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        let dx = canvas_x - active.last_canvas_x;
        let dy = canvas_y - active.last_canvas_y;
        active.last_canvas_x = canvas_x;
        active.last_canvas_y = canvas_y;
        gui.move_canvas_node_by(owner_id, dx, dy)
    }

    pub(crate) fn end(
        &mut self,
        gui: &mut Context,
        camera: &Camera,
        stable_id: &str,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(owner_id) = canvas_node_owner_id(stable_id) else {
            return false;
        };
        let Some(active) = self.active.as_mut() else {
            return false;
        };
        if active.owner_id != owner_id {
            return false;
        }
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        let dx = canvas_x - active.last_canvas_x;
        let dy = canvas_y - active.last_canvas_y;
        let moved = gui.move_canvas_node_by(owner_id, dx, dy);
        self.active = None;
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gui::canvas::CanvasNodeIdentity;
    use gui::renderer::Rect;

    #[test]
    fn drag_moves_canvas_node_in_canvas_space() {
        let mut gui = Context::new();
        gui.sync_canvas_node_layouts(&[CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 220.0,
                h: 96.0,
            },
        }]);
        let mut camera = Camera::new();
        camera.zoom = 2.0;
        let mut drag = CanvasNodeDragController::default();

        assert!(drag.start(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1",
            100.0,
            100.0
        ));
        assert!(drag.drag(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1",
            120.0,
            140.0
        ));
        assert!(drag.end(
            &mut gui,
            &camera,
            "canvas_node::engine_node::1",
            140.0,
            160.0
        ));

        let layout = gui.export_canvas_node_layouts().remove(0);
        assert_eq!(layout.rect.x, 30.0);
        assert_eq!(layout.rect.y, 50.0);
    }

    #[test]
    fn drag_ignores_non_canvas_node_ids() {
        let mut gui = Context::new();
        let camera = Camera::new();
        let mut drag = CanvasNodeDragController::default();

        assert!(!drag.start(&mut gui, &camera, "slider", 0.0, 0.0));
        assert!(!drag.drag(&mut gui, &camera, "slider", 10.0, 10.0));
        assert!(!drag.end(&mut gui, &camera, "slider", 10.0, 10.0));
    }
}
