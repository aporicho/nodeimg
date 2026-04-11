use super::execution_plan::RunId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExecutionProgress {
    pub completed: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalStatus {
    Finished,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeState {
    Idle,
    Planning {
        run_id: RunId,
    },
    Running {
        run_id: RunId,
        progress: ExecutionProgress,
    },
    LastCompleted {
        run_id: RunId,
        status: FinalStatus,
    },
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self::Idle
    }
}
