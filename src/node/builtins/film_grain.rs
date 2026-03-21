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
        type_id: "film_grain".into(),
        title: "Film Grain".into(),
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
        params: vec![
            ParamDef {
                name: "amount".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(0.3),
                widget_override: None,
            },
            ParamDef {
                name: "size".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.5, max: 3.0 },
                default: Value::Float(1.0),
                widget_override: None,
            },
            ParamDef {
                name: "seed".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 99999.0,
                },
                default: Value::Int(0),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: Some(Box::new(gpu_process)),
    });
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

    let amount = match params.get("amount") {
        Some(Value::Float(v)) => *v,
        _ => 0.3,
    };
    let size = match params.get("size") {
        Some(Value::Float(v)) => *v,
        _ => 1.0,
    };
    let seed = match params.get("seed") {
        Some(Value::Int(v)) => *v as f32,
        _ => 0.0,
    };

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        amount: f32,
        size: f32,
        seed: f32,
        _pad: f32,
    }
    let p = Params {
        amount,
        size,
        seed,
        _pad: 0.0,
    };

    let shader = include_str!("../../gpu/shaders/film_grain.wgsl");
    let out_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);
    let pipeline = gpu.pipeline("film_grain", shader);
    let bg = create_io_params_bind_group(&gpu.device, &pipeline, &gpu_input, &out_tex, &p);
    dispatch_compute(
        &gpu.device,
        &gpu.queue,
        &pipeline,
        &bg,
        out_tex.width,
        out_tex.height,
    );

    outputs.insert("image".into(), Value::GpuImage(Arc::new(out_tex)));
    outputs
}

fn process(
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::Image(img)) = inputs.get("image") {
        let amount = match params.get("amount") {
            Some(Value::Float(v)) => *v,
            _ => 0.3,
        };
        let size = match params.get("size") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };
        let seed = match params.get("seed") {
            Some(Value::Int(v)) => *v,
            _ => 0,
        };
        let result = crate::processing::filter::film_grain(img, amount, size, seed);
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
