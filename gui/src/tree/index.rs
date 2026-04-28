use super::{NodeId, StableId, TreeNode};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Default)]
pub struct TreeIndex {
    stable_to_node: HashMap<StableId, NodeId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeIndexError {
    DuplicateStableId {
        stable_id: StableId,
        existing: NodeId,
        duplicate: NodeId,
    },
    MissingIndexedNode {
        stable_id: StableId,
        node: NodeId,
    },
    StaleIndexEntry {
        stable_id: StableId,
        node: NodeId,
    },
}

impl TreeIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, stable_id: &StableId) -> Option<NodeId> {
        self.stable_to_node.get(stable_id).copied()
    }

    pub fn get_str(&self, stable_id: &str) -> Option<NodeId> {
        self.stable_to_node.get(stable_id).copied()
    }

    pub fn register(&mut self, stable_id: StableId, node: NodeId) -> Result<(), TreeIndexError> {
        if let Some(existing) = self.stable_to_node.get(&stable_id).copied() {
            if existing != node {
                return Err(TreeIndexError::DuplicateStableId {
                    stable_id,
                    existing,
                    duplicate: node,
                });
            }
        } else {
            self.stable_to_node.insert(stable_id, node);
        }
        Ok(())
    }

    pub fn unregister(&mut self, stable_id: &StableId, node: NodeId) {
        if self.stable_to_node.get(stable_id).copied() == Some(node) {
            self.stable_to_node.remove(stable_id);
        }
    }

    pub fn remove_node(&mut self, node: NodeId) {
        self.stable_to_node.retain(|_, indexed| *indexed != node);
    }

    pub fn validate<'a>(
        &self,
        live_nodes: impl IntoIterator<Item = (NodeId, &'a TreeNode)>,
    ) -> Result<(), TreeIndexError> {
        let mut seen = HashMap::<StableId, NodeId>::new();
        let mut live_ids = HashSet::<NodeId>::new();

        for (node_id, node) in live_nodes {
            live_ids.insert(node_id);
            let stable_id = node.id.clone();
            if let Some(existing) = seen.insert(stable_id.clone(), node_id) {
                return Err(TreeIndexError::DuplicateStableId {
                    stable_id,
                    existing,
                    duplicate: node_id,
                });
            }
            if self.get(&node.id) != Some(node_id) {
                return Err(TreeIndexError::MissingIndexedNode {
                    stable_id: node.id.clone(),
                    node: node_id,
                });
            }
        }

        for (stable_id, node) in &self.stable_to_node {
            if !live_ids.contains(node) {
                return Err(TreeIndexError::StaleIndexEntry {
                    stable_id: stable_id.clone(),
                    node: *node,
                });
            }
            if seen.get(stable_id).copied() != Some(*node) {
                return Err(TreeIndexError::StaleIndexEntry {
                    stable_id: stable_id.clone(),
                    node: *node,
                });
            }
        }

        Ok(())
    }
}
