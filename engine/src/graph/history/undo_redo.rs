use crate::graph::model::state::GraphState;
use std::sync::Arc;

impl GraphState {
    pub fn undo(&mut self) -> bool {
        if self.undo_stack.is_empty() {
            return false;
        }

        self.preview = None;
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(Arc::clone(&self.current));
            self.current = prev;
            self.graph_version += 1;
            self.recompute_dirty();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.redo_stack.is_empty() {
            return false;
        }

        self.preview = None;
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(Arc::clone(&self.current));
            self.current = next;
            self.graph_version += 1;
            self.recompute_dirty();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::model::state::GraphState;
    use types::NodeId;
    use types::Value;

    #[test]
    fn undo_empty_keeps_preview() {
        let mut state = GraphState::new(10);
        state.preview(NodeId(0), "x", Value::Float(1.0));
        assert!(!state.undo());
        assert!(state.has_preview());
    }

    #[test]
    fn undo_back_to_saved_clears_dirty() {
        let mut state = GraphState::new(10);
        state.mark_saved();
        let (g, _) = state.current().add_node("a", Default::default());
        state.commit(g);
        assert!(state.is_dirty());
        assert!(state.undo());
        assert!(!state.is_dirty());
    }
}
