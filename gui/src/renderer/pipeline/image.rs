#![allow(dead_code)]

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::geometry::Affine2D;

use super::super::affine::transformed_rect_corners;
use super::super::image::{ImageFilter, ImageSourceRect, ResolvedImageDraw};
use super::super::types::Rect;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ImageInstance {
    p0p1: [f32; 4],
    p2p3: [f32; 4],
    uv_rect: [f32; 4],
    modulate: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparedImageDraw {
    pub positions: [[f32; 2]; 4],
    pub uv_rect: ImageSourceRect,
    pub modulate: [f32; 4],
    pub filter: ImageFilter,
}

impl PreparedImageDraw {
    pub fn from_resolved(draw: ResolvedImageDraw, transform: Affine2D) -> Self {
        Self {
            positions: transformed_rect_corners(transform, draw.rect),
            uv_rect: draw.uv_rect,
            modulate: draw.modulate,
            filter: draw.filter,
        }
    }

    pub fn from_rect(rect: Rect, transform: Affine2D) -> Self {
        Self {
            positions: transformed_rect_corners(transform, rect),
            uv_rect: ImageSourceRect::FULL,
            modulate: [1.0; 4],
            filter: ImageFilter::Linear,
        }
    }
}

pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    linear_sampler: wgpu::Sampler,
    nearest_sampler: wgpu::Sampler,
}

impl ImagePipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/image.wgsl").into()),
        });

        let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image_linear_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("image_nearest_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image_bind_group_layout"),
            entries: &[
                // viewport uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(super::stencil::content_depth_stencil_state()),
            multisample,
            cache: None,
            multiview_mask: None,
        });

        Self {
            pipeline,
            bind_group_layout,
            linear_sampler,
            nearest_sampler,
        }
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
        draw: PreparedImageDraw,
        viewport_buf: &'a wgpu::Buffer,
    ) {
        let sampler = match draw.filter {
            ImageFilter::Linear => &self.linear_sampler,
            ImageFilter::Nearest => &self.nearest_sampler,
        };
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        let instance = ImageInstance {
            p0p1: [
                draw.positions[0][0],
                draw.positions[0][1],
                draw.positions[1][0],
                draw.positions[1][1],
            ],
            p2p3: [
                draw.positions[2][0],
                draw.positions[2][1],
                draw.positions[3][0],
                draw.positions[3][1],
            ],
            uv_rect: [
                draw.uv_rect.x,
                draw.uv_rect.y,
                draw.uv_rect.w,
                draw.uv_rect.h,
            ],
            modulate: draw.modulate,
        };
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("image_instance_buffer"),
            contents: bytemuck::cast_slice(&[instance]),
            usage: wgpu::BufferUsages::VERTEX,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, instance_buffer.slice(..));
        pass.draw(0..6, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::renderer::{ImageFilter, ImageSourceRect};

    #[test]
    fn prepared_image_draw_applies_affine_to_rect_corners() {
        let draw = ResolvedImageDraw {
            rect: Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            },
            uv_rect: ImageSourceRect::FULL,
            modulate: [1.0; 4],
            filter: ImageFilter::Linear,
        };

        let prepared = PreparedImageDraw::from_resolved(draw, Affine2D::translation(10.0, 20.0));

        assert_eq!(prepared.positions[0], [11.0, 22.0]);
        assert_eq!(prepared.positions[2], [14.0, 26.0]);
        assert_eq!(prepared.uv_rect, ImageSourceRect::FULL);
    }
}
