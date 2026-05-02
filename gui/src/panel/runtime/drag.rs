use super::model::{PanelPointerSession, PanelRootRuntime};
use super::store::{bring_to_front, panel_state_mut, PANEL_ROOT_ID};
use crate::tree::Tree;

pub(crate) fn move_by(tree: &mut Tree, id: &str, dx: f32, dy: f32) {
    let Some(panel) = panel_state_mut(tree, id) else {
        return;
    };
    panel.rect.x += dx;
    panel.rect.y += dy;
}

pub(crate) fn start_drag(tree: &mut Tree, id: &str, x: f32, y: f32) {
    bring_to_front(tree, id);
    let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
    root.active_drag = Some(PanelPointerSession {
        id: id.to_string(),
        last_x: x,
        last_y: y,
    });
}

pub(crate) fn move_drag(tree: &mut Tree, id: &str, x: f32, y: f32) {
    let Some((dx, dy)) = ({
        let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        let Some(session) = root.active_drag.as_mut() else {
            return;
        };
        if session.id != id {
            return;
        }
        let dx = x - session.last_x;
        let dy = y - session.last_y;
        session.last_x = x;
        session.last_y = y;
        Some((dx, dy))
    }) else {
        return;
    };
    move_by(tree, id, dx, dy);
}

pub(crate) fn end_drag(tree: &mut Tree) {
    let root = tree.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
    root.active_drag = None;
}
