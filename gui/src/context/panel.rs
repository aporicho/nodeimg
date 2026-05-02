use super::Context;
use crate::output::ControlEvent;

impl Context {
    pub(crate) fn handle_panel_control_event(&mut self, event: &ControlEvent) -> bool {
        crate::panel::event::apply_panel_control_event(&mut self.tree, event)
    }

    pub(crate) fn export_panel_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.tree.export_panel_layouts()
    }

    pub(crate) fn ensure_panel_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        self.tree.ensure_panel(config);
        self.tree.panel_state(config.id.as_str()).cloned()
    }

    pub(crate) fn panel_runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        self.tree.panel_state(id).cloned()
    }

    pub(crate) fn import_panel_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.tree.import_panel_layouts(layouts);
    }
}
