use super::{NodeId, Revision};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RepaintBoundaryId(pub NodeId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RepaintBoundaryReason {
    Root,
    PanelFrame,
    CanvasRoot,
    CanvasGridLayer,
    CanvasConnectionLayer,
    CanvasNodeCard,
    ActiveTextEditor,
    Overlay,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintDirtyReason {
    Visual,
    Text,
    PaintOrder,
    Placement,
    Structure,
    Theme,
    Animation,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaintDirtyQueues {
    pub boundaries: BTreeSet<RepaintBoundaryId>,
    pub paint_order: BTreeSet<RepaintBoundaryId>,
    pub composite: BTreeSet<NodeId>,
    pub placement: BTreeSet<RepaintBoundaryId>,
}

impl PaintDirtyQueues {
    pub fn remove_node(&mut self, node: NodeId) {
        let boundary = RepaintBoundaryId(node);
        self.boundaries.remove(&boundary);
        self.paint_order.remove(&boundary);
        self.composite.remove(&node);
        self.placement.remove(&boundary);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodePaintMeta {
    pub boundary: Option<RepaintBoundaryReason>,
    pub visual_revision: Revision,
    pub text_paint_revision: Revision,
    pub paint_order_revision: Revision,
    pub fragment_revision: Revision,
}

impl NodePaintMeta {
    pub fn boundary(reason: RepaintBoundaryReason) -> Self {
        Self {
            boundary: Some(reason),
            ..Self::default()
        }
    }

    pub fn set_boundary(&mut self, reason: RepaintBoundaryReason) {
        self.boundary = Some(reason);
    }

    pub fn bump_visual(&mut self) {
        self.visual_revision = self.visual_revision.next();
    }

    pub fn bump_text(&mut self) {
        self.text_paint_revision = self.text_paint_revision.next();
    }

    pub fn bump_paint_order(&mut self) {
        self.paint_order_revision = self.paint_order_revision.next();
    }

    pub fn bump_fragment(&mut self) {
        self.fragment_revision = self.fragment_revision.next();
    }
}

impl Default for NodePaintMeta {
    fn default() -> Self {
        Self {
            boundary: None,
            visual_revision: Revision::ZERO,
            text_paint_revision: Revision::ZERO,
            paint_order_revision: Revision::ZERO,
            fragment_revision: Revision::ZERO,
        }
    }
}
