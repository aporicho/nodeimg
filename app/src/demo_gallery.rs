use gui::renderer::{TextStyle, TextWeight};
use gui::theme::Theme;
use gui::tree::layout::{BoxStyle, LeafKind, TextureHandle};
use gui::tree::Desc;
use gui::widget::atoms::button::ButtonProps;
use gui::widget::atoms::checkbox::CheckboxProps;
use gui::widget::atoms::dropdown::DropdownProps;
use gui::widget::atoms::image_viewer::ImageViewerProps;
use gui::widget::atoms::label::{LabelProps, LabelVariant};
use gui::widget::atoms::number_input::NumberInputProps;
use gui::widget::atoms::radio::RadioProps;
use gui::widget::atoms::separator::{SeparatorOrientation, SeparatorProps};
use gui::widget::atoms::slider::SliderProps;
use gui::widget::atoms::text_input::TextInputProps;
use gui::widget::atoms::toggle::ToggleProps;
use gui::widget::frameworks::collapsible::CollapsibleProps;
use gui::widget::frameworks::list_view::ListViewProps;
use std::borrow::Cow;

pub(crate) const POPUP_TRIGGER_ID: &str = "popup_trigger";
pub(crate) const POPUP_CLOSE_ID: &str = "popup_close";
pub(crate) const TEXT_PROMPT_ID: &str = "text_prompt";
pub(crate) const SLIDER_RADIUS_ID: &str = "slider_radius";
pub(crate) const NUMBER_RADIUS_ID: &str = "number_radius";
pub(crate) const TOGGLE_GRID_ID: &str = "toggle_grid";
pub(crate) const ADVANCED_TOGGLE_ID: &str = "advanced_toggle";
pub(crate) const CHECKBOX_SNAP_ID: &str = "checkbox_snap";
pub(crate) const RADIO_FAST_ID: &str = "radio_quality_fast";
pub(crate) const RADIO_BALANCED_ID: &str = "radio_quality_balanced";
pub(crate) const DROPDOWN_BLEND_ID: &str = "dropdown_blend";
pub(crate) const ADVANCED_SECTION_ID: &str = "advanced_section";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QualityMode {
    Fast,
    Balanced,
}

#[derive(Debug, Clone)]
pub(crate) struct GalleryState {
    pub text_value: String,
    pub toggle_value: bool,
    pub snap_to_grid: bool,
    pub quality_mode: QualityMode,
    pub advanced_open: bool,
    pub blend_mode: usize,
    pub slider_value: f32,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            text_value: "Hello nodeimg".to_string(),
            toggle_value: true,
            snap_to_grid: true,
            quality_mode: QualityMode::Balanced,
            advanced_open: true,
            blend_mode: 0,
            slider_value: 5.0,
        }
    }
}

