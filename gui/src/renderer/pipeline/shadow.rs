use std::collections::HashMap;
use std::sync::Arc;

use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};
use wgpu::util::DeviceExt;

use crate::geometry::Affine2D;

use super::super::buffer::ViewportUniform;
use super::super::style::Shadow;
use super::super::types::Rect;
use super::blur::{BlurPipeline, BlurRun};
use super::image::PreparedImageDraw;
use super::quad::{build_rounded_rect_path, QuadVertex, DEFAULT_CORNER_SMOOTHING};

// ── 请求 ──

#[derive(Clone)]
pub struct ShadowRequest {
    pub rect: Rect,
    pub radius: [f32; 4],
    pub shadow: Shadow,
}

// ── 缓存 ──

#[derive(Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    // f32 不能直接 Hash，用 bits 表示
    w_bits: u32,
    h_bits: u32,
    radius_bits: [u32; 4],
    blur_bits: u32,
    spread_bits: u32,
    color_bits: [u32; 4],
}

impl CacheKey {
    fn new(rect: Rect, radius: [f32; 4], shadow: &Shadow) -> Self {
        Self {
            w_bits: rect.w.to_bits(),
            h_bits: rect.h.to_bits(),
            radius_bits: [
                radius[0].to_bits(),
                radius[1].to_bits(),
                radius[2].to_bits(),
                radius[3].to_bits(),
            ],
            blur_bits: shadow.blur.to_bits(),
            spread_bits: shadow.spread.to_bits(),
            color_bits: [
                shadow.color.r.to_bits(),
                shadow.color.g.to_bits(),
                shadow.color.b.to_bits(),
                shadow.color.a.to_bits(),
            ],
        }
    }
}

struct CachedShadow {
    texture_view: Arc<wgpu::TextureView>,
    tex_w: u32,
    tex_h: u32,
    unused_frames: u32,
}

// ── Pipeline ──

pub struct ShadowPipeline {
    blur: BlurPipeline,
    shape_pipeline: wgpu::RenderPipeline,
    shape_bind_group_layout: wgpu::BindGroupLayout,
    cache: HashMap<CacheKey, CachedShadow>,
    downsample_factor: u32,
}

struct ShapePass<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    device: &'a wgpu::Device,
    target: &'a wgpu::TextureView,
    rect: Rect,
    radius: [f32; 4],
    shadow: &'a Shadow,
    tex_w: u32,
    tex_h: u32,
}

impl ShadowPipeline {
    pub fn new(
        device: &wgpu::Device,
        _format: wgpu::TextureFormat,
        _multisample: wgpu::MultisampleState,
    ) -> Self {
        let blur = BlurPipeline::new(device);
        let (shape_pipeline, shape_bind_group_layout) = Self::create_shape_pipeline(device);

        Self {
            blur,
            shape_pipeline,
            shape_bind_group_layout,
            cache: HashMap::new(),
            downsample_factor: 1,
        }
    }

    /// 阶段一：在主 render pass 之前调用。检查缓存，miss 则渲染形状 + 模糊。
    pub fn prepare(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        request: &ShadowRequest,
    ) {
        let key = CacheKey::new(request.rect, request.radius, &request.shadow);

        // 命中缓存：重置计数器
        if let Some(cached) = self.cache.get_mut(&key) {
            cached.unused_frames = 0;
            return;
        }

        // miss：渲染 + 模糊 + 插入缓存
        let shadow = &request.shadow;
        let expand = shadow.blur + shadow.spread;
        let tex_w = ((request.rect.w + expand * 2.0) / self.downsample_factor as f32).ceil() as u32;
        let tex_h = ((request.rect.h + expand * 2.0) / self.downsample_factor as f32).ceil() as u32;

        let blurred_view =
            Arc::new(self.render_and_blur(device, encoder, request, tex_w, tex_h, expand));
        self.cache.insert(
            key,
            CachedShadow {
                texture_view: blurred_view,
                tex_w,
                tex_h,
                unused_frames: 0,
            },
        );
    }

    pub fn prepared_image(
        &self,
        request: &ShadowRequest,
        transform: Affine2D,
    ) -> Option<(Arc<wgpu::TextureView>, PreparedImageDraw)> {
        let key = CacheKey::new(request.rect, request.radius, &request.shadow);
        let shadow = &request.shadow;
        let expand = shadow.blur + shadow.spread;
        let cached = self.cache.get(&key)?;
        let dest_rect = Rect {
            x: request.rect.x + shadow.offset[0] - expand,
            y: request.rect.y + shadow.offset[1] - expand,
            w: cached.tex_w as f32 * self.downsample_factor as f32,
            h: cached.tex_h as f32 * self.downsample_factor as f32,
        };
        Some((
            cached.texture_view.clone(),
            PreparedImageDraw::from_rect(dest_rect, transform),
        ))
    }

