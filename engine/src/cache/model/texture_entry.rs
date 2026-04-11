use std::sync::Arc;
use std::time::Instant;

use types::GpuTexture;

use super::GenerationId;

#[derive(Clone, Debug)]
pub struct TextureEntry {
    pub generation: GenerationId,
    pub texture: Arc<GpuTexture>,
    pub bytes: usize,
    pub updated_at: Instant,
}

impl TextureEntry {
    pub fn new(generation: GenerationId, texture: Arc<GpuTexture>, bytes: usize) -> Self {
        Self {
            generation,
            texture,
            bytes,
            updated_at: Instant::now(),
        }
    }
}
