use crate::graph::Graph;
use crate::scheduler::model::{DirtyReason, DirtyState};

use super::dirty_propagation::rebuild_all_dirty;

pub fn rebuild_dirty_state(graph: &Graph, reason: DirtyReason) -> DirtyState {
    let mut dirty_state = DirtyState::new();
    rebuild_dirty_state_in_place(graph, &mut dirty_state, reason);
    dirty_state
}

pub fn rebuild_dirty_state_in_place(
    graph: &Graph,
    dirty_state: &mut DirtyState,
    reason: DirtyReason,
) {
    rebuild_all_dirty(graph, dirty_state, reason);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::model::DirtyReason;

    #[test]
    fn rebuild_from_empty_graph_returns_empty_dirty_state() {
        let graph = Graph::new();

        let dirty_state = rebuild_dirty_state(&graph, DirtyReason::StructureChanged);

        assert!(dirty_state.is_empty());
    }

    #[test]
    fn rebuild_marks_all_nodes_with_same_reason() {
        let graph = Graph::new();
        let (graph, a) = graph.add_node("a", Default::default());
        let (graph, b) = graph.add_node("b", Default::default());

        let dirty_state = rebuild_dirty_state(&graph, DirtyReason::StructureChanged);

        assert_eq!(dirty_state.len(), 2);
        assert_eq!(
            dirty_state.dirty_reasons.get(&a),
            Some(&DirtyReason::StructureChanged)
        );
        assert_eq!(
            dirty_state.dirty_reasons.get(&b),
            Some(&DirtyReason::StructureChanged)
        );
    }

    #[test]
    fn rebuild_in_place_replaces_previous_dirty_state() {
        let graph = Graph::new();
        let (graph, a) = graph.add_node("a", Default::default());
        let (graph, b) = graph.add_node("b", Default::default());

        let mut dirty_state = DirtyState::new();
        dirty_state.mark(a, DirtyReason::Forced);

        rebuild_dirty_state_in_place(&graph, &mut dirty_state, DirtyReason::StructureChanged);

        assert_eq!(dirty_state.len(), 2);
        assert_eq!(
            dirty_state.dirty_reasons.get(&a),
            Some(&DirtyReason::StructureChanged)
        );
        assert_eq!(
            dirty_state.dirty_reasons.get(&b),
            Some(&DirtyReason::StructureChanged)
        );
    }
}
