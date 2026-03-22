pub mod texture;

// Re-export everything from nodeimg-gpu so existing `crate::gpu::GpuContext` paths still work.
pub use nodeimg_gpu::*;

use std::sync::Arc;

/// Create a GpuContext from eframe's render state.
/// Returns None if wgpu render state is not available.
pub fn gpu_context_from_eframe(cc: &eframe::CreationContext<'_>) -> Option<Arc<GpuContext>> {
    let render_state = cc.wgpu_render_state.as_ref()?;
    Some(GpuContext::new(
        render_state.device.clone(),
        render_state.queue.clone(),
    ))
}
