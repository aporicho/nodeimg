use crate::animation::AnimationStore;
use crate::tree::layout::Overflow;
use crate::tree::{NodeId, Tree};

use crate::input::{PointerHitResolver, PointerHitSnapshot};

use super::ScrollRequest;

pub(crate) struct ScrollTargetResolver<'a> {
    tree: &'a Tree,
    pointer_hit: PointerHitResolver<'a>,
}

impl<'a> ScrollTargetResolver<'a> {
    pub(crate) fn new(tree: &'a Tree, animations: Option<&'a AnimationStore>) -> Self {
        Self {
            tree,
            pointer_hit: PointerHitResolver::new(tree, animations),
        }
    }

    pub(crate) fn target_for_request(
        &self,
        snapshot: Option<&PointerHitSnapshot>,
        request: ScrollRequest,
    ) -> Option<NodeId> {
        self.target_at(snapshot, request.x(), request.y())
    }

    fn target_at(&self, snapshot: Option<&PointerHitSnapshot>, x: f32, y: f32) -> Option<NodeId> {
        let chain = self.pointer_hit.chain_at(snapshot, x, y);
        let target = chain.iter().find(|&node_id| {
            self.tree
                .get(node_id)
                .map(|node| node.style.overflow == Overflow::Scroll)
                .unwrap_or(false)
        });
        target
    }
}
