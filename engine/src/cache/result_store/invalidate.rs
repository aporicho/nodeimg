use types::NodeId;
use types::Value;

use crate::cache::model::{CacheKey, GenerationId};

use super::lookup::ResultStore;

impl ResultStore {
    pub fn invalidate_output(&self, node_id: NodeId, output_pin: &str) -> usize {
        let keys: Vec<CacheKey> = {
            let entries = self
                .entries
                .read()
                .expect("result store read lock poisoned");
            entries
                .keys()
                .filter(|key| key.node_id == node_id && key.output_pin == output_pin)
                .cloned()
                .collect()
        };

        let removed = keys.len();
        for key in keys {
            let _ = self.remove(&key);
        }
        removed
    }

    pub fn invalidate_node(&self, node_id: NodeId) -> usize {
        let keys: Vec<CacheKey> = {
            let entries = self
                .entries
                .read()
                .expect("result store read lock poisoned");
            entries
                .keys()
                .filter(|key| key.node_id == node_id)
                .cloned()
                .collect()
        };

        let removed = keys.len();
        for key in keys {
            let _ = self.remove(&key);
        }
        removed
    }

    pub fn invalidate_subgraph(&self, node_ids: &[NodeId]) -> usize {
        let keys: Vec<CacheKey> = {
            let entries = self
                .entries
                .read()
                .expect("result store read lock poisoned");
            entries
                .keys()
                .filter(|key| node_ids.contains(&key.node_id))
                .cloned()
                .collect()
        };

        let removed = keys.len();
        for key in keys {
            let _ = self.remove(&key);
        }
        removed
    }

    pub fn remove_handles(&self) -> Vec<String> {
        let keys_and_handles: Vec<(CacheKey, String)> = {
            let entries = self
                .entries
                .read()
                .expect("result store read lock poisoned");
            entries
                .iter()
                .filter_map(|(key, entry)| match entry.value.as_ref() {
                    Value::Handle(handle) => Some((key.clone(), handle.handle_id.clone())),
                    _ => None,
                })
                .collect()
        };

        let mut handle_ids = Vec::with_capacity(keys_and_handles.len());
        for (key, handle_id) in keys_and_handles {
            let _ = self.remove(&key);
            handle_ids.push(handle_id);
        }
        handle_ids
    }

    pub fn remove_stale_handles(&self, current_generation: GenerationId) -> Vec<String> {
        let keys_and_handles: Vec<(CacheKey, String)> = {
            let entries = self
                .entries
                .read()
                .expect("result store read lock poisoned");
            entries
                .iter()
                .filter_map(|(key, entry)| match entry.value.as_ref() {
                    Value::Handle(handle) if entry.generation != current_generation => {
                        Some((key.clone(), handle.handle_id.clone()))
                    }
                    _ => None,
                })
                .collect()
        };

        let mut handle_ids = Vec::with_capacity(keys_and_handles.len());
        for (key, handle_id) in keys_and_handles {
            let _ = self.remove(&key);
            handle_ids.push(handle_id);
        }
        handle_ids
    }
}

#[cfg(test)]
mod tests {
    use types::{NodeId, Value};

    use crate::cache::model::{CacheKey, ExecSignature, GenerationId, ResultEntry};

    use super::ResultStore;

    fn insert_entry(store: &ResultStore, node_id: NodeId, output_pin: &str, sig: u64) {
        store.insert(
            CacheKey::new(node_id, output_pin, ExecSignature::new(1, 1, sig, 0)),
            ResultEntry::new(
                GenerationId::initial(),
                std::sync::Arc::new(Value::Int(sig as i64)),
                0,
            ),
        );
    }

    #[test]
    fn invalidate_output_removes_only_matching_output() {
        let store = ResultStore::new();
        insert_entry(&store, NodeId(1), "image", 1);
        insert_entry(&store, NodeId(1), "mask", 2);

        let removed = store.invalidate_output(NodeId(1), "image");

        assert_eq!(removed, 1);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn invalidate_node_removes_all_node_entries() {
        let store = ResultStore::new();
        insert_entry(&store, NodeId(1), "image", 1);
        insert_entry(&store, NodeId(1), "mask", 2);
        insert_entry(&store, NodeId(2), "image", 3);

        let removed = store.invalidate_node(NodeId(1));

        assert_eq!(removed, 2);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn invalidate_subgraph_removes_selected_nodes() {
        let store = ResultStore::new();
        insert_entry(&store, NodeId(1), "image", 1);
        insert_entry(&store, NodeId(2), "image", 2);
        insert_entry(&store, NodeId(3), "image", 3);

        let removed = store.invalidate_subgraph(&[NodeId(1), NodeId(3)]);

        assert_eq!(removed, 2);
        assert_eq!(store.len(), 1);
    }
}
