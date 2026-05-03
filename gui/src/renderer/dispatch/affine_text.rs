use std::sync::Arc;

use crate::geometry::{Point, Rect};

use super::super::command::AffineTextRequest;
use super::super::core::MSAA_SAMPLE_COUNT;
use super::super::offscreen::{transformed_rect_pixel_size, OffscreenTarget};
use super::super::pipeline::image::PreparedImageDraw;
use super::super::pipeline::text::{TextPipeline, TextRequest};
use super::super::text_measurer::TextMeasurer;

pub(super) struct RasterText<'a> {
    pub(super) req: &'a AffineTextRequest,
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
    pub(super) format: wgpu::TextureFormat,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
    pub(super) text_pipeline: &'a mut TextPipeline,
    pub(super) text_measurer: &'a mut TextMeasurer,
    pub(super) runtime_textures: &'a mut Vec<wgpu::Texture>,
}

pub(super) fn rasterize_affine_text(
    ctx: RasterText<'_>,
) -> Option<(Arc<wgpu::TextureView>, PreparedImageDraw)> {
    let RasterText {
        req,
        encoder,
        device,
        queue,
        format,
        scale_factor,
        render_scale,
        text_pipeline,
        text_measurer,
        runtime_textures,
    } = ctx;
    let bounds = text_local_bounds(req, text_measurer)?;
    let pixel_size = transformed_rect_pixel_size(req.transform, bounds, scale_factor, render_scale);
    let raster_scale = ((pixel_size.width as f32 / bounds.w.abs().max(1.0))
        .max(pixel_size.height as f32 / bounds.h.abs().max(1.0)))
    .max(1.0);
    let target = OffscreenTarget::new(
        device,
        format,
        pixel_size,
        MSAA_SAMPLE_COUNT,
        "affine_text_offscreen",
    );
    let text_req = TextRequest {
        pos: Point {
            x: req.pos.x - bounds.x,
            y: req.pos.y - bounds.y,
        },
        text: req.text.clone(),
        style: req.style,
        bounds: Some(Rect {
            x: 0.0,
            y: 0.0,
            w: bounds.w,
            h: bounds.h,
        }),
    };
    let batch_index = text_pipeline.prepare(
        device,
        queue,
        std::slice::from_ref(&text_req),
        target.size,
        raster_scale as f64,
        text_measurer,
    );

    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("affine_text_offscreen"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.msaa_view,
                resolve_target: Some(target.view.as_ref()),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_view,
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            ..Default::default()
        });
        pass.set_stencil_reference(0);
        text_pipeline.render_batch(batch_index, &mut pass);
    }

    let view = target.view.clone();
    let draw = PreparedImageDraw::from_rect(bounds, req.transform);
    target.keep_alive(runtime_textures);
    Some((view, draw))
}

fn text_local_bounds(req: &AffineTextRequest, text_measurer: &mut TextMeasurer) -> Option<Rect> {
    let bounds = if let Some(bounds) = req.bounds {
        bounds
    } else {
        let (w, h) = text_measurer.measure_with_style(&req.text, &req.style);
        Rect {
            x: req.pos.x,
            y: req.pos.y,
            w,
            h,
        }
    };
    (bounds.w.is_finite()
        && bounds.h.is_finite()
        && bounds.w.abs() > f32::EPSILON
        && bounds.h.abs() > f32::EPSILON)
        .then_some(bounds)
}
