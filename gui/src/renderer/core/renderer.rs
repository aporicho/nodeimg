use winit::dpi::PhysicalSize;

use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::paint::DisplayList;

use super::super::buffer::SharedViewport;
use super::super::command::BackendCommand;
use super::super::dispatch;
use super::super::display_backend::{lower_display_list, DisplayRenderReport};
use super::super::display_resources::DisplayResourceResolver;
use super::super::pipeline::blit::{self, BlitPipeline};
use super::super::pipeline::circle::CirclePipeline;
use super::super::pipeline::grid::GridPipeline;
use super::super::pipeline::image::ImagePipeline;
use super::super::pipeline::quad::QuadPipeline;
use super::super::pipeline::shadow::ShadowPipeline;
use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::text::TextPipeline;
use super::super::pipeline::vector::VectorPipeline;
use super::super::scene_prepare::RendererPrepareStats;
use super::super::svg::{SvgRasterCache, SvgVectorCache};
use super::super::text_measurer::TextMeasurer;
use super::super::types::Color;
use super::super::vector_tessellator::VectorTessellator;
use super::config::{msaa_multisample_state, DEFAULT_RENDER_SCALE};
use super::frame_state::FrameState;
use super::target::{create_msaa_texture, scale_size};
use super::trace::{
    RendererBeginFrameTraceSummary, RendererDrawTraceSummary, RendererEndFrameTraceSummary,
};

pub struct Renderer {
    shared_viewport: SharedViewport,
    quad_pipeline: QuadPipeline,
    text_measurer: TextMeasurer,
    text_pipeline: TextPipeline,
    image_pipeline: ImagePipeline,
    circle_pipeline: CirclePipeline,
    grid_pipeline: GridPipeline,
    vector_pipeline: VectorPipeline,
    vector_tessellator: VectorTessellator,
    svg_vector_cache: SvgVectorCache,
    svg_raster_cache: SvgRasterCache,
    shadow_pipeline: ShadowPipeline,
    stencil: StencilState,
    msaa_view: wgpu::TextureView,
    resolve_view: wgpu::TextureView,
    blit: BlitPipeline,
    format: wgpu::TextureFormat,
    render_scale: f32,
    clear_color: Color,
    backend_commands: Vec<BackendCommand>,
    last_prepare_stats: RendererPrepareStats,
    frame: Option<FrameState>,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        size: PhysicalSize<u32>,
    ) -> Self {
        let ms = msaa_multisample_state();
        let render_scale = DEFAULT_RENDER_SCALE;
        let internal = scale_size(size, render_scale);
        let mut text_measurer = TextMeasurer::new();

        Self {
            shared_viewport: SharedViewport::new(device),
            quad_pipeline: QuadPipeline::new(device, format, ms),
            text_pipeline: TextPipeline::new(
                device,
                queue,
                format,
                ms,
                &mut text_measurer.font_system,
            ),
            text_measurer,
            image_pipeline: ImagePipeline::new(device, format, ms),
            circle_pipeline: CirclePipeline::new(device, format, ms),
            grid_pipeline: GridPipeline::new(device, format, ms),
            vector_pipeline: VectorPipeline::new(device, format, ms),
            vector_tessellator: VectorTessellator::new(),
            svg_vector_cache: SvgVectorCache::new(),
            svg_raster_cache: SvgRasterCache::new(),
            shadow_pipeline: ShadowPipeline::new(device, format, ms),
            stencil: StencilState::new(device, internal, format, ms),
            msaa_view: create_msaa_texture(device, format, internal),
            resolve_view: blit::create_resolve_texture(device, format, internal),
            blit: BlitPipeline::new(device, format),
            format,
            render_scale,
            clear_color: Color::BLACK,
            backend_commands: Vec::new(),
            last_prepare_stats: RendererPrepareStats::default(),
            frame: None,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, size: PhysicalSize<u32>) {
        let internal = scale_size(size, self.render_scale);
        self.stencil.resize(device, internal);
        self.msaa_view = create_msaa_texture(device, self.format, internal);
        self.resolve_view = blit::create_resolve_texture(device, self.format, internal);
    }

    pub fn set_clear_color(&mut self, color: Color) {
        self.clear_color = color;
    }

    pub fn begin_frame(
        &mut self,
        view: wgpu::TextureView,
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) {
        self.backend_commands.clear();
        self.stencil.reset();
        render_trace::debug_gpu(
            RenderTraceStage::RendererDispatch,
            RendererBeginFrameTraceSummary {
                width: size.width,
                height: size.height,
                scale_factor,
                render_scale: self.render_scale,
            },
        );
        self.frame = Some(FrameState {
            view,
            size,
            scale_factor,
        });
    }

    pub fn measure_text(&mut self, text: &str, size: f32) -> (f32, f32) {
        self.text_measurer.measure(text, size)
    }

    pub fn text_measurer(&mut self) -> &mut TextMeasurer {
        &mut self.text_measurer
    }

    pub(crate) fn draw_display_list<R: DisplayResourceResolver>(
        &mut self,
        list: &DisplayList,
        resources: &R,
    ) -> DisplayRenderReport {
        let output = lower_display_list(list, resources, &mut self.svg_vector_cache);
        self.backend_commands.extend(output.commands);
        render_trace::debug_gpu(
            RenderTraceStage::DisplayListLowering,
            RendererDrawTraceSummary {
                display_commands: list.commands.len(),
                display_clips: list.clips.len(),
                backend_commands_total: self.backend_commands.len(),
                unsupported: output.report.unsupported.len(),
            },
        );
        output.report
    }

    pub fn last_prepare_stats(&self) -> &RendererPrepareStats {
        &self.last_prepare_stats
    }

    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame.take() else {
            return;
        };

        let internal = scale_size(frame.size, self.render_scale);
        render_trace::debug_gpu(
            RenderTraceStage::RendererDispatch,
            RendererEndFrameTraceSummary {
                backend_commands: self.backend_commands.len(),
                internal_w: internal.width,
                internal_h: internal.height,
            },
        );

        self.last_prepare_stats = dispatch::dispatch(
            &self.backend_commands,
            dispatch::DispatchFrame {
                frame_view: &frame.view,
                msaa_view: &self.msaa_view,
                resolve_view: &self.resolve_view,
                internal_size: internal,
                scale_factor: frame.scale_factor,
                render_scale: self.render_scale,
                format: self.format,
                clear_color: self.clear_color,
                device,
                queue,
            },
            dispatch::DispatchPipelines {
                blit: &self.blit,
                shared_viewport: &mut self.shared_viewport,
                quad_pipeline: &mut self.quad_pipeline,
                text_pipeline: &mut self.text_pipeline,
                image_pipeline: &mut self.image_pipeline,
                circle_pipeline: &mut self.circle_pipeline,
                grid_pipeline: &mut self.grid_pipeline,
                vector_pipeline: &mut self.vector_pipeline,
                vector_tessellator: &mut self.vector_tessellator,
                svg_raster_cache: &mut self.svg_raster_cache,
                shadow_pipeline: &mut self.shadow_pipeline,
                stencil: &mut self.stencil,
                text_measurer: &mut self.text_measurer,
            },
        );
        render_trace::debug_gpu(RenderTraceStage::RendererDispatch, &self.last_prepare_stats);
        self.shadow_pipeline.evict_cache();
    }
}
