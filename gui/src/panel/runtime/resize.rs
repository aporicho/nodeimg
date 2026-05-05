use super::model::{PanelResizeSession, PanelRootRuntime};
use super::store::{bring_to_front, panel_state, panel_state_mut, PANEL_ROOT_ID};
use crate::geometry::ResizeEdge;
use crate::geometry::{requested_resize_height, requested_resize_width, resize_rect_by_edge};
use crate::tree::Tree;

pub(crate) fn start_resize(tree: &mut Tree, id: &str, edge: ResizeEdge, x: f32, y: f32) {
    let Some(start_rect) = panel_state(tree, id).map(|panel| panel.rect) else {
        tracing::trace!(
            target: "gui::panel",
            panel_id = id,
            edge = ?edge,
            x,
            y,
            "skip panel resize start because panel runtime is missing"
        );
        return;
    };
    bring_to_front(tree, id);
    let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
    root.active_resize = Some(PanelResizeSession {
        id: id.to_string(),
        edge,
        start_x: x,
        start_y: y,
        start_rect,
    });
    tracing::trace!(
        target: "gui::panel",
        panel_id = id,
        edge = ?edge,
        x,
        y,
        start_rect_x = start_rect.x,
        start_rect_y = start_rect.y,
        start_rect_w = start_rect.w,
        start_rect_h = start_rect.h,
        "panel resize session started"
    );
}

pub(crate) fn move_resize(tree: &mut Tree, id: &str, edge: ResizeEdge, x: f32, y: f32) {
    let Some(session) = ({
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        let Some(session) = root.active_resize.as_ref() else {
            tracing::trace!(
                target: "gui::panel",
                panel_id = id,
                edge = ?edge,
                x,
                y,
                "skip panel resize move because there is no active session"
            );
            return;
        };
        if session.id != id || session.edge != edge {
            tracing::trace!(
                target: "gui::panel",
                panel_id = id,
                edge = ?edge,
                active_panel_id = %session.id,
                active_edge = ?session.edge,
                x,
                y,
                "skip panel resize move because active session does not match event"
            );
            return;
        }
        Some(session.clone())
    }) else {
        return;
    };
    resize_from_session(tree, &session, x, y);
}

pub(crate) fn end_resize(tree: &mut Tree, id: &str, edge: ResizeEdge, x: f32, y: f32) {
    let session = {
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_resize.as_ref().cloned()
    };
    if let Some(session) = session {
        if session.id == id && session.edge == edge {
            resize_from_session(tree, &session, x, y);
        }
    }
    let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
    tracing::trace!(
        target: "gui::panel",
        panel_id = id,
        edge = ?edge,
        active_panel_id = ?root.active_resize.as_ref().map(|session| session.id.as_str()),
        active_edge = ?root.active_resize.as_ref().map(|session| session.edge),
        x,
        y,
        "panel resize session ended"
    );
    root.active_resize = None;
}

fn resize_from_session(tree: &mut Tree, session: &PanelResizeSession, x: f32, y: f32) {
    let Some(panel) = panel_state_mut(tree, &session.id) else {
        return;
    };
    let dx = x - session.start_x;
    let dy = y - session.start_y;
    let requested_w = requested_resize_width(session.start_rect, session.edge, dx);
    let requested_h = requested_resize_height(session.start_rect, session.edge, dy);
    let before = panel.rect;
    let mut next = session.start_rect;
    resize_rect_by_edge(
        &mut next,
        session.edge,
        dx,
        dy,
        panel.min_size[0],
        panel.min_size[1],
    );
    panel.rect = next;
    tracing::trace!(
        target: "gui::panel",
        panel_id = %session.id,
        edge = ?session.edge,
        x,
        y,
        start_x = session.start_x,
        start_y = session.start_y,
        dx,
        dy,
        start_rect_x = session.start_rect.x,
        start_rect_y = session.start_rect.y,
        start_rect_w = session.start_rect.w,
        start_rect_h = session.start_rect.h,
        before_x = before.x,
        before_y = before.y,
        before_w = before.w,
        before_h = before.h,
        after_x = panel.rect.x,
        after_y = panel.rect.y,
        after_w = panel.rect.w,
        after_h = panel.rect.h,
        requested_w,
        requested_h,
        min_w = panel.min_size[0],
        min_h = panel.min_size[1],
        clamped_w = requested_w < panel.min_size[0],
        clamped_h = requested_h < panel.min_size[1],
        "resize panel runtime rect from pointer session"
    );
}
