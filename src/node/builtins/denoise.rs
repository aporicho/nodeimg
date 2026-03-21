use crate::gpu::pipeline::{create_io_params_bind_group, dispatch_compute};
use crate::gpu::texture::GpuTexture;
use crate::gpu::GpuContext;
use crate::node::category::CategoryId;
use crate::node::constraint::Constraint;
use crate::node::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use crate::node::types::{DataTypeId, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "denoise".into(),
        title: "Denoise".into(),
        category: CategoryId::new("filter"),
        inputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: true,
        }],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![ParamDef {
            name: "strength".into(),
            data_type: DataTypeId::new("float"),
            constraint: Constraint::Range {
                min: 0.0,
                max: 30.0,
            },
            default: Value::Float(10.0),
            widget_override: None,
        }],
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
    if let Some(Value::Image(img)) = inputs.get("image") {
        let strength = match params.get("strength") {
            Some(Value::Float(v)) => *v,
            _ => 10.0,
        };
        let result = crate::processing::filter::denoise(img, strength);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}

fn gpu_process(
    gpu: &GpuContext,
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let gpu_input = match inputs.get("image") {
        Some(Value::GpuImage(tex)) => Arc::clone(tex),
        Some(Value::Image(img)) => {
            Arc::new(GpuTexture::from_dynamic_image(&gpu.device, &gpu.queue, img))
        }
        _ => return outputs,
    };

    let strength = match params.get("strength") {
        Some(Value::Float(v)) => *v,
        _ => 10.0,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        strength: f32,
        _pad0: f32,
        _pad1: f32,
        _pad2: f32,
    }

    let p = Params {
        strength,
        _pad0: 0.0,
        _pad1: 0.0,
        _pad2: 0.0,
    };

    let pipeline = gpu.pipeline("denoise", include_str!("../../gpu/shaders/denoise.wgsl"));
    let bind_group =
        create_io_params_bind_group(&gpu.device, &pipeline, &gpu_input, &output_tex, &p);
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
