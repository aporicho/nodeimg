#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct RendererBeginFrameTraceSummary {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) scale_factor: f64,
    pub(super) render_scale: f32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct RendererDrawTraceSummary {
    pub(super) display_commands: usize,
    pub(super) display_clips: usize,
    pub(super) backend_commands_total: usize,
    pub(super) unsupported: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct RendererEndFrameTraceSummary {
    pub(super) backend_commands: usize,
    pub(super) internal_w: u32,
    pub(super) internal_h: u32,
}
