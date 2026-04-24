use super::{Affine2D, Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformSpec {
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotate: f32,
    pub skew: [f32; 2],
    pub origin: TransformOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransformOrigin {
    Point(Point),
    Percent { x: f32, y: f32 },
}

impl TransformOrigin {
    pub const fn top_left() -> Self {
        Self::Percent { x: 0.0, y: 0.0 }
    }

    pub const fn center() -> Self {
        Self::Percent { x: 0.5, y: 0.5 }
    }

    pub fn resolve(self, bounds: Rect) -> Point {
        match self {
            Self::Point(point) => point,
            Self::Percent { x, y } => Point {
                x: bounds.x + bounds.w * x,
                y: bounds.y + bounds.h * y,
            },
        }
    }
}

impl TransformSpec {
    pub const fn identity() -> Self {
        Self {
            translate: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotate: 0.0,
            skew: [0.0, 0.0],
            origin: TransformOrigin::top_left(),
        }
    }

    pub const fn translate_scale(translate: [f32; 2], scale: f32) -> Self {
        Self::translate_scale_rotate(translate, scale, 0.0)
    }

    pub const fn translate_scale_rotate(translate: [f32; 2], scale: f32, rotate: f32) -> Self {
        Self {
            translate,
            scale: [scale, scale],
            rotate,
            skew: [0.0, 0.0],
            origin: TransformOrigin::top_left(),
        }
    }

    pub fn to_affine(self, bounds: Rect) -> Affine2D {
        let origin = self.origin.resolve(bounds);
        let translate = Affine2D::translation(self.translate[0], self.translate[1]);
        let origin_to_parent = Affine2D::translation(origin.x, origin.y);
        let origin_to_local = Affine2D::translation(-origin.x, -origin.y);
        let rotate = Affine2D::rotation_radians(self.rotate);
        let skew = Affine2D::skew(self.skew[0], self.skew[1]);
        let scale = Affine2D::scale_non_uniform(self.scale[0], self.scale[1]);

        Affine2D::compose(
            translate,
            Affine2D::compose(
                origin_to_parent,
                Affine2D::compose(
                    rotate,
                    Affine2D::compose(skew, Affine2D::compose(scale, origin_to_local)),
                ),
            ),
        )
    }
}

impl Default for TransformSpec {
    fn default() -> Self {
        Self::identity()
    }
}

impl Default for TransformOrigin {
    fn default() -> Self {
        Self::top_left()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn rect() -> Rect {
        Rect {
            x: 10.0,
            y: 20.0,
            w: 100.0,
            h: 50.0,
        }
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn default_is_identity_top_left() {
        assert_eq!(TransformSpec::default(), TransformSpec::identity());
        assert_eq!(TransformSpec::default().origin, TransformOrigin::top_left());
    }

    #[test]
    fn translate_scale_uses_top_left_uniform_scale() {
        let spec = TransformSpec::translate_scale([10.0, 20.0], 2.0);

        assert_eq!(spec.translate, [10.0, 20.0]);
        assert_eq!(spec.scale, [2.0, 2.0]);
        assert_eq!(spec.rotate, 0.0);
        assert_eq!(spec.skew, [0.0, 0.0]);
        assert_eq!(spec.origin, TransformOrigin::top_left());
        assert_eq!(
            spec.to_affine(Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            })
            .transform_point(Point { x: 3.0, y: 4.0 }),
            Point { x: 16.0, y: 28.0 }
        );
    }

    #[test]
    fn translate_scale_rotate_includes_rotate() {
        let spec =
            TransformSpec::translate_scale_rotate([0.0, 0.0], 1.0, std::f32::consts::FRAC_PI_2);

        assert_point_near(
            spec.to_affine(Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 50.0,
            })
            .transform_point(Point { x: 10.0, y: 0.0 }),
            Point { x: 0.0, y: 10.0 },
        );
    }

    #[test]
    fn zero_translate_scale_has_no_inverse() {
        assert!(TransformSpec::translate_scale([10.0, 20.0], 0.0)
            .to_affine(rect())
            .inverse()
            .is_none());
    }

    #[test]
    fn origin_percent_resolves_against_bounds() {
        assert_eq!(
            TransformOrigin::Percent { x: 0.25, y: 0.5 }.resolve(rect()),
            Point { x: 35.0, y: 45.0 }
        );
    }

    #[test]
    fn origin_center_rotates_around_rect_center() {
        let spec = TransformSpec {
            rotate: std::f32::consts::FRAC_PI_2,
            origin: TransformOrigin::center(),
            ..TransformSpec::identity()
        };
        let center = TransformOrigin::center().resolve(rect());

        assert_point_near(spec.to_affine(rect()).transform_point(center), center);
    }

    #[test]
    fn to_affine_uses_declared_component_order() {
        let spec = TransformSpec {
            translate: [5.0, 7.0],
            scale: [2.0, 3.0],
            rotate: 0.0,
            skew: [0.0, 0.0],
            origin: TransformOrigin::Point(Point { x: 10.0, y: 20.0 }),
        };

        assert_point_near(
            spec.to_affine(rect())
                .transform_point(Point { x: 11.0, y: 21.0 }),
            Point { x: 17.0, y: 30.0 },
        );
    }
}
