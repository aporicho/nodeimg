use std::ops::Range;
use winit::dpi::PhysicalSize;

use crate::geometry::{Affine2D, Point, Rect};

use super::buffer::SharedViewport;
use super::command::AffineTextRequest;
use super::command::BackendCommand;
use super::core::MSAA_SAMPLE_COUNT;
use super::offscreen::{transformed_rect_pixel_size, OffscreenTarget};
use super::pipeline::blit::BlitPipeline;
use super::pipeline::circle::CirclePipeline;
use super::pipeline::grid::GridPipeline;
use super::pipeline::image::{ImagePipeline, PreparedImageDraw};
use super::pipeline::quad::QuadPipeline;
use super::pipeline::shadow::{ShadowPipeline, ShadowRequest};
use super::pipeline::stencil::StencilState;
use super::pipeline::text::TextPipeline;
use super::pipeline::vector::VectorPipeline;
use super::prepare::{prepare_frame, DrawOp};
use super::scene_prepare::RendererPrepareStats;
use super::svg::{SvgRasterCache, SvgRasterRequest};
use super::text_measurer::TextMeasurer;
use super::vector_tessellator::VectorTessellator;
use super::{resolve_image_draw, ImageStyle, TextureSize};

pub(super) struct DispatchFrame<'a> {
    pub(super) frame_view: &'a wgpu::TextureView,
    pub(super) msaa_view: &'a wgpu::TextureView,
    pub(super) resolve_view: &'a wgpu::TextureView,
    pub(super) internal_size: PhysicalSize<u32>,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
    pub(super) format: wgpu::TextureFormat,
    pub(super) clear_color: super::types::Color,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

pub(super) struct DispatchPipelines<'a> {
    pub(super) blit: &'a BlitPipeline,
    pub(super) shared_viewport: &'a mut SharedViewport,
    pub(super) quad_pipeline: &'a mut QuadPipeline,
    pub(super) text_pipeline: &'a mut TextPipeline,
    pub(super) image_pipeline: &'a mut ImagePipeline,
    pub(super) circle_pipeline: &'a mut CirclePipeline,
    pub(super) grid_pipeline: &'a mut GridPipeline,
    pub(super) vector_pipeline: &'a mut VectorPipeline,
    pub(super) vector_tessellator: &'a mut VectorTessellator,
    pub(super) svg_raster_cache: &'a mut SvgRasterCache,
    pub(super) shadow_pipeline: &'a mut ShadowPipeline,
    pub(super) stencil: &'a mut StencilState,
    pub(super) text_measurer: &'a mut TextMeasurer,
}

