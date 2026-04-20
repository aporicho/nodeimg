use super::{PendingRelease, RetryPolicy};

pub fn schedule_retry(pending: &mut PendingRelease, policy: RetryPolicy) -> bool {
    if !policy.should_retry(pending.retry_count) {
        return false;
    }

    pending.retry_count += 1;
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseAttempt {
    Released,
    Requeued,
    Dropped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseReport {
    pub handle_id: String,
    pub retry_count: u32,
    pub attempt: ReleaseAttempt,
}

pub fn process_pending_release<F>(
    mut pending: PendingRelease,
    policy: RetryPolicy,
    release_fn: F,
) -> (ReleaseReport, Option<PendingRelease>)
where
    F: FnOnce(&str) -> bool,
{
    if release_fn(&pending.handle_id) {
        return (
            ReleaseReport {
                handle_id: pending.handle_id,
                retry_count: pending.retry_count,
                attempt: ReleaseAttempt::Released,
            },
            None,
        );
    }

    if schedule_retry(&mut pending, policy) {
        return (
            ReleaseReport {
                handle_id: pending.handle_id.clone(),
                retry_count: pending.retry_count,
                attempt: ReleaseAttempt::Requeued,
            },
            Some(pending),
        );
    }

    (
        ReleaseReport {
            handle_id: pending.handle_id,
            retry_count: pending.retry_count,
            attempt: ReleaseAttempt::Dropped,
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::{process_pending_release, schedule_retry, ReleaseAttempt};
    use crate::cache::release_queue::{PendingRelease, RetryPolicy};

    #[test]
    fn schedule_retry_increments_retry_count() {
        let mut pending = PendingRelease::new("h1");
        let scheduled = schedule_retry(&mut pending, RetryPolicy::new(2));

        assert!(scheduled);
        assert_eq!(pending.retry_count, 1);
    }

    #[test]
    fn successful_release_is_not_requeued() {
        let pending = PendingRelease::new("h1");
        let (attempt, next) = process_pending_release(pending, RetryPolicy::new(2), |_| true);

        assert_eq!(attempt.attempt, ReleaseAttempt::Released);
        assert!(next.is_none());
    }

    #[test]
    fn failed_release_is_requeued_when_policy_allows() {
        let pending = PendingRelease::new("h1");
        let (attempt, next) = process_pending_release(pending, RetryPolicy::new(2), |_| false);

        assert_eq!(attempt.attempt, ReleaseAttempt::Requeued);
        assert_eq!(next.expect("requeued").retry_count, 1);
    }

    #[test]
    fn failed_release_is_dropped_after_retry_limit() {
        let mut pending = PendingRelease::new("h1");
        pending.retry_count = 2;

        let (attempt, next) = process_pending_release(pending, RetryPolicy::new(2), |_| false);

        assert_eq!(attempt.attempt, ReleaseAttempt::Dropped);
        assert!(next.is_none());
    }
}
