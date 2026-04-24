use crate::visual_audit::{build_visual_audit_page, VisualAuditBuildContext, VisualAuditState};
use gui::renderer::Rect;
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;
use gui::tree::Desc;

pub(crate) struct DeveloperModeBuildContext<'a> {
    pub(crate) viewport: Rect,
    pub(crate) theme: &'a Theme,
    pub(crate) visual_audit: &'a VisualAuditState,
    pub(crate) image: TextureHandle,
}

pub(crate) fn build_developer_page(ctx: DeveloperModeBuildContext<'_>) -> Desc {
    build_visual_audit_page(VisualAuditBuildContext {
        viewport: ctx.viewport,
        theme: ctx.theme,
        state: ctx.visual_audit,
        image: ctx.image,
    })
}
