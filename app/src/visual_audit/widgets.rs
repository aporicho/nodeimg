use super::section::VisualAuditSection;
use super::state::{
    QualityMode, VisualAuditState, ADVANCED_SECTION_ID, ADVANCED_TOGGLE_ID, CHECKBOX_SNAP_ID,
    DROPDOWN_BLEND_ID, NUMBER_RADIUS_ID, POPUP_TRIGGER_ID, RADIO_BALANCED_ID, RADIO_FAST_ID,
    SLIDER_RADIUS_ID, TEXT_PROMPT_ID, TOGGLE_GRID_ID,
};
use gui::renderer::{ImageStyle, TextStyle, TextWeight};
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;
use gui::tree::Desc;
use gui::ui;
use gui::ui::StyleBuilder;
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
use gui::widget::frameworks::collapsible::CollapsibleProps;
use gui::widget::frameworks::list_view::ListViewProps;
use std::borrow::Cow;

pub(super) fn overview_section() -> VisualAuditSection {
    VisualAuditSection {
        id: "overview",
        title: "Overview",
        column: 0,
        estimated_height: 150.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed("intro_label"),
                LabelProps {
                    text: Cow::Borrowed("GUI Visual Audit Baseline"),
                    variant: LabelVariant::Title,
                    muted: false,
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("intro_caption"),
                LabelProps {
                    text: Cow::Borrowed(
                        "这个页面是基础控件的可视回归基线，包含正常态、禁用态和边界态。",
                    ),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
        ],
    }
}

pub(super) fn typography_section(theme: &Theme) -> VisualAuditSection {
    VisualAuditSection {
        id: "typography",
        title: "Typography",
        column: 1,
        estimated_height: 260.0,
        content: vec![
            ui::text(
                "text_style_body",
                "Body / Sans / Regular",
                theme.text_style_body_md(),
            )
            .auto_width()
            .auto_height()
            .build(),
            ui::text(
                "text_style_title",
                "Title / Sans / Semibold",
                theme.text_style_title_sm(),
            )
            .auto_width()
            .auto_height()
            .build(),
            ui::text(
                "text_style_italic",
                "Caption / Sans / Italic",
                theme.text_style_label_sm().with_italic(true),
            )
            .auto_width()
            .auto_height()
            .build(),
            ui::text(
                "text_style_mono",
                "Mono / Medium / Value 42.00",
                theme.text_style_mono_md(),
            )
            .auto_width()
            .auto_height()
            .build(),
            ui::text(
                "text_style_bold",
                "Body / Sans / Bold override",
                TextStyle {
                    color: theme.colors.text,
                    ..theme.text_style_body_md().with_weight(TextWeight::Bold)
                },
            )
            .auto_width()
            .auto_height()
            .build(),
            ui::text(
                "text_style_disabled",
                "Caption / Disabled text color",
                TextStyle {
                    color: theme.colors.text_disabled,
                    ..theme.text_style_label_sm()
                },
            )
            .auto_width()
            .auto_height()
            .build(),
        ],
    }
}

pub(super) fn button_section() -> VisualAuditSection {
    VisualAuditSection {
        id: "buttons",
        title: "Buttons",
        column: 2,
        estimated_height: 220.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed("btn_primary"),
                ButtonProps {
                    label: Cow::Borrowed("Primary Action"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("btn_long"),
                ButtonProps {
                    label: Cow::Borrowed("A very long button label to test horizontal layout"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("btn_disabled"),
                ButtonProps {
                    label: Cow::Borrowed("Disabled Action"),
                    icon: None,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
        ],
    }
}

pub(super) fn input_section(state: &VisualAuditState) -> VisualAuditSection {
    VisualAuditSection {
        id: "inputs",
        title: "Inputs",
        column: 3,
        estimated_height: 420.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed(TEXT_PROMPT_ID),
                TextInputProps {
                    label: Some(Cow::Borrowed("Prompt")),
                    value: Cow::Owned(state.text_value.clone()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("text_prompt_disabled"),
                TextInputProps {
                    label: Some(Cow::Borrowed("Disabled Prompt")),
                    value: Cow::Borrowed("Disabled but visible"),
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(SLIDER_RADIUS_ID),
                SliderProps {
                    label: Some(Cow::Borrowed("Radius")),
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    value: state.slider_value,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(NUMBER_RADIUS_ID),
                NumberInputProps {
                    label: Some(Cow::Borrowed("Radius value")),
                    value: state.slider_value,
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    precision: 2,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("number_radius_disabled"),
                NumberInputProps {
                    label: Some(Cow::Borrowed("Disabled Number")),
                    value: 42.0,
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    precision: 0,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
        ],
    }
}

pub(super) fn selection_section(state: &VisualAuditState) -> VisualAuditSection {
    VisualAuditSection {
        id: "selection",
        title: "Selection",
        column: 0,
        estimated_height: 420.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed(TOGGLE_GRID_ID),
                ToggleProps {
                    label: Some(Cow::Borrowed("Show Grid")),
                    value: state.toggle_value,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("toggle_grid_disabled"),
                ToggleProps {
                    label: Some(Cow::Borrowed("Disabled Toggle")),
                    value: true,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(CHECKBOX_SNAP_ID),
                CheckboxProps {
                    label: Some(Cow::Borrowed("Snap to grid")),
                    checked: state.snap_to_grid,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("checkbox_disabled"),
                CheckboxProps {
                    label: Some(Cow::Borrowed("Disabled checkbox")),
                    checked: true,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(DROPDOWN_BLEND_ID),
                DropdownProps {
                    label: Some(Cow::Borrowed("Blend Mode")),
                    options: vec![
                        Cow::Borrowed("Normal"),
                        Cow::Borrowed("Multiply"),
                        Cow::Borrowed("Screen"),
                    ],
                    selected: state.blend_mode,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("dropdown_disabled"),
                DropdownProps {
                    label: Some(Cow::Borrowed("Disabled Select")),
                    options: vec![Cow::Borrowed("One"), Cow::Borrowed("Two")],
                    selected: 1,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(RADIO_FAST_ID),
                RadioProps {
                    label: Some(Cow::Borrowed("Fast quality")),
                    selected: state.quality_mode == QualityMode::Fast,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(RADIO_BALANCED_ID),
                RadioProps {
                    label: Some(Cow::Borrowed("Balanced quality")),
                    selected: state.quality_mode == QualityMode::Balanced,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
        ],
    }
}

pub(super) fn media_section(state: &VisualAuditState, image: TextureHandle) -> VisualAuditSection {
    let list_items = (1..=18)
        .map(|index| {
            Desc::from(ui::widget(
                Cow::Owned(format!("history_item_{index}")),
                LabelProps {
                    text: Cow::Owned(format!("Preset #{index:02} · Gaussian Blur")),
                    variant: LabelVariant::Body,
                    muted: index % 2 == 0,
                },
            ))
        })
        .collect();

    VisualAuditSection {
        id: "media",
        title: "Scrolling & Media",
        column: 1,
        estimated_height: 360.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed("preview_label"),
                LabelProps {
                    text: Cow::Borrowed("Preview"),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("preview_image"),
                ImageViewerProps {
                    texture: image,
                    height: 96.0,
                    style: ImageStyle::default(),
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("history_list"),
                ListViewProps {
                    height: 128.0,
                    items: list_items,
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("history_caption"),
                LabelProps {
                    text: Cow::Owned(format!(
                        "Current blend mode: {}",
                        ["Normal", "Multiply", "Screen"][state.blend_mode]
                    )),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
        ],
    }
}

pub(super) fn container_section(state: &VisualAuditState) -> VisualAuditSection {
    VisualAuditSection {
        id: "containers",
        title: "Containers",
        column: 2,
        estimated_height: if state.advanced_open { 320.0 } else { 210.0 },
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed(ADVANCED_SECTION_ID),
                CollapsibleProps {
                    title: Cow::Borrowed("Advanced"),
                    expanded: state.advanced_open,
                    disabled: false,
                    content: vec![
                        Desc::from(ui::widget(
                            Cow::Borrowed("advanced_label"),
                            LabelProps {
                                text: Cow::Borrowed(
                                    "Controlled collapsible: 点击头部由 app 状态切换 expanded。",
                                ),
                                variant: LabelVariant::Caption,
                                muted: true,
                            },
                        )),
                        Desc::from(ui::widget(
                            Cow::Borrowed(ADVANCED_TOGGLE_ID),
                            ToggleProps {
                                label: Some(Cow::Borrowed("Use denoise pass")),
                                value: state.toggle_value,
                                disabled: false,
                                size: Default::default(),
                                density: Default::default(),
                            },
                        )),
                    ],
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed("collapsible_disabled"),
                CollapsibleProps {
                    title: Cow::Borrowed("Disabled / collapsed preview"),
                    expanded: false,
                    disabled: true,
                    content: vec![],
                },
            )),
        ],
    }
}

pub(super) fn overlay_section() -> VisualAuditSection {
    VisualAuditSection {
        id: "overlay",
        title: "Overlay",
        column: 3,
        estimated_height: 170.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed("overlay_label"),
                LabelProps {
                    text: Cow::Borrowed(
                        "Popup 和 Dropdown 都建立在 framework-owned overlay host 之上。",
                    ),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
            Desc::from(ui::widget(
                Cow::Borrowed(POPUP_TRIGGER_ID),
                ButtonProps {
                    label: Cow::Borrowed("Open Popup"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
        ],
    }
}
