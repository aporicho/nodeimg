use std::collections::HashMap;

use crate::cache::model::ExecSignature;
use crate::execution::{
    CookingContext, CookingContextRange, ExecutionPlan, PinSource, PlanSubtask, PlannedNode,
    PlannedOutput,
};
use crate::graph;
use crate::graph::query::resolve_subgraph::resolve_subgraph;
use crate::node_manager::{ArtifactPolicy, Purity};
use crate::planner::error::PlannerError;
use crate::planner::request::PlanRequest;
use crate::planner::signature::{compute_exec_signature, SignatureInput};
use types::{NodeId, Value};

pub(crate) fn build_execution_plan(input: PlanRequest<'_>) -> Result<ExecutionPlan, PlannerError> {
    let resolved =
        resolve_subgraph(input.graph, input.request.target.clone()).map_err(|error| {
            PlannerError::Graph {
                message: error.to_string(),
            }
        })?;
    let effective_range = if input.cooking_range.axes.is_empty() {
        infer_cooking_range(input.graph, input.node_manager, &resolved.order)?
    } else {
        input.cooking_range.clone()
    };

    let mut subtasks = Vec::new();
    for cooking_context in effective_range.enumerate() {
        let order = plan_order(
            input.graph,
            input.node_manager,
            &resolved.order,
            &cooking_context,
        )?;
        subtasks.push(PlanSubtask {
            cooking_context,
            order,
        });
    }

    Ok(ExecutionPlan {
        target: input.request.target.clone(),
        mode: input.mode,
        subtasks,
    })
}

fn plan_order(
    graph: &graph::Graph,
    node_manager: &crate::node_manager::NodeManager,
    order: &[NodeId],
    cooking_context: &CookingContext,
) -> Result<Vec<PlannedNode>, PlannerError> {
    let mut signatures = HashMap::<NodeId, ExecSignature>::new();
    let mut planned = Vec::new();

    for node_id in order {
        let node = graph
            .nodes
            .get(node_id)
            .ok_or(PlannerError::NodeNotFound { node_id: *node_id })?;
        let def = node_manager.get_node_def(&node.type_id).ok_or_else(|| {
            PlannerError::NodeTypeNotRegistered {
                type_id: node.type_id.clone(),
            }
        })?;

        let mut inputs = HashMap::new();
        let mut upstream_signatures = Vec::new();
        for conn in &graph.connections {
            if conn.to.node == *node_id {
                inputs.insert(
                    conn.to.interface.clone(),
                    PinSource::UpstreamOutput {
                        node_id: conn.from.node,
                        output_pin: conn.from.interface.clone(),
                    },
                );
                if let Some(signature) = signatures.get(&conn.from.node) {
                    upstream_signatures.push(*signature);
                }
            }
        }

        let effective_params = node_manager
            .resolve_effective_params(&node.type_id, &node.params)
            .map_err(|error| PlannerError::Schema {
                message: error.to_string(),
            })?;
        let exec_signature = compute_exec_signature(SignatureInput {
            node_manager,
            node_def: def,
            effective_params: &effective_params,
            upstream_signatures: &upstream_signatures,
            cooking_context,
        });
        for (name, value) in &effective_params {
            inputs
                .entry(name.clone())
                .or_insert_with(|| PinSource::Value(value.clone()));
        }

        let artifact_policy = def.execution.artifact_policy.unwrap_or(match def.purity {
            Purity::Pure => ArtifactPolicy::None,
            Purity::Impure => ArtifactPolicy::Persist,
        });
        planned.push(PlannedNode {
            node_id: *node_id,
            type_id: node.type_id.clone(),
            params: effective_params,
            inputs,
            exec_signature,
            outputs: def
                .outputs
                .iter()
                .map(|output| PlannedOutput {
                    name: output.name.clone(),
                    optional: output.optional,
                })
                .collect(),
            requires: def.requires.clone(),
            timeout_ms: def.execution.timeout_ms,
            realtime_capable: def.realtime_capable,
            artifact_policy,
        });
        signatures.insert(*node_id, exec_signature);
    }

    Ok(planned)
}

fn infer_cooking_range(
    graph: &graph::Graph,
    node_manager: &crate::node_manager::NodeManager,
    order: &[NodeId],
) -> Result<CookingContextRange, PlannerError> {
    let mut max_frame_count = 0_u64;

    for node_id in order {
        let Some(node) = graph.nodes.get(node_id) else {
            continue;
        };
        let Some(def) = node_manager.get_node_def(&node.type_id) else {
            continue;
        };
        if !def.cooking_sensitivity.iter().any(|axis| axis == "frame") {
            continue;
        }

        let params = node_manager
            .resolve_effective_params(&node.type_id, &node.params)
            .map_err(|error| PlannerError::Schema {
                message: error.to_string(),
            })?;
        if let (Some(duration), Some(fps)) = (
            positive_int_param(&params, "duration_seconds"),
            inferred_fps_for_node(def.type_id.as_str(), &params),
        ) {
            max_frame_count = max_frame_count.max(duration.saturating_mul(fps));
        }
    }

    if max_frame_count > 0 {
        Ok(CookingContextRange::frame_range(0, max_frame_count))
    } else {
        Ok(CookingContextRange::default())
    }
}

fn positive_int_param(params: &HashMap<String, Value>, key: &str) -> Option<u64> {
    match params.get(key) {
        Some(Value::Int(value)) if *value > 0 => Some(*value as u64),
        Some(Value::Float(value)) if *value > 0.0 => Some(*value as u64),
        _ => None,
    }
}

