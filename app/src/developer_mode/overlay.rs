use super::catalog::PlaygroundItemId;
use super::ids;
use super::state::DeveloperControlState;
use gui::tree::Desc;
use gui::ui;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::frameworks::group::GroupProps;
use std::borrow::Cow;

pub(super) fn overlay_sample(item: PlaygroundItemId) -> Desc {
    match item {
        PlaygroundItemId::Popup => Desc::from(ui::widget(
            Cow::Borrowed(ids::CONTROL_POPUP_BUTTON_ID),
            ButtonProps {
                label: Cow::Borrowed("Open"),
                icon: None,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )),
        _ => ui::container(format!("overlay_empty_{}", item.key())).build(),
    }
}

pub(super) fn build_developer_popup(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed("developer_playground_popup_group"),
        GroupProps {
            title: Cow::Borrowed("Popup"),
            content: vec![
                Desc::from(ui::widget(
                    Cow::Borrowed("developer_playground_popup_state"),
                    LabelProps {
                        text: Cow::Owned(format!("Slider value: {:.1}", state.slider_value)),
                        variant: LabelVariant::Body,
                        muted: false,
                    },
                )),
                Desc::from(ui::widget(
                    Cow::Borrowed("developer_playground_popup_caption"),
                    LabelProps {
                        text: Cow::Borrowed("Overlay content rendered by developer_mode."),
                        variant: LabelVariant::Caption,
                        muted: true,
                    },
                )),
                Desc::from(ui::widget(
                    Cow::Borrowed(ids::CONTROL_POPUP_CLOSE_ID),
                    ButtonProps {
                        label: Cow::Borrowed("Close"),
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
