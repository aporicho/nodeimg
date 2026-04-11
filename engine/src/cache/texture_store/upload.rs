use std::sync::Arc;

use types::{GpuTexture, Value};

use crate::cache::model::{GenerationId, TextureEntry};

use super::bytes::estimate_texture_bytes;

pub fn upload_preview_value(
    value: &Value,
    generation: GenerationId,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Option<TextureEntry> {
    let image = match value {
        Value::Image(image) => image,
        _ => return None,
    };

    let texture: Arc<GpuTexture> = Arc::clone(image.as_gpu(device, queue));
    let bytes = estimate_texture_bytes(texture.as_ref());

    Some(TextureEntry::new(generation, texture, bytes))
}
