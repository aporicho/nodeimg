use std::collections::HashMap;

use crate::tree::layout::RelayoutBoundaryReason;
use crate::tree::{NodeId, Revision, Tree};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct HitOrderCache {
    entries: HashMap<HitOrderCacheKey, Vec<NodeId>>,
}

impl HitOrderCache {
    pub(crate) fn children_in_hit_order(
        &mut self,
        tree: &Tree,
        parent: NodeId,
        children: &[NodeId],
    ) -> (Vec<NodeId>, bool) {
        let key = HitOrderCacheKey::from_tree(tree, parent, children);
        if let Some(order) = self.entries.get(&key) {
            return (order.clone(), true);
        }
        let mut indexed = children
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(source_index, node_id)| {
                tree.get(node_id).map(|node| HitOrderChild {
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
        indexed.reverse();
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
struct HitOrderCacheKey {
    parent: NodeId,
    parent_children_revision: Revision,
    child_revisions: Vec<HitOrderChildRevision>,
}

impl HitOrderCacheKey {
    fn from_tree(tree: &Tree, parent: NodeId, children: &[NodeId]) -> Self {
        let parent_children_revision = tree
            .get(parent)
            .map(|node| node.layout_meta.children_revision)
            .unwrap_or(Revision::ZERO);
        let child_revisions = children
            .iter()
            .copied()
            .filter_map(|node_id| {
                tree.get(node_id).map(|node| HitOrderChildRevision {
                    node_id,
                    z_index: node.style.z_index,
                    style_revision: node.layout_meta.style_revision,
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
            child_revisions,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct HitOrderChildRevision {
    node_id: NodeId,
    z_index: i32,
    style_revision: Revision,
    boundary: RelayoutBoundaryReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HitOrderChild {
    source_index: usize,
    node_id: NodeId,
    z_index: i32,
}
