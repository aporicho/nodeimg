use std::sync::Arc;

use crate::artifact::manager::ArtifactManager;
use crate::cache::manager::CacheManager;
use crate::cache::model::generation_id::GenerationId;
use crate::cache::result_store::WriteResult;
use crate::executors::image::ImageExecutor;
use crate::graph::model::events::GraphChangedEvent;
use crate::graph::query::downstream::downstream;
use crate::graph::Graph;
use crate::node_manager::NodeManager;
use crate::scheduler::cancel::running_task::RunningTask;
use crate::scheduler::model::{
    DirtyReason, DirtyState, ExecutionEvent, ExecutionMode, ExecutionPlan, ExecutionRequest,
    ExecutorType, FinalStatus, RunId, RuntimeState, SchedulerError, SchedulerStateSummary,
};
use crate::scheduler::planner::dirty_propagation::apply_graph_event;
use crate::scheduler::planner::plan_builder::build_plan;
use crate::scheduler::planner::snapshot_rebuild::rebuild_dirty_state_in_place;
use crate::scheduler::runtime::artifact_sink::{persist_if_needed, ArtifactSinkRequest};
use crate::scheduler::runtime::cache_restore::{restore_from_artifact, CacheRestoreRequest};
use crate::scheduler::runtime::executor_registry::{ExecutorEntry, ExecutorRegistry};
use crate::scheduler::runtime::input_resolver::{resolve_node_inputs, ExecutionResults};
use crate::scheduler::runtime::node_runner::{run_planned_node, NodeRunError};
use types::NodeId;

#[derive(Clone, Debug)]
pub struct ArtifactSelection {
    pub node_id: NodeId,
    pub output_key: String,
    pub artifact_id: String,
}

#[derive(Clone, Debug)]
pub struct GraphReplaceSnapshotMeta {
    pub graph_snapshot: Arc<Graph>,
    pub graph_version: Option<u64>,
}

pub struct SchedulerManager {
    mode: ExecutionMode,
    dirty_state: DirtyState,
    runtime_state: RuntimeState,
    current_plan: Option<ExecutionPlan>,
    running_task: Option<RunningTask>,
    last_run_id: Option<RunId>,
    next_run_id: u64,
    node_manager: Option<Arc<NodeManager>>,
    image_executor: Option<Arc<ImageExecutor>>,
    cache_manager: Option<Arc<CacheManager>>,
    artifact_manager: Option<ArtifactManager>,
    executor_registry: ExecutorRegistry,
    events: Vec<ExecutionEvent>,
}

impl SchedulerManager {
    pub fn new() -> Self {
        Self {
            mode: ExecutionMode::Auto,
            dirty_state: DirtyState::new(),
            runtime_state: RuntimeState::Idle,
            current_plan: None,
            running_task: None,
            last_run_id: None,
            next_run_id: 1,
            node_manager: None,
            image_executor: None,
            cache_manager: None,
            artifact_manager: None,
            executor_registry: ExecutorRegistry::new(),
            events: Vec::new(),
        }
    }

    pub fn new_with_runtime(
        node_manager: Arc<NodeManager>,
        image_executor: Arc<ImageExecutor>,
        cache_manager: Arc<CacheManager>,
    ) -> Self {
        let mut manager = Self::new();
        manager.node_manager = Some(node_manager);
        manager.image_executor = Some(image_executor);
        manager.cache_manager = Some(cache_manager);
        manager.register_default_executors();
        manager
    }

    pub fn set_artifact_manager(&mut self, artifact_manager: ArtifactManager) {
        self.artifact_manager = Some(artifact_manager);
    }

    pub fn executor_registry(&self) -> &ExecutorRegistry {
        &self.executor_registry
    }

    pub fn executor_registry_mut(&mut self) -> &mut ExecutorRegistry {
        &mut self.executor_registry
    }

    pub fn mode(&self) -> ExecutionMode {
        self.mode
    }

    pub fn events(&self) -> &[ExecutionEvent] {
        &self.events
    }

    pub fn drain_events(&mut self) -> Vec<ExecutionEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn dirty_state(&self) -> &DirtyState {
        &self.dirty_state
    }

    pub fn current_plan(&self) -> Option<&ExecutionPlan> {
        self.current_plan.as_ref()
    }

    pub fn running_task(&self) -> Option<&RunningTask> {
        self.running_task.as_ref()
    }

