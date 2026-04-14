use crate::cache;
use crate::cache::manager::CacheManager;
use crate::capability::{CapabilityRegistry, LocalityHint};
use crate::execution::{
    CookingContext, CookingContextRange, EvaluationFidelity, ExecutionMode, ExecutionTerminalStatus,
    ExecutorProgressEvent, NodeExecutionRequest, NoopCancelToken, NoopProgressSink,
    PlanLifecycleContext, PlanLifecycleNode, ProgressSink, TickSource,
};
use crate::events::{EngineEvent, EventBus, EventSubscription, ExecutionState, ExecutionStatus, NodeExecutionStatus, RunningExecution};
use crate::executors;
use crate::executors::image::ImageExecutor;
use crate::execution::ExecutionOutputs;
use crate::facade::{EngineError, ExecutionRequest, ExecutionTicket};
use crate::graph;
use crate::graph::model::subgraph::ExecuteTarget;
use crate::graph::query::resolve_subgraph::resolve_subgraph;
use crate::graph::query::topo_sort::topo_sort;
use crate::graph::NodeInstance;
use crate::node_manager::NodeManager;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Condvar, Mutex};
use types::{NodeId, Value};

#[derive(Clone)]
pub struct Runtime {
    node_manager: Arc<NodeManager>,
    capability_registry: Arc<CapabilityRegistry>,
    executor: Arc<ImageExecutor>,
    cache: Arc<CacheManager>,
    next_execution_id: crate::execution::ExecutionId,
    execution_results:
        Arc<Mutex<HashMap<crate::execution::ExecutionId, HashMap<NodeId, ExecutionOutputs>>>>,
    execution_state: Arc<Mutex<ExecutionState>>,
    completion_notifiers: Arc<Mutex<HashMap<crate::execution::ExecutionId, ExecutionCompletion>>>,
    engine_events: EventBus<EngineEvent>,
}

type ExecutionCompletion = Arc<(Mutex<Option<ExecutionTerminalStatus>>, Condvar)>;

struct LifecycleExecutorBinding {
    executor: Arc<dyn executors::Executor>,
    nodes: Vec<PlanLifecycleNode>,
}

struct NodeProgressSink {
    engine_events: EventBus<EngineEvent>,
    execution_id: crate::execution::ExecutionId,
    node_id: NodeId,
    exec_signature: cache::model::ExecSignature,
    fidelity: EvaluationFidelity,
    downstream: Arc<dyn ProgressSink>,
}

impl ProgressSink for NodeProgressSink {
    fn report(&self, event: ExecutorProgressEvent) {
        match &event {
            ExecutorProgressEvent::Message { text } => {
                let _ = self.engine_events.publish(EngineEvent::NodeProgressMessage {
                    execution_id: self.execution_id,
                    node_id: self.node_id,
                    text: text.clone(),
                });
            }
            ExecutorProgressEvent::Fraction { current, total } => {
                let _ = self.engine_events.publish(EngineEvent::NodeProgressFraction {
                    execution_id: self.execution_id,
                    node_id: self.node_id,
                    current: *current,
                    total: *total,
                });
            }
            ExecutorProgressEvent::Preview { .. } => {}
            ExecutorProgressEvent::PreviewReady { .. } => {}
        }

        if let ExecutorProgressEvent::Preview { output, value } = &event {
            self.downstream.report(ExecutorProgressEvent::PreviewReady {
                node_id: self.node_id,
                output: output.clone(),
                value: value.clone(),
                exec_signature: self.exec_signature,
                fidelity: self.fidelity.clone(),
            });
            return;
        }

        self.downstream.report(event);
    }
}

