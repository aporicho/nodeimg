use std::collections::VecDeque;

use crate::cache::model::TextureKey;

#[derive(Debug, Default)]
pub struct TextureLru {
    order: VecDeque<TextureKey>,
}

impl TextureLru {
    pub fn touch(&mut self, key: &TextureKey) {
        self.remove(key);
        self.order.push_back(key.clone());
    }

    pub fn pop_lru(&mut self) -> Option<TextureKey> {
        self.order.pop_front()
    }

    pub fn remove(&mut self, key: &TextureKey) {
        if let Some(index) = self.order.iter().position(|existing| existing == key) {
            self.order.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.order.clear();
    }
}
