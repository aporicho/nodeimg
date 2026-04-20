use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use image::DynamicImage;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use types::{Constraint, Value};
use uuid::Uuid;

use crate::execution::{ExecutionOutputs, ExecutorError};
use crate::executors::HealthStatus;
use crate::node_manager::{ParamDef, ParamExpose};

use super::super::credential_store::CredentialStore;
use super::super::provider::{
    ImageGenerationMode, ImageGenerationProvider, ModelInfo, ProviderExecutionRequest,
    ProviderFuture, ProviderParamSchema, ProviderSchemaQuery,
};

const API_BASE: &str = "https://api.liblib.tv";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const POLL_TIMEOUT: Duration = Duration::from_secs(120);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";

#[derive(Clone, Copy)]
struct ImageModelSpec {
    id: &'static str,
    provider: &'static str,
    default_quality: &'static str,
    default_ratio: &'static str,
}

const IMAGE_MODELS: &[ImageModelSpec] = &[
    ImageModelSpec {
        id: "nebula-ultra",
        provider: "nebula",
        default_quality: "2K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "nebula-2-flash",
        provider: "nebula",
        default_quality: "2K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "nebula-core",
        provider: "nebula",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "seedream-5",
        provider: "Seedream",
        default_quality: "2K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "seedream-4.5",
        provider: "Seedream",
        default_quality: "2K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "seedream-4",
        provider: "Seedream",
        default_quality: "2K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "mj-v7",
        provider: "Midjourney",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "mj-niji7",
        provider: "Midjourney",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "z-image",
        provider: "zimage",
        default_quality: "1K",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "qwen",
        provider: "qwen",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "qwen-edit",
        provider: "qwen",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "flux-2-pro",
        provider: "flux",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "flux-2-flex",
        provider: "flux",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "flux-1",
        provider: "flux",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "image-editor",
        provider: "kontext",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "image-editor-pro",
        provider: "kontext",
        default_quality: "auto",
        default_ratio: "16:9",
    },
    ImageModelSpec {
        id: "orbit-2-image",
        provider: "orbit",
        default_quality: "auto",
        default_ratio: "auto",
    },
];

#[derive(Deserialize)]
struct LibTvEnvelope<T> {
    code: i32,
    msg: Option<String>,
    data: Option<T>,
}

#[derive(Deserialize)]
struct CreateTaskData {
    #[serde(rename = "taskId")]
    task_id: String,
}

#[derive(Deserialize)]
struct ProgressData {
    progresses: Vec<TaskProgress>,
}

#[derive(Deserialize)]
struct TaskProgress {
    status: i32,
    #[serde(rename = "taskResult")]
    task_result: Option<String>,
}

#[derive(Deserialize)]
struct TaskResult {
    images: Vec<TaskImage>,
}

#[derive(Deserialize)]
struct TaskImage {
    #[serde(rename = "previewPath")]
    preview_path: String,
}

pub struct LibTvProvider {
    client: reqwest::Client,
    credential_store: Arc<dyn CredentialStore>,
}

impl LibTvProvider {
    pub fn new(credential_store: Arc<dyn CredentialStore>) -> Self {
        Self {
            client: reqwest::Client::new(),
            credential_store,
        }
    }

