use std::collections::HashMap;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::execution::ExecutorError;
use crate::executors::remote_image::credential_store::CredentialStore;
use crate::node_manager::{ParamDef, ParamExpose};
use types::{Constraint, Value};

use super::super::provider::{
    GeneratedVideoAsset, ModelInfo, ProviderFuture, ProviderParamSchema, ProviderSchemaQuery,
    VideoGenerationMode, VideoGenerationProvider, VideoGenerationRequest,
};

const API_BASE: &str = "https://api.liblib.tv";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const POLL_TIMEOUT: Duration = Duration::from_secs(600);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";

#[derive(Clone, Copy)]
struct VideoModelSpec {
    id: &'static str,
    provider: &'static str,
    default_duration: u32,
    default_resolution: &'static str,
    default_ratio: &'static str,
    quality_kind: Option<&'static [&'static str]>,
}

const KLING_QUALITY: &[&str] = &["low", "high"];

const VIDEO_MODELS: &[VideoModelSpec] = &[
    VideoModelSpec {
        id: "star-video2-fast",
        provider: "star-video2-fast",
        default_duration: 5,
        default_resolution: "720p",
        default_ratio: "16:9",
        quality_kind: None,
    },
    VideoModelSpec {
        id: "kling-video-o3",
        provider: "Kling",
        default_duration: 5,
        default_resolution: "auto",
        default_ratio: "16:9",
        quality_kind: Some(KLING_QUALITY),
    },
    VideoModelSpec {
        id: "seedance2.0",
        provider: "seedance2.0",
        default_duration: 5,
        default_resolution: "720p",
        default_ratio: "16:9",
        quality_kind: None,
    },
    VideoModelSpec {
        id: "viduq3-pro",
        provider: "vidu",
        default_duration: 8,
        default_resolution: "720p",
        default_ratio: "16:9",
        quality_kind: None,
    },
    VideoModelSpec {
        id: "pixverse-v5.5",
        provider: "Pixverse",
        default_duration: 5,
        default_resolution: "720P",
        default_ratio: "16:9",
        quality_kind: None,
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
    videos: Vec<TaskVideo>,
}

#[derive(Deserialize)]
struct TaskVideo {
    #[serde(rename = "previewPath")]
    preview_path: String,
}

pub struct LibTvVideoProvider {
    client: Client,
    credential_store: std::sync::Arc<dyn CredentialStore>,
}

impl LibTvVideoProvider {
    pub fn new(credential_store: std::sync::Arc<dyn CredentialStore>) -> Self {
        Self {
            client: Client::new(),
            credential_store,
        }
    }
}

impl VideoGenerationProvider for LibTvVideoProvider {
    fn id(&self) -> &'static str {
        "libtv"
    }

    fn display_name(&self) -> &'static str {
        "LibTV"
    }

    fn models(&self, mode: VideoGenerationMode) -> Result<Vec<ModelInfo>, ExecutorError> {
        match mode {
            VideoGenerationMode::TextToVideo => Ok(VIDEO_MODELS
                .iter()
                .map(|model| ModelInfo {
                    id: model.id.to_string(),
                    display_name: model.id.to_string(),
                })
                .collect()),
        }
    }

    fn param_schema(
        &self,
        query: ProviderSchemaQuery,
    ) -> Result<ProviderParamSchema, ExecutorError> {
        let model = query
            .requested_model
            .as_deref()
            .and_then(find_model)
            .unwrap_or(&VIDEO_MODELS[0]);
        let mut params = vec![
            ParamDef {
                name: "resolution".into(),
                data_type: types::DataType::string(),
                constraint: Some(Constraint::enum_options(vec![
                    "auto".into(),
                    "480p".into(),
                    "540p".into(),
                    "720p".into(),
                    "1080p".into(),
                    "4K".into(),
                ])),
                default_value: Value::String(model.default_resolution.into()),
                expose: vec![ParamExpose::Control],
            },
            ParamDef {
                name: "ratio".into(),
                data_type: types::DataType::string(),
                constraint: Some(Constraint::enum_options(vec![
                    "auto".into(),
                    "adaptive".into(),
                    "16:9".into(),
                    "9:16".into(),
                    "1:1".into(),
                    "4:3".into(),
                    "3:4".into(),
                    "21:9".into(),
                ])),
                default_value: Value::String(model.default_ratio.into()),
                expose: vec![ParamExpose::Control],
            },
        ];
        if let Some(quality_values) = model.quality_kind {
            params.push(ParamDef {
                name: "quality".into(),
                data_type: types::DataType::string(),
                constraint: Some(Constraint::enum_options(
                    quality_values
                        .iter()
                        .map(|value| value.to_string())
                        .collect(),
                )),
                default_value: Value::String(quality_values[0].into()),
                expose: vec![ParamExpose::Control],
            });
        }

        Ok(ProviderParamSchema {
            params,
            schema_revision: 1,
        })
    }

    fn generate<'a>(
        &'a self,
        req: VideoGenerationRequest,
        output_path: std::path::PathBuf,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let model = find_model(&req.model_id).ok_or_else(|| ExecutorError::InvalidInput {
                message: format!("unsupported LibTV video model '{}'", req.model_id),
            })?;
            let headers = auth_headers(self.credential_store.as_ref())?;
            let duration = optional_int(&req.inputs, "duration_seconds")
                .unwrap_or(model.default_duration as i64);
            let resolution = optional_string(&req.inputs, "resolution")
                .unwrap_or_else(|| model.default_resolution.into());
            let ratio = optional_string(&req.inputs, "ratio")
                .unwrap_or_else(|| model.default_ratio.into());
            let quality = optional_string(&req.inputs, "quality").unwrap_or_else(|| {
                model
                    .quality_kind
                    .map(|values| values[0].to_string())
                    .unwrap_or_else(|| "low".into())
            });
            let body = json!({
                "params": build_params(&req, model, duration as u32, &resolution, &ratio, &quality),
                "metadata": {
                    "node_id": Uuid::new_v4().to_string(),
                    "project_id": self.credential_store.get_secret(self.id(), "project_id").unwrap_or_default(),
                },
                "provider": model.provider,
                "model": model.id,
                "taskType": "video",
                "requestId": Uuid::new_v4().to_string(),
            });

            let create_payload: LibTvEnvelope<CreateTaskData> = self
                .client
                .post(format!("{API_BASE}/api/task/generation/create"))
                .headers(headers.clone())
                .json(&body)
                .send()
                .await
                .map_err(http_error)?
                .error_for_status()
                .map_err(http_error)?
                .json()
                .await
                .map_err(http_error)?;
            if create_payload.code != 0 {
                return Err(classify_error(
                    create_payload.code,
                    create_payload.msg.as_deref(),
                ));
            }
            let task_id = create_payload
                .data
                .ok_or_else(|| ExecutorError::RuntimeFailed {
                    message: "LibTV video task returned empty payload".into(),
                })?
                .task_id;

            let deadline = tokio::time::Instant::now() + POLL_TIMEOUT;
            let video_url = loop {
                if tokio::time::Instant::now() >= deadline {
                    return Err(ExecutorError::Timeout);
                }

                let progress_payload: LibTvEnvelope<ProgressData> = self
                    .client
                    .post(format!("{API_BASE}/api/task/generation/progress"))
                    .headers(headers.clone())
                    .json(&json!({ "taskIds": [task_id.clone()] }))
                    .send()
                    .await
                    .map_err(http_error)?
                    .error_for_status()
                    .map_err(http_error)?
                    .json()
                    .await
                    .map_err(http_error)?;
                if progress_payload.code != 0 {
                    return Err(classify_error(
                        progress_payload.code,
                        progress_payload.msg.as_deref(),
                    ));
                }

                let progress = progress_payload
                    .data
                    .and_then(|data| data.progresses.into_iter().next())
                    .ok_or_else(|| ExecutorError::RuntimeFailed {
                        message: "LibTV video progress payload was empty".into(),
                    })?;
                if progress.status == 2 {
                    let task_result: TaskResult = serde_json::from_str(
                        progress.task_result.as_deref().ok_or_else(|| {
                            ExecutorError::RuntimeFailed {
                                message: "LibTV video task finished without taskResult".into(),
                            }
                        })?,
                    )
                    .map_err(http_error)?;
                    let video = task_result.videos.into_iter().next().ok_or_else(|| {
                        ExecutorError::RuntimeFailed {
                            message: "LibTV video task finished without videos".into(),
                        }
                    })?;
                    break video.preview_path;
                }
                if progress.status >= 3 {
                    return Err(ExecutorError::RuntimeFailed {
                        message: format!(
                            "LibTV video generation failed with status {}",
                            progress.status
                        ),
                    });
                }
                tokio::time::sleep(POLL_INTERVAL).await;
            };

            let bytes = self
                .client
                .get(video_url)
                .send()
                .await
                .map_err(http_error)?
                .error_for_status()
                .map_err(http_error)?
                .bytes()
                .await
                .map_err(http_error)?;
            if let Some(parent) = output_path.parent() {
                std::fs::create_dir_all(parent).map_err(http_error)?;
            }
            std::fs::write(&output_path, &bytes).map_err(http_error)?;

            Ok(GeneratedVideoAsset {
                video_path: output_path,
                fps: optional_int(&req.inputs, "fps").map(|fps| fps as u32),
            })
        })
    }
}

