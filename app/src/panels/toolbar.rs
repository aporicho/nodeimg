use super::PanelBuildContext;
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
                w: 128.0,
                h: 52.0,
            },
            min_size: [96.0, 48.0],
            titlebar_visible: true,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        },
        content: vec![Desc::Widget {
            id: Cow::Borrowed("toolbar_run"),
            props: Box::new(ButtonProps {
                label: Cow::Borrowed("Run"),
                icon: None,
                disabled: false,
            }),
        }],
    }
}
