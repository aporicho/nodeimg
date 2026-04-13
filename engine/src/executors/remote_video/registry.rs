use std::collections::HashMap;
use std::sync::Arc;

use crate::execution::ExecutorError;

use super::provider::{ModelInfo, VideoGenerationMode, VideoGenerationProvider};

#[derive(Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn VideoGenerationProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn VideoGenerationProvider>) {
        self.providers.insert(provider.id().to_string(), provider);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn VideoGenerationProvider>> {
        self.providers.get(provider_id).cloned()
    }

    pub fn provider_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.providers.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub fn models(
        &self,
        provider_id: &str,
        mode: VideoGenerationMode,
    ) -> Result<Vec<ModelInfo>, ExecutorError> {
        let provider = self
            .get(provider_id)
            .ok_or_else(|| ExecutorError::Unavailable {
                message: format!("provider '{provider_id}' is not registered"),
            })?;
        provider.models(mode)
    }
}
