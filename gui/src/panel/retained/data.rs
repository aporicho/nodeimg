use super::super::{PanelConfig, PanelRuntime};
use crate::control::{control_list_min_height, ControlMetrics, ControlNode};
use crate::theme::Theme;

#[derive(Clone, Debug)]
pub struct PanelFrameTemplateData {
    pub(in crate::panel::retained) config: PanelConfig,
    pub(in crate::panel::retained) runtime: PanelRuntime,
    pub(in crate::panel::retained) content: PanelContentTemplate,
    pub(in crate::panel::retained) theme: Theme,
}

#[derive(Clone, Debug)]
pub struct PanelContentTemplate {
    pub(in crate::panel::retained) nodes: Vec<ControlNode>,
}

impl PanelContentTemplate {
    pub fn new(nodes: Vec<ControlNode>) -> Self {
        Self { nodes }
    }

    pub fn body_min_height(&self, theme: &Theme) -> f32 {
        control_list_min_height(
            self.nodes.iter(),
            theme,
            ControlMetrics::from_theme(theme),
            theme.spacing.sm,
        )
    }

    pub fn panel_min_height(&self, config: &PanelConfig, theme: &Theme) -> f32 {
        let titlebar_height = if config.titlebar_visible {
            theme.components.panel.title_bar_height
        } else {
            0.0
        };
        titlebar_height + theme.components.panel.content_padding * 2.0 + self.body_min_height(theme)
    }
}

impl PanelFrameTemplateData {
    pub fn new(
        config: PanelConfig,
        runtime: PanelRuntime,
        content: PanelContentTemplate,
        theme: &Theme,
    ) -> Self {
        Self {
            config,
            runtime,
            content,
            theme: theme.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::panel::PanelId;
    use crate::renderer::{ImageStyle, Rect};
    use crate::theme::light_theme;
    use crate::tree::layout::TextureHandle;

    fn panel_config(titlebar_visible: bool) -> PanelConfig {
        PanelConfig {
            id: PanelId::new("test_panel"),
            title: Cow::Borrowed("Test"),
            default_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 120.0,
                h: 24.0,
            },
            min_size: [120.0, 24.0],
            titlebar_visible,
            draggable: true,
            resizable: true,
            closable: false,
            initially_visible: true,
        }
    }

    #[test]
    fn body_min_height_counts_buttons_and_gaps() {
        let theme = light_theme();
        let content = PanelContentTemplate::new(vec![
            ControlNode::button("add", "Add"),
            ControlNode::button("run", "Run"),
        ]);
        let button_height =
            theme.components.button.font_size + theme.components.button.padding_y * 2.0;

        assert_eq!(
            content.body_min_height(&theme),
            button_height * 2.0 + theme.spacing.sm
        );
    }

    #[test]
    fn group_min_height_counts_title_children_padding_and_gaps() {
        let theme = light_theme();
        let group = ControlNode::group(
            "runtime",
            "Runtime",
            vec![
                ControlNode::label("status", "Status"),
                ControlNode::muted_label("last", "Last"),
            ],
        );
        let title_height = {
            let style = theme.text_style_title_sm();
            style.size * style.line_height
        };
        let label_height = {
            let style = theme.text_style_label_sm();
            style.size * style.line_height
        };
        let expected = theme.components.group.padding * 2.0
            + title_height
            + label_height * 2.0
            + theme.components.group.gap * 2.0;

        assert_eq!(
            crate::control::control_node_min_height(
                &group,
                &theme,
                ControlMetrics::from_theme(&theme)
            ),
            expected
        );
    }

    #[test]
    fn panel_min_height_includes_titlebar_and_content_padding() {
        let theme = light_theme();
        let content = PanelContentTemplate::new(vec![ControlNode::image(
            "preview",
            TextureHandle(1),
            ImageStyle::default(),
        )]);
        let config = panel_config(true);

        assert_eq!(
            content.panel_min_height(&config, &theme),
            theme.components.panel.title_bar_height
                + theme.components.panel.content_padding * 2.0
                + content.body_min_height(&theme)
        );
    }
}