    pub fn take_current_plan(&mut self) -> Option<ExecutionPlan> {
        self.current_plan.take()
    }

    pub fn set_mode(&mut self, mode: ExecutionMode) {
        self.mode = mode;
    }

    pub fn notify_graph_changed<F>(
        &mut self,
        graph: Arc<Graph>,
        event: &GraphChangedEvent,
        generation: GenerationId,
        classify_executor: F,
    ) -> Result<Option<RunId>, SchedulerError>
    where
        F: Fn(NodeId) -> ExecutorType,
    {
        self.cancel_conflicting_run();
        apply_graph_event(&graph, &mut self.dirty_state, event);

        if self.mode == ExecutionMode::Auto {
            return self.plan_stage_and_maybe_execute(
                graph,
                ExecutionRequest::new(crate::scheduler::model::ExecuteTarget::Graph),
                generation,
                classify_executor,
            );
        }

        Ok(None)
    }

    pub fn notify_graph_replaced<F>(
        &mut self,
        snapshot_meta: GraphReplaceSnapshotMeta,
        generation: GenerationId,
        classify_executor: F,
    ) -> Result<Option<RunId>, SchedulerError>
    where
        F: Fn(NodeId) -> ExecutorType,
    {
        self.cancel_conflicting_run();
        rebuild_dirty_state_in_place(
            &snapshot_meta.graph_snapshot,
            &mut self.dirty_state,
            DirtyReason::StructureChanged,
        );

        if self.mode == ExecutionMode::Auto {
            return self.plan_stage_and_maybe_execute(
                snapshot_meta.graph_snapshot,
                ExecutionRequest::new(crate::scheduler::model::ExecuteTarget::Graph),
                generation,
                classify_executor,
            );
        }

        Ok(None)
    }

    pub fn notify_artifact_selected<F>(
        &mut self,
        graph: Arc<Graph>,
        selection: &ArtifactSelection,
        generation: GenerationId,
        classify_executor: F,
    ) -> Result<Option<RunId>, SchedulerError>
    where
        F: Fn(NodeId) -> ExecutorType,
    {
        self.cancel_conflicting_run();

        for downstream_node in downstream(&graph, selection.node_id) {
            self.dirty_state
                .mark(downstream_node, DirtyReason::ArtifactSelectionChanged);
        }

        if self.mode == ExecutionMode::Auto {
            return self.plan_stage_and_maybe_execute(
                graph,
                ExecutionRequest::new(crate::scheduler::model::ExecuteTarget::Graph),
                generation,
                classify_executor,
            );
        }

        Ok(None)
    }

    pub fn request_execution<F>(
        &mut self,
        graph: Arc<Graph>,
        request: ExecutionRequest,
        generation: GenerationId,
        classify_executor: F,
    ) -> Result<RunId, SchedulerError>
    where
        F: Fn(NodeId) -> ExecutorType,
    {
        if self.current_plan.is_some() {
            return Err(SchedulerError::RunAlreadyInProgress);
        }

        match self.plan_stage_and_maybe_execute(graph, request, generation, classify_executor)? {
            Some(run_id) => Ok(run_id),
            None => Err(SchedulerError::PlannerFailed {
                message: "no executable nodes selected".into(),
            }),
        }
    }

    pub fn cancel_execution(&mut self, run_id: RunId) -> Result<(), SchedulerError> {
        match self.current_plan.as_ref() {
            Some(plan) if plan.run_id == run_id => {
                self.push_event(ExecutionEvent::Cancelled { run_id });
                self.current_plan = None;
                self.running_task = None;
                self.runtime_state = RuntimeState::LastCompleted {
                    run_id,
                    status: FinalStatus::Cancelled,
                };
                self.last_run_id = Some(run_id);
                Ok(())
            }
            _ => Err(SchedulerError::RunNotFound { run_id }),
        }
    }

    pub fn query_state(&self) -> SchedulerStateSummary {
        SchedulerStateSummary {
            mode: self.mode,
            runtime_state: self.runtime_state,
            dirty_count: self.dirty_state.len(),
            has_pending_changes: !self.dirty_state.is_empty(),
            last_run_id: self.last_run_id,
        }
    }

