use crate::graph::model::events::{GraphChange, GraphChangedEvent};
use crate::graph::query::downstream::downstream;
use crate::graph::Graph;
use crate::scheduler::model::{DirtyReason, DirtyState};
use types::NodeId;

pub fn apply_graph_event(graph: &Graph, dirty_state: &mut DirtyState, event: &GraphChangedEvent) {
    for change in &event.changes {
        apply_graph_change(graph, dirty_state, change);
    }
}

pub fn apply_graph_change(graph: &Graph, dirty_state: &mut DirtyState, change: &GraphChange) {
    match change {
        GraphChange::NodeAdded { node_id } => {
            mark_node_and_downstream(graph, dirty_state, *node_id, DirtyReason::StructureChanged);
        }
        GraphChange::NodeRemoved { .. } => {
            // 删除节点后，该节点已不在当前图中。首版只依赖级联连接删除事件去标脏下游。
        }
        GraphChange::ConnectionAdded { to, .. } | GraphChange::ConnectionRemoved { to, .. } => {
            mark_node_and_downstream(graph, dirty_state, to.node, DirtyReason::StructureChanged);
        }
        GraphChange::ParamCommitted { node_id, .. } => {
            mark_node_and_downstream(graph, dirty_state, *node_id, DirtyReason::ParamChanged);
        }
        GraphChange::Replaced | GraphChange::Undone | GraphChange::Redone => {
            rebuild_all_dirty(graph, dirty_state, DirtyReason::StructureChanged);
        }
    }
}

fn mark_node_and_downstream(
    graph: &Graph,
    dirty_state: &mut DirtyState,
    node_id: NodeId,
    reason: DirtyReason,
) {
    dirty_state.mark(node_id, reason);
    for downstream_node in downstream(graph, node_id) {
        let downstream_reason = match reason {
            DirtyReason::ParamChanged => DirtyReason::UpstreamChanged,
            DirtyReason::ArtifactSelectionChanged => DirtyReason::UpstreamChanged,
            DirtyReason::Forced => DirtyReason::Forced,
            DirtyReason::StructureChanged | DirtyReason::UpstreamChanged => {
                DirtyReason::UpstreamChanged
            }
        };
        dirty_state.mark(downstream_node, downstream_reason);
    }
}

pub fn rebuild_all_dirty(graph: &Graph, dirty_state: &mut DirtyState, reason: DirtyReason) {
    dirty_state.clear();
    for node_id in graph.nodes.keys().copied() {
        dirty_state.mark(node_id, reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::model::events::{GraphChange, GraphChangedEvent};
    use crate::graph::PinRef;
    use crate::scheduler::model::DirtyReason;
    use std::collections::HashSet;

    fn sample_graph() -> (Graph, NodeId, NodeId, NodeId) {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let g = g.connect(crate::graph::Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        let g = g.connect(crate::graph::Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });
        (g, a, b, c)
    }

    #[test]
    fn param_change_marks_self_and_downstream() {
        let (graph, a, b, c) = sample_graph();
        let event = GraphChangedEvent {
            graph_version: 1,
            dirty: true,
            changes: vec![GraphChange::ParamCommitted {
                node_id: a,
                param: "strength".into(),
            }],
        };
        let mut dirty = DirtyState::new();

        apply_graph_event(&graph, &mut dirty, &event);

        let expected: HashSet<NodeId> = [a, b, c].into_iter().collect();
        assert_eq!(dirty.dirty_nodes, expected);
        assert_eq!(
            dirty.dirty_reasons.get(&a),
            Some(&DirtyReason::ParamChanged)
        );
        assert_eq!(
            dirty.dirty_reasons.get(&b),
            Some(&DirtyReason::UpstreamChanged)
        );
        assert_eq!(
            dirty.dirty_reasons.get(&c),
            Some(&DirtyReason::UpstreamChanged)
        );
    }

    #[test]
    fn connection_change_marks_target_and_downstream() {
        let (graph, _a, b, c) = sample_graph();
        let event = GraphChangedEvent {
            graph_version: 2,
            dirty: true,
            changes: vec![GraphChange::ConnectionAdded {
                from: PinRef {
                    node: b,
                    interface: "out".into(),
                },
                to: PinRef {
                    node: c,
                    interface: "mask".into(),
                },
            }],
        };
        let mut dirty = DirtyState::new();

        apply_graph_event(&graph, &mut dirty, &event);

        let expected: HashSet<NodeId> = [c].into_iter().collect();
        assert_eq!(dirty.dirty_nodes, expected);
        assert_eq!(
            dirty.dirty_reasons.get(&c),
            Some(&DirtyReason::StructureChanged)
        );
    }

    #[test]
    fn replaced_rebuilds_all_dirty() {
        let (graph, a, b, c) = sample_graph();
        let event = GraphChangedEvent {
            graph_version: 3,
            dirty: true,
            changes: vec![GraphChange::Replaced],
        };
        let mut dirty = DirtyState::new();
        dirty.mark(a, DirtyReason::Forced);

        apply_graph_event(&graph, &mut dirty, &event);

        let expected: HashSet<NodeId> = [a, b, c].into_iter().collect();
        assert_eq!(dirty.dirty_nodes, expected);
        assert_eq!(
            dirty.dirty_reasons.get(&a),
            Some(&DirtyReason::StructureChanged)
        );
        assert_eq!(
            dirty.dirty_reasons.get(&b),
            Some(&DirtyReason::StructureChanged)
        );
        assert_eq!(
            dirty.dirty_reasons.get(&c),
            Some(&DirtyReason::StructureChanged)
        );
    }
}
