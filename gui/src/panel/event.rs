use crate::output::ControlEvent;
use crate::tree::Tree;

pub(crate) fn apply_panel_control_event(tree: &mut Tree, event: &ControlEvent) -> bool {
    match event {
        ControlEvent::DragStart { id, x, y } if is_panel_control(tree, id) => {
            crate::panel::runtime::start_drag(tree, id, *x, *y);
            true
        }
        ControlEvent::DragMove { id, x, y } if is_panel_control(tree, id) => {
            crate::panel::runtime::move_drag(tree, id, *x, *y);
            true
        }
        ControlEvent::DragEnd { id, .. } if is_panel_control(tree, id) => {
            crate::panel::runtime::end_drag(tree);
            true
        }
        ControlEvent::ResizeStart { id, edge, x, y } if is_panel_control(tree, id) => {
            crate::panel::runtime::start_resize(tree, id, *edge, *x, *y);
            true
        }
        ControlEvent::ResizeMove { id, edge, x, y } if is_panel_control(tree, id) => {
            crate::panel::runtime::move_resize(tree, id, *edge, *x, *y);
            true
        }
        ControlEvent::ResizeEnd { id, edge, x, y } if is_panel_control(tree, id) => {
            crate::panel::runtime::end_resize(tree, id, *edge, *x, *y);
            true
        }
        _ => false,
    }
}

fn is_panel_control(tree: &Tree, id: &str) -> bool {
    tree.node_by_str(id)
        .and_then(|node_id| tree.get(node_id))
        .is_some_and(|node| node.props.semantic_role.is_some_and(|role| role.is_panel()))
}
