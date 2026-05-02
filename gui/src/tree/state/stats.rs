use super::Tree;
use crate::tree::frame_stats::FrameStats;

impl Tree {
    pub fn frame_stats_snapshot(&self) -> FrameStats {
        let mut stats = self.frame_stats.borrow().clone();
        stats.tree_nodes = self.iter().count();
        stats
    }

    pub fn clear_frame_stats(&self) {
        *self.frame_stats.borrow_mut() = FrameStats::default();
    }
    pub(crate) fn record_stable_id_lookup(&self) {
        self.frame_stats.borrow_mut().stable_id_lookups += 1;
    }

    pub(crate) fn record_parent_lookup_fallback_scan(&self) {
        self.frame_stats.borrow_mut().parent_lookup_fallback_scans += 1;
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
}
