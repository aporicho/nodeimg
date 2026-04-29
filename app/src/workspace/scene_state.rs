use gui::renderer::Rect;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub(crate) struct WorkspaceSceneState {
    panels: BTreeMap<String, PanelAppliedState>,
    node_palette: NodePaletteAppliedState,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PanelAppliedState {
    pub(crate) visible: bool,
    pub(crate) rect: Rect,
    pub(crate) z_index: i32,
    pub(crate) texts: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default)]
struct NodePaletteAppliedState {
    root_rect: Option<Rect>,
    child_labels: BTreeMap<String, String>,
}

impl WorkspaceSceneState {
    pub(crate) fn panel(&self, id: &str) -> Option<&PanelAppliedState> {
        self.panels.get(id)
    }

    pub(crate) fn set_panel(&mut self, id: impl Into<String>, state: PanelAppliedState) {
        self.panels.insert(id.into(), state);
    }

    pub(crate) fn remove_panel(&mut self, id: &str) {
        self.panels.remove(id);
    }

    pub(crate) fn palette_root_rect(&self) -> Option<Rect> {
        self.node_palette.root_rect
    }

    pub(crate) fn set_palette_root_rect(&mut self, rect: Option<Rect>) {
        self.node_palette.root_rect = rect;
    }

    pub(crate) fn palette_child_label(&self, id: &str) -> Option<&str> {
        self.node_palette.child_labels.get(id).map(String::as_str)
    }

    pub(crate) fn palette_child_ids(&self) -> impl Iterator<Item = &str> {
        self.node_palette.child_labels.keys().map(String::as_str)
    }

    pub(crate) fn set_palette_child_label(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
    ) {
        self.node_palette
            .child_labels
            .insert(id.into(), label.into());
    }

    pub(crate) fn remove_palette_child(&mut self, id: &str) {
        self.node_palette.child_labels.remove(id);
    }

    pub(crate) fn clear_palette(&mut self) {
        self.node_palette = NodePaletteAppliedState::default();
    }
}
