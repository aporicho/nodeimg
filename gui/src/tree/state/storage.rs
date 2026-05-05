use super::Tree;
use crate::tree::index::TreeIndexError;
use crate::tree::node::{NodeId, TreeNode};
use crate::tree::repaint::RepaintBoundaryId;
use crate::tree::StableId;

impl Tree {
    pub fn insert(&mut self, node: impl Into<TreeNode>) -> NodeId {
        self.insert_with_index_policy(node.into(), false)
            .expect("unchecked insertion does not reject duplicate stable ids")
    }

    pub fn insert_checked(&mut self, node: impl Into<TreeNode>) -> Result<NodeId, TreeIndexError> {
        self.insert_with_index_policy(node.into(), true)
    }

    fn insert_with_index_policy(
        &mut self,
        mut node: TreeNode,
        reject_duplicate: bool,
    ) -> Result<NodeId, TreeIndexError> {
        let retained_slots = self.retained_runtime.take(node.id.as_ref());
        if let Some(slots) = retained_slots {
            if node.runtime_slots.is_empty() {
                node.runtime_slots = slots;
            } else {
                node.runtime_slots.fill_missing_from(slots);
            }
        }
        if let Some(id) = self.free.pop() {
            self.index.remove_node(id);
            self.parents.remove(&id);
            self.layout_cache.clear();
            self.hit_order_cache.borrow_mut().clear();
            self.paint_cache.borrow_mut().evict(RepaintBoundaryId(id));
            self.paint_order_cache.borrow_mut().clear();
            self.paint_dirty.remove_node(id);
            self.layout_dirty.boundaries.remove(&id);
            self.layout_dirty.text_nodes.remove(&id);
            let stable_id = node.id.clone();
            let children = node.children.clone();
            self.nodes[id] = Some(node);
            if let Err(error) = self.index.register(stable_id, id) {
                if reject_duplicate {
                    self.nodes[id] = None;
                    self.free.push(id);
                    return Err(error);
                }
            }
            self.reparent_children(id, &children);
            Ok(id)
        } else {
            let id = self.nodes.len();
            let stable_id = node.id.clone();
            let children = node.children.clone();
            self.nodes.push(Some(node));
            if let Err(error) = self.index.register(stable_id, id) {
                if reject_duplicate {
                    self.nodes.pop();
                    return Err(error);
                }
            }
            self.reparent_children(id, &children);
            Ok(id)
        }
    }

    pub fn remove(&mut self, id: NodeId) {
        if id < self.nodes.len() {
            let evicted = self.paint_cache.borrow_mut().evict_subtree(self, id);
            self.record_paint_fragments_evicted(evicted);
            // 递归删除子节点
            if let Some(node) = self.nodes[id].take() {
                self.index.unregister(&node.id, id);
                self.parents.remove(&id);
                self.layout_dirty.boundaries.remove(&id);
                self.layout_dirty.text_nodes.remove(&id);
                self.paint_dirty.remove_node(id);
                self.layout_cache.clear();
                self.hit_order_cache.borrow_mut().clear();
                self.paint_order_cache.borrow_mut().clear();
                let children = node.children;
                for child_id in children {
                    self.parents.remove(&child_id);
                    self.remove(child_id);
                }
                if let Some(runtime_slots) = node.runtime_slots.retained_when_node_missing() {
                    self.retained_runtime
                        .preserve(node.id.as_ref().to_string(), runtime_slots);
                }
                self.free.push(id);
            }
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut TreeNode> {
        self.nodes.get_mut(id).and_then(|n| n.as_mut())
    }

    pub fn node_by_stable_id(&self, stable_id: &StableId) -> Option<NodeId> {
        self.record_stable_id_lookup();
        self.index.get(stable_id)
    }

    pub fn node_by_str(&self, stable_id: &str) -> Option<NodeId> {
        self.record_stable_id_lookup();
        self.index.get_str(stable_id)
    }

    pub fn contains_stable_id(&self, stable_id: &str) -> bool {
        self.node_by_str(stable_id).is_some()
    }

    pub fn validate_index(&self) -> Result<(), TreeIndexError> {
        self.index.validate(self.iter())
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &TreeNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| node.as_ref().map(|node| (id, node)))
    }
}
