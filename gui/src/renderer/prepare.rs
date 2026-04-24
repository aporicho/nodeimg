use std::sync::Arc;

use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use crate::geometry::{Affine2D, Point};
use crate::paint::ClipShape;

use super::affine::{similarity_scale, translate_uniform_scale};
use super::command::{
    AffineCircleRequest, AffineClipRequest, AffinePathRequest, AffineRectRequest,
    AffineShadowRequest, AffineSvgRasterRequest, AffineTextRequest, BackendCommand,
};
use super::image::resolve_image_draw;
use super::path::{PathRequest, PathStyle};
use super::path_geometry::{
    build_lyon_path, build_rounded_rect_path, circle_path_data, lyon_fill_rule,
    DEFAULT_CORNER_SMOOTHING,
};
use super::pipeline::circle::{CircleRequest, CircleVertex};
use super::pipeline::image::PreparedImageDraw;
use super::pipeline::quad::QuadVertex;
use super::pipeline::stencil::StencilVertex;
use super::pipeline::text::TextRequest;
use super::pipeline::vector::VectorVertex;
use super::style::Fill;
use super::vector_tessellator::VectorTessellator;

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
    Shadow(AffineShadowRequest),
    Image {
        view: Arc<wgpu::TextureView>,
        draw: PreparedImageDraw,
    },
    SvgRaster(AffineSvgRasterRequest),
    AffineText(AffineTextRequest),
    Noop,
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

pub fn prepare_frame(
    commands: &[BackendCommand],
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

    frame
}

