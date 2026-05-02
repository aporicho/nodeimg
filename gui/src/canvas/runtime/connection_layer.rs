use crate::tree::{PaintDirtyReason, Tree};

pub(super) fn mark_dirty(tree: &mut Tree) {
    if let Some(layer) = tree.node_by_str("canvas_connections") {
        tree.mark_paint_dirty(layer, PaintDirtyReason::Visual);
    }
}
