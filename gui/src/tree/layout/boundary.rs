use std::collections::BTreeSet;

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
    pub boundaries: BTreeSet<crate::tree::NodeId>,
    pub text_nodes: BTreeSet<crate::tree::NodeId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutFlushStats {
    pub boundaries_flushed: usize,
    pub boundaries_skipped_cache_hit: usize,
    pub nodes_visited: usize,
}
