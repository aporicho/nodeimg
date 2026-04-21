mod dark;
mod light;
mod tokens;

pub use tokens::{
    ButtonTheme, CheckboxTheme, CollapsibleTheme, ControlMetrics, ControlSize, Density,
    DropdownTheme, GroupTheme, ImageViewerTheme, ListViewTheme, PanelTheme, PanelVisual,
    RadioTheme, ScrollAreaTheme, SeparatorTheme, SliderTheme, SliderVisual, SurfaceVisual,
    TextInputTheme, Theme, ThemeColors, ThemeComponents, ThemeControls, ThemeMode, ThemeRadii,
    ThemeSpacing, ThemeText, ToggleTheme, ToggleVisual,
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

    pub fn scaled(&self, factor: f32) -> Self {
        let mut scaled = self.clone();
        scaled.revision = self.revision.wrapping_add((factor * 1000.0) as u64);

        scaled.text.label_sm *= factor;
        scaled.text.body_sm *= factor;
        scaled.text.body_md *= factor;
        scaled.text.title_sm *= factor;

        scaled.radii.sm *= factor;
        scaled.radii.md *= factor;

        scaled.spacing.xs *= factor;
        scaled.spacing.sm *= factor;
        scaled.spacing.md *= factor;
        scaled.spacing.lg *= factor;

        scaled.controls.scale(factor);

        scaled.components.button.padding_x *= factor;
        scaled.components.button.padding_y *= factor;
        scaled.components.button.border_width *= factor;
        scaled.components.button.radius *= factor;
        scaled.components.button.font_size *= factor;

        scaled.components.checkbox.gap *= factor;
        scaled.components.checkbox.box_size *= factor;
        scaled.components.checkbox.border_width *= factor;
        scaled.components.checkbox.radius *= factor;
        scaled.components.checkbox.font_size *= factor;
        scaled.components.checkbox.check_font_size *= factor;

        scaled.components.collapsible.gap *= factor;
        scaled.components.collapsible.header_padding_x *= factor;
        scaled.components.collapsible.header_padding_y *= factor;
        scaled.components.collapsible.content_padding *= factor;
        scaled.components.collapsible.border_width *= factor;
        scaled.components.collapsible.radius *= factor;
        scaled.components.collapsible.title_font_size *= factor;

        scaled.components.dropdown.gap *= factor;
        scaled.components.dropdown.padding_x *= factor;
        scaled.components.dropdown.padding_y *= factor;
        scaled.components.dropdown.border_width *= factor;
        scaled.components.dropdown.radius *= factor;
        scaled.components.dropdown.font_size *= factor;

        scaled.components.group.gap *= factor;
        scaled.components.group.padding *= factor;
        scaled.components.group.border_width *= factor;
        scaled.components.group.radius *= factor;
        scaled.components.group.title_font_size *= factor;
        scaled.components.group.title_padding_x *= factor;
        scaled.components.group.title_padding_y *= factor;
        scaled.components.group.title_gap *= factor;

        scaled.components.image_viewer.border_width *= factor;
        scaled.components.image_viewer.radius *= factor;

        scaled.components.list_view.item_gap *= factor;

        scaled.components.number_input.gap *= factor;
        scaled.components.number_input.label_size *= factor;
        scaled.components.number_input.value_size *= factor;
        scaled.components.number_input.field_height *= factor;
        scaled.components.number_input.padding_x *= factor;
        scaled.components.number_input.padding_y *= factor;
        scaled.components.number_input.border_width *= factor;
        scaled.components.number_input.radius *= factor;
        scaled.components.number_input.selection_radius *= factor;

        scaled.components.radio.gap *= factor;
        scaled.components.radio.ring_size *= factor;
        scaled.components.radio.border_width *= factor;
        scaled.components.radio.dot_size *= factor;
        scaled.components.radio.font_size *= factor;

        scaled.components.scroll_area.padding *= factor;
        scaled.components.scroll_area.border_width *= factor;
        scaled.components.scroll_area.radius *= factor;

        scaled.components.separator.thickness *= factor;

        scaled.components.slider.gap *= factor;
        scaled.components.slider.track_height *= factor;
        scaled.components.slider.track_padding *= factor;
        scaled.components.slider.track_radius *= factor;
        scaled.components.slider.thumb_size *= factor;
        scaled.components.slider.font_size *= factor;

        scaled.components.toggle.gap *= factor;
        scaled.components.toggle.track_width *= factor;
        scaled.components.toggle.track_height *= factor;
        scaled.components.toggle.track_padding *= factor;
        scaled.components.toggle.track_radius *= factor;
        scaled.components.toggle.thumb_size *= factor;
        scaled.components.toggle.font_size *= factor;

        scaled.components.text_input.gap *= factor;
        scaled.components.text_input.label_size *= factor;
        scaled.components.text_input.value_size *= factor;
        scaled.components.text_input.field_height *= factor;
        scaled.components.text_input.padding_x *= factor;
        scaled.components.text_input.padding_y *= factor;
        scaled.components.text_input.border_width *= factor;
        scaled.components.text_input.radius *= factor;
        scaled.components.text_input.selection_radius *= factor;

        scaled.components.panel.title_bar_height *= factor;
        scaled.components.panel.title_padding_x *= factor;
        scaled.components.panel.title_padding_y *= factor;
        scaled.components.panel.content_padding *= factor;
        scaled.components.panel.border_width *= factor;
        scaled.components.panel.radius *= factor;
        scaled.components.panel.title_font_size *= factor;

        scaled
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_metrics_resolve_all_size_density_pairs() {
        let theme = dark_theme();

        assert_eq!(
            theme
                .control_metrics(ControlSize::Small, Density::Compact)
                .height,
            24.0
        );
        assert_eq!(
            theme
                .control_metrics(ControlSize::Small, Density::Regular)
                .height,
            28.0
        );
        assert_eq!(
            theme
                .control_metrics(ControlSize::Medium, Density::Compact)
                .height,
            32.0
        );
        assert_eq!(
            theme
                .control_metrics(ControlSize::Medium, Density::Regular)
                .height,
            36.0
        );
        assert_eq!(
            theme
                .control_metrics(ControlSize::Large, Density::Compact)
                .height,
            40.0
        );
        assert_eq!(
            theme
                .control_metrics(ControlSize::Large, Density::Regular)
                .height,
            44.0
        );
    }

    #[test]
    fn scaled_theme_scales_control_metrics() {
        let theme = dark_theme();
        let scaled = theme.scaled(2.0);

        assert_eq!(
            scaled
                .control_metrics(ControlSize::Medium, Density::Regular)
                .height,
            72.0
        );
        assert_eq!(
            scaled
                .control_metrics(ControlSize::Small, Density::Compact)
                .padding_x,
            12.0
        );
    }

    #[test]
    fn text_field_metrics_are_derived_from_control_metrics() {
        let theme = dark_theme();
        let metrics = theme.control_metrics(ControlSize::Small, Density::Compact);
        let field = theme.text_field_metrics(ControlSize::Small, Density::Compact);

        assert_eq!(field.field_height, metrics.height);
        assert_eq!(field.padding_x, metrics.padding_x);
        assert_eq!(field.value_size, metrics.font_size);
        assert_eq!(field.label_size, metrics.label_font_size);
    }
}
