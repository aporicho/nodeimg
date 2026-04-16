use std::sync::Arc;

use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use super::command::DrawCommand;
use super::pipeline::circle::{CircleRequest, CircleVertex};
use super::pipeline::curve::{CurvePipeline, CurveRequest, CurveVertex};
use super::pipeline::quad::{
    build_rounded_rect_path, QuadRequest, QuadVertex, DEFAULT_CORNER_SMOOTHING,
};
use super::pipeline::shadow::ShadowRequest;
use super::pipeline::stencil::StencilVertex;
use super::pipeline::text::TextRequest;
use super::types::Rect;

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
    Curve {
        index_start: u32,
        index_count: u32,
    },
    Shadow(ShadowRequest),
    Image {
        rect: Rect,
        view: Arc<wgpu::TextureView>,
    },
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

    pub curve_vertices: Vec<CurveVertex>,
    pub curve_indices: Vec<u32>,

    pub circle_vertices: Vec<CircleVertex>,
    pub circle_indices: Vec<u32>,

    pub stencil_vertices: Vec<StencilVertex>,
    pub stencil_indices: Vec<u32>,
}

// ── 预处理 ──

pub fn prepare_frame(
    commands: &[DrawCommand],
    curve_pipeline: &mut CurvePipeline,
) -> PreparedFrame {
    let mut frame = PreparedFrame {
        ops: Vec::new(),
        text_requests: Vec::new(),
        quad_vertices: Vec::new(),
        quad_indices: Vec::new(),
        curve_vertices: Vec::new(),
        curve_indices: Vec::new(),
        circle_vertices: Vec::new(),
        circle_indices: Vec::new(),
        stencil_vertices: Vec::new(),
        stencil_indices: Vec::new(),
    };

    let mut quad_batch: Vec<&QuadRequest> = Vec::new();
    let mut circle_batch: Vec<&CircleRequest> = Vec::new();
    let mut curve_batch: Vec<&CurveRequest> = Vec::new();
    let mut clip_stack: Vec<(Rect, f32)> = Vec::new();

    for cmd in commands {
        match cmd {
            DrawCommand::Shadow(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                frame.ops.push(DrawOp::Shadow(req.clone()));
            }
            DrawCommand::Rect(req) => {
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                quad_batch.push(req);
            }
            DrawCommand::Circle(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                circle_batch.push(req);
            }
            DrawCommand::Text(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                let index = frame.text_requests.len();
                frame.text_requests.push(TextRequest {
                    pos: req.pos,
                    text: req.text.clone(),
                    style: req.style,
                    bounds: req.bounds,
                });
                frame.ops.push(DrawOp::Text { index });
            }
            DrawCommand::Image { rect, view } => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                frame.ops.push(DrawOp::Image {
                    rect: *rect,
                    view: view.clone(),
                });
            }
            DrawCommand::Curve(req) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                curve_batch.push(req);
            }
            DrawCommand::PushClip { rect, radius } => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                clip_stack.push((*rect, *radius));
                tessellate_stencil(&mut frame, *rect, *radius, true);
            }
            DrawCommand::PopClip => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                if let Some((rect, radius)) = clip_stack.pop() {
                    tessellate_stencil(&mut frame, rect, radius, false);
                }
            }
        }
    }

    flush_quad_batch(&mut quad_batch, &mut frame);
    flush_circle_batch(&mut circle_batch, &mut frame);
    flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);

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

fn flush_curve_batch(
    batch: &mut Vec<&CurveRequest>,
    frame: &mut PreparedFrame,
    curve_pipeline: &mut CurvePipeline,
) {
    if batch.is_empty() {
        return;
    }

    let mut total_vertices: Vec<CurveVertex> = Vec::new();
    let mut total_indices: Vec<u32> = Vec::new();

    for curve in batch.iter() {
        let (verts, idxs) = curve_pipeline.tessellate(curve);
        let vertex_offset = total_vertices.len() as u32;
        total_vertices.extend_from_slice(verts);
        for idx in idxs {
            total_indices.push(idx + vertex_offset);
        }
    }

    batch.clear();

    if total_indices.is_empty() {
        return;
    }

    let index_start = frame.curve_indices.len() as u32;
    let vertex_offset = frame.curve_vertices.len() as u32;

    for idx in &total_indices {
        frame.curve_indices.push(idx + vertex_offset);
    }
    frame.curve_vertices.extend_from_slice(&total_vertices);

    frame.ops.push(DrawOp::Curve {
        index_start,
        index_count: total_indices.len() as u32,
    });
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
    use crate::renderer::test_support::try_test_device;
    use crate::renderer::{Color, Point, Rect, RectStyle, TextStyle};

    fn test_curve_pipeline() -> Option<CurvePipeline> {
        let (device, _queue) = try_test_device("prepare-test-device")?;

        Some(CurvePipeline::new(
            &device,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::MultisampleState::default(),
        ))
    }

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

    #[test]
    fn prepare_frame_preserves_text_indices_in_order() {
        let Some(mut curve_pipeline) = test_curve_pipeline() else {
            return;
        };
        let frame = prepare_frame(
            &[
                rect_command(0.0),
                text_command("first"),
                rect_command(20.0),
                text_command("second"),
            ],
            &mut curve_pipeline,
        );

        assert!(matches!(frame.ops[0], DrawOp::Quad { .. }));
        assert!(matches!(frame.ops[1], DrawOp::Text { index: 0 }));
        assert!(matches!(frame.ops[2], DrawOp::Quad { .. }));
        assert!(matches!(frame.ops[3], DrawOp::Text { index: 1 }));
        assert_eq!(frame.text_requests.len(), 2);
        assert_eq!(frame.text_requests[0].text, "first");
        assert_eq!(frame.text_requests[1].text, "second");
    }
}
