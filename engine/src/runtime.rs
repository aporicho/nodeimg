use crate::cache;
use crate::cache::manager::CacheManager;
use crate::capability::{CapabilityRegistry, LocalityHint};
use crate::execution::{
    CookingContext, CookingContextRange, EvaluationFidelity, ExecutionMode, ExecutionTerminalStatus,
    ExecutorProgressEvent, NodeExecutionRequest, NoopProgressSink, PlanLifecycleContext,
    PlanLifecycleNode, ProgressSink, SharedCancelToken, TickSource,
};
use crate::events::{
    CancellationReason, CancellationSubject, EngineEvent, EventBus, EventSubscription,
    ExecutionState, ExecutionStatus, NodeExecutionStatus, PendingExecution, PendingExecutionId,
    QueueReason, ReplacementReason, RunningExecution,
};
use crate::executors;
use crate::executors::image::ImageExecutor;
use crate::execution::ExecutionOutputs;
use crate::facade::{EngineError, ExecutionRequest, ExecutionRequestResult, ExecutionTicket};
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
    next_pending_id: PendingExecutionId,
    execution_results:
        Arc<Mutex<HashMap<crate::execution::ExecutionId, HashMap<NodeId, ExecutionOutputs>>>>,
    completion_notifiers: Arc<Mutex<HashMap<crate::execution::ExecutionId, ExecutionCompletion>>>,
    single_flight: Arc<Mutex<SingleFlightGuard>>,
    engine_events: EventBus<EngineEvent>,
}

type ExecutionCompletion = Arc<(Mutex<Option<ExecutionTerminalStatus>>, Condvar)>;

#[derive(Clone)]
struct CurrentRun {
    execution_id: crate::execution::ExecutionId,
    request: ExecutionRequest,
    mode: ExecutionMode,
    fidelity: EvaluationFidelity,
    cancel_token: SharedCancelToken,
    completion: ExecutionCompletion,
    status: ExecutionStatus,
    cancellation_reason: Option<CancellationReason>,
    from_pending_id: Option<PendingExecutionId>,
}

#[derive(Clone)]
struct PendingFull {
    pending_id: PendingExecutionId,
    request: ExecutionRequest,
}

#[derive(Default)]
struct SingleFlightGuard {
    current: Option<CurrentRun>,
    pending_full: Option<PendingFull>,
}

enum StartDecision {
    StartNow {
        from_pending_id: Option<PendingExecutionId>,
    },
    WaitForCurrent,
    Queued {
        pending_id: PendingExecutionId,
        reason: QueueReason,
    },
}

