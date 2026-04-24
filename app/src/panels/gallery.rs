use super::PanelBuildContext;
use crate::demo_gallery::build_gallery_panel_content;
use gui::panel::{PanelConfig, PanelDeclaration, PanelId};
use gui::renderer::Rect;
use std::borrow::Cow;

pub(crate) fn panel(ctx: &PanelBuildContext<'_>) -> PanelDeclaration {
    PanelDeclaration {
        config: PanelConfig {
            id: PanelId::new("gallery"),
            title: Cow::Borrowed("Visual Audit"),
            default_rect: Rect {
                x: 560.0,
                y: 28.0,
                w: 360.0,
                h: 700.0,
            },
            min_size: [320.0, 420.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        },
        content: build_gallery_panel_content(ctx.theme, ctx.gallery, ctx.image),
    }
}
