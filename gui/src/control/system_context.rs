use crate::animation::AnimationStore;
use crate::input::{PointerHitResolver, PointerHitSnapshot};
use crate::interaction::InteractionState;
use crate::renderer::Point;
use crate::tree::{HitChain, NodeId, Tree};

pub(crate) struct SystemCx<'a> {
    tree: &'a Tree,
    animations: Option<&'a AnimationStore>,
    interaction: &'a mut InteractionState,
    pointer_hit: Option<&'a PointerHitSnapshot>,
}

impl<'a> SystemCx<'a> {
    pub(crate) fn new(
        tree: &'a Tree,
        animations: Option<&'a AnimationStore>,
        interaction: &'a mut InteractionState,
        pointer_hit: Option<&'a PointerHitSnapshot>,
    ) -> Self {
        Self {
            tree,
            animations,
            interaction,
            pointer_hit,
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
        PointerHitResolver::new(self.tree, self.animations).chain_at(self.pointer_hit, x, y)
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
