use super::super::Context;
use crate::output::ControlEvent;

pub struct PanelApi<'a> {
    pub(in crate::context) ctx: &'a Context,
}

impl PanelApi<'_> {
    pub fn export_layouts(&self) -> Vec<crate::panel::PanelLayout> {
        self.ctx.export_panel_layouts()
    }

    pub fn runtime(&self, id: &str) -> Option<crate::panel::PanelRuntime> {
        self.ctx.panel_runtime(id)
    }
}

pub struct PanelMutApi<'a> {
    pub(in crate::context) ctx: &'a mut Context,
}

impl PanelMutApi<'_> {
    pub fn handle_control_event(&mut self, event: &ControlEvent) -> bool {
        self.ctx.handle_panel_control_event(event)
    }

    pub fn ensure_runtime(
        &mut self,
        config: &crate::panel::PanelConfig,
    ) -> Option<crate::panel::PanelRuntime> {
        self.ctx.ensure_panel_runtime(config)
    }

    pub fn import_layouts(&mut self, layouts: &[crate::panel::PanelLayout]) {
        self.ctx.import_panel_layouts(layouts);
    }
}
