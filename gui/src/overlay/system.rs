#[cfg(test)]
use super::builder::compose_overlay_desc;
use super::dismiss::{close_overlay, handle_dismiss_event, DismissOutcome};
#[cfg(test)]
use super::placement::resolve_placement;
use super::runtime::OverlayState;
use super::OverlayRequest;
use crate::animation::AnimationStore;
use crate::interaction::InteractionState;
use crate::shell::AppEvent;
#[cfg(test)]
use crate::tree::Desc;
use crate::tree::Tree;

pub(crate) struct OverlaySystem {
    current: Option<OverlayState>,
}

impl OverlaySystem {
    pub(crate) fn new() -> Self {
        Self { current: None }
    }

    pub(crate) fn open(&mut self, tree: &Tree, request: OverlayRequest) {
        #[cfg(test)]
        let (x, y, width) = resolve_placement(tree, &request, (0.0, 0.0, None));
        #[cfg(not(test))]
        let _ = tree;
        self.current = Some(OverlayState {
            request,
            #[cfg(test)]
            last_x: x,
            #[cfg(test)]
            last_y: y,
            #[cfg(test)]
            last_width: width,
        });
    }

    pub(crate) fn close(&mut self, tree: &Tree, interaction: &mut InteractionState) {
        close_overlay(self.current.take(), tree, interaction);
    }

    pub(crate) fn close_no_focus_restore(&mut self) {
        self.current = None;
    }

    #[cfg(test)]
    pub(crate) fn compose_desc(
        &mut self,
        tree: &Tree,
        base_desc: Desc,
        viewport: crate::renderer::Rect,
    ) -> Desc {
        let Some(state) = self.current.as_mut() else {
            return base_desc;
        };
        compose_overlay_desc(state, tree, base_desc, viewport)
    }

    pub(crate) fn handle_event(
        &mut self,
        tree: &Tree,
        animations: Option<&AnimationStore>,
        interaction: &mut InteractionState,
        event: &AppEvent,
    ) -> bool {
        let Some(state) = &self.current else {
            return false;
        };
        match handle_dismiss_event(state, tree, animations, event) {
            DismissOutcome::Keep => false,
            DismissOutcome::CloseAndConsume => {
                self.close(tree, interaction);
                true
            }
            DismissOutcome::CloseNoFocusRestore => {
                self.close_no_focus_restore();
                false
            }
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.current.is_some()
    }
}

impl Default for OverlaySystem {
    fn default() -> Self {
        Self::new()
    }
}
