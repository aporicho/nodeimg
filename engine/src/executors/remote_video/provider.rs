use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use types::Value;

use crate::execution::ExecutorError;
use crate::executors::HealthStatus;
use crate::node_manager::ParamDef;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoGenerationMode {
    TextToVideo,
}

impl VideoGenerationMode {
    pub fn as_param_value(self) -> &'static str {
        match self {
            VideoGenerationMode::TextToVideo => "text_to_video",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

#[derive(Clone)]
pub struct ProviderSchemaQuery {
    pub mode: VideoGenerationMode,
    pub requested_model: Option<String>,
}

pub struct ProviderParamSchema {
    pub params: Vec<ParamDef>,
    pub schema_revision: u32,
}

pub struct VideoGenerationRequest {
    pub model_id: String,
    pub prompt: String,
    pub inputs: HashMap<String, Value>,
}

#[derive(Clone)]
pub struct GeneratedVideoAsset {
    pub video_path: PathBuf,
    pub fps: Option<u32>,
}

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<GeneratedVideoAsset, ExecutorError>> + Send + 'a>>;

pub trait VideoGenerationProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn models(&self, mode: VideoGenerationMode) -> Result<Vec<ModelInfo>, ExecutorError>;
    fn param_schema(
        &self,
        query: ProviderSchemaQuery,
    ) -> Result<ProviderParamSchema, ExecutorError>;
    fn generate<'a>(
        &'a self,
        req: VideoGenerationRequest,
        output_path: PathBuf,
    ) -> ProviderFuture<'a>;
    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
