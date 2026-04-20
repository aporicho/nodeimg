use std::collections::VecDeque;

use crate::cache::model::CacheKey;

#[derive(Debug, Default)]
pub struct ResultLru {
    order: VecDeque<CacheKey>,
}

impl ResultLru {
    pub fn touch(&mut self, key: &CacheKey) {
        self.remove(key);
        self.order.push_back(key.clone());
    }

    pub fn pop_lru(&mut self) -> Option<CacheKey> {
        self.order.pop_front()
    }

    pub fn remove(&mut self, key: &CacheKey) {
        if let Some(index) = self.order.iter().position(|existing| existing == key) {
            self.order.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.order.clear();
    }
}
