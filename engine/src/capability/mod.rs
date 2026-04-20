use std::collections::HashMap;
use std::sync::Arc;

use crate::executors::{Executor, HealthStatus};

pub type CapabilityId = String;
pub type CapabilityVersion = u32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub id: CapabilityId,
    pub version: CapabilityVersion,
    pub input_types: Vec<types::DataType>,
    pub output_types: Vec<types::DataType>,
    pub side_effects: Vec<SideEffect>,
}

impl Capability {
    pub fn new(
        id: impl Into<String>,
        input_types: Vec<types::DataType>,
        output_types: Vec<types::DataType>,
        side_effects: Vec<SideEffect>,
    ) -> Self {
        Self {
            id: id.into(),
            version: 0,
            input_types,
            output_types,
            side_effects,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SideEffect {
    None,
    FileRead(String),
    FileWrite(String),
    NetworkCall(String),
    PythonProcess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalityHint {
    PreferGpu,
    PreferCpu,
    PreferRemote,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityRegistryError {
    DuplicateCapabilityVersion,
    Frozen,
}

pub struct CapabilityRegistry {
    capabilities: HashMap<CapabilityId, Capability>,
    providers: HashMap<CapabilityId, Vec<Arc<dyn Executor>>>,
    frozen: bool,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: HashMap::new(),
            providers: HashMap::new(),
            frozen: false,
        }
    }

    pub fn register_executor(
        &mut self,
        executor: Arc<dyn Executor>,
    ) -> Result<(), CapabilityRegistryError> {
        if self.frozen {
            return Err(CapabilityRegistryError::Frozen);
        }

        for capability in executor.provides() {
            if let Some(existing) = self.capabilities.get(&capability.id) {
                if existing.version != capability.version {
                    return Err(CapabilityRegistryError::DuplicateCapabilityVersion);
                }
            } else {
                self.capabilities
                    .insert(capability.id.clone(), capability.clone());
            }

            self.providers
                .entry(capability.id.clone())
                .or_default()
                .push(Arc::clone(&executor));
        }

        Ok(())
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn get_capability(&self, id: &CapabilityId) -> Option<&Capability> {
        self.capabilities.get(id)
    }

    pub fn capability_version(&self, id: &CapabilityId) -> Option<CapabilityVersion> {
        self.capabilities.get(id).map(|cap| cap.version)
    }

    pub fn providers_of(&self, id: &CapabilityId) -> &[Arc<dyn Executor>] {
        self.providers.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn route(&self, id: &CapabilityId, locality: LocalityHint) -> Option<Arc<dyn Executor>> {
        self.providers
            .get(id)?
            .iter()
            .filter(|executor| executor.health() == HealthStatus::Healthy)
            .find(|executor| matches_locality(executor.as_ref(), locality))
            .cloned()
    }

    pub fn route_requirements(
        &self,
        ids: &[CapabilityId],
        locality: LocalityHint,
    ) -> Option<Arc<dyn Executor>> {
        if ids.is_empty() {
            return None;
        }

        let mut sorted = ids.to_vec();
        sorted.sort();
        sorted.dedup();

        let first = self.providers.get(sorted.first()?)?;
        first
            .iter()
            .filter(|executor| executor.health() == HealthStatus::Healthy)
            .filter(|executor| matches_locality(executor.as_ref(), locality))
            .find(|executor| {
                let ptr = Arc::as_ptr(executor) as *const ();
                sorted.iter().all(|capability| {
                    self.providers.get(capability).is_some_and(|providers| {
                        providers
                            .iter()
                            .any(|candidate| Arc::as_ptr(candidate) as *const () == ptr)
                    })
                })
            })
            .cloned()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn matches_locality(executor: &dyn Executor, hint: LocalityHint) -> bool {
    use crate::executors::LocalityProfile;

    match (hint, executor.locality_profile()) {
        (_, LocalityProfile::Any) => true,
        (LocalityHint::PreferGpu, LocalityProfile::Gpu) => true,
        (LocalityHint::PreferCpu, LocalityProfile::Cpu) => true,
        (LocalityHint::PreferRemote, LocalityProfile::Remote) => true,
        (LocalityHint::Any, _) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use crate::execution::NodeExecutionRequest;
    use crate::executors::{ExecutorFuture, LocalityProfile};

    struct TestExecutor;

    impl Executor for TestExecutor {
        fn provides(&self) -> Vec<Capability> {
            vec![Capability::new(
                "image.generate",
                vec![types::DataType::string()],
                vec![types::DataType::image()],
                vec![SideEffect::NetworkCall("*".into())],
            )]
        }

        fn execute<'a>(
            &'a self,
            _cap_id: &'a CapabilityId,
            _req: NodeExecutionRequest<'a>,
        ) -> ExecutorFuture<'a> {
            Box::pin(async move { Ok(crate::execution::ExecutionOutputs::full(HashMap::new())) })
        }

        fn locality_profile(&self) -> LocalityProfile {
            LocalityProfile::Remote
        }
    }

    #[test]
    fn registers_and_routes_capability() {
        let mut registry = CapabilityRegistry::new();
        registry.register_executor(Arc::new(TestExecutor)).unwrap();
        registry.freeze();

        assert_eq!(
            registry.capability_version(&"image.generate".into()),
            Some(0)
        );
        assert!(registry
            .route(&"image.generate".into(), LocalityHint::PreferRemote)
            .is_some());
    }
}
