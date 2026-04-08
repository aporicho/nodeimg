use bytemuck::{Pod, Zeroable};

use super::buffer::DynamicBuffer;
use super::corner::DEFAULT_CORNER_SMOOTHING;
use super::style::RectStyle;
use super::types::Rect;

// ── 渲染 ──

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct QuadVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone)]
pub struct QuadRequest {
    pub rect: Rect,
    pub style_color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
    pub border_width: f32,
    pub radius: [f32; 4],
    pub smoothing: f32,
}

impl QuadRequest {
    pub fn from_style(rect: Rect, style: &RectStyle) -> Self {
        Self {
            rect,
            style_color: style.color.to_array(),
            border_color: style.border.map(|b| b.color.to_array()),
            border_width: style.border.map_or(0.0, |b| b.width),
            radius: style.radius,
            smoothing: DEFAULT_CORNER_SMOOTHING,
        }
    }
}

pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buf: DynamicBuffer,
    index_buf: DynamicBuffer,
    viewport_bind_group: Option<wgpu::BindGroup>,
}

impl QuadPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, multisample: wgpu::MultisampleState) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("quad_bind_group_layout"),
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
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
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
            label: Some("quad_pipeline"),
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
            vertex_buf: DynamicBuffer::new(device, wgpu::BufferUsages::VERTEX, "quad_vertex_buffer", 4096),
            index_buf: DynamicBuffer::new(device, wgpu::BufferUsages::INDEX, "quad_index_buffer", 4096),
            viewport_bind_group: None,
        }
    }

    /// render pass 之前调用：上传顶点/索引数据
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[QuadVertex],
        indices: &[u32],
    ) {
        if vertices.is_empty() {
            return;
        }
        self.vertex_buf.write(device, queue, bytemuck::cast_slice(vertices));
        self.index_buf.write(device, queue, bytemuck::cast_slice(indices));
    }

    /// render pass 之前调用：确保 viewport bind group 已缓存
    pub fn update_bind_group(&mut self, device: &wgpu::Device, viewport_buf: &wgpu::Buffer) {
        if self.viewport_bind_group.is_none() {
            self.viewport_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("quad_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buf.as_entire_binding(),
                }],
            }));
        }
    }

    /// render pass 内调用：绑定管线 + buffer
    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let bg = self.viewport_bind_group.as_ref().expect("call update_bind_group before bind");
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.buffer().slice(..));
        pass.set_index_buffer(self.index_buf.buffer().slice(..), wgpu::IndexFormat::Uint32);
    }

    /// render pass 内调用：绘制指定索引范围
    pub fn draw_batch(pass: &mut wgpu::RenderPass<'_>, index_start: u32, index_count: u32) {
        pass.draw_indexed(index_start..index_start + index_count, 0, 0..1);
    }
}
