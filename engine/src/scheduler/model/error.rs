use super::execution_plan::RunId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    RunAlreadyInProgress,
    RunNotFound { run_id: RunId },
    GraphSnapshotUnavailable,
    PlannerFailed { message: String },
    RuntimeFailed { message: String },
    CancelFailed { run_id: RunId },
}
