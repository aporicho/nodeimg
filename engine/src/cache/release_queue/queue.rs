use std::collections::VecDeque;
use std::sync::RwLock;

use super::{process_pending_release, ReleaseReport, RetryPolicy};

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

    pub fn process_next<F>(&self, policy: RetryPolicy, release_fn: F) -> Option<ReleaseReport>
    where
        F: FnOnce(&str) -> bool,
    {
        let pending = self.dequeue()?;
        let (report, next) = process_pending_release(pending, policy, release_fn);
        if let Some(pending) = next {
            self.enqueue(pending);
        }
        Some(report)
    }
}

#[cfg(test)]
mod tests {
    use super::{PendingRelease, ReleaseQueue};
    use crate::cache::release_queue::{ReleaseAttempt, RetryPolicy};

    #[test]
    fn queue_roundtrip() {
        let queue = ReleaseQueue::new();
        queue.enqueue(PendingRelease::new("h1"));

        let pending = queue.dequeue().expect("pending release");
        assert_eq!(pending.handle_id, "h1");
        assert!(queue.is_empty());
    }

    #[test]
    fn process_next_requeues_on_failure() {
        let queue = ReleaseQueue::new();
        queue.enqueue(PendingRelease::new("h1"));

        let attempt = queue
            .process_next(RetryPolicy::new(2), |_| false)
            .expect("attempt result");

        assert_eq!(attempt.attempt, ReleaseAttempt::Requeued);
        assert_eq!(queue.len(), 1);
    }
}
