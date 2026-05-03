use crate::diagnostics::render_trace::{self, TARGET_RENDER};

use super::api::{
    DisplayCommandKindCounts, DisplayRenderReport, UnsupportedDisplayCommand,
    UnsupportedDisplayReason,
};

impl DisplayRenderReport {
    pub(super) fn record(&mut self, index: usize, reason: UnsupportedDisplayReason) {
        tracing::warn!(
            target: TARGET_RENDER,
            frame_id = render_trace::current_render_trace_frame().id,
            index,
            ?reason,
            "DisplayList command is unsupported by the renderer display backend"
        );
        self.unsupported
            .push(UnsupportedDisplayCommand { index, reason });
    }
}

impl DisplayCommandKindCounts {
    pub(super) fn add(&mut self, other: Self) {
        self.rect += other.rect;
        self.path += other.path;
        self.circle += other.circle;
        self.grid += other.grid;
        self.image += other.image;
        self.text += other.text;
        self.shadow += other.shadow;
        self.svg += other.svg;
        self.svg_raster += other.svg_raster;
        self.layer += other.layer;
    }
}
