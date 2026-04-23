use crate::renderer::Rect;

use super::types::{AbsolutePosition, Edges, Inset, Size};

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

pub(crate) fn absolute_available_rect(
    containing_block: Rect,
    position: AbsolutePosition,
    width: Size,
    height: Size,
    desired_size: (f32, f32),
) -> Rect {
    let w = absolute_axis_size(
        containing_block.w,
        position.inset.left,
        position.inset.right,
        width,
        desired_size.0,
    );
    let h = absolute_axis_size(
        containing_block.h,
        position.inset.top,
        position.inset.bottom,
        height,
        desired_size.1,
    );
    Rect {
        x: absolute_axis_origin(
            containing_block.x,
            containing_block.w,
            position.inset.left,
            position.inset.right,
            w,
        ),
        y: absolute_axis_origin(
            containing_block.y,
            containing_block.h,
            position.inset.top,
            position.inset.bottom,
            h,
        ),
        w,
        h,
    }
}

pub(crate) fn relative_offset_rect(rect: Rect, inset: Inset) -> Rect {
    Rect {
        x: rect.x + relative_axis_offset(inset.left, inset.right),
        y: rect.y + relative_axis_offset(inset.top, inset.bottom),
        ..rect
    }
}

fn absolute_axis_size(
    containing_size: f32,
    start: Option<f32>,
    end: Option<f32>,
    size: Size,
    desired_size: f32,
) -> f32 {
    match size {
        Size::Fixed(v) => v,
        Size::Fill => match (start, end) {
            (Some(s), Some(e)) => (containing_size - s - e).max(0.0),
            _ => containing_size,
        },
        Size::Auto => match (start, end) {
            (Some(s), Some(e)) => (containing_size - s - e).max(0.0),
            _ => desired_size,
        },
    }
}

fn absolute_axis_origin(
    containing_origin: f32,
    containing_size: f32,
    start: Option<f32>,
    end: Option<f32>,
    size: f32,
) -> f32 {
    if let Some(start) = start {
        containing_origin + start
    } else if let Some(end) = end {
        containing_origin + containing_size - end - size
    } else {
        containing_origin
    }
}

fn relative_axis_offset(start: Option<f32>, end: Option<f32>) -> f32 {
    if let Some(start) = start {
        start
    } else if let Some(end) = end {
        -end
    } else {
        0.0
    }
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
    use super::super::types::{AbsolutePosition, Inset, Size};
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
    fn absolute_xy_uses_containing_block_origin() {
        let actual = absolute_available_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            AbsolutePosition::xy(10.0, 15.0),
            Size::Fixed(40.0),
            Size::Fixed(30.0),
            (0.0, 0.0),
        );

        assert_rect(actual, rect(30.0, 45.0, 40.0, 30.0));
    }

    #[test]
    fn absolute_right_bottom_positions_from_containing_block_end() {
        let actual = absolute_available_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            AbsolutePosition {
                inset: Inset {
                    top: None,
                    right: Some(12.0),
                    bottom: Some(8.0),
                    left: None,
                },
            },
            Size::Fixed(40.0),
            Size::Fixed(30.0),
            (0.0, 0.0),
        );

        assert_rect(actual, rect(168.0, 92.0, 40.0, 30.0));
    }

    #[test]
    fn absolute_left_right_auto_width_stretches_between_insets() {
        let actual = absolute_available_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            AbsolutePosition {
                inset: Inset {
                    top: Some(5.0),
                    right: Some(12.0),
                    bottom: None,
                    left: Some(10.0),
                },
            },
            Size::Auto,
            Size::Fixed(30.0),
            (60.0, 0.0),
        );

        assert_rect(actual, rect(30.0, 35.0, 178.0, 30.0));
    }

    #[test]
    fn absolute_top_bottom_auto_height_stretches_between_insets() {
        let actual = absolute_available_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            AbsolutePosition {
                inset: Inset {
                    top: Some(5.0),
                    right: None,
                    bottom: Some(8.0),
                    left: Some(10.0),
                },
            },
            Size::Fixed(40.0),
            Size::Auto,
            (0.0, 30.0),
        );

        assert_rect(actual, rect(30.0, 35.0, 40.0, 87.0));
    }

    #[test]
    fn absolute_fixed_size_prefers_left_top_when_both_edges_set() {
        let actual = absolute_available_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            AbsolutePosition {
                inset: Inset::all(10.0),
            },
            Size::Fixed(40.0),
            Size::Fixed(30.0),
            (0.0, 0.0),
        );

        assert_rect(actual, rect(30.0, 40.0, 40.0, 30.0));
    }

    #[test]
    fn relative_offset_uses_start_edges_first() {
        let actual = relative_offset_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            Inset {
                top: Some(5.0),
                right: Some(12.0),
                bottom: Some(8.0),
                left: Some(10.0),
            },
        );

        assert_rect(actual, rect(30.0, 35.0, 200.0, 100.0));
    }

    #[test]
    fn relative_offset_uses_negative_end_edges_without_start_edges() {
        let actual = relative_offset_rect(
            rect(20.0, 30.0, 200.0, 100.0),
            Inset {
                top: None,
                right: Some(12.0),
                bottom: Some(8.0),
                left: None,
            },
        );

        assert_rect(actual, rect(8.0, 22.0, 200.0, 100.0));
    }
}
