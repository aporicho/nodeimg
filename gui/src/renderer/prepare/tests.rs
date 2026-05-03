use crate::geometry::{Affine2D, Point, Rect};
use crate::paint::GridPaint;
use crate::renderer::command::{
    AffineCircleRequest, AffineClipRequest, AffineGridRequest, AffinePathRequest,
    AffineRectRequest, AffineTextRequest, BackendCommand,
};
use crate::renderer::vector_tessellator::VectorTessellator;
use crate::renderer::{Color, Fill, FillRule, PathData, PathStyle, RectStyle, Stroke, TextStyle};

use super::{prepare_frame, DrawOp};

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

fn grid_command() -> BackendCommand {
    BackendCommand::Grid(AffineGridRequest {
        paint: GridPaint {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            },
            spacing: 10.0,
            dot_color: Color::WHITE,
            dot_size: 1.0,
        },
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
fn prepare_frame_batches_translate_scale_text_directly() {
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
            transform: Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0)),
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
fn prepare_frame_keeps_grid_as_high_level_quad() {
    let mut vector_tessellator = VectorTessellator::new();
    let frame = prepare_frame(&[grid_command()], &mut vector_tessellator);

    assert!(matches!(frame.ops[0], DrawOp::Grid { .. }));
    assert_eq!(frame.stats.grid_commands, 1);
    assert_eq!(frame.stats.grid_dot_expansions, 0);
    assert_eq!(frame.grid_vertices.len(), 4);
    assert_eq!(frame.grid_indices.len(), 6);
    assert!(frame.circle_vertices.is_empty());
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
                shape: crate::paint::ClipShape::Path {
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
