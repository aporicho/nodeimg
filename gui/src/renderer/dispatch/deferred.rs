use crate::diagnostics::render_trace::{self, TARGET_RENDER_GPU};
use crate::geometry::Affine2D;

use super::super::command::AffineShadowRequest;
use super::super::pipeline::image::PreparedImageDraw;
use super::super::pipeline::shadow::{ShadowPipeline, ShadowRequest};
use super::super::pipeline::text::TextPipeline;
use super::super::prepare::DrawOp;
use super::super::svg::{SvgRasterCache, SvgRasterRequest};
use super::super::text_measurer::TextMeasurer;
use super::super::{resolve_image_draw, ImageStyle, TextureSize};
use super::affine_text::{rasterize_affine_text, RasterText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct DeferredResolveStats {
    shadows: usize,
    affine_text: usize,
    affine_text_rasterized: usize,
    svg_raster: usize,
    svg_rasterized: usize,
    noop: usize,
}

pub(super) struct ResolveDeferredOps<'a> {
    pub(super) ops: &'a mut [DrawOp],
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) format: wgpu::TextureFormat,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
    pub(super) text_pipeline: &'a mut TextPipeline,
    pub(super) svg_raster_cache: &'a mut SvgRasterCache,
    pub(super) shadow_pipeline: &'a mut ShadowPipeline,
    pub(super) text_measurer: &'a mut TextMeasurer,
    pub(super) runtime_textures: &'a mut Vec<wgpu::Texture>,
}

pub(super) fn resolve_deferred_ops(ctx: ResolveDeferredOps<'_>) -> DeferredResolveStats {
    let ResolveDeferredOps {
        ops,
        encoder,
        device,
        queue,
        format,
        scale_factor,
        render_scale,
        text_pipeline,
        svg_raster_cache,
        shadow_pipeline,
        text_measurer,
        runtime_textures,
    } = ctx;

    let mut stats = DeferredResolveStats::default();
    for op in ops {
        match op {
            DrawOp::Shadow(req) => {
                stats.shadows += 1;
                let shadow_req = shadow_request(req);
                shadow_pipeline.prepare(encoder, device, &shadow_req);
            }
            DrawOp::AffineText(req) => {
                stats.affine_text += 1;
                match rasterize_affine_text(RasterText {
                    req,
                    encoder: &mut *encoder,
                    device,
                    queue,
                    format,
                    scale_factor,
                    render_scale,
                    text_pipeline: &mut *text_pipeline,
                    text_measurer: &mut *text_measurer,
                    runtime_textures: &mut *runtime_textures,
                }) {
                    Some((view, draw)) => {
                        stats.affine_text_rasterized += 1;
                        *op = DrawOp::Image { view, draw };
                    }
                    None => {
                        stats.noop += 1;
                        *op = DrawOp::Noop;
                    }
                }
            }
            DrawOp::SvgRaster(req) => {
                stats.svg_raster += 1;
                let pixel_size =
                    svg_raster_pixel_size(req.transform, req.rect, scale_factor, render_scale);
                let request = SvgRasterRequest::new(
                    req.source.clone(),
                    pixel_size,
                    req.style.raster_color(),
                    req.style.fit,
                );
                match svg_raster_cache.get_or_rasterize(device, queue, request) {
                    Ok(resource) => {
                        stats.svg_rasterized += 1;
                        let draw =
                            resolve_image_draw(req.rect, resource.size, ImageStyle::default());
                        *op = DrawOp::Image {
                            view: resource.view,
                            draw: PreparedImageDraw::from_resolved(draw, req.transform),
                        };
                    }
                    Err(err) => {
                        tracing::warn!(
                            target: TARGET_RENDER_GPU,
                            frame_id = render_trace::current_render_trace_frame().id,
                            "SVG icon '{}' could not be rasterized: {:?}",
                            req.source.key().id(),
                            err
                        );
                        stats.noop += 1;
                        *op = DrawOp::Noop;
                    }
                }
            }
            _ => {}
        }
    }
    stats
}

pub(super) fn shadow_request(req: &AffineShadowRequest) -> ShadowRequest {
    ShadowRequest {
        rect: req.rect,
        radius: req.radius,
        shadow: req.shadow,
    }
}

fn svg_raster_pixel_size(
    transform: Affine2D,
    rect: super::super::types::Rect,
    scale_factor: f64,
    render_scale: f32,
) -> TextureSize {
    let size = super::super::offscreen::transformed_rect_pixel_size(
        transform,
        rect,
        scale_factor,
        render_scale,
    );
    TextureSize::new(size.width.max(1), size.height.max(1))
}
