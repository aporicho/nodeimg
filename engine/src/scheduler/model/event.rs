use super::error::SchedulerError;
use super::execution_plan::RunId;
use types::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionEvent {
    Started {
        run_id: RunId,
        total_nodes: usize,
    },
    NodeStarted {
        run_id: RunId,
        node_id: NodeId,
    },
    NodeFinished {
        run_id: RunId,
        node_id: NodeId,
    },
    NodeFailed {
        run_id: RunId,
        node_id: NodeId,
        error: String,
    },
    Progress {
        run_id: RunId,
        completed: usize,
        total: usize,
    },
    Finished {
        run_id: RunId,
    },
    Cancelled {
        run_id: RunId,
    },
    Failed {
        run_id: RunId,
        error: SchedulerError,
    },
}
