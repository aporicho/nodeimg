use bytemuck::{Pod, Zeroable};
use lyon::math::{point, vector, Angle};
use lyon::path::traits::SvgPathBuilder;
use lyon::path::{ArcFlags, Path};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};
use wgpu::util::DeviceExt;

use super::style::RectStyle;
use super::types::Rect;

/// 默认 corner smoothing（iOS 风格）
pub const DEFAULT_CORNER_SMOOTHING: f32 = 0.6;

// ── Figma cornerSmoothing 算法 ──

struct CornerParams {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    p: f32,
    arc_section_length: f32,
    corner_radius: f32,
}

fn get_corner_params(
    corner_radius: f32,
    mut corner_smoothing: f32,
    rounding_and_smoothing_budget: f32,
) -> CornerParams {
    let mut p = (1.0 + corner_smoothing) * corner_radius;

    if p > rounding_and_smoothing_budget {
        let max_smoothing = rounding_and_smoothing_budget / corner_radius - 1.0;
        corner_smoothing = corner_smoothing.min(max_smoothing);
        p = p.min(rounding_and_smoothing_budget);
    }

    let arc_measure = 90.0 * (1.0 - corner_smoothing);
    let arc_section_length =
        (arc_measure / 2.0).to_radians().sin() * corner_radius * std::f32::consts::SQRT_2;

    let angle_alpha = (90.0 - arc_measure) / 2.0;
    let p3_to_p4_distance = corner_radius * (angle_alpha / 2.0).to_radians().tan();

    let angle_beta = 45.0 * corner_smoothing;
    let c = p3_to_p4_distance * angle_beta.to_radians().cos();
    let d = c * angle_beta.to_radians().tan();

    let b = (p - arc_section_length - c - d) / 3.0;
    let a = 2.0 * b;

    CornerParams {
        a,
        b,
        c,
        d,
        p,
        arc_section_length,
        corner_radius,
    }
}

/// 生成 Figma 风格的圆角矩形路径
///
/// 严格翻译自 figma-squircle 的 getSVGPathFromPathParams + drawCornerPath。
/// 使用 lyon 的 SVG builder，直接映射 SVG 相对命令（c, a）。
pub fn build_rounded_rect_path(rect: Rect, radius: [f32; 4], smoothing: f32) -> Path {
    let w = rect.w;
    let h = rect.h;
    let budget = w.min(h) / 2.0;

    let tr = get_corner_params(radius[1].min(budget), smoothing, budget);
    let br = get_corner_params(radius[2].min(budget), smoothing, budget);
    let bl = get_corner_params(radius[3].min(budget), smoothing, budget);
    let tl = get_corner_params(radius[0].min(budget), smoothing, budget);

    let x = rect.x;
    let y = rect.y;
    let flags = ArcFlags { large_arc: false, sweep: true };

    let mut b = Path::builder().with_svg();

    // M (w - tr.p) 0
    b.move_to(point(x + w - tr.p, y));

    // 右上角: c a 0 (a+b) 0 (a+b+c) d | a R R 0 0 1 al al | c d c d (b+c) d (a+b+c)
    if tr.corner_radius > 0.0 {
        let abc = tr.a + tr.b + tr.c;
        let al = tr.arc_section_length;
        let r = tr.corner_radius;
        b.relative_cubic_bezier_to(vector(tr.a, 0.0), vector(tr.a + tr.b, 0.0), vector(abc, tr.d));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, al));
        b.relative_cubic_bezier_to(vector(tr.d, tr.c), vector(tr.d, tr.b + tr.c), vector(tr.d, abc));
    }

    // L w (h - br.p)
    b.line_to(point(x + w, y + h - br.p));

    // 右下角: c 0 a 0 (a+b) -d (a+b+c) | a R R 0 0 1 -al al | c -c d -(b+c) d -(a+b+c) d
    if br.corner_radius > 0.0 {
        let abc = br.a + br.b + br.c;
        let al = br.arc_section_length;
        let r = br.corner_radius;
        b.relative_cubic_bezier_to(vector(0.0, br.a), vector(0.0, br.a + br.b), vector(-br.d, abc));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, al));
        b.relative_cubic_bezier_to(vector(-br.c, br.d), vector(-(br.b + br.c), br.d), vector(-abc, br.d));
    }

    // L bl.p h
    b.line_to(point(x + bl.p, y + h));

    // 左下角: c -a 0 -(a+b) 0 -(a+b+c) -d | a R R 0 0 1 -al -al | c -d -c -d -(b+c) -d -(a+b+c)
    if bl.corner_radius > 0.0 {
        let abc = bl.a + bl.b + bl.c;
        let al = bl.arc_section_length;
        let r = bl.corner_radius;
        b.relative_cubic_bezier_to(vector(-bl.a, 0.0), vector(-(bl.a + bl.b), 0.0), vector(-abc, -bl.d));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, -al));
        b.relative_cubic_bezier_to(vector(-bl.d, -bl.c), vector(-bl.d, -(bl.b + bl.c)), vector(-bl.d, -abc));
    }

    // L 0 tl.p
    b.line_to(point(x, y + tl.p));

    // 左上角: c 0 -a 0 -(a+b) d -(a+b+c) | a R R 0 0 1 al -al | c c -d (b+c) -d (a+b+c) -d
    if tl.corner_radius > 0.0 {
        let abc = tl.a + tl.b + tl.c;
        let al = tl.arc_section_length;
        let r = tl.corner_radius;
        b.relative_cubic_bezier_to(vector(0.0, -tl.a), vector(0.0, -(tl.a + tl.b)), vector(tl.d, -abc));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, -al));
        b.relative_cubic_bezier_to(vector(tl.c, -tl.d), vector(tl.b + tl.c, -tl.d), vector(abc, -tl.d));
    }

    // Z
    b.close();
    b.build()
}

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

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViewportUniform {
    size: [f32; 2],
    _padding: [f32; 2],
}

pub struct QuadPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
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
        }
    }

    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        requests: &[QuadRequest],
        viewport_size: [f32; 2],
    ) {
        if requests.is_empty() {
            return;
        }

        let mut geometry: VertexBuffers<QuadVertex, u32> = VertexBuffers::new();
        let mut fill_tessellator = FillTessellator::new();
        let mut stroke_tessellator = StrokeTessellator::new();

        for req in requests {
            let path = build_rounded_rect_path(req.rect, req.radius, req.smoothing);
            let color = req.style_color;

            // 填充
            fill_tessellator
                .tessellate_path(
                    &path,
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| QuadVertex {
                        position: vertex.position().to_array(),
                        color,
                    }),
                )
                .expect("failed to tessellate quad fill");

            // 描边
            if req.border_width > 0.0 {
                if let Some(border_color) = req.border_color {
                    stroke_tessellator
                        .tessellate_path(
                            &path,
                            &StrokeOptions::default().with_line_width(req.border_width),
                            &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                                QuadVertex {
                                    position: vertex.position().to_array(),
                                    color: border_color,
                                }
                            }),
                        )
                        .expect("failed to tessellate quad stroke");
                }
            }
        }

        if geometry.indices.is_empty() {
            return;
        }

        let uniform = ViewportUniform {
            size: viewport_size,
            _padding: [0.0; 2],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_viewport_uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_vertex_buffer"),
            contents: bytemuck::cast_slice(&geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_index_buffer"),
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
