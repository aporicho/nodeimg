use crate::geometry::ResizeEdge;
use crate::renderer::Rect;

pub(crate) fn resize_rect_by_edge(
    rect: &mut Rect,
    edge: ResizeEdge,
    dx: f32,
    dy: f32,
    min_width: f32,
    min_height: f32,
) {
    let start = *rect;
    let fixed_right = start.x + start.w;
    let fixed_bottom = start.y + start.h;

    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
            resize_rect_left(rect, start.x + dx, fixed_right, min_width);
        }
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
            resize_rect_right(rect, start.w + dx, min_width);
        }
        _ => {}
    }

    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
            resize_rect_top(rect, start.y + dy, fixed_bottom, min_height);
        }
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
            resize_rect_bottom(rect, start.h + dy, min_height);
        }
        _ => {}
    }

    rect.w = rect.w.max(min_width);
    rect.h = rect.h.max(min_height);
}

pub(crate) fn requested_resize_width(rect: Rect, edge: ResizeEdge, dx: f32) -> f32 {
    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => rect.w - dx,
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => rect.w + dx,
        ResizeEdge::Top | ResizeEdge::Bottom => rect.w,
    }
}

pub(crate) fn requested_resize_height(rect: Rect, edge: ResizeEdge, dy: f32) -> f32 {
    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => rect.h - dy,
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => rect.h + dy,
        ResizeEdge::Left | ResizeEdge::Right => rect.h,
    }
}

pub(crate) fn is_vertical_resize_edge(edge: ResizeEdge) -> bool {
    matches!(
        edge,
        ResizeEdge::Top
            | ResizeEdge::TopLeft
            | ResizeEdge::TopRight
            | ResizeEdge::Bottom
            | ResizeEdge::BottomLeft
            | ResizeEdge::BottomRight
    )
}

fn resize_rect_left(rect: &mut Rect, requested_left: f32, fixed_right: f32, min_width: f32) {
    let next_left = requested_left.min(fixed_right - min_width);
    rect.x = next_left;
    rect.w = fixed_right - next_left;
}

fn resize_rect_right(rect: &mut Rect, requested_width: f32, min_width: f32) {
    rect.w = requested_width.max(min_width);
}

fn resize_rect_top(rect: &mut Rect, requested_top: f32, fixed_bottom: f32, min_height: f32) {
    let next_top = requested_top.min(fixed_bottom - min_height);
    rect.y = next_top;
    rect.h = fixed_bottom - next_top;
}

fn resize_rect_bottom(rect: &mut Rect, requested_height: f32, min_height: f32) {
    rect.h = requested_height.max(min_height);
}
