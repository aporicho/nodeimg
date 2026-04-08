use winit::dpi::PhysicalSize;

use super::blit::BlitPipeline;
use super::buffer::SharedViewport;
use super::circle::CirclePipeline;
use super::command::DrawCommand;
use super::curve::CurvePipeline;
use super::image::ImagePipeline;
use super::prepare::{prepare_frame, DrawOp};
use super::quad::QuadPipeline;
use super::shadow::ShadowPipeline;
use super::stencil::StencilState;
use super::text::{TextPipeline, TextRequest};

pub fn dispatch(
    commands: &[DrawCommand],
    frame_view: &wgpu::TextureView,
    msaa_view: &wgpu::TextureView,
    resolve_view: &wgpu::TextureView,
    internal_size: PhysicalSize<u32>,
    scale_factor: f64,
    render_scale: f32,
    clear_color: super::types::Color,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    blit: &BlitPipeline,
    shared_viewport: &mut SharedViewport,
    quad_pipeline: &mut QuadPipeline,
    text_pipeline: &mut TextPipeline,
    image_pipeline: &mut ImagePipeline,
    circle_pipeline: &mut CirclePipeline,
    curve_pipeline: &mut CurvePipeline,
    shadow_pipeline: &mut ShadowPipeline,
    stencil: &mut StencilState,
) {
    // viewport 用放大后的逻辑尺寸，这样坐标 * render_scale 映射到内部分辨率
    let logical_w = internal_size.width as f64 / scale_factor / render_scale as f64;
    let logical_h = internal_size.height as f64 / scale_factor / render_scale as f64;
    let viewport_size = [logical_w as f32, logical_h as f32];

    // viewport uniform 只写一次
    shared_viewport.upload(device, queue, viewport_size);
    let viewport_buf = shared_viewport.buffer();

    // ── 阶段 1：文字 prepare ──

    let text_requests: Vec<&TextRequest> = commands
        .iter()
        .filter_map(|cmd| match cmd {
            DrawCommand::Text(req) => Some(req),
            _ => None,
        })
        .collect();

    if !text_requests.is_empty() {
        let refs: Vec<TextRequest> = text_requests
            .iter()
            .map(|r| TextRequest {
                pos: r.pos,
                text: r.text.clone(),
                style: super::style::TextStyle {
                    color: r.style.color,
                    size: r.style.size,
                },
            })
            .collect();
        text_pipeline.prepare(device, queue, &refs, internal_size, scale_factor * render_scale as f64);
    }

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("renderer"),
    });

    // ── 阶段 2：阴影 prepare ──

    for cmd in commands {
        if let DrawCommand::Shadow(req) = cmd {
            shadow_pipeline.prepare(&mut encoder, device, req);
        }
    }

    // ── 阶段 3：预处理 command list → PreparedFrame ──

    let prepared = prepare_frame(commands, curve_pipeline);

    // ── 阶段 4：上传 buffer（&mut pipeline）──

    quad_pipeline.upload(device, queue, &prepared.quad_vertices, &prepared.quad_indices);
    circle_pipeline.upload(device, queue, &prepared.circle_vertices, &prepared.circle_indices);
    curve_pipeline.upload(device, queue, &prepared.curve_vertices, &prepared.curve_indices);
    stencil.upload(device, queue, &prepared.stencil_vertices, &prepared.stencil_indices);

    // 确保 bind group 已缓存
    quad_pipeline.update_bind_group(device, viewport_buf);
    circle_pipeline.update_bind_group(device, viewport_buf);
    curve_pipeline.update_bind_group(device, viewport_buf);
    stencil.update_bind_group(device, viewport_buf);

    // ── 阶段 5：主 render pass（渲染到超分辨率 MSAA → resolve 到中间 texture）──

    let has_quads = !prepared.quad_vertices.is_empty();
    let has_stencils = !prepared.stencil_vertices.is_empty();
    let has_text = !text_requests.is_empty();

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(resolve_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: clear_color.r as f64,
                        g: clear_color.g as f64,
                        b: clear_color.b as f64,
                        a: clear_color.a as f64,
                    }),
                    store: wgpu::StoreOp::Discard,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: stencil.depth_stencil_view(),
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });

        // 绑定各管线 buffer（每帧一次）
        if has_quads {
            quad_pipeline.bind(&mut pass);
        }

        let mut last_bound = PipelineKind::None;
        let mut clip_depth: u32 = 0;

        for op in &prepared.ops {
            match op {
                DrawOp::Quad { index_start, index_count } => {
                    if last_bound != PipelineKind::Quad {
                        quad_pipeline.bind(&mut pass);
                        last_bound = PipelineKind::Quad;
                    }
                    pass.set_stencil_reference(clip_depth);
                    QuadPipeline::draw_batch(&mut pass, *index_start, *index_count);
                }
                DrawOp::Circle { index_start, index_count } => {
                    if last_bound != PipelineKind::Circle {
                        circle_pipeline.bind(&mut pass);
                        last_bound = PipelineKind::Circle;
                    }
                    pass.set_stencil_reference(clip_depth);
                    CirclePipeline::draw_batch(&mut pass, *index_start, *index_count);
                }
                DrawOp::Curve { index_start, index_count } => {
                    if last_bound != PipelineKind::Curve {
                        curve_pipeline.bind(&mut pass);
                        last_bound = PipelineKind::Curve;
                    }
                    pass.set_stencil_reference(clip_depth);
                    CurvePipeline::draw_batch(&mut pass, *index_start, *index_count);
                }
                DrawOp::Shadow(req) => {
                    last_bound = PipelineKind::Other;
                    shadow_pipeline.draw(&mut pass, device, req, viewport_buf, clip_depth);
                }
                DrawOp::Image { rect, view } => {
                    last_bound = PipelineKind::Other;
                    pass.set_stencil_reference(clip_depth);
                    image_pipeline.draw(&mut pass, device, view, *rect, viewport_buf);
                }
                DrawOp::Text => {
                    // text 在最后统一渲染
                }
                DrawOp::StencilWrite { index_start, index_count } => {
                    if has_stencils && last_bound != PipelineKind::Stencil {
                        stencil.bind_stencil(&mut pass);
                        last_bound = PipelineKind::Stencil;
                    }
                    stencil.draw_write(&mut pass, clip_depth, *index_start, *index_count);
                    clip_depth += 1;
                }
                DrawOp::StencilClear { index_start, index_count } => {
                    if has_stencils && last_bound != PipelineKind::Stencil {
                        stencil.bind_stencil(&mut pass);
                        last_bound = PipelineKind::Stencil;
                    }
                    stencil.draw_clear(&mut pass, clip_depth, *index_start, *index_count);
                    if clip_depth > 0 {
                        clip_depth -= 1;
                    }
                }
            }
        }

        if has_text {
            pass.set_stencil_reference(clip_depth);
            text_pipeline.render(&mut pass);
        }
    }

    // ── 阶段 6：blit pass（中间 texture → surface，缩放采样）──

    {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blit_bind_group"),
            layout: &blit.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(resolve_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&blit.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: frame_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&blit.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    queue.submit(std::iter::once(encoder.finish()));
}

#[derive(PartialEq)]
enum PipelineKind {
    None,
    Quad,
    Circle,
    Curve,
    Stencil,
    Other,
}