pub(super) fn dispatch(
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

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("renderer"),
    });

    let mut prepared = prepare_frame(commands, vector_tessellator);
    let mut runtime_textures = Vec::new();
    text_pipeline.begin_frame();
    text_measurer.mark_all_unused();
    resolve_deferred_ops(ResolveDeferredOps {
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

    upload_stats.add(quad_pipeline.upload(
        device,
        queue,
        &prepared.quad_vertices,
        &prepared.quad_indices,
    ));
    upload_stats.add(circle_pipeline.upload(
        device,
        queue,
        &prepared.circle_vertices,
        &prepared.circle_indices,
    ));
    upload_stats.add(grid_pipeline.upload(
        device,
        queue,
        &prepared.grid_vertices,
        &prepared.grid_indices,
    ));
    upload_stats.add(vector_pipeline.upload(
        device,
        queue,
        &prepared.vector_vertices,
        &prepared.vector_indices,
    ));
    upload_stats.add(stencil.upload(
        device,
        queue,
        &prepared.stencil_vertices,
        &prepared.stencil_indices,
    ));
    prepared.stats.upload_bytes = upload_stats.bytes;
    prepared.stats.upload_buffer_grows = upload_stats.buffer_grows;

    quad_pipeline.update_bind_group(device, viewport_buf);
    circle_pipeline.update_bind_group(device, viewport_buf);
    grid_pipeline.update_bind_group(device, viewport_buf);
    vector_pipeline.update_bind_group(device, viewport_buf);
    stencil.update_bind_group(device, viewport_buf);

    let has_quads = !prepared.quad_vertices.is_empty();
    let has_grids = !prepared.grid_vertices.is_empty();
    let has_stencils = !prepared.stencil_vertices.is_empty();
    let render_steps = plan_render_steps(&prepared.ops);
    prepared.stats.render_passes = render_steps.len();

    {
        let total_steps = render_steps.len();
        let mut first_pass = true;
        for (step_index, step) in render_steps.iter().enumerate() {
            let resolve_target =
                should_resolve_step(step_index, total_steps).then_some(resolve_view);
            match step {
                RenderStep::Ops {
                    range,
                    starting_clip_depth,
                } => {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("main_ops"),
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

                    let mut last_bound = PipelineKind::None;
                    let mut clip_depth = *starting_clip_depth;

                    if has_quads {
                        quad_pipeline.bind(&mut pass);
                    }

                    for op in &prepared.ops[range.clone()] {
                        match op {
                            DrawOp::Quad {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Quad {
                                    quad_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Quad;
                                }
                                pass.set_stencil_reference(clip_depth);
                                QuadPipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Circle {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Circle {
                                    circle_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Circle;
                                }
                                pass.set_stencil_reference(clip_depth);
                                CirclePipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Grid {
                                index_start,
                                index_count,
                            } => {
                                if has_grids && last_bound != PipelineKind::Grid {
                                    grid_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Grid;
                                }
                                pass.set_stencil_reference(clip_depth);
                                GridPipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Vector {
                                index_start,
                                index_count,
                            } => {
                                if last_bound != PipelineKind::Vector {
                                    vector_pipeline.bind(&mut pass);
                                    last_bound = PipelineKind::Vector;
                                }
                                pass.set_stencil_reference(clip_depth);
                                VectorPipeline::draw_batch(&mut pass, *index_start, *index_count);
                            }
                            DrawOp::Shadow(req) => {
                                last_bound = PipelineKind::Other;
                                let shadow_req = shadow_request(req);
                                if let Some((view, draw)) =
                                    shadow_pipeline.prepared_image(&shadow_req, req.transform)
                                {
                                    pass.set_stencil_reference(clip_depth);
                                    image_pipeline.draw(
                                        &mut pass,
                                        device,
                                        &view,
                                        draw,
                                        viewport_buf,
                                    );
                                }
                            }
                            DrawOp::Image { view, draw } => {
                                last_bound = PipelineKind::Other;
                                pass.set_stencil_reference(clip_depth);
                                image_pipeline.draw(&mut pass, device, view, *draw, viewport_buf);
                            }
                            DrawOp::SvgRaster(_) => {
                                unreachable!("svg raster ops are resolved before render")
                            }
                            DrawOp::Text { .. } => {
                                unreachable!("text ops are split into dedicated render steps")
                            }
                            DrawOp::AffineText(_) => {
                                unreachable!("affine text ops are resolved before render")
                            }
                            DrawOp::Noop => {}
                            DrawOp::StencilWrite {
                                index_start,
                                index_count,
                            } => {
                                if has_stencils && last_bound != PipelineKind::Stencil {
                                    stencil.bind_stencil(&mut pass);
                                    last_bound = PipelineKind::Stencil;
                                }
                                stencil.draw_write(
                                    &mut pass,
                                    clip_depth,
                                    *index_start,
                                    *index_count,
                                );
                                clip_depth += 1;
                            }
                            DrawOp::StencilClear {
                                index_start,
                                index_count,
                            } => {
                                if has_stencils && last_bound != PipelineKind::Stencil {
                                    stencil.bind_stencil(&mut pass);
                                    last_bound = PipelineKind::Stencil;
                                }
                                stencil.draw_clear(
                                    &mut pass,
                                    clip_depth,
                                    *index_start,
                                    *index_count,
                                );
                                clip_depth = clip_depth.saturating_sub(1);
                            }
                        }
                    }
                }
                RenderStep::TextBatch {
                    indices,
                    clip_depth,
                } => {
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

                    pass.set_stencil_reference(*clip_depth);
                    text_pipeline.render_batch(batch_index, &mut pass);
                }
            }
            first_pass = false;
        }
    }

    text_measurer.evict_unused();

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
    prepared.stats
}

struct ResolveDeferredOps<'a> {
    ops: &'a mut [DrawOp],
    encoder: &'a mut wgpu::CommandEncoder,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    format: wgpu::TextureFormat,
    scale_factor: f64,
    render_scale: f32,
    text_pipeline: &'a mut TextPipeline,
    svg_raster_cache: &'a mut SvgRasterCache,
    shadow_pipeline: &'a mut ShadowPipeline,
    text_measurer: &'a mut TextMeasurer,
    runtime_textures: &'a mut Vec<wgpu::Texture>,
}

fn resolve_deferred_ops(ctx: ResolveDeferredOps<'_>) {
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

    for op in ops {
        match op {
            DrawOp::Shadow(req) => {
                let shadow_req = shadow_request(req);
                shadow_pipeline.prepare(encoder, device, &shadow_req);
            }
            DrawOp::AffineText(req) => {
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
                        *op = DrawOp::Image { view, draw };
                    }
                    None => {
                        *op = DrawOp::Noop;
                    }
                }
            }
            DrawOp::SvgRaster(req) => {
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
                        let draw =
                            resolve_image_draw(req.rect, resource.size, ImageStyle::default());
                        *op = DrawOp::Image {
                            view: resource.view,
                            draw: PreparedImageDraw::from_resolved(draw, req.transform),
                        };
                    }
                    Err(err) => {
                        tracing::warn!(
                            "SVG icon '{}' could not be rasterized: {:?}",
                            req.source.key().id(),
                            err
                        );
                        *op = DrawOp::Noop;
                    }
                }
            }
            _ => {}
        }
    }
}

