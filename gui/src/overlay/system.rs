use super::dismiss::{close_overlay, handle_dismiss_event, DismissOutcome};
use super::placement::resolve_placement;
use super::runtime::OverlayState;
use super::OverlayRequest;
use crate::animation::AnimationStore;
use crate::input::PointerHitSnapshot;
use crate::interaction::InteractionState;
use crate::shell::AppEvent;
use crate::template::{
    InstanceId, TemplateId, TemplatePayload, TemplateRegistry, DROPDOWN_OVERLAY_TEMPLATE,
};
use crate::theme::Theme;
use crate::tree::{DirtyFlags, MutationError, NodeId, Tree, TreeMutation};

pub(crate) struct OverlaySystem {
    current: Option<OverlayState>,
}

impl OverlaySystem {
    pub(crate) fn new() -> Self {
        Self { current: None }
    }

    pub(crate) fn open(&mut self, tree: &mut Tree, request: OverlayRequest) {
        self.remove_current_root(tree);
        let (x, y, width) = resolve_placement(tree, &request, (0.0, 0.0, None));
        self.current = Some(OverlayState {
            request,
            last_x: x,
            last_y: y,
            last_width: width,
            root: None,
            dirty: true,
        });
    }

    pub(crate) fn close(&mut self, tree: &mut Tree, interaction: &mut InteractionState) {
        close_overlay(self.current.take(), tree, interaction);
    }

    pub(crate) fn close_no_focus_restore(&mut self, tree: &mut Tree) {
        if let Some(state) = self.current.take() {
            remove_overlay_root(tree, state.root);
        }
    }

    pub(crate) fn handle_event(
        &mut self,
        tree: &mut Tree,
        animations: Option<&AnimationStore>,
        interaction: &mut InteractionState,
        pointer_hit: Option<&PointerHitSnapshot>,
        event: &AppEvent,
    ) -> bool {
        let Some(state) = &self.current else {
            return false;
        };
        match handle_dismiss_event(state, tree, animations, pointer_hit, event) {
            DismissOutcome::Keep => false,
            DismissOutcome::CloseAndConsume => {
                self.close(tree, interaction);
                true
            }
            DismissOutcome::CloseNoFocusRestore => {
                self.close_no_focus_restore(tree);
                false
            }
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.current.is_some()
    }

    pub(crate) fn sync_tree(
        &mut self,
        tree: &mut Tree,
        registry: &TemplateRegistry,
        theme: &Theme,
    ) -> Result<(), MutationError> {
        let Some(state) = self.current.as_mut() else {
            return Ok(());
        };
        if !state.dirty {
            return Ok(());
        }
        let Some(parent) = tree.node_by_str("__overlay_root") else {
            return Ok(());
        };
        remove_overlay_root(tree, state.root.take());
        let payload =
            TemplatePayload::DropdownOverlay(crate::overlay::DropdownOverlayTemplateData::new(
                state.request.id.clone(),
                state.last_x,
                state.last_y,
                state.last_width,
                state.request.content.clone(),
                theme,
            ));
        let root_id = format!("__overlay::{}", state.request.id);
        tree.apply_mutation(
            registry,
            TreeMutation::MountTemplate {
                parent,
                template: TemplateId::from(DROPDOWN_OVERLAY_TEMPLATE),
                instance: InstanceId::from(state.request.id.clone()),
                payload,
            },
        )?;
        state.root = tree.node_by_str(&root_id);
        state.dirty = false;
        Ok(())
    }

    fn remove_current_root(&mut self, tree: &mut Tree) {
        if let Some(state) = self.current.as_mut() {
            remove_overlay_root(tree, state.root.take());
        }
    }
}

impl Default for OverlaySystem {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn remove_overlay_root(tree: &mut Tree, root: Option<NodeId>) {
    let Some(root) = root else {
        return;
    };
    if tree.get(root).is_none() {
        return;
    }
    let dirty_node = tree
        .parent_of(root)
        .or_else(|| tree.node_by_str("__overlay_root"))
        .or_else(|| tree.root())
        .unwrap_or(root);
    tree.detach_from_parent(root);
    tree.remove(root);
    tree.mark_dirty(
        dirty_node,
        DirtyFlags::STRUCTURE | DirtyFlags::LAYOUT | DirtyFlags::HIT | DirtyFlags::PAINT,
    );
}
