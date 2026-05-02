use super::dirty::{DirtyFlags, DirtyQueues};
use super::frame_stats::FrameStats;
use super::hit_order::HitOrderCache;
use super::index::{TreeIndex, TreeIndexError};
use super::layout::{
    LayoutCache, LayoutConstraints, LayoutDependencyKind, LayoutDependencyScope, LayoutDirtyQueues,
    LayoutDirtyReason, LayoutInput, LayoutOutput, RelayoutBoundaryReason,
};
use super::node::{NodeId, TreeNode};
use super::paint_cache::PaintCache;
use super::paint_order::PaintOrderCache;
use super::repaint::{
    PaintDirtyQueues, PaintDirtyReason, RepaintBoundaryId, RepaintBoundaryReason,
};
use super::runtime_slots::RuntimeSlot;
use super::snapshot::{
    index_error_message, repaint_boundary_list, DirtyQueueSummary, LayoutDirtyQueueSummary,
    PaintDirtyQueueSummary, TreeCacheSummary, TreeDumpLevel, TreeSnapshot, TreeSnapshotIssue,
    TreeSnapshotMaxNodes, TreeSnapshotNode, TreeSnapshotOptions, TreeSnapshotSummary,
};
use super::{RetainedRuntimeStore, RuntimeSlots, StableId};
use crate::canvas::runtime::{CanvasInteractionRuntime, CanvasNodeRuntime};
use crate::canvas::{
    canvas_node_stable_id, CanvasNodeIdentity, CanvasNodeLayout, CanvasPortGroupView,
    CanvasPortSide,
};
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::paint::{compose_fragments, DisplayList, PaintBuildError, PaintFlushStats};
use crate::panel::{
    PanelConfig, PanelLayout, PanelPointerSession, PanelResizeSession, PanelRootRuntime,
    PanelRuntime,
};
use crate::renderer::Rect;
use crate::widget::resize_edge::ResizeEdge;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

const PANEL_ROOT_ID: &str = "panel_root";
const CANVAS_INTERACTION_ID: &str = "canvas_interaction";
const CANVAS_NODE_MIN_WIDTH: f32 = 304.0;
const CANVAS_NODE_MIN_HEIGHT: f32 = 132.0;

#[allow(dead_code)]
#[derive(Debug)]
struct PaintDirtyTraceSummary<'a> {
    node: NodeId,
    stable_id: Option<&'a str>,
    reason: PaintDirtyReason,
    boundary: Option<NodeId>,
    dirty_boundaries: usize,
    dirty_paint_order: usize,
    dirty_composite: usize,
    dirty_placement: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
struct LayoutDirtyTraceSummary<'a> {
    node: NodeId,
    stable_id: Option<&'a str>,
    reason: LayoutDirtyReason,
    boundary: Option<NodeId>,
    dirty_boundaries: usize,
    dirty_text_nodes: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
