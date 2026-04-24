use super::runtime::OverlayState;
use crate::animation::AnimationStore;
use crate::interaction::InteractionState;
use crate::shell::{AppEvent, MouseButton};
use crate::tree::{NodeId, Tree};

pub(crate) fn close_overlay(
    state: Option<OverlayState>,
    tree: &Tree,
    interaction: &mut InteractionState,
) {
    let Some(state) = state else {
        return;
    };
    let restore_node_id = if state.request.restore_focus_to_anchor {
        let restore_id = state
            .request
            .restore_focus_id
            .as_deref()
            .unwrap_or(&state.request.anchor_id);
        find_node_id_by_str(tree, restore_id)
    } else {
        None
    };
    if let Some(node_id) = restore_node_id {
        interaction.focus(node_id);
    }
}

pub(crate) fn handle_dismiss_event(
    state: &OverlayState,
    tree: &Tree,
    animations: Option<&AnimationStore>,
    event: &AppEvent,
) -> DismissOutcome {
    match *event {
        AppEvent::KeyPress {
            key: crate::shell::Key::Escape,
            ..
        } if state.request.dismiss_on_escape => DismissOutcome::CloseAndConsume,
        AppEvent::MousePress { x, y, button }
            if button == MouseButton::Left && state.request.dismiss_on_outside_click =>
        {
            if hit_overlay(tree, animations, &state.request.id, x, y) {
                DismissOutcome::Keep
            } else {
                DismissOutcome::CloseNoFocusRestore
            }
        }
        _ => DismissOutcome::Keep,
    }
}

pub(crate) enum DismissOutcome {
    Keep,
    CloseAndConsume,
    CloseNoFocusRestore,
}

fn hit_overlay(
    tree: &Tree,
    animations: Option<&AnimationStore>,
    overlay_id: &str,
    x: f32,
    y: f32,
) -> bool {
    let Some(root) = tree.root() else {
        return false;
    };
    let prefix = format!("__overlay::{overlay_id}");
    crate::tree::hit_test_with_animations(tree, root, x, y, animations)
        .iter()
        .filter_map(|node_id| tree.get(node_id))
        .any(|node| node.id.as_ref().starts_with(&prefix))
}

fn find_node_id_by_str(tree: &Tree, node_id: &str) -> Option<NodeId> {
    tree.iter()
        .find_map(|(id, node)| (node.id.as_ref() == node_id).then_some(id))
}
