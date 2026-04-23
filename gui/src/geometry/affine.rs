use super::{Point, Rect, Vector};

const INVERSE_EPSILON: f32 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine2D {
    pub xx: f32,
    pub xy: f32,
    pub yx: f32,
    pub yy: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Affine2D {
    pub const IDENTITY: Self = Self {
        xx: 1.0,
        xy: 0.0,
        yx: 0.0,
        yy: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    pub const fn translation(tx: f32, ty: f32) -> Self {
        Self {
            tx,
            ty,
            ..Self::IDENTITY
        }
    }

    pub const fn scale(scale: f32) -> Self {
        Self::scale_non_uniform(scale, scale)
    }

    pub const fn scale_non_uniform(sx: f32, sy: f32) -> Self {
        Self {
            xx: sx,
            yy: sy,
            ..Self::IDENTITY
        }
    }

    pub fn rotation_radians(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            xx: cos,
            xy: -sin,
            yx: sin,
            yy: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn skew(skew_x: f32, skew_y: f32) -> Self {
        Self {
            xx: 1.0,
            xy: skew_x.tan(),
            yx: skew_y.tan(),
            yy: 1.0,
            tx: 0.0,
            ty: 0.0,
        }
    }

    pub fn compose(parent: Self, child: Self) -> Self {
        Self {
            xx: parent.xx * child.xx + parent.xy * child.yx,
            xy: parent.xx * child.xy + parent.xy * child.yy,
            yx: parent.yx * child.xx + parent.yy * child.yx,
            yy: parent.yx * child.xy + parent.yy * child.yy,
            tx: parent.xx * child.tx + parent.xy * child.ty + parent.tx,
            ty: parent.yx * child.tx + parent.yy * child.ty + parent.ty,
        }
    }

    pub fn then(self, next: Self) -> Self {
        Self::compose(next, self)
    }

    pub fn inverse(self) -> Option<Self> {
        if !self.is_finite() {
            return None;
        }
        let det = self.xx * self.yy - self.xy * self.yx;
        if det.abs() < INVERSE_EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        let xx = self.yy * inv_det;
        let xy = -self.xy * inv_det;
        let yx = -self.yx * inv_det;
        let yy = self.xx * inv_det;
        Some(Self {
            xx,
            xy,
            yx,
            yy,
            tx: -(xx * self.tx + xy * self.ty),
            ty: -(yx * self.tx + yy * self.ty),
        })
    }

    pub fn transform_point(self, point: Point) -> Point {
        Point {
            x: self.xx * point.x + self.xy * point.y + self.tx,
            y: self.yx * point.x + self.yy * point.y + self.ty,
        }
    }

    pub fn transform_vector(self, vector: Vector) -> Vector {
        Vector {
            x: self.xx * vector.x + self.xy * vector.y,
            y: self.yx * vector.x + self.yy * vector.y,
        }
    }

    pub fn transform_rect_corners(self, rect: Rect) -> [Point; 4] {
        rect.corners().map(|corner| self.transform_point(corner))
    }

    pub fn transformed_bounds(self, rect: Rect) -> Rect {
        Rect::from_points(self.transform_rect_corners(rect))
    }

    pub fn approx_uniform_scale(self) -> f32 {
        let sx = self.transform_vector(Vector { x: 1.0, y: 0.0 }).length();
        let sy = self.transform_vector(Vector { x: 0.0, y: 1.0 }).length();
        (sx + sy) * 0.5
    }

    pub fn is_similarity(self, epsilon: f32) -> bool {
        let col_x = self.transform_vector(Vector { x: 1.0, y: 0.0 });
        let col_y = self.transform_vector(Vector { x: 0.0, y: 1.0 });
        let dot = col_x.x * col_y.x + col_x.y * col_y.y;
        (col_x.length() - col_y.length()).abs() <= epsilon && dot.abs() <= epsilon
    }

    pub fn is_axis_aligned(self, epsilon: f32) -> bool {
        (self.xy.abs() <= epsilon && self.yx.abs() <= epsilon)
            || (self.xx.abs() <= epsilon && self.yy.abs() <= epsilon)
    }

    pub fn is_finite(self) -> bool {
        self.xx.is_finite()
            && self.xy.is_finite()
            && self.yx.is_finite()
            && self.yy.is_finite()
            && self.tx.is_finite()
            && self.ty.is_finite()
    }
}

impl Default for Affine2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn p(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn identity_leaves_point_unchanged() {
        assert_eq!(
            Affine2D::identity().transform_point(p(3.0, 4.0)),
            p(3.0, 4.0)
        );
    }

    #[test]
    fn translation_transforms_point_but_not_vector() {
        let affine = Affine2D::translation(10.0, 20.0);

        assert_eq!(affine.transform_point(p(1.0, 2.0)), p(11.0, 22.0));
        assert_eq!(
            affine.transform_vector(Vector { x: 1.0, y: 2.0 }),
            Vector { x: 1.0, y: 2.0 }
        );
    }

    #[test]
    fn scale_non_uniform_transforms_point_and_vector() {
        let affine = Affine2D::scale_non_uniform(2.0, 3.0);

        assert_eq!(affine.transform_point(p(4.0, 5.0)), p(8.0, 15.0));
        assert_eq!(
            affine.transform_vector(Vector { x: 4.0, y: 5.0 }),
            Vector { x: 8.0, y: 15.0 }
        );
    }

    #[test]
    fn rotation_radians_positive_is_clockwise_in_y_down_space() {
        let affine = Affine2D::rotation_radians(std::f32::consts::FRAC_PI_2);

        assert_point_near(affine.transform_point(p(1.0, 0.0)), p(0.0, 1.0));
    }

    #[test]
    fn compose_applies_child_then_parent() {
        let parent = Affine2D::translation(10.0, 0.0);
        let child = Affine2D::scale(2.0);

        let composed = Affine2D::compose(parent, child);

        assert_eq!(composed.transform_point(p(3.0, 4.0)), p(16.0, 8.0));
    }

    #[test]
    fn inverse_round_trips_point() {
        let affine = Affine2D::compose(
            Affine2D::translation(10.0, -2.0),
            Affine2D::compose(Affine2D::rotation_radians(0.25), Affine2D::scale(3.0)),
        );
        let inverse = affine.inverse().unwrap();
        let original = p(7.0, -4.0);

        assert_point_near(
            inverse.transform_point(affine.transform_point(original)),
            original,
        );
    }

    #[test]
    fn inverse_returns_none_for_singular_matrix() {
        assert!(Affine2D::scale_non_uniform(0.0, 1.0).inverse().is_none());
    }

    #[test]
    fn transformed_bounds_covers_rotated_rect_corners() {
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 20.0,
        };
        let bounds =
            Affine2D::rotation_radians(std::f32::consts::FRAC_PI_2).transformed_bounds(rect);

        assert_point_near(
            Point {
                x: bounds.x,
                y: bounds.y,
            },
            p(-20.0, 0.0),
        );
        assert!((bounds.w - 20.0).abs() < EPS);
        assert!((bounds.h - 10.0).abs() < EPS);
    }

    #[test]
    fn approx_uniform_scale_matches_uniform_scale() {
        assert!((Affine2D::scale(3.0).approx_uniform_scale() - 3.0).abs() < EPS);
    }

    #[test]
    fn is_similarity_rejects_skew() {
        assert!(Affine2D::rotation_radians(0.5).is_similarity(EPS));
        assert!(!Affine2D::skew(0.2, 0.0).is_similarity(EPS));
    }

    #[test]
    fn is_axis_aligned_accepts_90_degree_rotation() {
        assert!(Affine2D::rotation_radians(std::f32::consts::FRAC_PI_2).is_axis_aligned(EPS));
        assert!(!Affine2D::rotation_radians(0.25).is_axis_aligned(EPS));
    }
}
