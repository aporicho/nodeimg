use crate::geometry::{Affine2D, Point};

pub(super) fn transform_position(transform: Affine2D, position: [f32; 2]) -> [f32; 2] {
    let point = transform.transform_point(Point {
        x: position[0],
        y: position[1],
    });
    [point.x, point.y]
}
