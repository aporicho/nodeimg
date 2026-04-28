use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub struct CacheLifecycle<K> {
    live: BTreeSet<K>,
    used_this_frame: BTreeSet<K>,
}

impl<K> Default for CacheLifecycle<K>
where
    K: Ord,
{
    fn default() -> Self {
        Self {
            live: BTreeSet::new(),
            used_this_frame: BTreeSet::new(),
        }
    }
}

impl<K> CacheLifecycle<K>
where
    K: Clone + Ord,
{
    pub fn begin_frame(&mut self) {
        self.used_this_frame.clear();
    }

    pub fn mark_used(&mut self, key: K) {
        self.live.insert(key.clone());
        self.used_this_frame.insert(key);
    }

    pub fn end_frame(&mut self) {
        self.live.retain(|key| self.used_this_frame.contains(key));
        self.used_this_frame.clear();
    }

    pub fn contains(&self, key: &K) -> bool {
        self.live.contains(key)
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }
}
