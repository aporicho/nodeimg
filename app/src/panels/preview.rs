use super::PanelBuildContext;
use gui::panel::{PanelConfig, PanelDeclaration, PanelId};
use gui::renderer::{ImageStyle, Rect};
use gui::ui;
use gui::widget::atoms::image_viewer::ImageViewerProps;
use std::borrow::Cow;

pub(crate) fn panel(ctx: &PanelBuildContext<'_>) -> PanelDeclaration {
    PanelDeclaration {
        config: PanelConfig {
            id: PanelId::new("preview"),
            title: Cow::Borrowed("Preview"),
            default_rect: Rect {
                x: 180.0,
                y: 28.0,
                w: 360.0,
                h: 224.0,
            },
            min_size: [260.0, 180.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        },
        content: vec![ui::widget(
            Cow::Borrowed("preview_image"),
            ImageViewerProps {
                texture: ctx.preview_image,
                height: 208.0,
                style: ImageStyle::default(),
            },
        )
        .build()],
    }
}
