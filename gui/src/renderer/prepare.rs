use std::sync::Arc;

use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};


use super::circle::{CircleRequest, CircleVertex};
use super::command::DrawCommand;
use super::curve::{CurvePipeline, CurveRequest, CurveVertex};
use super::corner::{build_rounded_rect_path, DEFAULT_CORNER_SMOOTHING};
use super::quad::{QuadRequest, QuadVertex};
use super::shadow::ShadowRequest;
use super::stencil::StencilVertex;
use super::types::Rect;

// ── 绘制操作 ──

pub enum DrawOp {
    Quad { index_start: u32, index_count: u32 },
    Circle { index_start: u32, index_count: u32 },
    Curve { index_start: u32, index_count: u32 },
    Shadow(ShadowRequest),
    Image { rect: Rect, view: Arc<wgpu::TextureView> },
    Text,
    StencilWrite { index_start: u32, index_count: u32 },
    StencilClear { index_start: u32, index_count: u32 },
}

// ── 预处理结果 ──

pub struct PreparedFrame {
    pub ops: Vec<DrawOp>,

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

pub fn prepare_frame(commands: &[DrawCommand], curve_pipeline: &mut CurvePipeline) -> PreparedFrame {
    let mut frame = PreparedFrame {
        ops: Vec::new(),
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
            DrawCommand::Text(_) => {
                flush_quad_batch(&mut quad_batch, &mut frame);
                flush_circle_batch(&mut circle_batch, &mut frame);
                flush_curve_batch(&mut curve_batch, &mut frame, curve_pipeline);
                frame.ops.push(DrawOp::Text);
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
                        &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| QuadVertex {
                            position: vertex.position().to_array(),
                            color: border_color,
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

fn flush_curve_batch(batch: &mut Vec<&CurveRequest>, frame: &mut PreparedFrame, curve_pipeline: &mut CurvePipeline) {
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
            CircleVertex { position: [cx - ext, cy - ext], center, radius: r, color },
            CircleVertex { position: [cx + ext, cy - ext], center, radius: r, color },
            CircleVertex { position: [cx + ext, cy + ext], center, radius: r, color },
            CircleVertex { position: [cx - ext, cy + ext], center, radius: r, color },
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
        frame.ops.push(DrawOp::StencilWrite { index_start, index_count });
    } else {
        frame.ops.push(DrawOp::StencilClear { index_start, index_count });
    }
}
