use super::super::{PanelConfig, PanelRuntime};
use crate::renderer::ImageStyle;
use crate::theme::Theme;
use crate::tree::layout::TextureHandle;

#[derive(Clone, Debug)]
pub struct PanelFrameTemplateData {
    pub(in crate::panel::retained) config: PanelConfig,
    pub(in crate::panel::retained) runtime: PanelRuntime,
    pub(in crate::panel::retained) content: PanelContentTemplate,
    pub(in crate::panel::retained) theme: Theme,
}

#[derive(Clone, Debug)]
pub enum PanelContentTemplate {
    Toolbar {
        add_graph_id: String,
        run_graph_id: String,
    },
    Preview {
        image_id: String,
        texture: TextureHandle,
        image_style: ImageStyle,
    },
    Engine {
        group_id: String,
        status: String,
        catalog: String,
        last_action: String,
    },
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
