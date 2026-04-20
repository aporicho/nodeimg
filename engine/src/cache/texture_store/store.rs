use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use types::GpuTexture;
use types::NodeId;

use crate::cache::model::{PreviewRequest, TextureEntry, TextureKey};

use super::lru::TextureLru;

#[derive(Debug, Default)]
pub struct TextureStore {
    entries: RwLock<HashMap<TextureKey, TextureEntry>>,
    lru: RwLock<TextureLru>,
}

impl TextureStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, request: &PreviewRequest) -> Option<Arc<GpuTexture>> {
        let entries = self
            .entries
            .read()
            .expect("texture store read lock poisoned");
        let key = TextureKey::new(request.node_id, request.output_pin.clone());
        let entry = entries.get(&key)?;

        if entry.generation != request.generation {
            return None;
        }

        let texture = Arc::clone(&entry.texture);
        drop(entries);

        self.lru
            .write()
            .expect("texture store lru lock poisoned")
            .touch(&key);

        Some(texture)
    }

    pub fn get_entry(&self, request: &PreviewRequest) -> Option<TextureEntry> {
        let entries = self
            .entries
            .read()
            .expect("texture store read lock poisoned");
        let key = TextureKey::new(request.node_id, request.output_pin.clone());
        let entry = entries.get(&key)?;

        if entry.generation != request.generation {
            return None;
        }

        Some(entry.clone())
    }

    pub fn insert(&self, key: TextureKey, entry: TextureEntry) -> Option<TextureEntry> {
        let mut entries = self
            .entries
            .write()
            .expect("texture store write lock poisoned");
        let previous = entries.insert(key.clone(), entry);
        drop(entries);

        self.lru
            .write()
            .expect("texture store lru lock poisoned")
            .touch(&key);

        previous
    }

    pub fn invalidate(&self, key: &TextureKey) -> Option<TextureEntry> {
        let mut entries = self
            .entries
            .write()
            .expect("texture store write lock poisoned");
        let removed = entries.remove(key);
        drop(entries);

        self.lru
            .write()
            .expect("texture store lru lock poisoned")
            .remove(key);

        removed
    }

    pub fn invalidate_node(&self, node_id: NodeId) -> usize {
        let mut entries = self
            .entries
            .write()
            .expect("texture store write lock poisoned");
        let before = entries.len();
        entries.retain(|key, _| key.node_id != node_id);
        before - entries.len()
    }

    pub fn invalidate_subgraph(&self, node_ids: &[NodeId]) -> usize {
        let mut entries = self
            .entries
            .write()
            .expect("texture store write lock poisoned");
        let before = entries.len();
        entries.retain(|key, _| !node_ids.contains(&key.node_id));
        before - entries.len()
    }

    pub fn clear(&self) {
        let mut entries = self
            .entries
            .write()
            .expect("texture store write lock poisoned");
        entries.clear();
        drop(entries);

        self.lru
            .write()
            .expect("texture store lru lock poisoned")
            .clear();
    }

    pub fn total_bytes(&self) -> usize {
        let entries = self
            .entries
            .read()
            .expect("texture store read lock poisoned");
        entries.values().map(|entry| entry.bytes).sum()
    }

    pub fn evict_to_budget(&self, budget: crate::cache::model::BudgetWatermark) -> usize {
        let mut evicted = 0;

        loop {
            if self.total_bytes() <= budget.soft {
                break;
            }

            let Some(key) = self
                .lru
                .write()
                .expect("texture store lru lock poisoned")
                .pop_lru()
            else {
                break;
            };

            if self.invalidate(&key).is_some() {
                evicted += 1;
            }
        }

        evicted
    }
}

#[cfg(test)]
mod tests {
    use types::NodeId;

    use crate::cache::model::{ExecSignature, GenerationId, PreviewRequest, TextureKey};

    use super::TextureStore;

    #[test]
    fn empty_store_misses_preview_request() {
        let store = TextureStore::new();

        let request = PreviewRequest::new(
            NodeId(1),
            "image",
            ExecSignature::new(1, 1, 1, 1),
            GenerationId::initial(),
        );

        assert!(store.get_entry(&request).is_none());
    }

    #[test]
    fn invalidate_missing_key_returns_none() {
        let store = TextureStore::new();

        assert!(store
            .invalidate(&TextureKey::new(NodeId(1), "image"))
            .is_none());
    }

    #[test]
    fn invalidate_node_on_empty_store_removes_nothing() {
        let store = TextureStore::new();

        assert_eq!(store.invalidate_node(NodeId(1)), 0);
    }

    #[test]
    fn invalidate_subgraph_on_empty_store_removes_nothing() {
        let store = TextureStore::new();

        assert_eq!(store.invalidate_subgraph(&[NodeId(1), NodeId(2)]), 0);
    }
}
