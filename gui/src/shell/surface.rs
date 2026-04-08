use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub fn create_surface(
    instance: &wgpu::Instance,
    window: Arc<Window>,
) -> wgpu::Surface<'static> {
    instance
        .create_surface(window)
        .expect("failed to create surface")
}

pub fn configure(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    size: PhysicalSize<u32>,
) -> wgpu::SurfaceConfiguration {
    let caps = surface.get_capabilities(adapter);
    let format = caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };

    surface.configure(device, &config);
    config
}

pub fn resize(
    surface: &wgpu::Surface<'_>,
    device: &wgpu::Device,
    config: &mut wgpu::SurfaceConfiguration,
    new_size: PhysicalSize<u32>,
) {
    if new_size.width > 0 && new_size.height > 0 {
        config.width = new_size.width;
        config.height = new_size.height;
        surface.configure(device, config);
    }
}
