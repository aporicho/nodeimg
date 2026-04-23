use crate::theme::Theme;
use crate::widget::param_control::ParamControlMetrics;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NodeCardMetrics {
    pub card_width: f32,
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
    pub control: ParamControlMetrics,
}

impl NodeCardMetrics {
    pub(crate) fn from_theme(theme: &Theme) -> Self {
        Self {
            card_width: 304.0,
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
            control: ParamControlMetrics::from_theme(theme),
        }
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
        assert_eq!(metrics.card_padding, 24.0);
        assert_eq!(metrics.column_gap, 24.0);
        assert_eq!(metrics.row_gap, 12.0);
        assert_eq!(metrics.param_row_height, 36.0);
        assert_eq!(metrics.card_radius, 8.0);
        assert_eq!(metrics.port_group_trigger_diameter, 20.0);
        assert_eq!(metrics.port_group_trigger_icon_size, 10.0);
        assert_eq!(metrics.control.control_width, 128.0);
        assert_eq!(metrics.control.control_height, 24.0);
    }
}
