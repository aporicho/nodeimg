use super::state::POPUP_CLOSE_ID;
use gui::tree::Desc;
use gui::ui;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::atoms::separator::{SeparatorOrientation, SeparatorProps};
use std::borrow::Cow;

pub(crate) fn build_visual_audit_popup() -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed("visual_audit_popup_group"),
        gui::widget::frameworks::group::GroupProps {
            title: Cow::Borrowed("Quick Actions"),
            content: vec![
                Desc::from(ui::widget(
                    Cow::Borrowed("popup_label"),
                    LabelProps {
                        text: Cow::Borrowed(
                            "Overlay 由 framework 持有，支持 outside click、Escape 和焦点恢复。",
                        ),
                        variant: LabelVariant::Caption,
                        muted: true,
                    },
                )),
                Desc::from(ui::widget(
                    Cow::Borrowed("popup_separator"),
                    SeparatorProps {
                        orientation: SeparatorOrientation::Horizontal,
                    },
                )),
                Desc::from(ui::widget(
                    Cow::Borrowed(POPUP_CLOSE_ID),
                    ButtonProps {
                        label: Cow::Borrowed("Close Popup"),
                        icon: None,
                        disabled: false,
                        size: Default::default(),
                        density: Default::default(),
                    },
                )),
            ],
        },
    ))
}
