use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::execution::{
    CancelToken, EvaluationFidelity, ExecutionOutputs, ExecutorError, ProgressSink,
};
use crate::executors::HealthStatus;
use crate::node_manager::ParamDef;
use types::Value;

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExecutionOutputs, ExecutorError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageGenerationMode {
    TextToImage,
}

impl ImageGenerationMode {
    pub fn as_param_value(self) -> &'static str {
        match self {
            ImageGenerationMode::TextToImage => "text_to_image",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text_to_image" => Some(Self::TextToImage),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

pub struct ProviderSchemaQuery<'a> {
    pub mode: ImageGenerationMode,
    pub requested_model: Option<&'a str>,
    pub current_params: &'a HashMap<String, Value>,
}

pub struct ProviderParamSchema {
    pub params: Vec<ParamDef>,
    pub schema_revision: u32,
}

pub struct ProviderExecutionRequest {
    pub model_id: String,
    pub mode: ImageGenerationMode,
    pub inputs: HashMap<String, Value>,
    pub fidelity: EvaluationFidelity,
    pub timeout_ms: Option<u64>,
    pub cancel_token: Arc<dyn CancelToken>,
    pub progress_sink: Arc<dyn ProgressSink>,
}

pub trait ImageGenerationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn models(&self, mode: ImageGenerationMode) -> Result<Vec<ModelInfo>, ExecutorError>;
    fn param_schema(
        &self,
        query: ProviderSchemaQuery<'_>,
    ) -> Result<ProviderParamSchema, ExecutorError>;
    fn execute<'a>(&'a self, req: ProviderExecutionRequest) -> ProviderFuture<'a>;
    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