fn flush_all_batches(
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

fn flush_circle_and_vector(
    circle_batch: &mut Vec<CircleRequest>,
    vector_batch: &mut Vec<&AffinePathRequest>,
    frame: &mut PreparedFrame,
    vector_tessellator: &mut VectorTessellator,
) {
    flush_circle_batch(circle_batch, frame);
    flush_vector_batch(vector_batch, frame, vector_tessellator);
}

fn flush_quad_batch(batch: &mut Vec<&AffineRectRequest>, frame: &mut PreparedFrame) {
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

fn flush_vector_batch(
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

fn append_circle_requests(req: &AffineCircleRequest, scale: f32, batch: &mut Vec<CircleRequest>) {
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

fn push_text_op(req: &AffineTextRequest, frame: &mut PreparedFrame) {
    let Some(legacy) = translate_uniform_scale(req.transform) else {
        frame.ops.push(DrawOp::AffineText(req.clone()));
        return;
    };

    let index = frame.text_requests.len();
    frame.text_requests.push(TextRequest {
        pos: req.transform.transform_point(req.pos),
        text: req.text.clone(),
        style: scale_text_style(req.style, legacy.scale),
        bounds: req.bounds.map(|bounds| legacy.rect(bounds)),
    });
    frame.ops.push(DrawOp::Text { index });
}

fn scale_text_style(mut style: crate::paint::TextStyle, scale: f32) -> crate::paint::TextStyle {
    style.size *= scale;
    style
}

fn append_circle_vector_fallback(
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

fn flush_circle_batch(batch: &mut Vec<CircleRequest>, frame: &mut PreparedFrame) {
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

fn tessellate_stencil(frame: &mut PreparedFrame, req: &AffineClipRequest, is_write: bool) {
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

fn transform_position(transform: Affine2D, position: [f32; 2]) -> [f32; 2] {
    let point = transform.transform_point(Point {
        x: position[0],
        y: position[1],
    });
    [point.x, point.y]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::renderer::{Color, FillRule, PathData, RectStyle, Stroke, TextStyle};

    fn rect_command(x: f32) -> BackendCommand {
        BackendCommand::Rect(AffineRectRequest {
            rect: Rect {
                x,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            style: RectStyle {
                color: Color::WHITE,
                border: None,
                radius: [0.0; 4],
                shadow: None,
            },
            transform: Affine2D::IDENTITY,
        })
    }

    fn text_command(text: &str) -> BackendCommand {
        BackendCommand::Text(AffineTextRequest {
            pos: Point { x: 0.0, y: 0.0 },
            text: text.to_string(),
            style: TextStyle::new(Color::WHITE, 12.0),
            bounds: None,
            transform: Affine2D::IDENTITY,
        })
    }

    fn path_command(style: PathStyle) -> BackendCommand {
        BackendCommand::Path(AffinePathRequest {
            data: PathData::new()
                .move_to(Point { x: 0.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 0.0 })
                .line_to(Point { x: 10.0, y: 10.0 })
                .close(),
            style,
            transform: Affine2D::IDENTITY,
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
    fn prepare_frame_keeps_translate_scale_text_on_legacy_path() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Text(AffineTextRequest {
                pos: Point { x: 1.0, y: 2.0 },
                text: "fast".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                bounds: Some(Rect {
                    x: 1.0,
                    y: 2.0,
                    w: 40.0,
                    h: 16.0,
                }),
                transform: Affine2D::compose(
                    Affine2D::translation(10.0, 20.0),
                    Affine2D::scale(2.0),
                ),
            })],
            &mut vector_tessellator,
        );

        assert!(matches!(frame.ops[0], DrawOp::Text { index: 0 }));
        assert_eq!(frame.text_requests[0].pos, Point { x: 12.0, y: 24.0 });
        assert_eq!(frame.text_requests[0].style.size, 24.0);
    }

    #[test]
    fn prepare_frame_defers_rotated_text_to_affine_raster_path() {
        let mut vector_tessellator = VectorTessellator::new();
        let transform = Affine2D::rotation_radians(0.25);
        let frame = prepare_frame(
            &[BackendCommand::Text(AffineTextRequest {
                pos: Point { x: 1.0, y: 2.0 },
                text: "affine".to_string(),
                style: TextStyle::new(Color::WHITE, 12.0),
                bounds: None,
                transform,
            })],
            &mut vector_tessellator,
        );

        match &frame.ops[0] {
            DrawOp::AffineText(req) => {
                assert_eq!(req.text, "affine");
                assert_eq!(req.transform, transform);
            }
            _ => panic!("expected affine text op"),
        }
        assert!(frame.text_requests.is_empty());
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
            &[BackendCommand::Path(AffinePathRequest {
                data: PathData::line(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
                style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
                transform: Affine2D::IDENTITY,
            })],
            &mut vector_tessellator,
        );

        assert!(!frame.vector_vertices.is_empty());
        assert!(!frame.vector_indices.is_empty());
        assert!(matches!(frame.ops[0], DrawOp::Vector { .. }));
    }

    #[test]
    fn prepare_frame_scales_local_path_stroke_width_through_transform() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Path(AffinePathRequest {
                data: PathData::line(Point { x: 0.0, y: 0.0 }, Point { x: 10.0, y: 0.0 }),
                style: PathStyle::stroke(Stroke::new(2.0, Color::WHITE)),
                transform: Affine2D::scale(3.0),
            })],
            &mut vector_tessellator,
        );

        let min_y = frame
            .vector_vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        let max_y = frame
            .vector_vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);

        assert!((min_y + 3.0).abs() < 0.01, "min_y={min_y}");
        assert!((max_y - 3.0).abs() < 0.01, "max_y={max_y}");
    }

    #[test]
    fn prepare_frame_applies_affine_to_quad_vertices() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Rect(AffineRectRequest {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 10.0,
                    h: 10.0,
                },
                style: RectStyle {
                    color: Color::WHITE,
                    border: None,
                    radius: [0.0; 4],
                    shadow: None,
                },
                transform: Affine2D::translation(20.0, 30.0),
            })],
            &mut vector_tessellator,
        );

        assert!(!frame.quad_vertices.is_empty());
        assert!(frame
            .quad_vertices
            .iter()
            .all(|vertex| vertex.position[0] >= 20.0 && vertex.position[1] >= 30.0));
    }

    #[test]
    fn prepare_frame_applies_affine_to_vector_vertices() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Path(AffinePathRequest {
                data: PathData::new()
                    .move_to(Point { x: 0.0, y: 0.0 })
                    .line_to(Point { x: 10.0, y: 0.0 })
                    .line_to(Point { x: 10.0, y: 10.0 })
                    .close(),
                style: PathStyle::fill(Fill::non_zero(Color::WHITE)),
                transform: Affine2D::translation(5.0, 7.0),
            })],
            &mut vector_tessellator,
        );

        assert!(!frame.vector_vertices.is_empty());
        assert!(frame
            .vector_vertices
            .iter()
            .all(|vertex| vertex.position[0] >= 5.0 && vertex.position[1] >= 7.0));
    }

    #[test]
    fn prepare_frame_tessellates_path_clip_to_stencil() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[
                BackendCommand::PushClip(AffineClipRequest {
                    shape: ClipShape::Path {
                        data: PathData::new()
                            .move_to(Point { x: 0.0, y: 0.0 })
                            .line_to(Point { x: 10.0, y: 0.0 })
                            .line_to(Point { x: 10.0, y: 10.0 })
                            .close(),
                        fill_rule: FillRule::NonZero,
                    },
                    transform: Affine2D::translation(5.0, 0.0),
                }),
                BackendCommand::PopClip,
            ],
            &mut vector_tessellator,
        );

        assert!(matches!(frame.ops[0], DrawOp::StencilWrite { .. }));
        assert!(matches!(frame.ops[1], DrawOp::StencilClear { .. }));
        assert!(!frame.stencil_vertices.is_empty());
        assert!(frame
            .stencil_vertices
            .iter()
            .all(|vertex| vertex.position[0] >= 5.0));
    }

    #[test]
    fn non_similarity_circle_falls_back_to_vector_op() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Circle(AffineCircleRequest {
                paint: crate::paint::CirclePaint {
                    center: Point { x: 10.0, y: 10.0 },
                    radius: 5.0,
                    fill: Some(Color::WHITE),
                    stroke: None,
                },
                transform: Affine2D::scale_non_uniform(2.0, 1.0),
            })],
            &mut vector_tessellator,
        );

        assert!(frame.circle_vertices.is_empty());
        assert!(!frame.vector_vertices.is_empty());
        assert!(matches!(frame.ops[0], DrawOp::Vector { .. }));
    }

    #[test]
    fn prepare_frame_scales_local_circle_radius_and_stroke_width() {
        let mut vector_tessellator = VectorTessellator::new();
        let frame = prepare_frame(
            &[BackendCommand::Circle(AffineCircleRequest {
                paint: crate::paint::CirclePaint {
                    center: Point { x: 10.0, y: 10.0 },
                    radius: 5.0,
                    fill: Some(Color::BLACK),
                    stroke: Some(Stroke::new(1.0, Color::WHITE)),
                },
                transform: Affine2D::scale(3.0),
            })],
            &mut vector_tessellator,
        );

        assert!(frame
            .circle_vertices
            .iter()
            .any(|vertex| (vertex.radius - 15.0).abs() < 0.01));
        assert!(frame
            .circle_vertices
            .iter()
            .any(|vertex| (vertex.radius - 12.0).abs() < 0.01));
        assert!(matches!(frame.ops[0], DrawOp::Circle { .. }));
    }
}