    fn plan_stage_and_maybe_execute<F>(
        &mut self,
        graph: Arc<Graph>,
        request: ExecutionRequest,
        generation: GenerationId,
        classify_executor: F,
    ) -> Result<Option<RunId>, SchedulerError>
    where
        F: Fn(NodeId) -> ExecutorType,
    {
        let run_id = self.allocate_run_id();
        let maybe_plan = build_plan(
            graph,
            &self.dirty_state,
            request,
            self.mode,
            run_id,
            generation,
            classify_executor,
        )
        .map_err(|error| SchedulerError::PlannerFailed {
            message: error.to_string(),
        })?;

        let Some(plan) = maybe_plan else {
            return Ok(None);
        };

        self.runtime_state = RuntimeState::Planning { run_id };
        self.last_run_id = Some(run_id);
        self.current_plan = Some(plan.clone());
        self.running_task = Some(RunningTask::new(run_id));
        self.push_event(ExecutionEvent::Started {
            run_id,
            total_nodes: plan.layers.iter().map(|layer| layer.len()).sum(),
        });

        if self.runtime_components_available() {
            if let Err(error) = self.execute_staged_plan() {
                self.push_event(ExecutionEvent::Failed {
                    run_id,
                    error: error.clone(),
                });
                return Err(error);
            }
        }

        Ok(Some(run_id))
    }

