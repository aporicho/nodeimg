use crate::node::types::Value;
use std::collections::{HashMap, HashSet};

pub type NodeId = usize;

pub struct Cache {
    results: HashMap<NodeId, HashMap<String, Value>>,
    /// Downstream adjacency: node → set of nodes that depend on it.
    downstream: HashMap<NodeId, HashSet<NodeId>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            downstream: HashMap::new(),
        }
    }

    pub fn set_downstream(&mut self, from: NodeId, to: NodeId) {
        self.downstream.entry(from).or_default().insert(to);
    }

    pub fn clear_downstream(&mut self) {
        self.downstream.clear();
    }

    pub fn get(&self, node_id: NodeId) -> Option<&HashMap<String, Value>> {
        self.results.get(&node_id)
    }

    pub fn insert(&mut self, node_id: NodeId, outputs: HashMap<String, Value>) {
        self.results.insert(node_id, outputs);
    }

    /// Invalidate this node and all downstream nodes.
    pub fn invalidate(&mut self, node_id: NodeId) {
        self.results.remove(&node_id);
        if let Some(downstream) = self.downstream.get(&node_id).cloned() {
            for d in downstream {
                self.invalidate(d);
            }
        }
    }

    /// Invalidate all cached results.
    pub fn invalidate_all(&mut self) {
        self.results.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = Cache::new();
        let outputs = HashMap::from([("image".into(), Value::Float(1.0))]);
        cache.insert(0, outputs);
        assert!(cache.get(0).is_some());
        assert!(cache.get(1).is_none());
    }

    #[test]
    fn test_local_invalidation() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::new());
        cache.insert(1, HashMap::new());
        cache.insert(2, HashMap::new());

        // 0 → 1 → 2
        cache.set_downstream(0, 1);
        cache.set_downstream(1, 2);

        cache.invalidate(0);
        assert!(cache.get(0).is_none());
        assert!(cache.get(1).is_none());
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_partial_invalidation() {
        let mut cache = Cache::new();
        cache.insert(0, HashMap::new());
        cache.insert(1, HashMap::new());
        cache.insert(2, HashMap::new());

        // 0 → 1, 0 → 2 (independent branches)
        cache.set_downstream(0, 1);

        cache.invalidate(1);
        assert!(cache.get(0).is_some()); // upstream unaffected
        assert!(cache.get(1).is_none()); // invalidated
        assert!(cache.get(2).is_some()); // independent branch unaffected
    }
}
