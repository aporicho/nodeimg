use std::sync::Arc;

use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use super::command::DrawCommand;
use super::image::{resolve_image_draw, ResolvedImageDraw};
use super::path::PathRequest;
use super::pipeline::circle::{CircleRequest, CircleVertex};
use super::pipeline::quad::{
    build_rounded_rect_path, QuadRequest, QuadVertex, DEFAULT_CORNER_SMOOTHING,
};
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::stencil::StencilVertex;
use super::pipeline::text::TextRequest;
use super::pipeline::vector::VectorVertex;
use super::svg::SvgRasterDraw;
use super::types::Rect;
use super::vector_tessellator::VectorTessellator;

// ── 绘制操作 ──

pub enum DrawOp {
    Quad {
        index_start: u32,
        index_count: u32,
    },
    Circle {
        index_start: u32,
        index_count: u32,
    },
    Vector {
        index_start: u32,
        index_count: u32,
    },
    Shadow(ShadowRequest),
    Image {
        view: Arc<wgpu::TextureView>,
        draw: ResolvedImageDraw,
    },
    SvgRaster(SvgRasterDraw),
    Text {
        index: usize,
    },
    StencilWrite {
        index_start: u32,
        index_count: u32,
    },
    StencilClear {
        index_start: u32,
        index_count: u32,
    },
}

// ── 预处理结果 ──

pub struct PreparedFrame {
    pub ops: Vec<DrawOp>,
    pub text_requests: Vec<TextRequest>,

    pub quad_vertices: Vec<QuadVertex>,
    pub quad_indices: Vec<u32>,

    pub vector_vertices: Vec<VectorVertex>,
    pub vector_indices: Vec<u32>,

    pub circle_vertices: Vec<CircleVertex>,
    pub circle_indices: Vec<u32>,

    pub stencil_vertices: Vec<StencilVertex>,
    pub stencil_indices: Vec<u32>,
}

// ── 预处理 ──

pub fn prepare_frame(
    commands: &[DrawCommand],
    vector_tessellator: &mut VectorTessellator,
) -> PreparedFrame {
    let mut frame = PreparedFrame {
        ops: Vec::new(),
        text_requests: Vec::new(),
        quad_vertices: Vec::new(),
        quad_indices: Vec::new(),
        vector_vertices: Vec::new(),
        vector_indices: Vec::new(),
        circle_vertices: Vec::new(),
        circle_indices: Vec::new(),
        stencil_vertices: Vec::new(),
        stencil_indices: Vec::new(),
    };

    let mut quad_batch: Vec<&QuadRequest> = Vec::new();
    let mut circle_batch: Vec<&CircleRequest> = Vec::new();
    let mut vector_batch: Vec<&PathRequest> = Vec::new();
    let mut clip_stack: Vec<(Rect, f32)> = Vec::new();

    for cmd in commands {
        match cmd {
            DrawCommand::Shadow(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                frame.ops.push(DrawOp::Shadow(req.clone()));
            }
            DrawCommand::Rect(req) => {
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                quad_batch.push(req);
            }
            DrawCommand::Circle(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                circle_batch.push(req);
            }
            DrawCommand::Text(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                let index = frame.text_requests.len();
                frame.text_requests.push(TextRequest {
                    pos: req.pos,
                    text: req.text.clone(),
                    style: req.style,
                    bounds: req.bounds,
                });
                frame.ops.push(DrawOp::Text { index });
            }
            DrawCommand::Image {
                rect,
                view,
                size,
                style,
            } => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                frame.ops.push(DrawOp::Image {
                    view: view.clone(),
                    draw: resolve_image_draw(*rect, *size, *style),
                });
            }
            DrawCommand::SvgRaster(draw) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                frame.ops.push(DrawOp::SvgRaster(draw.clone()));
            }
            DrawCommand::Path(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                vector_batch.push(req);
            }
            DrawCommand::PushClip { rect, radius } => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                clip_stack.push((*rect, *radius));
                tessellate_stencil(&mut frame, *rect, *radius, true);
            }
            DrawCommand::PopClip => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);
                if let Some((rect, radius)) = clip_stack.pop() {
                    tessellate_stencil(&mut frame, rect, radius, false);
                }
            }
        }
    }

    flush_quad_batch(&mut quad_batch, &mut frame);
    flush_circle_batch(&mut circle_batch, &mut frame);
    flush_vector_batch(&mut vector_batch, &mut frame, vector_tessellator);

    frame
}

// ── 批次 flush ──

