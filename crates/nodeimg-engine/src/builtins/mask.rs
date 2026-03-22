use nodeimg_gpu::pipeline::dispatch_compute;
use nodeimg_types::gpu_texture::GpuTexture;
use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use crate::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "mask".into(),
        title: "Mask".into(),
        category: CategoryId::new("composite"),
        inputs: vec![
            PinDef {
                name: "image".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            },
            PinDef {
                name: "mask".into(),
                data_type: DataTypeId::new("mask"),
                required: true,
            },
        ],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![ParamDef {
            name: "invert".into(),
            data_type: DataTypeId::new("boolean"),
            constraint: Constraint::None,
            default: Value::Boolean(false),
            widget_override: None,
        }],
        has_preview: false,
        process: None,
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn gpu_process(
    gpu: &GpuContext,
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let gpu_image = match inputs.get("image") {
        Some(Value::GpuImage(tex)) => Arc::clone(tex),
        Some(Value::Image(img)) => {
            Arc::new(GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img))
        }
        _ => return outputs,
    };
    let gpu_mask = match inputs.get("mask") {
        Some(Value::GpuImage(tex)) => Arc::clone(tex),
        Some(Value::Image(img)) | Some(Value::Mask(img)) => {
            Arc::new(GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img))
        }
        _ => return outputs,
    };

    let invert_mask = match params.get("invert") {
        Some(Value::Boolean(b)) => {
            if *b {
                1.0_f32
            } else {
                0.0_f32
            }
        }
        _ => 0.0_f32,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_image.width, gpu_image.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        invert_mask: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        invert_mask,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let params_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mask_params"),
            contents: bytemuck::bytes_of(&p),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let pipeline = gpu.pipeline("mask", nodeimg_gpu::shaders::MASK);
    let layout = pipeline.get_bind_group_layout(0);
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mask_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&gpu_image.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&gpu_mask.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&output_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
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
