use bytemuck::{Pod, Zeroable};

use super::buffer::DynamicBuffer;
use super::types::{Color, Point};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CurveVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

pub struct CurveRequest {
    pub points: [Point; 4],
    pub width: f32,
    pub color: Color,
}

pub struct CurvePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buf: DynamicBuffer,
    index_buf: DynamicBuffer,
}

impl CurvePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, multisample: wgpu::MultisampleState) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("curve_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/curve.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("curve_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("curve_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CurveVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 8,
                    shader_location: 1,
                },
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("curve_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout],
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
            vertex_buf: DynamicBuffer::new(device, wgpu::BufferUsages::VERTEX, "curve_vertex_buffer", 4096),
            index_buf: DynamicBuffer::new(device, wgpu::BufferUsages::INDEX, "curve_index_buffer", 4096),
        }
    }

    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[CurveVertex],
        indices: &[u32],
    ) {
        if vertices.is_empty() {
            return;
        }
        self.vertex_buf.write(device, queue, bytemuck::cast_slice(vertices));
        self.index_buf.write(device, queue, bytemuck::cast_slice(indices));
    }

    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, device: &wgpu::Device, viewport_buf: &'a wgpu::Buffer) {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("curve_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buf.as_entire_binding(),
            }],
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.buffer().slice(..));
        pass.set_index_buffer(self.index_buf.buffer().slice(..), wgpu::IndexFormat::Uint32);
    }

    pub fn draw_batch(pass: &mut wgpu::RenderPass<'_>, index_start: u32, index_count: u32) {
        pass.draw_indexed(index_start..index_start + index_count, 0, 0..1);
    }
}
