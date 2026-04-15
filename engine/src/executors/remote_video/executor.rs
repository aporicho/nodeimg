use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use types::Value;

use crate::capability::{Capability, CapabilityId, SideEffect};
use crate::execution::{
    CookingContext, DimensionValue, ExecutionOutputs, ExecutorError, NodeExecutionRequest,
};
use crate::executors::{Executor, ExecutorFuture, LocalityProfile};
use crate::executors::video::{FfmpegVideoBackend, VideoDecodeBackend};

use super::provider::VideoGenerationRequest;
use super::registry::ProviderRegistry;

#[derive(Serialize, Clone)]
struct GenerationKey {
    provider_id: String,
    model_id: String,
    prompt: String,
    frame_count: u32,
    width: u32,
    height: u32,
    fps: u32,
    seed: u64,
}

pub struct RemoteVideoGenerationExecutor {
    providers: Arc<ProviderRegistry>,
    in_memory: Mutex<HashMap<String, Arc<CachedVideoAsset>>>,
    cache_dir: PathBuf,
    decode_backend: Arc<dyn VideoDecodeBackend>,
}

struct CachedVideoAsset {
    video_path: PathBuf,
    frames_dir: PathBuf,
    fps: Option<u32>,
}

impl RemoteVideoGenerationExecutor {
    pub fn new(providers: Arc<ProviderRegistry>) -> Self {
        let cache_dir = cache_dir("ai.video_generate");
        let _ = fs::create_dir_all(&cache_dir);
        Self {
            providers,
            in_memory: Mutex::new(HashMap::new()),
            cache_dir,
            decode_backend: Arc::new(FfmpegVideoBackend),
        }
    }
}

impl Executor for RemoteVideoGenerationExecutor {
    fn provides(&self) -> Vec<Capability> {
        vec![Capability::new(
            "ai.video_generate",
            vec![],
            vec![
                types::DataType::video(),
                types::DataType::image(),
                types::DataType::int(),
            ],
            vec![SideEffect::NetworkCall("*".into())],
        )]
    }

    fn execute<'a>(
        &'a self,
        cap_id: &'a CapabilityId,
        req: NodeExecutionRequest<'a>,
    ) -> ExecutorFuture<'a> {
        Box::pin(async move {
            if cap_id != "ai.video_generate" {
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
            let prompt = required_string(&req.inputs, "prompt")?;
            let duration_seconds = required_int(&req.inputs, "duration_seconds")? as u32;
            let fps = optional_int(&req.inputs, "fps")
                .filter(|fps| *fps > 0)
                .map(|fps| fps as u32)
                .or_else(|| provider.default_fps(&model_id))
                .ok_or_else(|| ExecutorError::InvalidInput {
                    message: format!(
                        "provider '{provider_id}' does not expose a default fps and no positive fps was provided"
                    ),
                })?;
            let seed = required_int(&req.inputs, "seed").unwrap_or(0) as u64;
            let frame_count = duration_seconds.saturating_mul(fps).max(1);
            let frame_index = current_frame(&req.cooking_context);

            let mut provider_inputs = req.inputs.clone();
            provider_inputs.insert(String::from("fps"), Value::Int(fps as i64));

            let key = GenerationKey {
                provider_id: provider_id.clone(),
                model_id: model_id.clone(),
                prompt: prompt.clone(),
                frame_count,
                width: required_int(&req.inputs, "width").unwrap_or(0) as u32,
                height: required_int(&req.inputs, "height").unwrap_or(0) as u32,
                fps,
                seed,
            };
            let key_hash = cache_key_for(&key)?;

            let cached_animation = {
                self.in_memory
                    .lock()
                    .expect("video generation cache poisoned")
                    .get(&key_hash)
                    .cloned()
            };

            let animation = if let Some(existing) = cached_animation {
                existing
            } else {
                let video_path = self.cache_dir.join(format!("{key_hash}.mp4"));
                let frames_dir = self.cache_dir.join(format!("{key_hash}_frames"));
                let asset = if video_path.exists() {
                    Arc::new(CachedVideoAsset {
                        video_path,
                        frames_dir,
                        fps: Some(fps),
                    })
                } else {
                    let generated = provider.generate(
                        VideoGenerationRequest {
                            model_id,
                            prompt,
                            inputs: provider_inputs,
                        },
                        video_path,
                    ).await?;
                    Arc::new(CachedVideoAsset {
                        video_path: generated.video_path,
                        frames_dir,
                        fps: generated.fps,
                    })
                };

                ensure_extracted_frames(self.decode_backend.as_ref(), &asset.video_path, &asset.frames_dir)?;

                self.in_memory
                    .lock()
                    .expect("video generation cache poisoned")
                    .insert(key_hash.clone(), Arc::clone(&asset));
                asset
            };

            ensure_extracted_frames(
                self.decode_backend.as_ref(),
                &animation.video_path,
                &animation.frames_dir,
            )?;

            let image = load_frame(&animation.frames_dir, frame_index)?;
            let output_fps = animation.fps.unwrap_or(fps) as i64;

            Ok(ExecutionOutputs::full(HashMap::from([
                (
                    String::from("video"),
                    Value::String(animation.video_path.display().to_string()),
                ),
                (
                    String::from("image"),
                    Value::Image(types::Image::from_cpu(image)),
                ),
                (String::from("fps"), Value::Int(output_fps)),
            ])))
        })
    }

    fn locality_profile(&self) -> LocalityProfile {
        LocalityProfile::Remote
    }
}

