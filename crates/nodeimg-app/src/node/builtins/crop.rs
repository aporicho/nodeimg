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
        type_id: "crop".into(),
        title: "Crop".into(),
        category: CategoryId::new("transform"),
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
                name: "x".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 8192.0,
                },
                default: Value::Int(0),
                widget_override: None,
            },
            ParamDef {
                name: "y".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 8192.0,
                },
                default: Value::Int(0),
                widget_override: None,
            },
            ParamDef {
                name: "width".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(256),
                widget_override: None,
            },
            ParamDef {
                name: "height".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(256),
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
        let x = match params.get("x") {
            Some(Value::Int(v)) => *v as u32,
            _ => 0,
        };
        let y = match params.get("y") {
            Some(Value::Int(v)) => *v as u32,
            _ => 0,
        };
        let w = match params.get("width") {
            Some(Value::Int(v)) => *v as u32,
            _ => 256,
        };
        let h = match params.get("height") {
            Some(Value::Int(v)) => *v as u32,
            _ => 256,
        };
        let result = crate::processing::transform::crop(img, x, y, w, h);
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

    let x = match params.get("x") {
        Some(Value::Int(v)) => *v as u32,
        _ => 0,
    };
    let y = match params.get("y") {
        Some(Value::Int(v)) => *v as u32,
        _ => 0,
    };
    let w = match params.get("width") {
        Some(Value::Int(v)) => *v as u32,
        _ => 256,
    };
    let h = match params.get("height") {
        Some(Value::Int(v)) => *v as u32,
        _ => 256,
    };

    // Clamp to input bounds (same as CPU)
    let cx = x.min(gpu_input.width.saturating_sub(1));
    let cy = y.min(gpu_input.height.saturating_sub(1));
    let cw = w.min(gpu_input.width - cx);
    let ch = h.min(gpu_input.height - cy);

    // Output texture has crop dimensions
    let output_tex = GpuTexture::create_empty(&gpu.device, cw, ch);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        offset_x: f32,
        offset_y: f32,
        _pad0: f32,
        _pad1: f32,
    }

    let p = Params {
        offset_x: cx as f32,
        offset_y: cy as f32,
        _pad0: 0.0,
        _pad1: 0.0,
    };

    let pipeline = gpu.pipeline("crop", include_str!("../../gpu/shaders/crop.wgsl"));
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
