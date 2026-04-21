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
    Desc::Widget {
        id: Cow::Borrowed("demo_popup_group"),
        props: Box::new(gui::widget::frameworks::group::GroupProps {
            title: Cow::Borrowed("Quick Actions"),
            content: vec![
                Desc::Widget {
                    id: Cow::Borrowed("popup_label"),
                    props: Box::new(LabelProps {
                        text: Cow::Borrowed(
                            "Overlay 由 framework 持有，支持 outside click、Escape 和焦点恢复。",
                        ),
                        variant: LabelVariant::Caption,
                        muted: true,
                    }),
                },
                Desc::Widget {
                    id: Cow::Borrowed("popup_separator"),
                    props: Box::new(SeparatorProps {
                        orientation: SeparatorOrientation::Horizontal,
                    }),
                },
                Desc::Widget {
                    id: Cow::Borrowed(POPUP_CLOSE_ID),
                    props: Box::new(ButtonProps {
                        label: "Close Popup".into(),
                        icon: None,
                        disabled: false,
                        size: Default::default(),
                        density: Default::default(),
                    }),
                },
            ],
        }),
    }
}

fn overview_section() -> GallerySection {
    GallerySection {
        id: "overview",
        title: "Overview",
        column: 0,
        estimated_height: 150.0,
        content: vec![
            Desc::Widget {
                id: Cow::Borrowed("intro_label"),
                props: Box::new(LabelProps {
                    text: Cow::Borrowed("GUI Component Gallery"),
                    variant: LabelVariant::Title,
                    muted: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("intro_caption"),
                props: Box::new(LabelProps {
                    text: Cow::Borrowed(
                        "这个页面是基础控件的可视回归基线，包含正常态、禁用态和边界态。",
                    ),
                    variant: LabelVariant::Caption,
                    muted: true,
                }),
            },
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
            Desc::Widget {
                id: Cow::Borrowed("btn_primary"),
                props: Box::new(ButtonProps {
                    label: "Primary Action".into(),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("btn_long"),
                props: Box::new(ButtonProps {
                    label: "A very long button label to test horizontal layout".into(),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("btn_disabled"),
                props: Box::new(ButtonProps {
                    label: "Disabled Action".into(),
                    icon: None,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
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
            Desc::Widget {
                id: Cow::Borrowed(TEXT_PROMPT_ID),
                props: Box::new(TextInputProps {
                    label: "Prompt".into(),
                    value: Cow::Owned(state.text_value.clone()),
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("text_prompt_disabled"),
                props: Box::new(TextInputProps {
                    label: "Disabled Prompt".into(),
                    value: Cow::Borrowed("Disabled but visible"),
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(SLIDER_RADIUS_ID),
                props: Box::new(SliderProps {
                    label: "Radius".into(),
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    value: state.slider_value,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(NUMBER_RADIUS_ID),
                props: Box::new(NumberInputProps {
                    label: "Radius value".into(),
                    value: state.slider_value,
                    min: 0.0,
                    max: 10.0,
                    step: 0.1,
                    precision: 2,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("number_radius_disabled"),
                props: Box::new(NumberInputProps {
                    label: "Disabled Number".into(),
                    value: 42.0,
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                    precision: 0,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
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
            Desc::Widget {
                id: Cow::Borrowed(TOGGLE_GRID_ID),
                props: Box::new(ToggleProps {
                    label: "Show Grid".into(),
                    value: state.toggle_value,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("toggle_grid_disabled"),
                props: Box::new(ToggleProps {
                    label: "Disabled Toggle".into(),
                    value: true,
                    disabled: true,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(CHECKBOX_SNAP_ID),
                props: Box::new(CheckboxProps {
                    label: "Snap to grid".into(),
                    checked: state.snap_to_grid,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("checkbox_disabled"),
                props: Box::new(CheckboxProps {
                    label: "Disabled checkbox".into(),
                    checked: true,
                    disabled: true,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(DROPDOWN_BLEND_ID),
                props: Box::new(DropdownProps {
                    label: Cow::Borrowed("Blend Mode"),
                    options: vec![
                        Cow::Borrowed("Normal"),
                        Cow::Borrowed("Multiply"),
                        Cow::Borrowed("Screen"),
                    ],
                    selected: state.blend_mode,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("dropdown_disabled"),
                props: Box::new(DropdownProps {
                    label: Cow::Borrowed("Disabled Select"),
                    options: vec![Cow::Borrowed("One"), Cow::Borrowed("Two")],
                    selected: 1,
                    disabled: true,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(RADIO_FAST_ID),
                props: Box::new(RadioProps {
                    label: "Fast quality".into(),
                    selected: state.quality_mode == QualityMode::Fast,
                    disabled: false,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(RADIO_BALANCED_ID),
                props: Box::new(RadioProps {
                    label: "Balanced quality".into(),
                    selected: state.quality_mode == QualityMode::Balanced,
                    disabled: false,
                }),
            },
        ],
    }
}

fn media_section(state: &GalleryState, image: TextureHandle) -> GallerySection {
    let list_items = (1..=18)
        .map(|index| Desc::Widget {
            id: Cow::Owned(format!("history_item_{index}")),
            props: Box::new(LabelProps {
                text: Cow::Owned(format!("Preset #{index:02} · Gaussian Blur")),
                variant: LabelVariant::Body,
                muted: index % 2 == 0,
            }),
        })
        .collect();

    GallerySection {
        id: "media",
        title: "Scrolling & Media",
        column: 1,
        estimated_height: 360.0,
        content: vec![
            Desc::Widget {
                id: Cow::Borrowed("preview_label"),
                props: Box::new(LabelProps {
                    text: Cow::Borrowed("Preview"),
                    variant: LabelVariant::Caption,
                    muted: true,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("preview_image"),
                props: Box::new(ImageViewerProps {
                    texture: image,
                    height: 96.0,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("history_list"),
                props: Box::new(ListViewProps {
                    height: 128.0,
                    items: list_items,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("history_caption"),
                props: Box::new(LabelProps {
                    text: Cow::Owned(format!(
                        "Current blend mode: {}",
                        ["Normal", "Multiply", "Screen"][state.blend_mode]
                    )),
                    variant: LabelVariant::Caption,
                    muted: true,
                }),
            },
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
            Desc::Widget {
                id: Cow::Borrowed(ADVANCED_SECTION_ID),
                props: Box::new(CollapsibleProps {
                    title: Cow::Borrowed("Advanced"),
                    expanded: state.advanced_open,
                    disabled: false,
                    content: vec![
                        Desc::Widget {
                            id: Cow::Borrowed("advanced_label"),
                            props: Box::new(LabelProps {
                                text: Cow::Borrowed(
                                    "Controlled collapsible: 点击头部由 app 状态切换 expanded。",
                                ),
                                variant: LabelVariant::Caption,
                                muted: true,
                            }),
                        },
                        Desc::Widget {
                            id: Cow::Borrowed(ADVANCED_TOGGLE_ID),
                            props: Box::new(ToggleProps {
                                label: "Use denoise pass".into(),
                                value: state.toggle_value,
                                disabled: false,
                            }),
                        },
                    ],
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed("collapsible_disabled"),
                props: Box::new(CollapsibleProps {
                    title: Cow::Borrowed("Disabled / collapsed preview"),
                    expanded: false,
                    disabled: true,
                    content: vec![],
                }),
            },
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
            Desc::Widget {
                id: Cow::Borrowed("overlay_label"),
                props: Box::new(LabelProps {
                    text: Cow::Borrowed(
                        "Popup 和 Dropdown 都建立在 framework-owned overlay host 之上。",
                    ),
                    variant: LabelVariant::Caption,
                    muted: true,
                }),
            },
            Desc::Widget {
                id: Cow::Borrowed(POPUP_TRIGGER_ID),
                props: Box::new(ButtonProps {
                    label: "Open Popup".into(),
                    icon: None,
                    disabled: false,
                    size: Default::default(),
                    density: Default::default(),
                }),
            },
        ],
    }
}

fn clone_desc(desc: &Desc) -> Desc {
    match desc {
        Desc::Container {
            id,
            style,
            decoration,
            children,
        } => Desc::Container {
            id: id.clone(),
            style: style.clone(),
            decoration: decoration.clone(),
            children: children.iter().map(clone_desc).collect(),
        },
        Desc::Leaf { id, style, kind } => Desc::Leaf {
            id: id.clone(),
            style: style.clone(),
            kind: kind.clone(),
        },
        Desc::Widget { id, props } => Desc::Widget {
            id: id.clone(),
            props: props.clone_box(),
        },
    }
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