struct DirtyPropagationTraceSummary<'a> {
    node: NodeId,
    stable_id: Option<&'a str>,
    flags: String,
    structure: usize,
    layout: usize,
    text_layout: usize,
    paint: usize,
    hit: usize,
    paint_order: usize,
    composite: usize,
    paint_placement: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PaintFragmentTraceSummary {
    commands_recorded: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
struct PaintCompositionTraceSummary {
    root_boundary: NodeId,
    display_commands: usize,
    display_clips: usize,
    fragments_flattened: usize,
}

/// 全局控件树存储。用 Vec<Option<>> 做 arena，索引访问。
pub struct Tree {
    nodes: Vec<Option<TreeNode>>,
    root: Option<NodeId>,
    free: Vec<NodeId>,
    retained_runtime: RetainedRuntimeStore,
    index: TreeIndex,
    parents: HashMap<NodeId, NodeId>,
    dirty: DirtyQueues,
    layout_dirty: LayoutDirtyQueues,
    layout_cache: LayoutCache,
    hit_order_cache: RefCell<HitOrderCache>,
    paint_dirty: PaintDirtyQueues,
    paint_cache: RefCell<PaintCache>,
    paint_order_cache: RefCell<PaintOrderCache>,
    frame_stats: RefCell<FrameStats>,
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

    pub fn insert(&mut self, node: TreeNode) -> NodeId {
        self.insert_with_index_policy(node, false)
            .expect("compat insertion does not reject duplicate stable ids")
    }

    pub fn insert_checked(&mut self, node: TreeNode) -> Result<NodeId, TreeIndexError> {
        self.insert_with_index_policy(node, true)
    }

    fn insert_with_index_policy(
        &mut self,
        mut node: TreeNode,
        reject_duplicate: bool,
    ) -> Result<NodeId, TreeIndexError> {
        let retained_slots = self.retained_runtime.take(node.id.as_ref());
        if let Some(slots) = retained_slots {
            node.runtime_slots = slots;
        }
        if let Some(id) = self.free.pop() {
            self.index.remove_node(id);
            self.parents.remove(&id);
            self.layout_cache.clear();
            self.hit_order_cache.borrow_mut().clear();
            self.paint_cache.borrow_mut().evict(RepaintBoundaryId(id));
            self.paint_order_cache.borrow_mut().clear();
            self.paint_dirty.remove_node(id);
            self.layout_dirty.boundaries.remove(&id);
            self.layout_dirty.text_nodes.remove(&id);
            let stable_id = node.id.clone();
            let children = node.children.clone();
            self.nodes[id] = Some(node);
            if let Err(error) = self.index.register(stable_id, id) {
                if reject_duplicate {
                    self.nodes[id] = None;
                    self.free.push(id);
                    return Err(error);
                }
            }
            self.reparent_children(id, &children);
            Ok(id)
        } else {
            let id = self.nodes.len();
            let stable_id = node.id.clone();
            let children = node.children.clone();
            self.nodes.push(Some(node));
            if let Err(error) = self.index.register(stable_id, id) {
                if reject_duplicate {
                    self.nodes.pop();
                    return Err(error);
                }
            }
            self.reparent_children(id, &children);
            Ok(id)
        }
    }

    pub fn remove(&mut self, id: NodeId) {
        if id < self.nodes.len() {
            let evicted = self.paint_cache.borrow_mut().evict_subtree(self, id);
            self.record_paint_fragments_evicted(evicted);
            // 递归删除子节点
            if let Some(node) = self.nodes[id].take() {
                self.index.unregister(&node.id, id);
                self.parents.remove(&id);
                self.layout_dirty.boundaries.remove(&id);
                self.layout_dirty.text_nodes.remove(&id);
                self.paint_dirty.remove_node(id);
                self.layout_cache.clear();
                self.hit_order_cache.borrow_mut().clear();
                self.paint_order_cache.borrow_mut().clear();
                let children = node.children;
                for child_id in children {
                    self.parents.remove(&child_id);
                    self.remove(child_id);
                }
                if !node.runtime_slots.is_empty() {
                    self.retained_runtime
                        .preserve(node.id.as_ref().to_string(), node.runtime_slots);
                }
                self.free.push(id);
            }
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&TreeNode> {
        self.nodes.get(id).and_then(|n| n.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut TreeNode> {
        self.nodes.get_mut(id).and_then(|n| n.as_mut())
    }

    pub fn node_by_stable_id(&self, stable_id: &StableId) -> Option<NodeId> {
        self.record_stable_id_lookup();
        self.index.get(stable_id)
    }

    pub fn node_by_str(&self, stable_id: &str) -> Option<NodeId> {
        self.record_stable_id_lookup();
        self.index.get_str(stable_id)
    }

    pub fn contains_stable_id(&self, stable_id: &str) -> bool {
        self.node_by_str(stable_id).is_some()
    }

    pub fn validate_index(&self) -> Result<(), TreeIndexError> {
        self.index.validate(self.iter())
    }

    pub fn runtime_slot<T: RuntimeSlot>(&self, id: NodeId) -> Option<&T> {
        self.get(id)?.runtime_slots.get::<T>()
    }

    pub fn runtime_slot_mut<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        self.get_mut(id)?.runtime_slots.get_mut::<T>()
    }

    pub fn ensure_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<&mut T> {
        Some(self.get_mut(id)?.runtime_slots.ensure::<T>())
    }

    pub fn remove_runtime_slot<T: RuntimeSlot>(&mut self, id: NodeId) -> Option<T> {
        self.get_mut(id)?.runtime_slots.remove::<T>()
    }

    #[cfg(test)]
    pub(crate) fn take_retained_runtime_slots(&mut self, id: &str) -> Option<RuntimeSlots> {
        self.retained_runtime.take(id)
    }

    pub(crate) fn runtime_slot_by_stable_id<T: RuntimeSlot>(&self, id: &str) -> Option<&T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot::<T>(node_id);
        }
        self.retained_runtime.get::<T>(id)
    }

    pub(crate) fn runtime_slot_by_stable_id_mut<T: RuntimeSlot>(
        &mut self,
        id: &str,
    ) -> Option<&mut T> {
        if let Some(node_id) = self.node_by_str(id) {
            return self.runtime_slot_mut::<T>(node_id);
        }
        self.retained_runtime.get_mut::<T>(id)
    }

    pub(crate) fn ensure_runtime_slot_by_stable_id<T: RuntimeSlot>(&mut self, id: &str) -> &mut T {
        if let Some(node_id) = self.node_by_str(id) {
            return self
                .ensure_runtime_slot::<T>(node_id)
                .expect("node id came from this tree");
        }
        self.retained_runtime.ensure::<T>(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &TreeNode)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(id, node)| node.as_ref().map(|node| (id, node)))
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, id: NodeId) {
        self.root = Some(id);
        self.parents.remove(&id);
        if let Some(node) = self.get_mut(id) {
            node.layout_meta.set_boundary(RelayoutBoundaryReason::Root);
            node.paint_meta.set_boundary(RepaintBoundaryReason::Root);
        }
    }

    pub(crate) fn detach_from_parent(&mut self, child: NodeId) {
        if let Some(parent_id) = self.parents.remove(&child) {
            if let Some(parent) = self.get_mut(parent_id) {
                parent.children.retain(|candidate| *candidate != child);
                parent.paint_meta.bump_paint_order();
            }
            self.record_layout_dependency_change(
                parent_id,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
            return;
        }

        let mut changed_parents = Vec::new();
        for (parent_id, node) in self
            .nodes
            .iter_mut()
            .enumerate()
            .filter_map(|(node_id, node)| node.as_mut().map(|node| (node_id, node)))
        {
            let before = node.children.len();
            node.children.retain(|candidate| *candidate != child);
            if node.children.len() != before {
                node.paint_meta.bump_paint_order();
                changed_parents.push(parent_id);
            }
        }
        for parent_id in changed_parents {
            self.record_layout_dependency_change(
                parent_id,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
        }
        if self.root == Some(child) {
            self.root = None;
        }
    }

    pub(crate) fn append_child(&mut self, parent: NodeId, child: NodeId) -> bool {
        if self.get(child).is_none() {
            return false;
        }
        let Some(parent_node) = self.get_mut(parent) else {
            return false;
        };
        parent_node.children.push(child);
        parent_node.paint_meta.bump_paint_order();
        self.parents.insert(child, parent);
        self.record_layout_dependency_change(
            parent,
            LayoutDependencyKind::Children,
            LayoutDependencyScope::RelayoutBoundary,
        );
        self.hit_order_cache.borrow_mut().clear();
        self.paint_order_cache.borrow_mut().clear();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_children(&mut self, parent: NodeId, children: Vec<NodeId>) -> bool {
        if self.get(parent).is_none() {
            return false;
        }
        let old_children = self
            .get(parent)
            .map(|node| node.children.clone())
            .unwrap_or_default();
        for child in old_children {
            self.parents.remove(&child);
        }
        self.reparent_children(parent, &children);
        let mut changed = false;
        if let Some(parent_node) = self.get_mut(parent) {
            if parent_node.children != children {
                parent_node.paint_meta.bump_paint_order();
                changed = true;
            }
            parent_node.children = children;
        }
        if changed {
            self.record_layout_dependency_change(
                parent,
                LayoutDependencyKind::Children,
                LayoutDependencyScope::RelayoutBoundary,
            );
        }
        self.hit_order_cache.borrow_mut().clear();
        self.paint_order_cache.borrow_mut().clear();
        true
    }

    fn reparent_children(&mut self, parent: NodeId, children: &[NodeId]) {
        for child in children {
            if self.get(*child).is_some() {
                self.parents.insert(*child, parent);
            }
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TreeNode> {
        self.nodes.iter_mut().filter_map(|n| n.as_mut())
    }

    pub(crate) fn children_in_hit_order_cached(
        &self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Vec<NodeId> {
        let (order, hit) = self
            .hit_order_cache
            .borrow_mut()
            .children_in_hit_order(self, parent, children);
        if hit {
            self.record_hit_order_cache_hit();
        } else {
            self.record_hit_order_cache_miss();
        }
        order
    }

    pub(crate) fn children_in_paint_order_cached(
        &self,
        parent: NodeId,
        children: &[NodeId],
    ) -> Vec<NodeId> {
        let (order, hit) = self
            .paint_order_cache
            .borrow_mut()
            .children_in_paint_order(self, parent, children);
        if hit {
            self.record_paint_order_cache_hit();
        } else {
            self.record_paint_order_cache_miss();
        }
        order
    }

    pub fn parent_of(&self, node: NodeId) -> Option<NodeId> {
        if let Some(parent) = self.parents.get(&node).copied() {
            return Some(parent);
        }
        self.record_full_tree_scan();
        self.iter().find_map(|(candidate, tree_node)| {
            tree_node.children.contains(&node).then_some(candidate)
        })
    }

    pub fn nearest_relayout_boundary(&self, node: NodeId) -> Option<NodeId> {
        let mut current = node;
        loop {
            let tree_node = self.get(current)?;
            if tree_node.layout_meta.boundary.is_some() {
                return Some(current);
            }
            let Some(parent) = self.parent_of(current) else {
                return self.root.or(Some(current));
            };
            current = parent;
        }
    }

    pub(crate) fn record_layout_dependency_change(
        &mut self,
        node: NodeId,
        kind: LayoutDependencyKind,
        scope: LayoutDependencyScope,
    ) {
        let Some(target) = self.get_mut(node) else {
            return;
        };
        match kind {
            LayoutDependencyKind::Style => target.layout_meta.bump_style(),
            LayoutDependencyKind::Text => target.layout_meta.bump_text(),
            LayoutDependencyKind::Children => target.layout_meta.bump_children(),
            LayoutDependencyKind::ExplicitRect => target.layout_meta.bump_explicit_rect(),
        }

        if scope == LayoutDependencyScope::LocalNode {
            return;
        }

        if let Some(boundary) = self.nearest_relayout_boundary(node) {
            if let Some(boundary_node) = self.get_mut(boundary) {
                boundary_node.layout_meta.bump_layout_dependency();
            }
            self.layout_cache.invalidate_node(boundary);
        }
    }

    pub fn set_repaint_boundary(&mut self, node: NodeId, reason: RepaintBoundaryReason) {
        if let Some(tree_node) = self.get_mut(node) {
            tree_node.paint_meta.set_boundary(reason);
        }
    }

    pub fn is_repaint_boundary(&self, node: NodeId) -> bool {
        self.get(node)
            .is_some_and(|tree_node| tree_node.paint_meta.boundary.is_some())
    }

    pub fn nearest_repaint_boundary(&self, node: NodeId) -> Option<RepaintBoundaryId> {
        let mut current = node;
        loop {
            let tree_node = self.get(current)?;
            if tree_node.paint_meta.boundary.is_some() {
                return Some(RepaintBoundaryId(current));
            }
            let Some(parent) = self.parent_of(current) else {
                return self.root.or(Some(current)).map(RepaintBoundaryId);
            };
            current = parent;
        }
    }

    pub fn mark_paint_dirty(&mut self, node: NodeId, reason: PaintDirtyReason) {
        if let Some(target) = self.get_mut(node) {
            match reason {
                PaintDirtyReason::Text => target.paint_meta.bump_text(),
                PaintDirtyReason::PaintOrder => target.paint_meta.bump_paint_order(),
                PaintDirtyReason::Placement => target.paint_meta.bump_fragment(),
                PaintDirtyReason::Visual
                | PaintDirtyReason::Structure
                | PaintDirtyReason::Theme
                | PaintDirtyReason::Animation => target.paint_meta.bump_visual(),
            }
        }

        let boundary = if matches!(reason, PaintDirtyReason::PaintOrder) {
            self.parent_of(node)
                .and_then(|parent| self.nearest_repaint_boundary(parent))
                .or_else(|| self.nearest_repaint_boundary(node))
        } else {
            self.nearest_repaint_boundary(node)
        };

        if let Some(boundary) = boundary {
            if let Some(boundary_node) = self.get_mut(boundary.0) {
                boundary_node.paint_meta.bump_fragment();
            }
            self.paint_dirty.boundaries.insert(boundary);
            if matches!(reason, PaintDirtyReason::PaintOrder) {
                self.paint_dirty.paint_order.insert(boundary);
            }
            self.dirty.paint.insert(boundary.0);
            self.record_paint_boundary_dirty();
        }

        if render_trace::is_debug_enabled() {
            render_trace::debug_stage(
                RenderTraceStage::DirtyPropagation,
                PaintDirtyTraceSummary {
                    node,
                    stable_id: self.get(node).map(|tree_node| tree_node.id.as_ref()),
                    reason,
                    boundary: boundary.map(|boundary| boundary.0),
                    dirty_boundaries: self.paint_dirty.boundaries.len(),
                    dirty_paint_order: self.paint_dirty.paint_order.len(),
                    dirty_composite: self.paint_dirty.composite.len(),
                    dirty_placement: self.paint_dirty.placement.len(),
                },
            );
        }
    }

    pub fn mark_repaint_boundary_placement_dirty(&mut self, node: NodeId) {
        let boundary = self
            .parent_of(node)
            .and_then(|parent| self.nearest_repaint_boundary(parent))
            .or_else(|| self.nearest_repaint_boundary(node));

        if let Some(boundary) = boundary {
            if let Some(boundary_node) = self.get_mut(boundary.0) {
                boundary_node.paint_meta.bump_fragment();
            }
            self.paint_dirty.boundaries.insert(boundary);
            self.paint_dirty.placement.insert(boundary);
            self.dirty.paint.insert(boundary.0);
            self.record_paint_boundary_dirty();
        }

        if render_trace::is_debug_enabled() {
            render_trace::debug_stage(
                RenderTraceStage::DirtyPropagation,
                PaintDirtyTraceSummary {
                    node,
                    stable_id: self.get(node).map(|tree_node| tree_node.id.as_ref()),
                    reason: PaintDirtyReason::Placement,
                    boundary: boundary.map(|boundary| boundary.0),
                    dirty_boundaries: self.paint_dirty.boundaries.len(),
                    dirty_paint_order: self.paint_dirty.paint_order.len(),
                    dirty_composite: self.paint_dirty.composite.len(),
                    dirty_placement: self.paint_dirty.placement.len(),
                },
            );
        }
    }

    pub fn mark_composite_dirty(&mut self, node: NodeId) {
        self.paint_dirty.composite.insert(node);
        if render_trace::is_debug_enabled() {
            render_trace::debug_stage(
                RenderTraceStage::DirtyPropagation,
                PaintDirtyTraceSummary {
                    node,
                    stable_id: self.get(node).map(|tree_node| tree_node.id.as_ref()),
                    reason: PaintDirtyReason::Visual,
                    boundary: None,
                    dirty_boundaries: self.paint_dirty.boundaries.len(),
                    dirty_paint_order: self.paint_dirty.paint_order.len(),
                    dirty_composite: self.paint_dirty.composite.len(),
                    dirty_placement: self.paint_dirty.placement.len(),
                },
            );
        }
    }

    pub fn take_paint_dirty(&mut self) -> PaintDirtyQueues {
        std::mem::take(&mut self.paint_dirty)
    }

    pub fn clear_paint_dirty(&mut self) {
        self.paint_dirty = PaintDirtyQueues::default();
    }

    pub(crate) fn paint_fragment_cached(&self, boundary: RepaintBoundaryId) -> bool {
        self.paint_cache.borrow().contains(boundary)
    }

    pub(crate) fn repaint_boundaries_in_subtree(&self, root: NodeId) -> Vec<RepaintBoundaryId> {
        let mut boundaries = Vec::new();
        self.collect_repaint_boundaries(root, &mut boundaries);
        boundaries
    }

    fn collect_repaint_boundaries(&self, node: NodeId, boundaries: &mut Vec<RepaintBoundaryId>) {
        let Some(tree_node) = self.get(node) else {
            return;
        };
        if tree_node.paint_meta.boundary.is_some() {
            boundaries.push(RepaintBoundaryId(node));
        }
        for child in tree_node.children.iter().copied() {
            self.collect_repaint_boundaries(child, boundaries);
        }
    }

    pub(crate) fn rebuild_paint_fragment(
        &self,
        boundary: RepaintBoundaryId,
        cx: super::paint::PaintCx<'_>,
        measure_text: impl FnMut(&str, &crate::paint::TextStyle) -> (f32, f32),
    ) -> Result<PaintFlushStats, PaintBuildError> {
        let fragment = super::paint::build_paint_fragment(self, boundary, cx, measure_text)?;
        let commands = fragment.commands.len();
        self.paint_cache.borrow_mut().insert(fragment);
        self.record_paint_fragment_rebuilt(commands);
        self.record_paint_commands(commands);
        render_trace::trace_node(
            RenderTraceStage::PaintFragment,
            boundary.0,
            self.get(boundary.0).map(|node| node.id.as_ref()),
            PaintFragmentTraceSummary {
                commands_recorded: commands,
            },
        );
        Ok(PaintFlushStats {
            fragments_rebuilt: 1,
            commands_recorded: commands,
            ..PaintFlushStats::default()
        })
    }

    pub(crate) fn compose_retained_display_list(
        &self,
        root: RepaintBoundaryId,
    ) -> (DisplayList, PaintFlushStats) {
        let cache = self.paint_cache.borrow();
        let (list, composition) = compose_fragments(&cache, root);
        self.record_paint_composition_fragments_flattened(composition.fragments_flattened);
        render_trace::debug_stage(
            RenderTraceStage::PaintFlush,
            PaintCompositionTraceSummary {
                root_boundary: root.0,
                display_commands: list.commands.len(),
                display_clips: list.clips.len(),
                fragments_flattened: composition.fragments_flattened,
            },
        );
        (
            list,
            PaintFlushStats {
                fragments_flattened: composition.fragments_flattened,
                ..PaintFlushStats::default()
            },
        )
    }

    pub fn mark_layout_dirty(&mut self, node: NodeId, _reason: LayoutDirtyReason) {
        let reason = _reason;
        let boundary = self.nearest_relayout_boundary(node);
        if let Some(boundary) = boundary {
            self.layout_dirty.boundaries.insert(boundary);
            self.dirty.layout.insert(boundary);
        }
        if render_trace::is_debug_enabled() {
            render_trace::debug_stage(
                RenderTraceStage::DirtyPropagation,
                LayoutDirtyTraceSummary {
                    node,
                    stable_id: self.get(node).map(|tree_node| tree_node.id.as_ref()),
                    reason,
                    boundary,
                    dirty_boundaries: self.layout_dirty.boundaries.len(),
                    dirty_text_nodes: self.layout_dirty.text_nodes.len(),
                },
            );
        }
    }

    pub fn mark_text_layout_dirty(&mut self, node: NodeId) {
        self.layout_dirty.text_nodes.insert(node);
        self.dirty.text_layout.insert(node);
    }

    pub fn mark_dirty(&mut self, node: NodeId, flags: DirtyFlags) {
        if flags.is_empty() {
            return;
        }
        self.dirty.mark(node, flags);
        if flags.contains(DirtyFlags::LAYOUT) {
            self.mark_layout_dirty(node, layout_dirty_reason(flags));
        }
        if flags.contains(DirtyFlags::TEXT_LAYOUT) {
            self.mark_text_layout_dirty(node);
            self.mark_layout_dirty(node, LayoutDirtyReason::TextIntrinsic);
        }
        if flags.contains(DirtyFlags::PAINT) {
            self.mark_paint_dirty(node, paint_dirty_reason(flags));
        }
        if flags.contains(DirtyFlags::PAINT_ORDER) {
            self.mark_paint_dirty(node, PaintDirtyReason::PaintOrder);
            self.paint_order_cache.borrow_mut().clear();
        }
        if flags.contains(DirtyFlags::COMPOSITE) {
            self.mark_composite_dirty(node);
        }
        if flags.contains(DirtyFlags::PAINT_PLACEMENT) {
            self.mark_repaint_boundary_placement_dirty(node);
        }
        if render_trace::is_debug_enabled() {
            render_trace::debug_stage(
                RenderTraceStage::DirtyPropagation,
                DirtyPropagationTraceSummary {
                    node,
                    stable_id: self.get(node).map(|tree_node| tree_node.id.as_ref()),
                    flags: flags.to_string(),
                    structure: self.dirty.structure.len(),
                    layout: self.dirty.layout.len(),
                    text_layout: self.dirty.text_layout.len(),
                    paint: self.dirty.paint.len(),
                    hit: self.dirty.hit.len(),
                    paint_order: self.dirty.paint_order.len(),
                    composite: self.dirty.composite.len(),
                    paint_placement: self.dirty.paint_placement.len(),
                },
            );
        }
    }

    pub fn take_dirty(&mut self) -> DirtyQueues {
        std::mem::take(&mut self.dirty)
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = DirtyQueues::default();
    }

    pub fn take_layout_dirty(&mut self) -> LayoutDirtyQueues {
        std::mem::take(&mut self.layout_dirty)
    }

    pub fn clear_layout_dirty(&mut self) {
        self.layout_dirty = LayoutDirtyQueues::default();
    }

    pub(crate) fn layout_cache_get(&self, input: LayoutInput) -> Option<LayoutOutput> {
        self.layout_cache.get(&input.key())
    }

    pub(crate) fn layout_cache_set(&mut self, input: LayoutInput, output: LayoutOutput) {
        self.layout_cache.set(input.key(), output);
    }

    pub(crate) fn layout_input_for(
        &self,
        node: NodeId,
        constraints: LayoutConstraints,
    ) -> Option<LayoutInput> {
        let tree_node = self.get(node)?;
        Some(LayoutInput {
            node,
            constraints,
            available_content_width: constraints.max_width,
            layout_dependency_revision: tree_node.layout_meta.layout_dependency_revision,
        })
    }

    pub fn frame_stats_snapshot(&self) -> FrameStats {
        let mut stats = self.frame_stats.borrow().clone();
        stats.tree_nodes = self.iter().count();
        stats
    }

    pub fn clear_frame_stats(&self) {
        *self.frame_stats.borrow_mut() = FrameStats::default();
    }

    pub fn debug_snapshot(&self, options: TreeSnapshotOptions) -> TreeSnapshot {
        let max_nodes = match options.level {
            TreeDumpLevel::Normal => options.max_nodes,
            TreeDumpLevel::Full => TreeSnapshotMaxNodes::All,
        };
        let mut issues = self.debug_snapshot_issues();
        let mut visited = HashSet::new();
        let mut ordered = Vec::<(usize, NodeId)>::new();

        if let Some(root) = self.root {
            collect_snapshot_order(self, root, 0, &mut visited, &mut ordered);
        }

        for (node_id, _) in self.iter() {
            if !visited.contains(&node_id) {
                issues.push(TreeSnapshotIssue {
                    message: format!("orphan or unreachable node node={node_id}"),
                });
                ordered.push((0, node_id));
            }
        }

        let live_nodes = self.iter().count();
        let limit = match max_nodes {
            TreeSnapshotMaxNodes::Limit(limit) => Some(limit),
            TreeSnapshotMaxNodes::All => None,
        };
        let included = limit.map_or(ordered.len(), |limit| ordered.len().min(limit));
        let truncated = included < ordered.len();
        let omitted_nodes = ordered.len().saturating_sub(included);
        let nodes = ordered
            .into_iter()
            .take(included)
            .filter_map(|(depth, node_id)| {
                let node = self.get(node_id)?;
                Some(TreeSnapshotNode {
                    node: node_id,
                    depth,
                    stable_id: node.id.as_ref().to_string(),
                    line: self.format_snapshot_node(node_id, depth, options.level),
                })
            })
            .collect::<Vec<_>>();

        let summary = TreeSnapshotSummary {
            level: options.level,
            root: self.root,
            live_nodes,
            indexed_nodes: self.index.len(),
            dirty: DirtyQueueSummary::from(&self.dirty),
            layout_dirty: LayoutDirtyQueueSummary {
                boundaries: self.layout_dirty.boundaries.len(),
                text_nodes: self.layout_dirty.text_nodes.len(),
            },
            paint_dirty: PaintDirtyQueueSummary::from(&self.paint_dirty),
            caches: TreeCacheSummary {
                layout_entries: self.layout_cache.len(),
                hit_order_entries: self.hit_order_cache.borrow().len(),
                paint_order_entries: self.paint_order_cache.borrow().len(),
                paint_fragments: self.paint_cache.borrow().len(),
            },
            issues: issues.len(),
            truncated,
            omitted_nodes,
        };

        TreeSnapshot {
            summary,
            nodes,
            issues,
        }
    }

    fn debug_snapshot_issues(&self) -> Vec<TreeSnapshotIssue> {
        let mut issues = Vec::new();
        match self.root {
            Some(root) if self.get(root).is_none() => issues.push(TreeSnapshotIssue {
                message: format!("root points to missing node root={root}"),
            }),
            None => issues.push(TreeSnapshotIssue {
                message: "missing root".to_string(),
            }),
            _ => {}
        }

        if let Err(error) = self.validate_index() {
            issues.push(TreeSnapshotIssue {
                message: index_error_message(error),
            });
        }

        for (parent_id, node) in self.iter() {
            let mut seen_children = HashSet::new();
            for child in node.children.iter().copied() {
                if !seen_children.insert(child) {
                    issues.push(TreeSnapshotIssue {
                        message: format!("duplicate child parent={parent_id} child={child}"),
                    });
                }
                if self.get(child).is_none() {
                    issues.push(TreeSnapshotIssue {
                        message: format!("missing child parent={parent_id} child={child}"),
                    });
                    continue;
                }
                if self.parents.get(&child).copied() != Some(parent_id) {
                    issues.push(TreeSnapshotIssue {
                        message: format!(
                            "parent map mismatch child={child} expected_parent={parent_id} actual_parent={:?}",
                            self.parents.get(&child).copied()
                        ),
                    });
                }
            }
        }

        for (child, parent) in &self.parents {
            if self.get(*child).is_none() {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map contains missing child child={child} parent={parent}"
                    ),
                });
                continue;
            }
            let Some(parent_node) = self.get(*parent) else {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map contains missing parent child={child} parent={parent}"
                    ),
                });
                continue;
            };
            if !parent_node.children.contains(child) {
                issues.push(TreeSnapshotIssue {
                    message: format!(
                        "parent map child not present in parent children child={child} parent={parent}"
                    ),
                });
            }
        }

        issues
    }

    fn format_snapshot_node(&self, node_id: NodeId, depth: usize, level: TreeDumpLevel) -> String {
        let Some(node) = self.get(node_id) else {
            return format!("{}node={node_id} <missing>", "  ".repeat(depth));
        };
        let indent = "  ".repeat(depth);
        let parent = self.parents.get(&node_id).copied();
        let dirty = self.snapshot_dirty_flags(node_id);
        let paint_boundary = RepaintBoundaryId(node_id);
        let normal = format!(
            "{indent}node={node_id} stable_id={:?} kind={} parent={parent:?} children={:?} rect={:?} visible={} dirty={} layout_dirty_boundary={} layout_dirty_text={} paint_dirty_boundary={} paint_dirty_order={} composite_dirty={} layout_boundary={:?} paint_boundary={:?} layout_rev=(style:{} text:{} children:{} rect:{} dependency:{}) paint_rev=(visual:{} text:{} order:{} fragment:{}) runtime_slots={}",
            node.id.as_ref(),
            normal_kind_summary(&node.kind),
            node.children,
            node.rect,
            node.local_runtime.visible,
            dirty,
            self.layout_dirty.boundaries.contains(&node_id),
            self.layout_dirty.text_nodes.contains(&node_id),
            self.paint_dirty.boundaries.contains(&paint_boundary),
            self.paint_dirty.paint_order.contains(&paint_boundary),
            self.paint_dirty.composite.contains(&node_id),
            node.layout_meta.boundary,
            node.paint_meta.boundary,
            node.layout_meta.style_revision.get(),
            node.layout_meta.text_revision.get(),
            node.layout_meta.children_revision.get(),
            node.layout_meta.explicit_rect_revision.get(),
            node.layout_meta.layout_dependency_revision.get(),
            node.paint_meta.visual_revision.get(),
            node.paint_meta.text_paint_revision.get(),
            node.paint_meta.paint_order_revision.get(),
            node.paint_meta.fragment_revision.get(),
            node.runtime_slots.len(),
        );

        if level == TreeDumpLevel::Normal {
            return normal;
        }

        format!(
            "{normal} props={:?} style={:?} decoration={:?} local_runtime={:?} layout_meta={:?} paint_meta={:?} kind_full={} runtime_slots_full=[{}] cache=(paint_fragment_cached:{}) retained_runtime_present={} paint_dirty_boundaries={:?} paint_dirty_order={:?} composite_dirty_nodes={:?}",
            node.props,
            node.style,
            node.decoration,
            node.local_runtime,
            node.layout_meta,
            node.paint_meta,
            full_kind_summary(&node.kind),
            runtime_slots_full(&node.runtime_slots),
            node.paint_meta
                .boundary
                .is_some_and(|_| self.paint_cache.borrow().contains(paint_boundary)),
            self.retained_runtime.contains(node.id.as_ref()),
            repaint_boundary_list(self.paint_dirty.boundaries.iter().copied()),
            repaint_boundary_list(self.paint_dirty.paint_order.iter().copied()),
            self.paint_dirty.composite,
        )
    }

    fn snapshot_dirty_flags(&self, node: NodeId) -> DirtyFlags {
        let mut flags = DirtyFlags::NONE;
        if self.dirty.structure.contains(&node) {
            flags |= DirtyFlags::STRUCTURE;
        }
        if self.dirty.layout.contains(&node) {
            flags |= DirtyFlags::LAYOUT;
        }
        if self.dirty.text_layout.contains(&node) {
            flags |= DirtyFlags::TEXT_LAYOUT;
        }
        if self.dirty.paint.contains(&node) {
            flags |= DirtyFlags::PAINT;
        }
        if self.dirty.hit.contains(&node) {
            flags |= DirtyFlags::HIT;
        }
        if self.dirty.paint_order.contains(&node) {
            flags |= DirtyFlags::PAINT_ORDER;
        }
        if self.dirty.composite.contains(&node) {
            flags |= DirtyFlags::COMPOSITE;
        }
        flags
    }

    pub(crate) fn record_stable_id_lookup(&self) {
        self.frame_stats.borrow_mut().stable_id_lookups += 1;
    }

    pub(crate) fn record_full_tree_scan(&self) {
        self.frame_stats.borrow_mut().full_tree_scans += 1;
    }

    pub(crate) fn record_layout_node_visited(&self) {
        self.frame_stats.borrow_mut().layout_nodes_visited += 1;
    }

    pub(crate) fn record_layout_boundary_flushed(&self) {
        self.frame_stats.borrow_mut().layout_boundaries_flushed += 1;
    }

    pub(crate) fn record_layout_cache_hit(&self) {
        self.frame_stats.borrow_mut().layout_cache_hits += 1;
    }

    pub(crate) fn record_layout_cache_miss(&self) {
        self.frame_stats.borrow_mut().layout_cache_misses += 1;
    }

    pub(crate) fn record_text_layout_request(&self) {
        self.frame_stats.borrow_mut().text_layout_requests += 1;
    }

    pub(crate) fn record_text_layout_cache_hit(&self) {
        self.frame_stats.borrow_mut().text_layout_cache_hits += 1;
    }

    pub(crate) fn record_hit_order_cache_hit(&self) {
        self.frame_stats.borrow_mut().hit_order_cache_hits += 1;
    }

    pub(crate) fn record_hit_order_cache_miss(&self) {
        self.frame_stats.borrow_mut().hit_order_cache_misses += 1;
    }

    pub(crate) fn record_paint_node_visited(&self) {
        self.frame_stats.borrow_mut().paint_nodes_visited += 1;
    }

    pub(crate) fn record_paint_commands(&self, count: usize) {
        self.frame_stats.borrow_mut().paint_commands += count;
    }

    pub(crate) fn record_paint_fragment_rebuilt(&self, commands: usize) {
        let mut stats = self.frame_stats.borrow_mut();
        stats.paint_fragments_rebuilt += 1;
        stats.paint_fragment_commands_recorded += commands;
    }

    pub(crate) fn record_paint_fragments_reused(&self, count: usize) {
        self.frame_stats.borrow_mut().paint_fragments_reused += count;
    }

    pub(crate) fn record_paint_fragments_evicted(&self, count: usize) {
        self.frame_stats.borrow_mut().paint_fragments_evicted += count;
    }

    pub(crate) fn record_paint_boundary_dirty(&self) {
        self.frame_stats.borrow_mut().paint_boundaries_dirty += 1;
    }

    pub(crate) fn record_paint_composition_fragments_flattened(&self, count: usize) {
        self.frame_stats
            .borrow_mut()
            .paint_composition_fragments_flattened += count;
    }

    pub(crate) fn record_paint_order_cache_hit(&self) {
        self.frame_stats.borrow_mut().paint_order_cache_hits += 1;
    }

    pub(crate) fn record_paint_order_cache_miss(&self) {
        self.frame_stats.borrow_mut().paint_order_cache_misses += 1;
    }

    pub fn ensure_panel(&mut self, config: &PanelConfig) {
        let id = config.id.as_str();
        if self.panel_state(id).is_some() {
            let panel = self.panel_state_mut(id).expect("panel state checked above");
            panel.min_size = config.min_size;
            panel.rect.w = panel.rect.w.max(config.min_size[0]);
            panel.rect.h = panel.rect.h.max(config.min_size[1]);
            return;
        }

        let z_index = {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let z_index = root.next_z;
            root.next_z += 1;
            z_index
        };
        *self.ensure_runtime_slot_by_stable_id::<PanelRuntime>(id) =
            PanelRuntime::from_config(config, z_index);
    }

    pub fn panel_state(&self, id: &str) -> Option<&PanelRuntime> {
        self.runtime_slot_by_stable_id::<PanelRuntime>(id)
    }

    pub fn panel_state_mut(&mut self, id: &str) -> Option<&mut PanelRuntime> {
        self.runtime_slot_by_stable_id_mut::<PanelRuntime>(id)
    }

    pub(crate) fn sync_canvas_node_layouts(
        &mut self,
        identities: &[CanvasNodeIdentity],
    ) -> Vec<CanvasNodeLayout> {
        let owner_ids: HashSet<&str> = identities
            .iter()
            .map(|identity| identity.owner_id.as_str())
            .collect();
        self.retained_runtime.retain(|stable_id, slots| {
            if slots.get::<CanvasNodeRuntime>().is_none() {
                return true;
            }
            let Some(owner_id) = stable_id.strip_prefix("canvas_node::") else {
                return true;
            };
            owner_ids.contains(owner_id)
        });
        if let Some(interaction) =
            self.runtime_slot_by_stable_id_mut::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
        {
            interaction.retain_owner_ids(&owner_ids);
        }

        let mut layouts = Vec::with_capacity(identities.len());
        for (index, identity) in identities.iter().enumerate() {
            let stable_id = canvas_node_stable_id(&identity.owner_id);
            let runtime = self.ensure_runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id);
            if runtime.owner_id.is_empty() {
                *runtime = CanvasNodeRuntime::from_identity(identity, index as i32);
            }
            layouts.push(runtime.to_layout());
        }

        layouts.sort_by(|a, b| {
            a.z_index
                .cmp(&b.z_index)
                .then_with(|| a.owner_id.cmp(&b.owner_id))
        });
        layouts
    }

    pub(crate) fn export_canvas_node_layouts(&self) -> Vec<CanvasNodeLayout> {
        let mut layouts: Vec<CanvasNodeLayout> = self
            .iter()
            .filter_map(|(_, node)| {
                node.runtime_slots
                    .get::<CanvasNodeRuntime>()
                    .map(CanvasNodeRuntime::to_layout)
            })
            .chain(self.retained_runtime.values().filter_map(|slots| {
                slots
                    .get::<CanvasNodeRuntime>()
                    .map(CanvasNodeRuntime::to_layout)
            }))
            .collect();

        layouts.sort_by(|a, b| a.owner_id.cmp(&b.owner_id));
        layouts
    }

    pub(crate) fn import_canvas_node_layouts(&mut self, layouts: &[CanvasNodeLayout]) {
        for layout in layouts {
            let stable_id = canvas_node_stable_id(&layout.owner_id);
            let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
            else {
                continue;
            };
            runtime.rect = layout.rect;
            runtime.z_index = layout.z_index;
            runtime.collapsed = layout.collapsed;
            runtime.user_min_height = layout.user_min_height;
        }
    }

    pub(crate) fn move_canvas_node_by(&mut self, owner_id: &str, dx: f32, dy: f32) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };
        runtime.rect.x += dx;
        runtime.rect.y += dy;
        self.mark_canvas_connection_layer_dirty();
        true
    }

    pub(crate) fn resize_canvas_node_by(
        &mut self,
        owner_id: &str,
        edge: ResizeEdge,
        dx: f32,
        dy: f32,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            tracing::trace!(
                target: "nodeimg::render_trace::node",
                owner_id,
                edge = ?edge,
                dx,
                dy,
                "ignore canvas node resize: runtime missing"
            );
            return false;
        };

        let before = runtime.rect;
        let requested_w = requested_resize_width(before, edge, dx);
        let requested_h = requested_resize_height(before, edge, dy);
        resize_rect_by_edge(
            &mut runtime.rect,
            edge,
            dx,
            dy,
            CANVAS_NODE_MIN_WIDTH,
            CANVAS_NODE_MIN_HEIGHT,
        );
        if is_vertical_resize_edge(edge) && runtime.rect.h != before.h {
            runtime.user_min_height = Some(runtime.rect.h);
        }
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            owner_id,
            edge = ?edge,
            dx,
            dy,
            before_x = before.x,
            before_y = before.y,
            before_w = before.w,
            before_h = before.h,
            after_x = runtime.rect.x,
            after_y = runtime.rect.y,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            requested_w,
            requested_h,
            min_w = CANVAS_NODE_MIN_WIDTH,
            min_h = CANVAS_NODE_MIN_HEIGHT,
            clamped_w = requested_w < CANVAS_NODE_MIN_WIDTH,
            clamped_h = requested_h < CANVAS_NODE_MIN_HEIGHT,
            applied_dx = runtime.rect.x - before.x,
            applied_dy = runtime.rect.y - before.y,
            applied_dw = runtime.rect.w - before.w,
            applied_dh = runtime.rect.h - before.h,
            user_min_height = runtime.user_min_height,
            "resize canvas node runtime rect"
        );
        self.mark_canvas_connection_layer_dirty();
        true
    }

    pub(crate) fn ensure_canvas_node_min_size(
        &mut self,
        owner_id: &str,
        min_width: f32,
        min_height: f32,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };

        let next_width = runtime.rect.w.max(min_width.max(CANVAS_NODE_MIN_WIDTH));
        let next_height = runtime.rect.h.max(min_height.max(CANVAS_NODE_MIN_HEIGHT));
        let changed = next_width != runtime.rect.w || next_height != runtime.rect.h;
        let before = runtime.rect;
        runtime.rect.w = next_width;
        runtime.rect.h = next_height;
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            owner_id,
            requested_min_w = min_width,
            requested_min_h = min_height,
            engine_min_w = CANVAS_NODE_MIN_WIDTH,
            engine_min_h = CANVAS_NODE_MIN_HEIGHT,
            before_w = before.w,
            before_h = before.h,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            changed,
            "ensure canvas node min size"
        );
        if changed {
            self.mark_canvas_connection_layer_dirty();
        }
        changed
    }

    pub(crate) fn apply_canvas_node_sizing(
        &mut self,
        owner_id: &str,
        request: crate::canvas::CanvasNodeSizingRequest,
    ) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        let Some(runtime) = self.runtime_slot_by_stable_id_mut::<CanvasNodeRuntime>(&stable_id)
        else {
            return false;
        };

        let before = runtime.rect;
        runtime.rect.w = request
            .target_width
            .max(request.min_width)
            .max(CANVAS_NODE_MIN_WIDTH);
        runtime.rect.h = request
            .target_height
            .max(request.min_height)
            .max(CANVAS_NODE_MIN_HEIGHT);
        let changed = runtime.rect.w != before.w || runtime.rect.h != before.h;
        tracing::trace!(
            target: "nodeimg::render_trace::node",
            owner_id,
            target_w = request.target_width,
            target_h = request.target_height,
            request_min_w = request.min_width,
            request_min_h = request.min_height,
            engine_min_w = CANVAS_NODE_MIN_WIDTH,
            engine_min_h = CANVAS_NODE_MIN_HEIGHT,
            before_w = before.w,
            before_h = before.h,
            after_w = runtime.rect.w,
            after_h = runtime.rect.h,
            user_min_height = runtime.user_min_height,
            changed,
            "apply canvas node absolute sizing"
        );
        if changed {
            self.mark_canvas_connection_layer_dirty();
        }
        changed
    }

    pub(crate) fn canvas_port_group_view(
        &self,
        owner_id: &str,
        side: CanvasPortSide,
    ) -> CanvasPortGroupView {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .map(|interaction| interaction.port_group_view(owner_id, side))
            .unwrap_or_default()
    }

    pub(crate) fn toggle_canvas_port_group(
        &mut self,
        owner_id: &str,
        side: CanvasPortSide,
    ) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .toggle_port_group(owner_id, side)
    }

    pub(crate) fn select_canvas_node(&mut self, owner_id: &str) -> bool {
        let stable_id = canvas_node_stable_id(owner_id);
        if self
            .runtime_slot_by_stable_id::<CanvasNodeRuntime>(&stable_id)
            .is_none()
        {
            return false;
        }
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .select_single(owner_id);
        true
    }

    pub(crate) fn clear_canvas_selection(&mut self) {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .clear();
    }

    pub(crate) fn is_canvas_node_selected(&self, owner_id: &str) -> bool {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .is_some_and(|interaction| interaction.is_selected(owner_id))
    }

    pub(crate) fn pending_canvas_connection(
        &self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .and_then(|interaction| interaction.pending_connection().cloned())
    }

    pub(crate) fn begin_pending_canvas_connection(
        &mut self,
        from_port_id: &str,
        cursor_canvas: [f32; 2],
    ) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .begin_pending_connection(from_port_id, cursor_canvas)
    }

    pub(crate) fn update_pending_canvas_connection(&mut self, cursor_canvas: [f32; 2]) -> bool {
        let changed = self
            .ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .update_pending_connection(cursor_canvas);
        if changed {
            self.mark_canvas_connection_layer_dirty();
        }
        changed
    }

    pub(crate) fn end_pending_canvas_connection(
        &mut self,
    ) -> Option<crate::canvas::CanvasPendingConnectionView> {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .end_pending_connection()
    }

    pub(crate) fn cancel_pending_canvas_connection(&mut self) -> bool {
        self.ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .cancel_pending_connection()
    }

    pub(crate) fn hovered_canvas_port_id(&self) -> Option<String> {
        self.runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .and_then(|interaction| interaction.hovered_port_id().map(str::to_string))
    }

    fn mark_canvas_connection_layer_dirty(&mut self) {
        if let Some(layer) = self.node_by_str("canvas_connections") {
            self.mark_paint_dirty(layer, PaintDirtyReason::Visual);
        }
    }

    pub(crate) fn set_hovered_canvas_port(&mut self, port_id: Option<&str>) -> bool {
        let changed = self
            .ensure_runtime_slot_by_stable_id::<CanvasInteractionRuntime>(CANVAS_INTERACTION_ID)
            .set_hovered_port(port_id);
        if changed {
            self.mark_canvas_connection_layer_dirty();
        }
        changed
    }

    pub(crate) fn export_panel_layouts(&self) -> Vec<PanelLayout> {
        let mut layouts: Vec<PanelLayout> = self
            .iter()
            .filter_map(|(_, node)| {
                let panel = node.runtime_slots.get::<PanelRuntime>()?;
                Some(panel_layout_from_runtime(node.id.as_ref(), panel))
            })
            .chain(self.retained_runtime.iter().filter_map(|(id, slots)| {
                let panel = slots.get::<PanelRuntime>()?;
                Some(panel_layout_from_runtime(id, panel))
            }))
            .collect();

        layouts.sort_by(|a, b| a.id.cmp(&b.id));
        layouts
    }

    pub(crate) fn import_panel_layouts(&mut self, layouts: &[PanelLayout]) {
        let mut max_imported_z: Option<i32> = None;
        for layout in layouts {
            let Some(panel) = self.panel_state_mut(&layout.id) else {
                continue;
            };
            panel.rect = layout.rect;
            panel.rect.w = panel.rect.w.max(panel.min_size[0]);
            panel.rect.h = panel.rect.h.max(panel.min_size[1]);
            panel.visible = layout.visible;
            panel.z_index = layout.z_index;
            panel.collapsed = layout.collapsed;
            max_imported_z = Some(max_imported_z.map_or(layout.z_index, |z| z.max(layout.z_index)));
        }

        if let Some(max_imported_z) = max_imported_z {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            root.next_z = root.next_z.max(max_imported_z + 1);
        }
    }

    pub fn move_panel_by(&mut self, id: &str, dx: f32, dy: f32) {
        let Some(panel) = self.panel_state_mut(id) else {
            return;
        };
        panel.rect.x += dx;
        panel.rect.y += dy;
    }

    pub fn resize_panel_by(&mut self, id: &str, edge: ResizeEdge, dx: f32, dy: f32) {
        let Some(panel) = self.panel_state_mut(id) else {
            return;
        };

        match edge {
            ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
                panel.rect.x += dx;
                panel.rect.w -= dx;
            }
            ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
                panel.rect.w += dx;
            }
            _ => {}
        }

        match edge {
            ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                panel.rect.y += dy;
                panel.rect.h -= dy;
            }
            ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
                panel.rect.h += dy;
            }
            _ => {}
        }

        panel.rect.w = panel.rect.w.max(panel.min_size[0]);
        panel.rect.h = panel.rect.h.max(panel.min_size[1]);
    }

    pub fn show_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = true;
        }
    }

    pub fn hide_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = false;
        }
    }

    pub fn toggle_panel(&mut self, id: &str) {
        if let Some(panel) = self.panel_state_mut(id) {
            panel.visible = !panel.visible;
        }
    }

    pub fn bring_panel_to_front(&mut self, id: &str) {
        if self.panel_state(id).is_none() {
            return;
        }
        let next_z = {
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let z = root.next_z;
            root.next_z += 1;
            root.focused = Some(id.to_string());
            z
        };
        if let Some(panel) = self.panel_state_mut(id) {
            panel.z_index = next_z;
        }
    }

    pub fn start_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        self.bring_panel_to_front(id);
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_drag = Some(PanelPointerSession {
            id: id.to_string(),
            last_x: x,
            last_y: y,
        });
    }

    pub fn move_panel_drag(&mut self, id: &str, x: f32, y: f32) {
        let Some((dx, dy)) = ({
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let Some(session) = root.active_drag.as_mut() else {
                return;
            };
            if session.id != id {
                return;
            }
            let dx = x - session.last_x;
            let dy = y - session.last_y;
            session.last_x = x;
            session.last_y = y;
            Some((dx, dy))
        }) else {
            return;
        };
        self.move_panel_by(id, dx, dy);
    }

    pub fn end_panel_drag(&mut self) {
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_drag = None;
    }

    pub fn start_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        self.bring_panel_to_front(id);
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_resize = Some(PanelResizeSession {
            id: id.to_string(),
            edge,
            last_x: x,
            last_y: y,
        });
    }

    pub fn move_panel_resize(&mut self, id: &str, edge: ResizeEdge, x: f32, y: f32) {
        let Some((dx, dy)) = ({
            let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
            let Some(session) = root.active_resize.as_mut() else {
                return;
            };
            if session.id != id || session.edge != edge {
                return;
            }
            let dx = x - session.last_x;
            let dy = y - session.last_y;
            session.last_x = x;
            session.last_y = y;
            Some((dx, dy))
        }) else {
            return;
        };
        self.resize_panel_by(id, edge, dx, dy);
    }

    pub fn end_panel_resize(&mut self) {
        let root = self.ensure_runtime_slot_by_stable_id::<PanelRootRuntime>(PANEL_ROOT_ID);
        root.active_resize = None;
    }
}

