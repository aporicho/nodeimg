use crate::animation::AnimationStore;
use crate::interaction::InteractionState;
use crate::overlay::{OverlayRequest, OverlaySystem};
use crate::renderer::Point;
use crate::tree::{hit_test_with_animations, HitChain, NodeId, Tree};
use crate::widget::state::dropdown::DropdownRuntime;

pub(crate) struct SystemCx<'a> {
    tree: &'a Tree,
    animations: Option<&'a AnimationStore>,
    interaction: &'a mut InteractionState,
}

impl<'a> SystemCx<'a> {
    pub(crate) fn new(
        tree: &'a Tree,
        animations: Option<&'a AnimationStore>,
        interaction: &'a mut InteractionState,
    ) -> Self {
        Self {
            tree,
            animations,
            interaction,
        }
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

    pub(crate) fn hit_chain(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(self.tree, root, x, y, self.animations)
    }

    pub(crate) fn node_name(&self, node_id: NodeId) -> Option<&str> {
        self.tree.get(node_id).map(|node| node.id.as_ref())
    }

    pub(crate) fn node_id_by_name(&self, id: &str) -> Option<NodeId> {
        self.tree
            .iter()
            .find_map(|(node_id, node)| (node.id.as_ref() == id).then_some(node_id))
    }

    pub(crate) fn screen_to_node_layout_point(
        &self,
        node_id: NodeId,
        x: f32,
        y: f32,
    ) -> Option<Point> {
        let root = self.tree.root()?;
        crate::tree::screen_to_node_layout_point(self.tree, root, node_id, x, y, self.animations)
    }

    pub(crate) fn blur(&mut self) {
        self.interaction.blur();
    }
}

pub(crate) struct OverlaySystemCx<'a> {
    tree: &'a mut Tree,
    animations: Option<&'a AnimationStore>,
    interaction: &'a mut InteractionState,
    overlay: &'a mut OverlaySystem,
}

impl<'a> OverlaySystemCx<'a> {
    pub(crate) fn new(
        tree: &'a mut Tree,
        animations: Option<&'a AnimationStore>,
        interaction: &'a mut InteractionState,
        overlay: &'a mut OverlaySystem,
    ) -> Self {
        Self {
            tree,
            animations,
            interaction,
            overlay,
        }
    }

    pub(crate) fn tree(&self) -> &Tree {
        self.tree
    }

    pub(crate) fn hit_chain(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(self.tree, root, x, y, self.animations)
    }

    pub(crate) fn dropdown_runtime(&self) -> &DropdownRuntime {
        self.tree
            .runtime_slot_by_stable_id::<DropdownRuntime>("__dropdown_runtime")
            .expect("dropdown runtime should be ensured before event handling")
    }

    pub(crate) fn dropdown_runtime_mut(&mut self) -> &mut DropdownRuntime {
        self.tree
            .ensure_runtime_slot_by_stable_id::<DropdownRuntime>("__dropdown_runtime")
    }

    pub(crate) fn focused_node(&self) -> Option<NodeId> {
        self.interaction.focused()
    }

    pub(crate) fn overlay_open(&self) -> bool {
        self.overlay.is_open()
    }

    pub(crate) fn open_overlay(&mut self, request: OverlayRequest) {
        self.overlay.open(self.tree, request);
    }

    pub(crate) fn close_overlay(&mut self) {
        self.overlay.close(self.tree, &mut *self.interaction);
    }

    pub(crate) fn close_overlay_no_focus_restore(&mut self) {
        self.overlay.close_no_focus_restore();
    }
}
