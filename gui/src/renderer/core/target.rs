use winit::dpi::PhysicalSize;

use super::config::MSAA_SAMPLE_COUNT;

pub(super) fn scale_size(size: PhysicalSize<u32>, scale: f32) -> PhysicalSize<u32> {
    PhysicalSize::new(
        ((size.width as f32 * scale) as u32).max(1),
        ((size.height as f32 * scale) as u32).max(1),
    )
}

pub(super) fn create_msaa_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: PhysicalSize<u32>,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_texture"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
