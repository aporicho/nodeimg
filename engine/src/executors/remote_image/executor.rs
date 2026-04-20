use std::sync::Arc;

use crate::capability::{Capability, CapabilityId, SideEffect};
use crate::execution::{ExecutorError, NodeExecutionRequest};
use crate::executors::{Executor, ExecutorFuture, LocalityProfile};
use types::Value;

use super::provider::{ImageGenerationMode, ProviderExecutionRequest};
use super::registry::ProviderRegistry;

pub struct RemoteImageGenerationExecutor {
    providers: Arc<ProviderRegistry>,
}

impl RemoteImageGenerationExecutor {
    pub fn new(providers: Arc<ProviderRegistry>) -> Self {
        Self { providers }
    }
}

impl Executor for RemoteImageGenerationExecutor {
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
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            if cap_id != "image.generate" {
                return Err(ExecutorError::Unavailable {
                    message: format!("unsupported capability '{cap_id}'"),
                });
            }

            let provider_id = required_string(&req.inputs, "__resolved_provider")
                .or_else(|_| required_string(&req.inputs, "provider"))?;
            let provider =
                self.providers
                    .get(&provider_id)
                    .ok_or_else(|| ExecutorError::Unavailable {
                        message: format!("provider '{provider_id}' is not registered"),
                    })?;
            let model_id = required_string(&req.inputs, "__resolved_model")
                .or_else(|_| required_string(&req.inputs, "model"))?;
            let mode = required_string(&req.inputs, "mode")
                .ok()
                .and_then(|value| ImageGenerationMode::parse(&value))
                .unwrap_or(ImageGenerationMode::TextToImage);

            provider
                .execute(ProviderExecutionRequest {
                    model_id,
                    mode,
                    inputs: req.inputs,
                    fidelity: req.fidelity,
                    timeout_ms: req.timeout_ms,
                    cancel_token: req.cancel_token,
                    progress_sink: req.progress_sink,
                })
                .await
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Remote
    }
}

fn required_string(
    inputs: &std::collections::HashMap<String, Value>,
    key: &str,
) -> Result<String, ExecutorError> {
    match inputs.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(ExecutorError::InvalidInput {
            message: format!("missing required string parameter '{key}'"),
        }),
    }
}
