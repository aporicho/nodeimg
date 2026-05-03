use std::borrow::Cow;

use gui::control::ControlNode;
use gui::panel::{PanelConfig, PanelContentTemplate, PanelId};
use gui::renderer::{ImageFit, ImageStyle, Rect};

use super::{PanelAppliedSnapshot, PanelModeSet, PanelRenderInput, PanelSpec};

pub(super) fn spec(input: PanelRenderInput<'_>) -> PanelSpec {
    PanelSpec {
        modes: PanelModeSet::Both,
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
        content: PanelContentTemplate::new(vec![ControlNode::image(
            "preview_image",
            input.preview_image,
            ImageStyle::default().with_fit(ImageFit::Contain),
        )]),
        applied: PanelAppliedSnapshot::new(),
    }
}
