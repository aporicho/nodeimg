use crate::control::ControlRole;
use crate::output::ControlEvent;
use crate::tree::Tree;

pub(crate) fn apply_panel_control_event(tree: &mut Tree, event: &ControlEvent) -> bool {
    match event {
        ControlEvent::DragStart { id, x, y } if is_panel_control(tree, id) => {
            tree.start_panel_drag(id, *x, *y);
            true
        }
        ControlEvent::DragMove { id, x, y } if is_panel_control(tree, id) => {
            tree.move_panel_drag(id, *x, *y);
            true
        }
        ControlEvent::DragEnd { id, .. } if is_panel_control(tree, id) => {
            tree.end_panel_drag();
            true
        }
        ControlEvent::ResizeStart { id, edge, x, y } if is_panel_control(tree, id) => {
            tree.start_panel_resize(id, *edge, *x, *y);
            true
        }
        ControlEvent::ResizeMove { id, edge, x, y } if is_panel_control(tree, id) => {
            tree.move_panel_resize(id, *edge, *x, *y);
            true
        }
        ControlEvent::ResizeEnd { id, .. } if is_panel_control(tree, id) => {
            tree.end_panel_resize();
            true
        }
        _ => false,
    }
}

fn is_panel_control(tree: &Tree, id: &str) -> bool {
    tree.node_by_str(id)
        .and_then(|node_id| tree.get(node_id))
        .is_some_and(|node| node.props.semantic_role == Some(ControlRole::Panel))
}
