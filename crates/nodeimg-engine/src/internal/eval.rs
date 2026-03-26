use nodeimg_gpu::GpuContext;
use crate::internal::backend::BackendClient;
use crate::internal::cache::{Cache, NodeId};
use crate::internal::registry::{NodeInstance, NodeRegistry};
use nodeimg_types::data_type::DataTypeRegistry;
use nodeimg_types::value::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

pub struct EvalEngine;

/// Represents a graph edge: from output pin to input pin.
pub struct Connection {
    pub from_node: NodeId,
    pub from_pin: String,
    pub to_node: NodeId,
    pub to_pin: String,
}

impl EvalEngine {
    /// Topological sort from a target node, returning nodes in evaluation order.
    /// Returns Err if a cycle is detected.
    pub fn topo_sort(target: NodeId, connections: &[Connection]) -> Result<Vec<NodeId>, String> {
        let mut deps: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        let mut all_nodes: HashSet<NodeId> = HashSet::new();

        let mut to_visit = VecDeque::new();
        to_visit.push_back(target);
        let mut visited = HashSet::new();

        while let Some(node) = to_visit.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            all_nodes.insert(node);
            for conn in connections {
                if conn.to_node == node {
                    deps.entry(node).or_default().insert(conn.from_node);
                    to_visit.push_back(conn.from_node);
                }
            }
        }

        // Kahn's algorithm
        let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
        for &n in &all_nodes {
            in_degree.entry(n).or_insert(0);
            if let Some(d) = deps.get(&n) {
                *in_degree.get_mut(&n).unwrap() = d.len();
            }
        }

