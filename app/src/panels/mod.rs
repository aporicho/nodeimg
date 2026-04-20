use crate::demo_gallery::GalleryState;
use gui::theme::Theme;
use gui::tree::layout::TextureHandle;

#[allow(dead_code)]
pub(crate) struct PanelBuildContext<'a> {
    pub(crate) theme: &'a Theme,
    pub(crate) gallery: &'a GalleryState,
    pub(crate) image: TextureHandle,
    pub(crate) engine: &'a EnginePanelState,
    pub(crate) node_library: &'a NodeLibraryPanelState,
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

pub(crate) const NODE_LIBRARY_ADD_PREFIX: &str = "node_library::add::";

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodeLibraryPanelState {
    pub(crate) open: bool,
    pub(crate) panel_id: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) items: Vec<NodeLibraryItem>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NodeLibraryItem {
    pub(crate) type_id: String,
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) source: String,
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
            node_library: &NodeLibraryPanelState {
                open: false,
                panel_id: "node_library_closed".to_string(),
                x: 0.0,
                y: 0.0,
                items: Vec::new(),
            },
        });
        let ids = panels
            .iter()
            .map(|panel| panel.config.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"preview"));
        assert!(ids.contains(&"toolbar"));
        assert!(ids.contains(&"engine"));
        assert!(ids.contains(&"node_library_closed"));
    }
}
