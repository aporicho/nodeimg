use super::super::command::AffinePathRequest;
use super::super::path::PathRequest;
use super::super::vector_tessellator::VectorTessellator;
use super::{DrawOp, PreparedFrame};

pub(super) fn flush_vector_batch(
    batch: &mut Vec<&AffinePathRequest>,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    if batch.is_empty() {
        return;
    }

    let index_start = frame.vector_indices.len() as u32;
    let mut index_count = 0;
    for req in batch.iter() {
        let path = PathRequest {
            data: req.data.clone(),
            style: req.style,
        };
        index_count += vector_tessellator.append_path_transformed(
            &path,
            req.transform,
            &mut frame.vector_vertices,
            &mut frame.vector_indices,
        );
    }
    batch.clear();

    if index_count > 0 {
        frame.ops.push(DrawOp::Vector {
            index_start,
            index_count,
        });
    }
}
