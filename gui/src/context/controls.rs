use super::Context;
use crate::control::ControlIntrinsic;
use crate::diagnostics::render_trace::{self, RenderTraceStage};
use crate::renderer::TextMeasurer;
use crate::theme::Theme;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct TextRuntimeTraceSummary {
    views: usize,
    dirty_intrinsics_before: usize,
    dirty_intrinsics_after: usize,
}

impl Context {
    pub(crate) fn control_intrinsics(&self) -> Vec<ControlIntrinsic> {
        self.systems.control_intrinsics()
    }

    pub(crate) fn take_dirty_control_intrinsics(&mut self) -> Vec<ControlIntrinsic> {
        self.systems.take_dirty_control_intrinsics()
    }

    pub(crate) fn sync_canvas_text_boxes(
        &mut self,
        views: &[crate::canvas::node_template::CanvasNodeRenderView],
        measurer: &mut TextMeasurer,
        theme: &Theme,
    ) {
        let before_dirty = self.systems.dirty_control_intrinsic_ids().len();
        self.systems.sync_canvas_text_boxes(
            &self.tree,
            views,
            measurer,
            theme,
            self.interaction.focused(),
        );
        render_trace::debug_stage(
            RenderTraceStage::TextRuntimeSync,
            TextRuntimeTraceSummary {
                views: views.len(),
                dirty_intrinsics_before: before_dirty,
                dirty_intrinsics_after: self.systems.dirty_control_intrinsic_ids().len(),
            },
        );
    }

    pub(crate) fn has_dirty_control_intrinsics(&self) -> bool {
        self.systems.has_dirty_control_intrinsics()
    }
}
