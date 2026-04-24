use super::ids;
use super::overlay::build_developer_popup;
use super::state::DeveloperModeState;
use gui::canvas::camera::Camera;
use gui::context::{Context, OverlayPlacement, OverlayRequest};

#[derive(Debug, Default)]
pub(crate) struct DeveloperModeController {
    state: DeveloperModeState,
}

impl DeveloperModeController {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn state(&self) -> &DeveloperModeState {
        &self.state
    }

    pub(crate) fn is_playground_drag_target(&self, id: &str) -> bool {
        ids::is_tile_drag_target(id)
    }

    pub(crate) fn is_playground_resize_target(&self, id: &str) -> bool {
        ids::is_tile_resize_target(id)
    }

    pub(crate) fn is_control_drag_target(&self, id: &str) -> bool {
        ids::is_slider_target(id)
    }

    pub(crate) fn handle_click(
        &mut self,
        id: &str,
        pointer_x: f32,
        camera: &Camera,
        gui: &mut Context,
    ) {
        if id == ids::CONTROL_POPUP_CLOSE_ID {
            gui.close_overlay();
            return;
        }

        if id == ids::CONTROL_POPUP_BUTTON_ID {
            self.toggle_popup(gui);
            return;
        }

        if ids::is_slider_target(id) {
            self.update_slider_from_pointer(pointer_x, camera, gui);
            return;
        }

        let _ = self.state.apply_click(id);
    }

    pub(crate) fn handle_double_click(&mut self, id: &str) {
        if ids::is_slider_target(id) {
            self.state.reset_slider();
        }
    }

    pub(crate) fn handle_text_change(&mut self, id: &str, value: String) {
        let _ = self.state.apply_text_change(id, value);
    }

    pub(crate) fn handle_number_change(&mut self, id: &str, value: f32) {
        let _ = self.state.apply_number_change(id, value);
    }

    pub(crate) fn handle_selection_change(&mut self, id: &str, selected: usize) {
        let _ = self.state.apply_selection_change(id, selected);
    }

    pub(crate) fn handle_drag_start(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        camera: &Camera,
        gui: &Context,
    ) {
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        if ids::is_tile_resize_target(id) {
            let _ = self.state.start_resize(id, canvas_x, canvas_y);
            return;
        }
        if ids::is_slider_target(id) {
            self.update_slider_from_pointer(x, camera, gui);
            return;
        }
        let _ = self.state.start_drag(id, canvas_x, canvas_y);
    }

    pub(crate) fn handle_drag_move(
        &mut self,
        id: &str,
        x: f32,
        y: f32,
        camera: &Camera,
        gui: &Context,
    ) {
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        if ids::is_tile_resize_target(id) {
            let _ = self.state.resize(id, canvas_x, canvas_y);
            return;
        }
        if ids::is_slider_target(id) {
            self.update_slider_from_pointer(x, camera, gui);
            return;
        }
        let _ = self.state.drag(id, canvas_x, canvas_y);
    }

    pub(crate) fn handle_drag_end(&mut self, id: &str, x: f32, y: f32, camera: &Camera) {
        let (canvas_x, canvas_y) = camera.screen_to_canvas(x, y);
        if ids::is_tile_resize_target(id) {
            let _ = self.state.end_resize(id, canvas_x, canvas_y);
            return;
        }
        let _ = self.state.end_drag(id, canvas_x, canvas_y);
    }

    fn toggle_popup(&mut self, gui: &mut Context) {
        if gui.overlay_open() {
            gui.close_overlay();
            return;
        }

        gui.open_overlay(OverlayRequest {
            id: ids::CONTROL_POPUP_OVERLAY_ID.to_string(),
            anchor_id: ids::CONTROL_POPUP_BUTTON_ID.to_string(),
            restore_focus_id: Some(ids::CONTROL_POPUP_BUTTON_ID.to_string()),
            placement: OverlayPlacement::BelowStart,
            content: build_developer_popup(self.state.controls()),
            offset_x: 0.0,
            offset_y: 8.0,
            match_anchor_width: false,
            dismiss_on_escape: true,
            dismiss_on_outside_click: true,
            restore_focus_to_anchor: true,
        });
    }

    fn update_slider_from_pointer(&mut self, x: f32, camera: &Camera, gui: &Context) {
        if let Some(track_rect) = gui.node_rect(ids::CONTROL_SLIDER_TRACK_ID) {
            let (canvas_x, _) = camera.screen_to_canvas(x, 0.0);
            self.state.update_slider_from_pointer(track_rect, canvas_x);
        }
    }
}