    fn execute_staged_plan(&mut self) -> Result<(), SchedulerError> {
        let plan = self
            .current_plan
            .clone()
            .ok_or_else(|| SchedulerError::RuntimeFailed {
                message: "no staged plan to execute".into(),
            })?;

        let node_manager = Arc::clone(self.node_manager.as_ref().ok_or_else(|| {
            SchedulerError::RuntimeFailed {
                message: "node manager not configured".into(),
            }
        })?);
        let image_executor = Arc::clone(self.image_executor.as_ref().ok_or_else(|| {
            SchedulerError::RuntimeFailed {
                message: "image executor not configured".into(),
            }
        })?);
        let cache_manager = Arc::clone(self.cache_manager.as_ref().ok_or_else(|| {
            SchedulerError::RuntimeFailed {
                message: "cache manager not configured".into(),
            }
        })?);

        let running_task =
            self.running_task
                .clone()
                .ok_or_else(|| SchedulerError::RuntimeFailed {
                    message: "running task not configured".into(),
                })?;

        let run_id = plan.run_id;
        let mut results = ExecutionResults::new();
        let total_nodes = plan.layers.iter().map(|layer| layer.len()).sum::<usize>();
        let mut completed = 0usize;

        self.runtime_state = RuntimeState::Running {
            run_id,
            progress: crate::scheduler::model::ExecutionProgress {
                completed,
                total: total_nodes,
            },
        };

        for layer in &plan.layers {
            for planned_node in layer {
                if running_task.is_cancelled() {
                    self.push_event(ExecutionEvent::Cancelled { run_id });
                    self.current_plan = None;
                    self.running_task = None;
                    self.runtime_state = RuntimeState::LastCompleted {
                        run_id,
                        status: FinalStatus::Cancelled,
                    };
                    return Ok(());
                }
                running_task.check_cancelled().map_err(|error| SchedulerError::RuntimeFailed {
                    message: error.to_string(),
                })?;
                if let Err(error) = running_task.check_timeout() {
                    self.push_event(ExecutionEvent::Cancelled { run_id });
                    self.current_plan = None;
                    self.running_task = None;
                    self.runtime_state = RuntimeState::LastCompleted {
                        run_id,
                        status: FinalStatus::Cancelled,
                    };
                    return Err(SchedulerError::RuntimeFailed {
                        message: error.to_string(),
                    });
                }

                let node = plan
                    .graph_snapshot
                    .nodes
                    .get(&planned_node.node_id)
                    .ok_or_else(|| SchedulerError::RuntimeFailed {
                        message: format!(
                            "node {:?} not found in graph snapshot",
                            planned_node.node_id
                        ),
                    })?;
                let node_def = node_manager
                    .get_node_def(&node.type_id)
                    .ok_or_else(|| SchedulerError::RuntimeFailed {
                        message: format!("node type '{}' not registered", node.type_id),
                    })?;

                self.push_event(ExecutionEvent::NodeStarted {
                    run_id,
                    node_id: planned_node.node_id,
                });

                if let Some(cached_outputs) = collect_cached_outputs(
                    &cache_manager,
                    planned_node,
                    node_def,
                    plan.generation,
                ) {
                    results.insert(planned_node.node_id, cached_outputs);
                    self.dirty_state.dirty_nodes.remove(&planned_node.node_id);
                    self.dirty_state.dirty_reasons.remove(&planned_node.node_id);
                    completed += 1;
                    self.runtime_state = RuntimeState::Running {
                        run_id,
                        progress: crate::scheduler::model::ExecutionProgress {
                            completed,
                            total: total_nodes,
                        },
                    };
                    self.push_event(ExecutionEvent::NodeFinished {
                        run_id,
                        node_id: planned_node.node_id,
                    });
                    self.push_event(ExecutionEvent::Progress {
                        run_id,
                        completed,
                        total: total_nodes,
                    });
                    continue;
                }

                if let Some(artifact_manager) = self.artifact_manager.as_ref() {
                    for output in &node_def.outputs {
                        let _ = restore_from_artifact(
                            &cache_manager,
                            artifact_manager,
                            &CacheRestoreRequest {
                                run_id,
                                node_id: planned_node.node_id,
                                output_key: output.name.clone(),
                                exec_signature: planned_node.exec_signature,
                                generation: plan.generation,
                                param_signature: param_signature(planned_node),
                                input_signature: input_signature(planned_node),
                            },
                        )
                        .map_err(|error| SchedulerError::RuntimeFailed {
                            message: error.to_string(),
                        })?;
                    }

                    if let Some(restored_outputs) = collect_cached_outputs(
                        &cache_manager,
                        planned_node,
                        node_def,
                        plan.generation,
                    ) {
                        results.insert(planned_node.node_id, restored_outputs);
                        self.dirty_state.dirty_nodes.remove(&planned_node.node_id);
                        self.dirty_state.dirty_reasons.remove(&planned_node.node_id);
                        completed += 1;
                        self.runtime_state = RuntimeState::Running {
                            run_id,
                            progress: crate::scheduler::model::ExecutionProgress {
                                completed,
                                total: total_nodes,
                            },
                        };
                        self.push_event(ExecutionEvent::NodeFinished {
                            run_id,
                            node_id: planned_node.node_id,
                        });
                        self.push_event(ExecutionEvent::Progress {
                            run_id,
                            completed,
                            total: total_nodes,
                        });
                        continue;
                    }
                }

                let inputs = resolve_node_inputs(
                    &plan.graph_snapshot,
                    &node_manager,
                    &results,
                    planned_node.node_id,
                )
                .map_err(|error| SchedulerError::RuntimeFailed {
                    message: error.to_string(),
                })?;

                let outputs = pollster::block_on(run_planned_node(
                    &self.executor_registry,
                    &node_manager,
                    &image_executor,
                    &node.type_id,
                    planned_node,
                    inputs,
                ))
                .map_err(|error| {
                    self.push_event(ExecutionEvent::NodeFailed {
                        run_id,
                        node_id: planned_node.node_id,
                        error: error.to_string(),
                    });
                    map_node_run_error(error)
                })?;

                for (output_pin, value) in &outputs {
                    match cache_manager.put_result(
                        planned_node.node_id,
                        output_pin,
                        planned_node.exec_signature,
                        plan.generation,
                        value.clone(),
                    ) {
                        Ok(WriteResult::Written { .. } | WriteResult::DroppedByGeneration) => {}
                        Err(error) => {
                            return Err(SchedulerError::RuntimeFailed {
                                message: format!("cache write failed: {error:?}"),
                            });
                        }
                    }
                }

                if let Some(artifact_manager) = self.artifact_manager.as_mut() {
                    for (output_pin, value) in &outputs {
                        let _ = persist_if_needed(
                            artifact_manager,
                            ArtifactSinkRequest {
                                node_id: planned_node.node_id,
                                output_key: output_pin.clone(),
                                value: value.clone(),
                                param_signature: param_signature(planned_node),
                                input_signature: input_signature(planned_node),
                            },
                        )
                        .map_err(|error| SchedulerError::RuntimeFailed {
                            message: error.to_string(),
                        })?;
                    }
                }

                results.insert(planned_node.node_id, outputs);
                self.dirty_state.dirty_nodes.remove(&planned_node.node_id);
                self.dirty_state.dirty_reasons.remove(&planned_node.node_id);

                completed += 1;
                self.runtime_state = RuntimeState::Running {
                    run_id,
                    progress: crate::scheduler::model::ExecutionProgress {
                        completed,
                        total: total_nodes,
                    },
                };
                self.push_event(ExecutionEvent::NodeFinished {
                    run_id,
                    node_id: planned_node.node_id,
                });
                self.push_event(ExecutionEvent::Progress {
                    run_id,
                    completed,
                    total: total_nodes,
                });
            }
        }

        self.current_plan = None;
        self.running_task = None;
        self.runtime_state = RuntimeState::LastCompleted {
            run_id,
            status: FinalStatus::Finished,
        };
        self.push_event(ExecutionEvent::Finished { run_id });
        Ok(())
    }