enum RunFailure {
    Cancelled,
    Error(EngineError),
}

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
            next_pending_id: 1,
            execution_results: Arc::new(Mutex::new(HashMap::new())),
            completion_notifiers: Arc::new(Mutex::new(HashMap::new())),
            single_flight: Arc::new(Mutex::new(SingleFlightGuard::default())),
            engine_events: EventBus::new(512),
        }
    }

    pub fn query_state(&self) -> ExecutionState {
        let guard = self
            .single_flight
            .lock()
            .expect("single-flight lock poisoned");
        let current = guard.current.as_ref().map(|current| RunningExecution {
            execution_id: current.execution_id,
            target: current.request.target.clone(),
            mode: current.mode.clone(),
            fidelity: current.fidelity.clone(),
            from_pending_id: current.from_pending_id,
        });
        let pending_full = guard.pending_full.as_ref().map(|pending| {
            let mode = pending.request.mode.clone().unwrap_or_default();
            PendingExecution {
                pending_id: pending.pending_id,
                target: pending.request.target.clone(),
                fidelity: fidelity_from_mode(&mode),
                mode,
            }
        });

        ExecutionState {
            status: guard
                .current
                .as_ref()
                .map(|current| current.status)
                .unwrap_or(ExecutionStatus::Idle),
            current,
            pending_full,
        }
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
            SharedCancelToken::new(),
            Arc::new(NoopProgressSink),
        )
            .await
            .map(|outputs| {
                outputs
                    .into_iter()
                    .map(|(node_id, outputs)| (node_id, outputs.values))
                    .collect()
            })
            .map_err(|error| match error {
                RunFailure::Cancelled => {
                    Box::new(EngineError::Execution {
                        message: "execution cancelled".into(),
                    }) as Box<dyn std::error::Error + Send + Sync>
                }
                RunFailure::Error(error) => Box::new(error) as Box<dyn std::error::Error + Send + Sync>,
            })
    }

    pub fn request_execution(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
    ) -> Result<ExecutionRequestResult, EngineError> {
        self.request_execution_internal(
            graph,
            cooking_range,
            request,
            Arc::new(NoopProgressSink),
            None,
            None,
        )
    }

    pub fn request_preview_execution(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<ExecutionTicket, EngineError> {
        match self.request_execution_internal(
            graph,
            cooking_range,
            request,
            progress_sink,
            None,
            None,
        )? {
            ExecutionRequestResult::Started(ticket) => Ok(ticket),
            ExecutionRequestResult::Queued { .. } => Err(EngineError::Execution {
                message: "preview execution unexpectedly queued".into(),
            }),
        }
    }

    pub fn request_execution_with_sink_and_hook(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
        pending_origin: Option<PendingExecutionId>,
    ) -> Result<ExecutionRequestResult, EngineError> {
        self.request_execution_internal(
            graph,
            cooking_range,
            request,
            progress_sink,
            completion_hook,
            pending_origin,
        )
    }

    pub fn request_execution_from_pending(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        pending_id: PendingExecutionId,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
    ) -> Result<ExecutionRequestResult, EngineError> {
        self.request_execution_internal(
            graph,
            cooking_range,
            request,
            Arc::new(NoopProgressSink),
            completion_hook,
            Some(pending_id),
        )
    }

    fn request_execution_internal(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
        pending_origin: Option<PendingExecutionId>,
    ) -> Result<ExecutionRequestResult, EngineError> {
        let mode = request.mode.clone().unwrap_or_default();
        validate_mode(&mode)?;

        loop {
            match self.try_start_run(&request, &mode)? {
                StartDecision::StartNow { from_pending_id } => {
                    return Ok(ExecutionRequestResult::Started(self.spawn_run(
                        graph,
                        cooking_range,
                        request.clone(),
                        Arc::clone(&progress_sink),
                        completion_hook.clone(),
                        from_pending_id.or(pending_origin),
                    )?));
                }
                StartDecision::Queued { pending_id, reason } => {
                    let _ = self.engine_events.publish(EngineEvent::ExecutionQueued {
                        pending_id,
                        target: request.target.clone(),
                        mode: mode.clone(),
                        fidelity: fidelity_from_mode(&mode),
                        reason,
                    });
                    return Ok(ExecutionRequestResult::Queued { pending_id, reason });
                }
                StartDecision::WaitForCurrent => {
                    let completion = self.current_completion()?;
                    let (lock, condvar) = &*completion;
                    let mut status = lock.lock().expect("execution completion lock poisoned");
                    while status.is_none() {
                        status = condvar
                            .wait(status)
                            .expect("execution completion wait poisoned");
                    }
                }
            }
        }
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

    pub fn cancel_execution(
        &self,
        execution_id: crate::execution::ExecutionId,
    ) -> Result<(), EngineError> {
        let mut guard = self
            .single_flight
            .lock()
            .expect("single-flight lock poisoned");
        let Some(current) = guard.current.as_mut() else {
            return Err(EngineError::Execution {
                message: format!("execution {execution_id} is not running"),
            });
        };
        if current.execution_id != execution_id {
            return Err(EngineError::Execution {
                message: format!("execution {execution_id} is not the active run"),
            });
        }

        current.status = ExecutionStatus::Cancelling;
        current.cancellation_reason = Some(CancellationReason::UserRequested);
        current.cancel_token.cancel();
        Ok(())
    }

    pub fn take_pending_full_for_resubmit(&self) -> Option<(PendingExecutionId, ExecutionRequest)> {
        self.single_flight
            .lock()
            .expect("single-flight lock poisoned")
            .pending_full
            .take()
            .map(|pending| (pending.pending_id, pending.request))
    }

    fn current_completion(&self) -> Result<ExecutionCompletion, EngineError> {
        self.single_flight
            .lock()
            .expect("single-flight lock poisoned")
            .current
            .as_ref()
            .map(|current| Arc::clone(&current.completion))
            .ok_or(EngineError::Execution {
                message: "no active execution to wait for".into(),
            })
    }

    fn try_start_run(
        &mut self,
        request: &ExecutionRequest,
        mode: &ExecutionMode,
    ) -> Result<StartDecision, EngineError> {
        let fidelity = fidelity_from_mode(mode);
        let next_pending_id = self.next_pending_id;
        let mut guard = self
            .single_flight
            .lock()
            .expect("single-flight lock poisoned");

        let Some(current_snapshot) = guard.current.clone() else {
            return Ok(StartDecision::StartNow {
                from_pending_id: None,
            });
        };

        match (&current_snapshot.fidelity, &fidelity) {
            (EvaluationFidelity::Full, EvaluationFidelity::Preview { .. }) => {
                let preempted_request = current_snapshot.request.clone();
                let pending = PendingFull {
                    pending_id: next_pending_id,
                    request: preempted_request,
                };
                if let Some(replaced) = guard.pending_full.replace(pending.clone()) {
                    let _ = self.engine_events.publish(EngineEvent::ExecutionReplaced {
                        pending_id: replaced.pending_id,
                        reason: ReplacementReason::ReplacedByNewerFull,
                    });
                }
                self.next_pending_id += 1;
                let pending_mode = pending.request.mode.clone().unwrap_or_default();
                let _ = self.engine_events.publish(EngineEvent::ExecutionQueued {
                    pending_id: pending.pending_id,
                    target: pending.request.target.clone(),
                    mode: pending_mode.clone(),
                    fidelity: fidelity_from_mode(&pending_mode),
                    reason: QueueReason::BehindPreview,
                });
                let current = guard.current.as_mut().expect("current run present");
                current.status = ExecutionStatus::Cancelling;
                current.cancellation_reason = Some(CancellationReason::ReplacedByPreview);
                current.cancel_token.cancel();
                Ok(StartDecision::WaitForCurrent)
            }
            (EvaluationFidelity::Preview { .. }, EvaluationFidelity::Preview { .. }) => {
                let current = guard.current.as_mut().expect("current run present");
                current.status = ExecutionStatus::Cancelling;
                current.cancellation_reason = Some(CancellationReason::ReplacedByNewerPreview);
                current.cancel_token.cancel();
                Ok(StartDecision::WaitForCurrent)
            }
            (EvaluationFidelity::Preview { .. }, EvaluationFidelity::Full) => {
                if let Some(replaced) = guard.pending_full.replace(PendingFull {
                    pending_id: next_pending_id,
                    request: request.clone(),
                }) {
                    let _ = self.engine_events.publish(EngineEvent::ExecutionReplaced {
                        pending_id: replaced.pending_id,
                        reason: ReplacementReason::ReplacedByNewerFull,
                    });
                }
                self.next_pending_id += 1;
                let pending_id = guard
                    .pending_full
                    .as_ref()
                    .expect("pending full just inserted")
                    .pending_id;
                Ok(StartDecision::Queued {
                    pending_id,
                    reason: QueueReason::BehindPreview,
                })
            }
            (EvaluationFidelity::Full, EvaluationFidelity::Full) => Err(EngineError::RunInFlight),
        }
    }

    fn spawn_run(
        &mut self,
        graph: &graph::Graph,
        cooking_range: &CookingContextRange,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
        from_pending_id: Option<PendingExecutionId>,
    ) -> Result<ExecutionTicket, EngineError> {
        let execution_id = self.next_execution_id;
        self.next_execution_id += 1;
        let mode = request.mode.clone().unwrap_or_default();
        let fidelity = fidelity_from_mode(&mode);
        let ticket_target = request.target.clone();
        let completion = Arc::new((Mutex::new(None), Condvar::new()));
        self.completion_notifiers
            .lock()
            .expect("completion lock poisoned")
            .insert(execution_id, Arc::clone(&completion));
        let cancel_token = SharedCancelToken::new();
        {
            let mut guard = self
                .single_flight
                .lock()
                .expect("single-flight lock poisoned");
            guard.current = Some(CurrentRun {
                execution_id,
                request: request.clone(),
                mode: mode.clone(),
                fidelity: fidelity.clone(),
                cancel_token: cancel_token.clone(),
                completion: Arc::clone(&completion),
                status: ExecutionStatus::Running,
                cancellation_reason: None,
                from_pending_id,
            });
        }
        let _ = self.engine_events.publish(EngineEvent::ExecutionStarted {
            execution_id,
            target: request.target.clone(),
            mode: mode.clone(),
            fidelity: fidelity.clone(),
            from_pending_id,
        });

        let runtime = self.clone();
        let graph = graph.clone();
        let cooking_range = cooking_range.clone();
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
                            request.clone(),
                            cancel_token,
                            Arc::clone(&progress_sink),
                        )
                        .await
                });
            runtime.finish_background_request(
                execution_id,
                task_result,
                completion,
                completion_hook,
            );
        });

        Ok(ExecutionTicket {
            execution_id,
            target: ticket_target,
        })
    }

    async fn run_request_in_background(
        &self,
        execution_id: crate::execution::ExecutionId,
        graph: graph::Graph,
        cooking_range: CookingContextRange,
        request: ExecutionRequest,
        cancel_token: SharedCancelToken,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<HashMap<NodeId, ExecutionOutputs>, RunFailure> {
        let mode = request.mode.clone().unwrap_or_default();
        let order = resolve_subgraph(&graph, request.target.clone())
            .map_err(|error| RunFailure::Error(EngineError::Execution {
                message: error.to_string(),
            }))?
            .order;
        let lifecycle_bindings = self
            .build_lifecycle_bindings(&graph, &order)
            .map_err(RunFailure::Error)?;
        let effective_range = if cooking_range.axes.is_empty() {
            self.infer_cooking_range(&graph, &order)
                .map_err(RunFailure::Error)?
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
                .map_err(|error| RunFailure::Error(EngineError::Execution {
                    message: error.to_string(),
                }))?;
        }

        let run_result = match mode.clone() {
            ExecutionMode::OneShot { fidelity } => {
                let mut aggregated = HashMap::new();
                for context in effective_range.enumerate() {
                    if cancel_token.is_cancelled() {
                        break;
                    }
                    let frame_outputs = self
                        .evaluate_order(
                            &graph,
                            &order,
                            execution_id,
                            &context,
                            fidelity.clone(),
                            cancel_token.clone(),
                            Arc::clone(&progress_sink),
                        )
                        .await?;
                    for (node_id, outputs) in frame_outputs {
                        aggregated.insert(node_id, outputs);
                    }
                }
                if cancel_token.is_cancelled() {
                    Err(RunFailure::Cancelled)
                } else {
                    Ok(aggregated)
                }
            }
            ExecutionMode::Continuous { tick_source, .. } => Err(RunFailure::Error(EngineError::Execution {
                message: match tick_source {
                    TickSource::OsClock => {
                        "Continuous mode is not implemented in the current runtime".into()
                    }
                    TickSource::External(name) | TickSource::DataPull(name) => {
                        format!("continuous tick source '{name}' is not implemented")
                    }
                },
            })),
        };

        let terminal_status = if run_result.is_ok() {
            ExecutionTerminalStatus::Finished
        } else if matches!(run_result, Err(RunFailure::Cancelled)) {
            ExecutionTerminalStatus::Cancelled
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
        result: Result<HashMap<NodeId, ExecutionOutputs>, RunFailure>,
        completion: ExecutionCompletion,
        completion_hook: Option<Arc<dyn Fn(ExecutionTerminalStatus) + Send + Sync>>,
    ) {
        let (target, mode, fidelity, terminal_status, cancellation_reason) = {
            let mut guard = self
                .single_flight
                .lock()
                .expect("single-flight lock poisoned");
            let matches_current = guard
                .current
                .as_ref()
                .map(|current| current.execution_id == execution_id)
                .unwrap_or(false);
            if !matches_current {
                return;
            }
            let current = guard.current.take().expect("current run exists when matching");
            let terminal_status = match &result {
                Ok(_) => ExecutionTerminalStatus::Finished,
                Err(RunFailure::Cancelled) => ExecutionTerminalStatus::Cancelled,
                Err(RunFailure::Error(_)) => ExecutionTerminalStatus::Error,
            };
            (
                current.request.target,
                current.mode,
                current.fidelity,
                terminal_status,
                current.cancellation_reason,
            )
        };

        match result {
            Ok(outputs) => {
                self.execution_results
                    .lock()
                    .expect("execution results lock poisoned")
                    .insert(execution_id, outputs);
            }
            Err(RunFailure::Cancelled) => {
                let _ = self.engine_events.publish(EngineEvent::ExecutionCancelled {
                    subject: CancellationSubject::Execution(execution_id),
                    reason: cancellation_reason.unwrap_or(CancellationReason::UserRequested),
                });
            }
            Err(RunFailure::Error(_)) => {}
        }

        let _ = self.engine_events.publish(EngineEvent::ExecutionFinished {
            execution_id,
            target,
            mode,
            fidelity,
            status: terminal_status,
        });
        let (status_lock, condvar) = &*completion;
        *status_lock
            .lock()
            .expect("execution completion lock poisoned") = Some(terminal_status);
        condvar.notify_all();

        if let Some(hook) = completion_hook {
            hook(terminal_status);
        }
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
        cancel_token: SharedCancelToken,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<HashMap<NodeId, ExecutionOutputs>, RunFailure> {
        let ctx = self.executor.context();
        let generation = self.cache.current_generation();
        let mut results: HashMap<NodeId, ExecutionOutputs> = HashMap::new();
        let mut signatures: HashMap<NodeId, cache::model::ExecSignature> = HashMap::new();

        for node_id in order {
            if cancel_token.is_cancelled() {
                return Err(RunFailure::Cancelled);
            }
            let _ = self.engine_events.publish(EngineEvent::NodeStarted {
                execution_id: run_id,
                node_id: *node_id,
            });
            let node = graph
                .nodes
                .get(node_id)
                .ok_or_else(|| RunFailure::Error(EngineError::Graph {
                    message: format!("Node {:?} not found in graph", node_id),
                }))?;
            let def = self
                .node_manager
                .get_node_def(&node.type_id)
                .ok_or_else(|| RunFailure::Error(EngineError::Graph {
                    message: format!("Node type '{}' not registered", node.type_id),
                }))?;

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
                .map_err(|error| RunFailure::Error(EngineError::Schema {
                    message: error.to_string(),
                }))?;
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
                                cancel_token: Arc::new(cancel_token.clone()),
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
                        .map_err(|error| match error {
                            crate::execution::ExecutorError::Cancelled => {
                                let _ = self.engine_events.publish(EngineEvent::NodeFinished {
                                    execution_id: run_id,
                                    node_id: *node_id,
                                    status: NodeExecutionStatus::Cancelled,
                                });
                                RunFailure::Cancelled
                            }
                            other => {
                                let message = other.to_string();
                                let _ = self.engine_events.publish(EngineEvent::NodeFailed {
                                    execution_id: run_id,
                                    node_id: *node_id,
                                    error: message.clone(),
                                });
                                RunFailure::Error(EngineError::Execution { message })
                            }
                        })?
                }
                None => {
                    let _ = self.engine_events.publish(EngineEvent::NodeFailed {
                        execution_id: run_id,
                        node_id: *node_id,
                        error: def.requires.join(" + "),
                    });
                    return Err(RunFailure::Error(EngineError::CapabilityUnavailable {
                        cap_id: def.requires.join(" + "),
                    }));
                }
            };

            if cancel_token.is_cancelled() {
                let _ = self.engine_events.publish(EngineEvent::NodeFinished {
                    execution_id: run_id,
                    node_id: *node_id,
                    status: NodeExecutionStatus::Cancelled,
                });
                return Err(RunFailure::Cancelled);
            }

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

fn fidelity_from_mode(mode: &ExecutionMode) -> EvaluationFidelity {
    match mode {
        ExecutionMode::OneShot { fidelity } => fidelity.clone(),
        ExecutionMode::Continuous { .. } => EvaluationFidelity::Full,
    }
}

fn validate_mode(mode: &ExecutionMode) -> Result<(), EngineError> {
    match mode {
        ExecutionMode::OneShot { .. } => Ok(()),
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
