use std::sync::Arc;

use super::Context;
use crate::icon::IconId;
use crate::renderer::TextureSize;
use crate::tree::layout::TextureHandle;

impl Context {
    pub(crate) fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.resources.register_texture(handle, view, size);
    }

    pub(crate) fn register_svg_icon(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        self.icons.register_svg(id, svg);
    }
}
