use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};

pub enum AppEvent {
    Resized(PhysicalSize<u32>),
    CloseRequested,
    MousePress { x: f32, y: f32 },
}

/// 有状态的事件翻译器，追踪光标位置。
pub struct EventTranslator {
    cursor_x: f64,
    cursor_y: f64,
    scale_factor: f64,
}

impl EventTranslator {
    pub fn new(scale_factor: f64) -> Self {
        Self { cursor_x: 0.0, cursor_y: 0.0, scale_factor }
    }

    pub fn translate(&mut self, event: &WindowEvent) -> Option<AppEvent> {
        match event {
            WindowEvent::Resized(size) => Some(AppEvent::Resized(*size)),
            WindowEvent::CloseRequested => Some(AppEvent::CloseRequested),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;
                None
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let x = (self.cursor_x / self.scale_factor) as f32;
                let y = (self.cursor_y / self.scale_factor) as f32;
                Some(AppEvent::MousePress { x, y })
            }
            _ => None,
        }
    }
}
