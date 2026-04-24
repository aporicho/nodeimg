use crate::geometry::{Affine2D, Point, Rect};

pub(super) const AFFINE_EPSILON: f32 = 1e-5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TranslateUniformScale {
    pub tx: f32,
    pub ty: f32,
    pub scale: f32,
}

impl TranslateUniformScale {
    pub fn rect(self, rect: Rect) -> Rect {
        Rect {
            x: self.tx + rect.x * self.scale,
            y: self.ty + rect.y * self.scale,
            w: rect.w * self.scale,
            h: rect.h * self.scale,
        }
    }
}

pub(super) fn translate_uniform_scale(transform: Affine2D) -> Option<TranslateUniformScale> {
    if !transform.is_finite()
        || transform.xy.abs() > AFFINE_EPSILON
        || transform.yx.abs() > AFFINE_EPSILON
        || (transform.xx - transform.yy).abs() > AFFINE_EPSILON
        || transform.xx < 0.0
    {
        return None;
    }
    Some(TranslateUniformScale {
        tx: transform.tx,
        ty: transform.ty,
        scale: transform.xx,
    })
}

pub(super) fn similarity_scale(transform: Affine2D) -> Option<f32> {
    if !transform.is_finite() || !transform.is_similarity(AFFINE_EPSILON) {
        return None;
    }
    let scale = transform.approx_uniform_scale();
    (scale.is_finite() && scale > AFFINE_EPSILON).then_some(scale)
}

pub(super) fn transformed_rect_corners(transform: Affine2D, rect: Rect) -> [[f32; 2]; 4] {
    let corners = rect.corners();
    [
        point_array(transform.transform_point(corners[0])),
        point_array(transform.transform_point(corners[1])),
        point_array(transform.transform_point(corners[2])),
        point_array(transform.transform_point(corners[3])),
    ]
}

pub(super) fn point_array(point: Point) -> [f32; 2] {
    [point.x, point.y]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translate_uniform_scale_accepts_existing_compat_shape() {
        let transform = Affine2D::compose(Affine2D::translation(10.0, 20.0), Affine2D::scale(2.0));

        assert_eq!(
            translate_uniform_scale(transform).unwrap().rect(Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            }),
            Rect {
                x: 12.0,
                y: 24.0,
                w: 6.0,
                h: 8.0,
            }
        );
    }

    #[test]
    fn translate_uniform_scale_rejects_rotate() {
        assert!(translate_uniform_scale(Affine2D::rotation_radians(0.25)).is_none());
    }

    #[test]
    fn similarity_scale_accepts_rotate_scale() {
        let transform = Affine2D::compose(Affine2D::rotation_radians(0.5), Affine2D::scale(3.0));

        assert!((similarity_scale(transform).unwrap() - 3.0).abs() < AFFINE_EPSILON);
    }

    #[test]
    fn transformed_rect_corners_preserves_corner_order() {
        let corners = transformed_rect_corners(
            Affine2D::translation(10.0, 20.0),
            Rect {
                x: 1.0,
                y: 2.0,
                w: 3.0,
                h: 4.0,
            },
        );

        assert_eq!(corners[0], [11.0, 22.0]);
        assert_eq!(corners[2], [14.0, 26.0]);
    }
}
