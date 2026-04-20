use crate::graph::model::state::GraphState;
use crate::graph::Graph;
use std::sync::Arc;
use types::NodeId;
use types::Value;

impl GraphState {
    pub fn commit(&mut self, graph: Graph) {
        self.preview = None;
        self.undo_stack.push(Arc::clone(&self.current));
        if self.undo_stack.len() > self.max_undo {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.current = Arc::new(graph);
        self.graph_version += 1;
        self.recompute_dirty();
    }

    pub fn preview(&mut self, node_id: NodeId, param: &str, value: Value) {
        self.preview = Some(crate::graph::model::state::PreviewOverlay {
            target: crate::graph::model::state::PreviewTarget {
                node_id,
                param: param.to_string(),
            },
            value,
        });
    }

    pub fn discard_preview(&mut self) -> Option<crate::graph::model::state::PreviewTarget> {
        self.preview.take().map(|overlay| overlay.target)
    }

    pub fn mark_saved(&mut self) {
        self.saved_graph = Some(Arc::clone(&self.current));
        self.dirty = false;
    }

    pub fn replace(&mut self, graph: Graph) {
        self.preview = None;
        self.current = Arc::new(graph);
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.saved_graph = None;
        self.graph_version += 1;
        self.dirty = true;
    }

    pub(crate) fn recompute_dirty(&mut self) {
        self.dirty = match &self.saved_graph {
            None => self.graph_version != 0,
            Some(saved) => !Arc::ptr_eq(saved, &self.current),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::model::state::GraphState;
    use types::NodeId;
    use types::Value;

    #[test]
    fn mark_saved_clears_dirty() {
        let mut state = GraphState::new(10);
        let (g, _) = state.current().add_node("a", Default::default());
        state.commit(g);
        assert!(state.is_dirty());
        state.mark_saved();
        assert!(!state.is_dirty());
    }

    #[test]
    fn preview_does_not_change_current() {
        let mut state = GraphState::new(10);
        state.preview(NodeId(0), "x", Value::Float(1.0));
        assert_eq!(state.current().nodes.len(), 0);
        assert!(state.has_preview());
        assert_eq!(state.preview_target().unwrap().node_id, NodeId(0));
        assert!(matches!(state.preview_value(), Some(Value::Float(v)) if *v == 1.0));
    }
}
