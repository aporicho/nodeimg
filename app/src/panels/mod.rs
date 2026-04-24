use crate::demo_gallery::GalleryState;
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;

#[allow(dead_code)]
pub(crate) struct PanelBuildContext<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) gallery: &'a GalleryState,
    pub(crate) image: TextureHandle,
    pub(crate) engine: &'a EnginePanelState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EnginePanelState {
    pub(crate) node_count: usize,
    pub(crate) connection_count: usize,
    pub(crate) node_def_count: usize,
    pub(crate) graph_version: u64,
    pub(crate) dirty: bool,
    pub(crate) execution_status: String,
    pub(crate) last_action: String,
}

include!(concat!(env!("OUT_DIR"), "/panels_generated.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_gallery::GalleryState;
    use gui::theme::light_theme;

    #[test]
    fn generated_panels_include_initial_workspace_panels() {
        let theme = light_theme();
        let gallery = GalleryState::default();
        let panels = collect_panels(&PanelBuildContext {
            theme: &theme,
            gallery: &gallery,
            image: TextureHandle(1),
            engine: &EnginePanelState {
                node_count: 0,
                connection_count: 0,
                node_def_count: 0,
                graph_version: 0,
                dirty: false,
                execution_status: "Idle".to_string(),
                last_action: "Ready".to_string(),
            },
        });
        let ids = panels
            .iter()
            .map(|panel| panel.config.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"preview"));
        assert!(ids.contains(&"toolbar"));
        assert!(ids.contains(&"engine"));
        assert!(ids.contains(&"gallery"));
    }
}
