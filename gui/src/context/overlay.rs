use super::Context;
use crate::diagnostics::render_trace::TARGET_RENDER;
use crate::overlay::OverlayRequest;

impl Context {
    pub(in crate::context) fn sync_overlay_tree(&mut self) {
        if let Err(error) = self.systems.sync_overlay_tree(
            &mut self.tree,
            &self.template_registry,
            &self.current_theme,
        ) {
            tracing::warn!(
                target: TARGET_RENDER,
                ?error,
                "failed to sync retained overlay tree"
            );
        }
    }

    pub(crate) fn open_overlay(&mut self, request: OverlayRequest) {
        self.systems.open_overlay(&mut self.tree, request);
        self.sync_overlay_tree();
    }

    pub(crate) fn close_overlay(&mut self) {
        self.systems
            .close_overlay(&mut self.tree, &mut self.interaction);
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.systems.overlay_open()
    }
}
