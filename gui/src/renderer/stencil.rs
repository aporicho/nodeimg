use bytemuck::{Pod, Zeroable};
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

use super::quad::{build_rounded_rect_path, DEFAULT_CORNER_SMOOTHING};
use super::types::Rect;

pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

/// 内容管线的 stencil state：只读，Equal 比较，不修改 stencil
pub fn content_depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_STENCIL_FORMAT,
        depth_write_enabled: false,
        depth_compare: wgpu::CompareFunction::Always,
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            back: wgpu::StencilFaceState {
                compare: wgpu::CompareFunction::Equal,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            read_mask: 0xFF,
            write_mask: 0x00,
        },
        bias: wgpu::DepthBiasState::default(),
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct StencilVertex {
    position: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViewportUniform {
    size: [f32; 2],
    _padding: [f32; 2],
}

pub struct StencilState {
    depth_stencil_view: wgpu::TextureView,
    increment_pipeline: wgpu::RenderPipeline,
    decrement_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    clip_depth: u32,
    #[allow(dead_code)]
    size: PhysicalSize<u32>,
}

impl StencilState {
    pub fn new(
        device: &wgpu::Device,
        size: PhysicalSize<u32>,
        surface_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let depth_stencil_view = create_depth_stencil_view(device, size, multisample.count);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stencil_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/stencil.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stencil_bind_group_layout"),
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
            label: Some("stencil_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StencilVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };

        let increment_pipeline = create_stencil_pipeline(
            device, &shader, &pipeline_layout, &vertex_layout,
            surface_format, wgpu::StencilOperation::IncrementClamp, "stencil_increment", multisample,
        );
        let decrement_pipeline = create_stencil_pipeline(
            device, &shader, &pipeline_layout, &vertex_layout,
            surface_format, wgpu::StencilOperation::DecrementClamp, "stencil_decrement", multisample,
        );

        Self {
            depth_stencil_view,
            increment_pipeline,
            decrement_pipeline,
            bind_group_layout,
            clip_depth: 0,
            size,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.depth_stencil_view = create_depth_stencil_view(device, size, super::renderer::MSAA_SAMPLE_COUNT);
        }
    }

    pub fn depth_stencil_view(&self) -> &wgpu::TextureView {
        &self.depth_stencil_view
    }

    pub fn clip_depth(&self) -> u32 {
        self.clip_depth
    }

    pub fn reset(&mut self) {
        self.clip_depth = 0;
    }

    pub fn write_clip(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        device: &wgpu::Device,
        rect: Rect,
        radius: f32,
        viewport_size: [f32; 2],
    ) {
        let (vertex_buffer, index_buffer, index_count) =
            tessellate_clip_shape(device, rect, radius);

        let (_uniform_buffer, bind_group) =
            self.create_bind_group(device, viewport_size);

        pass.set_pipeline(&self.increment_pipeline);
        pass.set_stencil_reference(self.clip_depth);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);

        self.clip_depth += 1;
    }

    pub fn clear_clip(
        &mut self,
        pass: &mut wgpu::RenderPass<'_>,
        device: &wgpu::Device,
        rect: Rect,
        radius: f32,
        viewport_size: [f32; 2],
    ) {
        if self.clip_depth == 0 {
            return;
        }

        let (vertex_buffer, index_buffer, index_count) =
            tessellate_clip_shape(device, rect, radius);

        let (_uniform_buffer, bind_group) =
            self.create_bind_group(device, viewport_size);

        pass.set_pipeline(&self.decrement_pipeline);
        pass.set_stencil_reference(self.clip_depth);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..index_count, 0, 0..1);

        self.clip_depth -= 1;
    }

    fn create_bind_group(
        &self,
        device: &wgpu::Device,
        viewport_size: [f32; 2],
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let uniform = ViewportUniform {
            size: viewport_size,
            _padding: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("stencil_uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stencil_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        (uniform_buffer, bind_group)
    }
}

fn tessellate_clip_shape(
    device: &wgpu::Device,
    rect: Rect,
    radius: f32,
) -> (wgpu::Buffer, wgpu::Buffer, u32) {
    let path = build_rounded_rect_path(rect, [radius; 4], DEFAULT_CORNER_SMOOTHING);

    let mut geometry: VertexBuffers<StencilVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    tessellator
        .tessellate_path(
            &path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| StencilVertex {
                position: vertex.position().to_array(),
            }),
        )
        .expect("failed to tessellate stencil shape");

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("stencil_vertex"),
        contents: bytemuck::cast_slice(&geometry.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("stencil_index"),
        contents: bytemuck::cast_slice(&geometry.indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    (vertex_buffer, index_buffer, geometry.indices.len() as u32)
}

fn create_depth_stencil_view(device: &wgpu::Device, size: PhysicalSize<u32>, sample_count: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_stencil"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_STENCIL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_stencil_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    vertex_layout: &wgpu::VertexBufferLayout<'_>,
    surface_format: wgpu::TextureFormat,
    pass_op: wgpu::StencilOperation,
    label: &str,
    multisample: wgpu::MultisampleState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_layout.clone()],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: None,
                write_mask: wgpu::ColorWrites::empty(),
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op,
                },
                back: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op,
                },
                read_mask: 0xFF,
                write_mask: 0xFF,
            },
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample,
        cache: None,
        multiview_mask: None,
    })
}
