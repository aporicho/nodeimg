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
        type_id: "levels".into(),
        title: "Levels".into(),
        category: CategoryId::new("color"),
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
                name: "in_black".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "in_white".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(1.0),
                widget_override: None,
            },
            ParamDef {
                name: "gamma".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: 0.1,
                    max: 10.0,
                },
                default: Value::Float(1.0),
                widget_override: None,
            },
            ParamDef {
                name: "out_black".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "out_white".into(),
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
    if let Some(Value::Image(img)) = inputs.get("image") {
        let in_black = match params.get("in_black") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let in_white = match params.get("in_white") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };
        let gamma = match params.get("gamma") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };
        let out_black = match params.get("out_black") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let out_white = match params.get("out_white") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        };
        let result =
            crate::processing::color::levels(img, in_black, in_white, gamma, out_black, out_white);
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

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        in_black: f32,
        in_white: f32,
        gamma: f32,
        out_black: f32,
        out_white: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        in_black: match params.get("in_black") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        },
        in_white: match params.get("in_white") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        },
        gamma: match params.get("gamma") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        },
        out_black: match params.get("out_black") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        },
        out_white: match params.get("out_white") {
            Some(Value::Float(v)) => *v,
            _ => 1.0,
        },
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let pipeline = gpu.pipeline("levels", include_str!("../../gpu/shaders/levels.wgsl"));
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