fn resize_rect_by_edge(
    rect: &mut Rect,
    edge: ResizeEdge,
    dx: f32,
    dy: f32,
    min_width: f32,
    min_height: f32,
) {
    let right = rect.x + rect.w;
    let bottom = rect.y + rect.h;

    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => {
            let next_x = (rect.x + dx).min(right - min_width);
            rect.x = next_x;
            rect.w = right - next_x;
        }
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => {
            rect.w = (rect.w + dx).max(min_width);
        }
        _ => {}
    }

    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => {
            let next_y = (rect.y + dy).min(bottom - min_height);
            rect.y = next_y;
            rect.h = bottom - next_y;
        }
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => {
            rect.h = (rect.h + dy).max(min_height);
        }
        _ => {}
    }

    rect.w = rect.w.max(min_width);
    rect.h = rect.h.max(min_height);
}

fn requested_resize_width(rect: Rect, edge: ResizeEdge, dx: f32) -> f32 {
    match edge {
        ResizeEdge::Left | ResizeEdge::TopLeft | ResizeEdge::BottomLeft => rect.w - dx,
        ResizeEdge::Right | ResizeEdge::TopRight | ResizeEdge::BottomRight => rect.w + dx,
        ResizeEdge::Top | ResizeEdge::Bottom => rect.w,
    }
}

