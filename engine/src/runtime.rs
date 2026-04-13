use crate::cache;
use crate::cache::manager::CacheManager;
use crate::capability::{CapabilityRegistry, LocalityHint};
use crate::execution::{
    CookingContext, CookingContextRange, EvaluationFidelity, ExecutionMode, ExecutionOutputs,
    ExecutionTerminalStatus, NodeExecutionRequest, NoopCancelToken, NoopProgressSink,
    PlanLifecycleContext, PlanLifecycleNode, TickSource,
};
use crate::executors;
use crate::executors::image::ImageExecutor;
use crate::facade::{EngineError, ExecuteRequest, ExecutionTicket};
use crate::graph;
use crate::graph::query::resolve_subgraph::resolve_subgraph;
use crate::graph::query::topo_sort::topo_sort;
use crate::graph::NodeInstance;
use crate::node_manager::NodeManager;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use types::{NodeId, Value};

pub struct Runtime {
    node_manager: Arc<NodeManager>,
    capability_registry: Arc<CapabilityRegistry>,
    executor: ImageExecutor,
    cache: CacheManager,
    next_execution_id: crate::execution::ExecutionId,
    execution_results: HashMap<crate::execution::ExecutionId, HashMap<NodeId, HashMap<String, Value>>>,
}

struct LifecycleExecutorBinding {
    executor: Arc<dyn executors::Executor>,
    nodes: Vec<PlanLifecycleNode>,
}

impl Runtime {
    pub fn new(
        node_manager: Arc<NodeManager>,
        capability_registry: Arc<CapabilityRegistry>,
        executor: ImageExecutor,
        cache: CacheManager,
    ) -> Self {
        Self {
            node_manager,
            capability_registry,
            executor,
            cache,
            next_execution_id: 1,
            execution_results: HashMap::new(),
        }
    }

