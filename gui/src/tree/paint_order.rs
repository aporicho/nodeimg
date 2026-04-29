use super::layout::RelayoutBoundaryReason;
use super::{NodeId, Revision, Tree};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PaintOrderCache {
    entries: HashMap<PaintOrderCacheKey, Vec<NodeId>>,
}

impl PaintOrderCache {
    pub(crate) fn children_in_paint_order(
        &mut self,
        tree: &Tree,
        parent: NodeId,
        children: &[NodeId],
    ) -> (Vec<NodeId>, bool) {
        let key = PaintOrderCacheKey::from_tree(tree, parent, children);
        if let Some(order) = self.entries.get(&key) {
            return (order.clone(), true);
        }

        let mut indexed = children
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(source_index, node_id)| {
                tree.get(node_id).map(|node| PaintOrderChild {
                    source_index,
                    node_id,
                    z_index: node.style.z_index,
                })
            })
            .collect::<Vec<_>>();

        indexed.sort_by(|a, b| {
            a.z_index
                .cmp(&b.z_index)
                .then_with(|| a.source_index.cmp(&b.source_index))
        });
        let order = indexed
            .into_iter()
            .map(|child| child.node_id)
            .collect::<Vec<_>>();
        self.entries.insert(key, order.clone());
        (order, false)
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PaintOrderCacheKey {
    parent: NodeId,
    parent_children_revision: Revision,
    parent_paint_order_revision: Revision,
    child_revisions: Vec<PaintOrderChildRevision>,
}

impl PaintOrderCacheKey {
    fn from_tree(tree: &Tree, parent: NodeId, children: &[NodeId]) -> Self {
        let (parent_children_revision, parent_paint_order_revision) = tree
            .get(parent)
            .map(|node| {
                (
                    node.layout_meta.children_revision,
                    node.paint_meta.paint_order_revision,
                )
            })
            .unwrap_or((Revision::ZERO, Revision::ZERO));
        let child_revisions = children
            .iter()
            .copied()
            .filter_map(|node_id| {
                tree.get(node_id).map(|node| PaintOrderChildRevision {
                    node_id,
                    z_index: node.style.z_index,
                    style_revision: node.layout_meta.style_revision,
                    paint_order_revision: node.paint_meta.paint_order_revision,
                    boundary: node
                        .layout_meta
                        .boundary
                        .unwrap_or(RelayoutBoundaryReason::Explicit),
                })
            })
            .collect();
        Self {
            parent,
            parent_children_revision,
            parent_paint_order_revision,
            child_revisions,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PaintOrderChildRevision {
    node_id: NodeId,
    z_index: i32,
    style_revision: Revision,
    paint_order_revision: Revision,
    boundary: RelayoutBoundaryReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PaintOrderChild {
    source_index: usize,
    node_id: NodeId,
    z_index: i32,
}
