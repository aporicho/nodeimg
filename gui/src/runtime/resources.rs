use std::{collections::HashMap, sync::Arc};

use crate::renderer::{TextureResource, TextureSize};
use crate::tree::layout::TextureHandle;

pub(crate) struct ResourceRegistry {
    textures: HashMap<TextureHandle, TextureResource>,
}

impl ResourceRegistry {
    pub(crate) fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub(crate) fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.textures
            .insert(handle, TextureResource::new(view, size));
    }

    pub(crate) fn textures(&self) -> &HashMap<TextureHandle, TextureResource> {
        &self.textures
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