    pub async fn evaluate(
        &self,
        graph: &graph::Graph,
        target: NodeId,
        fidelity: EvaluationFidelity,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>> {
        let order = topo_sort(graph, target)?;
        self.evaluate_order(graph, &order, 0, &CookingContext::default(), fidelity)
            .await
    }

    pub async fn execute_request(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecuteRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        let execution_id = self.next_execution_id;
        self.next_execution_id += 1;
        let mode = request.mode.unwrap_or_default();
        let order = resolve_subgraph(graph, request.target.clone())
            .map_err(|error| EngineError::Execution {
                message: error.to_string(),
            })?
            .order;

        let lifecycle_bindings = self.build_lifecycle_bindings(graph, &order)?;
        let effective_range = if cooking_range.axes.is_empty() {
            self.infer_cooking_range(graph, &order)?
        } else {
            cooking_range.clone()
        };

        for binding in &lifecycle_bindings {
            let lifecycle = PlanLifecycleContext {
                run_id: execution_id,
                my_nodes: &binding.nodes,
                mode: &mode,
            };
            binding
                .executor
                .on_plan_started(&lifecycle)
                .map_err(|error| EngineError::Execution {
                    message: error.to_string(),
                })?;
        }

        let run_result = match mode.clone() {
            ExecutionMode::OneShot { fidelity } => {
                let mut aggregated = HashMap::new();
                for context in effective_range.enumerate() {
                    let frame_outputs = self
                        .evaluate_order(graph, &order, execution_id, &context, fidelity.clone())
                        .await
                        .map_err(|error| EngineError::Execution {
                            message: error.to_string(),
                        })?;
                    for (node_id, outputs) in frame_outputs {
                        aggregated.insert(node_id, outputs);
                    }
                }
                Ok(aggregated)
            }
            ExecutionMode::Continuous { tick_source, .. } => Err(EngineError::Execution {
                message: match tick_source {
                    TickSource::OsClock => {
                        "Continuous mode is not implemented in the current runtime".into()
                    }
                    TickSource::External(name) | TickSource::DataPull(name) => {
                        format!("continuous tick source '{name}' is not implemented")
                    }
                },
            }),
        };

        let terminal_status = if run_result.is_ok() {
            ExecutionTerminalStatus::Finished
        } else {
            ExecutionTerminalStatus::Error
        };
        for binding in &lifecycle_bindings {
            let lifecycle = PlanLifecycleContext {
                run_id: execution_id,
                my_nodes: &binding.nodes,
                mode: &mode,
            };
            binding
                .executor
                .on_plan_finished(&lifecycle, terminal_status);
        }

        let outputs = run_result?;
        self.execution_results.insert(execution_id, outputs);

        Ok(ExecutionTicket {
            execution_id,
            target: request.target,
        })
    }

    pub fn get_execution_outputs(
        &self,
        execution_id: crate::execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        self.execution_results
            .get(&execution_id)
            .and_then(|outputs| outputs.get(&node_id))
            .cloned()
            .ok_or(EngineError::ResultNotFound {
                execution_id,
                node_id,
            })
    }

    pub fn invalidate_nodes(&self, node_ids: &HashSet<NodeId>) {
        if node_ids.is_empty() {
            return;
        }
        let ids = node_ids.iter().copied().collect::<Vec<_>>();
        self.cache.invalidate_subgraph(&ids);
    }

    async fn evaluate_order(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
        run_id: crate::execution::RunId,
        cooking_context: &CookingContext,
        fidelity: EvaluationFidelity,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>> {
        let ctx = self.executor.context();
        let generation = self.cache.current_generation();
        let mut results: HashMap<NodeId, HashMap<String, Value>> = HashMap::new();
        let mut signatures: HashMap<NodeId, cache::model::ExecSignature> = HashMap::new();

        for node_id in order {
            let node = graph
                .nodes
                .get(node_id)
                .ok_or_else(|| format!("Node {:?} not found in graph", node_id))?;
            let def = self
                .node_manager
                .get_node_def(&node.type_id)
                .ok_or_else(|| format!("Node type '{}' not registered", node.type_id))?;

            let mut upstream_inputs: HashMap<String, Value> = HashMap::new();
            let mut upstream_signatures = Vec::new();
            for conn in &graph.connections {
                if conn.to.node == *node_id {
                    if let Some(upstream_outputs) = results.get(&conn.from.node) {
                        if let Some(val) = upstream_outputs.get(&conn.from.interface) {
                            upstream_inputs.insert(conn.to.interface.clone(), val.clone());
                        }
                    }
                    if let Some(signature) = signatures.get(&conn.from.node) {
                        upstream_signatures.push(*signature);
                    }
                }
            }

            let effective_params = self
                .node_manager
                .resolve_effective_params(&node.type_id, &node.params)
                .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { Box::new(error) })?;
            let exec_signature = compute_exec_signature(
                self.node_manager.as_ref(),
                def,
                node,
                &effective_params,
                &upstream_signatures,
                cooking_context,
            );

            let mut inputs = upstream_inputs.clone();
            for (name, value) in effective_params {
                inputs.entry(name).or_insert(value);
            }

            if !def.outputs.is_empty() {
                let mut cached_outputs: HashMap<String, Value> = HashMap::new();
                let mut all_cached = true;

                for output in &def.outputs {
                    match self
                        .cache
                        .get_result(*node_id, &output.name, exec_signature, generation)
                    {
                        Some(value) => {
                            cached_outputs.insert(output.name.clone(), value.as_ref().clone());
                        }
                        None => {
                            all_cached = false;
                            break;
                        }
                    }
                }

                if all_cached {
                    results.insert(*node_id, cached_outputs);
                    signatures.insert(*node_id, exec_signature);
                    continue;
                }
            }

            let outputs = match self
                .capability_registry
                .route_requirements(&def.requires, LocalityHint::Any)
            {
                Some(executor) => {
                    let capability_id = def.requires.first().expect("requires checked non-empty");
                    executor
                        .execute(
                            capability_id,
                            NodeExecutionRequest {
                                run_id,
                                node_id: *node_id,
                                node_def: def,
                                inputs,
                                exec_signature,
                                generation,
                                cooking_context: cooking_context.clone(),
                                fidelity: fidelity.clone(),
                                timeout_ms: def.execution.timeout_ms,
                                cancel_token: Arc::new(NoopCancelToken),
                                progress_sink: Arc::new(NoopProgressSink),
                            },
                        )
                        .await
                        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> {
                            Box::new(error)
                        })?
                }
                None => ExecutionOutputs::full((def.execute)(ctx, inputs).await?),
            };

            for (output_pin, value) in &outputs.values {
                let _ = self.cache.put_result(
                    *node_id,
                    output_pin,
                    exec_signature,
                    generation,
                    value.clone(),
                );
            }

            results.insert(*node_id, outputs.values);
            signatures.insert(*node_id, exec_signature);
        }

        Ok(results)
    }

