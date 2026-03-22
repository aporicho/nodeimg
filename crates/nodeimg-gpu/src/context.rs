use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::pipeline::create_compute_pipeline;

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
    /// Create a new GpuContext from a wgpu device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Arc<Self> {
        Arc::new(Self {
            device,
            queue,
            pipelines: Mutex::new(HashMap::new()),
        })
    }

    /// Get or create a cached compute pipeline from WGSL shader source.
    pub fn pipeline(&self, name: &str, shader_source: &str) -> Arc<wgpu::ComputePipeline> {
        let mut cache = self.pipelines.lock().unwrap();
        cache
            .entry(name.to_string())
            .or_insert_with(|| {
                Arc::new(create_compute_pipeline(
                    &self.device,
                    name,
                    shader_source,
                    "main",
                ))
            })
            .clone()
    }
}
