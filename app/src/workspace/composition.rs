use crate::app_shell::AppMode;
use crate::panels::PanelWorkspaceMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceUiCompositionKind {
    CleanRoom,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceUiComposition {
    kind: WorkspaceUiCompositionKind,
    canvas_nodes: CanvasNodeComposition,
    connections: bool,
    panel_mode: PanelWorkspaceMode,
    node_palette: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CanvasNodeComposition {
    EngineCleanRoom,
    EngineAndShowcase,
}

impl WorkspaceUiComposition {
    pub(crate) fn clean_room() -> Self {
        Self {
            kind: WorkspaceUiCompositionKind::CleanRoom,
            canvas_nodes: CanvasNodeComposition::EngineCleanRoom,
            connections: false,
            panel_mode: PanelWorkspaceMode::CleanRoom,
            node_palette: false,
        }
    }

    pub(crate) fn full() -> Self {
        Self {
            kind: WorkspaceUiCompositionKind::Full,
            canvas_nodes: CanvasNodeComposition::EngineAndShowcase,
            connections: true,
            panel_mode: PanelWorkspaceMode::Full,
            node_palette: true,
        }
    }

    pub(crate) fn for_app_mode(mode: AppMode) -> Self {
        match mode {
            AppMode::Developer => Self::clean_room(),
            AppMode::User => Self::full(),
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self.kind {
            WorkspaceUiCompositionKind::CleanRoom => "clean_room",
            WorkspaceUiCompositionKind::Full => "full",
        }
    }

    pub(crate) fn canvas_nodes(self) -> CanvasNodeComposition {
        self.canvas_nodes
    }

    pub(crate) fn connections_enabled(self) -> bool {
        self.connections
    }

    pub(crate) fn panel_mode(self) -> PanelWorkspaceMode {
        self.panel_mode
    }

    pub(crate) fn node_palette_enabled(self) -> bool {
        self.node_palette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ui_composition_maps_app_mode() {
        assert_eq!(
            WorkspaceUiComposition::for_app_mode(AppMode::Developer),
            WorkspaceUiComposition::clean_room()
        );
        assert_eq!(
            WorkspaceUiComposition::for_app_mode(AppMode::User),
            WorkspaceUiComposition::full()
        );
        assert_eq!(
            WorkspaceUiComposition::clean_room().panel_mode(),
            PanelWorkspaceMode::CleanRoom
        );
        assert_eq!(
            WorkspaceUiComposition::full().panel_mode(),
            PanelWorkspaceMode::Full
        );
    }
}
