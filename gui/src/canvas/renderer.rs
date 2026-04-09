use super::background;
use super::camera::Camera;
use super::pan::PanState;
use crate::renderer::Renderer;
use crate::shell::{AppEvent, MouseButton};

pub struct CanvasRenderer {
    camera: Camera,
    pan: PanState,
}

impl CanvasRenderer {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            pan: PanState::new(),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, viewport_w: f32, viewport_h: f32) {
        background::render(renderer, &self.camera, viewport_w, viewport_h);
    }

    pub fn event(&mut self, event: &AppEvent) -> bool {
        match event {
            AppEvent::MousePress { x, y, button } => {
                if *button == MouseButton::Middle {
                    self.pan.start(*x, *y);
                    return true;
                }
            }
            AppEvent::MouseMove { x, y } => {
                if self.pan.is_active() {
                    self.pan.update(*x, *y, &mut self.camera);
                    return true;
                }
            }
            AppEvent::MouseRelease { button, .. } => {
                if *button == MouseButton::Middle {
                    self.pan.end();
                    return true;
                }
            }
            AppEvent::ScrollLine { x, y, delta_y, .. } => {
                self.camera.zoom_at(*x, *y, *delta_y * 0.1);
                return true;
            }
            AppEvent::ScrollPixel { delta_x, delta_y, .. } => {
                self.camera.pan(*delta_x, *delta_y);
                return true;
            }
            AppEvent::PinchZoom { x, y, delta } => {
                self.camera.zoom_at(*x, *y, *delta * 1.0);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }
}
