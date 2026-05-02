use super::Tree;
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::paint::{compose_fragments, DisplayList, PaintBuildError, PaintFlushStats};
use crate::tree::node::NodeId;
use crate::tree::repaint::{
    PaintDirtyQueues, PaintDirtyReason, RepaintBoundaryId, RepaintBoundaryReason,
};

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

impl Tree {
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
        cx: crate::tree::paint::PaintCx<'_>,
        measure_text: impl FnMut(&str, &crate::paint::TextStyle) -> (f32, f32),
    ) -> Result<PaintFlushStats, PaintBuildError> {
        let fragment = crate::tree::paint::build_paint_fragment(self, boundary, cx, measure_text)?;
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
}
