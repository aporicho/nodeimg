use crate::tree::NodeId;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelayoutBoundary {
    pub root: NodeId,
    pub reason: RelayoutBoundaryReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelayoutBoundaryReason {
    Root,
    Panel,
    CanvasRoot,
    CanvasNodeCard,
    FixedConstraintTextField,
    Explicit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayoutDirtyReason {
    Structure,
    Style,
    Size,
    TextIntrinsic,
    Viewport,
    Explicit,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutDirtyQueues {
    pub boundaries: BTreeSet<NodeId>,
    pub text_nodes: BTreeSet<NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutFlushStats {
    pub boundaries_flushed: usize,
    pub boundaries_skipped_cache_hit: usize,
    pub nodes_visited: usize,
}