fn requested_resize_height(rect: Rect, edge: ResizeEdge, dy: f32) -> f32 {
    match edge {
        ResizeEdge::Top | ResizeEdge::TopLeft | ResizeEdge::TopRight => rect.h - dy,
        ResizeEdge::Bottom | ResizeEdge::BottomLeft | ResizeEdge::BottomRight => rect.h + dy,
        ResizeEdge::Left | ResizeEdge::Right => rect.h,
    }
}

fn is_vertical_resize_edge(edge: ResizeEdge) -> bool {
    matches!(
        edge,
        ResizeEdge::Top
            | ResizeEdge::TopLeft
            | ResizeEdge::TopRight
            | ResizeEdge::Bottom
            | ResizeEdge::BottomLeft
            | ResizeEdge::BottomRight
    )
}

fn collect_snapshot_order(
    tree: &Tree,
    node: NodeId,
    depth: usize,
    visited: &mut HashSet<NodeId>,
    ordered: &mut Vec<(usize, NodeId)>,
) {
    if !visited.insert(node) {
        return;
    }
    ordered.push((depth, node));
    let Some(tree_node) = tree.get(node) else {
        return;
    };
    for child in tree_node.children.iter().copied() {
        collect_snapshot_order(tree, child, depth + 1, visited, ordered);
    }
}

