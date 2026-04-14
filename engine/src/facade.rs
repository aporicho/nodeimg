use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::events::{EngineEvent, EventSubscription, ExecutionState};
use crate::execution::{CookingContextRange, ExecutionId, ExecutionMode};
use crate::graph::model::batch::{EditBatchRequest, EditBatchResult};
use crate::graph::model::subgraph::ExecuteTarget;
use crate::graph::{Connection, Graph, PinRef};
use crate::node_registry::ResolvedSchema;
use types::{NodeId, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionRequest {
    pub target: ExecuteTarget,
    pub mode: Option<ExecutionMode>,
}

pub type ExecuteRequest = ExecutionRequest;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionTicket {
    pub execution_id: ExecutionId,
    pub target: ExecuteTarget,
}

pub type EngineSubscription = EventSubscription<EngineEvent>;

#[derive(Debug)]
pub enum EngineError {
    Graph {
        message: String,
    },
    Schema {
        message: String,
    },
    Execution {
        message: String,
    },
    CapabilityUnavailable {
        cap_id: String,
    },
    ResultNotFound {
        execution_id: ExecutionId,
        node_id: NodeId,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Graph { message }
            | EngineError::Schema { message }
            | EngineError::Execution { message } => write!(f, "{message}"),
            EngineError::CapabilityUnavailable { cap_id } => {
                write!(f, "no executor registered for capability '{cap_id}'")
            }
            EngineError::ResultNotFound {
                execution_id,
                node_id,
            } => write!(
                f,
                "no outputs recorded for execution {} node {:?}",
                execution_id, node_id
            ),
        }
    }
}

impl std::error::Error for EngineError {}

pub trait EngineFacade {
    fn query_graph_snapshot(&self) -> Arc<Graph>;
    fn list_node_defs(&self) -> Vec<&crate::node_manager::NodeDef>;
    fn resolve_node_schema(
        &self,
        type_id: &str,
        current_params: &HashMap<String, Value>,
    ) -> Result<ResolvedSchema, EngineError>;
    fn add_node(&mut self, type_id: &str) -> Result<NodeId, EngineError>;
    fn remove_node(&mut self, node_id: NodeId) -> Result<(), EngineError>;
    fn connect(&mut self, connection: Connection) -> Result<(), EngineError>;
    fn disconnect(&mut self, from: PinRef, to: PinRef) -> Result<(), EngineError>;
    fn set_param(&mut self, node_id: NodeId, param: &str, value: Value, preview: bool);
    fn apply_batch(&mut self, request: EditBatchRequest) -> Result<EditBatchResult, EngineError>;
    fn undo(&mut self) -> bool;
    fn redo(&mut self) -> bool;
    fn replace_graph(&mut self, graph: Graph) -> Result<(), EngineError>;
    fn set_cooking_range(&mut self, range: CookingContextRange);
    fn query_state(&self) -> ExecutionState;
    fn subscribe_engine_events(&self) -> EngineSubscription;
    fn query_execution_outputs(
        &self,
        execution_id: ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError>;
}
