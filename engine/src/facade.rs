use std::collections::HashMap;
use std::fmt;

use crate::execution::{CookingContextRange, ExecutionId, ExecutionMode};
use crate::graph::model::subgraph::ExecuteTarget;
use crate::node_registry::ResolvedSchema;
use types::{NodeId, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct ExecuteRequest {
    pub target: ExecuteTarget,
    pub mode: Option<ExecutionMode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionTicket {
    pub execution_id: ExecutionId,
    pub target: ExecuteTarget,
}

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
    fn list_node_defs(&self) -> Vec<&crate::node_manager::NodeDef>;
    fn resolve_node_schema(
        &self,
        type_id: &str,
        current_params: &HashMap<String, Value>,
    ) -> Result<ResolvedSchema, EngineError>;
    fn add_node(&mut self, type_id: &str) -> Result<NodeId, EngineError>;
    fn set_param(&mut self, node_id: NodeId, param: &str, value: Value, preview: bool);
    fn set_cooking_range(&mut self, range: CookingContextRange);
    fn get_execution_outputs(
        &self,
        execution_id: ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError>;
}
