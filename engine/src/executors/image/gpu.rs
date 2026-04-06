use std::collections::HashMap;
use std::sync::Mutex;
use types::texture::GpuTexture;
use wgpu::util::DeviceExt;

pub struct GpuExecutor {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pipelines: Mutex<HashMap<String, wgpu::ComputePipeline>>,
}

impl GpuExecutor {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            pipelines: Mutex::new(HashMap::new()),
        }
    }

    /// Get or create a compute pipeline from shader source. Cached by name.
    pub fn pipeline(&self, name: &str, shader_src: &str) -> wgpu::ComputePipeline {
        let mut cache = self.pipelines.lock().unwrap();
        if let Some(p) = cache.get(name) {
            return p.clone();
        }
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(name),
                source: wgpu::ShaderSource::Wgsl(shader_src.into()),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(name),
                layout: None,
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
        cache.insert(name.to_string(), pipeline.clone());
        pipeline
    }

    /// Create a bind group for input texture + output texture + params buffer.
    /// Common pattern for compute shaders: bind group 0 has
    /// [input_texture, output_texture, params_uniform].
    pub fn create_io_params_bind_group(
        &self,
        pipeline: &wgpu::ComputePipeline,
        input: &GpuTexture,
        output: &GpuTexture,
        params: &[u8],
    ) -> wgpu::BindGroup {
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("params"),
            contents: params,
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let layout = pipeline.get_bind_group_layout(0);
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
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

    /// Dispatch a compute shader with 16x16 workgroup size.
    pub fn dispatch_compute(
        &self,
        pipeline: &wgpu::ComputePipeline,
        bind_group: &wgpu::BindGroup,
        width: u32,
        height: u32,
    ) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(width.div_ceil(16), height.div_ceil(16), 1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