    fn allocate_run_id(&mut self) -> RunId {
        let run_id = RunId(self.next_run_id);
        self.next_run_id += 1;
        run_id
    }

    fn cancel_conflicting_run(&mut self) {
        if let Some(task) = &self.running_task {
            task.cancel();
        }
        if let Some(plan) = self.current_plan.take() {
            self.running_task = None;
            self.runtime_state = RuntimeState::LastCompleted {
                run_id: plan.run_id,
                status: FinalStatus::Cancelled,
            };
            self.last_run_id = Some(plan.run_id);
        }
    }

    fn runtime_components_available(&self) -> bool {
        self.node_manager.is_some() && self.image_executor.is_some() && self.cache_manager.is_some()
    }

    fn register_default_executors(&mut self) {
        if self.executor_registry.is_empty() {
            self.executor_registry.register(ExecutorEntry::new(
                "image",
                ExecutorType::Image,
                |_| true,
            ));
        }
    }

    fn push_event(&mut self, event: ExecutionEvent) {
        self.events.push(event);
    }
}

impl Default for SchedulerManager {
    fn default() -> Self {
        Self::new()
    }
}

fn map_node_run_error(error: NodeRunError) -> SchedulerError {
    SchedulerError::RuntimeFailed {
        message: error.to_string(),
    }
}

fn collect_cached_outputs(
    cache_manager: &CacheManager,
    planned_node: &crate::scheduler::model::PlannedNode,
    node_def: &crate::node_manager::NodeDef,
    generation: GenerationId,
) -> Option<ExecutionResultsEntry> {
    let mut outputs = ExecutionResultsEntry::new();

    for output in &node_def.outputs {
        let value = cache_manager.get_result(
            planned_node.node_id,
            &output.name,
            planned_node.exec_signature,
            generation,
        )?;
        outputs.insert(output.name.clone(), value.as_ref().clone());
    }

    Some(outputs)
}

type ExecutionResultsEntry = std::collections::HashMap<String, types::Value>;

fn param_signature(planned_node: &crate::scheduler::model::PlannedNode) -> String {
    format!(
        "schema:{}:node:{}:params:{}",
        planned_node.exec_signature.sig_schema_version,
        planned_node.exec_signature.node_version,
        planned_node.exec_signature.params_hash,
    )
}

