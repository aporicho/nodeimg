use crate::graph::validate::ConnectionError;
use crate::graph::Graph;
use std::collections::HashMap;
use types::{NodeId, Value};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BatchNodeRef {
    Existing(NodeId),
    Created(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchPinRef {
    pub node: BatchNodeRef,
    pub interface: String,
}

#[derive(Clone, Debug)]
pub enum GraphEdit {
    AddNode {
        client_key: String,
        type_id: String,
        param_overrides: HashMap<String, Value>,
    },
    RemoveNode {
        node_id: NodeId,
    },
    Connect {
        from: BatchPinRef,
        to: BatchPinRef,
    },
    Disconnect {
        from: BatchPinRef,
        to: BatchPinRef,
    },
    SetParam {
        node: BatchNodeRef,
        param: String,
        value: Value,
        preview: bool,
    },
}

#[derive(Clone, Debug, Default)]
pub struct EditBatchRequest {
    pub edits: Vec<GraphEdit>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditBatchResult {
    pub created_nodes: HashMap<String, NodeId>,
    pub graph_version: u64,
}

#[derive(Debug)]
pub enum ApplyBatchError {
    UnknownNodeType { type_id: String },
    NodeNotFound { node_id: NodeId },
    UnknownBatchNodeKey { key: String },
    ConnectionError(ConnectionError),
    PreviewNotAllowed,
}

impl std::fmt::Display for ApplyBatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownNodeType { type_id } => write!(f, "Unknown node type: {}", type_id),
            Self::NodeNotFound { node_id } => write!(f, "Node {:?} not found", node_id),
            Self::UnknownBatchNodeKey { key } => write!(f, "Unknown batch node key: {}", key),
            Self::ConnectionError(error) => write!(f, "{}", error),
            Self::PreviewNotAllowed => write!(f, "preview edits are not allowed in apply_batch"),
        }
    }
}

impl std::error::Error for ApplyBatchError {}

#[derive(Clone, Debug)]
pub struct BatchWorkResult {
    pub graph: Graph,
    pub changes: Vec<crate::graph::model::events::GraphChange>,
    pub created_nodes: HashMap<String, NodeId>,
}
