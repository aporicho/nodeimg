use super::engine_adapter;
use super::node_palette::NodePaletteState;
use crate::image_demo::ImageDemoController;
use crate::panels::EnginePanelState;
use engine::facade::EngineFacade;
use engine::Engine;
use gui::action::GuiAction;

pub(crate) struct WorkspaceController {
    engine: Engine,
    image_demo: ImageDemoController,
    last_engine_action: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceActionResult {
    pub(crate) handled_node_add: Option<String>,
    pub(crate) close_overlay: bool,
}

impl WorkspaceController {
    pub(crate) fn new() -> Self {
        Self {
            engine: Engine::new(None),
            image_demo: ImageDemoController::default(),
            last_engine_action: "Ready".to_string(),
        }
    }

    pub(crate) fn handle_gui_action(&mut self, action: GuiAction) -> WorkspaceActionResult {
        match action {
            GuiAction::AddNode { type_id } => self.add_node_from_library(&type_id),
            GuiAction::OpenOverlay { .. }
            | GuiAction::CloseOverlay { .. }
            | GuiAction::WidgetClicked { .. } => WorkspaceActionResult::default(),
        }
    }

    pub(crate) fn add_node_from_library(&mut self, type_id: &str) -> WorkspaceActionResult {
        match self.engine.add_node(type_id) {
            Ok(node_id) => {
                self.last_engine_action = format!("Added node {type_id} as {:?}", node_id);
                WorkspaceActionResult {
                    handled_node_add: Some(type_id.to_string()),
                    close_overlay: true,
                }
            }
            Err(error) => {
                self.last_engine_action = format!("Add node failed: {error}");
                WorkspaceActionResult {
                    handled_node_add: Some(type_id.to_string()),
                    close_overlay: false,
                }
            }
        }
    }

    pub(crate) fn handle_image_demo_button(&mut self, id: &str) -> bool {
        let Some(outcome) = self.image_demo.handle_button(id, &mut self.engine) else {
            return false;
        };
        self.last_engine_action = outcome.to_string();
        true
    }

    pub(crate) fn engine_panel_state(&self) -> EnginePanelState {
        engine_adapter::engine_panel_state(&self.engine, &self.last_engine_action)
    }

    pub(crate) fn node_palette_state(&self) -> NodePaletteState {
        engine_adapter::node_palette_state(&self.engine)
    }

    pub(crate) fn note_node_library_opened(&mut self) {
        self.last_engine_action = "Node library opened".to_string();
    }
}

impl Default for WorkspaceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_action_updates_engine_state_and_requests_overlay_close() {
        let mut controller = WorkspaceController::new();

        let result = controller.handle_gui_action(GuiAction::AddNode {
            type_id: "image_gen".to_string(),
        });
        let panel = controller.engine_panel_state();

        assert_eq!(result.handled_node_add, Some("image_gen".to_string()));
        assert!(result.close_overlay);
        assert_eq!(panel.node_count, 1);
        assert!(panel.last_action.contains("Added node image_gen"));
    }
}
