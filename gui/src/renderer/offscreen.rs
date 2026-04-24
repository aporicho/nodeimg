use std::sync::Arc;

use winit::dpi::PhysicalSize;

use crate::geometry::Affine2D;

use super::pipeline::stencil::DEPTH_STENCIL_FORMAT;
use super::types::{Point, Rect};

pub(super) struct OffscreenTarget {
    pub texture: wgpu::Texture,
    pub view: Arc<wgpu::TextureView>,
    pub msaa_texture: wgpu::Texture,
    pub msaa_view: wgpu::TextureView,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
    pub size: PhysicalSize<u32>,
}

impl OffscreenTarget {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        size: PhysicalSize<u32>,
        sample_count: u32,
        label: &'static str,
    ) -> Self {
        let size = PhysicalSize::new(size.width.max(1), size.height.max(1));
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent(size),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = Arc::new(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_msaa"),
            size: extent(size),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen_depth_stencil"),
            size: extent(size),
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_STENCIL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            msaa_texture,
            msaa_view,
            depth_texture,
            depth_view,
            size,
        }
    }

    pub fn keep_alive(self, textures: &mut Vec<wgpu::Texture>) {
        textures.push(self.texture);
        textures.push(self.msaa_texture);
        textures.push(self.depth_texture);
    }
}

pub(super) fn transformed_rect_pixel_size(
    transform: Affine2D,
    rect: Rect,
    scale_factor: f64,
    render_scale: f32,
) -> PhysicalSize<u32> {
    let corners = transform.transform_rect_corners(rect);
    let width = distance(corners[0], corners[1]).max(distance(corners[3], corners[2]));
    let height = distance(corners[0], corners[3]).max(distance(corners[1], corners[2]));
    let scale = scale_factor as f32 * render_scale;
    PhysicalSize::new(
        (width.abs() * scale).ceil().max(1.0) as u32,
        (height.abs() * scale).ceil().max(1.0) as u32,
    )
}

fn extent(size: PhysicalSize<u32>) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: size.width.max(1),
        height: size.height.max(1),
        depth_or_array_layers: 1,
    }
}

fn distance(a: Point, b: Point) -> f32 {
    (b.x - a.x).hypot(b.y - a.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformed_rect_pixel_size_uses_edge_lengths() {
        let size = transformed_rect_pixel_size(
            Affine2D::rotation_radians(std::f32::consts::FRAC_PI_4),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 20.0,
            },
            1.0,
            2.0,
        );

        assert_eq!(size.width, 20);
        assert_eq!(size.height, 40);
    }
}
