use super::execution_mode::ExecutionMode;
use super::execution_plan::RunId;
use super::runtime_state::RuntimeState;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SchedulerStateSummary {
    pub mode: ExecutionMode,
    pub runtime_state: RuntimeState,
    pub dirty_count: usize,
    pub has_pending_changes: bool,
    pub last_run_id: Option<RunId>,
}
