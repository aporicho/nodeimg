use super::super::Context;
use crate::renderer::{Rect, Renderer, TextMeasurer};
use crate::theme::Theme;
use crate::tree::layout::LayoutFlushStats;

pub struct RenderingApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl RenderingApi<'_> {
    pub fn flush_layout_dirty(
        &mut self,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
    ) -> LayoutFlushStats {
        self.ctx.flush_layout_dirty(root_rect, measurer)
    }

    pub fn render(
        &mut self,
        renderer: &mut Renderer,
        viewport_w: f32,
        viewport_h: f32,
        theme: &Theme,
    ) {
        self.ctx.render(renderer, viewport_w, viewport_h, theme)
    }
}
