use crate::renderer::Rect;

use super::types::Edges;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChildCoordinateSpace {
    Parent,
    Local,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LayoutBoxes {
    pub margin_box: Rect,
    pub border_box: Rect,
    pub content_box: Rect,
}

pub(crate) fn border_box_from_available(available: Rect, margin: Edges) -> Rect {
    inset_rect(available, margin)
}

pub(crate) fn child_content_box(
    border_box: Rect,
    padding: Edges,
    space: ChildCoordinateSpace,
) -> Rect {
    let size = content_size(border_box, padding);
    match space {
        ChildCoordinateSpace::Parent => Rect {
            x: border_box.x + padding.left,
            y: border_box.y + padding.top,
            w: size.0,
            h: size.1,
        },
        ChildCoordinateSpace::Local => Rect {
            x: padding.left,
            y: padding.top,
            w: size.0,
            h: size.1,
        },
    }
}

pub(crate) fn layout_boxes(
    margin_box: Rect,
    border_box: Rect,
    padding: Edges,
    space: ChildCoordinateSpace,
) -> LayoutBoxes {
    LayoutBoxes {
        margin_box,
        border_box,
        content_box: child_content_box(border_box, padding, space),
    }
}

pub(crate) fn rect_contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn inset_rect(rect: Rect, edges: Edges) -> Rect {
    Rect {
        x: rect.x + edges.left,
        y: rect.y + edges.top,
        w: (rect.w - edges.horizontal()).max(0.0),
        h: (rect.h - edges.vertical()).max(0.0),
    }
}

fn content_size(border_box: Rect, padding: Edges) -> (f32, f32) {
    (
        (border_box.w - padding.horizontal()).max(0.0),
        (border_box.h - padding.vertical()).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    fn assert_rect(actual: Rect, expected: Rect) {
        assert_eq!(actual.x, expected.x);
        assert_eq!(actual.y, expected.y);
        assert_eq!(actual.w, expected.w);
        assert_eq!(actual.h, expected.h);
    }

    #[test]
    fn border_box_insets_available_by_margin() {
        let actual = border_box_from_available(
            rect(10.0, 20.0, 200.0, 100.0),
            Edges {
                top: 4.0,
                right: 8.0,
                bottom: 12.0,
                left: 16.0,
            },
        );

        assert_rect(actual, rect(26.0, 24.0, 176.0, 84.0));
    }

    #[test]
    fn content_box_uses_parent_space_origin() {
        let actual = child_content_box(
            rect(10.0, 20.0, 200.0, 100.0),
            Edges::symmetric(4.0, 8.0),
            ChildCoordinateSpace::Parent,
        );

        assert_rect(actual, rect(18.0, 24.0, 184.0, 92.0));
    }

    #[test]
    fn content_box_can_use_local_space_origin() {
        let actual = child_content_box(
            rect(10.0, 20.0, 200.0, 100.0),
            Edges::symmetric(4.0, 8.0),
            ChildCoordinateSpace::Local,
        );

        assert_rect(actual, rect(8.0, 4.0, 184.0, 92.0));
    }

    #[test]
    fn inset_dimensions_clamp_to_zero() {
        let actual = child_content_box(
            rect(0.0, 0.0, 10.0, 10.0),
            Edges::all(20.0),
            ChildCoordinateSpace::Parent,
        );

        assert_rect(actual, rect(20.0, 20.0, 0.0, 0.0));
    }

    #[test]
    fn layout_boxes_groups_margin_border_and_content() {
        let boxes = layout_boxes(
            rect(0.0, 0.0, 100.0, 100.0),
            rect(10.0, 12.0, 80.0, 70.0),
            Edges::all(5.0),
            ChildCoordinateSpace::Parent,
        );

        assert_rect(boxes.margin_box, rect(0.0, 0.0, 100.0, 100.0));
        assert_rect(boxes.border_box, rect(10.0, 12.0, 80.0, 70.0));
        assert_rect(boxes.content_box, rect(15.0, 17.0, 70.0, 60.0));
    }

    #[test]
    fn rect_contains_includes_edges() {
        let r = rect(10.0, 20.0, 30.0, 40.0);

        assert!(rect_contains(r, 10.0, 20.0));
        assert!(rect_contains(r, 40.0, 60.0));
        assert!(!rect_contains(r, 9.9, 20.0));
        assert!(!rect_contains(r, 40.1, 60.0));
    }
}
