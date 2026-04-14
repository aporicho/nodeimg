pub mod artifact;
pub mod builtins;
pub mod cache;
pub mod capability;
pub mod execution;
pub mod executors;
pub mod events;
pub mod facade;
pub mod graph;
pub mod node_manager;
pub mod node_registry;
pub mod runtime;
pub mod scheduler_facade;
pub mod session;

use cache::manager::CacheManager;
use events::{EngineEvent, EventRecord, ExecutionState};
use execution::{CookingContextRange, EvaluationFidelity, ExecutionTerminalStatus};
use executors::image::{GpuExecutor, ImageExecutor};
use facade::{EngineError, EngineFacade, EngineSubscription, ExecutionRequest, ExecutionTicket};
use graph::model::batch::{EditBatchRequest, EditBatchResult};
use graph::model::events::{GraphChangedEvent, GraphEvent, GraphEventKind};
use graph::model::state::GraphStateSummary;
use graph::{Connection, Graph, GraphController, PinRef};
use node_manager::NodeManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use types::{NodeId, Value};

/// Engine：顶层门面与装配根。
pub struct Engine {
    graph: GraphController,
    node_manager: Arc<NodeManager>,
    scheduler: Arc<Mutex<scheduler_facade::SchedulerFacade>>,
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
        let capability_registry = Arc::new(registry.capability_registry);
        let image_executor = ImageExecutor::new(gpu);
        let cache = Arc::new(CacheManager::new());

        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            scheduler: Arc::new(Mutex::new(scheduler_facade::SchedulerFacade::new(
                runtime::Runtime::new(
                    Arc::clone(&node_manager),
                    Arc::clone(&capability_registry),
                    image_executor,
                    Arc::clone(&cache),
                ),
                cache,
            ))),
            node_manager,
        }
    }

    pub(crate) fn scheduler_handle(&self) -> Arc<Mutex<scheduler_facade::SchedulerFacade>> {
        Arc::clone(&self.scheduler)
    }

    pub fn graph_snapshot(&self) -> Arc<Graph> {
        self.graph.snapshot()
    }

    pub fn graph_state_summary(&self) -> GraphStateSummary {
        self.graph.state_summary()
    }

    pub fn graph_events_snapshot(&self) -> Vec<GraphEvent> {
        self.graph.events_snapshot()
    }

    pub fn engine_events_snapshot(&self) -> Vec<EventRecord<EngineEvent>> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .runtime()
            .events_snapshot()
    }

    pub fn graph_version(&self) -> u64 {
        self.graph.graph_version()
    }

    pub fn execution_state(&self) -> ExecutionState {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_state()
    }

    pub fn subscribe_engine_events(&self) -> EngineSubscription {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .subscribe_engine_events()
    }

    pub fn discard_preview(&mut self) -> bool {
        self.graph.discard_preview()
    }

    pub fn mark_saved(&mut self) {
        let event_offset = self.graph.event_count();
        self.graph.mark_saved();
        self.sync_scheduler_graph_events(event_offset);
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
        let runtime = self
            .scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .runtime()
            .clone();
        runtime.evaluate(&graph, target, fidelity).await
    }

    pub async fn execute_request(
        &mut self,
        request: ExecutionRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        let graph = self.graph.snapshot();
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .request_execution(&graph, request)
            .await
    }

    pub fn await_execution(
        &self,
        execution_id: execution::ExecutionId,
    ) -> Result<ExecutionTerminalStatus, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .await_execution(execution_id)
    }

    fn sync_scheduler_graph_events(&mut self, offset: usize) {
        let graph = self.graph.snapshot();
        let graph_events: Vec<GraphChangedEvent> = self
            .graph
            .events_since(offset)
            .iter()
            .filter_map(|event| match &event.kind {
                GraphEventKind::GraphChanged(event) => Some(event.clone()),
                GraphEventKind::PreviewChanged(_) => None,
            })
            .collect();

        for event in graph_events {
            self.scheduler
                .lock()
                .expect("scheduler lock poisoned")
                .notify_graph_changed(&graph, &event);
        }
    }

    fn format_validation_report(report: &graph::model::validation::ValidationReport) -> String {
        if report.issues.is_empty() {
            return "graph validation failed".into();
        }

        report
            .issues
            .iter()
            .map(|issue| issue.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl EngineFacade for Engine {
    fn query_graph_snapshot(&self) -> Arc<Graph> {
        self.graph.snapshot()
    }

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
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .add_node(type_id)
            .map_err(|message| EngineError::Graph { message });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn remove_node(&mut self, node_id: NodeId) -> Result<(), EngineError> {
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .remove_node(node_id)
            .map_err(|message| EngineError::Graph { message });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn connect(&mut self, connection: Connection) -> Result<(), EngineError> {
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .connect(connection)
            .map_err(|error| EngineError::Graph {
                message: error.to_string(),
            });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn disconnect(&mut self, from: PinRef, to: PinRef) -> Result<(), EngineError> {
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .disconnect(from, to)
            .map_err(|error| EngineError::Graph {
                message: error.to_string(),
            });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn set_param(&mut self, node_id: NodeId, param: &str, value: Value, preview: bool) {
        let event_offset = self.graph.event_count();
        self.graph.set_param(node_id, param, value, preview);
        self.sync_scheduler_graph_events(event_offset);
    }

    fn apply_batch(&mut self, request: EditBatchRequest) -> Result<EditBatchResult, EngineError> {
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .apply_batch(request)
            .map_err(|error| EngineError::Graph {
                message: error.to_string(),
            });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn undo(&mut self) -> bool {
        let event_offset = self.graph.event_count();
        let result = self.graph.undo();
        if result {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn redo(&mut self) -> bool {
        let event_offset = self.graph.event_count();
        let result = self.graph.redo();
        if result {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn replace_graph(&mut self, graph: Graph) -> Result<(), EngineError> {
        let event_offset = self.graph.event_count();
        let result = self
            .graph
            .replace_graph(graph)
            .map_err(|report| EngineError::Graph {
                message: Self::format_validation_report(&report),
            });
        if result.is_ok() {
            self.sync_scheduler_graph_events(event_offset);
        }
        result
    }

    fn set_cooking_range(&mut self, range: CookingContextRange) {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .set_cooking_range(range);
    }

    fn query_state(&self) -> ExecutionState {
        Engine::execution_state(self)
    }

    fn subscribe_engine_events(&self) -> EngineSubscription {
        Engine::subscribe_engine_events(self)
    }

    fn query_execution_outputs(
        &self,
        execution_id: execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_execution_outputs(execution_id, node_id)
    }
}
