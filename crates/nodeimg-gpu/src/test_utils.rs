use std::sync::Arc;

use crate::GpuContext;

/// Try to create a headless GPU context for testing.
///
/// Returns `None` if no GPU adapter is available (e.g. CI without GPU).
/// Tests should early-return when this returns `None`.
pub fn try_create_headless_context() -> Option<Arc<GpuContext>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("test_device"),
        required_limits: wgpu::Limits {
            max_storage_textures_per_shader_stage: 8,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }))
    .ok()?;
    Some(GpuContext::new(device, queue))
}
