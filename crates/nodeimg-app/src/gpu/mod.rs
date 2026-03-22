pub mod pipeline;
pub mod texture;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// GPU context holding wgpu device and queue, shared across the app.
///
/// In wgpu 27+, `Device` and `Queue` are internally reference-counted and `Clone`,
/// so wrapping them in `Arc` is unnecessary. The `GpuContext` itself is wrapped in
/// `Arc` for sharing across the application.
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipelines: Mutex<HashMap<String, Arc<wgpu::ComputePipeline>>>,
}

impl GpuContext {
    /// Create GpuContext from eframe's render state.
    /// Returns None if wgpu render state is not available.
    pub fn from_eframe(cc: &eframe::CreationContext<'_>) -> Option<Arc<Self>> {
        let render_state = cc.wgpu_render_state.as_ref()?;
        Some(Arc::new(Self {
            device: render_state.device.clone(),
            queue: render_state.queue.clone(),
            pipelines: Mutex::new(HashMap::new()),
        }))
    }

    /// Get or create a cached compute pipeline from WGSL shader source.
    pub fn pipeline(&self, name: &str, shader_source: &str) -> Arc<wgpu::ComputePipeline> {
        let mut cache = self.pipelines.lock().unwrap();
        cache
            .entry(name.to_string())
            .or_insert_with(|| {
                Arc::new(crate::gpu::pipeline::create_compute_pipeline(
                    &self.device,
                    name,
                    shader_source,
                    "main",
                ))
            })
            .clone()
    }
}
