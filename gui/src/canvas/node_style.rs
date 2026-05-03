use crate::control::ControlMetrics;
use crate::renderer::Rect;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NodeCardMetrics {
    pub card_width: f32,
    pub card_height: f32,
    pub pin_dot_diameter: f32,
    pub title_dot_diameter: f32,
    pub port_group_trigger_diameter: f32,
    pub port_group_trigger_icon_size: f32,
    pub pin_label_width: f32,
    pub title_label_width: f32,
    pub param_row_height: f32,
    pub card_padding: f32,
    pub column_gap: f32,
    pub row_gap: f32,
    pub pin_row_gap: f32,
    pub pin_label_gap: f32,
    pub title_label_gap: f32,
    pub param_label_gap: f32,
    pub title_lift: f32,
    pub card_radius: f32,
    pub row_radius: f32,
    pub control: ControlMetrics,
}

impl NodeCardMetrics {
    pub(crate) fn from_theme(theme: &Theme) -> Self {
        let mut control = ControlMetrics::from_theme(theme);
        control.control_width = 256.0;
        Self {
            card_width: 304.0,
            card_height: 132.0,
            pin_dot_diameter: 10.0,
            title_dot_diameter: 6.0,
            port_group_trigger_diameter: 20.0,
            port_group_trigger_icon_size: 10.0,
            pin_label_width: 78.0,
            title_label_width: 180.0,
            param_row_height: 36.0,
            card_padding: 24.0,
            column_gap: 24.0,
            row_gap: 12.0,
            pin_row_gap: 4.0,
            pin_label_gap: 6.0,
            title_label_gap: 5.0,
            param_label_gap: 10.0,
            title_lift: 20.0,
            card_radius: 8.0,
            row_radius: 0.0,
            control,
        }
    }

    pub(crate) fn for_layout(theme: &Theme, rect: Rect) -> Self {
        let mut metrics = Self::from_theme(theme);
        metrics.card_width = rect.w.max(metrics.card_width);
        metrics.card_height = rect.h.max(metrics.card_height);
        metrics.title_label_width = (metrics.card_width
            - metrics.card_padding * 2.0
            - metrics.title_dot_diameter
            - metrics.title_label_gap)
            .max(48.0);
        metrics.control.control_width = (metrics.card_width - metrics.card_padding * 2.0).max(64.0);
        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::light_theme;

    #[test]
    fn node_card_metrics_preserve_figma_baseline_values() {
        let metrics = NodeCardMetrics::from_theme(&light_theme());

        assert_eq!(metrics.card_width, 304.0);
        assert_eq!(metrics.card_height, 132.0);
        assert_eq!(metrics.card_padding, 24.0);
        assert_eq!(metrics.column_gap, 24.0);
        assert_eq!(metrics.row_gap, 12.0);
        assert_eq!(metrics.param_row_height, 36.0);
        assert_eq!(metrics.card_radius, 8.0);
        assert_eq!(metrics.port_group_trigger_diameter, 20.0);
        assert_eq!(metrics.port_group_trigger_icon_size, 10.0);
        assert_eq!(metrics.control.control_width, 256.0);
        assert_eq!(metrics.control.control_height, 24.0);
    }

    #[test]
    fn node_card_metrics_expand_control_width_from_layout() {
        let metrics = NodeCardMetrics::for_layout(
            &light_theme(),
            Rect {
                x: 0.0,
                y: 0.0,
                w: 420.0,
                h: 240.0,
            },
        );

        assert_eq!(metrics.card_width, 420.0);
        assert_eq!(metrics.card_height, 240.0);
        assert_eq!(metrics.control.control_width, 372.0);
    }
}
