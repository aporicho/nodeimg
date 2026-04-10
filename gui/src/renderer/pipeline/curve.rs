use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use lyon::math::point;
use lyon::path::Path;
use lyon::tessellation::{BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers};

use super::super::buffer::DynamicBuffer;
use super::super::types::{Color, Point};

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

#[derive(Hash, PartialEq, Eq, Clone)]
struct CurveCacheKey {
    points: [u32; 8], // 4 个 Point 的 f32 bits
    width_bits: u32,
    color_bits: [u32; 4],
}

impl CurveCacheKey {
    fn new(req: &CurveRequest) -> Self {
        let c = req.color.to_array();
        Self {
            points: [
                req.points[0].x.to_bits(), req.points[0].y.to_bits(),
                req.points[1].x.to_bits(), req.points[1].y.to_bits(),
                req.points[2].x.to_bits(), req.points[2].y.to_bits(),
                req.points[3].x.to_bits(), req.points[3].y.to_bits(),
            ],
            width_bits: req.width.to_bits(),
            color_bits: [c[0].to_bits(), c[1].to_bits(), c[2].to_bits(), c[3].to_bits()],
        }
    }
}

struct CachedTessellation {
    vertices: Vec<CurveVertex>,
    indices: Vec<u32>,
}

pub struct CurvePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    vertex_buf: DynamicBuffer,
    index_buf: DynamicBuffer,
    viewport_bind_group: Option<wgpu::BindGroup>,
    tess_cache: HashMap<CurveCacheKey, CachedTessellation>,
}

impl CurvePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, multisample: wgpu::MultisampleState) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("curve_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/curve.wgsl").into()),
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
            viewport_bind_group: None,
            tess_cache: HashMap::new(),
        }
    }

    /// 三角化一条曲线，命中缓存则跳过。返回 (vertices, indices)。
    pub fn tessellate(&mut self, req: &CurveRequest) -> (&[CurveVertex], &[u32]) {
        let key = CurveCacheKey::new(req);
        self.tess_cache.entry(key.clone()).or_insert_with(|| {
            let color = req.color.to_array();
            let mut geometry: VertexBuffers<CurveVertex, u32> = VertexBuffers::new();
            let mut tessellator = StrokeTessellator::new();

            let mut builder = Path::builder();
            builder.begin(point(req.points[0].x, req.points[0].y));
            builder.cubic_bezier_to(
                point(req.points[1].x, req.points[1].y),
                point(req.points[2].x, req.points[2].y),
                point(req.points[3].x, req.points[3].y),
            );
            builder.end(false);
            let path = builder.build();

            tessellator
                .tessellate_path(
                    &path,
                    &StrokeOptions::default().with_line_width(req.width),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| CurveVertex {
                        position: vertex.position().to_array(),
                        color,
                    }),
                )
                .expect("failed to tessellate curve");

            CachedTessellation {
                vertices: geometry.vertices,
                indices: geometry.indices,
            }
        });
        let cached = &self.tess_cache[&key];
        (&cached.vertices, &cached.indices)
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

    pub fn update_bind_group(&mut self, device: &wgpu::Device, viewport_buf: &wgpu::Buffer) {
        if self.viewport_bind_group.is_none() {
            self.viewport_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("curve_bind_group"),
                layout: &self.bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buf.as_entire_binding(),
                }],
            }));
        }
    }

    pub fn bind<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let bg = self.viewport_bind_group.as_ref().expect("call update_bind_group before bind");
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bg, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.buffer().slice(..));
        pass.set_index_buffer(self.index_buf.buffer().slice(..), wgpu::IndexFormat::Uint32);
    }

    pub fn draw_batch(pass: &mut wgpu::RenderPass<'_>, index_start: u32, index_count: u32) {
        pass.draw_indexed(index_start..index_start + index_count, 0, 0..1);
    }
}
