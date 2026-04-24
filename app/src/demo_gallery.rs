use gui::geometry::TransformSpec;
use gui::icon::names;
use gui::paint::{Fill, ImageFilter, ImageFit, ImageOpacity, ImageSourceRect, LineCap, LineJoin};
use gui::renderer::{
    Border, Color, ImageStyle, PathData, PathStyle, Point, Shadow, Stroke, TextStyle, TextWeight,
};
use gui::theme::Theme;
use gui::tree::layout::{LeafKind, Overflow, TextAlign, TextLayout, TextOverflow, TextureHandle};
use gui::tree::Desc;
use gui::ui::{self, DecorationBuilder, StyleBuilder};
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
        primitive_section(theme, image),
        typography_section(theme),
        button_section(),
        input_section(state),
        selection_section(state),
        media_section(state, image),
        container_section(state),
        overlay_section(),
    ]
}

pub(crate) fn build_gallery_panel_content(
    theme: &Theme,
    state: &GalleryState,
    image: TextureHandle,
) -> Vec<Desc> {
    let items = build_gallery_sections(theme, state, image)
        .into_iter()
        .map(|section| section_card(theme, section))
        .collect();

    vec![
        Desc::from(ui::widget(
            Cow::Borrowed("visual_audit_gallery_list"),
            ListViewProps {
                height: 586.0,
                items,
            },
        )),
        Desc::from(ui::widget(
            Cow::Borrowed("visual_audit_caption"),
            LabelProps {
                text: Cow::Borrowed(
                    "Audit order: primitives first, then controls, panels, node cards, workspace.",
                ),
                variant: LabelVariant::Caption,
                muted: true,
            },
        )),
    ]
}

pub(crate) fn build_demo_popup() -> Desc {
    Desc::from(ui::widget(
        Cow::Borrowed("demo_popup_group"),
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

fn primitive_section(theme: &Theme, image: TextureHandle) -> GallerySection {
    GallerySection {
        id: "primitives",
        title: "Primitive Graphics",
        column: 0,
        estimated_height: 760.0,
        content: vec![
            primitive_card(
                theme,
                "primitive_rect_card",
                "Rect / radius / shadow",
                primitive_rect_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_text_card",
                "Text clip / ellipsis / align",
                primitive_text_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_media_card",
                "Image / SVG icon",
                primitive_media_stage(theme, image),
            ),
            primitive_card(
                theme,
                "primitive_vector_card",
                "Circle / line / curve / path",
                primitive_vector_stage(theme),
            ),
            primitive_card(
                theme,
                "primitive_grid_connection_card",
                "Grid / connection / transform",
                primitive_grid_connection_stage(theme),
            ),
        ],
    }
}

fn section_card(theme: &Theme, section: GallerySection) -> Desc {
    ui::column(format!("gallery_section_{}", section.id))
        .fill_width()
        .auto_height()
        .gap(theme.spacing.sm)
        .padding_all(theme.spacing.md)
        .background(theme.colors.surface)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(theme.radii.md.min(8.0))
        .child(Desc::from(ui::widget(
            Cow::Owned(format!("gallery_section_{}::title", section.id)),
            LabelProps {
                text: Cow::Borrowed(section.title),
                variant: LabelVariant::Title,
                muted: false,
            },
        )))
        .children(section.content)
        .build()
}

fn primitive_card(theme: &Theme, id: &'static str, title: &'static str, stage: Desc) -> Desc {
    ui::column(id)
        .fill_width()
        .auto_height()
        .gap(theme.spacing.xs)
        .child(Desc::from(ui::widget(
            Cow::Owned(format!("{id}::label")),
            LabelProps {
                text: Cow::Borrowed(title),
                variant: LabelVariant::Caption,
                muted: true,
            },
        )))
        .child(stage)
        .build()
}

fn primitive_stage(
    theme: &Theme,
    id: &'static str,
    width: f32,
    height: f32,
) -> gui::tree::build::ContainerBuilder {
    ui::container(id)
        .relative()
        .fixed_width(width)
        .fixed_height(height)
        .background(theme.colors.canvas_bg)
        .border(Border {
            width: 1.0,
            color: theme.colors.border,
        })
        .radius_all(theme.radii.sm.min(8.0))
        .overflow(Overflow::Hidden)
}

fn primitive_rect_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_rect_stage", 286.0, 76.0)
        .child(
            ui::container("primitive_rect_square")
                .absolute_xy(14.0, 16.0)
                .fixed_width(48.0)
                .fixed_height(44.0)
                .background(theme.colors.accent)
                .border(Border {
                    width: 2.0,
                    color: theme.colors.border_focus,
                })
                .build(),
        )
        .child(
            ui::container("primitive_rect_radius")
                .absolute_xy(80.0, 16.0)
                .fixed_width(58.0)
                .fixed_height(44.0)
                .background(theme.colors.accent_soft)
                .border(Border {
                    width: 1.5,
                    color: theme.colors.accent,
                })
                .radius([8.0, 2.0, 8.0, 2.0])
                .build(),
        )
        .child(
            ui::container("primitive_rect_shadow")
                .absolute_xy(158.0, 14.0)
                .fixed_width(92.0)
                .fixed_height(42.0)
                .background(theme.colors.surface)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.border,
                })
                .radius_all(8.0)
                .shadow(Shadow {
                    color: color(0.0, 0.0, 0.0, 0.28),
                    offset: [0.0, 8.0],
                    blur: 16.0,
                    spread: 1.0,
                })
                .build(),
        )
        .build()
}

