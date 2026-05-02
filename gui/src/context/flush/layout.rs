use super::super::Context;
use crate::diagnostics::render_trace::{self, RectSummary, RenderTraceStage};
use crate::diagnostics::tree_dump::TreeDumpPhase;
use crate::renderer::{Rect, TextMeasurer};
use crate::tree::layout::{LayoutConstraints, LayoutFlushStats, LayoutOutput};
use crate::tree::{NodeId, Tree};

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct LayoutFlushTraceSummary {
    root_rect: RectSummary,
    dirty_boundaries: usize,
    dirty_text_nodes: usize,
    boundary_candidates: usize,
    stats: LayoutFlushStats,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct LayoutBoundaryTraceSummary {
    available: Option<RectSummary>,
    cache_hit: bool,
    skipped: &'static str,
    nodes_visited: usize,
}

impl Context {
    pub(crate) fn flush_layout_dirty(
        &mut self,
        root_rect: Rect,
        measurer: &mut TextMeasurer,
    ) -> LayoutFlushStats {
        self.maybe_dump_tree(
            RenderTraceStage::LayoutFlush,
            TreeDumpPhase::Before,
            "before layout dirty flush",
        );
        let dirty = self.tree.take_layout_dirty();
        let dirty_boundaries = dirty.boundaries.len();
        let dirty_text_nodes = dirty.text_nodes.len();
        let mut boundaries = dirty.boundaries;
        for text_node in dirty.text_nodes {
            if let Some(boundary) = self.tree.nearest_relayout_boundary(text_node) {
                boundaries.insert(boundary);
            }
        }

        let mut stats = LayoutFlushStats::default();
        let boundary_candidates = boundaries.len();
        for boundary in boundaries {
            let Some(available) = boundary_available_rect(&self.tree, boundary, root_rect) else {
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: None,
                        cache_hit: false,
                        skipped: "missing available rect",
                        nodes_visited: 0,
                    },
                );
                continue;
            };
            let constraints = LayoutConstraints::from_available(available);
            let Some(input) = self.tree.layout_input_for(boundary, constraints) else {
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: Some(RectSummary::from(available)),
                        cache_hit: false,
                        skipped: "missing layout input",
                        nodes_visited: 0,
                    },
                );
                continue;
            };
            if self.tree.layout_cache_get(input).is_some() {
                self.tree.record_layout_cache_hit();
                stats.boundaries_skipped_cache_hit += 1;
                render_trace::trace_node(
                    RenderTraceStage::LayoutFlush,
                    boundary,
                    self.tree.get(boundary).map(|node| node.id.as_ref()),
                    LayoutBoundaryTraceSummary {
                        available: Some(RectSummary::from(available)),
                        cache_hit: true,
                        skipped: "cache hit",
                        nodes_visited: 0,
                    },
                );
                continue;
            }

            self.tree.record_layout_cache_miss();
            let visited_before = self.tree.frame_stats_snapshot().layout_nodes_visited;
            crate::tree::layout(&mut self.tree, boundary, available, &mut |text, style| {
                measurer.measure_with_style(text, style)
            });
            let visited_after = self.tree.frame_stats_snapshot().layout_nodes_visited;
            self.tree.record_layout_boundary_flushed();
            stats.boundaries_flushed += 1;
            let nodes_visited = visited_after.saturating_sub(visited_before);
            stats.nodes_visited += nodes_visited;
            if let Some(output) = layout_output_from_tree(&self.tree, boundary) {
                self.tree.layout_cache_set(input, output);
            }
            render_trace::trace_node(
                RenderTraceStage::LayoutFlush,
                boundary,
                self.tree.get(boundary).map(|node| node.id.as_ref()),
                LayoutBoundaryTraceSummary {
                    available: Some(RectSummary::from(available)),
                    cache_hit: false,
                    skipped: "",
                    nodes_visited,
                },
            );
        }
        render_trace::debug_stage(
            RenderTraceStage::LayoutFlush,
            LayoutFlushTraceSummary {
                root_rect: RectSummary::from(root_rect),
                dirty_boundaries,
                dirty_text_nodes,
                boundary_candidates,
                stats: stats.clone(),
            },
        );
        self.maybe_dump_tree(
            RenderTraceStage::LayoutFlush,
            TreeDumpPhase::After,
            "after layout dirty flush",
        );
        stats
    }
}

fn boundary_available_rect(tree: &Tree, boundary: NodeId, root_rect: Rect) -> Option<Rect> {
    if tree.root() == Some(boundary) {
        return Some(root_rect);
    }
    tree.get(boundary).map(|node| {
        if node.rect.w > 0.0 || node.rect.h > 0.0 {
            node.rect
        } else {
            root_rect
        }
    })
}

fn layout_output_from_tree(tree: &Tree, node: NodeId) -> Option<LayoutOutput> {
    let tree_node = tree.get(node)?;
    let padding = tree_node.style.padding;
    let content_rect = Rect {
        x: tree_node.rect.x + padding.left,
        y: tree_node.rect.y + padding.top,
        w: (tree_node.rect.w - padding.horizontal()).max(0.0),
        h: (tree_node.rect.h - padding.vertical()).max(0.0),
    };
    Some(LayoutOutput {
        rect: tree_node.rect,
        content_rect,
        intrinsic_width: tree_node.rect.w,
        intrinsic_height: tree_node.rect.h,
        baseline: None,
    })
}
