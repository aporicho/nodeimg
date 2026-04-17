use std::{collections::HashMap, sync::Arc};

use crate::tree::layout::TextureHandle;

pub(crate) struct ResourceRegistry {
    textures: HashMap<TextureHandle, Arc<wgpu::TextureView>>,
}

impl ResourceRegistry {
    pub(crate) fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub(crate) fn register_texture(&mut self, handle: TextureHandle, view: Arc<wgpu::TextureView>) {
        self.textures.insert(handle, view);
    }

    pub(crate) fn textures(&self) -> &HashMap<TextureHandle, Arc<wgpu::TextureView>> {
        &self.textures
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}
