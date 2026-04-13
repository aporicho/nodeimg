use std::collections::HashMap;
use std::sync::Arc;

use super::provider::Provider;

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(provider_id).cloned()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executors::api::provider::{placeholder_execute, ProviderRequest};
    use crate::executors::api::ProviderFuture;
    use crate::node_manager::NodeDef;

    struct TestProvider;

    impl Provider for TestProvider {
        fn id(&self) -> &'static str {
            "test"
        }

        fn nodes(&self) -> Vec<NodeDef> {
            vec![NodeDef {
                type_id: "test.node".into(),
                version: 1,
                source: crate::node_manager::NodeSourceKind::Api,
                name: "Test Node".into(),
                category: "test".into(),
                requires: vec!["test.run".into()],
                purity: crate::node_manager::Purity::Impure,
                cooking_sensitivity: vec![],
                realtime_capable: false,
                execution: crate::node_manager::ExecutionPolicy::default(),
                api: Some(crate::node_manager::ApiNodeMeta {
                    provider_id: "test".into(),
                    operation: "run".into(),
                }),
                inputs: vec![],
                outputs: vec![],
                params: vec![],
                execute: placeholder_execute("test.node"),
            }]
        }

        fn execute<'a>(&'a self, _request: ProviderRequest<'a>) -> ProviderFuture<'a> {
            Box::pin(async move { Ok(Default::default()) })
        }
    }

    #[test]
    fn registers_and_resolves_provider() {
        let mut registry = ProviderRegistry::new();
        registry.register(Arc::new(TestProvider));

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test").is_some());
        assert!(registry.get("missing").is_none());
    }
}