impl Runtime {
    pub fn new(
        node_manager: Arc<NodeManager>,
        capability_registry: Arc<CapabilityRegistry>,
        executor: ImageExecutor,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            node_manager,
            capability_registry,
            executor: Arc::new(executor),
            cache,
            next_execution_id: 1,
            execution_results: Arc::new(Mutex::new(HashMap::new())),
            execution_state: Arc::new(Mutex::new(ExecutionState::default())),
            completion_notifiers: Arc::new(Mutex::new(HashMap::new())),
            engine_events: EventBus::new(512),
        }
    }

    pub fn query_state(&self) -> ExecutionState {
        self.execution_state
            .lock()
            .expect("execution state lock poisoned")
            .clone()
    }

    pub fn subscribe_events(&self) -> EventSubscription<EngineEvent> {
        self.engine_events.subscribe()
    }

    pub fn events_snapshot(&self) -> Vec<crate::events::EventRecord<EngineEvent>> {
        self.engine_events.snapshot()
    }

    pub async fn evaluate(
        &self,
        graph: &graph::Graph,
        target: NodeId,
        fidelity: EvaluationFidelity,
    ) -> Result<HashMap<NodeId, HashMap<String, Value>>, Box<dyn std::error::Error + Send + Sync>> {
        let order = topo_sort(graph, target)?;
        self.evaluate_order(
            graph,
            &order,
            0,
            &CookingContext::default(),
            fidelity,
            Arc::new(NoopProgressSink),
        )
            .await
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|(node_id, outputs)| (node_id, outputs.values))
                    .collect()
            })
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
    }

    pub async fn execute_request(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        self.execute_request_with_sink_and_hook(
            graph,
            cooking_range,
            request,
            Arc::new(NoopProgressSink),
            None,
        )
        .await
    }

    pub async fn execute_request_with_sink(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<ExecutionTicket, EngineError> {
        self.execute_request_with_sink_and_hook(graph, cooking_range, request, progress_sink, None)
            .await
    }

    pub async fn execute_request_with_sink_and_hook(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
    ) -> Result<ExecutionTicket, EngineError> {
        let execution_id = self.next_execution_id;
        self.next_execution_id += 1;
        let mode = request.mode.unwrap_or_default();
        let fidelity = match &mode {
            ExecutionMode::OneShot { fidelity } => fidelity.clone(),
            ExecutionMode::Continuous { .. } => EvaluationFidelity::Full,
        };
        let completion = Arc::new((Mutex::new(None), Condvar::new()));
        self.completion_notifiers
            .lock()
            .expect("completion lock poisoned")
            .insert(execution_id, Arc::clone(&completion));

        *self
            .execution_state
            .lock()
            .expect("execution state lock poisoned") = ExecutionState {
            status: ExecutionStatus::Running,
            current: Some(RunningExecution {
                execution_id,
                target: request.target.clone(),
                mode: mode.clone(),
                fidelity: fidelity.clone(),
            }),
        };
        let _ = self.engine_events.publish(EngineEvent::ExecutionStarted {
            execution_id,
            target: request.target.clone(),
            mode: mode.clone(),
            fidelity: fidelity.clone(),
        });

        let runtime = self.clone();
        let graph = graph.clone();
        let cooking_range = cooking_range.clone();
        let request_for_task = ExecutionRequest {
            target: request.target.clone(),
            mode: Some(mode.clone()),
        };
        std::thread::spawn(move || {
            let task_result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime worker tokio runtime")
                .block_on(async {
                    runtime
                        .run_request_in_background(
                            execution_id,
                            graph,
                            cooking_range,
                            request_for_task.clone(),
                            Arc::clone(&progress_sink),
                        )
                        .await
                });
            runtime.finish_background_request(
                execution_id,
                request_for_task.target,
                mode,
                fidelity,
                task_result,
                completion,
                completion_hook,
            );
        });

        Ok(ExecutionTicket {
            execution_id,
            target: request.target,
        })
    }

    pub fn query_execution_outputs(
        &self,
        execution_id: crate::execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        Ok(self.query_execution_result(execution_id, node_id)?.values)
    }

    pub fn query_execution_result(
        &self,
        execution_id: crate::execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<ExecutionOutputs, EngineError> {
        self.execution_results
            .lock()
            .expect("execution results lock poisoned")
            .get(&execution_id)
            .and_then(|outputs| outputs.get(&node_id))
            .cloned()
            .ok_or(EngineError::ResultNotFound {
                execution_id,
                node_id,
            })
    }

    pub fn await_execution(
        &self,
        execution_id: crate::execution::ExecutionId,
    ) -> Result<ExecutionTerminalStatus, EngineError> {
        let completion = self
            .completion_notifiers
            .lock()
            .expect("completion lock poisoned")
            .get(&execution_id)
            .cloned()
            .ok_or(EngineError::Execution {
                message: format!("execution {execution_id} not found"),
            })?;

        let (status_lock, condvar) = &*completion;
        let mut status = status_lock.lock().expect("execution completion lock poisoned");
        while status.is_none() {
            status = condvar
                .wait(status)
                .expect("execution completion wait poisoned");
        }
        Ok(status.expect("completion status set before notify"))
    }

    async fn run_request_in_background(
        &self,
        execution_id: crate::execution::ExecutionId,
        graph: graph::Graph,
        cooking_range: CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<HashMap<NodeId, ExecutionOutputs>, EngineError> {
        let mode = request.mode.clone().unwrap_or_default();
        let order = resolve_subgraph(&graph, request.target.clone())
            .map_err(|error| EngineError::Execution {
                message: error.to_string(),
            })?
            .order;
        let lifecycle_bindings = self.build_lifecycle_bindings(&graph, &order)?;
        let effective_range = if cooking_range.axes.is_empty() {
            self.infer_cooking_range(&graph, &order)?
        } else {
            cooking_range
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
                        .evaluate_order(
                            &graph,
                            &order,
                            execution_id,
                            &context,
                            fidelity.clone(),
                            Arc::clone(&progress_sink),
                        )
                        .await?;
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

        run_result
    }

    fn finish_background_request(
        &self,
        execution_id: crate::execution::ExecutionId,
        target: ExecuteTarget,
        mode: ExecutionMode,
        fidelity: EvaluationFidelity,
        result: Result<HashMap<NodeId, ExecutionOutputs>, EngineError>,
        completion: ExecutionCompletion,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
    ) {
        let terminal_status = match result {
            Ok(outputs) => {
                self.execution_results
                    .lock()
                    .expect("execution results lock poisoned")
                    .insert(execution_id, outputs);
                ExecutionTerminalStatus::Finished
            }
            Err(error) => {
                let _ = self.engine_events.publish(EngineEvent::ExecutionCancelled {
                    execution_id,
                    reason: error.to_string(),
                });
                ExecutionTerminalStatus::Error
            }
        };

        *self
            .execution_state
            .lock()
            .expect("execution state lock poisoned") = ExecutionState::default();
        let _ = self.engine_events.publish(EngineEvent::ExecutionFinished {
            execution_id,
            target,
            mode,
            fidelity,
            status: terminal_status,
        });
        if let Some(hook) = completion_hook {
            hook(terminal_status);
        }

        let (status_lock, condvar) = &*completion;
        *status_lock
            .lock()
            .expect("execution completion lock poisoned") = Some(terminal_status);
        condvar.notify_all();
    }

    pub fn query_signatures(
        &self,
        graph: &graph::Graph,
        target: &ExecuteTarget,
        cooking_range: &CookingContextRange,
    ) -> Result<HashMap<NodeId, cache::model::ExecSignature>, EngineError> {
        let order = resolve_subgraph(graph, target.clone())
            .map_err(|error| EngineError::Execution {
                message: error.to_string(),
            })?
            .order;
        let effective_range = if cooking_range.axes.is_empty() {
            self.infer_cooking_range(graph, &order)?
        } else {
            cooking_range.clone()
        };

        let mut signatures = HashMap::new();
        for context in effective_range.enumerate() {
            signatures = self.compute_signatures_for_order(graph, &order, &context)?;
        }

        Ok(signatures)
    }

    pub fn output_names_for_node(
        &self,
        graph: &graph::Graph,
        node_id: NodeId,
    ) -> Result<Vec<String>, EngineError> {
        let node = graph.nodes.get(&node_id).ok_or_else(|| EngineError::Graph {
            message: format!("Node {:?} not found", node_id),
        })?;
        let def = self
            .node_manager
            .get_node_def(&node.type_id)
            .ok_or_else(|| EngineError::Graph {
                message: format!("Node type '{}' not registered", node.type_id),
            })?;
        Ok(def.outputs.iter().map(|output| output.name.clone()).collect())
    }

    async fn evaluate_order(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
        run_id: crate::execution::RunId,
        cooking_context: &CookingContext,
        fidelity: EvaluationFidelity,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<HashMap<NodeId, ExecutionOutputs>, EngineError> {
        let ctx = self.executor.context();
        let generation = self.cache.current_generation();
        let mut results: HashMap<NodeId, ExecutionOutputs> = HashMap::new();
        let mut signatures: HashMap<NodeId, cache::model::ExecSignature> = HashMap::new();

        for node_id in order {
            let _ = self.engine_events.publish(EngineEvent::NodeStarted {
                execution_id: run_id,
                node_id: *node_id,
            });
            let node = graph
                .nodes
                .get(node_id)
                .ok_or_else(|| EngineError::Graph {
                    message: format!("Node {:?} not found in graph", node_id),
                })?;
            let def = self
                .node_manager
                .get_node_def(&node.type_id)
                .ok_or_else(|| EngineError::Graph {
                    message: format!("Node type '{}' not registered", node.type_id),
                })?;

            let mut upstream_inputs: HashMap<String, Value> = HashMap::new();
            let mut upstream_signatures = Vec::new();
            for conn in &graph.connections {
                if conn.to.node == *node_id {
                    if let Some(upstream_outputs) = results.get(&conn.from.node) {
                        if let Some(val) = upstream_outputs.values.get(&conn.from.interface) {
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
                .map_err(|error| EngineError::Schema {
                    message: error.to_string(),
                })?;
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
                    if matches!(fidelity, EvaluationFidelity::Preview { .. }) {
                        for (output_pin, value) in &cached_outputs {
                            progress_sink.report(ExecutorProgressEvent::PreviewReady {
                                node_id: *node_id,
                                output: output_pin.clone(),
                                value: value.clone(),
                                exec_signature,
                                fidelity: fidelity.clone(),
                            });
                        }
                    }
                    results.insert(*node_id, ExecutionOutputs::full(cached_outputs));
                    signatures.insert(*node_id, exec_signature);
                    let _ = self.engine_events.publish(EngineEvent::NodeFinished {
                        execution_id: run_id,
                        node_id: *node_id,
                        status: NodeExecutionStatus::Finished,
                    });
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
                                exec_context: ctx,
                                inputs,
                                exec_signature,
                                generation,
                                cooking_context: cooking_context.clone(),
                                fidelity: fidelity.clone(),
                                timeout_ms: def.execution.timeout_ms,
                                cancel_token: Arc::new(NoopCancelToken),
                                progress_sink: Arc::new(NodeProgressSink {
                                    engine_events: self.engine_events.clone(),
                                    execution_id: run_id,
                                    node_id: *node_id,
                                    exec_signature,
                                    fidelity: fidelity.clone(),
                                    downstream: Arc::clone(&progress_sink),
                                }),
                            },
                        )
                        .await
                        .map_err(|error| {
                            let message = error.to_string();
                            let _ = self.engine_events.publish(EngineEvent::NodeFailed {
                                execution_id: run_id,
                                node_id: *node_id,
                                error: message.clone(),
                            });
                            EngineError::Execution { message }
                        })?
                }
                None => {
                    let _ = self.engine_events.publish(EngineEvent::NodeFailed {
                        execution_id: run_id,
                        node_id: *node_id,
                        error: def.requires.join(" + "),
                    });
                    return Err(EngineError::CapabilityUnavailable {
                        cap_id: def.requires.join(" + "),
                    });
                }
            };

            if matches!(fidelity, EvaluationFidelity::Preview { .. }) {
                for (output_pin, value) in &outputs.values {
                    progress_sink.report(ExecutorProgressEvent::PreviewReady {
                        node_id: *node_id,
                        output: output_pin.clone(),
                        value: value.clone(),
                        exec_signature,
                        fidelity: fidelity.clone(),
                    });
                }
            }

            if matches!(outputs.fidelity_achieved, EvaluationFidelity::Full) {
                for (output_pin, value) in &outputs.values {
                    let _ = self.cache.put_result(
                        *node_id,
                        output_pin,
                        exec_signature,
                        generation,
                        value.clone(),
                    );
                }
            }

            results.insert(*node_id, outputs);
            signatures.insert(*node_id, exec_signature);
            let _ = self.engine_events.publish(EngineEvent::NodeFinished {
                execution_id: run_id,
                node_id: *node_id,
                status: NodeExecutionStatus::Finished,
            });
        }

        Ok(results)
    }

    fn compute_signatures_for_order(
        &self,
        graph: &graph::Graph,
        order: &[NodeId],
        cooking_context: &CookingContext,
    ) -> Result<HashMap<NodeId, cache::model::ExecSignature>, EngineError> {
        let mut signatures = HashMap::new();

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

            let mut upstream_signatures = Vec::new();
            for conn in &graph.connections {
                if conn.to.node == *node_id {
                    if let Some(signature) = signatures.get(&conn.from.node) {
                        upstream_signatures.push(*signature);
                    }
                }
            }

            let effective_params = self
                .node_manager
                .resolve_effective_params(&node.type_id, &node.params)
                .map_err(|error| EngineError::Schema {
                    message: error.to_string(),
                })?;
            let exec_signature = compute_exec_signature(
                self.node_manager.as_ref(),
                def,
                node,
                &effective_params,
                &upstream_signatures,
                cooking_context,
            );
            signatures.insert(*node_id, exec_signature);
        }

        Ok(signatures)
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
