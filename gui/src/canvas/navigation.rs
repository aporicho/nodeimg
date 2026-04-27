use super::camera::Camera;
use super::pan::PanState;
use crate::shell::{AppEvent, MouseButton};

/// 画布导航控制器：统一处理中键拖拽、双指平移、滚轮缩放、捏合缩放。
pub struct CanvasNavigationController {
    pan: PanState,
}

impl CanvasNavigationController {
    pub fn new() -> Self {
        Self {
            pan: PanState::new(),
        }
    }

    /// 处理画布导航输入。返回值表示事件是否被导航层消费。
    pub fn handle_event(&mut self, event: &AppEvent, camera: &mut Camera) -> bool {
        match *event {
            AppEvent::MousePress {
                x,
                y,
                button: MouseButton::Middle,
            } => {
                self.pan.start(x, y);
                true
            }
            AppEvent::MouseMove { x, y } if self.pan.is_active() => {
                self.pan.update(x, y, camera);
                true
            }
            AppEvent::MouseRelease {
                button: MouseButton::Middle,
                ..
            } => {
                self.pan.end();
                true
            }
            AppEvent::ScrollLine { x, y, delta_y, .. } => {
                camera.zoom_at(x, y, delta_y * 0.1);
                true
            }
            AppEvent::ScrollPixel {
                delta_x, delta_y, ..
            } => {
                // macOS trackpad natural scrolling: keep canvas content moving with the fingers.
                camera.pan(delta_x, delta_y);
                true
            }
            AppEvent::PinchZoom { x, y, delta } => {
                camera.zoom_at(x, y, delta);
                true
            }
            _ => false,
        }
    }

    pub fn is_panning(&self) -> bool {
        self.pan.is_active()
    }
}

impl Default for CanvasNavigationController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_scroll_pans_canvas_content_with_trackpad_delta() {
        let mut navigation = CanvasNavigationController::new();
        let mut camera = Camera::new();

        assert!(navigation.handle_event(
            &AppEvent::ScrollPixel {
                x: 0.0,
                y: 0.0,
                delta_x: 20.0,
                delta_y: 10.0,
            },
            &mut camera,
        ));

        assert_eq!(camera.x, 20.0);
        assert_eq!(camera.y, 10.0);
    }
}
