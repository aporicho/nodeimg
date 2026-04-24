use super::connection_endpoint::node_screen_center;
use super::layout::LeafKind;
use super::paint_helpers::{connection_path, grid_cells, CONNECTION_WIDTH};
use super::paint_space::{NodePaintSpace, PaintSpace};
use super::tree::Tree;
use crate::geometry::{Point, Rect};
use crate::paint::{FillRule, PathCommand, PathData, PathStyle};

const CURVE_SEGMENTS: usize = 16;
const EPSILON: f32 = 1e-5;

pub(crate) fn leaf_shape_hit(
    tree: &Tree,
    leaf: &LeafKind,
    local_point: Point,
    node_space: NodePaintSpace,
    current_space: PaintSpace,
) -> Option<bool> {
    match leaf {
        LeafKind::Circle {
            radius,
            fill,
            stroke,
        } => {
            let center = Point {
                x: node_space.local_rect.w * 0.5,
                y: node_space.local_rect.h * 0.5,
            };
            let stroke_width = stroke.map(|border| border.width).unwrap_or(0.0);
            Some(circle_hit(
                local_point,
                center,
                *radius,
                fill.is_some(),
                stroke_width,
            ))
        }
        LeafKind::Line { start, end, stroke } => Some(stroke_hit(
            local_point,
            &PathData::line(*start, *end),
            stroke.width,
        )),
        LeafKind::Curve { points, stroke } => Some(stroke_hit(
            local_point,
            &PathData::cubic(*points),
            stroke.width,
        )),
        LeafKind::Path { data, style } => Some(path_hit(local_point, data, *style)),
        LeafKind::Grid {
            spacing, dot_size, ..
        } => Some(grid_hit(
            local_point,
            node_space.local_rect,
            *spacing,
            *dot_size,
        )),
        LeafKind::Connection { from_port, to_port } => {
            let path = connection_path_for_ports(tree, node_space, from_port, to_port)?;
            Some(stroke_hit(local_point, &path, CONNECTION_WIDTH))
        }
        LeafKind::PendingConnection {
            from_port,
            cursor_canvas,
        } => {
            let path = pending_connection_path(
                tree,
                node_space,
                current_space,
                from_port,
                *cursor_canvas,
            )?;
            Some(stroke_hit(local_point, &path, CONNECTION_WIDTH))
        }
        LeafKind::Text { .. }
        | LeafKind::Image { .. }
        | LeafKind::Icon { .. }
        | LeafKind::CustomPaint(_) => None,
    }
}

fn connection_path_for_ports(
    tree: &Tree,
    node_space: NodePaintSpace,
    from_port: &str,
    to_port: &str,
) -> Option<PathData> {
    let from_screen = node_screen_center(tree, from_port)?;
    let to_screen = node_screen_center(tree, to_port)?;
    let inverse = node_space.local_to_screen.inverse()?;
    Some(connection_path(
        inverse.transform_point(from_screen),
        inverse.transform_point(to_screen),
    ))
}

fn pending_connection_path(
    tree: &Tree,
    node_space: NodePaintSpace,
    current_space: PaintSpace,
    from_port: &str,
    cursor_canvas: Point,
) -> Option<PathData> {
    let from_screen = node_screen_center(tree, from_port)?;
    let inverse = node_space.local_to_screen.inverse()?;
    let to_screen = current_space.to_screen.transform_point(cursor_canvas);
    Some(connection_path(
        inverse.transform_point(from_screen),
        inverse.transform_point(to_screen),
    ))
}

fn circle_hit(point: Point, center: Point, radius: f32, has_fill: bool, stroke_width: f32) -> bool {
    if radius <= 0.0 || !radius.is_finite() {
        return false;
    }

    let distance = distance(point, center);
    if has_fill && distance <= radius + EPSILON {
        return true;
    }

    let half_stroke = stroke_width.max(0.0) * 0.5;
    half_stroke > 0.0 && (distance - radius).abs() <= half_stroke + EPSILON
}

