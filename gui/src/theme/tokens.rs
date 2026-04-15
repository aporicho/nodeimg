use crate::renderer::Color;
use crate::widget::state::WidgetVisualState;

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
    pub fn text_color_for_visual(&self, visual: WidgetVisualState) -> Color {
        match visual {
            WidgetVisualState::Disabled => self.colors.text_disabled,
            _ => self.colors.text,
        }
    }

    pub fn button_visual(&self, visual: WidgetVisualState) -> SurfaceVisual {
        match visual {
            WidgetVisualState::Normal => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border),
                text: self.colors.text,
            },
            WidgetVisualState::Hovered => SurfaceVisual {
                background: self.colors.surface_hover,
                border: Some(self.colors.border_hover),
                text: self.colors.text,
            },
            WidgetVisualState::Pressed => SurfaceVisual {
                background: self.colors.surface_pressed,
                border: Some(self.colors.border_pressed),
                text: self.colors.text,
            },
            WidgetVisualState::Focused => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_focus),
                text: self.colors.text,
            },
            WidgetVisualState::Disabled => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_disabled),
                text: self.colors.text_disabled,
            },
        }
    }

    pub fn dropdown_visual(&self, visual: WidgetVisualState) -> SurfaceVisual {
        self.button_visual(visual)
    }

    pub fn checkbox_visual(&self, checked: bool, visual: WidgetVisualState) -> CheckboxVisual {
        match visual {
            WidgetVisualState::Disabled => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent_disabled
                } else {
                    self.colors.surface_disabled
                },
                box_border: Some(self.colors.border_disabled),
                check: self.colors.surface,
                text: self.colors.text_disabled,
            },
            WidgetVisualState::Focused => CheckboxVisual {
                box_background: if checked {
                    self.colors.accent
                } else {
                    self.colors.surface
                },
                box_border: Some(self.colors.border_focus),
                check: self.colors.surface,
                text: self.colors.text,
            },
            WidgetVisualState::Hovered => CheckboxVisual {
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
            WidgetVisualState::Pressed => CheckboxVisual {
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
            WidgetVisualState::Normal => CheckboxVisual {
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

    pub fn toggle_visual(&self, on: bool, visual: WidgetVisualState) -> ToggleVisual {
        let track_background = match (on, visual) {
            (_, WidgetVisualState::Disabled) => self.colors.accent_disabled,
            (true, WidgetVisualState::Pressed) => self.colors.accent_pressed,
            (true, WidgetVisualState::Hovered) => self.colors.accent_hover,
            (true, _) => self.colors.accent,
            (false, WidgetVisualState::Pressed) => self.colors.border_pressed,
            (false, WidgetVisualState::Hovered) => self.colors.border,
            (false, _) => self.colors.border_hover,
        };
        let track_border =
            matches!(visual, WidgetVisualState::Focused).then_some(self.colors.border_focus);
        ToggleVisual {
            track_background,
            track_border,
            thumb: self.colors.surface,
            text: self.text_color_for_visual(visual),
        }
    }

    pub fn slider_visual(&self, visual: WidgetVisualState) -> SliderVisual {
        let track_background = match visual {
            WidgetVisualState::Disabled => self.colors.border_disabled,
            WidgetVisualState::Pressed => self.colors.border_hover,
            WidgetVisualState::Hovered => self.colors.surface_hover,
            _ => self.colors.border,
        };
        let fill = match visual {
            WidgetVisualState::Disabled => self.colors.text_disabled,
            WidgetVisualState::Pressed => self.colors.surface_pressed,
            WidgetVisualState::Hovered => self.colors.surface_disabled,
            _ => self.colors.text,
        };
        let track_border =
            matches!(visual, WidgetVisualState::Focused).then_some(self.colors.border_focus);
        SliderVisual {
            track_background,
            track_border,
            fill,
            thumb: self.colors.surface,
            text: self.text_color_for_visual(visual),
        }
    }

    pub fn radio_visual(&self, selected: bool, visual: WidgetVisualState) -> RadioVisual {
        match visual {
            WidgetVisualState::Disabled => RadioVisual {
                ring_background: self.colors.surface_disabled,
                ring_border: Some(self.colors.border_disabled),
                dot: if selected {
                    self.colors.accent_disabled
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text_disabled,
            },
            WidgetVisualState::Focused => RadioVisual {
                ring_background: self.colors.surface,
                ring_border: Some(self.colors.border_focus),
                dot: if selected {
                    self.colors.accent
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            WidgetVisualState::Hovered => RadioVisual {
                ring_background: self.colors.surface_hover,
                ring_border: Some(self.colors.border_hover),
                dot: if selected {
                    self.colors.accent_hover
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            WidgetVisualState::Pressed => RadioVisual {
                ring_background: self.colors.surface_pressed,
                ring_border: Some(self.colors.border_pressed),
                dot: if selected {
                    self.colors.accent_pressed
                } else {
                    Color::TRANSPARENT
                },
                text: self.colors.text,
            },
            WidgetVisualState::Normal => RadioVisual {
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

    pub fn text_input_visual(&self, visual: WidgetVisualState) -> SurfaceVisual {
        match visual {
            WidgetVisualState::Disabled => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_disabled),
                text: self.colors.text_disabled,
            },
            WidgetVisualState::Pressed => SurfaceVisual {
                background: self.colors.surface_disabled,
                border: Some(self.colors.border_pressed),
                text: self.colors.text,
            },
            WidgetVisualState::Hovered => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_hover),
                text: self.colors.text,
            },
            WidgetVisualState::Focused => SurfaceVisual {
                background: self.colors.surface,
                border: Some(self.colors.border_focus),
                text: self.colors.text,
            },
            WidgetVisualState::Normal => SurfaceVisual {
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
