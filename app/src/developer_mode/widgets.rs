use super::catalog::PlaygroundItemId;
use super::ids;
use super::state::{DeveloperControlState, PlaygroundQuality};
use gui::renderer::ImageStyle;
use gui::theme::Theme;
use gui::tree::layout::{Align, Overflow, TextureHandle};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::checkbox::CheckboxProps;
use gui::widget::atoms::dropdown::DropdownProps;
use gui::widget::atoms::image_viewer::ImageViewerProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::atoms::number_input::NumberInputProps;
use gui::widget::atoms::radio::RadioProps;
use gui::widget::atoms::slider::SliderProps;
use gui::widget::atoms::text_input::TextInputProps;
use gui::widget::atoms::toggle::ToggleProps;
use std::borrow::Cow;

pub(super) fn control_sample(
    item: PlaygroundItemId,
    theme: &Theme,
    state: &DeveloperControlState,
    image: TextureHandle,
) -> Desc {
    match item {
        PlaygroundItemId::Label => label_sample(theme),
        PlaygroundItemId::Button => button_sample(state),
        PlaygroundItemId::TextInput => text_input_sample(state),
        PlaygroundItemId::Slider => slider_sample(state),
        PlaygroundItemId::NumberInput => number_input_sample(state),
        PlaygroundItemId::Toggle => toggle_sample(state),
        PlaygroundItemId::Checkbox => checkbox_sample(state),
        PlaygroundItemId::Radio => radio_sample(state),
        PlaygroundItemId::Dropdown => dropdown_sample(state),
        PlaygroundItemId::ImageViewer => image_viewer_sample(image),
        _ => ui::container(format!("control_empty_{}", item.key())).build(),
    }
}

fn stage(id: impl Into<Cow<'static, str>>, theme: &Theme) -> gui::ui::ContainerBuilder {
    ui::container(id)
        .fill_width()
        .fill_height()
        .overflow(Overflow::Hidden)
        .align_items(Align::Stretch)
        .background(theme.colors.canvas_bg)
        .radius_all(6.0)
}

fn label_sample(theme: &Theme) -> Desc {
    stage("control_label_sample", theme)
        .gap(3.0)
        .padding_all(6.0)
        .child(Desc::from(ui::widget(
            Cow::Borrowed(ids::CONTROL_LABEL_ID),
            LabelProps {
                text: Cow::Borrowed("Label title"),
                variant: LabelVariant::Title,
                muted: false,
            },
        )))
        .child(Desc::from(ui::widget(
            Cow::Borrowed("playground_control_label_caption"),
            LabelProps {
                text: Cow::Borrowed("Caption value"),
                variant: LabelVariant::Caption,
                muted: true,
            },
        )))
        .build()
}

fn button_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_BUTTON_ID),
        ButtonProps {
            label: Cow::Owned(if state.button_clicks == 0 {
                "Run".to_string()
            } else {
                format!("Run {}", state.button_clicks)
            }),
            icon: None,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn text_input_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_TEXT_INPUT_ID),
        TextInputProps {
            label: Some(Cow::Borrowed("Text")),
            value: Cow::Owned(state.text_value.clone()),
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn slider_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_SLIDER_ID),
        SliderProps {
            label: Some(Cow::Borrowed("Value")),
            min: 0.0,
            max: 10.0,
            step: 0.1,
            value: state.slider_value,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn number_input_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_NUMBER_INPUT_ID),
        NumberInputProps {
            label: Some(Cow::Borrowed("Value")),
            value: state.slider_value,
            min: 0.0,
            max: 10.0,
            step: 0.1,
            precision: 1,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn toggle_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_TOGGLE_ID),
        ToggleProps {
            label: Some(Cow::Borrowed("Enabled")),
            value: state.toggle_enabled,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn checkbox_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_CHECKBOX_ID),
        CheckboxProps {
            label: Some(Cow::Borrowed("Checked")),
            checked: state.checkbox_checked,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn radio_sample(state: &DeveloperControlState) -> Desc {
    ui::column("control_radio_sample")
        .gap(4.0)
        .child(Desc::from(ui::widget(
            Cow::Borrowed(ids::CONTROL_RADIO_FAST_ID),
            RadioProps {
                label: Some(Cow::Borrowed("Fast")),
                selected: state.quality == PlaygroundQuality::Fast,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )))
        .child(Desc::from(ui::widget(
            Cow::Borrowed(ids::CONTROL_RADIO_BALANCED_ID),
            RadioProps {
                label: Some(Cow::Borrowed("Balanced")),
                selected: state.quality == PlaygroundQuality::Balanced,
                disabled: false,
                size: Default::default(),
                density: Default::default(),
            },
        )))
        .build()
}

fn dropdown_sample(state: &DeveloperControlState) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_DROPDOWN_ID),
        DropdownProps {
            label: Some(Cow::Borrowed("Mode")),
            options: vec![
                Cow::Borrowed("Normal"),
                Cow::Borrowed("Multiply"),
                Cow::Borrowed("Screen"),
            ],
            selected: state.dropdown_selected,
            disabled: false,
            size: Default::default(),
            density: Default::default(),
        },
    ))
}

fn image_viewer_sample(image: TextureHandle) -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed(ids::CONTROL_IMAGE_VIEWER_ID),
        ImageViewerProps {
            texture: image,
            height: 54.0,
            style: ImageStyle::default(),
        },
    ))
}
