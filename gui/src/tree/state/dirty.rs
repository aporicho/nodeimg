use super::Tree;
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::tree::dirty::{DirtyFlags, DirtyQueues};
use crate::tree::layout::LayoutDirtyReason;
use crate::tree::node::NodeId;
use crate::tree::repaint::PaintDirtyReason;

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

impl Tree {
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
