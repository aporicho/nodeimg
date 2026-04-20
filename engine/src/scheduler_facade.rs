use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::ArtifactRecord;
use crate::cache::manager::CacheManager;
use crate::cache::model::ExecSignature;
use crate::events::{EngineEvent, EventSubscription, ExecutionState, PendingExecutionId};
use crate::execution::{CookingContextRange, ExecutionMode, ProgressSink};
use crate::facade::{EngineError, ExecutionRequest, ExecutionRequestResult, ExecutionTicket};
use crate::graph;
use crate::graph::model::events::{GraphChange, GraphChangedEvent};
use crate::graph::model::subgraph::ExecuteTarget;
use crate::graph::query::downstream::downstream;
#[cfg(test)]
use crate::graph::query::resolve_subgraph::resolve_subgraph;
use crate::planner::{self, PlanRequest, PlannerError};
use crate::runtime::Runtime;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};
use types::{NodeId, Value};

#[cfg(test)]
use crate::graph::model::subgraph::ExecuteTarget as TestExecuteTarget;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExecutionScope {
    CommittedGraph,
    PreviewSnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DirtyReason {
    ParamChanged,
    UpstreamChanged,
    StructureChanged,
    ArtifactSelectionChanged,
}

#[derive(Clone, Debug, Default)]
struct DirtyState {
    dirty_nodes: HashSet<NodeId>,
    dirty_reasons: HashMap<NodeId, DirtyReason>,
}

impl DirtyState {
    fn new() -> Self {
        Self::default()
    }

    fn is_empty(&self) -> bool {
        self.dirty_nodes.is_empty()
    }

    fn mark(&mut self, node_id: NodeId, reason: DirtyReason) {
        self.dirty_nodes.insert(node_id);
        self.dirty_reasons.insert(node_id, reason);
    }
}

fn apply_graph_event(
    graph: &graph::Graph,
    dirty_state: &mut DirtyState,
    event: &GraphChangedEvent,
) {
    for change in &event.changes {
        apply_graph_change(graph, dirty_state, change);
    }
}

fn apply_graph_change(graph: &graph::Graph, dirty_state: &mut DirtyState, change: &GraphChange) {
    match change {
        GraphChange::NodeAdded { node_id } => {
            dirty_state.mark(*node_id, DirtyReason::StructureChanged);
        }
        GraphChange::NodeRemoved { .. } => {
            // 删除节点后通过同事件内的 ConnectionRemoved 去标脏仍然存在的下游节点。
        }
        GraphChange::ConnectionAdded { to, .. } | GraphChange::ConnectionRemoved { to, .. } => {
            mark_node_and_downstream(graph, dirty_state, to.node, DirtyReason::StructureChanged);
        }
        GraphChange::ParamCommitted { node_id, .. } => {
            mark_node_and_downstream(graph, dirty_state, *node_id, DirtyReason::ParamChanged);
        }
        GraphChange::Replaced | GraphChange::Undone | GraphChange::Redone => {
            dirty_state.dirty_nodes.clear();
            dirty_state.dirty_reasons.clear();
            for node_id in graph.nodes.keys().copied() {
                dirty_state.mark(node_id, DirtyReason::StructureChanged);
            }
        }
    }
}

fn mark_node_and_downstream(
    graph: &graph::Graph,
    dirty_state: &mut DirtyState,
    node_id: NodeId,
    reason: DirtyReason,
) {
    dirty_state.mark(node_id, reason);
    for downstream_node in downstream(graph, node_id) {
        let downstream_reason = match reason {
            DirtyReason::ParamChanged => DirtyReason::UpstreamChanged,
            DirtyReason::StructureChanged
            | DirtyReason::UpstreamChanged
            | DirtyReason::ArtifactSelectionChanged => DirtyReason::UpstreamChanged,
        };
        dirty_state.mark(downstream_node, downstream_reason);
    }
}

pub struct SchedulerFacade {
    runtime: Runtime,
    cache: Arc<CacheManager>,
    artifacts: Option<Arc<Mutex<ArtifactManager>>>,
    cooking_range: CookingContextRange,
    dirty: Arc<Mutex<DirtyState>>,
    committed_graph: Arc<Mutex<Option<Arc<graph::Graph>>>>,
    self_handle: Weak<Mutex<SchedulerFacade>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Connection, Graph, PinRef};

    #[test]
    fn collect_dirty_nodes_expands_downstream_for_graph_target() {
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

        let cache = std::sync::Arc::new(crate::cache::manager::CacheManager::new());
        let mut scheduler = SchedulerFacade {
            runtime: crate::runtime::Runtime::new(
                std::sync::Arc::new(crate::node_manager::NodeManager::new()),
                std::sync::Arc::new(crate::capability::CapabilityRegistry::new()),
                crate::executors::image::ImageExecutor::new(None),
                std::sync::Arc::clone(&cache),
                None,
            ),
            cache,
            artifacts: None,
            cooking_range: CookingContextRange::default(),
            dirty: Arc::new(Mutex::new(DirtyState::default())),
            committed_graph: Arc::new(Mutex::new(None)),
            self_handle: Weak::new(),
        };
        scheduler.notify_graph_changed(
            &g,
            &GraphChangedEvent {
                graph_version: 1,
                dirty: true,
                changes: vec![crate::graph::model::events::GraphChange::ParamCommitted {
                    node_id: a,
                    param: "value".into(),
                }],
            },
        );

        let dirty = scheduler.collect_dirty_nodes(&g, &TestExecuteTarget::Graph);
        assert_eq!(dirty, [a, b, c].into_iter().collect());
    }

    #[test]
    fn collect_dirty_nodes_intersects_with_node_target_closure() {
        let g = Graph::new();
        let (g, a) = g.add_node("a", Default::default());
        let (g, b) = g.add_node("b", Default::default());
        let (g, c) = g.add_node("c", Default::default());
        let (g, d) = g.add_node("d", Default::default());
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

        let cache = std::sync::Arc::new(crate::cache::manager::CacheManager::new());
        let mut scheduler = SchedulerFacade {
            runtime: crate::runtime::Runtime::new(
                std::sync::Arc::new(crate::node_manager::NodeManager::new()),
                std::sync::Arc::new(crate::capability::CapabilityRegistry::new()),
                crate::executors::image::ImageExecutor::new(None),
                std::sync::Arc::clone(&cache),
                None,
            ),
            cache,
            artifacts: None,
            cooking_range: CookingContextRange::default(),
            dirty: Arc::new(Mutex::new(DirtyState::default())),
            committed_graph: Arc::new(Mutex::new(None)),
            self_handle: Weak::new(),
        };
        scheduler.notify_graph_changed(
            &g,
            &GraphChangedEvent {
                graph_version: 1,
                dirty: true,
                changes: vec![
                    crate::graph::model::events::GraphChange::ParamCommitted {
                        node_id: a,
                        param: "value".into(),
                    },
                    crate::graph::model::events::GraphChange::ParamCommitted {
                        node_id: d,
                        param: "value".into(),
                    },
                ],
            },
        );

        let dirty = scheduler.collect_dirty_nodes(&g, &TestExecuteTarget::Node(c));
        assert_eq!(dirty, [a, b, c].into_iter().collect());
        assert!(!dirty.contains(&d));
    }

    #[test]
    fn replaced_event_advances_cache_generation() {
        let cache = std::sync::Arc::new(crate::cache::manager::CacheManager::new());
        let mut scheduler = SchedulerFacade::new(
            crate::runtime::Runtime::new(
                std::sync::Arc::new(crate::node_manager::NodeManager::new()),
                std::sync::Arc::new(crate::capability::CapabilityRegistry::new()),
                crate::executors::image::ImageExecutor::new(None),
                std::sync::Arc::clone(&cache),
                None,
            ),
            std::sync::Arc::clone(&cache),
            None,
        );

        let graph = Graph::new();
        let before = cache.current_generation();
        scheduler.notify_graph_changed(
            &graph,
            &GraphChangedEvent {
                graph_version: 1,
                dirty: true,
                changes: vec![crate::graph::model::events::GraphChange::Replaced],
            },
        );

        assert_ne!(cache.current_generation(), before);
    }
}

