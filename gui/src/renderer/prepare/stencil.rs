use lyon::path::Path as LyonPath;
use lyon::tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers};

use crate::geometry::Affine2D;
use crate::paint::ClipShape;

use super::super::command::AffineClipRequest;
use super::super::path_geometry::{
    build_lyon_path, build_rounded_rect_path, lyon_fill_rule, DEFAULT_CORNER_SMOOTHING,
};
use super::super::pipeline::stencil::StencilVertex;
use super::transform::transform_position;
use super::{DrawOp, PreparedFrame};

pub(super) fn tessellate_stencil(
    frame: &mut PreparedFrame,
    req: &AffineClipRequest,
    is_write: bool,
) {
    match &req.shape {
        ClipShape::Rect(rect) => {
            let path = build_rounded_rect_path(*rect, [0.0; 4], DEFAULT_CORNER_SMOOTHING);
            append_stencil_path(
                frame,
                &path,
                FillOptions::default(),
                req.transform,
                is_write,
            );
        }
        ClipShape::RoundedRect { rect, radius } => {
            let path = build_rounded_rect_path(*rect, *radius, DEFAULT_CORNER_SMOOTHING);
            append_stencil_path(
                frame,
                &path,
                FillOptions::default(),
                req.transform,
                is_write,
            );
        }
        ClipShape::Path { data, fill_rule } => {
            let path = build_lyon_path(data);
            append_stencil_path(
                frame,
                &path,
                FillOptions::default().with_fill_rule(lyon_fill_rule(*fill_rule)),
                req.transform,
                is_write,
            );
        }
    }
}

fn append_stencil_path(
    frame: &mut PreparedFrame,
    path: &LyonPath,
    fill_options: FillOptions,
    transform: Affine2D,
    is_write: bool,
) {
    let mut geometry: VertexBuffers<StencilVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    tessellator
        .tessellate_path(
            path,
            &fill_options,
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| StencilVertex {
                position: transform_position(transform, vertex.position().to_array()),
            }),
        )
        .expect("failed to tessellate stencil shape");

    if geometry.indices.is_empty() {
        return;
    }

    let index_start = frame.stencil_indices.len() as u32;
    let vertex_offset = frame.stencil_vertices.len() as u32;

    frame
        .stencil_indices
        .extend(geometry.indices.iter().map(|idx| idx + vertex_offset));
    frame.stencil_vertices.extend_from_slice(&geometry.vertices);

    let index_count = geometry.indices.len() as u32;

    if is_write {
        frame.ops.push(DrawOp::StencilWrite {
            index_start,
            index_count,
        });
    } else {
        frame.ops.push(DrawOp::StencilClear {
            index_start,
            index_count,
        });
    }
}