struct RasterText<'a> {
    req: &'a AffineTextRequest,
    encoder: &'a mut wgpu::CommandEncoder,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
    format: wgpu::TextureFormat,
    scale_factor: f64,
    render_scale: f32,
    text_pipeline: &'a mut TextPipeline,
    text_measurer: &'a mut TextMeasurer,
    runtime_textures: &'a mut Vec<wgpu::Texture>,
}

fn rasterize_affine_text(
    ctx: RasterText<'_>,
) -> Option<(std::sync::Arc<wgpu::TextureView>, PreparedImageDraw)> {
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
    let text_req = super::pipeline::text::TextRequest {
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

fn shadow_request(req: &super::command::AffineShadowRequest) -> ShadowRequest {
    ShadowRequest {
        rect: req.rect,
        radius: req.radius,
        shadow: req.shadow,
    }
}

fn svg_raster_pixel_size(
    transform: Affine2D,
    rect: super::types::Rect,
    scale_factor: f64,
    render_scale: f32,
) -> TextureSize {
    let size = transformed_rect_pixel_size(transform, rect, scale_factor, render_scale);
    TextureSize::new(size.width.max(1), size.height.max(1))
}

#[derive(Debug, PartialEq)]
enum RenderStep {
    Ops {
        range: Range<usize>,
        starting_clip_depth: u32,
    },
    TextBatch {
        indices: Vec<usize>,
        clip_depth: u32,
    },
}

fn plan_render_steps(ops: &[DrawOp]) -> Vec<RenderStep> {
    let mut steps = Vec::new();
    let mut clip_depth = 0u32;
    let mut ops_start: Option<usize> = None;
    let mut ops_clip_depth = 0u32;

    for (i, op) in ops.iter().enumerate() {
        match op {
            DrawOp::Text { index } => {
                if let Some(start) = ops_start.take() {
                    steps.push(RenderStep::Ops {
                        range: start..i,
                        starting_clip_depth: ops_clip_depth,
                    });
                }
                match steps.last_mut() {
                    Some(RenderStep::TextBatch {
                        indices,
                        clip_depth: existing_clip_depth,
                    }) if *existing_clip_depth == clip_depth => {
                        indices.push(*index);
                    }
                    _ => steps.push(RenderStep::TextBatch {
                        indices: vec![*index],
                        clip_depth,
                    }),
                }
            }
            DrawOp::StencilWrite { .. } => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
                clip_depth += 1;
            }
            DrawOp::StencilClear { .. } => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
                clip_depth = clip_depth.saturating_sub(1);
            }
            _ => {
                if ops_start.is_none() {
                    ops_start = Some(i);
                    ops_clip_depth = clip_depth;
                }
            }
        }
    }

    if let Some(start) = ops_start {
        steps.push(RenderStep::Ops {
            range: start..ops.len(),
            starting_clip_depth: ops_clip_depth,
        });
    }

    if steps.is_empty() {
        steps.push(RenderStep::Ops {
            range: 0..0,
            starting_clip_depth: 0,
        });
    }

    steps
}

