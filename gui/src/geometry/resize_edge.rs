#[cfg(test)]
use crate::renderer::Rect;

pub const DEFAULT_RESIZE_EDGE_THRESHOLD: f32 = 10.0;

/// Resize gesture edge or corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[cfg(test)]
pub(crate) fn detect_resize_edge(rect: Rect, x: f32, y: f32, threshold: f32) -> Option<ResizeEdge> {
    let threshold = threshold.max(0.0);
    let near_left = (x - rect.x).abs() < threshold;
    let near_right = (x - (rect.x + rect.w)).abs() < threshold;
    let near_top = (y - rect.y).abs() < threshold;
    let near_bottom = (y - (rect.y + rect.h)).abs() < threshold;

    match (near_left, near_right, near_top, near_bottom) {
        (true, _, true, _) => Some(ResizeEdge::TopLeft),
        (true, _, _, true) => Some(ResizeEdge::BottomLeft),
        (_, true, true, _) => Some(ResizeEdge::TopRight),
        (_, true, _, true) => Some(ResizeEdge::BottomRight),
        (true, _, _, _) => Some(ResizeEdge::Left),
        (_, true, _, _) => Some(ResizeEdge::Right),
        (_, _, true, _) => Some(ResizeEdge::Top),
        (_, _, _, true) => Some(ResizeEdge::Bottom),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_100() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        }
    }

    #[test]
    fn detect_resize_edge_uses_supplied_threshold() {
        let rect = rect_100();

        assert_eq!(
            detect_resize_edge(rect, 8.0, 50.0, 10.0),
            Some(ResizeEdge::Left)
        );
        assert_eq!(detect_resize_edge(rect, 8.0, 50.0, 6.0), None);
    }

    #[test]
    fn detect_resize_edge_resolves_corners_before_sides() {
        let rect = rect_100();

        assert_eq!(
            detect_resize_edge(rect, 2.0, 2.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::TopLeft)
        );
        assert_eq!(
            detect_resize_edge(rect, 98.0, 98.0, DEFAULT_RESIZE_EDGE_THRESHOLD),
            Some(ResizeEdge::BottomRight)
        );
    }
}