impl SchedulerFacade {
    pub fn new(
        runtime: Runtime,
        cache: Arc<CacheManager>,
        artifacts: Option<Arc<Mutex<ArtifactManager>>>,
    ) -> Self {
        Self {
            runtime,
            cache,
            artifacts,
            cooking_range: CookingContextRange::default(),
            dirty: Arc::new(Mutex::new(DirtyState::default())),
            committed_graph: Arc::new(Mutex::new(None)),
            self_handle: Weak::new(),
        }
    }

    pub fn bind_handle(&mut self, handle: &Arc<Mutex<SchedulerFacade>>) {
        self.self_handle = Arc::downgrade(handle);
    }

    pub fn notify_graph_changed(&mut self, graph: &graph::Graph, event: &GraphChangedEvent) {
        if event.changes.is_empty() {
            return;
        }

        *self
            .committed_graph
            .lock()
            .expect("committed graph lock poisoned") = Some(Arc::new(graph.clone()));

        let mut affected = DirtyState::new();
        apply_graph_event(graph, &mut affected, event);
        apply_graph_event(
            graph,
            &mut self.dirty.lock().expect("dirty state lock poisoned"),
            event,
        );

        if event
            .changes
            .iter()
            .any(|change| matches!(change, GraphChange::Replaced))
        {
            self.cache.clear_all();
            return;
        }

        if !affected.is_empty() {
            let affected_nodes = affected.dirty_nodes.iter().copied().collect::<Vec<_>>();
            self.cache.invalidate_subgraph(&affected_nodes);
        }
    }

