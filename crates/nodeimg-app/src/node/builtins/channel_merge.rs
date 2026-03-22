use crate::gpu::pipeline::dispatch_compute;
use crate::gpu::texture::GpuTexture;
use crate::gpu::GpuContext;
use crate::node::category::CategoryId;
use crate::node::registry::{NodeDef, NodeRegistry, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "channel_merge".into(),
        title: "Channel Merge".into(),
        category: CategoryId::new("composite"),
        inputs: vec![
            PinDef {
                name: "red".into(),
                data_type: DataTypeId::new("mask"),
                required: false,
            },
            PinDef {
                name: "green".into(),
                data_type: DataTypeId::new("mask"),
                required: false,
            },
            PinDef {
                name: "blue".into(),
                data_type: DataTypeId::new("mask"),
                required: false,
            },
            PinDef {
                name: "alpha".into(),
                data_type: DataTypeId::new("mask"),
                required: false,
            },
        ],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn get_image<'a>(
    inputs: &'a HashMap<String, Value>,
    key: &str,
) -> Option<&'a Arc<image::DynamicImage>> {
    match inputs.get(key) {
        Some(Value::Image(img)) => Some(img),
        Some(Value::Mask(img)) => Some(img),
        _ => None,
    }
}

fn process(
    inputs: &HashMap<String, Value>,
    _params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let red = get_image(inputs, "red");
    let green = get_image(inputs, "green");
    let blue = get_image(inputs, "blue");
    let alpha = get_image(inputs, "alpha");

    let red_ref = red.map(|a| a.as_ref());
    let green_ref = green.map(|a| a.as_ref());
    let blue_ref = blue.map(|a| a.as_ref());
    let alpha_ref = alpha.map(|a| a.as_ref());

    if let Some(result) =
        crate::processing::composite::channel_merge(red_ref, green_ref, blue_ref, alpha_ref)
    {
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }

    outputs
}

fn get_gpu_texture(
    gpu: &GpuContext,
    inputs: &HashMap<String, Value>,
    key: &str,
) -> Option<Arc<GpuTexture>> {
    match inputs.get(key) {
        Some(Value::GpuImage(tex)) => Some(Arc::clone(tex)),
        Some(Value::Image(img)) | Some(Value::Mask(img)) => Some(Arc::new(
            GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img),
        )),
        _ => None,
    }
}

fn gpu_process(
    gpu: &GpuContext,
    inputs: &HashMap<String, Value>,
    _params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let red = get_gpu_texture(gpu, inputs, "red");
    let green = get_gpu_texture(gpu, inputs, "green");
    let blue = get_gpu_texture(gpu, inputs, "blue");
    let alpha = get_gpu_texture(gpu, inputs, "alpha");

    // Need at least one channel to determine dimensions
    let first = red
        .as_ref()
        .or(green.as_ref())
        .or(blue.as_ref())
        .or(alpha.as_ref());
    let first = match first {
        Some(tex) => tex,
        None => return outputs,
    };

    let w = first.width;
    let h = first.height;

    // Build channel mask bitmask
    let mut channel_mask: i32 = 0;
    if red.is_some() {
        channel_mask |= 1;
    }
    if green.is_some() {
        channel_mask |= 2;
    }
    if blue.is_some() {
        channel_mask |= 4;
    }
    if alpha.is_some() {
        channel_mask |= 8;
    }

    // Use dummy 1x1 textures for missing channels
    let dummy = Arc::new(GpuTexture::create_empty(&gpu.device, 1, 1));
    let red_tex = red.unwrap_or_else(|| Arc::clone(&dummy));
    let green_tex = green.unwrap_or_else(|| Arc::clone(&dummy));
    let blue_tex = blue.unwrap_or_else(|| Arc::clone(&dummy));
    let alpha_tex = alpha.unwrap_or_else(|| Arc::clone(&dummy));

    let output_tex = GpuTexture::create_empty(&gpu.device, w, h);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        channel_mask: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        channel_mask: channel_mask as f32,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let params_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("channel_merge_params"),
            contents: bytemuck::bytes_of(&p),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let pipeline = gpu.pipeline(
        "channel_merge",
        include_str!("../../gpu/shaders/channel_merge.wgsl"),
    );
    let layout = pipeline.get_bind_group_layout(0);
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("channel_merge_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&red_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&green_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&blue_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&alpha_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&output_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params_buffer.as_entire_binding(),
            },
        ],
    });

    dispatch_compute(
        &gpu.device,
        &gpu.queue,
        &pipeline,
        &bind_group,
        output_tex.width,
        output_tex.height,
    );

    outputs.insert("image".into(), Value::GpuImage(Arc::new(output_tex)));
    outputs
}
