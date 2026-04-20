use crate::output::PanelEvent;
use crate::tree::Tree;

pub(crate) fn apply_panel_event(tree: &mut Tree, event: &PanelEvent) -> bool {
    match event {
        PanelEvent::DragStart { id, x, y } => {
            tree.start_panel_drag(id, *x, *y);
            true
        }
        PanelEvent::DragMove { id, x, y } => {
            tree.move_panel_drag(id, *x, *y);
            true
        }
        PanelEvent::DragEnd { .. } => {
            tree.end_panel_drag();
            true
        }
        PanelEvent::ResizeStart { id, edge, x, y } => {
            tree.start_panel_resize(id, *edge, *x, *y);
            true
        }
        PanelEvent::ResizeMove { id, edge, x, y } => {
            tree.move_panel_resize(id, *edge, *x, *y);
            true
        }
        PanelEvent::ResizeEnd { .. } => {
            tree.end_panel_resize();
            true
        }
    }
}
