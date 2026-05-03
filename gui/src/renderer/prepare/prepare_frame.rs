use super::super::affine::similarity_scale;
use super::super::command::{
    AffineClipRequest, AffinePathRequest, AffineRectRequest, BackendCommand,
};
use super::super::image::resolve_image_draw;
use super::super::pipeline::circle::CircleRequest;
use super::super::pipeline::image::PreparedImageDraw;
use super::super::vector_tessellator::VectorTessellator;
use super::batches::{flush_all_batches, flush_circle_and_vector};
use super::circle::{append_circle_requests, append_circle_vector_fallback, flush_circle_batch};
use super::grid::append_grid_request;
use super::quad::flush_quad_batch;
use super::stencil::tessellate_stencil;
use super::text::push_text_op;
use super::vector::flush_vector_batch;
use super::{DrawOp, PreparedFrame};

pub(in crate::renderer) fn prepare_frame(
    commands: &[BackendCommand],
    vector_tessellator: &mut VectorTessellator,
) -> PreparedFrame {
    let mut frame = PreparedFrame::new(commands.len());

    let mut quad_batch: Vec<&AffineRectRequest> = Vec::new();
    let mut circle_batch: Vec<CircleRequest> = Vec::new();
    let mut vector_batch: Vec<&AffinePathRequest> = Vec::new();
    let mut clip_stack: Vec<AffineClipRequest> = Vec::new();

    for cmd in commands {
        match cmd {
            BackendCommand::Shadow(req) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                frame.ops.push(DrawOp::Shadow(*req));
            }
            BackendCommand::Rect(req) => {
                flush_circle_and_vector(
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                quad_batch.push(req);
            }
            BackendCommand::Circle(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                if let Some(scale) = similarity_scale(req.transform) {
                    append_circle_requests(req, scale, &mut circle_batch);
                } else {
                    flush_circle_batch(&mut circle_batch, &mut frame);
                    append_circle_vector_fallback(req, &mut frame, vector_tessellator);
                }
            }
            BackendCommand::Grid(req) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                append_grid_request(req, &mut frame);
            }
            BackendCommand::Text(req) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                push_text_op(req, &mut frame);
            }
            BackendCommand::Image(req) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                let draw = resolve_image_draw(req.rect, req.size, req.style);
                frame.ops.push(DrawOp::Image {
                    view: req.view.clone(),
                    draw: PreparedImageDraw::from_resolved(draw, req.transform),
                });
            }
            BackendCommand::SvgRaster(draw) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                frame.ops.push(DrawOp::SvgRaster(draw.clone()));
            }
            BackendCommand::Path(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                vector_batch.push(req);
            }
            BackendCommand::PushClip(req) => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                clip_stack.push(req.clone());
                tessellate_stencil(&mut frame, req, true);
            }
            BackendCommand::PopClip => {
                flush_all_batches(
                    &mut quad_batch,
                    &mut circle_batch,
                    &mut vector_batch,
                    &mut frame,
                    vector_tessellator,
                );
                if let Some(req) = clip_stack.pop() {
                    tessellate_stencil(&mut frame, &req, false);
                }
            }
        }
    }

    flush_all_batches(
        &mut quad_batch,
        &mut circle_batch,
        &mut vector_batch,
        &mut frame,
        vector_tessellator,
    );

    let geometry_stats = vector_tessellator.take_frame_stats();
    frame.stats.geometry_cache_hits = geometry_stats.hits;
    frame.stats.geometry_cache_misses = geometry_stats.misses;
    frame.stats.tessellated_vertices = geometry_stats.tessellated_vertices;

    frame
}
