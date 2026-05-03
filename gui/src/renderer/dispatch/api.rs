use winit::dpi::PhysicalSize;

use super::super::buffer::SharedViewport;
use super::super::pipeline::blit::BlitPipeline;
use super::super::pipeline::circle::CirclePipeline;
use super::super::pipeline::grid::GridPipeline;
use super::super::pipeline::image::ImagePipeline;
use super::super::pipeline::quad::QuadPipeline;
use super::super::pipeline::shadow::ShadowPipeline;
use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::text::TextPipeline;
use super::super::pipeline::vector::VectorPipeline;
use super::super::svg::SvgRasterCache;
use super::super::text_measurer::TextMeasurer;
use super::super::types::Color;
use super::super::vector_tessellator::VectorTessellator;

pub(in crate::renderer) struct DispatchFrame<'a> {
    pub(in crate::renderer) frame_view: &'a wgpu::TextureView,
    pub(in crate::renderer) msaa_view: &'a wgpu::TextureView,
    pub(in crate::renderer) resolve_view: &'a wgpu::TextureView,
    pub(in crate::renderer) internal_size: PhysicalSize<u32>,
    pub(in crate::renderer) scale_factor: f64,
    pub(in crate::renderer) render_scale: f32,
    pub(in crate::renderer) format: wgpu::TextureFormat,
    pub(in crate::renderer) clear_color: Color,
    pub(in crate::renderer) device: &'a wgpu::Device,
    pub(in crate::renderer) queue: &'a wgpu::Queue,
}

pub(in crate::renderer) struct DispatchPipelines<'a> {
    pub(in crate::renderer) blit: &'a BlitPipeline,
    pub(in crate::renderer) shared_viewport: &'a mut SharedViewport,
    pub(in crate::renderer) quad_pipeline: &'a mut QuadPipeline,
    pub(in crate::renderer) text_pipeline: &'a mut TextPipeline,
    pub(in crate::renderer) image_pipeline: &'a mut ImagePipeline,
    pub(in crate::renderer) circle_pipeline: &'a mut CirclePipeline,
    pub(in crate::renderer) grid_pipeline: &'a mut GridPipeline,
    pub(in crate::renderer) vector_pipeline: &'a mut VectorPipeline,
    pub(in crate::renderer) vector_tessellator: &'a mut VectorTessellator,
    pub(in crate::renderer) svg_raster_cache: &'a mut SvgRasterCache,
    pub(in crate::renderer) shadow_pipeline: &'a mut ShadowPipeline,
    pub(in crate::renderer) stencil: &'a mut StencilState,
    pub(in crate::renderer) text_measurer: &'a mut TextMeasurer,
}
