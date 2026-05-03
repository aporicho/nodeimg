use crate::geometry::{Affine2D, Point};
use crate::paint::Fill;

use super::super::command::AffineCircleRequest;
use super::super::path::{PathRequest, PathStyle};
use super::super::path_geometry::circle_path_data;
use super::super::pipeline::circle::{CircleRequest, CircleVertex};
use super::super::vector_tessellator::VectorTessellator;
use super::{DrawOp, PreparedFrame};

pub(super) fn append_circle_requests(
    req: &AffineCircleRequest,
    scale: f32,
    batch: &mut Vec<CircleRequest>,
) {
    let center = req.transform.transform_point(req.paint.center);
    let radius = req.paint.radius * scale;
    if let Some(stroke) = req.paint.stroke {
        batch.push(CircleRequest {
            center,
            radius,
            color: stroke.color,
        });
    }
    if let Some(fill) = req.paint.fill {
        let fill_radius = req
            .paint
            .stroke
            .map(|stroke| radius - stroke.width * scale)
            .unwrap_or(radius)
            .max(0.0);
        batch.push(CircleRequest {
            center,
            radius: fill_radius,
            color: fill,
        });
    }
}

pub(super) fn append_circle_vector_fallback(
    req: &AffineCircleRequest,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    let index_start = frame.vector_indices.len() as u32;
    let mut index_count = 0;

    if let Some(stroke) = req.paint.stroke {
        index_count += append_filled_circle_path(
            req.paint.center,
            req.paint.radius,
            Fill::non_zero(stroke.color),
            req.transform,
            frame,
            vector_tessellator,
        );
    }
    if let Some(fill) = req.paint.fill {
        let fill_radius = req
            .paint
            .stroke
            .map(|stroke| req.paint.radius - stroke.width)
            .unwrap_or(req.paint.radius)
            .max(0.0);
        index_count += append_filled_circle_path(
            req.paint.center,
            fill_radius,
            Fill::non_zero(fill),
            req.transform,
            frame,
            vector_tessellator,
        );
    }

    if index_count > 0 {
        frame.ops.push(DrawOp::Vector {
            index_start,
            index_count,
        });
    }
}

fn append_filled_circle_path(
    center: Point,
    radius: f32,
    fill: Fill,
    transform: Affine2D,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) -> u32 {
    let req = PathRequest {
        data: circle_path_data(center, radius),
        style: PathStyle::fill(fill),
    };
    vector_tessellator.append_path_transformed(
        &req,
        transform,
        &mut frame.vector_vertices,
        &mut frame.vector_indices,
    )
}

pub(super) fn flush_circle_batch(batch: &mut Vec<CircleRequest>, frame: &mut PreparedFrame) {
    if batch.is_empty() {
        return;
    }

    let index_start = frame.circle_indices.len() as u32;
    let mut index_count: u32 = 0;

    for req in batch.iter() {
        let cx = req.center.x;
        let cy = req.center.y;
        let r = req.radius;
        let color = req.color.to_array();
        let center = [cx, cy];
        let ext = r + 1.0;

        let vertex_offset = frame.circle_vertices.len() as u32;

        frame.circle_vertices.extend_from_slice(&[
            CircleVertex {
                position: [cx - ext, cy - ext],
                center,
                radius: r,
                color,
            },
            CircleVertex {
                position: [cx + ext, cy - ext],
                center,
                radius: r,
                color,
            },
            CircleVertex {
                position: [cx + ext, cy + ext],
                center,
                radius: r,
                color,
            },
            CircleVertex {
                position: [cx - ext, cy + ext],
                center,
                radius: r,
                color,
            },
        ]);

        frame.circle_indices.extend_from_slice(&[
            vertex_offset,
            vertex_offset + 1,
            vertex_offset + 2,
            vertex_offset,
            vertex_offset + 2,
            vertex_offset + 3,
        ]);

        index_count += 6;
    }

    batch.clear();

    frame.ops.push(DrawOp::Circle {
        index_start,
        index_count,
    });
}