fn color_load_op(first_pass: bool, clear_color: super::types::Color) -> wgpu::LoadOp<wgpu::Color> {
    if first_pass {
        wgpu::LoadOp::Clear(wgpu::Color {
            r: clear_color.r as f64,
            g: clear_color.g as f64,
            b: clear_color.b as f64,
            a: clear_color.a as f64,
        })
    } else {
        wgpu::LoadOp::Load
    }
}

fn color_store_op() -> wgpu::StoreOp {
    wgpu::StoreOp::Store
}

fn should_resolve_step(step_index: usize, total_steps: usize) -> bool {
    total_steps > 0 && step_index + 1 == total_steps
}

fn stencil_load_op(first_pass: bool) -> wgpu::LoadOp<u32> {
    if first_pass {
        wgpu::LoadOp::Clear(0)
    } else {
        wgpu::LoadOp::Load
    }
}

#[derive(PartialEq)]
enum PipelineKind {
    None,
    Quad,
    Circle,
    Grid,
    Vector,
    Stencil,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    #[test]
    fn plan_render_steps_tracks_clip_depth_per_text_op() {
        let steps = plan_render_steps(&[
            DrawOp::StencilWrite {
                index_start: 0,
                index_count: 6,
            },
            DrawOp::Text { index: 0 },
            DrawOp::StencilClear {
                index_start: 6,
                index_count: 6,
            },
            DrawOp::Text { index: 1 },
        ]);

        assert_eq!(
            steps,
            vec![
                RenderStep::Ops {
                    range: Range { start: 0, end: 1 },
                    starting_clip_depth: 0,
                },
                RenderStep::TextBatch {
                    indices: vec![0],
                    clip_depth: 1,
                },
                RenderStep::Ops {
                    range: Range { start: 2, end: 3 },
                    starting_clip_depth: 1,
                },
                RenderStep::TextBatch {
                    indices: vec![1],
                    clip_depth: 0,
                },
            ]
        );
    }

    #[test]
    fn plan_render_steps_batches_adjacent_text_ops_at_same_clip_depth() {
        let steps = plan_render_steps(&[
            DrawOp::Text { index: 0 },
            DrawOp::Text { index: 1 },
            DrawOp::StencilWrite {
                index_start: 0,
                index_count: 6,
            },
            DrawOp::Text { index: 2 },
            DrawOp::Text { index: 3 },
        ]);

        assert_eq!(
            steps,
            vec![
                RenderStep::TextBatch {
                    indices: vec![0, 1],
                    clip_depth: 0,
                },
                RenderStep::Ops {
                    range: Range { start: 2, end: 3 },
                    starting_clip_depth: 0,
                },
                RenderStep::TextBatch {
                    indices: vec![2, 3],
                    clip_depth: 1,
                },
            ]
        );
    }

    #[test]
    fn plan_render_steps_emits_clear_step_for_empty_ops() {
        assert_eq!(
            plan_render_steps(&[]),
            vec![RenderStep::Ops {
                range: Range { start: 0, end: 0 },
                starting_clip_depth: 0,
            }]
        );
    }

    #[test]
    fn resolves_only_on_final_render_step() {
        assert!(!should_resolve_step(0, 3));
        assert!(!should_resolve_step(1, 3));
        assert!(should_resolve_step(2, 3));
    }

    #[test]
    fn single_render_step_still_resolves() {
        assert!(should_resolve_step(0, 1));
    }
}