fn primitive_text_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_text_stage", 286.0, 96.0)
        .child(
            ui::text_with_layout(
                "primitive_text_ellipsis",
                "Ellipsis should trim this long primitive text",
                theme.text_style_body_md(),
                TextLayout {
                    overflow: TextOverflow::Ellipsis,
                    align: TextAlign::Start,
                },
            )
            .absolute_xy(12.0, 12.0)
            .fixed_width(138.0)
            .fixed_height(22.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "primitive_text_center",
                "Centered",
                TextStyle {
                    color: theme.colors.accent,
                    ..theme.text_style_title_sm()
                },
                TextLayout {
                    overflow: TextOverflow::Clip,
                    align: TextAlign::Center,
                },
            )
            .absolute_xy(158.0, 12.0)
            .fixed_width(96.0)
            .fixed_height(24.0)
            .build(),
        )
        .child(
            ui::text_with_layout(
                "primitive_text_end",
                "Right aligned",
                theme.text_style_mono_md(),
                TextLayout {
                    overflow: TextOverflow::Clip,
                    align: TextAlign::End,
                },
            )
            .absolute_xy(12.0, 50.0)
            .fixed_width(242.0)
            .fixed_height(22.0)
            .build(),
        )
        .build()
}

fn primitive_media_stage(theme: &Theme, image: TextureHandle) -> Desc {
    primitive_stage(theme, "primitive_media_stage", 286.0, 92.0)
        .child(
            ui::leaf(
                "primitive_image_cover",
                LeafKind::Image {
                    texture: image,
                    style: ImageStyle::default()
                        .with_fit(ImageFit::Cover)
                        .with_filter(ImageFilter::Nearest)
                        .with_source(ImageSourceRect::new(0.1, 0.1, 0.8, 0.8)),
                },
            )
            .absolute_xy(12.0, 12.0)
            .fixed_width(74.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_image_tint",
                LeafKind::Image {
                    texture: image,
                    style: ImageStyle::default()
                        .with_fit(ImageFit::Contain)
                        .with_tint(color(1.0, 0.72, 0.36, 1.0))
                        .with_opacity(ImageOpacity::new(0.78)),
                },
            )
            .absolute_xy(104.0, 12.0)
            .fixed_width(74.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::icon(
                "primitive_svg_icon",
                names::SETTINGS,
                36.0,
                theme.colors.text,
            )
            .absolute_xy(210.0, 22.0)
            .build(),
        )
        .build()
}

fn primitive_vector_stage(theme: &Theme) -> Desc {
    let stroke = Stroke::new(3.0, theme.colors.accent)
        .with_cap(LineCap::Round)
        .with_join(LineJoin::Round);
    let path = PathData::new()
        .move_to(point(14.0, 54.0))
        .line_to(point(34.0, 18.0))
        .line_to(point(58.0, 54.0))
        .close()
        .move_to(point(28.0, 42.0))
        .line_to(point(36.0, 28.0))
        .line_to(point(44.0, 42.0))
        .close();

    primitive_stage(theme, "primitive_vector_stage", 286.0, 118.0)
        .child(
            ui::leaf(
                "primitive_circle",
                LeafKind::Circle {
                    radius: 18.0,
                    fill: Some(theme.colors.accent_soft),
                    stroke: Some(Border {
                        width: 2.0,
                        color: theme.colors.accent,
                    }),
                },
            )
            .absolute_xy(14.0, 16.0)
            .fixed_width(42.0)
            .fixed_height(42.0)
            .build(),
        )
        .child(
            ui::line(
                "primitive_line",
                point(0.0, 22.0),
                point(74.0, 6.0),
                stroke.with_cap(LineCap::Square),
            )
            .absolute_xy(74.0, 16.0)
            .fixed_width(78.0)
            .fixed_height(42.0)
            .build(),
        )
        .child(
            ui::curve(
                "primitive_curve",
                [
                    point(0.0, 34.0),
                    point(22.0, 0.0),
                    point(42.0, 58.0),
                    point(68.0, 18.0),
                ],
                stroke,
            )
            .absolute_xy(174.0, 10.0)
            .fixed_width(76.0)
            .fixed_height(58.0)
            .build(),
        )
        .child(
            ui::path(
                "primitive_path_even_odd",
                path,
                PathStyle::fill_and_stroke(
                    Fill::even_odd(theme.colors.accent_soft),
                    Stroke::new(2.0, theme.colors.text),
                ),
            )
            .absolute_xy(92.0, 60.0)
            .fixed_width(72.0)
            .fixed_height(56.0)
            .transform(TransformSpec::translate_scale_rotate([0.0, 0.0], 1.0, 0.12))
            .build(),
        )
        .build()
}

fn primitive_grid_connection_stage(theme: &Theme) -> Desc {
    primitive_stage(theme, "primitive_grid_connection_stage", 286.0, 118.0)
        .child(
            ui::leaf(
                "primitive_grid",
                LeafKind::Grid {
                    spacing: 12.0,
                    dot_color: theme.colors.canvas_grid,
                    dot_size: 1.4,
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(286.0)
            .fixed_height(118.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection_from",
                LeafKind::Circle {
                    radius: 5.0,
                    fill: Some(theme.colors.accent),
                    stroke: Some(Border {
                        width: 1.0,
                        color: theme.colors.text,
                    }),
                },
            )
            .absolute_xy(28.0, 54.0)
            .fixed_width(10.0)
            .fixed_height(10.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection_to",
                LeafKind::Circle {
                    radius: 5.0,
                    fill: Some(theme.colors.accent),
                    stroke: Some(Border {
                        width: 1.0,
                        color: theme.colors.text,
                    }),
                },
            )
            .absolute_xy(234.0, 34.0)
            .fixed_width(10.0)
            .fixed_height(10.0)
            .build(),
        )
        .child(
            ui::leaf(
                "primitive_connection",
                LeafKind::Connection {
                    from_port: Cow::Borrowed("primitive_connection_from"),
                    to_port: Cow::Borrowed("primitive_connection_to"),
                },
            )
            .absolute_xy(0.0, 0.0)
            .fixed_width(0.0)
            .fixed_height(0.0)
            .build(),
        )
        .child(
            ui::container("primitive_transform_marker")
                .absolute_xy(118.0, 68.0)
                .fixed_width(42.0)
                .fixed_height(22.0)
                .background(theme.colors.accent_soft)
                .border(Border {
                    width: 1.0,
                    color: theme.colors.accent,
                })
                .radius_all(5.0)
                .transform(TransformSpec::translate_scale_rotate(
                    [0.0, 0.0],
                    1.0,
                    -0.35,
                ))
                .build(),
        )
        .build()
}

fn overview_section() -> GallerySection {
    GallerySection {
        id: "overview",
        title: "Overview",
        column: 0,
        estimated_height: 150.0,
        content: vec![
            Desc::from(ui::widget(
                Cow::Borrowed("intro_label"),
                LabelProps {
                    text: Cow::Borrowed("GUI Component Gallery"),
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

fn typography_section(theme: &Theme) -> GallerySection {
    GallerySection {
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

fn button_section() -> GallerySection {
    GallerySection {
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

fn input_section(state: &GalleryState) -> GallerySection {
    GallerySection {
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

fn selection_section(state: &GalleryState) -> GallerySection {
    GallerySection {
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

fn media_section(state: &GalleryState, image: TextureHandle) -> GallerySection {
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

    GallerySection {
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

fn container_section(state: &GalleryState) -> GallerySection {
    GallerySection {
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

fn overlay_section() -> GallerySection {
    GallerySection {
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

fn clone_desc(desc: &Desc) -> Desc {
    desc.clone()
}

fn point(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn color(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
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
        assert!(ids.contains(&"primitives"));
        assert!(ids.contains(&"typography"));
        assert!(ids.contains(&"buttons"));
        assert!(ids.contains(&"inputs"));
        assert!(ids.contains(&"selection"));
        assert!(ids.contains(&"media"));
        assert!(ids.contains(&"containers"));
        assert!(ids.contains(&"overlay"));
    }

    #[test]
    fn primitive_section_covers_direct_primitive_leaf_kinds() {
        let theme = light_theme();
        let section = primitive_section(&theme, TextureHandle(1));
        let content = &section.content;

        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Circle { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Line { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Curve { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Path { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Image { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Icon { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Grid { .. }
        )));
        assert!(contains_leaf(content, |kind| matches!(
            kind,
            LeafKind::Connection { .. }
        )));
    }

    fn contains_leaf(descs: &[Desc], predicate: impl Fn(&LeafKind) -> bool + Copy) -> bool {
        descs.iter().any(|desc| match desc {
            Desc::Leaf { kind, .. } => predicate(kind),
            Desc::Container { children, .. } => contains_leaf(children, predicate),
            Desc::Widget(_) => false,
        })
    }
}
