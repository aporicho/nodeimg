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
    let mut flags = DirtyFlags::NONE;

    if size_changed {
        flags |= size_invalidation_flags();
    }
    if position_changed {
        flags |= position_invalidation_flags(tree, node);
    }

    RectInvalidation {
        flags,
        size_changed,
        position_changed,
    }
}

fn size_invalidation_flags() -> DirtyFlags {
    DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT
}

fn position_invalidation_flags(tree: &Tree, node: NodeId) -> DirtyFlags {
    match tree
        .get(node)
        .map(|tree_node| tree_node.mutation_meta.rect_move)
        .unwrap_or_default()
    {
        RectMoveInvalidation::Layout => DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT,
        RectMoveInvalidation::Repaint => DirtyFlags::PAINT | DirtyFlags::HIT,
        RectMoveInvalidation::BoundaryPlacement => DirtyFlags::PAINT_PLACEMENT | DirtyFlags::HIT,
        RectMoveInvalidation::LayoutAndBoundaryPlacement => {
            DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT | DirtyFlags::PAINT_PLACEMENT
        }
    }
}
