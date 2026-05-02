use crate::tree::dirty::DirtyQueues;
use crate::tree::frame_stats::FrameStats;
use crate::tree::hit_order::HitOrderCache;
use crate::tree::index::TreeIndex;
use crate::tree::layout::{LayoutCache, LayoutDirtyQueues};
use crate::tree::node::{NodeId, TreeNode};
use crate::tree::paint_cache::PaintCache;
use crate::tree::paint_order::PaintOrderCache;
use crate::tree::repaint::PaintDirtyQueues;
use crate::tree::retained_runtime::RetainedRuntimeStore;
use std::cell::RefCell;
use std::collections::HashMap;

/// 全局控件树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct Tree {
    pub(in crate::tree::state) nodes: Vec<Option<TreeNode>>,
    pub(in crate::tree::state) root: Option<NodeId>,
    pub(in crate::tree::state) free: Vec<NodeId>,
    pub(in crate::tree::state) retained_runtime: RetainedRuntimeStore,
    pub(in crate::tree::state) index: TreeIndex,
    pub(in crate::tree::state) parents: HashMap<NodeId, NodeId>,
    pub(in crate::tree::state) dirty: DirtyQueues,
    pub(in crate::tree::state) layout_dirty: LayoutDirtyQueues,
    pub(in crate::tree::state) layout_cache: LayoutCache,
    pub(in crate::tree::state) hit_order_cache: RefCell<HitOrderCache>,
    pub(in crate::tree::state) paint_dirty: PaintDirtyQueues,
    pub(in crate::tree::state) paint_cache: RefCell<PaintCache>,
    pub(in crate::tree::state) paint_order_cache: RefCell<PaintOrderCache>,
    pub(in crate::tree::state) frame_stats: RefCell<FrameStats>,
}

impl Tree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: None,
            free: Vec::new(),
            retained_runtime: RetainedRuntimeStore::default(),
            index: TreeIndex::new(),
            parents: HashMap::new(),
            dirty: DirtyQueues::default(),
            layout_dirty: LayoutDirtyQueues::default(),
            layout_cache: LayoutCache::default(),
            hit_order_cache: RefCell::new(HitOrderCache::default()),
            paint_dirty: PaintDirtyQueues::default(),
            paint_cache: RefCell::new(PaintCache::default()),
            paint_order_cache: RefCell::new(PaintOrderCache::default()),
            frame_stats: RefCell::new(FrameStats::default()),
        }
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}