    fn build_lifecycle_bindings(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
    ) -> Result<Vec<LifecycleExecutorBinding>, EngineError> {
        let mut bindings = Vec::<LifecycleExecutorBinding>::new();
        let mut binding_index = HashMap::<usize, usize>::new();

        for node_id in order {
            let node = graph.nodes.get(node_id).ok_or_else(|| EngineError::Graph {
                message: format!("Node {:?} not found in graph", node_id),
            })?;
            let def = self
                .node_manager
                .get_node_def(&node.type_id)
                .ok_or_else(|| EngineError::Graph {
                    message: format!("Node type '{}' not registered", node.type_id),
                })?;
            if def.requires.is_empty() {
                continue;
            }

            let Some(executor) = self
                .capability_registry
                .route_requirements(&def.requires, LocalityHint::Any)
            else {
                continue;
            };
            let key = Arc::as_ptr(&executor) as *const () as usize;
            let index = if let Some(index) = binding_index.get(&key).copied() {
                index
            } else {
                let index = bindings.len();
                bindings.push(LifecycleExecutorBinding {
                    executor: Arc::clone(&executor),
                    nodes: Vec::new(),
                });
                binding_index.insert(key, index);
                index
            };

            let resolved_params = self
                .node_manager
                .resolve_effective_params(&node.type_id, &node.params)
                .map_err(|error| EngineError::Schema {
                    message: error.to_string(),
                })?;
            bindings[index].nodes.push(PlanLifecycleNode {
                node_id: *node_id,
                type_id: node.type_id.clone(),
                resolved_params,
            });
        }

        Ok(bindings)
    }

    fn infer_cooking_range(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
    ) -> Result<CookingContextRange, EngineError> {
        let mut max_frame_count = 0_u64;

        for node_id in order {
            let Some(node) = graph.nodes.get(node_id) else {
                continue;
            };
            let Some(def) = self.node_manager.get_node_def(&node.type_id) else {
                continue;
            };
            if !def.cooking_sensitivity.iter().any(|axis| axis == "frame") {
                continue;
            }

            let params = self
                .node_manager
                .resolve_effective_params(&node.type_id, &node.params)
                .map_err(|error| EngineError::Schema {
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
}

fn compute_exec_signature(
    node_manager: &NodeManager,
    def: &crate::node_manager::model::NodeDef,
    node: &NodeInstance,
    effective_params: &HashMap<String, Value>,
    upstream_signatures: &[cache::model::ExecSignature],
    cooking_context: &CookingContext,
) -> cache::model::ExecSignature {
    let mut params_entries: Vec<(&String, &Value)> = effective_params.iter().collect();
    params_entries.sort_by(|a, b| a.0.cmp(b.0));

    let params_hash = hash_entries(&params_entries);
    let upstream_hash = hash_signatures(upstream_signatures);
    let node_version = def.version.max(1) as u16;
    let mut capability_versions = def
        .requires
        .iter()
        .filter_map(|capability| node_manager.capability_version_of(&def.type_id, capability))
        .collect::<Vec<_>>();
    capability_versions.sort();
    let capability_version = capability_versions.into_iter().fold(0_u32, |acc, version| {
        acc.wrapping_mul(31).wrapping_add(version)
    });
    let cooking_context_hash = cooking_context.hash_filtered(&def.cooking_sensitivity);

    let _ = node;

    cache::model::ExecSignature::with_context(
        2,
        node_version,
        params_hash,
        upstream_hash,
        cooking_context_hash,
        capability_version,
    )
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

fn hash_entries(entries: &[(&String, &Value)]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for (key, value) in entries {
        key.hash(&mut hasher);
        format!("{value:?}").hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_signatures(signatures: &[cache::model::ExecSignature]) -> u64 {
    let mut ordered = signatures.to_vec();
    ordered.sort_by_key(|signature| {
        (
            signature.sig_schema_version,
            signature.node_version,
            signature.params_hash,
            signature.upstream_hash,
            signature.cooking_context_hash,
            signature.capability_version,
        )
    });

    let mut hasher = DefaultHasher::new();
    for signature in ordered {
        signature.hash(&mut hasher);
    }
    hasher.finish()
}
