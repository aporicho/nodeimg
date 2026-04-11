use std::collections::HashMap;

use crate::graph::Graph;
use crate::node_manager::NodeManager;
use types::{NodeId, Value};

pub type NodeOutputs = HashMap<String, Value>;
pub type ExecutionResults = HashMap<NodeId, NodeOutputs>;

pub fn resolve_node_inputs(
    graph: &Graph,
    node_manager: &NodeManager,
    results: &ExecutionResults,
    node_id: NodeId,
) -> Result<NodeOutputs, ResolveInputError> {
    let node = graph
        .nodes
        .get(&node_id)
        .ok_or(ResolveInputError::NodeNotFound { node_id })?;

    let def = node_manager
        .get_node_def(&node.type_id)
        .ok_or_else(|| ResolveInputError::NodeDefNotFound {
            type_id: node.type_id.clone(),
        })?;

    let mut inputs: NodeOutputs = HashMap::new();

    for conn in &graph.connections {
        if conn.to.node == node_id {
            if let Some(upstream_outputs) = results.get(&conn.from.node) {
                if let Some(value) = upstream_outputs.get(&conn.from.interface) {
                    inputs.insert(conn.to.interface.clone(), value.clone());
                }
            }
        }
    }

    for param_def in &def.params {
        if !inputs.contains_key(&param_def.name) {
            let value = node
                .params
                .get(&param_def.name)
                .cloned()
                .unwrap_or_else(|| param_def.default_value.clone());
            inputs.insert(param_def.name.clone(), value);
        }
    }

    Ok(inputs)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveInputError {
    NodeNotFound { node_id: NodeId },
    NodeDefNotFound { type_id: String },
}

impl std::fmt::Display for ResolveInputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound { node_id } => write!(f, "Node {:?} not found in graph", node_id),
            Self::NodeDefNotFound { type_id } => {
                write!(f, "Node type '{}' not registered", type_id)
            }
        }
    }
}

impl std::error::Error for ResolveInputError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, PinRef};
    use crate::node_manager::{ExecutorType, NodeDef, ParamDef, ParamExpose, PinDef};
    use types::{DataType, NodeId};

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

    fn sample_graph() -> (Graph, NodeId, NodeId) {
        let graph = Graph::new();
        let (graph, a) = graph.add_node("source", Default::default());
        let (graph, b) = graph.add_node("target", Default::default());
        let graph = graph.connect(Connection {
            from: PinRef {
                node: a,
                interface: "out".into(),
            },
            to: PinRef {
                node: b,
                interface: "in".into(),
            },
        });
        (graph, a, b)
    }

    #[test]
    fn merges_upstream_outputs_with_default_params() {
        let (graph, a, b) = sample_graph();
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("source"));
        node_manager.register(make_test_def("target"));

        let mut results = ExecutionResults::new();
        results.insert(
            a,
            HashMap::from([(String::from("out"), Value::Float(2.0))]),
        );

        let inputs = resolve_node_inputs(&graph, &node_manager, &results, b).unwrap();

        assert!(matches!(inputs.get("in"), Some(Value::Float(v)) if *v == 2.0));
        assert!(matches!(inputs.get("amount"), Some(Value::Float(v)) if *v == 1.0));
    }

    #[test]
    fn node_params_override_defaults() {
        let (graph, _a, b) = sample_graph();
        let graph = graph.set_param(b, "amount", Value::Float(5.0));
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("source"));
        node_manager.register(make_test_def("target"));

        let results = ExecutionResults::new();
        let inputs = resolve_node_inputs(&graph, &node_manager, &results, b).unwrap();

        assert!(matches!(inputs.get("amount"), Some(Value::Float(v)) if *v == 5.0));
    }

    #[test]
    fn returns_error_when_node_missing() {
        let graph = Graph::new();
        let node_manager = NodeManager::new();
        let results = ExecutionResults::new();

        let error = resolve_node_inputs(&graph, &node_manager, &results, NodeId(999)).unwrap_err();

        assert_eq!(error, ResolveInputError::NodeNotFound { node_id: NodeId(999) });
    }
}
