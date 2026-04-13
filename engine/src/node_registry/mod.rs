pub mod sources;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::capability::{CapabilityId, CapabilityRegistry};
use crate::executors::Executor;
use crate::node_manager::{NodeDef, NodeManager, ParamDef};
use types::Value;

pub struct SchemaQuery<'a> {
    pub type_id: &'a str,
    pub current_params: &'a HashMap<String, Value>,
}

#[derive(Clone)]
pub struct ResolvedSchema {
    pub params: Vec<ParamDef>,
    pub system_values: HashMap<String, Value>,
}

impl ResolvedSchema {
    pub fn empty() -> Self {
        Self {
            params: Vec::new(),
            system_values: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct SchemaError {
    pub message: String,
}

impl std::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SchemaError {}

pub trait SchemaProvider: Send + Sync {
    fn resolve_schema(&self, query: SchemaQuery<'_>) -> Result<ResolvedSchema, SchemaError>;
}

#[derive(Clone, Debug, Default)]
pub struct NodePresentation {
    pub variant: Option<String>,
}

pub trait PresentationProvider: Send + Sync {
    fn resolve_presentation(
        &self,
        type_id: &str,
        current_params: &HashMap<String, Value>,
    ) -> Result<NodePresentation, String>;
}

pub struct NodeRegistration {
    pub static_def: NodeDef,
    pub schema_provider: Option<Arc<dyn SchemaProvider>>,
    pub presentation_provider: Option<Arc<dyn PresentationProvider>>,
    pub capability_bindings: Vec<(CapabilityId, Arc<dyn Executor>)>,
}

pub trait NodeSource: Send + Sync {
    fn collect(&self) -> Vec<NodeRegistration>;
}

pub struct RegistryBundle {
    pub node_manager: NodeManager,
    pub capability_registry: CapabilityRegistry,
}

#[derive(Default)]
pub struct NodeRegistry {
    sources: Vec<Box<dyn NodeSource>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<S>(&mut self, source: S)
    where
        S: NodeSource + 'static,
    {
        self.sources.push(Box::new(source));
    }

    pub fn build(self) -> RegistryBundle {
        let mut node_manager = NodeManager::new();
        let mut capability_registry = CapabilityRegistry::new();
        let mut executor_keys = HashSet::new();
        let mut registrations = Vec::new();

        for source in self.sources {
            registrations.extend(source.collect());
        }

        for registration in &registrations {
            for (_, executor) in &registration.capability_bindings {
                let key = Arc::as_ptr(executor) as *const () as usize;
                if executor_keys.insert(key) {
                    let _ = capability_registry.register_executor(Arc::clone(executor));
                }
            }
        }
        capability_registry.freeze();

        for registration in registrations {
            node_manager.register_registration(registration, &capability_registry);
        }

        RegistryBundle {
            node_manager,
            capability_registry,
        }
    }
}
