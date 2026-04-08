pub fn init_gpu(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'_>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(surface),
        force_fallback_adapter: false,
    }))
    .expect("failed to find GPU adapter");

    let info = adapter.get_info();
    log::info!("GPU: {} ({:?})", info.name, info.backend);

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("nodeimg"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        },
    ))
    .expect("failed to create GPU device");

    (adapter, device, queue)
}
