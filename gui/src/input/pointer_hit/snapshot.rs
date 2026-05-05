use crate::control::ResizeEdge;
use crate::tree::{HitChain, NodeId, ResizeHit};

#[derive(Debug, Clone)]
pub struct PointerHitSnapshot {
    x: f32,
    y: f32,
    chain: HitChain,
    resize_hit: Option<ResizeHit>,
}

impl PointerHitSnapshot {
    pub(crate) fn new(x: f32, y: f32, chain: HitChain, resize_hit: Option<ResizeHit>) -> Self {
        Self {
            x,
            y,
            chain,
            resize_hit,
        }
    }

    pub fn x(&self) -> f32 {
        self.x
    }

    pub fn y(&self) -> f32 {
        self.y
    }

    pub fn matches_point(&self, x: f32, y: f32) -> bool {
        self.x == x && self.y == y
    }

    pub fn chain(&self) -> &HitChain {
        &self.chain
    }

    pub fn resize_hit(&self) -> Option<(NodeId, ResizeEdge)> {
        self.resize_hit.map(|hit| (hit.node_id, hit.edge))
    }

    pub(crate) fn raw_resize_hit(&self) -> Option<ResizeHit> {
        self.resize_hit
    }
}
