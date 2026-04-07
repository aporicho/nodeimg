use winit::dpi::PhysicalSize;

use super::command::DrawCommand;
use super::curve::{CurvePipeline, CurveRequest};
use super::image::ImagePipeline;
use super::quad::{QuadPipeline, QuadRequest};
use super::shadow::ShadowPipeline;
use super::stencil::StencilState;
use super::text::{TextPipeline, TextRequest};

pub fn dispatch(
    commands: &[DrawCommand],
    frame_view: &wgpu::TextureView,
    msaa_view: &wgpu::TextureView,
    size: PhysicalSize<u32>,
    scale_factor: f64,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    quad_pipeline: &QuadPipeline,
    text_pipeline: &mut TextPipeline,
    image_pipeline: &ImagePipeline,
    curve_pipeline: &CurvePipeline,
    shadow_pipeline: &mut ShadowPipeline,
    stencil: &mut StencilState,
) {
    let logical_w = size.width as f64 / scale_factor;
    let logical_h = size.height as f64 / scale_factor;
    let viewport_size = [logical_w as f32, logical_h as f32];

    // 收集文字请求用于 prepare（必须在 render pass 之前）
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
        text_pipeline.prepare(device, queue, &refs, size, scale_factor);
    }

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("renderer"),
    });

    // 阶段一：在主 render pass 之前，prepare 所有阴影（形状绘制 + compute blur）
    for cmd in commands {
        if let DrawCommand::Shadow(req) = cmd {
            shadow_pipeline.prepare(&mut encoder, device, req);
        }
    }

    // 阶段二：主 render pass
    // 收集 clip 信息用于 PopClip
    let mut clip_stack: Vec<(super::types::Rect, f32)> = Vec::new();

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: msaa_view,
                resolve_target: Some(frame_view),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.15,
                        g: 0.15,
                        b: 0.15,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
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

        let mut quad_batch: Vec<QuadRequest> = Vec::new();
        let mut curve_batch: Vec<CurveRequest> = Vec::new();
        let mut text_rendered = false;

        for cmd in commands {
            match cmd {
                DrawCommand::Shadow(req) => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    shadow_pipeline.draw(&mut pass, device, req, viewport_size, stencil.clip_depth());
                }
                DrawCommand::Rect(instance) => {
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    quad_batch.push(instance.clone());
                }
                DrawCommand::Text(_) => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    if !text_rendered {
                        text_rendered = true;
                    }
                }
                DrawCommand::Image { rect, view } => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    pass.set_stencil_reference(stencil.clip_depth());
                    image_pipeline.draw(&mut pass, device, view, *rect, viewport_size);
                }
                DrawCommand::Curve(req) => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    curve_batch.push(CurveRequest {
                        points: req.points,
                        width: req.width,
                        color: req.color,
                    });
                }
                DrawCommand::PushClip { rect, radius } => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    clip_stack.push((*rect, *radius));
                    stencil.write_clip(&mut pass, device, *rect, *radius, viewport_size);
                }
                DrawCommand::PopClip => {
                    flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
                    flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());
                    if let Some((rect, radius)) = clip_stack.pop() {
                        stencil.clear_clip(&mut pass, device, rect, radius, viewport_size);
                    }
                }
            }
        }

        flush_quads(&mut quad_batch, &mut pass, quad_pipeline, device, viewport_size, stencil.clip_depth());
        flush_curves(&mut curve_batch, &mut pass, curve_pipeline, device, viewport_size, stencil.clip_depth());

        if text_rendered {
            pass.set_stencil_reference(stencil.clip_depth());
            text_pipeline.render(&mut pass);
        }
    }

    queue.submit(std::iter::once(encoder.finish()));
}

fn flush_quads<'a>(
    batch: &mut Vec<QuadRequest>,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline: &'a QuadPipeline,
    device: &wgpu::Device,
    viewport_size: [f32; 2],
    stencil_ref: u32,
) {
    if batch.is_empty() {
        return;
    }
    pass.set_stencil_reference(stencil_ref);
    pipeline.draw(pass, device, batch, viewport_size);
    batch.clear();
}

fn flush_curves<'a>(
    batch: &mut Vec<CurveRequest>,
    pass: &mut wgpu::RenderPass<'a>,
    pipeline: &'a CurvePipeline,
    device: &wgpu::Device,
    viewport_size: [f32; 2],
    stencil_ref: u32,
) {
    if batch.is_empty() {
        return;
    }
    pass.set_stencil_reference(stencil_ref);
    pipeline.draw(pass, device, batch, viewport_size);
    batch.clear();
}
