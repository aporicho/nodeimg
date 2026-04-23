//! Paint 子系统的纯函数辅助。

use crate::geometry::{Point, Rect};
use crate::paint::PathData;

pub fn connection_path(from: Point, to: Point) -> PathData {
    PathData::cubic(bezier_control_points(from, to))
}

/// 遍历 rect 内 spacing 为间距的格点。spacing <= 0 时返回空 Vec。
pub fn grid_cells(rect: Rect, spacing: f32) -> Vec<Point> {
    if spacing <= 0.0 {
        return Vec::new();
    }
    let cols = (rect.w / spacing).floor() as usize + 1;
    let rows = (rect.h / spacing).floor() as usize + 1;
    let mut cells = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            cells.push(Point {
                x: rect.x + col as f32 * spacing,
                y: rect.y + row as f32 * spacing,
            });
        }
    }
    cells
}

/// 三次贝塞尔 4 个控制点（水平偏移启发式）。
/// p0=from, p1=(from.x+|dx|/2, from.y), p2=(to.x-|dx|/2, to.y), p3=to
pub fn bezier_control_points(from: Point, to: Point) -> [Point; 4] {
    let dx_abs = (to.x - from.x).abs();
    let offset = dx_abs * 0.5;
    [
        from,
        Point {
            x: from.x + offset,
            y: from.y,
        },
        Point {
            x: to.x - offset,
            y: to.y,
        },
        to,
    ]
}

/// 取 rect 中心点。端口圆点使用自身中心作为连接锚点。
pub fn rect_center(rect: Rect) -> Point {
    Point {
        x: rect.x + rect.w * 0.5,
        y: rect.y + rect.h * 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::PathCommand;

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn connection_path_uses_bezier_control_points() {
        let path = connection_path(point(0.0, 0.0), point(100.0, 50.0));

        assert_eq!(
            path.commands,
            vec![
                PathCommand::MoveTo(point(0.0, 0.0)),
                PathCommand::CubicTo(point(50.0, 0.0), point(50.0, 50.0), point(100.0, 50.0)),
            ]
        );
    }

    #[test]
    fn grid_cells_single_point() {
        let cells = grid_cells(rect(0.0, 0.0, 0.0, 0.0), 10.0);

        assert_eq!(cells, vec![point(0.0, 0.0)]);
    }

    #[test]
    fn grid_cells_multi() {
        let cells = grid_cells(rect(0.0, 0.0, 30.0, 30.0), 10.0);

        assert_eq!(cells.len(), 16);
        assert!(cells.contains(&point(0.0, 0.0)));
        assert!(cells.contains(&point(30.0, 30.0)));
    }

    #[test]
    fn grid_cells_zero_spacing() {
        let rect = rect(0.0, 0.0, 100.0, 100.0);

        assert!(grid_cells(rect, 0.0).is_empty());
        assert!(grid_cells(rect, -5.0).is_empty());
    }

    #[test]
    fn bezier_horizontal() {
        let result = bezier_control_points(point(0.0, 0.0), point(100.0, 0.0));

        assert_eq!(result[0], point(0.0, 0.0));
        assert_eq!(result[1], point(50.0, 0.0));
        assert_eq!(result[2], point(50.0, 0.0));
        assert_eq!(result[3], point(100.0, 0.0));
    }

    #[test]
    fn bezier_reverse() {
        let result = bezier_control_points(point(100.0, 0.0), point(0.0, 0.0));

        assert_eq!(result[0], point(100.0, 0.0));
        assert_eq!(result[1], point(150.0, 0.0));
        assert_eq!(result[2], point(-50.0, 0.0));
        assert_eq!(result[3], point(0.0, 0.0));
    }

    #[test]
    fn rect_center_returns_midpoint() {
        assert_eq!(rect_center(rect(10.0, 20.0, 32.0, 32.0)), point(26.0, 36.0));
    }
}
