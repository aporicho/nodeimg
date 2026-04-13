use std::collections::HashMap;

use crate::node_manager::NodeDef;
use types::Value;

use super::provider::ProviderRequest;
use super::providers;
use super::registry::ProviderRegistry;
use super::types::ExecutionOutputs;

pub struct ApiExecutor {
    provider_registry: ProviderRegistry,
}

impl ApiExecutor {
    pub fn new() -> Self {
        let mut provider_registry = ProviderRegistry::new();
        for provider in providers::builtin_providers() {
            provider_registry.register(provider);
        }

        Self { provider_registry }
    }

    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.provider_registry
    }

    pub async fn execute(
        &self,
        node_def: &NodeDef,
        inputs: HashMap<String, Value>,
    ) -> Result<ExecutionOutputs, Box<dyn std::error::Error + Send + Sync>> {
        let api = node_def.api.as_ref().ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "API node '{}' is missing provider metadata",
                node_def.type_id
            ))
        })?;
        let provider = self
            .provider_registry
            .get(&api.provider_id)
            .ok_or_else(|| {
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "API provider '{}' is not registered",
                    api.provider_id
                ))
            })?;

        provider
            .execute(ProviderRequest {
                node_def,
                operation: &api.operation,
                inputs,
            })
            .await
    }
}

impl Default for ApiExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::executors::api::provider::{placeholder_execute, Provider, ProviderRequest};
    use crate::executors::api::{ProviderFuture, ProviderRegistry};
    use crate::node_manager::{ApiNodeMeta, ExecutorType, NodeDef};
    use types::Value;

    struct EchoProvider;

    impl Provider for EchoProvider {
        fn id(&self) -> &'static str {
            "echo"
        }

        fn nodes(&self) -> Vec<NodeDef> {
            vec![]
        }

        fn execute<'a>(&'a self, mut request: ProviderRequest<'a>) -> ProviderFuture<'a> {
            Box::pin(async move {
                request
                    .inputs
                    .insert("image".into(), Value::String(request.operation.to_string()));
                Ok(request.inputs)
            })
        }
    }

    #[tokio::test]
    async fn routes_request_to_matching_provider() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(EchoProvider));
        let executor = ApiExecutor {
            provider_registry: registry,
        };
        let node_def = NodeDef {
            type_id: "echo.image".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Api,
            name: "Echo Image".into(),
            category: "test".into(),
            executor_type: ExecutorType::Api,
            requires: vec![],
            purity: crate::node_manager::Purity::Impure,
            cooking_sensitivity: vec![],
            realtime_capable: false,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: Some(ApiNodeMeta {
                provider_id: "echo".into(),
                operation: "image_gen".into(),
            }),
            inputs: vec![],
            outputs: vec![],
            params: vec![],
            execute: placeholder_execute("echo.image"),
        };

        let outputs = executor.execute(&node_def, HashMap::new()).await.unwrap();

        assert!(matches!(
            outputs.get("image"),
            Some(Value::String(op)) if op == "image_gen"
        ));
    }
}
