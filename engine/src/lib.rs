pub mod artifact;
pub mod builtins;
pub mod cache;
pub mod capability;
pub mod execution;
pub mod executors;
pub mod facade;
pub mod graph;
pub mod node_manager;
pub mod node_registry;
pub mod scheduler;

use cache::manager::CacheManager;
use capability::{CapabilityRegistry, LocalityHint};
use execution::{
    CookingContext, CookingContextRange, EvaluationFidelity, ExecutionMode, ExecutionOutputs,
    ExecutionTerminalStatus, NodeExecutionRequest, NoopCancelToken, NoopProgressSink,
    PlanLifecycleContext, PlanLifecycleNode, TickSource,
};
use executors::image::{GpuExecutor, ImageExecutor};
use facade::{EngineError, EngineFacade, ExecuteRequest, ExecutionTicket};
use graph::model::subgraph::ExecuteTarget;
use graph::query::topo_sort::topo_sort;
use graph::{GraphController, NodeInstance};
use node_manager::NodeManager;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use types::{NodeId, Value};

/// Engine：顶层协调者。
///
/// 当前持有：
/// - graph：节点图实例与编辑
/// - node_manager：节点定义收集、注册、索引与查询
/// - executor：图像执行入口
pub struct Engine {
    pub graph: GraphController,
    pub node_manager: Arc<NodeManager>,
    pub capability_registry: Arc<CapabilityRegistry>,
    pub executor: ImageExecutor,
    pub cache: CacheManager,
    cooking_range: CookingContextRange,
    next_execution_id: execution::ExecutionId,
    execution_results: HashMap<execution::ExecutionId, HashMap<NodeId, HashMap<String, Value>>>,
}

impl Engine {
    pub fn new(gpu: Option<GpuExecutor>) -> Self {
        let registry = {
            let mut registry = node_registry::NodeRegistry::new();
            registry.register(node_registry::sources::InventoryNodeSource);
            registry.register(node_registry::sources::GenericImageGenerationSource);
            registry.register(node_registry::sources::ApiVideoGenerationSource::new());
            registry.register(node_registry::sources::ColorAdjustSource);
            registry.register(node_registry::sources::SaveVideoSource);
            registry.build()
        };
        Self::from_registry_bundle(registry, gpu)
    }

    pub fn from_registry_bundle(
        registry: node_registry::RegistryBundle,
        gpu: Option<GpuExecutor>,
    ) -> Self {
        let node_manager = Arc::new(registry.node_manager);
        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            node_manager,
            capability_registry: Arc::new(registry.capability_registry),
            executor: ImageExecutor::new(gpu),
            cache: CacheManager::new(),
            cooking_range: CookingContextRange::default(),
            next_execution_id: 1,
            execution_results: HashMap::new(),
        }
    }

    pub async fn evaluate(
        &self,
        target: NodeId,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>>
    {
        self.evaluate_with_fidelity(target, EvaluationFidelity::Full)
            .await
    }

    pub async fn evaluate_with_fidelity(
        &self,
        target: NodeId,
        fidelity: EvaluationFidelity,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>>
    {
        let graph = self.graph.snapshot();
        let order = topo_sort(&graph, target)?;
        self.evaluate_order(&graph, &order, 0, &CookingContext::default(), fidelity)
            .await
    }

    async fn evaluate_order(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
        run_id: execution::RunId,
        cooking_context: &CookingContext,
        fidelity: EvaluationFidelity,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>>
    {
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

            let outputs = if !def.requires.is_empty() {
                let executor = self
                    .capability_registry
                    .route_requirements(&def.requires, locality_hint_for(def.executor_type))
                    .ok_or_else(|| {
                        Box::<dyn std::error::Error + Send + Sync>::from(format!(
                            "no executor registered for requirements {:?}",
                            def.requires
                        ))
                    })?;
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
            } else {
                ExecutionOutputs::full((def.execute)(ctx, inputs).await?)
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

    pub async fn execute_request(
        &mut self,
        request: ExecuteRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        let execution_id = self.next_execution_id;
        self.next_execution_id += 1;
        let mode = request.mode.unwrap_or_default();
        let graph = self.graph.snapshot();
        let order = match request.target.clone() {
            ExecuteTarget::Graph => {
                self.graph
                    .resolve_subgraph(ExecuteTarget::Graph)
                    .map_err(|message| EngineError::Graph { message })?
                    .order
            }
            ExecuteTarget::Node(node_id) => {
                topo_sort(&graph, node_id).map_err(|error| EngineError::Execution {
                    message: error.to_string(),
                })?
            }
        };

        let lifecycle_bindings = self.build_lifecycle_bindings(graph.as_ref(), &order)?;
        let cooking_range = if self.cooking_range.axes.is_empty() {
            self.infer_cooking_range(graph.as_ref(), &order)?
        } else {
            self.cooking_range.clone()
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
                for context in cooking_range.enumerate() {
                    let frame_outputs = self
                        .evaluate_order(
                            graph.as_ref(),
                            &order,
                            execution_id,
                            &context,
                            fidelity.clone(),
                        )
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
                        "Continuous mode is not implemented in the current engine adapter".into()
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
}

impl EngineFacade for Engine {
    fn list_node_defs(&self) -> Vec<&crate::node_manager::NodeDef> {
        self.node_manager.list_node_defs()
    }

    fn resolve_node_schema(
        &self,
        type_id: &str,
        current_params: &HashMap<String, Value>,
    ) -> Result<node_registry::ResolvedSchema, EngineError> {
        self.node_manager
            .resolve_schema(type_id, current_params)
            .map_err(|error| EngineError::Schema {
                message: error.to_string(),
            })
    }

    fn add_node(&mut self, type_id: &str) -> Result<NodeId, EngineError> {
        self.graph
            .add_node(type_id)
            .map_err(|message| EngineError::Graph { message })
    }

    fn set_param(&mut self, node_id: NodeId, param: &str, value: Value, preview: bool) {
        self.graph.set_param(node_id, param, value, preview);
    }

    fn set_cooking_range(&mut self, range: CookingContextRange) {
        self.cooking_range = range;
    }

    fn get_execution_outputs(
        &self,
        execution_id: execution::ExecutionId,
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
}

fn compute_exec_signature(
    node_manager: &node_manager::NodeManager,
    def: &node_manager::model::NodeDef,
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

struct LifecycleExecutorBinding {
    executor: Arc<dyn executors::Executor>,
    nodes: Vec<PlanLifecycleNode>,
}

impl Engine {
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

            let executor = self
                .capability_registry
                .route_requirements(&def.requires, locality_hint_for(def.executor_type))
                .ok_or_else(|| EngineError::Execution {
                    message: format!("no executor registered for requirements {:?}", def.requires),
                })?;
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
                positive_int_param(&params, "fps"),
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

fn locality_hint_for(executor_type: node_manager::ExecutorType) -> LocalityHint {
    match executor_type {
        node_manager::ExecutorType::Image => LocalityHint::PreferCpu,
        node_manager::ExecutorType::Ai | node_manager::ExecutorType::Api => {
            LocalityHint::PreferRemote
        }
    }
}

fn positive_int_param(params: &HashMap<String, Value>, key: &str) -> Option<u64> {
    match params.get(key) {
        Some(Value::Int(value)) if *value > 0 => Some(*value as u64),
        Some(Value::Float(value)) if *value > 0.0 => Some(*value as u64),
        _ => None,
    }
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
