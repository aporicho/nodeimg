use super::connection_layer;
use super::model::{CanvasInteractionRuntime, CanvasNodeRuntime, CANVAS_INTERACTION_ID};
use crate::canvas::{
    canvas_node_stable_id, CanvasPendingConnectionView, CanvasPortGroupView, CanvasPortSide,
};
use crate::tree::Tree;

pub(crate) fn port_group_view(
    tree: &Tree,
    owner_id: &str,
    side: CanvasPortSide,
) -> CanvasPortGroupView {
    tree.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .map(|interaction| interaction.port_group_view(owner_id, side))
        .unwrap_or_default()
}

pub(crate) fn toggle_port_group(tree: &mut Tree, owner_id: &str, side: CanvasPortSide) -> bool {
    tree.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .toggle_port_group(owner_id, side)
}

pub(crate) fn select_node(tree: &mut Tree, owner_id: &str) -> bool {
    let stable_id = canvas_node_stable_id(owner_id);
    if tree
        .runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id)
        .is_none()
    {
        return false;
    }
    tree.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .select_single(owner_id);
    true
}

pub(crate) fn clear_selection(tree: &mut Tree) -> bool {
    tree.runtime_slot_by_stable_id_mut::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .is_some_and(CanvasInteractionRuntime::clear)
}

pub(crate) fn is_node_selected(tree: &Tree, owner_id: &str) -> bool {
    tree.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .is_some_and(|interaction| interaction.is_selected(owner_id))
}

pub(crate) fn pending_connection(tree: &Tree) -> Option<CanvasPendingConnectionView> {
    tree.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .and_then(|interaction| interaction.pending_connection().cloned())
}

pub(crate) fn begin_pending_connection(
    tree: &mut Tree,
    from_port_id: &str,
    cursor_canvas: [f32; 2],
) -> bool {
    tree.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .begin_pending_connection(from_port_id, cursor_canvas)
}

pub(crate) fn update_pending_connection(tree: &mut Tree, cursor_canvas: [f32; 2]) -> bool {
    let changed = tree
        .ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .update_pending_connection(cursor_canvas);
    if changed {
        connection_layer::mark_dirty(tree);
    }
    changed
}

pub(crate) fn end_pending_connection(tree: &mut Tree) -> Option<CanvasPendingConnectionView> {
    tree.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .end_pending_connection()
}

pub(crate) fn cancel_pending_connection(tree: &mut Tree) -> bool {
    tree.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .cancel_pending_connection()
}

pub(crate) fn hovered_port_id(tree: &Tree) -> Option<String> {
    tree.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .and_then(|interaction| interaction.hovered_port_id().map(str::to_string))
}

pub(crate) fn set_hovered_port(tree: &mut Tree, port_id: Option<&str>) -> bool {
    let changed = tree
        .ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        .set_hovered_port(port_id);
    if changed {
        connection_layer::mark_dirty(tree);
    }
    changed
}
