use super::{PendingRelease, RetryPolicy};

pub fn schedule_retry(pending: &mut PendingRelease, policy: RetryPolicy) -> bool {
    if !policy.should_retry(pending.retry_count) {
        return false;
    }

    pending.retry_count += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::schedule_retry;
    use crate::cache::release_queue::{PendingRelease, RetryPolicy};

    #[test]
    fn schedule_retry_increments_retry_count() {
        let mut pending = PendingRelease::new("h1");
        let scheduled = schedule_retry(&mut pending, RetryPolicy::new(2));

        assert!(scheduled);
        assert_eq!(pending.retry_count, 1);
    }
}
