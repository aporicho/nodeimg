use super::{DirtyFlags, NodeId, RectMoveInvalidation, Tree};
use crate::renderer::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RectInvalidation {
    pub(crate) flags: DirtyFlags,
    pub(crate) size_changed: bool,
    pub(crate) position_changed: bool,
}

pub(crate) fn rect_invalidation(
    tree: &Tree,
    node: NodeId,
    old: Rect,
    new: Rect,
) -> RectInvalidation {
    let size_changed = old.w != new.w || old.h != new.h;
    let position_changed = old.x != new.x || old.y != new.y;
    let flags = if size_changed {
        DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT
    } else if position_changed {
        match tree
            .get(node)
            .map(|tree_node| tree_node.mutation_meta.rect_move)
            .unwrap_or_default()
        {
            RectMoveInvalidation::Layout => {
                DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT
            }
            RectMoveInvalidation::Repaint => DirtyFlags::PAINT | DirtyFlags::HIT,
            RectMoveInvalidation::BoundaryPlacement => {
                DirtyFlags::PAINT_PLACEMENT | DirtyFlags::HIT
            }
            RectMoveInvalidation::LayoutAndBoundaryPlacement => {
                DirtyFlags::LAYOUT
                    | DirtyFlags::HIT
                    | DirtyFlags::PAINT
                    | DirtyFlags::PAINT_PLACEMENT
            }
        }
    } else {
        DirtyFlags::NONE
    };

    RectInvalidation {
        flags,
        size_changed,
        position_changed,
    }
}
