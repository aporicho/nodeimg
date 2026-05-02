use super::super::Context;
use crate::icon::IconId;
use crate::renderer::TextureSize;
use crate::tree::layout::TextureHandle;
use std::sync::Arc;

pub struct ResourcesApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl ResourcesApi<'_> {
    pub fn register_texture(
        &mut self,
        handle: TextureHandle,
        view: Arc<wgpu::TextureView>,
        size: TextureSize,
    ) {
        self.ctx.register_texture(handle, view, size);
    }

    pub fn register_svg_icon(&mut self, id: impl Into<IconId>, svg: impl AsRef<[u8]>) {
        self.ctx.register_svg_icon(id, svg);
    }
}
