use std::collections::HashMap;
use std::path::Path;

use image::{DynamicImage, Rgba, RgbaImage};
use uuid::Uuid;

use crate::execution::ExecutorError;
use crate::executors::video::encode_frames_to_mp4;
use types::Value;

use super::super::provider::{
    GeneratedVideoAsset, ModelInfo, ProviderFuture, ProviderParamSchema, ProviderSchemaQuery,
    VideoGenerationMode, VideoGenerationProvider, VideoGenerationRequest,
};

pub struct MockVideoGenerationProvider;

impl MockVideoGenerationProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockVideoGenerationProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoGenerationProvider for MockVideoGenerationProvider {
    fn id(&self) -> &'static str {
        "mock"
    }

    fn display_name(&self) -> &'static str {
        "Mock API"
    }

    fn default_fps(&self, _model_id: &str) -> Option<u32> {
        Some(8)
    }

    fn models(&self, mode: VideoGenerationMode) -> Result<Vec<ModelInfo>, ExecutorError> {
        match mode {
            VideoGenerationMode::TextToVideo => Ok(vec![ModelInfo {
                id: "mock-wave".into(),
                display_name: "Mock Wave".into(),
            }]),
        }
    }

    fn param_schema(
        &self,
        _query: ProviderSchemaQuery,
    ) -> Result<ProviderParamSchema, ExecutorError> {
        Ok(ProviderParamSchema {
            params: vec![],
            schema_revision: 1,
        })
    }

    fn generate<'a>(
        &'a self,
        req: VideoGenerationRequest,
        output_path: std::path::PathBuf,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let fps = required_u32(&req.inputs, "fps")?;
            let duration_seconds = required_u32(&req.inputs, "duration_seconds")?;
            let frame_count = duration_seconds.saturating_mul(fps).max(1);
            let width = required_u32(&req.inputs, "width")?;
            let height = required_u32(&req.inputs, "height")?;
            let seed = optional_u64(&req.inputs, "seed").unwrap_or(0);

            let prompt_hash = req.prompt.bytes().fold(seed, |acc, byte| {
                acc.wrapping_mul(16777619).wrapping_add(byte as u64)
            });
            let frames = (0..frame_count)
                .map(|frame| generate_frame(width, height, prompt_hash, frame, frame_count))
                .collect::<Vec<_>>();

            encode_via_temp_dir(&output_path, &frames, fps)?;

            Ok(GeneratedVideoAsset {
                video_path: output_path,
                fps: Some(fps),
            })
        })
    }
}

fn encode_via_temp_dir(
    output_path: &Path,
    frames: &[DynamicImage],
    fps: u32,
) -> Result<(), ExecutorError> {
    let frames_dir = std::env::temp_dir().join(format!(
        "nodeimg_mock_video_frames_{}_{}",
        std::process::id(),
        Uuid::new_v4()
    ));
    std::fs::create_dir_all(&frames_dir).map_err(io_error)?;
    for (index, frame) in frames.iter().enumerate() {
        frame
            .save(frames_dir.join(format!("frame_{index:06}.png")))
            .map_err(io_error)?;
    }
    let result = encode_frames_to_mp4(&frames_dir, output_path, fps);
    let _ = std::fs::remove_dir_all(&frames_dir);
    result
}

fn required_u32(inputs: &HashMap<String, Value>, key: &str) -> Result<u32, ExecutorError> {
    match inputs.get(key) {
        Some(Value::Int(value)) if *value > 0 => Ok(*value as u32),
        Some(Value::Float(value)) if *value > 0.0 => Ok(*value as u32),
        _ => Err(ExecutorError::InvalidInput {
            message: format!("missing required positive numeric parameter '{key}'"),
        }),
    }
}

fn optional_u64(inputs: &HashMap<String, Value>, key: &str) -> Option<u64> {
    match inputs.get(key) {
        Some(Value::Int(value)) if *value >= 0 => Some(*value as u64),
        Some(Value::Float(value)) if *value >= 0.0 => Some(*value as u64),
        _ => None,
    }
}

fn generate_frame(
    width: u32,
    height: u32,
    prompt_hash: u64,
    frame: u32,
    frame_count: u32,
) -> DynamicImage {
    let mut rgba = RgbaImage::new(width, height);
    let phase = if frame_count <= 1 {
        0.0
    } else {
        frame as f32 / (frame_count - 1) as f32
    };
    let seed_r = ((prompt_hash & 0xff) as f32) / 255.0;
    let seed_g = (((prompt_hash >> 8) & 0xff) as f32) / 255.0;
    let seed_b = (((prompt_hash >> 16) & 0xff) as f32) / 255.0;

    for (x, y, pixel) in rgba.enumerate_pixels_mut() {
        let nx = x as f32 / width.max(1) as f32;
        let ny = y as f32 / height.max(1) as f32;
        let wave = ((nx * 6.2831 + phase * 6.2831).sin() * 0.5 + 0.5).clamp(0.0, 1.0);
        let swirl = ((ny * 6.2831 - phase * 4.0).cos() * 0.5 + 0.5).clamp(0.0, 1.0);

        let r = ((wave * 0.7 + seed_r * 0.3) * 255.0).round() as u8;
        let g = ((swirl * 0.7 + seed_g * 0.3) * 255.0).round() as u8;
        let b = ((((1.0 - wave) * 0.5 + phase * 0.5) * 0.7 + seed_b * 0.3) * 255.0).round() as u8;
        *pixel = Rgba([r, g, b, 255]);
    }

    DynamicImage::ImageRgba8(rgba)
}

fn io_error(error: impl std::fmt::Display) -> ExecutorError {
    ExecutorError::RuntimeFailed {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_mp4_file() {
        let provider = MockVideoGenerationProvider::new();
        let output_path =
            std::env::temp_dir().join(format!("mock_provider_{}.mp4", Uuid::new_v4()));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let animation = runtime
            .block_on(provider.generate(
                VideoGenerationRequest {
                    model_id: "mock-wave".into(),
                    prompt: "ocean".into(),
                    inputs: HashMap::from([
                        ("duration_seconds".into(), Value::Int(1)),
                        ("fps".into(), Value::Int(8)),
                        ("width".into(), Value::Int(16)),
                        ("height".into(), Value::Int(16)),
                        ("seed".into(), Value::Int(42)),
                    ]),
                },
                output_path.clone(),
            ))
            .unwrap();

        assert_eq!(animation.fps, Some(8));
        assert!(animation.video_path.exists());
        let _ = std::fs::remove_file(output_path);
    }
}
