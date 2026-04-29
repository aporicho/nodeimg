use super::{DirtyQueues, TreeIndexError};
use super::{NodeId, PaintDirtyQueues, RepaintBoundaryId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeDumpLevel {
    #[default]
    Normal,
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeSnapshotMaxNodes {
    Limit(usize),
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeSnapshotOptions {
    pub level: TreeDumpLevel,
    pub max_nodes: TreeSnapshotMaxNodes,
}

impl TreeSnapshotOptions {
    pub const fn normal(max_nodes: usize) -> Self {
        Self {
            level: TreeDumpLevel::Normal,
            max_nodes: TreeSnapshotMaxNodes::Limit(max_nodes),
        }
    }

    pub const fn full() -> Self {
        Self {
            level: TreeDumpLevel::Full,
            max_nodes: TreeSnapshotMaxNodes::All,
        }
    }
}

impl Default for TreeSnapshotOptions {
    fn default() -> Self {
        Self::normal(500)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSnapshot {
    pub summary: TreeSnapshotSummary,
    pub nodes: Vec<TreeSnapshotNode>,
    pub issues: Vec<TreeSnapshotIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSnapshotSummary {
    pub level: TreeDumpLevel,
    pub root: Option<NodeId>,
    pub live_nodes: usize,
    pub indexed_nodes: usize,
    pub dirty: DirtyQueueSummary,
    pub layout_dirty: LayoutDirtyQueueSummary,
    pub paint_dirty: PaintDirtyQueueSummary,
    pub caches: TreeCacheSummary,
    pub issues: usize,
    pub truncated: bool,
    pub omitted_nodes: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirtyQueueSummary {
    pub structure: usize,
    pub layout: usize,
    pub text_layout: usize,
    pub paint: usize,
    pub hit: usize,
    pub paint_order: usize,
    pub composite: usize,
    pub paint_placement: usize,
}

impl DirtyQueueSummary {
    pub fn total(&self) -> usize {
        self.structure
            + self.layout
            + self.text_layout
            + self.paint
            + self.hit
            + self.paint_order
            + self.composite
            + self.paint_placement
    }
}

impl From<&DirtyQueues> for DirtyQueueSummary {
    fn from(value: &DirtyQueues) -> Self {
        Self {
            structure: value.structure.len(),
            layout: value.layout.len(),
            text_layout: value.text_layout.len(),
            paint: value.paint.len(),
            hit: value.hit.len(),
            paint_order: value.paint_order.len(),
            composite: value.composite.len(),
            paint_placement: value.paint_placement.len(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutDirtyQueueSummary {
    pub boundaries: usize,
    pub text_nodes: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaintDirtyQueueSummary {
    pub boundaries: usize,
    pub paint_order: usize,
    pub composite: usize,
    pub placement: usize,
}

impl PaintDirtyQueueSummary {
    pub fn total(&self) -> usize {
        self.boundaries + self.paint_order + self.composite + self.placement
    }
}

impl From<&PaintDirtyQueues> for PaintDirtyQueueSummary {
    fn from(value: &PaintDirtyQueues) -> Self {
        Self {
            boundaries: value.boundaries.len(),
            paint_order: value.paint_order.len(),
            composite: value.composite.len(),
            placement: value.placement.len(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeCacheSummary {
    pub layout_entries: usize,
    pub hit_order_entries: usize,
    pub paint_order_entries: usize,
    pub paint_fragments: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSnapshotNode {
    pub node: NodeId,
    pub depth: usize,
    pub stable_id: String,
    pub line: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSnapshotIssue {
    pub message: String,
}

impl TreeSnapshot {
    pub fn has_dirty(&self) -> bool {
        self.summary.dirty.total() > 0
            || self.summary.layout_dirty.boundaries > 0
            || self.summary.layout_dirty.text_nodes > 0
            || self.summary.paint_dirty.total() > 0
    }

    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }
}

pub(crate) fn index_error_message(error: TreeIndexError) -> String {
    match error {
        TreeIndexError::DuplicateStableId {
            stable_id,
            existing,
            duplicate,
        } => format!(
            "duplicate stable id stable_id={stable_id:?} existing={existing} duplicate={duplicate}"
        ),
        TreeIndexError::MissingIndexedNode { stable_id, node } => {
            format!("missing indexed node stable_id={stable_id:?} node={node}")
        }
        TreeIndexError::StaleIndexEntry { stable_id, node } => {
            format!("stale index entry stable_id={stable_id:?} node={node}")
        }
    }
}

pub(crate) fn repaint_boundary_list(
    boundaries: impl IntoIterator<Item = RepaintBoundaryId>,
) -> Vec<NodeId> {
    boundaries.into_iter().map(|boundary| boundary.0).collect()
}
