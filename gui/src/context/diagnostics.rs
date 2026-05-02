use super::Context;
use crate::diagnostics::render_trace::RenderTraceStage;
use crate::diagnostics::tree_dump::{self, TreeDumpPhase, TreeDumpPoint};
use crate::tree::TreeSnapshotOptions;

impl Context {
    pub fn maybe_dump_tree(&mut self, stage: RenderTraceStage, phase: TreeDumpPhase, reason: &str) {
        self.tree_dump
            .maybe_dump(&self.tree, TreeDumpPoint::new(stage, phase), reason);
    }

    pub fn dump_tree_now(
        &self,
        stage: RenderTraceStage,
        phase: TreeDumpPhase,
        reason: &str,
        options: TreeSnapshotOptions,
    ) {
        let snapshot = self.tree.debug_snapshot(options);
        tree_dump::emit_snapshot(&snapshot, TreeDumpPoint::new(stage, phase), reason);
    }
}
