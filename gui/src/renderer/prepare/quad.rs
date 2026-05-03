use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use super::super::command::AffineRectRequest;
use super::super::path_geometry::{build_rounded_rect_path, DEFAULT_CORNER_SMOOTHING};
use super::super::pipeline::quad::QuadVertex;
use super::transform::transform_position;
use super::{DrawOp, PreparedFrame};

pub(super) fn flush_quad_batch(batch: &mut Vec<&AffineRectRequest>, frame: &mut PreparedFrame) {
    if batch.is_empty() {
        return;
    }

    let mut geometry: VertexBuffers<QuadVertex, u32> = VertexBuffers::new();
    let mut fill_tessellator = FillTessellator::new();
    let mut stroke_tessellator = StrokeTessellator::new();

    for req in batch.iter() {
        let path = build_rounded_rect_path(req.rect, req.style.radius, DEFAULT_CORNER_SMOOTHING);
        let transform = req.transform;
        let color = req.style.color.to_array();

        fill_tessellator
            .tessellate_path(
                &path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| QuadVertex {
                    position: transform_position(transform, vertex.position().to_array()),
                    color,
                }),
            )
            .expect("failed to tessellate quad fill");

        if let Some(border) = req.style.border {
            if border.width > 0.0 {
                let border_color = border.color.to_array();
                stroke_tessellator
                    .tessellate_path(
                        &path,
                        &StrokeOptions::default().with_line_width(border.width),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                            QuadVertex {
                                position: transform_position(
                                    transform,
                                    vertex.position().to_array(),
                                ),
                                color: border_color,
                            }
                        }),
                    )
                    .expect("failed to tessellate quad stroke");
            }
        }
    }

    batch.clear();

    if geometry.indices.is_empty() {
        return;
    }

    let index_start = frame.quad_indices.len() as u32;
    let vertex_offset = frame.quad_vertices.len() as u32;

    frame
        .quad_indices
        .extend(geometry.indices.iter().map(|idx| idx + vertex_offset));
    frame.quad_vertices.extend_from_slice(&geometry.vertices);

    frame.ops.push(DrawOp::Quad {
        index_start,
        index_count: geometry.indices.len() as u32,
    });
}
