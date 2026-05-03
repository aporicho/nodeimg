use winit::dpi::PhysicalSize;

use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::text::TextPipeline;
use super::super::prepare::PreparedFrame;
use super::super::text_measurer::TextMeasurer;
use super::super::types::Color;
use super::load_ops::{color_load_op, color_store_op, stencil_load_op};

pub(super) struct TextPassContext<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) prepared: &'a mut PreparedFrame,
    pub(super) indices: &'a [usize],
    pub(super) clip_depth: u32,
    pub(super) internal_size: PhysicalSize<u32>,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
    pub(super) msaa_view: &'a wgpu::TextureView,
    pub(super) resolve_target: Option<&'a wgpu::TextureView>,
    pub(super) clear_color: Color,
    pub(super) first_pass: bool,
    pub(super) stencil: &'a mut StencilState,
    pub(super) text_pipeline: &'a mut TextPipeline,
    pub(super) text_measurer: &'a mut TextMeasurer,
}

pub(super) fn render_text_pass(ctx: TextPassContext<'_>) {
    let TextPassContext {
        encoder,
        device,
        queue,
        prepared,
        indices,
        clip_depth,
        internal_size,
        scale_factor,
        render_scale,
        msaa_view,
        resolve_target,
        clear_color,
        first_pass,
        stencil,
        text_pipeline,
        text_measurer,
    } = ctx;

    let text_requests = indices
        .iter()
        .map(|index| prepared.text_requests[*index].clone())
        .collect::<Vec<_>>();
    let batch_index = text_pipeline.prepare(
        device,
        queue,
        &text_requests,
        internal_size,
        scale_factor * render_scale as f64,
        text_measurer,
    );
    prepared.stats.text_batches += 1;

    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("main_text"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: msaa_view,
            resolve_target,
            ops: wgpu::Operations {
                load: color_load_op(first_pass, clear_color),
                store: color_store_op(),
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: stencil.depth_stencil_view(),
            depth_ops: None,
            stencil_ops: Some(wgpu::Operations {
                load: stencil_load_op(first_pass),
                store: wgpu::StoreOp::Store,
            }),
        }),
        ..Default::default()
    });

    pass.set_stencil_reference(clip_depth);
    text_pipeline.render_batch(batch_index, &mut pass);
}
