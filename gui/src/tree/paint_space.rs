use crate::geometry::{Affine2D, Point, Rect};
use crate::tree::layout::Transform;

use super::transform::legacy_transform_affine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PaintSpace {
    pub to_screen: Affine2D,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NodePaintSpace {
    pub local_rect: Rect,
    pub local_to_screen: Affine2D,
    pub child_to_screen: Affine2D,
    pub children_are_local: bool,
}

impl PaintSpace {
    pub(crate) fn root() -> Self {
        Self {
            to_screen: Affine2D::IDENTITY,
        }
    }

    pub(crate) fn node_space(
        self,
        node_rect: Rect,
        transform: Option<Transform>,
    ) -> NodePaintSpace {
        let local_rect = Rect {
            x: 0.0,
            y: 0.0,
            w: node_rect.w,
            h: node_rect.h,
        };
        let local_to_screen = Affine2D::compose(
            self.to_screen,
            Affine2D::translation(node_rect.x, node_rect.y),
        );
        let (child_to_screen, children_are_local) = match transform {
            Some(transform) => (
                Affine2D::compose(
                    local_to_screen,
                    legacy_transform_affine(transform, local_rect),
                ),
                true,
            ),
            None => (self.to_screen, false),
        };

        NodePaintSpace {
            local_rect,
            local_to_screen,
            child_to_screen,
            children_are_local,
        }
    }

    pub(crate) fn point_to_local(self, node_rect: Rect, screen: Point) -> Option<Point> {
        self.node_space(node_rect, None)
            .local_to_screen
            .inverse()
            .map(|inverse| inverse.transform_point(screen))
    }
}

impl NodePaintSpace {
    pub(crate) fn child_space(self) -> PaintSpace {
        PaintSpace {
            to_screen: self.child_to_screen,
        }
    }

    pub(crate) fn screen_bounds(self) -> Rect {
        self.local_to_screen.transformed_bounds(self.local_rect)
    }

    pub(crate) fn contains_local_bounds(self, local: Point) -> bool {
        self.local_rect.contains(local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn point(x: f32, y: f32) -> Point {
        Point { x, y }
    }

    fn assert_point_near(actual: Point, expected: Point) {
        assert!(
            (actual.x - expected.x).abs() < EPS && (actual.y - expected.y).abs() < EPS,
            "actual={actual:?}, expected={expected:?}"
        );
    }

    #[test]
    fn root_space_is_identity() {
        assert_eq!(PaintSpace::root().to_screen, Affine2D::IDENTITY);
    }

    #[test]
    fn node_local_to_screen_translates_by_node_rect() {
        let space = PaintSpace::root().node_space(rect(10.0, 20.0, 30.0, 40.0), None);

        assert_eq!(space.local_rect, rect(0.0, 0.0, 30.0, 40.0));
        assert_eq!(
            space.local_to_screen.transform_point(point(1.0, 2.0)),
            point(11.0, 22.0)
        );
    }

    #[test]
    fn ordinary_child_space_keeps_parent_current_space() {
        let current = PaintSpace {
            to_screen: Affine2D::translation(100.0, 200.0),
        };
        let node = current.node_space(rect(10.0, 20.0, 30.0, 40.0), None);

        assert!(!node.children_are_local);
        assert_eq!(node.child_space().to_screen, current.to_screen);
    }

    #[test]
    fn transform_child_space_uses_node_local_then_transform() {
        let current = PaintSpace {
            to_screen: Affine2D::translation(100.0, 200.0),
        };
        let node = current.node_space(
            rect(10.0, 20.0, 30.0, 40.0),
            Some(Transform {
                translate: [5.0, 7.0],
                scale: 2.0,
                rotate: 0.0,
            }),
        );

        assert!(node.children_are_local);
        assert_eq!(
            node.child_space()
                .to_screen
                .transform_point(point(3.0, 4.0)),
            point(121.0, 235.0)
        );
    }

    #[test]
    fn screen_to_node_local_round_trips_for_translate_scale() {
        let node = PaintSpace::root().node_space(
            rect(10.0, 20.0, 30.0, 40.0),
            Some(Transform {
                translate: [5.0, 7.0],
                scale: 2.0,
                rotate: 0.0,
            }),
        );
        let child_space = node.child_space();
        let screen = child_space.to_screen.transform_point(point(6.0, 8.0));

        assert_eq!(
            child_space.point_to_local(rect(0.0, 0.0, 1.0, 1.0), screen),
            Some(point(6.0, 8.0))
        );
    }

    #[test]
    fn screen_to_node_local_round_trips_for_rotate() {
        let node = PaintSpace::root().node_space(
            rect(0.0, 0.0, 30.0, 40.0),
            Some(Transform {
                translate: [0.0, 0.0],
                scale: 1.0,
                rotate: std::f32::consts::FRAC_PI_2,
            }),
        );
        let child_space = node.child_space();
        let screen = child_space.to_screen.transform_point(point(10.0, 15.0));
        let local = child_space
            .point_to_local(rect(0.0, 0.0, 1.0, 1.0), screen)
            .expect("rotated transform should be invertible");

        assert_point_near(local, point(10.0, 15.0));
    }

    #[test]
    fn non_invertible_transform_returns_none() {
        let node = PaintSpace::root().node_space(
            rect(0.0, 0.0, 30.0, 40.0),
            Some(Transform {
                translate: [0.0, 0.0],
                scale: 0.0,
                rotate: 0.0,
            }),
        );

        assert_eq!(
            node.child_space()
                .point_to_local(rect(0.0, 0.0, 1.0, 1.0), point(1.0, 1.0)),
            None
        );
    }
}
