use wgpu::util::DeviceExt;

use nodeimg_types::gpu_texture::GpuTexture;

/// Create a compute pipeline from WGSL shader source.
pub fn create_compute_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader_source: &str,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None, // use auto layout
        module: &shader_module,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
}

/// Dispatch a compute shader assuming 16x16 workgroup size.
pub fn dispatch_compute(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    width: u32,
    height: u32,
) {
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        // Ceil-divide to cover all pixels with 16x16 workgroups
        let wg_x = width.div_ceil(16);
        let wg_y = height.div_ceil(16);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }
    queue.submit(std::iter::once(encoder.finish()));
}

/// Create a bind group with input and output textures (group 0, bindings 0 & 1).
pub fn create_io_bind_group(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    input: &GpuTexture,
    output: &GpuTexture,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("io_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&input.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&output.view),
            },
        ],
    })
}

/// Create a bind group with input texture, output texture, and a uniform params buffer
/// (group 0, bindings 0, 1, 2).
pub fn create_io_params_bind_group<T: bytemuck::Pod>(
    device: &wgpu::Device,
    pipeline: &wgpu::ComputePipeline,
    input: &GpuTexture,
    output: &GpuTexture,
    params: &T,
) -> wgpu::BindGroup {
    let layout = pipeline.get_bind_group_layout(0);
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params_uniform"),
        contents: bytemuck::bytes_of(params),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("io_params_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&input.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&output.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    })
}