fn flush_quad_batch(batch: &mut Vec<&QuadRequest>, frame: &mut PreparedFrame) {
    if batch.is_empty() {
        return;
    }

    let mut geometry: VertexBuffers<QuadVertex, u32> = VertexBuffers::new();
    let mut fill_tessellator = FillTessellator::new();
    let mut stroke_tessellator = StrokeTessellator::new();

    for req in batch.iter() {
        let path = build_rounded_rect_path(req.rect, req.radius, req.smoothing);
        let color = req.style_color;

        fill_tessellator
            .tessellate_path(
                &path,
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| QuadVertex {
                    position: vertex.position().to_array(),
                    color,
                }),
            )
            .expect("failed to tessellate quad fill");

        if req.border_width > 0.0 {
            if let Some(border_color) = req.border_color {
                stroke_tessellator
                    .tessellate_path(
                        &path,
                        &StrokeOptions::default().with_line_width(req.border_width),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
                            QuadVertex {
                                position: vertex.position().to_array(),
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

    // 偏移索引值
    for idx in &geometry.indices {
        frame.quad_indices.push(idx + vertex_offset);
    }
    frame.quad_vertices.extend_from_slice(&geometry.vertices);

    frame.ops.push(DrawOp::Quad {
        index_start,
        index_count: geometry.indices.len() as u32,
    });
}

fn flush_vector_batch(
    batch: &mut Vec<&PathRequest>,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    if batch.is_empty() {
        return;
    }

    let index_start = frame.vector_indices.len() as u32;
    let mut index_count = 0;
    for req in batch.iter() {
        index_count += vector_tessellator.append_path(
            req,
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

fn flush_circle_batch(batch: &mut Vec<&CircleRequest>, frame: &mut PreparedFrame) {
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

fn tessellate_stencil(frame: &mut PreparedFrame, rect: Rect, radius: f32, is_write: bool) {
    let path = build_rounded_rect_path(rect, [radius; 4], DEFAULT_CORNER_SMOOTHING);

    let mut geometry: VertexBuffers<StencilVertex, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    tessellator
        .tessellate_path(
            &path,
            &FillOptions::default(),
            &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| StencilVertex {
                position: vertex.position().to_array(),
            }),
        )
        .expect("failed to tessellate stencil shape");

    if geometry.indices.is_empty() {
        return;
    }

    let index_start = frame.stencil_indices.len() as u32;
    let vertex_offset = frame.stencil_vertices.len() as u32;

    for idx in &geometry.indices {
        frame.stencil_indices.push(idx + vertex_offset);
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{
        Color, Fill, PathData, PathRequest, PathStyle, Point, Rect, RectStyle, Stroke, TextStyle,
    };

    fn rect_command(x: f32) -> DrawCommand {
        DrawCommand::Rect(QuadRequest::from_style(
            Rect {
                x,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            &RectStyle {
                color: Color::WHITE,
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
        ))
    }

    fn text_command(text: &str) -> DrawCommand {
        DrawCommand::Text(TextRequest {
            pos: Point { x: 0.0, y: 0.0 },
            text: text.to_string(),
            style: TextStyle::new(Color::WHITE, 12.0),
            bounds: None,
        })
    }

    fn path_command(style: PathStyle) -> DrawCommand {
        DrawCommand::Path(PathRequest {
            data: PathData::new()
                .move_to(Point { x: 0.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 10.0 })
                .close(),
            style,
        })
    }

    #[test]
    fn prepare_frame_preserves_text_indices_in_order() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[
                rect_command(0.0),
                text_command("first"),
                rect_command(20.0),
                text_command("second"),
            ],
            &mut vector_tessellator,
        );

        assert!(matches!(frame.ops[0], DrawOp::Quad { .. }));
        assert!(matches!(frame.ops[1], DrawOp::Text { index: 0 }));
        assert!(matches!(frame.ops[2], DrawOp::Quad { .. }));
        assert!(matches!(frame.ops[3], DrawOp::Text { index: 1 }));
        assert_eq!(frame.text_requests.len(), 2);
        assert_eq!(frame.text_requests[0].text, "first");
        assert_eq!(frame.text_requests[1].text, "second");
    }

    #[test]
    fn prepare_frame_tessellates_path_fill_to_vector_op() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[path_command(PathStyle::fill(Fill::non_zero(Color::WHITE)))],
            &mut vector_tessellator,
        );

        assert!(!frame.vector_vertices.is_empty());
        assert!(!frame.vector_indices.is_empty());
        assert!(matches!(frame.ops[0], DrawOp::Vector { .. }));
    }

    #[test]
    fn prepare_frame_tessellates_path_stroke_to_vector_op() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[DrawCommand::Path(PathRequest {
                data: PathData::line(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
                style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
            })],
            &mut vector_tessellator,
        );

        assert!(!frame.vector_vertices.is_empty());
        assert!(!frame.vector_indices.is_empty());
        assert!(matches!(frame.ops[0], DrawOp::Vector { .. }));
    }
}
