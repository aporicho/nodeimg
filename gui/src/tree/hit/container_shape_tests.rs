use super::container_shape::ContainerShape;
use crate::geometry::ResizeEdge;
use crate::geometry::{Point, Rect};

fn rect_100() -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w: 100.0,
        h: 100.0,
    }
}

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

#[test]
fn rect_resize_edge_resolves_sides_and_corners() {
    let shape = ContainerShape::rect(rect_100());

    assert_eq!(
        shape.resize_edge(point(100.0, 50.0), 10.0),
        Some(ResizeEdge::Right)
    );
    assert_eq!(
        shape.resize_edge(point(50.0, 100.0), 10.0),
        Some(ResizeEdge::Bottom)
    );
    assert_eq!(
        shape.resize_edge(point(100.0, 100.0), 10.0),
        Some(ResizeEdge::BottomRight)
    );
}

#[test]
fn rounded_rect_contains_rejects_empty_corner() {
    let shape = ContainerShape::rounded_rect(rect_100(), [40.0; 4]);

    assert!(!shape.contains(point(100.0, 100.0)));
    assert!(shape.contains(point(75.0, 75.0)));
}

#[test]
fn rounded_rect_resize_rejects_far_empty_corner() {
    let shape = ContainerShape::rounded_rect(rect_100(), [40.0; 4]);

    assert_eq!(shape.resize_edge(point(100.0, 100.0), 10.0), None);
}

#[test]
fn rounded_rect_resize_accepts_corner_arc() {
    let shape = ContainerShape::rounded_rect(rect_100(), [40.0; 4]);

    assert_eq!(
        shape.resize_edge(point(88.0, 88.0), 10.0),
        Some(ResizeEdge::BottomRight)
    );
}

#[test]
fn rounded_rect_resize_ignores_non_edge_interior() {
    let shape = ContainerShape::rounded_rect(rect_100(), [16.0; 4]);

    assert_eq!(shape.resize_edge(point(50.0, 50.0), 10.0), None);
}