fn required_string(inputs: &HashMap<String, Value>, key: &str) -> Result<String, ExecutorError> {
    match inputs.get(key) {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(ExecutorError::InvalidInput {
            message: format!("missing required string parameter '{key}'"),
        }),
    }
}

fn required_int(inputs: &HashMap<String, Value>, key: &str) -> Result<i64, ExecutorError> {
    match inputs.get(key) {
        Some(Value::Int(value)) if *value >= 0 => Ok(*value),
        Some(Value::Float(value)) if *value >= 0.0 => Ok(*value as i64),
        Some(_) => Err(ExecutorError::InvalidInput {
            message: format!("parameter '{key}' must be numeric"),
        }),
        None => Err(ExecutorError::InvalidInput {
            message: format!("missing required numeric parameter '{key}'"),
        }),
    }
}

fn optional_int(inputs: &HashMap<String, Value>, key: &str) -> Option<i64> {
    match inputs.get(key) {
        Some(Value::Int(value)) if *value >= 0 => Some(*value),
        Some(Value::Float(value)) if *value >= 0.0 => Some(*value as i64),
        _ => None,
    }
}

fn current_frame(context: &CookingContext) -> u32 {
    match context.dimensions.get("frame") {
        Some(DimensionValue::Int(value)) if *value >= 0 => *value as u32,
        _ => 0,
    }
}

fn cache_dir(namespace: &str) -> PathBuf {
    let base = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    base.join(".cache").join("nodeimg").join(namespace)
}

fn ensure_extracted_frames(
    backend: &dyn VideoDecodeBackend,
    video_path: &PathBuf,
    frames_dir: &PathBuf,
) -> Result<(), ExecutorError> {
    let has_frames = fs::read_dir(frames_dir)
        .ok()
        .and_then(|mut entries| entries.next())
        .is_some();
    if has_frames {
        return Ok(());
    }

    if frames_dir.exists() {
        fs::remove_dir_all(frames_dir).map_err(io_error)?;
    }
    backend.extract_frames(video_path, frames_dir)
}

fn load_frame(frames_dir: &PathBuf, frame_index: u32) -> Result<image::DynamicImage, ExecutorError> {
    let exact = frames_dir.join(format!("frame_{frame_index:06}.png"));
    if exact.exists() {
        return image::open(&exact).map_err(io_error);
    }

    let mut entries = fs::read_dir(frames_dir)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect::<Vec<_>>();
    entries.sort();
    let fallback = entries.last().cloned().ok_or_else(|| ExecutorError::RuntimeFailed {
        message: format!("no extracted frames available in {}", frames_dir.display()),
    })?;
    image::open(fallback).map_err(io_error)
}

fn io_error(error: impl std::fmt::Display) -> ExecutorError {
    ExecutorError::RuntimeFailed {
        message: error.to_string(),
    }
}

fn cache_key_for(key: &GenerationKey) -> Result<String, ExecutorError> {
    let bytes = serde_json::to_vec(key).map_err(|error| ExecutorError::RuntimeFailed {
        message: error.to_string(),
    })?;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    use std::hash::Hasher;
    bytes.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}
