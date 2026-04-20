pub mod artifact;
pub mod builtins;
pub mod cache;
pub mod capability;
pub mod events;
pub mod execution;
pub mod executors;
pub mod facade;
pub mod graph;
pub mod node_manager;
pub mod node_registry;
pub mod planner;
pub mod runtime;
pub mod scheduler_facade;
pub mod session;

use cache::manager::CacheManager;
use events::{EngineEvent, EventRecord, ExecutionState};
use execution::{CookingContextRange, EvaluationFidelity, ExecutionTerminalStatus};
use executors::image::{GpuExecutor, ImageExecutor};
use facade::{
    EngineError, EngineFacade, EngineResourceConfig, EngineSubscription, ExecutionRequest,
    ExecutionRequestResult,
};
use graph::model::batch::{EditBatchRequest, EditBatchResult};
use graph::model::events::{GraphChangedEvent, GraphEvent, GraphEventKind};
use graph::model::state::GraphStateSummary;
use graph::{Connection, Graph, GraphController, PinRef};
use node_manager::NodeManager;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
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
        Self::new_with_resources(gpu, EngineResourceConfig::default())
    }

    pub fn new_with_resources(gpu: Option<GpuExecutor>, resources: EngineResourceConfig) -> Self {
        let registry = {
            let mut registry = node_registry::NodeRegistry::new();
            registry.register(node_registry::sources::InventoryNodeSource);
            registry.register(node_registry::sources::GenericImageGenerationSource);
            registry.register(node_registry::sources::ApiVideoGenerationSource::new());
            registry.register(node_registry::sources::ColorAdjustSource);
            registry.register(node_registry::sources::SaveVideoSource);
            registry.build()
        };
        Self::from_registry_bundle_with_resources(registry, gpu, resources)
    }

    pub fn from_registry_bundle(
        registry: node_registry::RegistryBundle,
        gpu: Option<GpuExecutor>,
    ) -> Self {
        Self::from_registry_bundle_with_resources(registry, gpu, EngineResourceConfig::default())
    }

    pub fn from_registry_bundle_with_resources(
        registry: node_registry::RegistryBundle,
        gpu: Option<GpuExecutor>,
        resources: EngineResourceConfig,
    ) -> Self {
        let node_manager = Arc::new(registry.node_manager);
        let capability_registry = Arc::new(registry.capability_registry);
        let image_executor = ImageExecutor::new(gpu);
        let cache = Arc::new(CacheManager::new());
        let artifacts = resources
            .artifact_root
            .or_else(default_artifact_root)
            .map(|root| {
                let mut manager = crate::artifact::manager::ArtifactManager::new(root);
                let _ = manager.load_index();
                manager
            })
            .map(|manager| Arc::new(Mutex::new(manager)));
        let scheduler = Arc::new(Mutex::new(scheduler_facade::SchedulerFacade::new(
            runtime::Runtime::new(
                Arc::clone(&node_manager),
                Arc::clone(&capability_registry),
                image_executor,
                Arc::clone(&cache),
                artifacts.clone(),
            ),
            cache,
            artifacts,
        )));
        scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .bind_handle(&scheduler);

        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            scheduler,
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

    pub fn query_artifact_history(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Vec<crate::artifact::model::ArtifactRecord>, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_artifact_history(node_id, output_key)
    }

    pub fn query_selected_artifact(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Option<crate::artifact::model::ArtifactRecord>, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_selected_artifact(node_id, output_key)
    }

    pub fn select_artifact_version(
        &mut self,
        node_id: NodeId,
        output_key: &str,
        artifact_id: &str,
    ) -> Result<(), EngineError> {
        let record = self
            .query_artifact_history(node_id, output_key)?
            .into_iter()
            .find(|record| record.artifact_id == artifact_id)
            .ok_or_else(|| EngineError::Artifact {
                message: format!("artifact not found: {artifact_id}"),
            })?;
        self.apply_artifact_params_snapshot(node_id, &record.params_snapshot)?;
        let graph = self.graph.snapshot();
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .select_artifact_version(&graph, node_id, output_key, artifact_id)
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
    ) -> Result<ExecutionRequestResult, EngineError> {
        let graph = self.graph.snapshot();
        Ok(self
            .scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .request_execution(&graph, request)?)
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

    pub fn cancel_execution(
        &self,
        execution_id: execution::ExecutionId,
    ) -> Result<(), EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .cancel_execution(execution_id)
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

    fn apply_artifact_params_snapshot(
        &mut self,
        node_id: NodeId,
        params_snapshot: &crate::artifact::model::ArtifactParamsSnapshot,
    ) -> Result<(), EngineError> {
        let graph = self.graph.snapshot();
        let node = graph
            .nodes
            .get(&node_id)
            .ok_or_else(|| EngineError::Graph {
                message: format!("Node {:?} not found", node_id),
            })?;
        let param_defs = self
            .node_manager
            .get_node_def(&node.type_id)
            .ok_or_else(|| EngineError::Graph {
                message: format!("Node type '{}' not registered", node.type_id),
            })?
            .params
            .iter()
            .map(|param| (param.name.clone(), param.data_type.clone()))
            .collect::<Vec<_>>();

        for (param_name, data_type) in param_defs {
            let Some(raw_value) = params_snapshot.get(&param_name) else {
                continue;
            };
            let value = parse_param_snapshot_value(&data_type, raw_value)?;
            self.set_param(node_id, &param_name, value, false);
        }

        Ok(())
    }
}

fn parse_param_snapshot_value(
    data_type: &types::DataType,
    raw: &str,
) -> Result<Value, EngineError> {
    if *data_type == types::DataType::string() {
        return Ok(Value::String(raw.to_owned()));
    }
    if *data_type == types::DataType::int() {
        return raw
            .parse::<i64>()
            .map(Value::Int)
            .map_err(|error| EngineError::Artifact {
                message: format!("failed to parse int parameter snapshot '{raw}': {error}"),
            });
    }
    if *data_type == types::DataType::float() {
        return raw
            .parse::<f32>()
            .map(Value::Float)
            .map_err(|error| EngineError::Artifact {
                message: format!("failed to parse float parameter snapshot '{raw}': {error}"),
            });
    }
    if *data_type == types::DataType::bool() {
        return raw
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|error| EngineError::Artifact {
                message: format!("failed to parse bool parameter snapshot '{raw}': {error}"),
            });
    }

    Err(EngineError::Artifact {
        message: format!(
            "parameter snapshot restore does not support data type '{}'",
            data_type
        ),
    })
}

fn default_artifact_root() -> Option<PathBuf> {
    static NEXT_ID: AtomicU64 = AtomicU64::new(1);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let seq = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    Some(std::env::temp_dir().join(format!("nodeimg-engine-artifacts-{unique}-{seq}")))
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

    fn cancel_execution(&self, execution_id: execution::ExecutionId) -> Result<(), EngineError> {
        Engine::cancel_execution(self, execution_id)
    }

    fn subscribe_engine_events(&self) -> EngineSubscription {
        Engine::subscribe_engine_events(self)
    }

    fn query_artifact_history(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Vec<crate::artifact::model::ArtifactRecord>, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_artifact_history(node_id, output_key)
    }

    fn query_selected_artifact(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<Option<crate::artifact::model::ArtifactRecord>, EngineError> {
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .query_selected_artifact(node_id, output_key)
    }

    fn select_artifact_version(
        &mut self,
        node_id: NodeId,
        output_key: &str,
        artifact_id: &str,
    ) -> Result<(), EngineError> {
        let graph = self.graph.snapshot();
        self.scheduler
            .lock()
            .expect("scheduler lock poisoned")
            .select_artifact_version(&graph, node_id, output_key, artifact_id)
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
