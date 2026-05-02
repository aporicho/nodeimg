use crate::interaction::ControlVisualState;
use crate::renderer::{Color, TextFamily, TextStyle, TextWeight};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub revision: u64,
    pub mode: ThemeMode,
    pub colors: ThemeColors,
    pub text: ThemeText,
    pub radii: ThemeRadii,
    pub spacing: ThemeSpacing,
    pub controls: ThemeControls,
    pub components: ThemeComponents,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub canvas_bg: Color,
    pub canvas_grid: Color,
    pub surface: Color,
    pub surface_hover: Color,
    pub surface_pressed: Color,
    pub surface_disabled: Color,
    pub border: Color,
    pub border_hover: Color,
    pub border_pressed: Color,
    pub border_focus: Color,
    pub border_disabled: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_disabled: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_pressed: Color,
    pub accent_disabled: Color,
    pub accent_soft: Color,
    pub caret: Color,
    pub connection: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeText {
    pub label_sm: f32,
    pub body_sm: f32,
    pub body_md: f32,
    pub title_sm: f32,
    pub default_line_height: f32,
    pub body_family: TextFamily,
    pub mono_family: TextFamily,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeRadii {
    pub sm: f32,
    pub md: f32,
    pub pill: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeSpacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeComponents {
    pub button: ButtonTheme,
    pub checkbox: CheckboxTheme,
    pub collapsible: CollapsibleTheme,
    pub dropdown: DropdownTheme,
    pub group: GroupTheme,
    pub image_viewer: ImageViewerTheme,
    pub list_view: ListViewTheme,
    pub number_input: TextInputTheme,
    pub radio: RadioTheme,
    pub scroll_area: ScrollAreaTheme,
    pub separator: SeparatorTheme,
    pub slider: SliderTheme,
    pub toggle: ToggleTheme,
    pub text_input: TextInputTheme,
    pub panel: PanelTheme,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlSize {
    Small,
    #[default]
    Medium,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    Compact,
    #[default]
    Regular,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    pub height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub gap: f32,
    pub font_size: f32,
    pub label_font_size: f32,
    pub icon_size: f32,
    pub radius: f32,
    pub border_width: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThemeControls {
    pub small_compact: ControlMetrics,
    pub small_regular: ControlMetrics,
    pub medium_compact: ControlMetrics,
    pub medium_regular: ControlMetrics,
    pub large_compact: ControlMetrics,
    pub large_regular: ControlMetrics,
}

impl ThemeControls {
    pub(crate) fn scale(&mut self, factor: f32) {
        self.small_compact.scale(factor);
        self.small_regular.scale(factor);
        self.medium_compact.scale(factor);
        self.medium_regular.scale(factor);
        self.large_compact.scale(factor);
        self.large_regular.scale(factor);
    }
}

impl Default for ThemeControls {
    fn default() -> Self {
        Self {
            small_compact: ControlMetrics {
                height: 24.0,
                padding_x: 6.0,
                padding_y: 4.0,
                gap: 4.0,
                font_size: 11.0,
                label_font_size: 11.0,
                icon_size: 12.0,
                radius: 4.0,
                border_width: 1.0,
            },
            small_regular: ControlMetrics {
                height: 28.0,
                padding_x: 8.0,
                padding_y: 5.0,
                gap: 6.0,
                font_size: 12.0,
                label_font_size: 11.0,
                icon_size: 14.0,
                radius: 4.0,
                border_width: 1.0,
            },
            medium_compact: ControlMetrics {
                height: 32.0,
                padding_x: 10.0,
                padding_y: 6.0,
                gap: 6.0,
                font_size: 12.0,
                label_font_size: 11.0,
                icon_size: 14.0,
                radius: 4.0,
                border_width: 1.0,
            },
            medium_regular: ControlMetrics {
                height: 36.0,
                padding_x: 12.0,
                padding_y: 8.0,
                gap: 8.0,
                font_size: 12.0,
                label_font_size: 11.0,
                icon_size: 16.0,
                radius: 4.0,
                border_width: 1.0,
            },
            large_compact: ControlMetrics {
                height: 40.0,
                padding_x: 14.0,
                padding_y: 9.0,
                gap: 8.0,
                font_size: 13.0,
                label_font_size: 12.0,
                icon_size: 18.0,
                radius: 6.0,
                border_width: 1.0,
            },
            large_regular: ControlMetrics {
                height: 44.0,
                padding_x: 16.0,
                padding_y: 10.0,
                gap: 10.0,
                font_size: 13.0,
                label_font_size: 12.0,
                icon_size: 20.0,
                radius: 6.0,
                border_width: 1.0,
            },
        }
    }
}

impl ControlMetrics {
    fn scale(&mut self, factor: f32) {
        self.height *= factor;
        self.padding_x *= factor;
        self.padding_y *= factor;
        self.gap *= factor;
        self.font_size *= factor;
        self.label_font_size *= factor;
        self.icon_size *= factor;
        self.radius *= factor;
        self.border_width *= factor;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonTheme {
    pub padding_x: f32,
    pub padding_y: f32,
    pub border_width: f32,
    pub radius: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckboxTheme {
    pub gap: f32,
    pub box_size: f32,
    pub border_width: f32,
    pub radius: f32,
    pub font_size: f32,
    pub check_font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CollapsibleTheme {
    pub gap: f32,
    pub header_padding_x: f32,
    pub header_padding_y: f32,
    pub content_padding: f32,
    pub border_width: f32,
    pub radius: f32,
    pub title_font_size: f32,
    pub background: Color,
    pub border: Color,
    pub header_background: Color,
    pub title_text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct DropdownTheme {
    pub gap: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub border_width: f32,
    pub radius: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct GroupTheme {
    pub gap: f32,
    pub padding: f32,
    pub border_width: f32,
    pub radius: f32,
    pub title_font_size: f32,
    pub title_padding_x: f32,
    pub title_padding_y: f32,
    pub title_gap: f32,
    pub background: Color,
    pub border: Color,
    pub title_text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct SeparatorTheme {
    pub thickness: f32,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollAreaTheme {
    pub padding: f32,
    pub border_width: f32,
    pub radius: f32,
    pub background: Color,
    pub border: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ListViewTheme {
    pub item_gap: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ImageViewerTheme {
    pub border_width: f32,
    pub radius: f32,
    pub background: Color,
    pub border: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct RadioTheme {
    pub gap: f32,
    pub ring_size: f32,
    pub border_width: f32,
    pub dot_size: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderTheme {
    pub gap: f32,
    pub track_height: f32,
    pub track_padding: f32,
    pub track_radius: f32,
    pub thumb_size: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct ToggleTheme {
    pub gap: f32,
    pub track_width: f32,
    pub track_height: f32,
    pub track_padding: f32,
    pub track_radius: f32,
    pub thumb_size: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TextInputTheme {
    pub gap: f32,
    pub label_size: f32,
    pub value_size: f32,
    pub field_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub border_width: f32,
    pub radius: f32,
    pub selection_radius: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PanelTheme {
    pub title_bar_height: f32,
    pub title_padding_x: f32,
    pub title_padding_y: f32,
    pub content_padding: f32,
    pub border_width: f32,
    pub radius: f32,
    pub title_font_size: f32,
    pub frame_background: Color,
    pub frame_border: Color,
    pub titlebar_background: Color,
    pub title_text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceVisual {
    pub background: Color,
    pub border: Option<Color>,
    pub text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ToggleVisual {
    pub track_background: Color,
    pub track_border: Option<Color>,
    pub thumb: Color,
    pub text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CheckboxVisual {
    pub box_background: Color,
    pub box_border: Option<Color>,
    pub check: Color,
    pub text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct RadioVisual {
    pub ring_background: Color,
    pub ring_border: Option<Color>,
    pub dot: Color,
    pub text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderVisual {
    pub track_background: Color,
    pub track_border: Option<Color>,
    pub fill: Color,
    pub thumb: Color,
    pub text: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct PanelVisual {
    pub frame_background: Color,
    pub frame_border: Color,
    pub titlebar_background: Color,
    pub title_text: Color,
}

impl Theme {
    pub fn control_metrics(&self, size: ControlSize, density: Density) -> ControlMetrics {
        match (size, density) {
            (ControlSize::Small, Density::Compact) => self.controls.small_compact,
            (ControlSize::Small, Density::Regular) => self.controls.small_regular,
            (ControlSize::Medium, Density::Compact) => self.controls.medium_compact,
            (ControlSize::Medium, Density::Regular) => self.controls.medium_regular,
            (ControlSize::Large, Density::Compact) => self.controls.large_compact,
            (ControlSize::Large, Density::Regular) => self.controls.large_regular,
        }
    }

    pub fn text_field_metrics(&self, size: ControlSize, density: Density) -> TextInputTheme {
        let metrics = self.control_metrics(size, density);
        TextInputTheme {
            gap: (metrics.gap / 2.0).max(2.0),
            label_size: metrics.label_font_size,
            value_size: metrics.font_size,
            field_height: metrics.height,
            padding_x: metrics.padding_x,
            padding_y: metrics.padding_y,
            border_width: metrics.border_width,
            radius: metrics.radius,
            selection_radius: 2.0,
        }
    }

    pub fn text_style_body_sm(&self) -> TextStyle {
        TextStyle::new(self.colors.text, self.text.body_sm)
            .with_family(self.text.body_family)
            .with_line_height(self.text.default_line_height)
            .with_weight(TextWeight::Normal)
    }

    pub fn text_style_body_md(&self) -> TextStyle {
        TextStyle::new(self.colors.text, self.text.body_md)
            .with_family(self.text.body_family)
            .with_line_height(self.text.default_line_height)
            .with_weight(TextWeight::Normal)
    }

    pub fn text_style_label_sm(&self) -> TextStyle {
        TextStyle::new(self.colors.text_muted, self.text.label_sm)
            .with_family(self.text.body_family)
            .with_line_height(self.text.default_line_height)
            .with_weight(TextWeight::Medium)
    }

    pub fn text_style_title_sm(&self) -> TextStyle {
        TextStyle::new(self.colors.text, self.text.title_sm)
            .with_family(self.text.body_family)
            .with_line_height(self.text.default_line_height)
            .with_weight(TextWeight::Semibold)
    }

    pub fn text_style_mono_md(&self) -> TextStyle {
        TextStyle::new(self.colors.text, self.text.body_md)
            .with_family(self.text.mono_family)
            .with_line_height(self.text.default_line_height)
            .with_weight(TextWeight::Medium)
    }

    pub fn text_color_for_visual(&self, visual: ControlVisualState) -> Color {
        match visual {
            ControlVisualState::Disabled => self.colors.text_disabled,
            _ => self.colors.text,
        }
    }

    pub fn button_visual(&self, visual: ControlVisualState) -> SurfaceVisual {
        match visual {
            ControlVisualState::Normal => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border),
                text: self.colors.text,
            },
            ControlVisualState::Hovered => SurfaceVisual {
                background: self.colors.surface_hover,
                border: Some(self.colors.border_hover),
                text: self.colors.text,
            },
            ControlVisualState::Pressed => SurfaceVisual {
                background: self.colors.surface_pressed,
                border: Some(self.colors.border_pressed),
                text: self.colors.text,
            },
            ControlVisualState::Focused => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_focus),
                text: self.colors.text,
            },
            ControlVisualState::Disabled => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_disabled),
                text: self.colors.text_disabled,
            },
        }
    }

    pub fn dropdown_visual(&self, visual: ControlVisualState) -> SurfaceVisual {
        self.button_visual(visual)
    }

    pub fn checkbox_visual(&self, checked: bool, visual: ControlVisualState) -> CheckboxVisual {
        match visual {
            ControlVisualState::Disabled => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent_disabled
                } else {
                    self.colors.surface_disabled
                },
                box_border: Some(self.colors.border_disabled),
                check: self.colors.surface,
                text: self.colors.text_disabled,
            },
            ControlVisualState::Focused => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent
                } else {
                    self.colors.surface
                },
                box_border: Some(self.colors.border_focus),
                check: self.colors.surface,
                text: self.colors.text,
            },
            ControlVisualState::Hovered => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent_hover
                } else {
                    self.colors.surface_hover
                },
                box_border: Some(if checked {
                    self.colors.accent_hover
                } else {
                    self.colors.border_hover
                }),
                check: self.colors.surface,
                text: self.colors.text,
            },
            ControlVisualState::Pressed => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent_pressed
                } else {
                    self.colors.surface_pressed
                },
                box_border: Some(if checked {
                    self.colors.accent_pressed
                } else {
                    self.colors.border_pressed
                }),
                check: self.colors.surface,
                text: self.colors.text,
            },
            ControlVisualState::Normal => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent
                } else {
                    self.colors.surface
                },
                box_border: Some(if checked {
                    self.colors.accent
                } else {
                    self.colors.border
                }),
                check: self.colors.surface,
                text: self.colors.text,
            },
        }
    }

    pub fn toggle_visual(&self, on: bool, visual: ControlVisualState) -> ToggleVisual {
        let track_background = match (on, visual) {
            (_, ControlVisualState::Disabled) => self.colors.accent_disabled,
            (true, ControlVisualState::Pressed) => self.colors.accent_pressed,
            (true, ControlVisualState::Hovered) => self.colors.accent_hover,
            (true, _) => self.colors.accent,
            (false, ControlVisualState::Pressed) => self.colors.border_pressed,
            (false, ControlVisualState::Hovered) => self.colors.border,
            (false, _) => self.colors.border_hover,
        };
        let track_border =
            matches!(visual, ControlVisualState::Focused).then_some(self.colors.border_focus);
        ToggleVisual {
            track_background,
            track_border,
            thumb: self.colors.surface,
            text: self.text_color_for_visual(visual),
        }
    }

    pub fn slider_visual(&self, visual: ControlVisualState) -> SliderVisual {
        let track_background = match visual {
            ControlVisualState::Disabled => self.colors.border_disabled,
            ControlVisualState::Pressed => self.colors.border_hover,
            ControlVisualState::Hovered => self.colors.surface_hover,
            _ => self.colors.border,
        };
        let fill = match visual {
            ControlVisualState::Disabled => self.colors.text_disabled,
            ControlVisualState::Pressed => self.colors.surface_pressed,
            ControlVisualState::Hovered => self.colors.surface_disabled,
            _ => self.colors.text,
        };
        let track_border =
            matches!(visual, ControlVisualState::Focused).then_some(self.colors.border_focus);
        SliderVisual {
            track_background,
            track_border,
            fill,
            thumb: self.colors.surface,
            text: self.text_color_for_visual(visual),
        }
    }

    pub fn radio_visual(&self, selected: bool, visual: ControlVisualState) -> RadioVisual {
        match visual {
            ControlVisualState::Disabled => RadioVisual {
                ring_background: self.colors.surface_disabled,
                ring_border: Some(self.colors.border_disabled),
                dot: if selected {
                    self.colors.accent_disabled
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text_disabled,
            },
            ControlVisualState::Focused => RadioVisual {
                ring_background: self.colors.surface,
                ring_border: Some(self.colors.border_focus),
                dot: if selected {
                    self.colors.accent
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            ControlVisualState::Hovered => RadioVisual {
                ring_background: self.colors.surface_hover,
                ring_border: Some(self.colors.border_hover),
                dot: if selected {
                    self.colors.accent_hover
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            ControlVisualState::Pressed => RadioVisual {
                ring_background: self.colors.surface_pressed,
                ring_border: Some(self.colors.border_pressed),
                dot: if selected {
                    self.colors.accent_pressed
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            ControlVisualState::Normal => RadioVisual {
                ring_background: self.colors.surface,
                ring_border: Some(self.colors.border),
                dot: if selected {
                    self.colors.accent
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
        }
    }

    pub fn text_input_visual(&self, visual: ControlVisualState) -> SurfaceVisual {
        match visual {
            ControlVisualState::Disabled => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_disabled),
                text: self.colors.text_disabled,
            },
            ControlVisualState::Pressed => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_pressed),
                text: self.colors.text,
            },
            ControlVisualState::Hovered => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_hover),
                text: self.colors.text,
            },
            ControlVisualState::Focused => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_focus),
                text: self.colors.text,
            },
            ControlVisualState::Normal => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border),
                text: self.colors.text,
            },
        }
    }

    pub fn selection_color(&self) -> Color {
        self.colors.accent_soft
    }

    pub fn caret_color(&self) -> Color {
        self.colors.caret
    }

    pub fn preedit_underline_color(&self) -> Color {
        self.colors.caret
    }

    pub fn panel_visual(&self) -> PanelVisual {
        PanelVisual {
            frame_background: self.components.panel.frame_background,
            frame_border: self.components.panel.frame_border,
            titlebar_background: self.components.panel.titlebar_background,
            title_text: self.components.panel.title_text,
        }
    }
}
