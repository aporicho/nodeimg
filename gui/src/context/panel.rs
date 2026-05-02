use super::Context;
use crate::output::ControlEvent;

impl Context {
    pub(crate) fn handle_panel_control_event(&mut self, event: &ControlEvent) -> bool {
        crate::panel::event::apply_panel_control_event(&mut self.tree, event)
    }

    pub(crate) fn export_panel_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        crate::panel::runtime::export_layouts(&self.tree)
    }

    pub(crate) fn ensure_panel_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        crate::panel::runtime::ensure_panel(&mut self.tree, config);
        crate::panel::runtime::panel_state(&self.tree, config.id.as_str()).cloned()
    }

    pub(crate) fn panel_runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        crate::panel::runtime::panel_state(&self.tree, id).cloned()
    }

    pub(crate) fn import_panel_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        crate::panel::runtime::import_layouts(&mut self.tree, layouts);
    }
}
