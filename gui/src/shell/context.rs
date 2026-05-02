use arboard::Clipboard;
use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::window::Window;

use crate::context::ImeRequest;
use crate::cursor::{CursorKind, CursorState};

pub struct AppContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub window: Arc<Window>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub scale_factor: f64,
    pub(crate) cursor: CursorState,
    pub(crate) ime_allowed: bool,
    pub(crate) clipboard: Option<Clipboard>,
    pub(crate) redraw_requested: bool,
}

impl AppContext {
    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    pub fn set_cursor(&mut self, cursor: CursorKind) {
        self.cursor.set(cursor);
    }

    pub fn cursor(&self) -> CursorKind {
        self.cursor.desired()
    }

    pub(crate) fn apply_cursor(&mut self) {
        self.cursor.apply_to_window(&self.window);
    }

    pub(crate) fn take_redraw_request(&mut self) -> bool {
        std::mem::take(&mut self.redraw_requested)
    }

    pub fn apply_ime_request(&mut self, request: ImeRequest) {
        if self.ime_allowed != request.allowed {
            self.window.set_ime_allowed(request.allowed);
            self.ime_allowed = request.allowed;
        }

        if request.allowed {
            if let Some(rect) = request.cursor_area {
                self.window.set_ime_cursor_area(
                    LogicalPosition::new(rect.x as f64, rect.y as f64),
                    LogicalSize::new(rect.w.max(1.0) as f64, rect.h.max(1.0) as f64),
                );
            }
        }
    }

    pub fn clipboard_read_text(&mut self) -> Option<String> {
        self.clipboard.as_mut()?.get_text().ok()
    }

    pub fn clipboard_write_text(&mut self, text: &str) -> bool {
        self.clipboard
            .as_mut()
            .map(|clipboard| clipboard.set_text(text.to_string()).is_ok())
            .unwrap_or(false)
    }
}
