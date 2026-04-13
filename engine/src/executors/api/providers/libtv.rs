use std::collections::HashMap;
use std::time::Duration;

use image::DynamicImage;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use types::{Constraint, Value};
use uuid::Uuid;

use crate::executors::api::provider::{placeholder_execute, Provider, ProviderRequest};
use crate::executors::api::{ExecutionOutputs, ProviderFuture};
use crate::node_manager::{ApiNodeMeta, ExecutorType, NodeDef, ParamDef, ParamExpose, PinDef};

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
}

impl LibTvProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn execute_image_generation(
        &self,
        inputs: HashMap<String, Value>,
    ) -> Result<ExecutionOutputs, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = required_string(&inputs, "prompt")?;
        let model = optional_string(&inputs, "model").unwrap_or_else(|| "nebula-ultra".into());
        let spec = lookup_model(&model).ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "unsupported LibTV image model '{model}'"
            ))
        })?;
        let quality = resolve_choice(optional_string(&inputs, "quality"), spec.default_quality);
        let ratio = resolve_choice(optional_string(&inputs, "ratio"), spec.default_ratio);
        let headers = auth_headers()?;

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
                "project_id": required_env("LIBTV_PROJECT_ID")?,
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
            .await?;
        let create_response = create_response.error_for_status()?;
        let create_payload: LibTvEnvelope<CreateTaskData> = create_response.json().await?;

        if is_auth_error(create_payload.code, create_payload.msg.as_deref()) {
            return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                "LibTV credentials are missing or expired",
            ));
        }
        if create_payload.code != 0 {
            return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "LibTV create task failed: {}",
                create_payload.msg.unwrap_or_else(|| "unknown error".into())
            )));
        }

        let task_id = create_payload
            .data
            .ok_or_else(|| {
                Box::<dyn std::error::Error + Send + Sync>::from(
                    "LibTV create task returned empty payload",
                )
            })?
            .task_id;

        let deadline = tokio::time::Instant::now() + POLL_TIMEOUT;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                    "LibTV image generation timed out",
                ));
            }

            let progress_response = self
                .client
                .post(format!("{API_BASE}/api/task/generation/progress"))
                .headers(headers.clone())
                .json(&json!({ "taskIds": [task_id.clone()] }))
                .send()
                .await?;
            let progress_response = progress_response.error_for_status()?;
            let progress_payload: LibTvEnvelope<ProgressData> = progress_response.json().await?;

            if is_auth_error(progress_payload.code, progress_payload.msg.as_deref()) {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from(
                    "LibTV credentials are missing or expired",
                ));
            }
            if progress_payload.code != 0 {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "LibTV progress poll failed: {}",
                    progress_payload
                        .msg
                        .unwrap_or_else(|| "unknown error".into())
                )));
            }

            let progress = progress_payload
                .data
                .and_then(|data| data.progresses.into_iter().next())
                .ok_or_else(|| {
                    Box::<dyn std::error::Error + Send + Sync>::from(
                        "LibTV progress payload was empty",
                    )
                })?;

            if progress.status == 2 {
                let task_result: TaskResult =
                    serde_json::from_str(progress.task_result.as_deref().ok_or_else(|| {
                        Box::<dyn std::error::Error + Send + Sync>::from(
                            "LibTV completed task without taskResult",
                        )
                    })?)?;
                let image_url = task_result
                    .images
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        Box::<dyn std::error::Error + Send + Sync>::from(
                            "LibTV completed task without images",
                        )
                    })?
                    .preview_path;

                let image = self.download_image(&image_url).await?;
                return Ok(HashMap::from([(
                    String::from("image"),
                    Value::Image(types::Image::from_cpu(image)),
                )]));
            }

            if progress.status >= 3 {
                return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "LibTV image generation failed with status {}",
                    progress.status
                )));
            }

            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn download_image(
        &self,
        image_url: &str,
    ) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
        let response = self.client.get(image_url).send().await?;
        let response = response.error_for_status()?;
        let bytes = response.bytes().await?;
        Ok(image::load_from_memory(bytes.as_ref())?)
    }
}

impl Default for LibTvProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Provider for LibTvProvider {
    fn id(&self) -> &'static str {
        "libtv"
    }

    fn nodes(&self) -> Vec<NodeDef> {
        vec![NodeDef {
            type_id: "libtv.image_gen".into(),
            version: 1,
            source: crate::node_manager::NodeSourceKind::Api,
            name: "LibTV Image Gen".into(),
            category: "api/libtv".into(),
            executor_type: ExecutorType::Api,
            requires: vec![],
            purity: crate::node_manager::Purity::Impure,
            cooking_sensitivity: vec![],
            realtime_capable: false,
            execution: crate::node_manager::ExecutionPolicy::default(),
            api: Some(ApiNodeMeta {
                provider_id: self.id().into(),
                operation: "image_gen".into(),
            }),
            inputs: vec![],
            outputs: vec![PinDef {
                name: "image".into(),
                data_type: types::DataType::image(),
                optional: false,
            }],
            params: vec![
                ParamDef {
                    name: "prompt".into(),
                    data_type: types::DataType::string(),
                    constraint: None,
                    default_value: Value::String(String::new()),
                    expose: vec![ParamExpose::Control],
                },
                ParamDef {
                    name: "model".into(),
                    data_type: types::DataType::string(),
                    constraint: Some(Constraint::enum_options(
                        IMAGE_MODELS
                            .iter()
                            .map(|spec| spec.id.to_string())
                            .collect(),
                    )),
                    default_value: Value::String("nebula-ultra".into()),
                    expose: vec![ParamExpose::Control],
                },
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
                    default_value: Value::String("default".into()),
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
                    default_value: Value::String("default".into()),
                    expose: vec![ParamExpose::Control],
                },
            ],
            execute: placeholder_execute("libtv.image_gen"),
        }]
    }

    fn execute<'a>(&'a self, request: ProviderRequest<'a>) -> ProviderFuture<'a> {
        Box::pin(async move {
            match request.operation {
                "image_gen" => self.execute_image_generation(request.inputs).await,
                other => Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "LibTV provider does not support operation '{other}'"
                ))),
            }
        })
    }
}

fn required_string(
    inputs: &HashMap<String, Value>,
    key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let value = optional_string(inputs, key).ok_or_else(|| {
        Box::<dyn std::error::Error + Send + Sync>::from(format!(
            "missing required string input '{key}'"
        ))
    })?;
    if value.trim().is_empty() {
        return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
            "input '{key}' must not be empty"
        )));
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

fn required_env(name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    std::env::var(name).map_err(|_| {
        Box::<dyn std::error::Error + Send + Sync>::from(format!(
            "environment variable '{name}' is required for LibTV API nodes"
        ))
    })
}

fn auth_headers() -> Result<HeaderMap, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "token",
        HeaderValue::from_str(&required_env("LIBTV_TOKEN")?)?,
    );
    headers.insert(
        "webid",
        HeaderValue::from_str(&required_env("LIBTV_WEBID")?)?,
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

fn is_auth_error(code: i32, msg: Option<&str>) -> bool {
    matches!(code, 10001 | 10401 | 10403 | 401 | 403)
        || msg.is_some_and(|message| {
            let message = message.to_ascii_lowercase();
            message.contains("token") || message.contains("unauthorized")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_image_generation_node() {
        let provider = LibTvProvider::new();
        let node = provider.nodes().remove(0);

        assert_eq!(node.type_id, "libtv.image_gen");
        assert_eq!(node.executor_type, ExecutorType::Api);
        assert_eq!(
            node.api.as_ref().map(|meta| meta.provider_id.as_str()),
            Some("libtv")
        );
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
