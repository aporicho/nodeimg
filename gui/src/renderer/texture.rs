use std::sync::Arc;

use super::TextureSize;

#[derive(Clone)]
pub(crate) struct TextureResource {
    pub(crate) view: Arc<wgpu::TextureView>,
    pub(crate) size: TextureSize,
}

impl TextureResource {
    pub(crate) fn new(view: Arc<wgpu::TextureView>, size: TextureSize) -> Self {
        Self { view, size }
    }
}
