use super::super::Context;
use crate::diagnostics::render_trace::{self, RenderTraceStage, TARGET_RENDER};
use crate::diagnostics::tree_dump::TreeDumpPhase;
use crate::paint::PaintFlushStats;
use crate::renderer::{RegistryDisplayResources, Renderer, TextMeasurer};
use crate::theme::Theme;
use crate::tree::{NodeId, PaintDirtyReason, RepaintBoundaryId};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct PaintDirtyTraceSummary {
    root: NodeId,
    root_boundary: NodeId,
    dirty_boundaries: usize,
    dirty_paint_order: usize,
    dirty_composite: usize,
    dirty_placement: usize,
    rebuild_boundaries: usize,
    retained_display_list_present: bool,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct PaintComposeTraceSummary {
    fragments_flattened: usize,
    display_commands: usize,
    clips: usize,
}

impl Context {
    pub(crate) fn render(
        &mut self,
        renderer: &mut Renderer,
        _viewport_w: f32,
        _viewport_h: f32,
        theme: &Theme,
    ) {
        if self.tree.root().is_some() {
            self.current_theme = theme.clone();
            self.sync_overlay_tree();
            let flush = self.flush_paint_dirty(theme, renderer.text_measurer());
            render_trace::debug_stage(RenderTraceStage::PaintFlush, flush);
            let Some(list) = self.retained_display_list.as_ref() else {
                tracing::warn!(
                    target: TARGET_RENDER,
                    frame_id = render_trace::current_render_trace_frame().id,
                    "retained paint did not produce a display list"
                );
                return;
            };
            let resources = RegistryDisplayResources::new(self.resources.textures(), &self.icons);
            let report = renderer.draw_display_list(&list, &resources);
            if !report.unsupported.is_empty() {
                tracing::debug!(
                    target: TARGET_RENDER,
                    frame_id = render_trace::current_render_trace_frame().id,
                    unsupported = report.unsupported.len(),
                    "display list renderer skipped unsupported commands"
                );
            }
        }
    }

    pub fn flush_paint_dirty(
        &mut self,
        theme: &Theme,
        measurer: &mut TextMeasurer,
    ) -> PaintFlushStats {
        self.current_theme = theme.clone();
        self.sync_overlay_tree();
        let Some(root) = self.tree.root() else {
            self.retained_display_list = None;
            return PaintFlushStats::default();
        };

        if self.retained_display_list.is_none()
            || self.last_paint_theme_revision != Some(theme.revision)
        {
            self.tree.mark_paint_dirty(root, PaintDirtyReason::Theme);
            self.last_paint_theme_revision = Some(theme.revision);
        }

        self.maybe_dump_tree(
            RenderTraceStage::PaintFlush,
            TreeDumpPhase::Before,
            "before paint dirty flush",
        );

        let root_boundary = self
            .tree
            .nearest_repaint_boundary(root)
            .unwrap_or(RepaintBoundaryId(root));
        let dirty = self.tree.take_paint_dirty();
        let dirty_boundaries = dirty.boundaries.len();
        let dirty_paint_order = dirty.paint_order.len();
        let dirty_composite = dirty.composite.len();
        let dirty_placement = dirty.placement.len();
        let mut rebuild = dirty.boundaries;
        rebuild.extend(dirty.paint_order);
        rebuild.extend(dirty.placement);

        if self.retained_display_list.is_none() {
            rebuild.extend(self.tree.repaint_boundaries_in_subtree(root));
        } else {
            let dirty_snapshot = rebuild.iter().copied().collect::<Vec<_>>();
            for boundary in dirty_snapshot {
                for descendant in self.tree.repaint_boundaries_in_subtree(boundary.0) {
                    if !self.tree.paint_fragment_cached(descendant) {
                        rebuild.insert(descendant);
                    }
                }
            }
        }

        let mut stats = PaintFlushStats {
            boundaries_dirty: rebuild.len(),
            ..PaintFlushStats::default()
        };
        render_trace::debug_stage(
            RenderTraceStage::PaintFlush,
            PaintDirtyTraceSummary {
                root,
                root_boundary: root_boundary.0,
                dirty_boundaries,
                dirty_paint_order,
                dirty_composite,
                dirty_placement,
                rebuild_boundaries: rebuild.len(),
                retained_display_list_present: self.retained_display_list.is_some(),
            },
        );

        for boundary in rebuild.iter().copied() {
            let rebuilt = match self.tree.rebuild_paint_fragment(
                boundary,
                crate::tree::PaintCx {
                    interaction: Some(&self.interaction),
                    text_boxes: Some(self.systems.text_box_store()),
                    animations: Some(&self.animations),
                    theme,
                },
                |text, style| measurer.measure_with_style(text, style),
            ) {
                Ok(stats) => stats,
                Err(err) => {
                    tracing::warn!(
                        target: TARGET_RENDER,
                        frame_id = render_trace::current_render_trace_frame().id,
                        ?boundary,
                        ?err,
                        "failed to rebuild paint fragment"
                    );
                    continue;
                }
            };
            stats.fragments_rebuilt += rebuilt.fragments_rebuilt;
            stats.commands_recorded += rebuilt.commands_recorded;
            render_trace::trace_node(
                RenderTraceStage::PaintFragment,
                boundary.0,
                self.tree.get(boundary.0).map(|node| node.id.as_ref()),
                rebuilt,
            );
        }

        if stats.fragments_rebuilt == 0 && self.retained_display_list.is_some() {
            self.tree.record_paint_fragments_reused(1);
            stats.fragments_reused = 1;
            return stats;
        }

        let (display_list, compose_stats) = self.tree.compose_retained_display_list(root_boundary);
        stats.fragments_flattened += compose_stats.fragments_flattened;
        render_trace::debug_stage(
            RenderTraceStage::PaintFlush,
            PaintComposeTraceSummary {
                fragments_flattened: compose_stats.fragments_flattened,
                display_commands: display_list.commands.len(),
                clips: display_list.clips.len(),
            },
        );
        self.retained_display_list = Some(display_list);
        stats
    }
}
