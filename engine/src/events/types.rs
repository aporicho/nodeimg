use std::time::SystemTime;

use crate::execution::{EvaluationFidelity, ExecutionMode, ExecutionTerminalStatus};
use crate::graph::model::subgraph::ExecuteTarget;
use types::NodeId;

#[derive(Clone, Debug, PartialEq)]
pub struct EventRecord<T> {
    pub seq: u64,
    pub at: SystemTime,
    pub payload: T,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    ExecutionStarted {
        execution_id: u64,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
    },
    NodeStarted {
        execution_id: u64,
        node_id: NodeId,
    },
    NodeProgressMessage {
        execution_id: u64,
        node_id: NodeId,
        text: String,
    },
    NodeProgressFraction {
        execution_id: u64,
        node_id: NodeId,
        current: u32,
        total: u32,
    },
    NodeFinished {
        execution_id: u64,
        node_id: NodeId,
        status: NodeExecutionStatus,
    },
    NodeFailed {
        execution_id: u64,
        node_id: NodeId,
        error: String,
    },
    ExecutionFinished {
        execution_id: u64,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
        status: ExecutionTerminalStatus,
    },
    ExecutionCancelled {
        execution_id: u64,
        reason: String,
    },
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
    pub execution_id: u64,
    pub target: ExecuteTarget,
    pub mode: ExecutionMode,
    pub fidelity: EvaluationFidelity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionState {
    pub status: ExecutionStatus,
    pub current: Option<RunningExecution>,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Idle,
            current: None,
        }
    }
}
