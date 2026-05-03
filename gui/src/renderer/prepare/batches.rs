use super::super::command::{AffinePathRequest, AffineRectRequest};
use super::super::pipeline::circle::CircleRequest;
use super::super::vector_tessellator::VectorTessellator;
use super::circle::flush_circle_batch;
use super::quad::flush_quad_batch;
use super::vector::flush_vector_batch;
use super::PreparedFrame;

pub(super) fn flush_all_batches(
    quad_batch: &mut Vec<&AffineRectRequest>,
    circle_batch: &mut Vec<CircleRequest>,
    vector_batch: &mut Vec<&AffinePathRequest>,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    flush_quad_batch(quad_batch, frame);
    flush_circle_batch(circle_batch, frame);
    flush_vector_batch(vector_batch, frame, vector_tessellator);
}

pub(super) fn flush_circle_and_vector(
    circle_batch: &mut Vec<CircleRequest>,
    vector_batch: &mut Vec<&AffinePathRequest>,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    flush_circle_batch(circle_batch, frame);
    flush_vector_batch(vector_batch, frame, vector_tessellator);
}
