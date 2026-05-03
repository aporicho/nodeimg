use std::ops::Range;

use super::super::pipeline::circle::CirclePipeline;
use super::super::pipeline::grid::GridPipeline;
use super::super::pipeline::image::ImagePipeline;
use super::super::pipeline::quad::QuadPipeline;
use super::super::pipeline::shadow::ShadowPipeline;
use super::super::pipeline::stencil::StencilState;
use super::super::pipeline::vector::VectorPipeline;
use super::super::prepare::{DrawOp, PreparedFrame};
use super::super::types::Color;
use super::deferred::shadow_request;
use super::load_ops::{color_load_op, color_store_op, stencil_load_op};

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

pub(super) struct OpsPassContext<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) device: &'a wgpu::Device,
    pub(super) prepared: &'a PreparedFrame,
    pub(super) range: Range<usize>,
    pub(super) starting_clip_depth: u32,
    pub(super) msaa_view: &'a wgpu::TextureView,
    pub(super) resolve_target: Option<&'a wgpu::TextureView>,
    pub(super) clear_color: Color,
    pub(super) first_pass: bool,
    pub(super) viewport_buf: &'a wgpu::Buffer,
    pub(super) has_quads: bool,
    pub(super) has_grids: bool,
    pub(super) has_stencils: bool,
    pub(super) quad_pipeline: &'a mut QuadPipeline,
    pub(super) image_pipeline: &'a mut ImagePipeline,
    pub(super) circle_pipeline: &'a mut CirclePipeline,
    pub(super) grid_pipeline: &'a mut GridPipeline,
    pub(super) vector_pipeline: &'a mut VectorPipeline,
    pub(super) shadow_pipeline: &'a mut ShadowPipeline,
    pub(super) stencil: &'a mut StencilState,
}

pub(super) fn render_ops_pass(ctx: OpsPassContext<'_>) {
    let OpsPassContext {
        encoder,
        device,
        prepared,
        range,
        starting_clip_depth,
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
    } = ctx;

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
    let mut clip_depth = starting_clip_depth;

    if has_quads {
        quad_pipeline.bind(&mut pass);
    }

    for op in &prepared.ops[range] {
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
                    image_pipeline.draw(&mut pass, device, &view, draw, viewport_buf);
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
                stencil.draw_write(&mut pass, clip_depth, *index_start, *index_count);
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
                stencil.draw_clear(&mut pass, clip_depth, *index_start, *index_count);
                clip_depth = clip_depth.saturating_sub(1);
            }
        }
    }
}
