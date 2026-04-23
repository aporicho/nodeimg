use winit::dpi::PhysicalSize;

use crate::paint::DisplayList;

use super::buffer::SharedViewport;
use super::command::BackendCommand;
use super::dispatch;
use super::display_backend::{lower_display_list, DisplayRenderReport};
use super::display_resources::DisplayResourceResolver;
use super::pipeline::blit::{self, BlitPipeline};
use super::pipeline::circle::CirclePipeline;
use super::pipeline::image::ImagePipeline;
use super::pipeline::quad::QuadPipeline;
use super::pipeline::shadow::ShadowPipeline;
use super::pipeline::stencil::StencilState;
use super::pipeline::text::TextPipeline;
use super::pipeline::vector::VectorPipeline;
use super::svg::{SvgRasterCache, SvgVectorCache};
use super::text_measurer::TextMeasurer;
use super::types::Color;
use super::vector_tessellator::VectorTessellator;

pub const MSAA_SAMPLE_COUNT: u32 = 4;
const DEFAULT_RENDER_SCALE: f32 = 2.0;

pub struct Renderer {
    shared_viewport: SharedViewport,
    quad_pipeline: QuadPipeline,
    text_measurer: TextMeasurer,
    text_pipeline: TextPipeline,
    image_pipeline: ImagePipeline,
    circle_pipeline: CirclePipeline,
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
    frame: Option<FrameState>,
}

struct FrameState {
    view: wgpu::TextureView,
    size: PhysicalSize<u32>,
    scale_factor: f64,
}

fn msaa_multisample_state() -> wgpu::MultisampleState {
    wgpu::MultisampleState {
        count: MSAA_SAMPLE_COUNT,
        mask: !0,
        alpha_to_coverage_enabled: false,
    }
}

pub(super) fn scale_size(size: PhysicalSize<u32>, scale: f32) -> PhysicalSize<u32> {
    PhysicalSize::new(
        ((size.width as f32 * scale) as u32).max(1),
        ((size.height as f32 * scale) as u32).max(1),
    )
}

fn create_msaa_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: PhysicalSize<u32>,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_texture"),
        size: wgpu::Extent3d {
            width: size.width.max(1),
            height: size.height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: MSAA_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
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
        output.report
    }

    pub fn end_frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(frame) = self.frame.take() else {
            return;
        };

        let internal = scale_size(frame.size, self.render_scale);

        dispatch::dispatch(
            &self.backend_commands,
            dispatch::DispatchFrame {
                frame_view: &frame.view,
                msaa_view: &self.msaa_view,
                resolve_view: &self.resolve_view,
                internal_size: internal,
                scale_factor: frame.scale_factor,
                render_scale: self.render_scale,
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
                vector_pipeline: &mut self.vector_pipeline,
                vector_tessellator: &mut self.vector_tessellator,
                svg_raster_cache: &mut self.svg_raster_cache,
                shadow_pipeline: &mut self.shadow_pipeline,
                stencil: &mut self.stencil,
                text_measurer: &mut self.text_measurer,
            },
        );
        self.shadow_pipeline.evict_cache();
    }
}
