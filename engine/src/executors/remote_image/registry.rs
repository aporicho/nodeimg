use std::collections::HashMap;
use std::sync::Arc;

use crate::execution::ExecutorError;

use super::provider::{ImageGenerationMode, ImageGenerationProvider, ModelInfo};

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn ImageGenerationProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn ImageGenerationProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ImageGenerationProvider>> {
        self.providers.get(provider_id).cloned()
    }

    pub fn provider_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.providers.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn first_provider_id(&self) -> Option<String> {
        self.provider_ids().into_iter().next()
    }

    pub fn models(
        &self,
        provider_id: &str,
        mode: ImageGenerationMode,
    ) -> Result<Vec<ModelInfo>, ExecutorError> {
        let provider = self
            .get(provider_id)
            .ok_or_else(|| ExecutorError::Unavailable {
                message: format!("provider '{provider_id}' is not registered"),
            })?;
        provider.models(mode)
    }
}
