use crate::interaction::InteractionState;
use crate::tree::{NodeId, Tree};

use super::popup::{OverlayRequest, PopupSystem};

pub(crate) struct SystemCx<'a> {
    tree: &'a Tree,
    interaction: &'a mut InteractionState,
}

impl<'a> SystemCx<'a> {
    pub(crate) fn new(tree: &'a Tree, interaction: &'a mut InteractionState) -> Self {
        Self { tree, interaction }
    }

    pub(crate) fn tree(&self) -> &Tree {
        self.tree
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn captured_node(&self) -> Option<NodeId> {
        self.interaction.captured()
    }

    pub(crate) fn focus(&mut self, node_id: NodeId) {
        self.interaction.focus(node_id);
    }

    pub(crate) fn blur(&mut self) {
        self.interaction.blur();
    }
}

pub(crate) struct OverlaySystemCx<'a> {
    tree: &'a Tree,
    interaction: &'a mut InteractionState,
    popup: &'a mut PopupSystem,
}

impl<'a> OverlaySystemCx<'a> {
    pub(crate) fn new(
        tree: &'a Tree,
        interaction: &'a mut InteractionState,
        popup: &'a mut PopupSystem,
    ) -> Self {
        Self {
            tree,
            interaction,
            popup,
        }
    }

    pub(crate) fn tree(&self) -> &Tree {
        self.tree
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn popup_open(&self) -> bool {
        self.popup.is_open()
    }

    pub(crate) fn open_overlay(&mut self, request: OverlayRequest) {
        self.popup.open(self.tree, request);
    }

    pub(crate) fn close_overlay(&mut self) {
        self.popup
            .close(SystemCx::new(self.tree, &mut *self.interaction));
    }

    pub(crate) fn close_overlay_no_focus_restore(&mut self) {
        self.popup.close_no_focus_restore();
    }
}