fn grid_hit(point: Point, rect: Rect, spacing: f32, radius: f32) -> bool {
    grid_cells(rect, spacing)
        .into_iter()
        .any(|center| circle_hit(point, center, radius, true, 0.0))
}

fn path_hit(point: Point, data: &PathData, style: PathStyle) -> bool {
    if let Some(fill) = style.fill {
        if fill_hit(point, data, fill.rule) {
            return true;
        }
    }

    style
        .stroke
        .is_some_and(|stroke| stroke_hit(point, data, stroke.width))
}

fn stroke_hit(point: Point, data: &PathData, width: f32) -> bool {
    let half_width = width.max(0.0) * 0.5;
    if half_width <= 0.0 {
        return false;
    }

    flattened_segments(data, false).into_iter().any(|segment| {
        distance_to_segment(point, segment.start, segment.end) <= half_width + EPSILON
    })
}

fn fill_hit(point: Point, data: &PathData, fill_rule: FillRule) -> bool {
    let segments = flattened_segments(data, true);
    match fill_rule {
        FillRule::EvenOdd => {
            segments
                .iter()
                .filter(|segment| ray_crosses_segment(point, segment.start, segment.end))
                .count()
                % 2
                == 1
        }
        FillRule::NonZero => {
            let winding = segments.iter().fold(0, |acc, segment| {
                acc + winding_crossing(point, segment.start, segment.end)
            });
            winding != 0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Segment {
    start: Point,
    end: Point,
}

fn flattened_segments(data: &PathData, close_open_contours: bool) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut current = None;
    let mut contour_start = None;

    for command in &data.commands {
        match *command {
            PathCommand::MoveTo(point) => {
                if close_open_contours {
                    close_contour(&mut segments, current, contour_start);
                }
                current = Some(point);
                contour_start = Some(point);
            }
            PathCommand::LineTo(to) => {
                if let Some(from) = current {
                    push_segment(&mut segments, from, to);
                }
                current = Some(to);
            }
            PathCommand::QuadTo(ctrl, to) => {
                if let Some(from) = current {
                    let mut prev = from;
                    for step in 1..=CURVE_SEGMENTS {
                        let t = step as f32 / CURVE_SEGMENTS as f32;
                        let next = quad_point(from, ctrl, to, t);
                        push_segment(&mut segments, prev, next);
                        prev = next;
                    }
                }
                current = Some(to);
            }
            PathCommand::CubicTo(ctrl1, ctrl2, to) => {
                if let Some(from) = current {
                    let mut prev = from;
                    for step in 1..=CURVE_SEGMENTS {
                        let t = step as f32 / CURVE_SEGMENTS as f32;
                        let next = cubic_point(from, ctrl1, ctrl2, to, t);
                        push_segment(&mut segments, prev, next);
                        prev = next;
                    }
                }
                current = Some(to);
            }
            PathCommand::Close => {
                close_contour(&mut segments, current, contour_start);
                current = contour_start;
            }
        }
    }

    if close_open_contours {
        close_contour(&mut segments, current, contour_start);
    }

    segments
}

fn close_contour(segments: &mut Vec<Segment>, current: Option<Point>, start: Option<Point>) {
    if let (Some(current), Some(start)) = (current, start) {
        push_segment(segments, current, start);
    }
}

fn push_segment(segments: &mut Vec<Segment>, start: Point, end: Point) {
    if distance(start, end) > EPSILON {
        segments.push(Segment { start, end });
    }
}

fn quad_point(start: Point, ctrl: Point, end: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    Point {
        x: mt * mt * start.x + 2.0 * mt * t * ctrl.x + t * t * end.x,
        y: mt * mt * start.y + 2.0 * mt * t * ctrl.y + t * t * end.y,
    }
}

fn cubic_point(start: Point, ctrl1: Point, ctrl2: Point, end: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    Point {
        x: mt * mt * mt * start.x
            + 3.0 * mt * mt * t * ctrl1.x
            + 3.0 * mt * t * t * ctrl2.x
            + t * t * t * end.x,
        y: mt * mt * mt * start.y
            + 3.0 * mt * mt * t * ctrl1.y
            + 3.0 * mt * t * t * ctrl2.y
            + t * t * t * end.y,
    }
}

fn ray_crosses_segment(point: Point, a: Point, b: Point) -> bool {
    if (a.y > point.y) == (b.y > point.y) {
        return false;
    }
    let x = a.x + (point.y - a.y) * (b.x - a.x) / (b.y - a.y);
    x > point.x
}

fn winding_crossing(point: Point, a: Point, b: Point) -> i32 {
    if a.y <= point.y {
        if b.y > point.y && is_left(a, b, point) > 0.0 {
            return 1;
        }
    } else if b.y <= point.y && is_left(a, b, point) < 0.0 {
        return -1;
    }
    0
}

fn is_left(a: Point, b: Point, point: Point) -> f32 {
    (b.x - a.x) * (point.y - a.y) - (point.x - a.x) * (b.y - a.y)
}

fn distance_to_segment(point: Point, start: Point, end: Point) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq <= EPSILON {
        return distance(point, start);
    }

    let t = (((point.x - start.x) * dx + (point.y - start.y) * dy) / len_sq).clamp(0.0, 1.0);
    distance(
        point,
        Point {
            x: start.x + dx * t,
            y: start.y + dy * t,
        },
    )
}

fn distance(a: Point, b: Point) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{Color, Fill, Stroke};

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn circle_hit_rejects_bounding_box_corner() {
        assert!(circle_hit(p(5.0, 5.0), p(5.0, 5.0), 5.0, true, 0.0));
        assert!(!circle_hit(p(0.0, 0.0), p(5.0, 5.0), 5.0, true, 0.0));
    }

    #[test]
    fn stroke_hit_uses_distance_to_line() {
        let data = PathData::line(p(0.0, 0.0), p(10.0, 0.0));

        assert!(stroke_hit(p(5.0, 0.9), &data, 2.0));
        assert!(!stroke_hit(p(5.0, 2.0), &data, 2.0));
    }

    #[test]
    fn stroke_hit_flattens_cubic_curve() {
        let data = PathData::cubic([p(0.0, 0.0), p(5.0, 0.0), p(5.0, 10.0), p(10.0, 10.0)]);

        assert!(stroke_hit(p(5.0, 5.0), &data, 3.0));
        assert!(!stroke_hit(p(5.0, 9.0), &data, 1.0));
    }

    #[test]
    fn fill_hit_respects_even_odd_rule() {
        let data = PathData::new()
            .move_to(p(0.0, 0.0))
            .line_to(p(10.0, 0.0))
            .line_to(p(10.0, 10.0))
            .line_to(p(0.0, 10.0))
            .close()
            .move_to(p(3.0, 3.0))
            .line_to(p(7.0, 3.0))
            .line_to(p(7.0, 7.0))
            .line_to(p(3.0, 7.0))
            .close();

        assert!(fill_hit(p(1.0, 1.0), &data, FillRule::EvenOdd));
        assert!(!fill_hit(p(5.0, 5.0), &data, FillRule::EvenOdd));
    }

    #[test]
    fn path_hit_accepts_fill_or_stroke() {
        let data = PathData::new()
            .move_to(p(0.0, 0.0))
            .line_to(p(10.0, 0.0))
            .line_to(p(10.0, 10.0))
            .close();
        let style = PathStyle::fill_and_stroke(
            Fill::non_zero(Color::WHITE),
            Stroke::new(2.0, Color::WHITE),
        );

        assert!(path_hit(p(5.0, 2.0), &data, style));
        assert!(path_hit(p(5.0, -0.5), &data, style));
        assert!(!path_hit(p(20.0, 20.0), &data, style));
    }

    #[test]
    fn grid_hit_checks_dot_circles() {
        assert!(grid_hit(
            p(0.5, 0.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            10.0,
            1.0,
        ));
        assert!(!grid_hit(
            p(5.0, 5.0),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            10.0,
            1.0,
        ));
    }
}
