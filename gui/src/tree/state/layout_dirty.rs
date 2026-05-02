use super::Tree;
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::tree::layout::{
    LayoutConstraints, LayoutDependencyKind, LayoutDependencyScope, LayoutDirtyQueues,
    LayoutDirtyReason, LayoutInput, LayoutOutput,
};
use crate::tree::node::NodeId;

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

impl Tree {
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
}
