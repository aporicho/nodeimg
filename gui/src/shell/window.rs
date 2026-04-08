use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    let attrs = Window::default_attributes()
        .with_title("nodeimg")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0));

    Arc::new(event_loop.create_window(attrs).expect("failed to create window"))
}
