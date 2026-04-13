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
pub mod runtime;
pub mod scheduler_facade;

use capability::CapabilityRegistry;
use execution::{CookingContextRange, EvaluationFidelity};
use executors::image::{GpuExecutor, ImageExecutor};
use facade::{EngineError, EngineFacade, ExecuteRequest, ExecutionTicket};
use graph::GraphController;
use node_manager::NodeManager;
use std::collections::HashMap;
use std::sync::Arc;
use types::{NodeId, Value};

/// Engine：顶层门面与装配根。
pub struct Engine {
    pub graph: GraphController,
    pub node_manager: Arc<NodeManager>,
    pub capability_registry: Arc<CapabilityRegistry>,
    pub scheduler: scheduler_facade::SchedulerFacade,
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

        Self {
            graph: GraphController::new(Arc::clone(&node_manager), 50),
            scheduler: scheduler_facade::SchedulerFacade::new(runtime::Runtime::new(
                Arc::clone(&node_manager),
                Arc::clone(&capability_registry),
                image_executor,
                cache::manager::CacheManager::new(),
            )),
            node_manager,
            capability_registry,
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
        self.scheduler.runtime().evaluate(&graph, target, fidelity).await
    }

    pub async fn execute_request(
        &mut self,
        request: ExecuteRequest,
    ) -> Result<ExecutionTicket, EngineError> {
        let graph = self.graph.snapshot();
        self.scheduler.execute_request(&graph, request).await
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
        let result = self
            .graph
            .add_node(type_id)
            .map_err(|message| EngineError::Graph { message });
        if let Ok(node_id) = result {
            self.scheduler.mark_dirty(node_id);
            return Ok(node_id);
        }
        result
    }

    fn set_param(&mut self, node_id: NodeId, param: &str, value: Value, preview: bool) {
        self.graph.set_param(node_id, param, value, preview);
        self.scheduler.mark_dirty(node_id);
    }

    fn set_cooking_range(&mut self, range: CookingContextRange) {
        self.scheduler.set_cooking_range(range);
    }

    fn get_execution_outputs(
        &self,
        execution_id: execution::ExecutionId,
        node_id: NodeId,
    ) -> Result<HashMap<String, Value>, EngineError> {
        self.scheduler.get_execution_outputs(execution_id, node_id)
    }
}
