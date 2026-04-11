#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_retries: u32,
}

impl RetryPolicy {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    pub fn should_retry(&self, retry_count: u32) -> bool {
        retry_count < self.max_retries
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_retries: 3 }
    }
}

#[cfg(test)]
mod tests {
    use super::RetryPolicy;

    #[test]
    fn retry_policy_stops_at_limit() {
        let policy = RetryPolicy::new(2);

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(!policy.should_retry(2));
    }
}