fn normal_kind_summary(kind: &super::NodeKind) -> String {
    match kind {
        super::NodeKind::Container => "Container".to_string(),
        super::NodeKind::Widget(widget) => format!("Widget({})", widget.widget_type()),
        super::NodeKind::Leaf(super::layout::LeafKind::Text { content, .. }) => {
            format!(
                "Leaf::Text(bytes:{} chars:{})",
                content.len(),
                content.chars().count()
            )
        }
        super::NodeKind::Leaf(leaf) => format!("Leaf::{leaf:?}"),
    }
}

fn full_kind_summary(kind: &super::NodeKind) -> String {
    match kind {
        super::NodeKind::Container => "Container".to_string(),
        super::NodeKind::Widget(widget) => {
            format!("Widget(type:{} props:{widget:?})", widget.widget_type())
        }
        super::NodeKind::Leaf(super::layout::LeafKind::Text {
            content,
            style,
            layout,
        }) => {
            format!("Leaf::Text {{ content: {content:?}, style: {style:?}, layout: {layout:?} }}")
        }
        super::NodeKind::Leaf(leaf) => format!("Leaf::{leaf:?}"),
    }
}

fn runtime_slots_full(slots: &RuntimeSlots) -> String {
    slots
        .debug_entries()
        .into_iter()
        .map(|entry| {
            format!(
                "{} policy={:?} value={}",
                entry.type_name, entry.policy, entry.value
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn layout_dirty_reason(flags: DirtyFlags) -> LayoutDirtyReason {
    if flags.contains(DirtyFlags::STRUCTURE) {
        LayoutDirtyReason::Structure
    } else if flags.contains(DirtyFlags::STYLE) {
        LayoutDirtyReason::Style
    } else if flags.contains(DirtyFlags::TEXT_LAYOUT) {
        LayoutDirtyReason::TextIntrinsic
    } else if flags.contains(DirtyFlags::LAYOUT) {
        LayoutDirtyReason::Size
    } else {
        LayoutDirtyReason::Explicit
    }
}

fn paint_dirty_reason(flags: DirtyFlags) -> PaintDirtyReason {
    if flags.contains(DirtyFlags::STRUCTURE) {
        PaintDirtyReason::Structure
    } else if flags.contains(DirtyFlags::TEXT_LAYOUT) {
        PaintDirtyReason::Text
    } else if flags.contains(DirtyFlags::PAINT_ORDER) {
        PaintDirtyReason::PaintOrder
    } else if flags.contains(DirtyFlags::STYLE) {
        PaintDirtyReason::Visual
    } else {
        PaintDirtyReason::Visual
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

fn panel_layout_from_runtime(id: &str, panel: &PanelRuntime) -> PanelLayout {
    PanelLayout {
        id: id.to_string(),
        rect: panel.rect,
        visible: panel.visible,
        z_index: panel.z_index,
        collapsed: panel.collapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::Rect;
    use crate::tree::layout::{
        BoxStyle, LayoutConstraints, LayoutOutput, LeafKind, RelayoutBoundaryReason, Size,
        TextLayout,
    };
    use crate::tree::{
        DirtyFlags, DirtyQueues, NodeKind, NodeLocalRuntime, NodeProps, StylePatch, TreeDumpLevel,
        TreeMutation, TreeSnapshotOptions,
    };

    #[derive(Debug, Default)]
    struct TestRuntime {
        value: usize,
    }

    impl RuntimeSlot for TestRuntime {}

    fn container_node(id: &'static str) -> TreeNode {
        TreeNode {
            id: StableId::from(id),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Container,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn text_node(id: &'static str, content: &'static str) -> TreeNode {
        TreeNode {
            id: StableId::from(id),
            props: NodeProps::default(),
            style: BoxStyle::default(),
            decoration: None,
            kind: NodeKind::Leaf(LeafKind::Text {
                content: content.to_string(),
                style: crate::renderer::TextStyle::new(crate::renderer::Color::BLACK, 12.0),
                layout: TextLayout::default(),
            }),
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            children: Vec::new(),
            local_runtime: NodeLocalRuntime::default(),
            layout_meta: Default::default(),
            paint_meta: Default::default(),
            mutation_meta: Default::default(),
            runtime_slots: RuntimeSlots::default(),
        }
    }

    fn layout_output(rect: Rect) -> LayoutOutput {
        LayoutOutput {
            rect,
            content_rect: rect,
            intrinsic_width: rect.w,
            intrinsic_height: rect.h,
            baseline: None,
        }
    }

    #[test]
    fn tree_dump_normal_uses_compact_fields() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        let text = tree.insert(text_node("text", "secret text"));
        tree.set_root(root);
        tree.set_children(root, vec![text]);

        let snapshot = tree.debug_snapshot(TreeSnapshotOptions::normal(500));

        assert_eq!(snapshot.summary.level, TreeDumpLevel::Normal);
        assert!(snapshot
            .nodes
            .iter()
            .any(|node| node.line.contains("Leaf::Text(bytes:11")));
        assert!(!snapshot
            .nodes
            .iter()
            .any(|node| node.line.contains("secret text")));
    }

    #[test]
    fn tree_dump_full_prints_all_node_fields_and_raw_text() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        let text = tree.insert(text_node("text", "secret text"));
        tree.set_root(root);
        tree.set_children(root, vec![text]);

        let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
        let text_line = snapshot
            .nodes
            .iter()
            .find(|node| node.stable_id == "text")
            .expect("text node")
            .line
            .as_str();

        assert!(text_line.contains("props="));
        assert!(text_line.contains("style="));
        assert!(text_line.contains("layout_meta="));
        assert!(text_line.contains("paint_meta="));
        assert!(text_line.contains("secret text"));
    }

    #[test]
    fn tree_dump_full_prints_runtime_slot_debug_values() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        tree.set_root(root);
        tree.ensure_runtime_slot::<TestRuntime>(root)
            .expect("runtime slot")
            .value = 7;

        let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
        let root_line = &snapshot.nodes[0].line;

        assert!(root_line.contains("TestRuntime"));
        assert!(root_line.contains("value: 7"));
    }

    #[test]
    fn tree_dump_detects_parent_mismatch_duplicate_children_and_stale_index() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        let child = tree.insert(container_node("child"));
        tree.set_root(root);
        tree.get_mut(root).expect("root").children = vec![child, child];

        let snapshot = tree.debug_snapshot(TreeSnapshotOptions::full());
        let issues = snapshot
            .issues
            .iter()
            .map(|issue| issue.message.as_str())
            .collect::<Vec<_>>();

        assert!(issues.iter().any(|issue| issue.contains("duplicate child")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("parent map mismatch")));
    }

    #[test]
    fn tree_dump_respects_max_nodes_in_normal_mode_but_not_full_mode() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        let first = tree.insert(container_node("first"));
        let second = tree.insert(container_node("second"));
        tree.set_root(root);
        tree.set_children(root, vec![first, second]);

        let normal = tree.debug_snapshot(TreeSnapshotOptions::normal(1));
        let full = tree.debug_snapshot(TreeSnapshotOptions {
            level: TreeDumpLevel::Full,
            max_nodes: crate::tree::TreeSnapshotMaxNodes::Limit(1),
        });

        assert!(normal.summary.truncated);
        assert_eq!(normal.nodes.len(), 1);
        assert!(!full.summary.truncated);
        assert_eq!(full.nodes.len(), 3);
    }

    #[test]
    fn tree_dump_does_not_mutate_frame_stats() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        tree.set_root(root);
        let before = tree.frame_stats_snapshot();

        let _ = tree.debug_snapshot(TreeSnapshotOptions::full());
        let after = tree.frame_stats_snapshot();

        assert_eq!(before, after);
    }

    #[test]
    fn runtime_slot_roundtrips_by_type() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        tree.set_root(root);

        tree.ensure_runtime_slot::<TestRuntime>(root)
            .expect("slot")
            .value = 42;

        assert_eq!(
            tree.runtime_slot::<TestRuntime>(root)
                .expect("stored runtime")
                .value,
            42
        );

        let removed = tree
            .remove_runtime_slot::<TestRuntime>(root)
            .expect("removed runtime");
        assert_eq!(removed.value, 42);
        assert!(tree.runtime_slot::<TestRuntime>(root).is_none());
    }

    #[test]
    fn stable_id_lookup_uses_tree_node_identity() {
        let mut tree = Tree::new();
        let root = tree.insert(container_node("root"));
        tree.set_root(root);

        assert_eq!(tree.node_by_stable_id(&StableId::from("root")), tree.root());
        assert_eq!(tree.node_by_str("root"), tree.root());
    }

    #[test]
    fn tree_index_registers_and_removes_live_nodes() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("insert");
        tree.set_root(root);

        assert_eq!(tree.node_by_str("root"), Some(root));
        assert!(tree.contains_stable_id("root"));
        tree.validate_index().expect("valid index");

        tree.remove(root);

        assert_eq!(tree.node_by_str("root"), None);
        tree.validate_index().expect("valid after remove");
    }

    #[test]
    fn tree_index_rejects_duplicate_stable_ids() {
        let mut tree = Tree::new();
        tree.insert_checked(container_node("duplicate"))
            .expect("first insert");

        let err = tree
            .insert_checked(container_node("duplicate"))
            .expect_err("duplicate should fail checked insertion");

        assert!(matches!(
            err,
            TreeIndexError::DuplicateStableId {
                stable_id,
                existing: _,
                duplicate: _
            } if stable_id.as_ref() == "duplicate"
        ));
    }

    #[test]
    fn tree_index_clears_before_node_id_reuse() {
        let mut tree = Tree::new();
        let first = tree.insert_checked(container_node("first")).expect("first");
        tree.remove(first);

        let second = tree
            .insert_checked(container_node("second"))
            .expect("second");

        assert_eq!(first, second);
        assert_eq!(tree.node_by_str("first"), None);
        assert_eq!(tree.node_by_str("second"), Some(second));
        tree.validate_index().expect("valid reused index");
    }

    #[test]
    fn runtime_slot_lookup_uses_tree_index() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        tree.ensure_runtime_slot::<TestRuntime>(root)
            .expect("slot")
            .value = 9;
        tree.clear_frame_stats();

        let value = tree
            .runtime_slot_by_stable_id::<TestRuntime>("root")
            .expect("runtime")
            .value;
        let stats = tree.frame_stats_snapshot();

        assert_eq!(value, 9);
        assert_eq!(stats.stable_id_lookups, 1);
        assert_eq!(stats.full_tree_scans, 0);
    }

    #[test]
    fn dirty_take_clears_queues() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);

        tree.mark_dirty(root, DirtyFlags::LAYOUT | DirtyFlags::PAINT);
        let dirty = tree.take_dirty();

        assert!(dirty.layout.contains(&root));
        assert!(dirty.paint.contains(&root));
        assert!(tree.take_dirty().layout.is_empty());
        assert!(tree.take_dirty().paint.is_empty());
    }

    #[test]
    fn layout_dirty_bubbles_to_nearest_relayout_boundary() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let mut boundary_node = container_node("boundary");
        boundary_node
            .layout_meta
            .set_boundary(crate::tree::layout::RelayoutBoundaryReason::Panel);
        let boundary = tree.insert_checked(boundary_node).expect("boundary");
        let child = tree.insert_checked(container_node("child")).expect("child");
        tree.append_child(root, boundary);
        tree.append_child(boundary, child);

        tree.mark_layout_dirty(child, crate::tree::layout::LayoutDirtyReason::Size);
        let dirty = tree.take_layout_dirty();

        assert!(dirty.boundaries.contains(&boundary));
        assert!(!dirty.boundaries.contains(&root));
    }

    #[test]
    fn set_rect_move_invalidates_cached_boundary_layout_input() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let mut panel_node = container_node("panel");
        panel_node
            .layout_meta
            .set_boundary(RelayoutBoundaryReason::Panel);
        panel_node.mutation_meta.rect_move =
            crate::tree::RectMoveInvalidation::LayoutAndBoundaryPlacement;
        let panel = tree.insert_checked(panel_node).expect("panel");
        let child = tree.insert_checked(container_node("child")).expect("child");
        tree.append_child(root, panel);
        tree.append_child(panel, child);
        let constraints = LayoutConstraints::from_available(Rect {
            x: 0.0,
            y: 0.0,
            w: 180.0,
            h: 120.0,
        });
        let input = tree
            .layout_input_for(panel, constraints)
            .expect("layout input");
        tree.layout_cache_set(
            input,
            layout_output(Rect {
                x: 0.0,
                y: 0.0,
                w: 180.0,
                h: 120.0,
            }),
        );
        let before_dependency = tree
            .get(panel)
            .expect("panel")
            .layout_meta
            .layout_dependency_revision;

        tree.apply_mutation(
            &crate::template::TemplateRegistry::new(),
            TreeMutation::SetRect {
                node: panel,
                rect: Rect {
                    x: 24.0,
                    y: 18.0,
                    w: 0.0,
                    h: 0.0,
                },
            },
        )
        .expect("move panel");

        let after_dependency = tree
            .get(panel)
            .expect("panel")
            .layout_meta
            .layout_dependency_revision;
        let moved_input = tree
            .layout_input_for(panel, constraints)
            .expect("moved layout input");
        assert!(after_dependency > before_dependency);
        assert!(tree.layout_cache_get(input).is_none());
        assert!(tree.layout_cache_get(moved_input).is_none());
    }

    #[test]
    fn child_style_change_invalidates_cached_ancestor_boundary_layout_input() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let mut boundary_node = container_node("boundary");
        boundary_node
            .layout_meta
            .set_boundary(RelayoutBoundaryReason::Panel);
        let boundary = tree.insert_checked(boundary_node).expect("boundary");
        let child = tree.insert_checked(container_node("child")).expect("child");
        tree.append_child(root, boundary);
        tree.append_child(boundary, child);
        let constraints = LayoutConstraints::from_available(Rect {
            x: 0.0,
            y: 0.0,
            w: 240.0,
            h: 160.0,
        });
        let input = tree
            .layout_input_for(boundary, constraints)
            .expect("layout input");
        tree.layout_cache_set(
            input,
            layout_output(Rect {
                x: 0.0,
                y: 0.0,
                w: 240.0,
                h: 160.0,
            }),
        );
        let before_dependency = tree
            .get(boundary)
            .expect("boundary")
            .layout_meta
            .layout_dependency_revision;

        tree.apply_mutation(
            &crate::template::TemplateRegistry::new(),
            TreeMutation::SetStyle {
                node: child,
                patch: StylePatch {
                    width: Some(Size::Fixed(80.0)),
                    ..StylePatch::default()
                },
            },
        )
        .expect("style child");

        let after_dependency = tree
            .get(boundary)
            .expect("boundary")
            .layout_meta
            .layout_dependency_revision;
        assert!(after_dependency > before_dependency);
        assert!(tree.layout_cache_get(input).is_none());
    }

    #[test]
    fn text_layout_dirty_records_text_node_and_boundary() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let text = tree
            .insert_checked(text_node("text", "value"))
            .expect("text");
        tree.append_child(root, text);

        tree.mark_dirty(text, DirtyFlags::TEXT_LAYOUT);
        let dirty = tree.take_layout_dirty();

        assert!(dirty.text_nodes.contains(&text));
        assert!(dirty.boundaries.contains(&root));
    }

    #[test]
    fn paint_dirty_bubbles_to_nearest_repaint_boundary() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let boundary = tree
            .insert_checked(container_node("node_card"))
            .expect("boundary");
        let child = tree.insert_checked(container_node("child")).expect("child");
        tree.set_repaint_boundary(boundary, RepaintBoundaryReason::CanvasNodeCard);
        tree.append_child(root, boundary);
        tree.append_child(boundary, child);

        tree.mark_dirty(child, DirtyFlags::PAINT);
        let dirty = tree.take_paint_dirty();

        assert!(dirty.boundaries.contains(&RepaintBoundaryId(boundary)));
        assert!(!dirty.boundaries.contains(&RepaintBoundaryId(root)));
    }

    #[test]
    fn composite_dirty_does_not_mark_repaint_boundary() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);

        tree.mark_dirty(root, DirtyFlags::COMPOSITE | DirtyFlags::HIT);
        let dirty = tree.take_paint_dirty();

        assert!(dirty.boundaries.is_empty());
        assert!(dirty.composite.contains(&root));
    }

    #[test]
    fn tree_mutation_set_text_marks_text_layout_and_paint() {
        let mut tree = Tree::new();
        let text = tree.insert_checked(text_node("text", "old")).expect("text");
        tree.set_root(text);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetText {
                    node: text,
                    value: "new".to_string(),
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::TEXT_LAYOUT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
        let dirty = tree.take_dirty();
        assert!(dirty.text_layout.contains(&text));
        assert!(dirty.paint.contains(&text));
    }

    #[test]
    fn tree_mutation_noop_set_text_does_not_dirty_or_bump_revision() {
        let mut tree = Tree::new();
        let text = tree.insert_checked(text_node("text", "old")).expect("text");
        tree.set_root(text);
        let registry = crate::template::TemplateRegistry::new();
        let before_layout = tree.get(text).expect("text").layout_meta.text_revision;
        let before_paint = tree.get(text).expect("text").paint_meta.text_paint_revision;

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetText {
                    node: text,
                    value: "old".to_string(),
                },
            )
            .expect("mutation");

        assert_eq!(invalidation.flags, DirtyFlags::NONE);
        assert_eq!(
            tree.get(text).expect("text").layout_meta.text_revision,
            before_layout
        );
        assert_eq!(
            tree.get(text).expect("text").paint_meta.text_paint_revision,
            before_paint
        );
        assert_eq!(tree.take_dirty(), DirtyQueues::default());
    }

    #[test]
    fn tree_mutation_set_rect_position_marks_layout_by_default() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: root,
                    rect: Rect {
                        x: 10.0,
                        y: 10.0,
                        w: 0.0,
                        h: 0.0,
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
        assert!(tree.take_dirty().layout.contains(&root));
    }

    #[test]
    fn tree_mutation_set_rect_grid_move_marks_paint_not_layout() {
        let mut tree = Tree::new();
        let mut root_node = container_node("grid");
        root_node.mutation_meta.rect_move = crate::tree::RectMoveInvalidation::Repaint;
        let root = tree.insert_checked(root_node).expect("grid");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: root,
                    rect: Rect {
                        x: 10.0,
                        y: 10.0,
                        w: 0.0,
                        h: 0.0,
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(!invalidation.flags.contains(DirtyFlags::LAYOUT));
        assert!(tree.take_dirty().paint.contains(&root));
    }

    #[test]
    fn tree_mutation_set_rect_boundary_move_marks_placement_not_layout() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let mut child_node = container_node("child");
        child_node
            .paint_meta
            .set_boundary(RepaintBoundaryReason::CanvasNodeCard);
        child_node.mutation_meta.rect_move = crate::tree::RectMoveInvalidation::BoundaryPlacement;
        let child = tree.insert_checked(child_node).expect("child");
        tree.append_child(root, child);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: child,
                    rect: Rect {
                        x: 10.0,
                        y: 10.0,
                        w: 0.0,
                        h: 0.0,
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(!invalidation.flags.contains(DirtyFlags::LAYOUT));
        let dirty = tree.take_paint_dirty();
        assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
    }

    #[test]
    fn tree_mutation_set_rect_layout_boundary_move_marks_layout_and_placement() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let mut panel_node = container_node("panel");
        panel_node
            .layout_meta
            .set_boundary(RelayoutBoundaryReason::Panel);
        panel_node
            .paint_meta
            .set_boundary(RepaintBoundaryReason::PanelFrame);
        panel_node.mutation_meta.rect_move =
            crate::tree::RectMoveInvalidation::LayoutAndBoundaryPlacement;
        let panel = tree.insert_checked(panel_node).expect("panel");
        tree.append_child(root, panel);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: panel,
                    rect: Rect {
                        x: 24.0,
                        y: 18.0,
                        w: 0.0,
                        h: 0.0,
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT_PLACEMENT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(tree.take_layout_dirty().boundaries.contains(&panel));
        let dirty = tree.take_paint_dirty();
        assert!(dirty.boundaries.contains(&RepaintBoundaryId(panel)));
        assert!(dirty.placement.contains(&RepaintBoundaryId(root)));
    }

    #[test]
    fn tree_mutation_set_rect_size_marks_layout_hit_paint() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: root,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 100.0,
                        h: 80.0,
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::LAYOUT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    }

    #[test]
    fn tree_mutation_noop_set_rect_does_not_dirty_or_bump_revision() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();
        let before = tree
            .get(root)
            .expect("root")
            .layout_meta
            .explicit_rect_revision;

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetRect {
                    node: root,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.0,
                        h: 0.0,
                    },
                },
            )
            .expect("mutation");

        assert_eq!(invalidation.flags, DirtyFlags::NONE);
        assert_eq!(
            tree.get(root)
                .expect("root")
                .layout_meta
                .explicit_rect_revision,
            before
        );
        assert_eq!(tree.take_dirty(), DirtyQueues::default());
    }

    #[test]
    fn tree_mutation_set_rect_survives_layout_flush() {
        let mut tree = Tree::new();
        let mut root_node = container_node("root");
        root_node.style.width = Size::Fixed(400.0);
        root_node.style.height = Size::Fixed(300.0);
        let root = tree.insert_checked(root_node).expect("root");
        tree.set_root(root);
        let child = tree.insert_checked(container_node("child")).expect("child");
        tree.append_child(root, child);
        let registry = crate::template::TemplateRegistry::new();
        let explicit = Rect {
            x: 32.0,
            y: 48.0,
            w: 120.0,
            h: 64.0,
        };

        tree.apply_mutation(
            &registry,
            TreeMutation::SetRect {
                node: child,
                rect: explicit,
            },
        )
        .expect("set rect");
        crate::tree::layout(
            &mut tree,
            root,
            Rect {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
            &mut |_text, _style| (0.0, 0.0),
        );

        assert_eq!(tree.get(child).expect("child").rect, explicit);
    }

    #[test]
    fn tree_mutation_set_z_index_marks_paint_order_hit_paint() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetZIndex {
                    node: root,
                    z_index: 8,
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::PAINT_ORDER));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
    }

    #[test]
    fn tree_mutation_noop_set_z_index_does_not_dirty_or_bump_revision() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();
        let before_layout = tree.get(root).expect("root").layout_meta.style_revision;
        let before_order = tree
            .get(root)
            .expect("root")
            .paint_meta
            .paint_order_revision;

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetZIndex {
                    node: root,
                    z_index: 0,
                },
            )
            .expect("mutation");

        assert_eq!(invalidation.flags, DirtyFlags::NONE);
        assert_eq!(
            tree.get(root).expect("root").layout_meta.style_revision,
            before_layout
        );
        assert_eq!(
            tree.get(root)
                .expect("root")
                .paint_meta
                .paint_order_revision,
            before_order
        );
        assert_eq!(tree.take_dirty(), DirtyQueues::default());
    }

    #[test]
    fn tree_mutation_empty_style_patch_is_noop() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetStyle {
                    node: root,
                    patch: StylePatch::default(),
                },
            )
            .expect("mutation");

        assert_eq!(invalidation.flags, DirtyFlags::NONE);
        assert_eq!(tree.take_dirty(), DirtyQueues::default());
    }

    #[test]
    fn tree_mutation_set_transform_marks_paint_not_composite() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::new();

        let invalidation = tree
            .apply_mutation(
                &registry,
                TreeMutation::SetStyle {
                    node: root,
                    patch: StylePatch {
                        transform: Some(Some(crate::geometry::TransformSpec::translate_scale(
                            [10.0, 20.0],
                            2.0,
                        ))),
                        ..StylePatch::default()
                    },
                },
            )
            .expect("mutation");

        assert!(invalidation.flags.contains(DirtyFlags::PAINT));
        assert!(invalidation.flags.contains(DirtyFlags::HIT));
        assert!(!invalidation.flags.contains(DirtyFlags::COMPOSITE));
    }

    #[test]
    fn tree_mutation_mount_unmount_marks_structure_layout_hit_paint() {
        let mut tree = Tree::new();
        let root = tree.insert_checked(container_node("root")).expect("root");
        tree.set_root(root);
        let registry = crate::template::TemplateRegistry::with_builtin_templates();

        let mount = tree
            .apply_mutation(
                &registry,
                TreeMutation::MountTemplate {
                    parent: root,
                    template: crate::template::TemplateId::from(crate::template::TEXT_BOX_TEMPLATE),
                    instance: crate::template::InstanceId::from("textbox"),
                    payload: crate::template::TemplatePayload::from(
                        crate::template::SlotValues::default(),
                    ),
                },
            )
            .expect("mount");

        assert!(mount.flags.contains(DirtyFlags::STRUCTURE));
        assert!(mount.flags.contains(DirtyFlags::LAYOUT));
        assert!(mount.flags.contains(DirtyFlags::HIT));
        assert!(mount.flags.contains(DirtyFlags::PAINT));
        let mounted = tree.node_by_str("textbox").expect("mounted root");

        let unmount = tree
            .apply_mutation(&registry, TreeMutation::Unmount { node: mounted })
            .expect("unmount");

        assert!(unmount.flags.contains(DirtyFlags::STRUCTURE));
        assert!(unmount.flags.contains(DirtyFlags::LAYOUT));
        assert!(unmount.flags.contains(DirtyFlags::HIT));
        assert!(unmount.flags.contains(DirtyFlags::PAINT));
        assert!(tree.node_by_str("textbox").is_none());
    }

    #[test]
    fn canvas_node_layout_sync_uses_owner_identity() {
        let mut tree = Tree::new();
        let first = crate::canvas::CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: crate::renderer::Rect {
                x: 10.0,
                y: 20.0,
                w: 220.0,
                h: 96.0,
            },
        };

        let layouts = tree.sync_canvas_node_layouts(std::slice::from_ref(&first));
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].owner_id, "engine_node::1");
        assert_eq!(layouts[0].rect.x, 10.0);

        tree.import_canvas_node_layouts(&[crate::canvas::CanvasNodeLayout {
            owner_id: "engine_node::1".to_string(),
            rect: crate::renderer::Rect {
                x: 80.0,
                y: 90.0,
                w: 260.0,
                h: 120.0,
            },
            z_index: 7,
            collapsed: true,
            user_min_height: Some(120.0),
        }]);

        let layouts = tree.sync_canvas_node_layouts(&[first]);
        assert_eq!(layouts[0].rect.x, 80.0);
        assert_eq!(layouts[0].rect.y, 90.0);
        assert_eq!(layouts[0].z_index, 7);
        assert!(layouts[0].collapsed);
        assert_eq!(layouts[0].user_min_height, Some(120.0));

        assert!(tree.move_canvas_node_by("engine_node::1", 10.0, -5.0));
        let layouts = tree.export_canvas_node_layouts();
        assert_eq!(layouts[0].rect.x, 90.0);
        assert_eq!(layouts[0].rect.y, 85.0);

        assert!(tree.resize_canvas_node_by("engine_node::1", ResizeEdge::Right, 80.0, 20.0));
        let layouts = tree.export_canvas_node_layouts();
        assert_eq!(layouts[0].rect.w, 340.0);
        assert_eq!(layouts[0].rect.h, 132.0);

        let stale = tree.sync_canvas_node_layouts(&[]);
        assert!(stale.is_empty());
        assert!(tree.export_canvas_node_layouts().is_empty());
    }

    #[test]
    fn canvas_port_group_toggle_is_tree_runtime_state() {
        let mut tree = Tree::new();

        assert!(
            !tree
                .canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(
            tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
        assert!(
            tree.canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(
            !tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
    }

    #[test]
    fn canvas_interaction_state_tracks_selection_and_prunes_stale_nodes() {
        let mut tree = Tree::new();
        let first = CanvasNodeIdentity {
            owner_id: "engine_node::1".to_string(),
            default_rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 100.0,
                h: 80.0,
            },
        };
        let second = CanvasNodeIdentity {
            owner_id: "engine_node::2".to_string(),
            default_rect: Rect {
                x: 120.0,
                y: 0.0,
                w: 100.0,
                h: 80.0,
            },
        };

        tree.sync_canvas_node_layouts(&[first.clone(), second.clone()]);
        assert!(tree.select_canvas_node("engine_node::1"));
        assert!(tree.is_canvas_node_selected("engine_node::1"));
        assert!(!tree.is_canvas_node_selected("engine_node::2"));
        assert!(
            tree.toggle_canvas_port_group("engine_node::1", crate::canvas::CanvasPortSide::Input)
        );
        assert!(
            tree.canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(!tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::input::prompt",
            [2.0, 3.0],
        ));
        assert!(tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::output::image",
            [2.0, 3.0],
        ));
        assert_eq!(
            tree.pending_canvas_connection()
                .map(|pending| pending.cursor_canvas),
            Some([2.0, 3.0])
        );
        assert!(tree.update_pending_canvas_connection([4.0, 5.0]));
        assert!(
            tree.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::prompt"))
        );
        assert_eq!(
            tree.hovered_canvas_port_id(),
            Some("canvas_node::engine_node::1::port::input::prompt".to_string())
        );
        assert!(tree.cancel_pending_canvas_connection());
        assert!(tree.pending_canvas_connection().is_none());
        assert!(tree.hovered_canvas_port_id().is_none());
        assert!(!tree.cancel_pending_canvas_connection());
        assert!(tree.begin_pending_canvas_connection(
            "canvas_node::engine_node::1::port::output::image",
            [2.0, 3.0],
        ));
        assert!(
            tree.set_hovered_canvas_port(Some("canvas_node::engine_node::1::port::input::prompt"))
        );

        tree.sync_canvas_node_layouts(&[second]);

        assert!(!tree.is_canvas_node_selected("engine_node::1"));
        assert!(
            !tree
                .canvas_port_group_view("engine_node::1", crate::canvas::CanvasPortSide::Input)
                .open
        );
        assert!(tree.pending_canvas_connection().is_none());
        assert!(tree.hovered_canvas_port_id().is_none());
    }
}
