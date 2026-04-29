use super::{LayoutCacheKey, LayoutOutput};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutCache {
    entries: HashMap<LayoutCacheKey, LayoutOutput>,
}

impl LayoutCache {
    pub fn get(&self, key: &LayoutCacheKey) -> Option<LayoutOutput> {
        self.entries.get(key).copied()
    }

    pub fn set(&mut self, key: LayoutCacheKey, output: LayoutOutput) {
        self.entries.insert(key, output);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn remove(&mut self, key: &LayoutCacheKey) {
        self.entries.remove(key);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
