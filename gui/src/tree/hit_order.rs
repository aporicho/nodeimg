use std::collections::HashMap;

use super::layout::RelayoutBoundaryReason;
use super::Tree;
use super::{NodeId, Revision};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::tree::layout::BoxStyle;
    use crate::tree::node::{NodeKind, NodeLocalRuntime, TreeNode};
    use crate::tree::{NodeProps, RuntimeSlots, StableId};

    fn container(id: &'static str, z_index: i32) -> TreeNode {
        TreeNode {
            id: StableId::from(id),
            props: NodeProps::default(),
            style: BoxStyle {
                z_index,
                ..Default::default()
            },
            decoration: None,
            kind: NodeKind::Container,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    #[test]
    fn hit_order_cache_reuses_order_until_child_revision_changes() {
        let mut tree = Tree::new();
        let first = tree.insert(container("first", 1));
        let second = tree.insert(container("second", 2));
        let parent = tree.insert(container("parent", 0));
        tree.set_children(parent, vec![first, second]);
        let mut cache = HitOrderCache::default();

        let (first_order, first_hit) = cache.children_in_hit_order(&tree, parent, &[first, second]);
        let (second_order, second_hit) =
            cache.children_in_hit_order(&tree, parent, &[first, second]);

        assert!(!first_hit);
        assert!(second_hit);
        assert_eq!(first_order, vec![second, first]);
        assert_eq!(second_order, first_order);
    }
}
