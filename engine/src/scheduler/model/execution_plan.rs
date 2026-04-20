use std::sync::Arc;

use types::NodeId;

use crate::cache::model::{exec_signature::ExecSignature, generation_id::GenerationId};
use crate::graph::model::graph::Graph;
use crate::node_manager::ExecutorType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlannedNode {
    pub node_id: NodeId,
    pub executor_type: ExecutorType,
    pub exec_signature: ExecSignature,
}

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub run_id: RunId,
    pub graph_snapshot: Arc<Graph>,
    pub generation: GenerationId,
    pub layers: Vec<Vec<PlannedNode>>,
}
