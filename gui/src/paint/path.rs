use crate::geometry::{Affine2D, Point, Rect};

use super::style::{Fill, Stroke};

#[derive(Debug, Clone, PartialEq)]
pub struct PathData {
    pub commands: Vec<PathCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    QuadTo(Point, Point),
    CubicTo(Point, Point, Point),
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathStyle {
    pub fill: Option<Fill>,
    pub stroke: Option<Stroke>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathRequest {
    pub data: PathData,
    pub style: PathStyle,
}

impl PathData {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn line(start: Point, end: Point) -> Self {
        Self::new().move_to(start).line_to(end)
    }

    pub fn cubic(points: [Point; 4]) -> Self {
        Self::new()
            .move_to(points[0])
            .cubic_to(points[1], points[2], points[3])
    }

    pub fn move_to(mut self, point: Point) -> Self {
        self.commands.push(PathCommand::MoveTo(point));
        self
    }

    pub fn line_to(mut self, point: Point) -> Self {
        self.commands.push(PathCommand::LineTo(point));
        self
    }

    pub fn quad_to(mut self, ctrl: Point, to: Point) -> Self {
        self.commands.push(PathCommand::QuadTo(ctrl, to));
        self
    }

    pub fn cubic_to(mut self, ctrl1: Point, ctrl2: Point, to: Point) -> Self {
        self.commands.push(PathCommand::CubicTo(ctrl1, ctrl2, to));
        self
    }

    pub fn close(mut self) -> Self {
        self.commands.push(PathCommand::Close);
        self
    }

    pub fn translated_scaled(&self, tx: f32, ty: f32, scale: f32) -> Self {
        self.map_points(|point| Point {
            x: tx + point.x * scale,
            y: ty + point.y * scale,
        })
    }

    pub fn transformed(&self, transform: Affine2D) -> Self {
        self.map_points(|point| transform.transform_point(point))
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut bounds: Option<(f32, f32, f32, f32)> = None;

        for point in self.points() {
            bounds = Some(match bounds {
                Some((min_x, min_y, max_x, max_y)) => (
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                ),
                None => (point.x, point.y, point.x, point.y),
            });
        }

        bounds.map(|(min_x, min_y, max_x, max_y)| Rect {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        })
    }

    pub fn map_points(&self, mut map: impl FnMut(Point) -> Point) -> Self {
        let commands = self
            .commands
            .iter()
            .map(|command| match *command {
                PathCommand::MoveTo(point) => PathCommand::MoveTo(map(point)),
                PathCommand::LineTo(point) => PathCommand::LineTo(map(point)),
                PathCommand::QuadTo(ctrl, to) => PathCommand::QuadTo(map(ctrl), map(to)),
                PathCommand::CubicTo(ctrl1, ctrl2, to) => {
                    PathCommand::CubicTo(map(ctrl1), map(ctrl2), map(to))
                }
                PathCommand::Close => PathCommand::Close,
            })
            .collect();
        Self { commands }
    }

    fn points(&self) -> impl Iterator<Item = Point> + '_ {
        self.commands.iter().flat_map(|command| match *command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => SmallPointIter::one(point),
            PathCommand::QuadTo(ctrl, to) => SmallPointIter::two(ctrl, to),
            PathCommand::CubicTo(ctrl1, ctrl2, to) => SmallPointIter::three(ctrl1, ctrl2, to),
            PathCommand::Close => SmallPointIter::empty(),
        })
    }
}

struct SmallPointIter {
    points: [Point; 3],
    len: usize,
    index: usize,
}

impl SmallPointIter {
    fn empty() -> Self {
        Self::new([Point { x: 0.0, y: 0.0 }; 3], 0)
    }

    fn one(point: Point) -> Self {
        Self::new(
            [point, Point { x: 0.0, y: 0.0 }, Point { x: 0.0, y: 0.0 }],
            1,
        )
    }

    fn two(first: Point, second: Point) -> Self {
        Self::new([first, second, Point { x: 0.0, y: 0.0 }], 2)
    }

