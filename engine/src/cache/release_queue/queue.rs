use std::collections::VecDeque;
use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRelease {
    pub handle_id: String,
    pub retry_count: u32,
}

impl PendingRelease {
    pub fn new(handle_id: impl Into<String>) -> Self {
        Self {
            handle_id: handle_id.into(),
            retry_count: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct ReleaseQueue {
    entries: RwLock<VecDeque<PendingRelease>>,
}

impl ReleaseQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&self, pending: PendingRelease) {
        let mut entries = self
            .entries
            .write()
            .expect("release queue write lock poisoned");
        entries.push_back(pending);
    }

    pub fn dequeue(&self) -> Option<PendingRelease> {
        let mut entries = self
            .entries
            .write()
            .expect("release queue write lock poisoned");
        entries.pop_front()
    }

    pub fn clear(&self) {
        let mut entries = self
            .entries
            .write()
            .expect("release queue write lock poisoned");
        entries.clear();
    }

    pub fn len(&self) -> usize {
        let entries = self
            .entries
            .read()
            .expect("release queue read lock poisoned");
        entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{PendingRelease, ReleaseQueue};

    #[test]
    fn queue_roundtrip() {
        let queue = ReleaseQueue::new();
        queue.enqueue(PendingRelease::new("h1"));

        let pending = queue.dequeue().expect("pending release");
        assert_eq!(pending.handle_id, "h1");
        assert!(queue.is_empty());
    }
}
