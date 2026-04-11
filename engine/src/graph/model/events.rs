use std::time::SystemTime;

use crate::graph::PinRef;
use types::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphChange {
    NodeAdded { node_id: NodeId },
    NodeRemoved { node_id: NodeId },
    ConnectionAdded { from: PinRef, to: PinRef },
    ConnectionRemoved { from: PinRef, to: PinRef },
    ParamCommitted { node_id: NodeId, param: String },
    Replaced,
    Undone,
    Redone,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphChangedEvent {
    pub graph_version: u64,
    pub dirty: bool,
    pub changes: Vec<GraphChange>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewChangedEvent {
    pub node_id: NodeId,
    pub param: String,
    pub has_preview: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GraphEventKind {
    GraphChanged(GraphChangedEvent),
    PreviewChanged(PreviewChangedEvent),
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEvent {
    pub at: SystemTime,
    pub kind: GraphEventKind,
}

impl GraphEvent {
    pub fn new(kind: GraphEventKind) -> Self {
        Self {
            at: SystemTime::now(),
            kind,
        }
    }
}