    fn three(first: Point, second: Point, third: Point) -> Self {
        Self::new([first, second, third], 3)
    }

    fn new(points: [Point; 3], len: usize) -> Self {
        Self {
            points,
            len,
            index: 0,
        }
    }
}

impl Iterator for SmallPointIter {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        let point = self.points[self.index];
        self.index += 1;
        Some(point)
    }
}

impl Default for PathData {
    fn default() -> Self {
        Self::new()
    }
}

impl PathStyle {
    pub fn fill(fill: Fill) -> Self {
        Self {
            fill: Some(fill),
            stroke: None,
        }
    }

    pub fn stroke(stroke: Stroke) -> Self {
        Self {
            fill: None,
            stroke: Some(stroke),
        }
    }

    pub fn fill_and_stroke(fill: Fill, stroke: Stroke) -> Self {
        Self {
            fill: Some(fill),
            stroke: Some(stroke),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::{Color, Stroke};

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn line_path_data_uses_move_and_line() {
        let data = PathData::line(p(1.0, 2.0), p(3.0, 4.0));

        assert_eq!(
            data.commands,
            vec![
                PathCommand::MoveTo(p(1.0, 2.0)),
                PathCommand::LineTo(p(3.0, 4.0))
            ]
        );
    }

    #[test]
    fn cubic_path_data_uses_move_and_cubic() {
        let points = [p(0.0, 1.0), p(2.0, 3.0), p(4.0, 5.0), p(6.0, 7.0)];

        let data = PathData::cubic(points);

        assert_eq!(
            data.commands,
            vec![
                PathCommand::MoveTo(points[0]),
                PathCommand::CubicTo(points[1], points[2], points[3])
            ]
        );
    }

    #[test]
    fn translated_scaled_transforms_all_points() {
        let data = PathData::new()
            .move_to(p(1.0, 2.0))
            .line_to(p(3.0, 4.0))
            .quad_to(p(5.0, 6.0), p(7.0, 8.0))
            .cubic_to(p(9.0, 10.0), p(11.0, 12.0), p(13.0, 14.0))
            .close();

        let transformed = data.translated_scaled(10.0, 20.0, 2.0);

        assert_eq!(
            transformed.commands,
            vec![
                PathCommand::MoveTo(p(12.0, 24.0)),
                PathCommand::LineTo(p(16.0, 28.0)),
                PathCommand::QuadTo(p(20.0, 32.0), p(24.0, 36.0)),
                PathCommand::CubicTo(p(28.0, 40.0), p(32.0, 44.0), p(36.0, 48.0)),
                PathCommand::Close,
            ]
        );
    }

    #[test]
    fn path_bounds_cover_all_command_points() {
        let data = PathData::new()
            .move_to(p(5.0, 6.0))
            .line_to(p(-1.0, 2.0))
            .quad_to(p(3.0, -4.0), p(8.0, 9.0))
            .cubic_to(p(10.0, 1.0), p(2.0, 20.0), p(0.0, 0.0))
            .close();

        assert_eq!(
            data.bounds(),
            Some(Rect {
                x: -1.0,
                y: -4.0,
                w: 11.0,
                h: 24.0,
            })
        );
    }

    #[test]
    fn empty_path_has_no_bounds() {
        assert_eq!(PathData::new().bounds(), None);
    }

    #[test]
    fn path_transformed_uses_affine_for_all_points() {
        let data = PathData::line(p(1.0, 2.0), p(3.0, 4.0));

        let transformed = data.transformed(Affine2D::translation(10.0, 20.0));

        assert_eq!(transformed, PathData::line(p(11.0, 22.0), p(13.0, 24.0)));
    }

    #[test]
    fn path_style_constructors_set_expected_fields() {
        let stroke = Stroke::new(1.0, Color::WHITE);

        assert_eq!(PathStyle::stroke(stroke).stroke, Some(stroke));
        assert!(PathStyle::stroke(stroke).fill.is_none());
    }
}
