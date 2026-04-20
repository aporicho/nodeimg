use std::time::SystemTime;

use crate::execution::ExecutionId;
use crate::execution::{EvaluationFidelity, ExecutionMode, ExecutionTerminalStatus};
use crate::graph::model::subgraph::ExecuteTarget;
use types::NodeId;

pub type PendingExecutionId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct EventRecord<T> {
    pub seq: u64,
    pub at: SystemTime,
    pub payload: T,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    ExecutionStarted {
        execution_id: ExecutionId,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
        from_pending_id: Option<PendingExecutionId>,
    },
    ExecutionQueued {
        pending_id: PendingExecutionId,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
        reason: QueueReason,
    },
    ExecutionReplaced {
        pending_id: PendingExecutionId,
        reason: ReplacementReason,
    },
    NodeStarted {
        execution_id: ExecutionId,
        node_id: NodeId,
    },
    NodeProgressMessage {
        execution_id: ExecutionId,
        node_id: NodeId,
        text: String,
    },
    NodeProgressFraction {
        execution_id: ExecutionId,
        node_id: NodeId,
        current: u32,
        total: u32,
    },
    NodeFinished {
        execution_id: ExecutionId,
        node_id: NodeId,
        status: NodeExecutionStatus,
    },
    NodeFailed {
        execution_id: ExecutionId,
        node_id: NodeId,
        error: String,
    },
    ExecutionFinished {
        execution_id: ExecutionId,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
        status: ExecutionTerminalStatus,
    },
    ExecutionCancelled {
        subject: CancellationSubject,
        reason: CancellationReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueueReason {
    BehindPreview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplacementReason {
    ReplacedByNewerFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationSubject {
    Execution(ExecutionId),
    Pending(PendingExecutionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancellationReason {
    UserRequested,
    ReplacedByPreview,
    ReplacedByNewerPreview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeExecutionStatus {
    Finished,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionStatus {
    Idle,
    Running,
    Cancelling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunningExecution {
    pub execution_id: ExecutionId,
    pub target: ExecuteTarget,
    pub mode: ExecutionMode,
    pub fidelity: EvaluationFidelity,
    pub from_pending_id: Option<PendingExecutionId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PendingExecution {
    pub pending_id: PendingExecutionId,
    pub target: ExecuteTarget,
    pub mode: ExecutionMode,
    pub fidelity: EvaluationFidelity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    pub current: Option<RunningExecution>,
    pub pending_full: Option<PendingExecution>,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Idle,
            current: None,
            pending_full: None,
        }
    }
}