    async fn execute_image_generation(
        &self,
        req: ProviderExecutionRequest,
    ) -> Result<ExecutionOutputs, ExecutorError> {
        let prompt = required_string(&req.inputs, "prompt")?;
        let spec = lookup_model(&req.model_id).ok_or_else(|| ExecutorError::InvalidInput {
            message: format!("unsupported LibTV image model '{}'", req.model_id),
        })?;
        let quality = resolve_choice(
            optional_string(&req.inputs, "quality"),
            spec.default_quality,
        );
        let ratio = resolve_choice(optional_string(&req.inputs, "ratio"), spec.default_ratio);
        let headers = auth_headers(self.credential_store.as_ref())?;
        let project_id = self
            .credential_store
            .get_secret(self.id(), "project_id")
            .unwrap_or_default();

        req.progress_sink
            .report(crate::execution::ExecutorProgressEvent::Message {
                text: format!(
                    "Submitting image generation request to {}",
                    self.display_name()
                ),
            });

        let create_body = json!({
            "params": {
                "prompt": prompt,
                "model": spec.id,
                "count": 1,
                "modeType": "text2image",
                "quality": quality,
                "ratio": ratio,
                "textList": [],
                "imageList": [],
                "videoList": [],
                "audioList": [],
                "infiniteSwitch": 0,
            },
            "metadata": {
                "node_id": Uuid::new_v4().to_string(),
                "project_id": project_id,
            },
            "provider": spec.provider,
            "model": spec.id,
            "taskType": "image",
            "requestId": Uuid::new_v4().to_string(),
        });

        let create_response = self
            .client
            .post(format!("{API_BASE}/api/task/generation/create"))
            .headers(headers.clone())
            .json(&create_body)
            .send()
            .await
            .map_err(http_error)?
            .error_for_status()
            .map_err(http_error)?;
        let create_payload: LibTvEnvelope<CreateTaskData> =
            create_response.json().await.map_err(http_error)?;
        if is_auth_error(create_payload.code, create_payload.msg.as_deref()) {
            return Err(ExecutorError::Unavailable {
                message: "LibTV credentials are missing or expired".into(),
            });
        }
        if create_payload.code != 0 {
            return Err(ExecutorError::RuntimeFailed {
                message: format!(
                    "LibTV create task failed: {}",
                    create_payload.msg.unwrap_or_else(|| "unknown error".into())
                ),
            });
        }
        let task_id = create_payload
            .data
            .ok_or_else(|| ExecutorError::RuntimeFailed {
                message: "LibTV create task returned empty payload".into(),
            })?
            .task_id;

        let deadline = tokio::time::Instant::now() + POLL_TIMEOUT;
        let mut poll_attempts = 0;
        loop {
            if req.cancel_token.is_cancelled() {
                return Err(ExecutorError::Cancelled);
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(ExecutorError::Timeout);
            }

            poll_attempts += 1;
            req.progress_sink
                .report(crate::execution::ExecutorProgressEvent::Fraction {
                    current: poll_attempts,
                    total: 40,
                });

            let progress_response = self
                .client
                .post(format!("{API_BASE}/api/task/generation/progress"))
                .headers(headers.clone())
                .json(&json!({ "taskIds": [task_id.clone()] }))
                .send()
                .await
                .map_err(http_error)?
                .error_for_status()
                .map_err(http_error)?;
            let progress_payload: LibTvEnvelope<ProgressData> =
                progress_response.json().await.map_err(http_error)?;

            if is_auth_error(progress_payload.code, progress_payload.msg.as_deref()) {
                return Err(ExecutorError::Unavailable {
                    message: "LibTV credentials are missing or expired".into(),
                });
            }
            if progress_payload.code != 0 {
                return Err(ExecutorError::RuntimeFailed {
                    message: format!(
                        "LibTV progress poll failed: {}",
                        progress_payload
                            .msg
                            .unwrap_or_else(|| "unknown error".into())
                    ),
                });
            }

            let progress = progress_payload
                .data
                .and_then(|data| data.progresses.into_iter().next())
                .ok_or_else(|| ExecutorError::RuntimeFailed {
                    message: "LibTV progress payload was empty".into(),
                })?;

            if progress.status == 2 {
                let task_result: TaskResult =
                    serde_json::from_str(progress.task_result.as_deref().ok_or_else(|| {
                        ExecutorError::RuntimeFailed {
                            message: "LibTV completed task without taskResult".into(),
                        }
                    })?)
                    .map_err(http_error)?;
                let image_url = task_result
                    .images
                    .into_iter()
                    .next()
                    .ok_or_else(|| ExecutorError::RuntimeFailed {
                        message: "LibTV completed task without images".into(),
                    })?
                    .preview_path;

                let image = self.download_image(&image_url).await?;
                return Ok(ExecutionOutputs::full(HashMap::from([(
                    String::from("image"),
                    Value::Image(types::Image::from_cpu(image)),
                )])));
            }

            if progress.status >= 3 {
                return Err(ExecutorError::RuntimeFailed {
                    message: format!(
                        "LibTV image generation failed with status {}",
                        progress.status
                    ),
                });
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn download_image(&self, image_url: &str) -> Result<DynamicImage, ExecutorError> {
        let response = self
            .client
            .get(image_url)
            .send()
            .await
            .map_err(http_error)?
            .error_for_status()
            .map_err(http_error)?;
        let bytes = response.bytes().await.map_err(http_error)?;
        image::load_from_memory(bytes.as_ref()).map_err(http_error)
    }
}

impl ImageGenerationProvider for LibTvProvider {
    fn id(&self) -> &'static str {
        "libtv"
    }

    fn display_name(&self) -> &'static str {
        "LibTV"
    }

    fn models(&self, mode: ImageGenerationMode) -> Result<Vec<ModelInfo>, ExecutorError> {
        match mode {
            ImageGenerationMode::TextToImage => Ok(IMAGE_MODELS
                .iter()
                .map(|spec| ModelInfo {
                    id: spec.id.to_string(),
                    display_name: spec.id.to_string(),
                })
                .collect()),
        }
    }

    fn param_schema(
        &self,
        query: ProviderSchemaQuery<'_>,
    ) -> Result<ProviderParamSchema, ExecutorError> {
        match query.mode {
            ImageGenerationMode::TextToImage => {
                let model_id = query.requested_model.unwrap_or("nebula-ultra");
                let spec = lookup_model(model_id).ok_or_else(|| ExecutorError::InvalidInput {
                    message: format!("unsupported LibTV image model '{model_id}'"),
                })?;

                Ok(ProviderParamSchema {
                    params: vec![
                        ParamDef {
                            name: "quality".into(),
                            data_type: types::DataType::string(),
                            constraint: Some(Constraint::enum_options(vec![
                                "default".into(),
                                "auto".into(),
                                "1K".into(),
                                "2K".into(),
                                "3K".into(),
                                "4K".into(),
                            ])),
                            default_value: Value::String(spec.default_quality.into()),
                            expose: vec![ParamExpose::Control],
                        },
                        ParamDef {
                            name: "ratio".into(),
                            data_type: types::DataType::string(),
                            constraint: Some(Constraint::enum_options(vec![
                                "default".into(),
                                "auto".into(),
                                "1:1".into(),
                                "9:16".into(),
                                "16:9".into(),
                                "3:4".into(),
                                "4:3".into(),
                                "3:2".into(),
                                "2:3".into(),
                                "4:5".into(),
                                "5:4".into(),
                                "21:9".into(),
                                "8:1".into(),
                                "1:8".into(),
                                "4:1".into(),
                                "1:4".into(),
                            ])),
                            default_value: Value::String(spec.default_ratio.into()),
                            expose: vec![ParamExpose::Control],
                        },
                    ],
                    schema_revision: 1,
                })
            }
        }
    }

    fn execute<'a>(&'a self, req: ProviderExecutionRequest) -> ProviderFuture<'a> {
        Box::pin(async move {
            match req.mode {
                ImageGenerationMode::TextToImage => self.execute_image_generation(req).await,
            }
        })
    }

    fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

