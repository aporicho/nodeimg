use crate::animation::AnimationStore;
use crate::tree::{
    hit_test_with_animations, resize_hit_at_screen_point, HitChain, ResizeHit, TargetChain, Tree,
};

use super::{PointerHitRequest, PointerHitSnapshot};

pub(crate) struct PointerHitResolver<'a> {
    tree: &'a Tree,
    animations: Option<&'a AnimationStore>,
}

impl<'a> PointerHitResolver<'a> {
    pub(crate) fn new(tree: &'a Tree, animations: Option<&'a AnimationStore>) -> Self {
        Self { tree, animations }
    }

    pub(crate) fn snapshot_for_request(&self, request: PointerHitRequest) -> PointerHitSnapshot {
        let chain = self.hit_chain_at(request.x(), request.y());
        let resize_hit = request
            .include_resize_hit()
            .then(|| self.compute_resize_hit_at(request.x(), request.y()))
            .flatten();
        PointerHitSnapshot::new(request.x(), request.y(), chain, resize_hit)
    }

    pub(crate) fn chain_at(
        &self,
        snapshot: Option<&PointerHitSnapshot>,
        x: f32,
        y: f32,
    ) -> HitChain {
        if let Some(snapshot) = snapshot.filter(|snapshot| snapshot.matches_point(x, y)) {
            return snapshot.chain().clone();
        }
        self.hit_chain_at(x, y)
    }

    pub(crate) fn resize_hit_at(
        &self,
        snapshot: Option<&PointerHitSnapshot>,
        x: f32,
        y: f32,
    ) -> Option<ResizeHit> {
        snapshot
            .filter(|snapshot| snapshot.matches_point(x, y))
            .and_then(PointerHitSnapshot::raw_resize_hit)
            .or_else(|| self.compute_resize_hit_at(x, y))
    }

    pub(crate) fn target_chain_at(
        &self,
        snapshot: Option<&PointerHitSnapshot>,
        x: f32,
        y: f32,
    ) -> TargetChain {
        let chain = self.chain_at(snapshot, x, y);
        TargetChain::from_hit_chain(self.tree, &chain)
    }

    fn hit_chain_at(&self, x: f32, y: f32) -> HitChain {
        let Some(root) = self.tree.root() else {
            return HitChain::empty();
        };
        hit_test_with_animations(self.tree, root, x, y, self.animations)
    }

    fn compute_resize_hit_at(&self, x: f32, y: f32) -> Option<ResizeHit> {
        let root = self.tree.root()?;
        resize_hit_at_screen_point(self.tree, root, x, y, self.animations)
    }
}