    pub fn query_artifact_history(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Vec<ArtifactRecord>, EngineError> {
        let Some(artifacts) = self.artifacts.as_ref() else {
            return Err(EngineError::Artifact {
                message: "artifact history is not configured".into(),
            });
        };

        Ok(artifacts
            .lock()
            .expect("artifact manager lock poisoned")
            .list_artifacts(node_id, output_key))
    }

    pub fn query_selected_artifact(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Option<ArtifactRecord>, EngineError> {
        let Some(artifacts) = self.artifacts.as_ref() else {
            return Err(EngineError::Artifact {
                message: "artifact history is not configured".into(),
            });
        };
        let artifacts = artifacts.lock().expect("artifact manager lock poisoned");
        match artifacts.get_selected_artifact(node_id, output_key) {
            Ok(record) => Ok(Some(record.clone())),
            Err(crate::artifact::model::ArtifactError::SelectedArtifactNotFound { .. }) => Ok(None),
            Err(error) => Err(EngineError::Artifact {
                message: error.to_string(),
            }),
        }
    }

    pub fn select_artifact_version(
        &mut self,
        graph: &graph::Graph,
        node_id: NodeId,
        output_key: &str,
        artifact_id: &str,
    ) -> Result<(), EngineError> {
        let Some(artifacts) = self.artifacts.as_ref() else {
            return Err(EngineError::Artifact {
                message: "artifact history is not configured".into(),
            });
        };

        {
            let mut manager = artifacts.lock().expect("artifact manager lock poisoned");
            let record = manager.get_artifact(artifact_id).cloned().ok_or_else(|| {
                EngineError::Artifact {
                    message: format!("artifact not found: {artifact_id}"),
                }
            })?;
            if record.node_id != node_id || record.output_key != output_key {
                return Err(EngineError::Artifact {
                    message: format!(
                        "artifact '{artifact_id}' does not belong to node {:?} output '{output_key}'",
                        node_id
                    ),
                });
            }
            manager
                .select_artifact_and_save(artifact_id)
                .map_err(|error| EngineError::Artifact {
                    message: error.to_string(),
                })?;
        }

        *self
            .committed_graph
            .lock()
            .expect("committed graph lock poisoned") = Some(Arc::new(graph.clone()));

        let mut affected_nodes = vec![node_id];
        affected_nodes.extend(downstream(graph, node_id));
        {
            let mut dirty = self.dirty.lock().expect("dirty state lock poisoned");
            for affected in &affected_nodes {
                let reason = if *affected == node_id {
                    DirtyReason::ArtifactSelectionChanged
                } else {
                    DirtyReason::UpstreamChanged
                };
                dirty.mark(*affected, reason);
            }
        }
        self.cache.invalidate_subgraph(&affected_nodes);
        Ok(())
    }

    pub fn set_cooking_range(&mut self, range: CookingContextRange) {
        self.cooking_range = range;
    }

    pub fn request_execution(
        &mut self,
        graph: &graph::Graph,
        request: ExecutionRequest,
    ) -> Result<ExecutionRequestResult, EngineError> {
        self.request_execution_internal(graph, request, None)
    }

    fn request_execution_from_pending(
        &mut self,
        graph: &graph::Graph,
        request: ExecutionRequest,
        pending_id: PendingExecutionId,
    ) -> Result<ExecutionRequestResult, EngineError> {
        self.request_execution_internal(graph, request, Some(pending_id))
    }

    fn request_execution_internal(
        &mut self,
        graph: &graph::Graph,
        request: ExecutionRequest,
        pending_id: Option<PendingExecutionId>,
    ) -> Result<ExecutionRequestResult, EngineError> {
        *self
            .committed_graph
            .lock()
            .expect("committed graph lock poisoned") = Some(Arc::new(graph.clone()));
        self.request_execution_in_scope_with_sink(
            graph,
            request,
            ExecutionScope::CommittedGraph,
            Arc::new(crate::execution::NoopProgressSink),
            pending_id,
        )
    }

    pub fn request_preview_execution(
        &mut self,
        graph: &graph::Graph,
        request: ExecutionRequest,
        progress_sink: Arc<dyn ProgressSink>,
    ) -> Result<ExecutionTicket, EngineError> {
        match self.request_execution_in_scope_with_sink(
            graph,
            request,
            ExecutionScope::PreviewSnapshot,
            progress_sink,
            None,
        )? {
            ExecutionRequestResult::Started(ticket) => Ok(ticket),
            ExecutionRequestResult::Queued { .. } => Err(EngineError::Execution {
                message: "preview execution unexpectedly queued".into(),
            }),
        }
    }

    pub(crate) fn request_execution_in_scope_with_sink(
        &mut self,
        graph: &graph::Graph,
        request: ExecutionRequest,
        scope: ExecutionScope,
        progress_sink: Arc<dyn ProgressSink>,
        pending_id: Option<PendingExecutionId>,
    ) -> Result<ExecutionRequestResult, EngineError> {
        let mode = request.mode.clone().unwrap_or_default();
        let is_preview_request = matches!(
            &mode,
            crate::execution::ExecutionMode::OneShot {
                fidelity: crate::execution::EvaluationFidelity::Preview { .. }
            }
        );
        let plan = planner::build_execution_plan(PlanRequest {
            graph,
            request: &request,
            mode: mode.clone(),
            cooking_range: &self.cooking_range,
            node_manager: self.runtime.node_manager(),
        })
        .map_err(planner_error_to_engine_error)?;
        let completion_hook = if matches!(scope, ExecutionScope::CommittedGraph)
            && !is_preview_request
        {
            let dirty = Arc::clone(&self.dirty);
            let resolved_nodes = plan
                .subtasks
                .iter()
                .flat_map(|subtask| subtask.order.iter().map(|node| node.node_id))
                .collect::<HashSet<_>>();
            Some(Arc::new(move |status| {
                if !matches!(status, crate::execution::ExecutionTerminalStatus::Finished) {
                    return;
                }
                let mut dirty = dirty.lock().expect("dirty state lock poisoned");
                for node_id in &resolved_nodes {
                    dirty.dirty_nodes.remove(node_id);
                    dirty.dirty_reasons.remove(node_id);
                }
            })
                as Arc<
                    dyn Fn(crate::execution::ExecutionTerminalStatus) + Send + Sync,
                >)
        } else if let Some(handle) = self.self_handle.upgrade() {
            Some(Arc::new(move |status| {
                if !matches!(status, crate::execution::ExecutionTerminalStatus::Finished) {
                    return;
                }
                let mut scheduler = handle.lock().expect("scheduler lock poisoned");
                let Some((pending_id, request)) =
                    scheduler.runtime.take_pending_full_for_resubmit()
                else {
                    return;
                };
                let graph = scheduler
                    .committed_graph
                    .lock()
                    .expect("committed graph lock poisoned")
                    .clone();
                let Some(graph) = graph else {
                    return;
                };
                let _ =
                    scheduler.request_execution_from_pending(graph.as_ref(), request, pending_id);
            })
                as Arc<
                    dyn Fn(crate::execution::ExecutionTerminalStatus) + Send + Sync,
                >)
        } else {
            None
        };
        let pending_origin = if matches!(scope, ExecutionScope::CommittedGraph) {
            pending_id
        } else {
            None
        };
        self.runtime.request_execution_plan_with_sink_and_hook(
            plan,
            request,
            progress_sink,
            completion_hook,
            pending_origin,
        )
    }

    pub fn cancel_execution(
        &self,
        execution_id: crate::execution::ExecutionId,
    ) -> Result<(), EngineError> {
        self.runtime.cancel_execution(execution_id)
    }

    fn query_signature_map(
        &self,
        graph: &graph::Graph,
        target: &ExecuteTarget,
    ) -> Result<HashMap<NodeId, ExecSignature>, EngineError> {
        let mode = ExecutionMode::default();
        let request = ExecutionRequest {
            target: target.clone(),
            mode: Some(mode.clone()),
        };
        let plan = planner::build_execution_plan(PlanRequest {
            graph,
            request: &request,
            mode,
            cooking_range: &self.cooking_range,
            node_manager: self.runtime.node_manager(),
        })
        .map_err(planner_error_to_engine_error)?;
        Ok(plan
            .subtasks
            .last()
            .map(|subtask| {
                subtask
                    .order
                    .iter()
                    .map(|node| (node.node_id, node.exec_signature))
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn query_signature(
        &self,
        graph: &graph::Graph,
        target: &ExecuteTarget,
    ) -> Result<Option<ExecSignature>, EngineError> {
        let signatures = self.query_signature_map(graph, target)?;
        Ok(match target {
            ExecuteTarget::Node(node_id) => signatures.get(node_id).copied(),
            ExecuteTarget::Graph => None,
        })
    }

    pub fn output_names_for_node(
        &self,
        graph: &graph::Graph,
        node_id: NodeId,
    ) -> Result<Vec<String>, EngineError> {
        let node = graph
            .nodes
            .get(&node_id)
            .ok_or_else(|| EngineError::Graph {
                message: format!("Node {:?} not found", node_id),
            })?;
        let def = self
            .runtime
            .node_manager()
            .get_node_def(&node.type_id)
            .ok_or_else(|| EngineError::Graph {
                message: format!("Node type '{}' not registered", node.type_id),
            })?;
        Ok(def
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect())
    }

    pub fn query_execution_outputs(
        &self,
        execution_id: crate::execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        self.runtime.query_execution_outputs(execution_id, node_id)
    }

    pub fn await_execution(
        &self,
        execution_id: crate::execution::ExecutionId,
    ) -> Result<crate::execution::ExecutionTerminalStatus, EngineError> {
        self.runtime.await_execution(execution_id)
    }

    pub fn query_state(&self) -> ExecutionState {
        self.runtime.query_state()
    }

    pub fn subscribe_engine_events(&self) -> EventSubscription<EngineEvent> {
        self.runtime.subscribe_events()
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    #[cfg(test)]
    fn collect_dirty_nodes(&self, graph: &graph::Graph, target: &ExecuteTarget) -> HashSet<NodeId> {
        match target {
            ExecuteTarget::Graph => self
                .dirty
                .lock()
                .expect("dirty state lock poisoned")
                .dirty_nodes
                .clone(),
            ExecuteTarget::Node(node_id) => {
                let target_closure = {
                    let mut set = HashSet::from([*node_id]);
                    set.extend(
                        resolve_subgraph(graph, ExecuteTarget::Node(*node_id))
                            .map(|info| info.nodes)
                            .unwrap_or_default(),
                    );
                    set
                };
                self.dirty
                    .lock()
                    .expect("dirty state lock poisoned")
                    .dirty_nodes
                    .iter()
                    .copied()
                    .filter(|node_id| target_closure.contains(node_id))
                    .collect()
            }
        }
    }
}

fn planner_error_to_engine_error(error: PlannerError) -> EngineError {
    match error {
        PlannerError::Schema { message } => EngineError::Schema { message },
        PlannerError::Graph { message } => EngineError::Execution { message },
        PlannerError::NodeNotFound { node_id } => EngineError::Graph {
            message: format!("Node {:?} not found in graph", node_id),
        },
        PlannerError::NodeTypeNotRegistered { type_id } => EngineError::Graph {
            message: format!("Node type '{type_id}' not registered"),
        },
    }
}