fn required_string(inputs: &HashMap<String, Value>, key: &str) -> Result<String, ExecutorError> {
    let value = optional_string(inputs, key).ok_or_else(|| ExecutorError::InvalidInput {
        message: format!("missing required string input '{key}'"),
    })?;
    if value.trim().is_empty() {
        return Err(ExecutorError::InvalidInput {
            message: format!("input '{key}' must not be empty"),
        });
    }
    Ok(value)
}

fn optional_string(inputs: &HashMap<String, Value>, key: &str) -> Option<String> {
    match inputs.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn resolve_choice(value: Option<String>, default: &'static str) -> String {
    match value.as_deref() {
        Some("") | Some("default") | None => default.to_string(),
        Some(value) => value.to_string(),
    }
}

fn lookup_model(model: &str) -> Option<&'static ImageModelSpec> {
    IMAGE_MODELS.iter().find(|spec| spec.id == model)
}

fn auth_headers(credential_store: &dyn CredentialStore) -> Result<HeaderMap, ExecutorError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "token",
        HeaderValue::from_str(&required_credential(credential_store, "libtv", "token")?)
            .map_err(http_error)?,
    );
    headers.insert(
        "webid",
        HeaderValue::from_str(&required_credential(credential_store, "libtv", "webid")?)
            .map_err(http_error)?,
    );
    headers.insert("x-language", HeaderValue::from_static("zh"));
    headers.insert("origin", HeaderValue::from_static("https://www.liblib.tv"));
    headers.insert(
        "referer",
        HeaderValue::from_static("https://www.liblib.tv/"),
    );
    headers.insert("user-agent", HeaderValue::from_static(USER_AGENT));
    Ok(headers)
}

fn required_credential(
    credential_store: &dyn CredentialStore,
    provider_id: &str,
    key: &str,
) -> Result<String, ExecutorError> {
    credential_store
        .get_secret(provider_id, key)
        .ok_or_else(|| ExecutorError::Unavailable {
            message: format!("missing credential '{key}' for provider '{provider_id}'"),
        })
}

fn is_auth_error(code: i32, msg: Option<&str>) -> bool {
    matches!(code, 10001 | 10401 | 10403 | 401 | 403)
        || msg.is_some_and(|message| {
            let message = message.to_ascii_lowercase();
            message.contains("token") || message.contains("unauthorized")
        })
}

fn http_error(error: impl std::fmt::Display) -> ExecutorError {
    ExecutorError::RuntimeFailed {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyStore;

    impl CredentialStore for EmptyStore {
        fn get_secret(&self, _provider_id: &str, _key: &str) -> Option<String> {
            None
        }
    }

    #[test]
    fn exposes_models_for_text_to_image() {
        let provider = LibTvProvider::new(Arc::new(EmptyStore));
        let models = provider.models(ImageGenerationMode::TextToImage).unwrap();

        assert!(models.iter().any(|model| model.id == "nebula-ultra"));
    }

    #[test]
    fn resolves_default_quality_and_ratio_from_model() {
        let spec = lookup_model("nebula-ultra").unwrap();

        assert_eq!(
            resolve_choice(Some("default".into()), spec.default_quality),
            "2K"
        );
        assert_eq!(resolve_choice(None, spec.default_ratio), "16:9");
    }
}
