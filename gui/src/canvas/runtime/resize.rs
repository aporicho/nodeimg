use super::connection_layer;
use super::model::CanvasNodeRuntime;
use crate::canvas::{canvas_node_stable_id, CanvasNodeSizingRequest};
use crate::geometry::ResizeEdge;
use crate::geometry::{
    is_vertical_resize_edge, requested_resize_height, requested_resize_width, resize_rect_by_edge,
};
use crate::tree::Tree;

const CANVAS_NODE_MIN_WIDTH: f32 = 304.0;
const CANVAS_NODE_MIN_HEIGHT: f32 = 132.0;

pub(crate) fn move_node_by(tree: &mut Tree, owner_id: &str, dx: f32, dy: f32) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id) else {
        return false;
    };
    runtime.rect.x += dx;
    runtime.rect.y += dy;
    connection_layer::mark_dirty(tree);
    true
}

pub(crate) fn resize_node_by(
    tree: &mut Tree,
    owner_id: &str,
    edge: ResizeEdge,
    dx: f32,
    dy: f32,
) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id) else {
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            owner_id,
            edge = ?edge,
            dx,
            dy,
            "ignore canvas node resize: runtime missing"
        );
        return false;
    };

    let before = runtime.rect;
    let requested_w = requested_resize_width(before, edge, dx);
    let requested_h = requested_resize_height(before, edge, dy);
    resize_rect_by_edge(
        &mut runtime.rect,
        edge,
        dx,
        dy,
        CANVAS_NODE_MIN_WIDTH,
        CANVAS_NODE_MIN_HEIGHT,
    );
    if is_vertical_resize_edge(edge) && runtime.rect.h != before.h {
        runtime.user_min_height = Some(runtime.rect.h);
    }
    tracing::trace!(
        target: "nodeimg::render_trace::node",
        owner_id,
        edge = ?edge,
        dx,
        dy,
        before_x = before.x,
        before_y = before.y,
        before_w = before.w,
        before_h = before.h,
        after_x = runtime.rect.x,
        after_y = runtime.rect.y,
        after_w = runtime.rect.w,
        after_h = runtime.rect.h,
        requested_w,
        requested_h,
        min_w = CANVAS_NODE_MIN_WIDTH,
        min_h = CANVAS_NODE_MIN_HEIGHT,
        clamped_w = requested_w < CANVAS_NODE_MIN_WIDTH,
        clamped_h = requested_h < CANVAS_NODE_MIN_HEIGHT,
        applied_dx = runtime.rect.x - before.x,
        applied_dy = runtime.rect.y - before.y,
        applied_dw = runtime.rect.w - before.w,
        applied_dh = runtime.rect.h - before.h,
        user_min_height = runtime.user_min_height,
        "resize canvas node runtime rect"
    );
    connection_layer::mark_dirty(tree);
    true
}

pub(crate) fn ensure_node_min_size(
    tree: &mut Tree,
    owner_id: &str,
    min_width: f32,
    min_height: f32,
) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id) else {
        return false;
    };

    let next_width = runtime.rect.w.max(min_width.max(CANVAS_NODE_MIN_WIDTH));
    let next_height = runtime.rect.h.max(min_height.max(CANVAS_NODE_MIN_HEIGHT));
    let changed = next_width != runtime.rect.w || next_height != runtime.rect.h;
    let before = runtime.rect;
    runtime.rect.w = next_width;
    runtime.rect.h = next_height;
    tracing::trace!(
        target: "nodeimg::render_trace::node",
        owner_id,
        requested_min_w = min_width,
        requested_min_h = min_height,
        engine_min_w = CANVAS_NODE_MIN_WIDTH,
        engine_min_h = CANVAS_NODE_MIN_HEIGHT,
        before_w = before.w,
        before_h = before.h,
        after_w = runtime.rect.w,
        after_h = runtime.rect.h,
        changed,
        "ensure canvas node min size"
    );
    if changed {
        connection_layer::mark_dirty(tree);
    }
    changed
}

pub(crate) fn apply_node_sizing(
    tree: &mut Tree,
    owner_id: &str,
    request: CanvasNodeSizingRequest,
) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    let Some(runtime) = tree.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id) else {
        return false;
    };

    let before = runtime.rect;
    runtime.rect.w = request
        .target_width
        .max(request.min_width)
        .max(CANVAS_NODE_MIN_WIDTH);
    runtime.rect.h = request
        .target_height
        .max(request.min_height)
        .max(CANVAS_NODE_MIN_HEIGHT);
    let changed = runtime.rect.w != before.w || runtime.rect.h != before.h;
    tracing::trace!(
        target: "nodeimg::render_trace::node",
        owner_id,
        target_w = request.target_width,
        target_h = request.target_height,
        request_min_w = request.min_width,
        request_min_h = request.min_height,
        engine_min_w = CANVAS_NODE_MIN_WIDTH,
        engine_min_h = CANVAS_NODE_MIN_HEIGHT,
        before_w = before.w,
        before_h = before.h,
        after_w = runtime.rect.w,
        after_h = runtime.rect.h,
        user_min_height = runtime.user_min_height,
        changed,
        "apply canvas node absolute sizing"
    );
    if changed {
        connection_layer::mark_dirty(tree);
    }
    changed
}
