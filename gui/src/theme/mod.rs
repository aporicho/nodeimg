mod dark;
mod light;
mod tokens;

pub use tokens::{
    ButtonTheme, CheckboxTheme, CollapsibleTheme, DropdownTheme, GroupTheme, ImageViewerTheme,
    ListViewTheme, PanelTheme, PanelVisual, RadioTheme, ScrollAreaTheme, SeparatorTheme,
    SliderTheme, SliderVisual, SurfaceVisual, TextInputTheme, Theme, ThemeColors, ThemeComponents,
    ThemeMode, ThemeRadii, ThemeSpacing, ThemeText, ToggleTheme, ToggleVisual,
};

impl Theme {
    pub fn dark() -> Self {
        dark_theme()
    }

    pub fn light() -> Self {
        light_theme()
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => dark_theme(),
            ThemeMode::Light => light_theme(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        dark_theme()
    }
}

pub fn dark_theme() -> Theme {
    dark::build_theme()
}

pub fn light_theme() -> Theme {
    light::build_theme()
}
