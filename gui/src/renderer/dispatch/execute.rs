use crate::diagnostics::render_trace::{self, RenderTraceStage};

use super::super::command::BackendCommand;
use super::super::prepare::prepare_frame;
use super::super::scene_prepare::RendererPrepareStats;
use super::api::{DispatchFrame, DispatchPipelines};
use super::blit::{render_blit_pass, BlitPassContext};
use super::deferred::{resolve_deferred_ops, ResolveDeferredOps};
use super::render_pass::{render_main_passes, RenderMainPasses};
use super::render_steps::plan_render_steps;
use super::trace::{
    DispatchPlanTraceSummary, DispatchPrepareTraceSummary, DispatchSubmitTraceSummary,
};
use super::upload::{upload_prepared_buffers, UploadPreparedBuffers};

pub(in crate::renderer) fn dispatch(
    commands: &[BackendCommand],
    frame: DispatchFrame<'_>,
    pipelines: DispatchPipelines<'_>,
) -> RendererPrepareStats {
    let logical_w =
        frame.internal_size.width as f64 / frame.scale_factor / frame.render_scale as f64;
    let logical_h =
        frame.internal_size.height as f64 / frame.scale_factor / frame.render_scale as f64;
    let viewport_size = [logical_w as f32, logical_h as f32];

    let DispatchFrame {
        frame_view,
        msaa_view,
        resolve_view,
        internal_size,
        scale_factor,
        render_scale,
        format,
        clear_color,
        device,
        queue,
    } = frame;
    let DispatchPipelines {
        blit,
        shared_viewport,
        quad_pipeline,
        text_pipeline,
        image_pipeline,
        circle_pipeline,
        grid_pipeline,
        vector_pipeline,
        vector_tessellator,
        svg_raster_cache,
        shadow_pipeline,
        stencil,
        text_measurer,
    } = pipelines;

    let mut upload_stats = shared_viewport.upload(device, queue, viewport_size);
    let viewport_buf = shared_viewport.buffer();
    render_trace::debug_gpu(
        RenderTraceStage::RendererPrepare,
        DispatchPrepareTraceSummary {
            backend_commands: commands.len(),
            logical_w: viewport_size[0],
            logical_h: viewport_size[1],
        },
    );

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("renderer"),
    });

    let mut prepared = prepare_frame(commands, vector_tessellator);
    render_trace::debug_gpu(RenderTraceStage::RendererPrepare, &prepared.stats);
    let mut runtime_textures = Vec::new();
    text_pipeline.begin_frame();
    text_measurer.mark_all_unused();
    let deferred_stats = resolve_deferred_ops(ResolveDeferredOps {
        ops: &mut prepared.ops,
        encoder: &mut encoder,
        device,
        queue,
        format,
        scale_factor,
        render_scale,
        text_pipeline,
        svg_raster_cache,
        shadow_pipeline,
        text_measurer,
        runtime_textures: &mut runtime_textures,
    });
    render_trace::debug_gpu(RenderTraceStage::RendererDispatch, deferred_stats);

    upload_prepared_buffers(UploadPreparedBuffers {
        upload_stats: &mut upload_stats,
        prepared: &mut prepared,
        device,
        queue,
        viewport_buf,
        quad_pipeline,
        circle_pipeline,
        grid_pipeline,
        vector_pipeline,
        stencil,
    });

    let render_steps = plan_render_steps(&prepared.ops);
    prepared.stats.render_passes = render_steps.len();
    render_trace::debug_gpu(
        RenderTraceStage::RendererDispatch,
        DispatchPlanTraceSummary {
            ops: prepared.ops.len(),
            text_requests: prepared.text_requests.len(),
            render_steps: render_steps.len(),
            quad_vertices: prepared.quad_vertices.len(),
            circle_vertices: prepared.circle_vertices.len(),
            grid_vertices: prepared.grid_vertices.len(),
            vector_vertices: prepared.vector_vertices.len(),
            stencil_vertices: prepared.stencil_vertices.len(),
        },
    );

    render_main_passes(RenderMainPasses {
        encoder: &mut encoder,
        device,
        queue,
        prepared: &mut prepared,
        render_steps: &render_steps,
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
    });

    text_measurer.evict_unused();
    render_blit_pass(BlitPassContext {
        encoder: &mut encoder,
        device,
        frame_view,
        resolve_view,
        blit,
    });

    queue.submit(std::iter::once(encoder.finish()));
    render_trace::debug_gpu(
        RenderTraceStage::RendererDispatch,
        DispatchSubmitTraceSummary {
            backend_commands: prepared.stats.backend_commands,
            render_passes: prepared.stats.render_passes,
            text_batches: prepared.stats.text_batches,
            upload_bytes: prepared.stats.upload_bytes,
            upload_buffer_grows: prepared.stats.upload_buffer_grows,
        },
    );
    prepared.stats
}
