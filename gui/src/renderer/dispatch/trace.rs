#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct DispatchPrepareTraceSummary {
    pub(super) backend_commands: usize,
    pub(super) logical_w: f32,
    pub(super) logical_h: f32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct DispatchPlanTraceSummary {
    pub(super) ops: usize,
    pub(super) text_requests: usize,
    pub(super) render_steps: usize,
    pub(super) quad_vertices: usize,
    pub(super) circle_vertices: usize,
    pub(super) grid_vertices: usize,
    pub(super) vector_vertices: usize,
    pub(super) stencil_vertices: usize,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct RenderStepTraceSummary {
    pub(super) step_index: usize,
    pub(super) total_steps: usize,
    pub(super) kind: &'static str,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(super) struct DispatchSubmitTraceSummary {
    pub(super) backend_commands: usize,
    pub(super) render_passes: usize,
    pub(super) text_batches: usize,
    pub(super) upload_bytes: usize,
    pub(super) upload_buffer_grows: usize,
}
