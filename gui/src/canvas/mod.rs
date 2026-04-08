pub mod background;
pub mod camera;
pub mod pan;

use camera::Camera;
use pan::PanState;

use crate::renderer::Renderer;
use crate::shell::{AppEvent, MouseButton};

const SCROLL_LINE_ZOOM_SPEED: f32 = 0.1;
const PINCH_ZOOM_SPEED: f32 = 1.0;

pub struct Canvas {
    pub camera: Camera,
    pan: PanState,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            pan: PanState::new(),
        }
    }

    /// 处理画布事件。返回 true 表示事件被消费。
    pub fn event(&mut self, event: &AppEvent) -> bool {
        match event {
            // 中键拖拽 = 画布平移
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
            // 鼠标滚轮 = 缩放
            AppEvent::ScrollLine { x, y, delta_y, .. } => {
                self.camera.zoom_at(*x, *y, *delta_y * SCROLL_LINE_ZOOM_SPEED);
                return true;
            }
            // trackpad 双指滑动 = 平移
            AppEvent::ScrollPixel { delta_x, delta_y, .. } => {
                self.camera.pan(*delta_x, *delta_y);
                return true;
            }
            // trackpad 双指捏合 = 缩放
            AppEvent::PinchZoom { x, y, delta } => {
                self.camera.zoom_at(*x, *y, *delta * PINCH_ZOOM_SPEED);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn render(&self, renderer: &mut Renderer, viewport_w: f32, viewport_h: f32) {
        background::render(renderer, &self.camera, viewport_w, viewport_h);
    }
}
