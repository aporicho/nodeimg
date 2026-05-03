use std::borrow::Cow;

use gui::control::ControlNode;
use gui::panel::{PanelConfig, PanelContentTemplate, PanelId};
use gui::renderer::Rect;

use crate::image_demo::{ADD_IMAGE_DEMO_GRAPH_ID, RUN_IMAGE_DEMO_ID};

use super::{PanelAppliedSnapshot, PanelModeSet, PanelRenderInput, PanelSpec};

pub(super) fn spec(_input: PanelRenderInput<'_>) -> PanelSpec {
    PanelSpec {
        modes: PanelModeSet::Both,
        config: PanelConfig {
            id: PanelId::new("toolbar"),
            title: Cow::Borrowed("Toolbar"),
            default_rect: Rect {
                x: 28.0,
                y: 28.0,
                w: 180.0,
                h: 82.0,
            },
            min_size: [156.0, 72.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        },
        content: PanelContentTemplate::new(vec![
            ControlNode::button(ADD_IMAGE_DEMO_GRAPH_ID, "Add Image Demo"),
            ControlNode::button(RUN_IMAGE_DEMO_ID, "Run Image Demo"),
        ]),
        applied: PanelAppliedSnapshot::new(),
    }
}
