use std::time::{Duration, Instant};

use crate::scheduler::cancel::cancel_token::{CancelToken, CancelledError};
use crate::scheduler::model::RunId;

#[derive(Clone, Debug)]
pub struct RunningTask {
    run_id: RunId,
    cancel_token: CancelToken,
    started_at: Instant,
    timeout: Option<Duration>,
}

impl RunningTask {
    pub fn new(run_id: RunId) -> Self {
        Self {
            run_id,
            cancel_token: CancelToken::new(),
            started_at: Instant::now(),
            timeout: None,
        }
    }

    pub fn with_timeout(run_id: RunId, timeout: Duration) -> Self {
        Self {
            run_id,
            cancel_token: CancelToken::new(),
            started_at: Instant::now(),
            timeout: Some(timeout),
        }
    }

    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    pub fn cancel_token(&self) -> CancelToken {
        self.cancel_token.clone()
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn is_timed_out(&self) -> bool {
        match self.timeout {
            Some(timeout) => self.elapsed() >= timeout,
            None => false,
        }
    }

    pub fn check_cancelled(&self) -> Result<(), CancelledError> {
        self.cancel_token.check()
    }

    pub fn check_timeout(&self) -> Result<(), TaskTimeoutError> {
        if self.is_timed_out() {
            Err(TaskTimeoutError {
                run_id: self.run_id,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskTimeoutError {
    pub run_id: RunId,
}

impl std::fmt::Display for TaskTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "execution timed out for run {:?}", self.run_id)
    }
}

impl std::error::Error for TaskTimeoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_has_no_timeout() {
        let task = RunningTask::new(RunId(1));

        assert_eq!(task.run_id(), RunId(1));
        assert_eq!(task.timeout(), None);
        assert!(!task.is_cancelled());
        assert!(!task.is_timed_out());
    }

    #[test]
    fn cancel_marks_running_task() {
        let task = RunningTask::new(RunId(2));

        task.cancel();

        assert!(task.is_cancelled());
        assert_eq!(task.check_cancelled(), Err(CancelledError));
    }

    #[test]
    fn timeout_is_reported_after_deadline() {
        let task = RunningTask::with_timeout(RunId(3), Duration::from_millis(0));

        assert!(task.is_timed_out());
        assert_eq!(
            task.check_timeout(),
            Err(TaskTimeoutError { run_id: RunId(3) })
        );
    }

    #[test]
    fn cancel_token_is_shared_with_task() {
        let task = RunningTask::new(RunId(4));
        let token = task.cancel_token();

        token.cancel();

        assert!(task.is_cancelled());
    }
}
