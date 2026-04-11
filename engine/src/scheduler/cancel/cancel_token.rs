use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> Result<(), CancelledError> {
        if self.is_cancelled() {
            Err(CancelledError)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CancelledError;

impl std::fmt::Display for CancelledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "execution cancelled")
    }
}

impl std::error::Error for CancelledError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_token_is_not_cancelled() {
        let token = CancelToken::new();

        assert!(!token.is_cancelled());
        assert_eq!(token.check(), Ok(()));
    }

    #[test]
    fn cancel_marks_token_as_cancelled() {
        let token = CancelToken::new();

        token.cancel();

        assert!(token.is_cancelled());
        assert_eq!(token.check(), Err(CancelledError));
    }

    #[test]
    fn clones_share_cancellation_state() {
        let token = CancelToken::new();
        let cloned = token.clone();

        cloned.cancel();

        assert!(token.is_cancelled());
        assert!(cloned.is_cancelled());
    }
}