fn build_params(
    req: &VideoGenerationRequest,
    model: &VideoModelSpec,
    duration: u32,
    resolution: &str,
    ratio: &str,
    quality: &str,
) -> serde_json::Value {
    let mut params = json!({
        "prompt": req.prompt,
        "model": model.id,
        "modeType": "text2video",
        "count": 1,
        "ratio": ratio,
        "duration": duration,
        "resolution": resolution,
        "textList": [],
        "imageList": [],
        "videoList": [],
        "audioList": [],
        "infiniteSwitch": 0,
    });
    if model.id == "star-video2-fast" || model.id == "seedance2.0" {
        params["enableSound"] = json!("on");
        params["search_enabled"] = json!(1);
    }
    if model.quality_kind.is_some() {
        params["quality"] = json!(quality);
    }
    params
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

fn classify_error(code: i32, message: Option<&str>) -> ExecutorError {
    let message = message.unwrap_or("unknown LibTV error");
    if matches!(code, 10001 | 10401 | 10403 | 401 | 403)
        || message.contains("登录")
        || message.to_ascii_lowercase().contains("token")
    {
        return ExecutorError::Unavailable {
            message: format!("LibTV authentication failed: {message}"),
        };
    }

    ExecutorError::RuntimeFailed {
        message: format!("LibTV video generation failed (code {code}): {message}"),
    }
}

fn optional_string(inputs: &HashMap<String, Value>, key: &str) -> Option<String> {
    match inputs.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn optional_int(inputs: &HashMap<String, Value>, key: &str) -> Option<i64> {
    match inputs.get(key) {
        Some(Value::Int(value)) if *value >= 0 => Some(*value),
        Some(Value::Float(value)) if *value >= 0.0 => Some(*value as i64),
        _ => None,
    }
}

fn find_model(model_id: &str) -> Option<&'static VideoModelSpec> {
    VIDEO_MODELS.iter().find(|model| model.id == model_id)
}

fn http_error(error: impl std::fmt::Display) -> ExecutorError {
    ExecutorError::RuntimeFailed {
        message: error.to_string(),
    }
}
