use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::cache::model::ExecSignature;
use crate::events::{EventBus, EventRecord, EventSubscription};
use crate::execution::ExecutionTerminalStatus;
use crate::execution::{EvaluationFidelity, ExecutionMode, ExecutorProgressEvent, PreviewPrecision, ProgressSink};
use crate::facade::{
    EngineError, EngineFacade, EngineSubscription, ExecutionRequest, ExecutionRequestResult,
    ExecutionTicket,
};
use crate::graph::model::batch::{EditBatchRequest, EditBatchResult, GraphEdit};
use crate::graph::model::state::{PreviewOverlay, PreviewTarget};
use crate::graph::model::subgraph::ExecuteTarget;
use crate::graph::query::resolve_subgraph::resolve_subgraph;
use crate::scheduler_facade::SchedulerFacade;
use crate::Engine;
use crate::{events::ExecutionState, graph::Graph};
use types::{NodeId, Value};

#[derive(Debug)]
pub enum SessionError {
    Engine(EngineError),
    InvalidInteraction(String),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Engine(error) => write!(f, "{error}"),
            SessionError::InvalidInteraction(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<EngineError> for SessionError {
    fn from(value: EngineError) -> Self {
        SessionError::Engine(value)
    }
}

#[derive(Clone, Debug)]
pub enum InteractionEvent {
    ParamChanged {
        node_id: NodeId,
        param: String,
        value: Value,
        preview: bool,
    },
    UndoRequested,
    RedoRequested,
    ExportRequested {
        target: ExecuteTarget,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvent {
    PreviewUpdated { node_id: NodeId, output_pin: String },
    InteractionAck { event_id: u64 },
    UndoStackChanged { can_undo: bool, can_redo: bool },
}

pub type SessionSubscription = EventSubscription<SessionEvent>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreviewRequestStatus {
    CacheHit,
    Executed(ExecutionTicket),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PreviewCacheKey {
    node_id: NodeId,
    output_pin: String,
    exec_signature: ExecSignature,
    fidelity: EvaluationFidelity,
}

#[derive(Default)]
struct PreviewStore {
    preview_cache: HashMap<PreviewCacheKey, Value>,
    latest_preview: HashMap<(NodeId, String), Value>,
}

struct SessionPreviewSink {
    store: Arc<Mutex<PreviewStore>>,
    events: EventBus<SessionEvent>,
}

impl ProgressSink for SessionPreviewSink {
    fn report(&self, event: ExecutorProgressEvent) {
        let ExecutorProgressEvent::PreviewReady {
            node_id,
            output,
            value,
            exec_signature,
            fidelity,
        } = event
        else {
            return;
        };

        let mut store = self.store.lock().expect("preview store lock poisoned");
        let key = PreviewCacheKey {
            node_id,
            output_pin: output.clone(),
            exec_signature,
            fidelity,
        };
        store.preview_cache.insert(key, value.clone());
        store.latest_preview.insert((node_id, output.clone()), value);
        drop(store);

        let _ = self.events.publish(SessionEvent::PreviewUpdated {
            node_id,
            output_pin: output,
        });
    }
}

pub struct Session {
    engine: Arc<Mutex<Engine>>,
    scheduler: Arc<Mutex<SchedulerFacade>>,
    preview_overlay: Option<PreviewOverlay>,
    preview_store: Arc<Mutex<PreviewStore>>,
    events: EventBus<SessionEvent>,
    next_event_id: u64,
}

impl Session {
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        let scheduler = engine
            .lock()
            .expect("session engine lock poisoned")
            .scheduler_handle();
        Self {
            engine,
            scheduler,
            preview_overlay: None,
            preview_store: Arc::new(Mutex::new(PreviewStore::default())),
            events: EventBus::new(256),
            next_event_id: 0,
        }
    }

    pub fn apply_edit(&mut self, edit: GraphEdit) -> Result<EditBatchResult, SessionError> {
        if matches!(edit, GraphEdit::SetParam { preview: true, .. }) {
            return Err(SessionError::InvalidInteraction(
                "preview edits must use submit_interaction".into(),
            ));
        }

        self.clear_preview_state();
        let (result, summary) = {
            let mut engine = self.engine.lock().expect("session engine lock poisoned");
            let result = engine.apply_batch(EditBatchRequest {
                edits: vec![edit],
                label: None,
            })?;
            let summary = engine.graph_state_summary();
            (result, summary)
        };
        self.record_interaction_ack();
        self.record_undo_stack_changed(summary.can_undo, summary.can_redo);
        Ok(result)
    }

    pub fn submit_interaction(&mut self, event: InteractionEvent) -> Result<(), SessionError> {
        match event {
            InteractionEvent::ParamChanged {
                node_id,
                param,
                value,
                preview,
            } => {
                let graph = self
                    .engine
                    .lock()
                    .expect("session engine lock poisoned")
                    .graph_snapshot();
                if !graph.nodes.contains_key(&node_id) {
                    return Err(SessionError::InvalidInteraction(format!(
                        "node {:?} not found for param '{}'",
                        node_id, param
                    )));
                }

                if preview {
                    self.preview_overlay = Some(PreviewOverlay {
                        target: PreviewTarget {
                            node_id,
                            param,
                        },
                        value,
                    });
                    self.preview_store
                        .lock()
                        .expect("preview store lock poisoned")
                        .latest_preview
                        .clear();
                } else {
                    self.clear_preview_state();
                    let summary = {
                        let mut engine = self.engine.lock().expect("session engine lock poisoned");
                        engine.set_param(node_id, &param, value, false);
                        engine.graph_state_summary()
                    };
                    self.record_undo_stack_changed(summary.can_undo, summary.can_redo);
                }
                self.record_interaction_ack();
                Ok(())
            }
            InteractionEvent::UndoRequested => {
                self.clear_preview_state();
                let summary = {
                    let mut engine = self.engine.lock().expect("session engine lock poisoned");
                    engine.undo();
                    engine.graph_state_summary()
                };
                self.record_interaction_ack();
                self.record_undo_stack_changed(summary.can_undo, summary.can_redo);
                Ok(())
            }
            InteractionEvent::RedoRequested => {
                self.clear_preview_state();
                let summary = {
                    let mut engine = self.engine.lock().expect("session engine lock poisoned");
                    engine.redo();
                    engine.graph_state_summary()
                };
                self.record_interaction_ack();
                self.record_undo_stack_changed(summary.can_undo, summary.can_redo);
                Ok(())
            }
            InteractionEvent::ExportRequested { .. } => {
                self.record_interaction_ack();
                Ok(())
            }
        }
    }

    pub async fn request_preview(&mut self, target: ExecuteTarget) -> Result<PreviewRequestStatus, SessionError> {
        let (preview_graph, cacheable_target) = {
            let engine = self.engine.lock().expect("session engine lock poisoned");
            let base_graph = engine.graph_snapshot();
            let preview_graph = self.build_preview_graph(base_graph.as_ref())?;
            resolve_subgraph(&preview_graph, target.clone()).map_err(|error| {
                SessionError::InvalidInteraction(format!("failed to resolve preview target: {error}"))
            })?;
            drop(engine);
            let scheduler = self.scheduler.lock().expect("session scheduler lock poisoned");
            let cacheable_target = match target {
                ExecuteTarget::Node(node_id) => Some((
                    node_id,
                    scheduler.query_signature(&preview_graph, &target)?.ok_or_else(|| {
                        SessionError::InvalidInteraction(format!(
                            "missing execution signature for preview target {:?}",
                            node_id
                        ))
                    })?,
                    scheduler.output_names_for_node(&preview_graph, node_id)?,
                )),
                ExecuteTarget::Graph => None,
            };
            (preview_graph, cacheable_target)
        };
        let fidelity = Self::preview_fidelity();

        if let Some((node_id, exec_signature, output_names)) = &cacheable_target {
            if self.populate_latest_preview_from_cache(
                *node_id,
                *exec_signature,
                output_names,
                &fidelity,
            ) {
                return Ok(PreviewRequestStatus::CacheHit);
            }
        }

        self.preview_store
            .lock()
            .expect("preview store lock poisoned")
            .latest_preview
            .clear();
        let preview_sink: Arc<dyn ProgressSink> = Arc::new(SessionPreviewSink {
            store: Arc::clone(&self.preview_store),
            events: self.events.clone(),
        });
        let ticket = self
            .scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .request_preview_execution(
                &preview_graph,
                ExecutionRequest {
                    target,
                    mode: Some(ExecutionMode::OneShot {
                        fidelity: fidelity.clone(),
                    }),
                },
                preview_sink,
            )?;

        Ok(PreviewRequestStatus::Executed(ticket))
    }

    pub async fn request_export(
        &mut self,
        target: ExecuteTarget,
    ) -> Result<ExecutionRequestResult, SessionError> {
        let graph = self
            .engine
            .lock()
            .expect("session engine lock poisoned")
            .graph_snapshot();
        Ok(self
            .scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .request_execution(
                &graph,
                ExecutionRequest {
                    target,
                    mode: Some(ExecutionMode::OneShot {
                        fidelity: EvaluationFidelity::Full,
                    }),
                },
            )?)
    }

    pub fn cancel_execution(&self, execution_id: u64) -> Result<(), SessionError> {
        Ok(self
            .scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .cancel_execution(execution_id)?)
    }

    pub fn graph_snapshot(&self) -> Arc<Graph> {
        self.engine
            .lock()
            .expect("session engine lock poisoned")
            .graph_snapshot()
    }

    pub fn graph_state_summary(&self) -> crate::graph::model::state::GraphStateSummary {
        self.engine
            .lock()
            .expect("session engine lock poisoned")
            .graph_state_summary()
    }

    pub fn execution_state(&self) -> ExecutionState {
        self.scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .query_state()
    }

    pub fn subscribe_engine_events(&self) -> EngineSubscription {
        self.scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .subscribe_engine_events()
    }

    pub fn await_execution(&self, execution_id: u64) -> Result<ExecutionTerminalStatus, SessionError> {
        Ok(self
            .scheduler
            .lock()
            .expect("session scheduler lock poisoned")
            .await_execution(execution_id)?)
    }

    pub fn get_preview(&self, node_id: NodeId, output_pin: &str) -> Option<Value> {
        self.preview_store
            .lock()
            .expect("preview store lock poisoned")
            .latest_preview
            .get(&(node_id, output_pin.to_string()))
            .cloned()
    }

    pub fn subscribe_session_events(&self) -> SessionSubscription {
        self.events.subscribe()
    }

    pub fn events_snapshot(&self) -> Vec<EventRecord<SessionEvent>> {
        self.events.snapshot()
    }

    pub fn discard_preview(&mut self) {
        self.preview_overlay = None;
        self.preview_store
            .lock()
            .expect("preview store lock poisoned")
            .latest_preview
            .clear();
    }

    fn preview_fidelity() -> EvaluationFidelity {
        EvaluationFidelity::Preview {
            max_side: Some(1024),
            precision: PreviewPrecision::Float16,
        }
    }

    fn clear_preview_state(&mut self) {
        self.preview_overlay = None;
        let mut store = self
            .preview_store
            .lock()
            .expect("preview store lock poisoned");
        store.preview_cache.clear();
        store.latest_preview.clear();
    }

    fn build_preview_graph(&self, base_graph: &crate::graph::Graph) -> Result<crate::graph::Graph, SessionError> {
        let Some(overlay) = &self.preview_overlay else {
            return Ok(base_graph.clone());
        };
        if !base_graph.nodes.contains_key(&overlay.target.node_id) {
            return Err(SessionError::InvalidInteraction(format!(
                "node {:?} not found for preview overlay '{}'",
                overlay.target.node_id, overlay.target.param
            )));
        }
        Ok(base_graph.set_param(
            overlay.target.node_id,
            &overlay.target.param,
            overlay.value.clone(),
        ))
    }

    fn populate_latest_preview_from_cache(
        &mut self,
        node_id: NodeId,
        exec_signature: ExecSignature,
        output_names: &[String],
        fidelity: &EvaluationFidelity,
    ) -> bool {
        let mut latest_preview = HashMap::new();
        let store = self
            .preview_store
            .lock()
            .expect("preview store lock poisoned");

        for output_pin in output_names {
            let key = PreviewCacheKey {
                node_id,
                output_pin: output_pin.clone(),
                exec_signature,
                fidelity: fidelity.clone(),
            };
            let Some(value) = store.preview_cache.get(&key).cloned() else {
                return false;
            };
            latest_preview.insert((node_id, output_pin.clone()), value);
        }

        drop(store);
        self.preview_store
            .lock()
            .expect("preview store lock poisoned")
            .latest_preview = latest_preview;
        self.emit_preview_updates();
        true
    }

    fn emit_preview_updates(&mut self) {
        let preview_keys = self
            .preview_store
            .lock()
            .expect("preview store lock poisoned")
            .latest_preview
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for (node_id, output_pin) in preview_keys {
            let _ = self.events.publish(SessionEvent::PreviewUpdated {
                node_id,
                output_pin,
            });
        }
    }

    fn record_interaction_ack(&mut self) {
        let event_id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);
        let _ = self.events.publish(SessionEvent::InteractionAck { event_id });
    }

    fn record_undo_stack_changed(&mut self, can_undo: bool, can_redo: bool) {
        let _ = self.events.publish(SessionEvent::UndoStackChanged {
            can_undo,
            can_redo,
        });
    }
}
