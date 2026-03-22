use crate::gpu::pipeline::dispatch_compute;
use crate::gpu::texture::GpuTexture;
use crate::gpu::GpuContext;
use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "blend".into(),
        title: "Blend".into(),
        category: CategoryId::new("composite"),
        inputs: vec![
            PinDef {
                name: "base".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            },
            PinDef {
                name: "layer".into(),
                data_type: DataTypeId::new("image"),
                required: true,
            },
        ],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![
            ParamDef {
                name: "mode".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("normal".into(), "normal".into()),
                        ("multiply".into(), "multiply".into()),
                        ("screen".into(), "screen".into()),
                        ("overlay".into(), "overlay".into()),
                        ("add".into(), "add".into()),
                        ("subtract".into(), "subtract".into()),
                        ("difference".into(), "difference".into()),
                    ],
                },
                default: Value::String("normal".into()),
                widget_override: None,
            },
            ParamDef {
                name: "opacity".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(1.0),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn process(
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let base = match inputs.get("base") {
        Some(Value::Image(img)) => img,
        _ => return outputs,
    };
    let layer = match inputs.get("layer") {
        Some(Value::Image(img)) => img,
        _ => return outputs,
    };

    let mode = match params.get("mode") {
        Some(Value::String(s)) => s.as_str(),
        _ => "normal",
    };
    let opacity = match params.get("opacity") {
        Some(Value::Float(v)) => *v,
        _ => 1.0,
    };

    let result = crate::processing::composite::blend(base, layer, mode, opacity);
    outputs.insert("image".into(), Value::Image(Arc::new(result)));
    outputs
}

fn gpu_process(
    gpu: &GpuContext,
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let gpu_base = match inputs.get("base") {
        Some(Value::GpuImage(tex)) => Arc::clone(tex),
        Some(Value::Image(img)) => {
            Arc::new(GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img))
        }
        _ => return outputs,
    };
    let gpu_layer = match inputs.get("layer") {
        Some(Value::GpuImage(tex)) => Arc::clone(tex),
        Some(Value::Image(img)) => {
            Arc::new(GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img))
        }
        _ => return outputs,
    };

    let mode_str = match params.get("mode") {
        Some(Value::String(s)) => s.as_str(),
        _ => "normal",
    };
    let mode_val: f32 = match mode_str {
        "normal" => 0.0,
        "multiply" => 1.0,
        "screen" => 2.0,
        "overlay" => 3.0,
        "add" => 4.0,
        "subtract" => 5.0,
        "difference" => 6.0,
        _ => 0.0,
    };
    let opacity = match params.get("opacity") {
        Some(Value::Float(v)) => *v,
        _ => 1.0,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_base.width, gpu_base.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        mode: f32,
        opacity: f32,
        _pad1: f32,
        _pad2: f32,
    }

    let p = Params {
        mode: mode_val,
        opacity,
        _pad1: 0.0,
        _pad2: 0.0,
    };

    let params_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blend_params"),
            contents: bytemuck::bytes_of(&p),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    let pipeline = gpu.pipeline("blend", include_str!("../../gpu/shaders/blend.wgsl"));
    let layout = pipeline.get_bind_group_layout(0);
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blend_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&gpu_base.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&gpu_layer.view),
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
