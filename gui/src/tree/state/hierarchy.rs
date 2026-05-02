use super::Tree;
use crate::tree::layout::{LayoutDependencyKind, LayoutDependencyScope, RelayoutBoundaryReason};
use crate::tree::node::{NodeId, TreeNode};
use crate::tree::repaint::RepaintBoundaryReason;

impl Tree {
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
        self.parents.remove(&id);
        if let Some(node) = self.get_mut(id) {
            node.layout_meta.set_boundary(RelayoutBoundaryReason::Root);
            node.paint_meta.set_boundary(RepaintBoundaryReason::Root);
        }
    }

    pub(crate) fn detach_from_parent(&mut self, child: NodeId) {
        if let Some(parent_id) = self.parents.remove(&child) {
            if let Some(parent) = self.get_mut(parent_id) {
                parent.children.retain(|candidate| *candidate != child);
                parent.paint_meta.bump_paint_order();
            }
            self.record_layout_dependency_change(
                parent_id,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
            return;
        }

        let mut changed_parents = Vec::new();
        for (parent_id, node) in self
            .nodes
            .iter_mut()
            .enumerate()
            .filter_map(|(node_id, node)| node.as_mut().map(|node| (node_id, node)))
        {
            let before = node.children.len();
            node.children.retain(|candidate| *candidate != child);
            if node.children.len() != before {
                node.paint_meta.bump_paint_order();
                changed_parents.push(parent_id);
            }
        }
        for parent_id in changed_parents {
            self.record_layout_dependency_change(
                parent_id,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
        }
        if self.root == Some(child) {
            self.root = None;
        }
    }

    pub(crate) fn append_child(&mut self, parent: NodeId, child: NodeId) -> bool {
        if self.get(child).is_none() {
            return false;
        }
        let Some(parent_node) = self.get_mut(parent) else {
            return false;
        };
        parent_node.children.push(child);
        parent_node.paint_meta.bump_paint_order();
        self.parents.insert(child, parent);
        self.record_layout_dependency_change(
            parent,
            LayoutDependencyKind::Children,
            LayoutDependencyScope::RelayoutBoundary,
        );
        self.hit_order_cache.borrow_mut().clear();
        self.paint_order_cache.borrow_mut().clear();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_children(&mut self, parent: NodeId, children: Vec<NodeId>) -> bool {
        if self.get(parent).is_none() {
            return false;
        }
        let old_children = self
            .get(parent)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        for child in old_children {
            self.parents.remove(&child);
        }
        self.reparent_children(parent, &children);
        let mut changed = false;
        if let Some(parent_node) = self.get_mut(parent) {
            if parent_node.children != children {
                parent_node.paint_meta.bump_paint_order();
                changed = true;
            }
            parent_node.children = children;
        }
        if changed {
            self.record_layout_dependency_change(
                parent,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
        }
        self.hit_order_cache.borrow_mut().clear();
        self.paint_order_cache.borrow_mut().clear();
        true
    }

    pub(in crate::tree::state) fn reparent_children(
        &mut self,
        parent: NodeId,
        children: &[NodeId],
    ) {
        for child in children {
            if self.get(*child).is_some() {
                self.parents.insert(*child, parent);
            }
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TreeNode> {
        self.nodes.iter_mut().filter_map(|n| n.as_mut())
    }

    pub(crate) fn children_in_hit_order_cached(
        &self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Vec<NodeId> {
        let (order, hit) = self
            .hit_order_cache
            .borrow_mut()
            .children_in_hit_order(self, parent, children);
        if hit {
            self.record_hit_order_cache_hit();
        } else {
            self.record_hit_order_cache_miss();
        }
        order
    }

    pub(crate) fn children_in_paint_order_cached(
        &self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Vec<NodeId> {
        let (order, hit) = self
            .paint_order_cache
            .borrow_mut()
            .children_in_paint_order(self, parent, children);
        if hit {
            self.record_paint_order_cache_hit();
        } else {
            self.record_paint_order_cache_miss();
        }
        order
    }

    pub fn parent_of(&self, node: NodeId) -> Option<NodeId> {
        if let Some(parent) = self.parents.get(&node).copied() {
            return Some(parent);
        }
        self.record_parent_lookup_fallback_scan();
        self.iter().find_map(|(candidate, tree_node)| {
            tree_node.children.contains(&node).then_some(candidate)
        })
    }
}
