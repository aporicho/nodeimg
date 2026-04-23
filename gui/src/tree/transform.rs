use crate::geometry::{Affine2D, Rect, TransformOrigin, TransformSpec};
use crate::tree::layout::Transform;

pub(crate) fn legacy_transform_spec(transform: Transform) -> TransformSpec {
    TransformSpec {
        translate: transform.translate,
        scale: [transform.scale, transform.scale],
        rotate: transform.rotate,
        skew: [0.0, 0.0],
        origin: TransformOrigin::top_left(),
    }
}

pub(crate) fn legacy_transform_affine(transform: Transform, local_bounds: Rect) -> Affine2D {
    legacy_transform_spec(transform).to_affine(local_bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Point;

    const EPS: f32 = 1e-5;

    fn local_bounds() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
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
    fn legacy_transform_spec_preserves_translate_scale_rotate() {
        let transform = Transform {
            translate: [10.0, 20.0],
            scale: 2.0,
            rotate: 0.25,
        };
        let spec = legacy_transform_spec(transform);

        assert_eq!(spec.translate, [10.0, 20.0]);
        assert_eq!(spec.scale, [2.0, 2.0]);
        assert_eq!(spec.rotate, 0.25);
        assert_eq!(spec.skew, [0.0, 0.0]);
        assert_eq!(spec.origin, TransformOrigin::top_left());
    }

    #[test]
    fn legacy_transform_affine_matches_old_translate_scale_for_top_left_origin() {
        let affine = legacy_transform_affine(
            Transform {
                translate: [10.0, 20.0],
                scale: 2.0,
                rotate: 0.0,
            },
            local_bounds(),
        );

        assert_eq!(
            affine.transform_point(Point { x: 3.0, y: 4.0 }),
            Point { x: 16.0, y: 28.0 }
        );
    }

    #[test]
    fn legacy_transform_affine_includes_rotate_in_recording_semantics() {
        let affine = legacy_transform_affine(
            Transform {
                translate: [0.0, 0.0],
                scale: 1.0,
                rotate: std::f32::consts::FRAC_PI_2,
            },
            local_bounds(),
        );

        assert_point_near(
            affine.transform_point(Point { x: 10.0, y: 0.0 }),
            Point { x: 0.0, y: 10.0 },
        );
    }

    #[test]
    fn zero_scale_inverse_returns_none_through_affine() {
        let affine = legacy_transform_affine(
            Transform {
                translate: [10.0, 20.0],
                scale: 0.0,
                rotate: 0.0,
            },
            local_bounds(),
        );

        assert!(affine.inverse().is_none());
    }
}
