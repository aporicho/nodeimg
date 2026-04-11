use std::collections::{HashMap, HashSet};

use types::NodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DirtyReason {
    ParamChanged,
    UpstreamChanged,
    StructureChanged,
    ArtifactSelectionChanged,
    Forced,
}

#[derive(Clone, Debug, Default)]
pub struct DirtyState {
    pub dirty_nodes: HashSet<NodeId>,
    pub dirty_reasons: HashMap<NodeId, DirtyReason>,
}

impl DirtyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark(&mut self, node_id: NodeId, reason: DirtyReason) {
        self.dirty_nodes.insert(node_id);
        self.dirty_reasons.insert(node_id, reason);
    }

    pub fn contains(&self, node_id: &NodeId) -> bool {
        self.dirty_nodes.contains(node_id)
    }

    pub fn clear(&mut self) {
        self.dirty_nodes.clear();
        self.dirty_reasons.clear();
    }

    pub fn len(&self) -> usize {
        self.dirty_nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dirty_nodes.is_empty()
    }
}