    /// 每帧结束时清理过期缓存
    pub fn evict_cache(&mut self) {
        const MAX_UNUSED_FRAMES: u32 = 60;
        self.cache.retain(|_, v| {
            v.unused_frames += 1;
            v.unused_frames <= MAX_UNUSED_FRAMES
        });
    }

    // ── 内部：形状绘制 + 模糊 ──

    fn render_and_blur(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        request: &ShadowRequest,
        tex_w: u32,
        tex_h: u32,
        expand: f32,
    ) -> wgpu::TextureView {
        let shadow = &request.shadow;

        // 创建三张纹理：shape → blur_a → blur_b → blur_a（最终结果）
        let shape_tex = Self::create_texture(
            device,
            tex_w,
            tex_h,
            "shadow_shape",
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let blur_a = Self::create_texture(
            device,
            tex_w,
            tex_h,
            "shadow_blur_a",
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        );
        let blur_b = Self::create_texture(
            device,
            tex_w,
            tex_h,
            "shadow_blur_b",
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        );

        let shape_view = shape_tex.create_view(&Default::default());
        let blur_a_view = blur_a.create_view(&Default::default());
        let blur_b_view = blur_b.create_view(&Default::default());

        // 1. 画阴影形状到 shape_tex
        let spread_rect = Rect {
            x: expand,
            y: expand,
            w: request.rect.w + shadow.spread * 2.0,
            h: request.rect.h + shadow.spread * 2.0,
        };
        self.render_shape(ShapePass {
            encoder,
            device,
            target: &shape_view,
            rect: spread_rect,
            radius: request.radius,
            shadow,
            tex_w,
            tex_h,
        });

        let sigma = shadow.blur / 2.0;

        // 2. 水平模糊：shape_tex → blur_b
        self.blur.run(BlurRun {
            encoder,
            device,
            input: &shape_view,
            output: &blur_b_view,
            tex_w,
            tex_h,
            direction: [1.0, 0.0],
            sigma,
        });

        // 3. 垂直模糊：blur_b → blur_a
        self.blur.run(BlurRun {
            encoder,
            device,
            input: &blur_b_view,
            output: &blur_a_view,
            tex_w,
            tex_h,
            direction: [0.0, 1.0],
            sigma,
        });

        blur_a_view
    }

    fn render_shape(&self, pass: ShapePass<'_>) {
        let ShapePass {
            encoder,
            device,
            target,
            rect,
            radius,
            shadow,
            tex_w,
            tex_h,
        } = pass;
        let path = build_rounded_rect_path(rect, radius, DEFAULT_CORNER_SMOOTHING);
        let color = shadow.color.to_array();

        let mut geometry: VertexBuffers<QuadVertex, u32> = VertexBuffers::new();
        let mut tessellator = FillTessellator::new();
        tessellator
            .tessellate_path(
                &path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| QuadVertex {
                    position: vertex.position().to_array(),
                    color,
                }),
            )
            .expect("failed to tessellate shadow shape");

        if geometry.indices.is_empty() {
            return;
        }

        let viewport_size = [tex_w as f32, tex_h as f32];
        let uniform = ViewportUniform {
            size: viewport_size,
            _padding: [0.0; 2],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_shape_viewport"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_shape_bind_group"),
            layout: &self.shape_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_shape_vertices"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_shape_indices"),
            contents: bytemuck::cast_slice(&geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_shape_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });

            pass.set_pipeline(&self.shape_pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..geometry.indices.len() as u32, 0, 0..1);
        }
    }

    // ── Pipeline 创建 ──

    fn create_shape_pipeline(
        device: &wgpu::Device,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        // 复用 quad shader（逻辑像素 → NDC + 纯色）画到临时纹理
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shape_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/quad.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow_shape_bind_group_layout"),
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

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_shape_pipeline_layout"),
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
            label: Some("shadow_shape_pipeline"),
            layout: Some(&layout),
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
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        (pipeline, bind_group_layout)
    }

    fn create_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
        usage: wgpu::TextureUsages,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    }
}
