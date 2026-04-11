use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::cache::model::generation_id::GenerationId;
use crate::graph::query::topo_sort::{topo_sort, CycleError};
use crate::graph::query::upstream::upstream;
use crate::graph::Graph;
use crate::node_manager::NodeManager;
use crate::scheduler::model::{
    DirtyState, ExecuteTarget, ExecutionMode, ExecutionPlan, ExecutionRequest, ExecutorType,
    PlannedNode, RunId,
};
use crate::scheduler::planner::signature_compose::compose_exec_signature;
use crate::scheduler::runtime::input_resolver::resolve_node_inputs;
use types::NodeId;

pub fn build_plan(
    graph: Arc<Graph>,
    node_manager: &NodeManager,
    dirty_state: &DirtyState,
    request: ExecutionRequest,
    mode: ExecutionMode,
    run_id: RunId,
    generation: GenerationId,
) -> Result<Option<ExecutionPlan>, CycleError>
{
    let selected_nodes = select_nodes(&graph, dirty_state, request)?;
    if selected_nodes.is_empty() {
        return Ok(None);
    }

    let filtered_nodes = filter_by_mode(node_manager, selected_nodes, mode);
    if filtered_nodes.is_empty() {
        return Ok(None);
    }

    let layers = build_layers(&graph, node_manager, &filtered_nodes);
    if layers.is_empty() {
        return Ok(None);
    }

    Ok(Some(ExecutionPlan {
        run_id,
        graph_snapshot: graph,
        generation,
        layers,
    }))
}

fn select_nodes(
    graph: &Graph,
    dirty_state: &DirtyState,
    request: ExecutionRequest,
) -> Result<HashSet<NodeId>, CycleError> {
    let dirty_nodes = &dirty_state.dirty_nodes;

    let selected = match request.target {
        ExecuteTarget::Graph => dirty_nodes.clone(),
        ExecuteTarget::Node(target) | ExecuteTarget::Force(target) => {
            let mut reachable = upstream(graph, target);
            reachable.insert(target);

            let topo = topo_sort(graph, target)?;
            topo.into_iter()
                .filter(|node_id| dirty_nodes.contains(node_id) && reachable.contains(node_id))
                .collect()
        }
    };

    Ok(selected)
}

fn filter_by_mode(
    node_manager: &NodeManager,
    nodes: HashSet<NodeId>,
    mode: ExecutionMode,
) -> HashSet<NodeId> {
    match mode {
        ExecutionMode::Manual => nodes,
        ExecutionMode::Auto => nodes
            .into_iter()
            .filter(|node_id| {
                node_manager
                    .get_node_def_for_node_id(*node_id)
                    .is_some_and(|def| matches!(def.executor_type, ExecutorType::Image))
            })
            .collect(),
    }
}

