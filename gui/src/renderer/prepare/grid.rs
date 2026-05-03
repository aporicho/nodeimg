use super::super::command::AffineGridRequest;
use super::super::pipeline::grid::GridVertex;
use super::transform::transform_position;
use super::{DrawOp, PreparedFrame};

pub(super) fn append_grid_request(req: &AffineGridRequest, frame: &mut PreparedFrame) {
    let paint = req.paint;
    if paint.rect.w <= 0.0 || paint.rect.h <= 0.0 || paint.spacing <= 0.0 || paint.dot_size <= 0.0 {
        return;
    }

    let index_start = frame.grid_indices.len() as u32;
    let vertex_offset = frame.grid_vertices.len() as u32;
    let color = paint.dot_color.to_array();
    let radius = paint.dot_size;
    let r = radius + 1.0;
    let left = paint.rect.x - r;
    let top = paint.rect.y - r;
    let right = paint.rect.x + paint.rect.w + r;
    let bottom = paint.rect.y + paint.rect.h + r;
    let pattern_left = -r;
    let pattern_top = -r;
    let pattern_right = paint.rect.w + r;
    let pattern_bottom = paint.rect.h + r;

    let corners = [
        (left, top, pattern_left, pattern_top),
        (right, top, pattern_right, pattern_top),
        (right, bottom, pattern_right, pattern_bottom),
        (left, bottom, pattern_left, pattern_bottom),
    ];

    frame
        .grid_vertices
        .extend(corners.into_iter().map(|(x, y, px, py)| GridVertex {
            position: transform_position(req.transform, [x, y]),
            pattern_pos: [px, py],
            spacing: paint.spacing,
            radius,
            color,
        }));
    frame.grid_indices.extend_from_slice(&[
        vertex_offset,
        vertex_offset + 1,
        vertex_offset + 2,
        vertex_offset,
        vertex_offset + 2,
        vertex_offset + 3,
    ]);
    frame.stats.grid_commands += 1;
    frame.ops.push(DrawOp::Grid {
        index_start,
        index_count: 6,
    });
}
