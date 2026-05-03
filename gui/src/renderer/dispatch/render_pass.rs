use crate::diagnostics::render_trace::{self, RenderTraceStage};
use winit::dpi::PhysicalSize;

use super::super::pipeline::circle::CirclePipeline;
use super::super::pipeline::grid::GridPipeline;
use super::super::pipeline::image::ImagePipeline;
use super::super::pipeline::quad::QuadPipeline;
use super::super::pipeline::shadow::ShadowPipeline;
use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::text::TextPipeline;
use super::super::pipeline::vector::VectorPipeline;
use super::super::prepare::PreparedFrame;
use super::super::text_measurer::TextMeasurer;
use super::super::types::Color;
use super::ops_pass::{render_ops_pass, OpsPassContext};
use super::render_steps::{should_resolve_step, RenderStep};
use super::text_pass::{render_text_pass, TextPassContext};
use super::trace::RenderStepTraceSummary;

pub(super) struct RenderMainPasses<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) prepared: &'a mut PreparedFrame,
    pub(super) render_steps: &'a [RenderStep],
    pub(super) msaa_view: &'a wgpu::TextureView,
    pub(super) resolve_view: &'a wgpu::TextureView,
    pub(super) internal_size: PhysicalSize<u32>,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
    pub(super) clear_color: Color,
    pub(super) viewport_buf: &'a wgpu::Buffer,
    pub(super) quad_pipeline: &'a mut QuadPipeline,
    pub(super) text_pipeline: &'a mut TextPipeline,
    pub(super) image_pipeline: &'a mut ImagePipeline,
    pub(super) circle_pipeline: &'a mut CirclePipeline,
    pub(super) grid_pipeline: &'a mut GridPipeline,
    pub(super) vector_pipeline: &'a mut VectorPipeline,
    pub(super) shadow_pipeline: &'a mut ShadowPipeline,
    pub(super) stencil: &'a mut StencilState,
    pub(super) text_measurer: &'a mut TextMeasurer,
}

pub(super) fn render_main_passes(ctx: RenderMainPasses<'_>) {
    let RenderMainPasses {
        encoder,
        device,
        queue,
        prepared,
        render_steps,
        msaa_view,
        resolve_view,
        internal_size,
        scale_factor,
        render_scale,
        clear_color,
        viewport_buf,
        quad_pipeline,
        text_pipeline,
        image_pipeline,
        circle_pipeline,
        grid_pipeline,
        vector_pipeline,
        shadow_pipeline,
        stencil,
        text_measurer,
    } = ctx;

    let has_quads = !prepared.quad_vertices.is_empty();
    let has_grids = !prepared.grid_vertices.is_empty();
    let has_stencils = !prepared.stencil_vertices.is_empty();
    let total_steps = render_steps.len();
    let mut first_pass = true;
    for (step_index, step) in render_steps.iter().enumerate() {
        render_trace::trace_gpu(
            RenderTraceStage::RendererDispatch,
            RenderStepTraceSummary {
                step_index,
                total_steps,
                kind: step.kind(),
            },
        );
        let resolve_target = should_resolve_step(step_index, total_steps).then_some(resolve_view);
        match step {
            RenderStep::Ops {
                range,
                starting_clip_depth,
            } => render_ops_pass(OpsPassContext {
                encoder,
                device,
                prepared,
                range: range.clone(),
                starting_clip_depth: *starting_clip_depth,
                msaa_view,
                resolve_target,
                clear_color,
                first_pass,
                viewport_buf,
                has_quads,
                has_grids,
                has_stencils,
                quad_pipeline,
                image_pipeline,
                circle_pipeline,
                grid_pipeline,
                vector_pipeline,
                shadow_pipeline,
                stencil,
            }),
            RenderStep::TextBatch {
                indices,
                clip_depth,
            } => render_text_pass(TextPassContext {
                encoder,
                device,
                queue,
                prepared,
                indices,
                clip_depth: *clip_depth,
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
            }),
        }
        first_pass = false;
    }
}