fn input_signature(planned_node: &crate::scheduler::model::PlannedNode) -> String {
    format!(
        "schema:{}:upstream:{}",
        planned_node.exec_signature.sig_schema_version,
        planned_node.exec_signature.upstream_hash,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::cache::manager::CacheManager;
    use crate::executors::image::GpuExecutor;
    use crate::graph::model::events::{GraphChange, GraphChangedEvent};
    use crate::graph::{Connection, PinRef};
    use crate::node_manager::{ExecutorType, NodeDef, ParamDef, ParamExpose, PinDef};
    use crate::scheduler::model::{ExecuteTarget, FinalStatus};
    use types::{DataType, Value};

    fn sample_graph() -> (Arc<Graph>, NodeId, NodeId, NodeId) {
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
        (Arc::new(g), a, b, c)
    }

    fn make_runtime_manager() -> SchedulerManager {
        let mut node_manager = NodeManager::new();
        node_manager.register(make_test_def("a"));
        node_manager.register(make_test_def("b"));
        node_manager.register(make_test_def("c"));

        SchedulerManager::new_with_runtime(
            Arc::new(node_manager),
            Arc::new(ImageExecutor::new(None::<GpuExecutor>)),
            Arc::new(CacheManager::new()),
        )
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
            execute: Box::new(|_ctx, mut inputs| {
                Box::pin(async move {
                    let base = match inputs.remove("in") {
                        Some(Value::Float(v)) => v,
                        _ => 0.0,
                    };
                    Ok(HashMap::from([(String::from("out"), Value::Float(base + 1.0))]))
                })
            }),
        }
    }

    #[test]
    fn query_state_reflects_defaults() {
        let manager = SchedulerManager::new();
        let summary = manager.query_state();

        assert_eq!(summary.mode, ExecutionMode::Auto);
        assert_eq!(summary.runtime_state, RuntimeState::Idle);
        assert_eq!(summary.dirty_count, 0);
        assert!(!summary.has_pending_changes);
        assert_eq!(summary.last_run_id, None);
    }

    #[test]
    fn manual_graph_change_marks_dirty_without_staging_plan() {
        let (graph, a, b, c) = sample_graph();
        let event = GraphChangedEvent {
            graph_version: 1,
            dirty: true,
            changes: vec![GraphChange::ParamCommitted {
                node_id: a,
                param: "strength".into(),
            }],
        };

        let mut manager = SchedulerManager::new();
        manager.set_mode(ExecutionMode::Manual);

        let run_id = manager
            .notify_graph_changed(graph, &event, GenerationId(0), |_| ExecutorType::Image)
            .unwrap();

        assert_eq!(run_id, None);
        assert!(manager.current_plan().is_none());
        assert!(manager.dirty_state().contains(&a));
        assert!(manager.dirty_state().contains(&b));
        assert!(manager.dirty_state().contains(&c));
    }

    #[test]
    fn request_execution_stages_plan_and_updates_state() {
        let (graph, a, _b, _c) = sample_graph();
        let mut manager = SchedulerManager::new();
        manager.set_mode(ExecutionMode::Manual);
        manager.dirty_state.mark(a, DirtyReason::ParamChanged);

        let run_id = manager
            .request_execution(
                Arc::clone(&graph),
                ExecutionRequest::new(ExecuteTarget::Graph),
                GenerationId(0),
                |_| ExecutorType::Image,
            )
            .unwrap();

        assert_eq!(manager.current_plan().map(|plan| plan.run_id), Some(run_id));
        assert_eq!(
            manager.query_state().runtime_state,
            RuntimeState::Planning { run_id }
        );
    }

    #[test]
    fn request_execution_runs_immediately_when_runtime_is_configured() {
        let (graph, a, _b, _c) = sample_graph();
        let mut manager = make_runtime_manager();
        manager.set_mode(ExecutionMode::Manual);
        manager.dirty_state.mark(a, DirtyReason::ParamChanged);

        let run_id = manager
            .request_execution(
                graph,
                ExecutionRequest::new(ExecuteTarget::Graph),
                GenerationId(0),
                |_| ExecutorType::Image,
            )
            .unwrap();

        assert!(manager.current_plan().is_none());
        assert!(manager.running_task().is_none());
        assert_eq!(
            manager.query_state().runtime_state,
            RuntimeState::LastCompleted {
                run_id,
                status: FinalStatus::Finished,
            }
        );
    }

    #[test]
    fn request_execution_emits_basic_event_sequence() {
        let (graph, a, _b, _c) = sample_graph();
        let mut manager = make_runtime_manager();
        manager.set_mode(ExecutionMode::Manual);
        manager.dirty_state.mark(a, DirtyReason::ParamChanged);

        let run_id = manager
            .request_execution(
                graph,
                ExecutionRequest::new(ExecuteTarget::Graph),
                GenerationId(0),
                |_| ExecutorType::Image,
            )
            .unwrap();

        let events = manager.drain_events();

        assert!(matches!(
            events.first(),
            Some(ExecutionEvent::Started { run_id: id, .. }) if *id == run_id
        ));
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutionEvent::NodeStarted { run_id: id, node_id } if *id == run_id && *node_id == a
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ExecutionEvent::Finished { run_id: id } if *id == run_id
        )));
    }

    #[test]
    fn cancel_execution_clears_current_plan() {
        let (graph, a, _b, _c) = sample_graph();
        let mut manager = SchedulerManager::new();
        manager.set_mode(ExecutionMode::Manual);
        manager.dirty_state.mark(a, DirtyReason::ParamChanged);

        let run_id = manager
            .request_execution(
                graph,
                ExecutionRequest::new(ExecuteTarget::Graph),
                GenerationId(0),
                |_| ExecutorType::Image,
            )
            .unwrap();

        manager.cancel_execution(run_id).unwrap();

        assert!(manager.current_plan().is_none());
        assert_eq!(
            manager.query_state().runtime_state,
            RuntimeState::LastCompleted {
                run_id,
                status: FinalStatus::Cancelled,
            }
        );
    }
}