fn build_layers(
    graph: &Graph,
    node_manager: &NodeManager,
    selected_nodes: &HashSet<NodeId>,
) -> Vec<Vec<PlannedNode>> {
    let mut in_degree: HashMap<NodeId, usize> = selected_nodes.iter().map(|&n| (n, 0)).collect();
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for conn in &graph.connections {
        let from = conn.from.node;
        let to = conn.to.node;
        if selected_nodes.contains(&from) && selected_nodes.contains(&to) {
            adjacency.entry(from).or_default().push(to);
            *in_degree.entry(to).or_default() += 1;
        }
    }

    let mut frontier: Vec<NodeId> = in_degree
        .iter()
        .filter_map(|(&node_id, &deg)| (deg == 0).then_some(node_id))
        .collect();
    frontier.sort_by_key(|id| id.0);

    let mut layers = Vec::new();
    let mut accumulated_results = crate::scheduler::runtime::input_resolver::ExecutionResults::new();

    while !frontier.is_empty() {
        let current_layer = frontier;
        let mut planned_layer = Vec::with_capacity(current_layer.len());
        let mut next_frontier = Vec::new();

        for node_id in current_layer {
            let node = match graph.nodes.get(&node_id) {
                Some(node) => node,
                None => continue,
            };
            let node_def = match node_manager.get_node_def(&node.type_id) {
                Some(def) => def,
                None => continue,
            };
            let inputs = match resolve_node_inputs(graph, node_manager, &accumulated_results, node_id) {
                Ok(inputs) => inputs,
                Err(_) => HashMap::new(),
            };
            let exec_signature = compose_exec_signature(node_def, node, &inputs);

            planned_layer.push(PlannedNode {
                node_id,
                executor_type: node_def.executor_type,
                exec_signature,
            });
            accumulated_results.entry(node_id).or_default();

            if let Some(children) = adjacency.get(&node_id) {
                for &child in children {
                    if let Some(deg) = in_degree.get_mut(&child) {
                        *deg -= 1;
                        if *deg == 0 {
                            next_frontier.push(child);
                        }
                    }
                }
            }
        }

        planned_layer.sort_by_key(|node| node.node_id.0);
        next_frontier.sort_by_key(|id| id.0);
        layers.push(planned_layer);
        frontier = next_frontier;
    }

    layers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::{ExecutorType, NodeDef, NodeManager, ParamDef, ParamExpose, PinDef};
    use crate::scheduler::model::{DirtyReason, DirtyState};
    use crate::{graph::Connection, graph::PinRef};
    use types::{DataType, Value};

    fn sample_graph() -> (Arc<Graph>, NodeManager, NodeId, NodeId, NodeId) {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let g = g.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        let g = g.connect(Connection {
            from: PinRef {
                node: b,
                interface: "out".into(),
            },
            to: PinRef {
                node: c,
                interface: "in".into(),
            },
        });
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("a"));
        node_manager.register(make_test_def("b"));
        node_manager.register(make_test_def("c"));
        (Arc::new(g), node_manager, a, b, c)
    }

    fn make_test_def(type_id: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            name: type_id.into(),
            category: "test".into(),
            executor_type: ExecutorType::Image,
            inputs: vec![PinDef {
                name: "in".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataType::float(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "amount".into(),
                data_type: DataType::float(),
                constraint: None,
                default_value: Value::Float(1.0),
                expose: vec![ParamExpose::Control],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    #[test]
    fn graph_target_keeps_only_dirty_nodes() {
        let (graph, node_manager, a, _b, c) = sample_graph();
        let mut dirty = DirtyState::new();
        dirty.mark(a, DirtyReason::ParamChanged);
        dirty.mark(c, DirtyReason::UpstreamChanged);

        let plan = build_plan(
            graph,
            &node_manager,
            &dirty,
            ExecutionRequest::new(ExecuteTarget::Graph),
            ExecutionMode::Manual,
            RunId(1),
            GenerationId(0),
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.layers.len(), 1);
        assert_eq!(plan.layers[0].len(), 2);
        assert_eq!(plan.layers[0][0].node_id, a);
        assert_eq!(plan.layers[0][1].node_id, c);
    }

    #[test]
    fn node_target_intersects_dirty_with_upstream_subgraph() {
        let (graph, node_manager, a, b, c) = sample_graph();
        let mut dirty = DirtyState::new();
        dirty.mark(a, DirtyReason::ParamChanged);
        dirty.mark(b, DirtyReason::UpstreamChanged);

        let plan = build_plan(
            graph,
            &node_manager,
            &dirty,
            ExecutionRequest::new(ExecuteTarget::Node(c)),
            ExecutionMode::Manual,
            RunId(2),
            GenerationId(0),
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.layers.len(), 2);
        assert_eq!(plan.layers[0][0].node_id, a);
        assert_eq!(plan.layers[1][0].node_id, b);
    }

    #[test]
    fn auto_mode_filters_non_image_nodes() {
        let (graph, mut node_manager, a, b, _c) = sample_graph();
        let mut dirty = DirtyState::new();
        dirty.mark(a, DirtyReason::ParamChanged);
        dirty.mark(b, DirtyReason::UpstreamChanged);
        node_manager.register(NodeDef {
            type_id: "b".into(),
            name: "b".into(),
            category: "test".into(),
            executor_type: ExecutorType::Ai,
            inputs: vec![],
            outputs: vec![],
            params: vec![],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        });

        let plan = build_plan(
            graph,
            &node_manager,
            &dirty,
            ExecutionRequest::new(ExecuteTarget::Graph),
            ExecutionMode::Auto,
            RunId(3),
            GenerationId(0),
        )
        .unwrap()
        .unwrap();

        assert_eq!(plan.layers.len(), 1);
        assert_eq!(plan.layers[0][0].node_id, a);
    }

    #[test]
    fn returns_none_when_no_nodes_survive_selection() {
        let (graph, node_manager, _a, _b, c) = sample_graph();
        let dirty = DirtyState::new();

        let plan = build_plan(
            graph,
            &node_manager,
            &dirty,
            ExecutionRequest::new(ExecuteTarget::Node(c)),
            ExecutionMode::Manual,
            RunId(4),
            GenerationId(0),
        )
        .unwrap();

        assert!(plan.is_none());
    }
}
