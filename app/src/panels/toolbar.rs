use super::PanelBuildContext;
use crate::image_demo::{ADD_IMAGE_DEMO_GRAPH_ID, RUN_IMAGE_DEMO_ID};
use gui::panel::{PanelConfig, PanelDeclaration, PanelId};
use gui::renderer::Rect;
use gui::tree::Desc;
use gui::widget::atoms::button::ButtonProps;
use std::borrow::Cow;

pub(crate) fn panel(_ctx: &PanelBuildContext<'_>) -> PanelDeclaration {
    PanelDeclaration {
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
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(ADD_IMAGE_DEMO_GRAPH_ID),
                ButtonProps {
                    label: Cow::Borrowed("Add Image Demo"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(RUN_IMAGE_DEMO_ID),
                ButtonProps {
                    label: Cow::Borrowed("Run Image Demo"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
        ],
    }
}
