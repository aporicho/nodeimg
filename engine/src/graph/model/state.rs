use super::graph::Graph;
use std::sync::Arc;
use types::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphStateSummary {
    pub graph_version: u64,
    pub dirty: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub has_preview: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewTarget {
    pub node_id: types::NodeId,
    pub param: String,
}

#[derive(Clone, Debug)]
pub struct PreviewOverlay {
    pub target: PreviewTarget,
    pub value: Value,
}

pub struct GraphState {
    pub(crate) current: Arc<Graph>,
    pub(crate) preview: Option<PreviewOverlay>,
    pub(crate) undo_stack: Vec<Arc<Graph>>,
    pub(crate) redo_stack: Vec<Arc<Graph>>,
    pub(crate) max_undo: usize,
    pub(crate) graph_version: u64,
    pub(crate) dirty: bool,
    pub(crate) saved_graph: Option<Arc<Graph>>,
}

impl GraphState {
    pub fn new(max_undo: usize) -> Self {
        Self {
            current: Arc::new(Graph::new()),
            preview: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo,
            graph_version: 0,
            dirty: false,
            saved_graph: None,
        }
    }

    pub fn current(&self) -> &Graph {
        &self.current
    }

    pub fn snapshot(&self) -> Arc<Graph> {
        Arc::clone(&self.current)
    }

    pub fn preview_target(&self) -> Option<&PreviewTarget> {
        self.preview.as_ref().map(|overlay| &overlay.target)
    }

    pub fn preview_value(&self) -> Option<&Value> {
        self.preview.as_ref().map(|overlay| &overlay.value)
    }

    pub fn graph_version(&self) -> u64 {
        self.graph_version
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn has_preview(&self) -> bool {
        self.preview.is_some()
    }

    pub fn summary(&self) -> GraphStateSummary {
        GraphStateSummary {
            graph_version: self.graph_version,
            dirty: self.dirty,
            can_undo: self.can_undo(),
            can_redo: self.can_redo(),
            has_preview: self.has_preview(),
        }
    }
}
