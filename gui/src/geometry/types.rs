#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Vector {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl Rect {
    pub fn min_x(self) -> f32 {
        self.x.min(self.x + self.w)
    }

    pub fn min_y(self) -> f32 {
        self.y.min(self.y + self.h)
    }

    pub fn max_x(self) -> f32 {
        self.x.max(self.x + self.w)
    }

    pub fn max_y(self) -> f32 {
        self.y.max(self.y + self.h)
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn contains(self, point: Point) -> bool {
        point.x >= self.min_x()
            && point.x <= self.max_x()
            && point.y >= self.min_y()
            && point.y <= self.max_y()
    }

    pub fn corners(self) -> [Point; 4] {
        let min_x = self.min_x();
        let min_y = self.min_y();
        let max_x = self.max_x();
        let max_y = self.max_y();
        [
            Point { x: min_x, y: min_y },
            Point { x: max_x, y: min_y },
            Point { x: max_x, y: max_y },
            Point { x: min_x, y: max_y },
        ]
    }

    pub fn from_points(points: [Point; 4]) -> Self {
        let mut min_x = points[0].x;
        let mut min_y = points[0].y;
        let mut max_x = points[0].x;
        let mut max_y = points[0].y;

        for point in &points[1..] {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        Self {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    }

    pub fn union(self, other: Self) -> Self {
        let min_x = self.min_x().min(other.min_x());
        let min_y = self.min_y().min(other.min_y());
        let max_x = self.max_x().max(other.max_x());
        let max_y = self.max_y().max(other.max_y());
        Self {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    }

    pub fn translate(self, offset: Vector) -> Self {
        Self {
            x: self.x + offset.x,
            y: self.y + offset.y,
            w: self.w,
            h: self.h,
        }
    }

    pub fn inflate(self, amount: f32) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            w: self.w + amount * 2.0,
            h: self.h + amount * 2.0,
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.w.is_finite() && self.h.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    #[test]
    fn rect_contains_inclusive_edges() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 40.0,
        };

        assert!(rect.contains(p(10.0, 20.0)));
        assert!(rect.contains(p(40.0, 60.0)));
        assert!(!rect.contains(p(40.1, 60.0)));
    }

    #[test]
    fn rect_corners_are_normalized_for_negative_size() {
        let rect = Rect {
            x: 10.0,
            y: 20.0,
            w: -4.0,
            h: -5.0,
        };

        assert_eq!(
            rect.corners(),
            [p(6.0, 15.0), p(10.0, 15.0), p(10.0, 20.0), p(6.0, 20.0)]
        );
    }

    #[test]
    fn rect_from_points_builds_bounds() {
        let rect = Rect::from_points([p(10.0, 5.0), p(-2.0, 8.0), p(4.0, -3.0), p(7.0, 2.0)]);

        assert_eq!(
            rect,
            Rect {
                x: -2.0,
                y: -3.0,
                w: 12.0,
                h: 11.0,
            }
        );
    }

    #[test]
    fn rect_union_covers_both_rects() {
        let a = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 20.0,
        };
        let b = Rect {
            x: -5.0,
            y: 4.0,
            w: 8.0,
            h: 40.0,
        };

        assert_eq!(
            a.union(b),
            Rect {
                x: -5.0,
                y: 0.0,
                w: 15.0,
                h: 44.0,
            }
        );
    }
}
