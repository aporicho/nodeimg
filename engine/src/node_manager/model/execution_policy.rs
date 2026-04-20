#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CachePolicy {
    Disabled,
    SuccessOnly,
    SuccessAndHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactPolicy {
    None,
    Persist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerPolicy {
    Auto,
    ManualOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryPolicy {
    Never,
    Once,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPolicy {
    pub timeout_ms: Option<u64>,
    pub cache: Option<CachePolicy>,
    pub artifact_policy: Option<ArtifactPolicy>,
    pub trigger_policy: TriggerPolicy,
    pub retry_policy: RetryPolicy,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            cache: None,
            artifact_policy: None,
            trigger_policy: TriggerPolicy::Auto,
            retry_policy: RetryPolicy::Never,
        }
    }
}
