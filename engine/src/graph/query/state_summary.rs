use crate::graph::model::state::{GraphState, GraphStateSummary};

pub fn state_summary(state: &GraphState) -> GraphStateSummary {
    state.summary()
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::NodeId;
    use types::Value;

    #[test]
    fn summary_reflects_state_flags() {
        let mut state = GraphState::new(10);
        let initial = state_summary(&state);
        assert_eq!(initial.graph_version, 0);
        assert!(!initial.dirty);
        assert!(!initial.can_undo);
        assert!(!initial.can_redo);
        assert!(!initial.has_preview);

        let (g, _) = state.current().add_node("a", Default::default());
        state.commit(g);
        let after_commit = state_summary(&state);
        assert_eq!(after_commit.graph_version, 1);
        assert!(after_commit.dirty);
        assert!(after_commit.can_undo);
        assert!(!after_commit.can_redo);
        assert!(!after_commit.has_preview);

        state.preview(NodeId(1), "x", Value::Float(1.0));
        let after_preview = state_summary(&state);
        assert!(after_preview.has_preview);
        assert_eq!(after_preview.graph_version, 1);
    }
}
