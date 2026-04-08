use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct AppContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub window: Arc<Window>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    pub scale_factor: f64,
}
