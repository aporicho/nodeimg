pub(crate) fn try_test_device(label: &'static str) -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = match pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: true,
    })) {
        Ok(adapter) => adapter,
        Err(err) => {
            eprintln!("skipping renderer GPU test: failed to create test adapter: {err:?}");
            return None;
        }
    };

    match pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    })) {
        Ok(device) => Some(device),
        Err(err) => {
            eprintln!("skipping renderer GPU test: failed to create test device: {err:?}");
            None
        }
    }
}
