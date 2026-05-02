use super::leaf_shape::{circle_hit, fill_hit, grid_hit, path_hit, stroke_hit};
use crate::geometry::{Point, Rect};
use crate::paint::{Color, Fill, FillRule, PathData, PathStyle, Stroke};

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
    let style =
        PathStyle::fill_and_stroke(Fill::non_zero(Color::WHITE), Stroke::new(2.0, Color::WHITE));

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
