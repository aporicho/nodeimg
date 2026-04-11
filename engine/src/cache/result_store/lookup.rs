use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use types::Value;

use crate::cache::model::{CacheKey, ResultEntry};

use super::lru::ResultLru;

#[derive(Debug, Default)]
pub struct ResultStore {
    pub(crate) entries: RwLock<HashMap<CacheKey, ResultEntry>>,
    pub(crate) lru: RwLock<ResultLru>,
}

impl ResultStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &CacheKey) -> Option<Arc<Value>> {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        let value = entries.get(key).map(|entry| Arc::clone(&entry.value));
        drop(entries);

        if matches!(value.as_deref(), Some(v) if !matches!(v, Value::Handle(_))) {
            self.lru
                .write()
                .expect("result store lru lock poisoned")
                .touch(key);
        }

        value
    }

    pub fn get_entry(&self, key: &CacheKey) -> Option<ResultEntry> {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        entries.get(key).cloned()
    }

    pub fn insert(&self, key: CacheKey, entry: ResultEntry) -> Option<ResultEntry> {
        let should_touch_lru = !matches!(entry.value.as_ref(), Value::Handle(_));
        let mut entries = self
            .entries
            .write()
            .expect("result store write lock poisoned");
        let previous = entries.insert(key.clone(), entry);
        drop(entries);

        if should_touch_lru {
            self.lru
                .write()
                .expect("result store lru lock poisoned")
                .touch(&key);
        }

        previous
    }

    pub fn contains(&self, key: &CacheKey) -> bool {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        let mut entries = self
            .entries
            .write()
            .expect("result store write lock poisoned");
        entries.clear();
        drop(entries);

        self.lru
            .write()
            .expect("result store lru lock poisoned")
            .clear();
    }

    pub fn total_bytes(&self) -> usize {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        entries.values().map(|entry| entry.bytes).sum()
    }

    pub fn total_handle_bytes(&self) -> usize {
        let entries = self
            .entries
            .read()
            .expect("result store read lock poisoned");
        entries
            .values()
            .filter_map(|entry| match entry.value.as_ref() {
                Value::Handle(handle) => Some(handle.bytes),
                _ => None,
            })
            .sum()
    }

    pub(crate) fn remove(&self, key: &CacheKey) -> Option<ResultEntry> {
        let mut entries = self
            .entries
            .write()
            .expect("result store write lock poisoned");
        let removed = entries.remove(key);
        drop(entries);

        self.lru
            .write()
            .expect("result store lru lock poisoned")
            .remove(key);

        removed
    }

    pub(crate) fn pop_lru_key(&self) -> Option<CacheKey> {
        self.lru
            .write()
            .expect("result store lru lock poisoned")
            .pop_lru()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use types::{NodeId, Value};

    use crate::cache::model::{CacheKey, ExecSignature, GenerationId, ResultEntry};

    use super::ResultStore;

    fn key(params_hash: u64) -> CacheKey {
        CacheKey::new(
            NodeId(7),
            "image",
            ExecSignature::new(1, 1, params_hash, 99),
        )
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let store = ResultStore::new();
        let entry = ResultEntry::new(GenerationId::initial(), Arc::new(Value::Int(42)), 0);

        store.insert(key(1), entry);

        let value = store.get(&key(1)).expect("cached value");
        assert!(matches!(value.as_ref(), Value::Int(42)));
    }

    #[test]
    fn different_signatures_can_coexist() {
        let store = ResultStore::new();

        store.insert(
            key(1),
            ResultEntry::new(GenerationId::initial(), Arc::new(Value::Int(1)), 0),
        );
        store.insert(
            key(2),
            ResultEntry::new(GenerationId::initial(), Arc::new(Value::Int(2)), 0),
        );

        assert_eq!(store.len(), 2);
        assert!(matches!(store.get(&key(1)).as_deref(), Some(Value::Int(1))));
        assert!(matches!(store.get(&key(2)).as_deref(), Some(Value::Int(2))));
    }
}
