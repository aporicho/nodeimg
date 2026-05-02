use crate::control::ResizeEdge;
use crate::geometry::{Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ContainerShape {
    RoundedRect { rect: Rect, radius: [f32; 4] },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Corner {
    center: Point,
    radius: f32,
    edge: ResizeEdge,
    horizontal_sign: f32,
    vertical_sign: f32,
}

impl ContainerShape {
    pub(crate) fn rounded_rect(rect: Rect, radius: [f32; 4]) -> Self {
        Self::RoundedRect { rect, radius }
    }

    #[cfg(test)]
    pub(crate) fn rect(rect: Rect) -> Self {
        Self::RoundedRect {
            rect,
            radius: [0.0; 4],
        }
    }

    pub(crate) fn contains(self, point: Point) -> bool {
        match self {
            Self::RoundedRect { rect, radius } => rounded_rect_contains(rect, radius, point),
        }
    }

    pub(crate) fn resize_edge(self, point: Point, threshold: f32) -> Option<ResizeEdge> {
        match self {
            Self::RoundedRect { rect, radius } => {
                rounded_rect_resize_edge(rect, radius, point, threshold)
            }
        }
    }
}

fn rounded_rect_contains(rect: Rect, radius: [f32; 4], point: Point) -> bool {
    if rect.is_empty() || !rect.contains(point) {
        return false;
    }

    let radii = normalized_radii(rect, radius);
    corners(rect, radii)
        .iter()
        .all(|corner| !inside_corner_square(point, corner) || inside_corner_arc(point, corner))
}

fn rounded_rect_resize_edge(
    rect: Rect,
    radius: [f32; 4],
    point: Point,
    threshold: f32,
) -> Option<ResizeEdge> {
    if rect.is_empty() {
        return None;
    }

    let threshold = threshold.max(0.0);
    if !rect.inflate(threshold).contains(point) {
        return None;
    }

    let radii = normalized_radii(rect, radius);
    for corner in corners(rect, radii) {
        if !inside_corner_probe(point, &corner, threshold) {
            continue;
        }
        if corner.radius <= 0.0 {
            if near_corner_edges(rect, point, threshold, corner.edge) {
                return Some(corner.edge);
            }
            continue;
        }
        if near_corner_arc(point, &corner, threshold) {
            return Some(corner.edge);
        }
    }

    straight_edge(rect, radii, point, threshold)
}

fn straight_edge(rect: Rect, radius: [f32; 4], point: Point, threshold: f32) -> Option<ResizeEdge> {
    let left = rect.min_x();
    let right = rect.max_x();
    let top = rect.min_y();
    let bottom = rect.max_y();
    let [top_left, top_right, bottom_right, bottom_left] = radius;

    let in_top_span = point.x >= left + top_left && point.x <= right - top_right;
    let in_right_span = point.y >= top + top_right && point.y <= bottom - bottom_right;
    let in_bottom_span = point.x >= left + bottom_left && point.x <= right - bottom_right;
    let in_left_span = point.y >= top + top_left && point.y <= bottom - bottom_left;

    if in_top_span && (point.y - top).abs() <= threshold {
        Some(ResizeEdge::Top)
    } else if in_right_span && (point.x - right).abs() <= threshold {
        Some(ResizeEdge::Right)
    } else if in_bottom_span && (point.y - bottom).abs() <= threshold {
        Some(ResizeEdge::Bottom)
    } else if in_left_span && (point.x - left).abs() <= threshold {
        Some(ResizeEdge::Left)
    } else {
        None
    }
}

fn normalized_radii(rect: Rect, radius: [f32; 4]) -> [f32; 4] {
    let max_radius = (rect.w.abs().min(rect.h.abs()) * 0.5).max(0.0);
    radius.map(|value| value.max(0.0).min(max_radius))
}

fn corners(rect: Rect, radius: [f32; 4]) -> [Corner; 4] {
    let left = rect.min_x();
    let right = rect.max_x();
    let top = rect.min_y();
    let bottom = rect.max_y();
    let [top_left, top_right, bottom_right, bottom_left] = radius;

    [
        Corner {
            center: Point {
                x: left + top_left,
                y: top + top_left,
            },
            radius: top_left,
            edge: ResizeEdge::TopLeft,
            horizontal_sign: -1.0,
            vertical_sign: -1.0,
        },
        Corner {
            center: Point {
                x: right - top_right,
                y: top + top_right,
            },
            radius: top_right,
            edge: ResizeEdge::TopRight,
            horizontal_sign: 1.0,
            vertical_sign: -1.0,
        },
        Corner {
            center: Point {
                x: right - bottom_right,
                y: bottom - bottom_right,
            },
            radius: bottom_right,
            edge: ResizeEdge::BottomRight,
            horizontal_sign: 1.0,
            vertical_sign: 1.0,
        },
        Corner {
            center: Point {
                x: left + bottom_left,
                y: bottom - bottom_left,
            },
            radius: bottom_left,
            edge: ResizeEdge::BottomLeft,
            horizontal_sign: -1.0,
            vertical_sign: 1.0,
        },
    ]
}

fn inside_corner_square(point: Point, corner: &Corner) -> bool {
    if corner.radius <= 0.0 {
        return false;
    }

    let dx = (point.x - corner.center.x) * corner.horizontal_sign;
    let dy = (point.y - corner.center.y) * corner.vertical_sign;
    dx >= 0.0 && dx <= corner.radius && dy >= 0.0 && dy <= corner.radius
}

fn inside_corner_arc(point: Point, corner: &Corner) -> bool {
    distance_from_corner_center(point, corner) <= corner.radius
}

fn inside_corner_probe(point: Point, corner: &Corner, threshold: f32) -> bool {
    let radius = corner.radius;
    let dx = (point.x - corner.center.x) * corner.horizontal_sign;
    let dy = (point.y - corner.center.y) * corner.vertical_sign;
    dx >= -threshold && dx <= radius + threshold && dy >= -threshold && dy <= radius + threshold
}

fn near_corner_arc(point: Point, corner: &Corner, threshold: f32) -> bool {
    (distance_from_corner_center(point, corner) - corner.radius).abs() <= threshold
}

fn distance_from_corner_center(point: Point, corner: &Corner) -> f32 {
    (point.x - corner.center.x).hypot(point.y - corner.center.y)
}

fn near_corner_edges(rect: Rect, point: Point, threshold: f32, edge: ResizeEdge) -> bool {
    let left = rect.min_x();
    let right = rect.max_x();
    let top = rect.min_y();
    let bottom = rect.max_y();
    let near_left = (point.x - left).abs() <= threshold;
    let near_right = (point.x - right).abs() <= threshold;
    let near_top = (point.y - top).abs() <= threshold;
    let near_bottom = (point.y - bottom).abs() <= threshold;

    match edge {
        ResizeEdge::TopLeft => near_left && near_top,
        ResizeEdge::TopRight => near_right && near_top,
        ResizeEdge::BottomRight => near_right && near_bottom,
        ResizeEdge::BottomLeft => near_left && near_bottom,
        ResizeEdge::Top | ResizeEdge::Right | ResizeEdge::Bottom | ResizeEdge::Left => false,
    }
}