impl GalleryState {
    pub(crate) fn apply_click(&mut self, id: &str) -> bool {
        match id {
            TOGGLE_GRID_ID | ADVANCED_TOGGLE_ID => {
                self.toggle_value = !self.toggle_value;
                true
            }
            CHECKBOX_SNAP_ID => {
                self.snap_to_grid = !self.snap_to_grid;
                true
            }
            RADIO_FAST_ID => {
                self.quality_mode = QualityMode::Fast;
                true
            }
            RADIO_BALANCED_ID => {
                self.quality_mode = QualityMode::Balanced;
                true
            }
            ADVANCED_SECTION_ID => {
                self.advanced_open = !self.advanced_open;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_text_change(&mut self, id: &str, value: String) -> bool {
        match id {
            TEXT_PROMPT_ID => {
                self.text_value = value;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_number_change(&mut self, id: &str, value: f32) -> bool {
        match id {
            NUMBER_RADIUS_ID => {
                self.slider_value = value;
                true
            }
            _ => false,
        }
    }

    pub(crate) fn apply_select_change(&mut self, id: &str, selected: usize) -> bool {
        match id {
            DROPDOWN_BLEND_ID => {
                self.blend_mode = selected;
                true
            }
            _ => false,
        }
    }
}

pub(crate) struct GallerySection {
    pub id: &'static str,
    pub title: &'static str,
    pub column: usize,
    pub estimated_height: f32,
    pub content: Vec<Desc>,
}

impl Clone for GallerySection {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            title: self.title,
            column: self.column,
            estimated_height: self.estimated_height,
            content: self.content.iter().map(clone_desc).collect(),
        }
    }
}

impl std::fmt::Debug for GallerySection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GallerySection")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("column", &self.column)
            .field("estimated_height", &self.estimated_height)
            .field("content_len", &self.content.len())
            .finish()
    }
}

pub(crate) fn build_gallery_sections(
    theme: &Theme,
    state: &GalleryState,
    image: TextureHandle,
) -> Vec<GallerySection> {
    vec![
        overview_section(),
        typography_section(theme),
        button_section(),
        input_section(state),
        selection_section(state),
        media_section(state, image),
        container_section(state),
        overlay_section(),
    ]
}

pub(crate) fn build_demo_popup() -> Desc {
    Desc::Widget(gui::widget::WidgetDesc::new(
        Cow::Borrowed("demo_popup_group"),
        gui::widget::frameworks::group::GroupProps {
            title: Cow::Borrowed("Quick Actions"),
            content: vec![
                Desc::Widget(gui::widget::WidgetDesc::new(
                    Cow::Borrowed("popup_label"),
                    LabelProps {
                        text: Cow::Borrowed(
                            "Overlay 由 framework 持有，支持 outside click、Escape 和焦点恢复。",
                        ),
                        variant: LabelVariant::Caption,
                        muted: true,
                    },
                )),
                Desc::Widget(gui::widget::WidgetDesc::new(
                    Cow::Borrowed("popup_separator"),
                    SeparatorProps {
                        orientation: SeparatorOrientation::Horizontal,
                    },
                )),
                Desc::Widget(gui::widget::WidgetDesc::new(
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

fn overview_section() -> GallerySection {
    GallerySection {
        id: "overview",
        title: "Overview",
        column: 0,
        estimated_height: 150.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("intro_label"),
                LabelProps {
                    text: Cow::Borrowed("GUI Component Gallery"),
                    variant: LabelVariant::Title,
                    muted: false,
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn typography_section(theme: &Theme) -> GallerySection {
    GallerySection {
        id: "typography",
        title: "Typography",
        column: 1,
        estimated_height: 260.0,
        content: vec![
            Desc::Leaf {
                id: Cow::Borrowed("text_style_body"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Body / Sans / Regular".to_string(),
                    style: theme.text_style_body_md(),
                    layout: Default::default(),
                },
            },
            Desc::Leaf {
                id: Cow::Borrowed("text_style_title"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Title / Sans / Semibold".to_string(),
                    style: theme.text_style_title_sm(),
                    layout: Default::default(),
                },
            },
            Desc::Leaf {
                id: Cow::Borrowed("text_style_italic"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Caption / Sans / Italic".to_string(),
                    style: theme.text_style_label_sm().with_italic(true),
                    layout: Default::default(),
                },
            },
            Desc::Leaf {
                id: Cow::Borrowed("text_style_mono"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Mono / Medium / Value 42.00".to_string(),
                    style: theme.text_style_mono_md(),
                    layout: Default::default(),
                },
            },
            Desc::Leaf {
                id: Cow::Borrowed("text_style_bold"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Body / Sans / Bold override".to_string(),
                    style: TextStyle {
                        color: theme.colors.text,
                        ..theme.text_style_body_md().with_weight(TextWeight::Bold)
                    },
                    layout: Default::default(),
                },
            },
            Desc::Leaf {
                id: Cow::Borrowed("text_style_disabled"),
                style: BoxStyle::default(),
                kind: LeafKind::Text {
                    content: "Caption / Disabled text color".to_string(),
                    style: TextStyle {
                        color: theme.colors.text_disabled,
                        ..theme.text_style_label_sm()
                    },
                    layout: Default::default(),
                },
            },
        ],
    }
}

fn button_section() -> GallerySection {
    GallerySection {
        id: "buttons",
        title: "Buttons",
        column: 2,
        estimated_height: 220.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("btn_primary"),
                ButtonProps {
                    label: Cow::Borrowed("Primary Action"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("btn_long"),
                ButtonProps {
                    label: Cow::Borrowed("A very long button label to test horizontal layout"),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn input_section(state: &GalleryState) -> GallerySection {
    GallerySection {
        id: "inputs",
        title: "Inputs",
        column: 3,
        estimated_height: 420.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(TEXT_PROMPT_ID),
                TextInputProps {
                    label: Some(Cow::Borrowed("Prompt")),
                    value: Cow::Owned(state.text_value.clone()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("text_prompt_disabled"),
                TextInputProps {
                    label: Some(Cow::Borrowed("Disabled Prompt")),
                    value: Cow::Borrowed("Disabled but visible"),
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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
            Desc::Widget(gui::widget::WidgetDesc::new(
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
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn selection_section(state: &GalleryState) -> GallerySection {
    GallerySection {
        id: "selection",
        title: "Selection",
        column: 0,
        estimated_height: 420.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(TOGGLE_GRID_ID),
                ToggleProps {
                    label: Some(Cow::Borrowed("Show Grid")),
                    value: state.toggle_value,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("toggle_grid_disabled"),
                ToggleProps {
                    label: Some(Cow::Borrowed("Disabled Toggle")),
                    value: true,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(CHECKBOX_SNAP_ID),
                CheckboxProps {
                    label: Some(Cow::Borrowed("Snap to grid")),
                    checked: state.snap_to_grid,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("checkbox_disabled"),
                CheckboxProps {
                    label: Some(Cow::Borrowed("Disabled checkbox")),
                    checked: true,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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
            Desc::Widget(gui::widget::WidgetDesc::new(
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
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(RADIO_FAST_ID),
                RadioProps {
                    label: Some(Cow::Borrowed("Fast quality")),
                    selected: state.quality_mode == QualityMode::Fast,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn media_section(state: &GalleryState, image: TextureHandle) -> GallerySection {
    let list_items = (1..=18)
        .map(|index| {
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Owned(format!("history_item_{index}")),
                LabelProps {
                    text: Cow::Owned(format!("Preset #{index:02} · Gaussian Blur")),
                    variant: LabelVariant::Body,
                    muted: index % 2 == 0,
                },
            ))
        })
        .collect();

    GallerySection {
        id: "media",
        title: "Scrolling & Media",
        column: 1,
        estimated_height: 360.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("preview_label"),
                LabelProps {
                    text: Cow::Borrowed("Preview"),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("preview_image"),
                ImageViewerProps {
                    texture: image,
                    height: 96.0,
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("history_list"),
                ListViewProps {
                    height: 128.0,
                    items: list_items,
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn container_section(state: &GalleryState) -> GallerySection {
    GallerySection {
        id: "containers",
        title: "Containers",
        column: 2,
        estimated_height: if state.advanced_open { 320.0 } else { 210.0 },
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed(ADVANCED_SECTION_ID),
                CollapsibleProps {
                    title: Cow::Borrowed("Advanced"),
                    expanded: state.advanced_open,
                    disabled: false,
                    content: vec![
                        Desc::Widget(gui::widget::WidgetDesc::new(
                            Cow::Borrowed("advanced_label"),
                            LabelProps {
                                text: Cow::Borrowed(
                                    "Controlled collapsible: 点击头部由 app 状态切换 expanded。",
                                ),
                                variant: LabelVariant::Caption,
                                muted: true,
                            },
                        )),
                        Desc::Widget(gui::widget::WidgetDesc::new(
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
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn overlay_section() -> GallerySection {
    GallerySection {
        id: "overlay",
        title: "Overlay",
        column: 3,
        estimated_height: 170.0,
        content: vec![
            Desc::Widget(gui::widget::WidgetDesc::new(
                Cow::Borrowed("overlay_label"),
                LabelProps {
                    text: Cow::Borrowed(
                        "Popup 和 Dropdown 都建立在 framework-owned overlay host 之上。",
                    ),
                    variant: LabelVariant::Caption,
                    muted: true,
                },
            )),
            Desc::Widget(gui::widget::WidgetDesc::new(
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

fn clone_desc(desc: &Desc) -> Desc {
    desc.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gui::theme::light_theme;

    #[test]
    fn gallery_contains_expected_sections() {
        let sections =
            build_gallery_sections(&light_theme(), &GalleryState::default(), TextureHandle(1));
        let ids: Vec<&str> = sections.iter().map(|section| section.id).collect();

        assert!(ids.contains(&"overview"));
        assert!(ids.contains(&"typography"));
        assert!(ids.contains(&"buttons"));
        assert!(ids.contains(&"inputs"));
        assert!(ids.contains(&"selection"));
        assert!(ids.contains(&"media"));
        assert!(ids.contains(&"containers"));
        assert!(ids.contains(&"overlay"));
    }
}