        let mut queue: VecDeque<NodeId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&n, _)| n)
            .collect();

        let mut order = Vec::new();
        while let Some(n) = queue.pop_front() {
            order.push(n);
            for (&node, node_deps) in &deps {
                if node_deps.contains(&n) {
                    let deg = in_degree.get_mut(&node).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(node);
                    }
                }
            }
        }

        if order.len() != all_nodes.len() {
            return Err("Cycle detected in node graph".to_string());
        }

        Ok(order)
    }

    /// Check if evaluating `target` would require an AI backend execution.
    ///
    /// Walks the graph from `target` in topo order and finds the **last**
    /// (closest to output) uncached AI node. `serialize_ai_subgraph` then
    /// BFS-es upstream from that node, collecting the entire AI subgraph
    /// into a single backend request — avoiding redundant re-execution of
    /// shared upstream nodes like LoadCheckpoint.
    ///
    /// This is a **read-only** operation — it does not modify the cache.
    pub fn pending_ai_execution(
        target: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
        cache: &Cache,
    ) -> Option<(NodeId, serde_json::Value)> {
        let order = Self::topo_sort(target, connections).ok()?;

        // Find the last uncached AI node in topo order (closest to output).
        // Skip missing/unknown nodes instead of aborting the entire search.
        let mut last_ai: Option<NodeId> = None;
        for node_id in order {
            if cache.get(node_id).is_some() {
                continue;
            }
            let instance = match nodes.get(&node_id) {
                Some(inst) => inst,
                None => continue,
            };
            let def = match node_registry.get(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };
            if def.is_ai_node() {
                last_ai = Some(node_id);
            }
        }

        let ai_node_id = last_ai?;
        let graph_json = BackendClient::serialize_ai_subgraph(
            ai_node_id,
            nodes,
            connections,
            node_registry,
        )?;
        Some((ai_node_id, graph_json))
    }

    /// Evaluate a target node, using cache where possible.
    ///
    /// When `backend` is provided, AI nodes (those with `process: None`) will be
    /// serialized and sent to the Python backend for execution. Without a backend,
    /// AI nodes are silently skipped.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        target: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
        type_registry: &DataTypeRegistry,
        cache: &mut Cache,
        backend: Option<&BackendClient>,
        gpu_ctx: Option<&Arc<GpuContext>>,
    ) -> Result<(), String> {
        let order = Self::topo_sort(target, connections)?;

        cache.clear_downstream();
        for conn in connections {
            cache.set_downstream(conn.from_node, conn.to_node);
        }

        // Track AI nodes that have been sent to the backend so we skip them
        let mut sent_to_backend: HashSet<NodeId> = HashSet::new();

        for node_id in order {
            if cache.get(node_id).is_some() || sent_to_backend.contains(&node_id) {
                continue;
            }

            // Error isolation: skip nodes that can't be evaluated
            let instance = match nodes.get(&node_id) {
                Some(inst) => inst,
                None => continue,
            };
            let def = match node_registry.get(&instance.type_id) {
                Some(d) => d,
                None => continue,
            };

            // Gather inputs (shared by GPU and CPU paths)
            let mut inputs: HashMap<String, Value> = HashMap::new();
            for conn in connections {
                if conn.to_node == node_id {
                    if let Some(upstream_outputs) = cache.get(conn.from_node) {
                        if let Some(val) = upstream_outputs.get(&conn.from_pin) {
                            let expected_type = def
                                .inputs
                                .iter()
                                .find(|pin| pin.name == conn.to_pin)
                                .map(|pin| &pin.data_type);

                            let upstream_type = nodes
                                .get(&conn.from_node)
                                .and_then(|inst| node_registry.get(&inst.type_id))
                                .and_then(|upstream_def| {
                                    upstream_def
                                        .outputs
                                        .iter()
                                        .find(|pin| pin.name == conn.from_pin)
                                        .map(|pin| &pin.data_type)
                                });

                            let converted = match (upstream_type, expected_type) {
                                (Some(from_type), Some(to_type)) if from_type != to_type => {
                                    type_registry
                                        .convert(val.clone(), from_type, to_type)
                                        .unwrap_or_else(|| val.clone())
                                }
                                _ => val.clone(),
                            };

                            inputs.insert(conn.to_pin.clone(), converted);
                        }
                    }
                }
            }

            // GPU path (highest priority)
            if let Some(ref gpu_process) = def.gpu_process {
                if let Some(ctx) = gpu_ctx {
                    eprintln!("[eval] node {} ({}) -> GPU", node_id, def.title);
                    let outputs = gpu_process(ctx, &inputs, &instance.params);
                    cache.insert(node_id, outputs);
                    continue;
                }
            }

            // GPU-only nodes without GPU context: return an error
            if def.gpu_process.is_some() && gpu_ctx.is_none() && def.process.is_none() {
                return Err(format!("Node '{}' requires GPU but no GPU context available", def.title));
            }

            // CPU path — auto-convert GpuImage inputs to CPU Image via readback
            if let Some(ref process) = def.process {
                if let Some(ctx) = gpu_ctx {
                    for val in inputs.values_mut() {
                        if let Value::GpuImage(tex) = val {
                            *val = Value::Image(std::sync::Arc::new(
                                tex.to_dynamic_image(&ctx.device, &ctx.queue),
                            ));
                        }
                    }
                }
                eprintln!("[eval] node {} ({}) -> CPU", node_id, def.title);
                let outputs = process(&inputs, &instance.params);
                cache.insert(node_id, outputs);
            } else if def.gpu_process.is_none() {
                // AI node (process == None && gpu_process == None): send to backend
                if let Some(client) = backend {
                    Self::execute_ai_subgraph(
                        node_id,
                        nodes,
                        connections,
                        node_registry,
                        cache,
                        client,
                        &mut sent_to_backend,
                    )?;
                }
                // Without backend, AI nodes are silently skipped
            }
        }

        Ok(())
    }

    /// Execute an AI subgraph by sending it to the Python backend.
    ///
    /// Serializes all connected AI nodes upstream of `ai_node_id`, sends them
    /// to the backend, and caches the result. All nodes in the subgraph are
    /// added to `sent_to_backend` so they won't be processed again.
    pub(crate) fn execute_ai_subgraph(
        ai_node_id: NodeId,
        nodes: &HashMap<NodeId, NodeInstance>,
        connections: &[Connection],
        node_registry: &NodeRegistry,
        cache: &mut Cache,
        client: &BackendClient,
        sent_to_backend: &mut HashSet<NodeId>,
    ) -> Result<(), String> {
        // Collect all AI nodes in this subgraph
        let ai_node_ids =
            BackendClient::collect_ai_node_ids(ai_node_id, nodes, connections, node_registry);

        // Mark all as sent so we don't process them individually
        for &nid in &ai_node_ids {
            sent_to_backend.insert(nid);
        }

        // Serialize the AI subgraph
        let graph_json = match BackendClient::serialize_ai_subgraph(
            ai_node_id,
            nodes,
            connections,
            node_registry,
        ) {
            Some(json) => json,
            None => return Ok(()), // Not actually an AI node, skip
        };

        eprintln!(
            "[backend] Executing AI subgraph with output node {} ({} nodes)...",
            ai_node_id,
            ai_node_ids.len()
        );

        // Send to backend (this blocks the UI — acceptable for minimal demo)
        let response = client.execute_graph(&graph_json).map_err(|e| {
            eprintln!("[backend] Execution failed: {}", e);
            e
        })?;

        // Check for error response
        if let Some(error) = response.get("error").and_then(|v| v.as_str()) {
            let failed_node = response
                .get("failed_node")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let msg = format!("Backend error at node {}: {}", failed_node, error);
            eprintln!("[backend] {}", msg);
            return Err(msg);
        }

        // Parse the response and cache the output node's results
        match BackendClient::parse_backend_response(&response, ai_node_id) {
            Ok(outputs) => {
                eprintln!(
                    "[backend] Received {} output(s) for node {}",
                    outputs.len(),
                    ai_node_id
                );
                cache.insert(ai_node_id, outputs);
            }
            Err(e) => {
                eprintln!("[backend] Failed to parse response: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topo_sort_linear() {
        let connections = vec![
            Connection {
                from_node: 0,
                from_pin: "out".into(),
                to_node: 1,
                to_pin: "in".into(),
            },
            Connection {
                from_node: 1,
                from_pin: "out".into(),
                to_node: 2,
                to_pin: "in".into(),
            },
        ];
        let order = EvalEngine::topo_sort(2, &connections).unwrap();
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn test_topo_sort_diamond() {
        let connections = vec![
            Connection {
                from_node: 0,
                from_pin: "out".into(),
                to_node: 1,
                to_pin: "in".into(),
            },
            Connection {
                from_node: 0,
                from_pin: "out".into(),
                to_node: 2,
                to_pin: "in".into(),
            },
            Connection {
                from_node: 1,
                from_pin: "out".into(),
                to_node: 3,
                to_pin: "in1".into(),
            },
            Connection {
                from_node: 2,
                from_pin: "out".into(),
                to_node: 3,
                to_pin: "in2".into(),
            },
        ];
        let order = EvalEngine::topo_sort(3, &connections).unwrap();
        assert_eq!(order[0], 0);
        assert_eq!(order[3], 3);
    }

    #[test]
    fn test_topo_sort_cycle() {
        let connections = vec![
            Connection {
                from_node: 0,
                from_pin: "out".into(),
                to_node: 1,
                to_pin: "in".into(),
            },
            Connection {
                from_node: 1,
                from_pin: "out".into(),
                to_node: 0,
                to_pin: "in".into(),
            },
        ];
        let result = EvalEngine::topo_sort(0, &connections);
        assert!(result.is_err());
    }

    #[test]
    fn test_evaluate_with_real_registry() {
        use nodeimg_types::category::CategoryId;
        use nodeimg_types::constraint::Constraint;
        use nodeimg_types::data_type::DataTypeId;
        use crate::internal::registry::{NodeDef, ParamDef, PinDef};

        let mut node_reg = NodeRegistry::new();
        let type_reg = DataTypeRegistry::with_builtins();

        node_reg.register(NodeDef {
            type_id: "source".into(),
            title: "Source".into(),
            category: CategoryId::new("tool"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "value".into(),
                data_type: DataTypeId::new("float"),
                required: false,
            }],
            params: vec![ParamDef {
                name: "value".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::None,
                default: Value::Float(0.0),
                widget_override: None,
            }],
            has_preview: false,
            process: Some(Box::new(|_inputs, params| {
                let mut out = HashMap::new();
                if let Some(v) = params.get("value") {
                    out.insert("value".into(), v.clone());
                }
                out
            })),
            gpu_process: None,
        });

        node_reg.register(NodeDef {
            type_id: "add_one".into(),
            title: "Add One".into(),
            category: CategoryId::new("tool"),
            inputs: vec![PinDef {
                name: "value".into(),
                data_type: DataTypeId::new("float"),
                required: true,
            }],
            outputs: vec![PinDef {
                name: "result".into(),
                data_type: DataTypeId::new("float"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|inputs, _params| {
                let mut out = HashMap::new();
                if let Some(Value::Float(v)) = inputs.get("value") {
                    out.insert("result".into(), Value::Float(v + 1.0));
                }
                out
            })),
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            NodeInstance {
                type_id: "source".into(),
                params: HashMap::from([("value".into(), Value::Float(5.0))]),
            },
        );
        nodes.insert(
            1,
            NodeInstance {
                type_id: "add_one".into(),
                params: HashMap::new(),
            },
        );

        let connections = vec![Connection {
            from_node: 0,
            from_pin: "value".into(),
            to_node: 1,
            to_pin: "value".into(),
        }];

        let mut cache = Cache::new();
        EvalEngine::evaluate(
            1,
            &nodes,
            &connections,
            &node_reg,
            &type_reg,
            &mut cache,
            None,
            None,
        )
        .unwrap();

        let result = cache.get(1).unwrap();
        match result.get("result") {
            Some(Value::Float(v)) => assert_eq!(*v, 6.0),
            other => panic!("expected Float(6.0), got {:?}", other),
        }
    }

    #[test]
    fn test_evaluate_gpu_only_node_without_gpu_context_returns_error() {
        use nodeimg_types::category::CategoryId;
        use nodeimg_types::data_type::DataTypeId;
        use crate::internal::registry::{NodeDef, PinDef};

        let mut node_reg = NodeRegistry::new();
        let type_reg = DataTypeRegistry::with_builtins();

        // GPU-only 节点：只有 gpu_process，没有 process（CPU 回退）
        node_reg.register(NodeDef {
            type_id: "gpu_only".into(),
            title: "GPU Only Node".into(),
            category: CategoryId::new("tool"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataTypeId::new("float"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: None,
            gpu_process: Some(Box::new(|_ctx, _inputs, _params| HashMap::new())),
        });

        let mut nodes = HashMap::new();
        nodes.insert(
            0,
            NodeInstance {
                type_id: "gpu_only".into(),
                params: HashMap::new(),
            },
        );

        let connections = vec![];
        let mut cache = Cache::new();

        // gpu_ctx: None 时，GPU-only 节点应返回包含 "requires GPU" 的错误
        let result = EvalEngine::evaluate(
            0,
            &nodes,
            &connections,
            &node_reg,
            &type_reg,
            &mut cache,
            None,
            None, // 没有 GPU 上下文
        );

        assert!(result.is_err(), "期望返回错误，但返回了 Ok");
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("requires GPU"),
            "错误信息应包含 'requires GPU'，实际为：{}",
            err_msg
        );
    }

    #[test]
    fn test_topo_sort_single_node() {
        let order = EvalEngine::topo_sort(0, &[]).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn test_evaluate_disconnected_node() {
        use nodeimg_types::category::CategoryId;
        use nodeimg_types::data_type::DataTypeId;
        use crate::internal::registry::{NodeDef, PinDef};

        let mut node_reg = NodeRegistry::new();
        let type_reg = DataTypeRegistry::with_builtins();

        node_reg.register(NodeDef {
            type_id: "passthrough".into(),
            title: "Passthrough".into(),
            category: CategoryId::new("tool"),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "out".into(),
                data_type: DataTypeId::new("float"),
                required: false,
            }],
            params: vec![],
            has_preview: false,
            process: Some(Box::new(|_inputs, _params| {
                HashMap::from([("out".into(), Value::Float(42.0))])
            })),
            gpu_process: None,
        });

        let mut nodes = HashMap::new();
        nodes.insert(0, NodeInstance {
            type_id: "passthrough".into(),
            params: HashMap::new(),
        });

        let mut cache = Cache::new();
        EvalEngine::evaluate(0, &nodes, &[], &node_reg, &type_reg, &mut cache, None, None).unwrap();

        let result = cache.get(0).unwrap();
        match result.get("out") {
            Some(Value::Float(v)) => assert_eq!(*v, 42.0),
            other => panic!("expected Float(42.0), got {:?}", other),
        }
    }
}
