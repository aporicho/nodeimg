use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;

pub enum AppEvent {
    Resized(PhysicalSize<u32>),
    CloseRequested,
}

pub fn translate(event: &WindowEvent) -> Option<AppEvent> {
    match event {
        WindowEvent::Resized(size) => Some(AppEvent::Resized(*size)),
        WindowEvent::CloseRequested => Some(AppEvent::CloseRequested),
        _ => None,
    }
}