fn inferred_fps_for_node(type_id: &str, params: &HashMap<String, Value>) -> Option<u64> {
    positive_int_param(params, "fps").or_else(|| match type_id {
        "ai_video_generate_api" => match params.get("provider") {
            Some(Value::String(provider)) if provider == "libtv" || provider == "mock" => Some(8),
            _ => None,
        },
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{DimensionValue, ExecutionMode};
    use crate::facade::ExecutionRequest;
    use crate::graph::model::subgraph::ExecuteTarget;
    use crate::graph::{Connection, Graph, PinRef};
    use crate::node_manager::{
        ExecutionPolicy, NodeDef, NodeManager, NodeSourceKind, ParamDef, ParamExpose, PinDef,
    };
    use types::DataType;

    fn make_def(type_id: &str) -> NodeDef {
        NodeDef {
            type_id: type_id.into(),
            version: 1,
            source: NodeSourceKind::Builtin,
            name: type_id.into(),
            category: "test".into(),
            requires: vec!["test.capability".into()],
            purity: Purity::Pure,
            cooking_sensitivity: vec![],
            realtime_capable: true,
            execution: ExecutionPolicy::default(),
            api: None,
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

    fn make_frame_def(type_id: &str) -> NodeDef {
        let mut def = make_def(type_id);
        def.cooking_sensitivity = vec!["frame".into()];
        def.params.push(ParamDef {
            name: "duration_seconds".into(),
            data_type: DataType::int(),
            constraint: None,
            default_value: Value::Int(2),
            expose: vec![ParamExpose::Control],
        });
        def.params.push(ParamDef {
            name: "fps".into(),
            data_type: DataType::int(),
            constraint: None,
            default_value: Value::Int(3),
            expose: vec![ParamExpose::Control],
        });
        def
    }

    fn request(target: ExecuteTarget) -> ExecutionRequest {
        ExecutionRequest {
            target,
            mode: Some(ExecutionMode::default()),
        }
    }

    #[test]
    fn plans_single_node_with_default_params() {
        let graph = Graph::new();
        let (graph, node_id) = graph.add_node("a", Default::default());
        let mut node_manager = NodeManager::new();
        node_manager.register(make_def("a"));

        let plan = build_execution_plan(PlanRequest {
            graph: &graph,
            request: &request(ExecuteTarget::Node(node_id)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap();

        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].order.len(), 1);
        let planned = &plan.subtasks[0].order[0];
        assert_eq!(planned.node_id, node_id);
        assert!(matches!(planned.params.get("amount"), Some(Value::Float(v)) if *v == 1.0));
        assert!(matches!(
            planned.inputs.get("amount"),
            Some(PinSource::Value(Value::Float(v))) if *v == 1.0
        ));
    }

    #[test]
    fn plans_topological_order_and_upstream_inputs() {
        let graph = Graph::new();
        let (graph, a) = graph.add_node("a", Default::default());
        let (graph, b) = graph.add_node("b", Default::default());
        let (graph, c) = graph.add_node("c", Default::default());
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
        let graph = graph.connect(Connection {
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
        node_manager.register(make_def("a"));
        node_manager.register(make_def("b"));
        node_manager.register(make_def("c"));

        let plan = build_execution_plan(PlanRequest {
            graph: &graph,
            request: &request(ExecuteTarget::Node(c)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap();

        let order = &plan.subtasks[0].order;
        assert_eq!(
            order.iter().map(|node| node.node_id).collect::<Vec<_>>(),
            vec![a, b, c]
        );
        assert!(matches!(
            order[1].inputs.get("in"),
            Some(PinSource::UpstreamOutput { node_id, output_pin })
                if *node_id == a && output_pin == "out"
        ));
    }

    #[test]
    fn infers_frame_cooking_range() {
        let graph = Graph::new();
        let (graph, node_id) = graph.add_node("video", Default::default());
        let mut node_manager = NodeManager::new();
        node_manager.register(make_frame_def("video"));

        let plan = build_execution_plan(PlanRequest {
            graph: &graph,
            request: &request(ExecuteTarget::Node(node_id)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap();

        assert_eq!(plan.subtasks.len(), 6);
        for (idx, subtask) in plan.subtasks.iter().enumerate() {
            assert_eq!(
                subtask.cooking_context.dimensions.get("frame"),
                Some(&DimensionValue::Int(idx as i64))
            );
        }
    }

    #[test]
    fn parameter_changes_affect_signature() {
        let graph = Graph::new();
        let (graph, node_id) =
            graph.add_node("a", HashMap::from([("amount".into(), Value::Float(1.0))]));
        let changed = graph.set_param(node_id, "amount", Value::Float(2.0));
        let mut node_manager = NodeManager::new();
        node_manager.register(make_def("a"));

        let original = build_execution_plan(PlanRequest {
            graph: &graph,
            request: &request(ExecuteTarget::Node(node_id)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap();
        let changed = build_execution_plan(PlanRequest {
            graph: &changed,
            request: &request(ExecuteTarget::Node(node_id)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap();

        assert_ne!(
            original.subtasks[0].order[0].exec_signature,
            changed.subtasks[0].order[0].exec_signature
        );
    }

    #[test]
    fn returns_error_for_unregistered_node_type() {
        let graph = Graph::new();
        let (graph, node_id) = graph.add_node("missing", Default::default());
        let node_manager = NodeManager::new();

        let error = build_execution_plan(PlanRequest {
            graph: &graph,
            request: &request(ExecuteTarget::Node(node_id)),
            mode: ExecutionMode::default(),
            cooking_range: &CookingContextRange::default(),
            node_manager: &node_manager,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            PlannerError::NodeTypeNotRegistered { type_id } if type_id == "missing"
        ));
    }
}
