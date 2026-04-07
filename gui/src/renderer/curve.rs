use bytemuck::{Pod, Zeroable};
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{
    BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
};
use wgpu::util::DeviceExt;

use super::types::{Color, Point};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CurveVertex {
    position: [f32; 2],
    color: [f32; 4],
}

pub struct CurveRequest {
    pub points: [Point; 4],
    pub width: f32,
    pub color: Color,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViewportUniform {
    size: [f32; 2],
    _padding: [f32; 2],
}

pub struct CurvePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
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
        }
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        curves: &[CurveRequest],
        viewport_size: [f32; 2],
    ) {
        if curves.is_empty() {
            return;
        }

        let mut geometry: VertexBuffers<CurveVertex, u32> = VertexBuffers::new();
        let mut tessellator = StrokeTessellator::new();

        for curve in curves {
            let color = curve.color.to_array();

            let mut builder = Path::builder();
            builder.begin(point(curve.points[0].x, curve.points[0].y));
            builder.cubic_bezier_to(
                point(curve.points[1].x, curve.points[1].y),
                point(curve.points[2].x, curve.points[2].y),
                point(curve.points[3].x, curve.points[3].y),
            );
            builder.end(false);
            let path = builder.build();

            tessellator
                .tessellate_path(
                    &path,
                    &StrokeOptions::default().with_line_width(curve.width),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                        CurveVertex {
                            position: vertex.position().to_array(),
                            color,
                        }
                    }),
                )
                .expect("failed to tessellate curve");
        }

        if geometry.indices.is_empty() {
            return;
        }

        let uniform = ViewportUniform {
            size: viewport_size,
            _padding: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve_viewport_uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("curve_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve_vertex_buffer"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("curve_index_buffer"),
            contents: bytemuck::cast_slice(&geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..geometry.indices.len() as u32, 0, 0..1);
    }
}
