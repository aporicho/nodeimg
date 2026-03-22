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
        type_id: "solid_color".into(),
        title: "Solid Color".into(),
        category: CategoryId::new("generate"),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![
            ParamDef {
                name: "color".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([1.0, 1.0, 1.0, 1.0]),
                widget_override: None,
            },
            ParamDef {
                name: "width".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(512),
                widget_override: None,
            },
            ParamDef {
                name: "height".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 8192.0,
                },
                default: Value::Int(512),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn process(
    _inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let color = match params.get("color") {
        Some(Value::Color(c)) => *c,
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    let w = match params.get("width") {
        Some(Value::Int(v)) => *v,
        _ => 512,
    };
    let h = match params.get("height") {
        Some(Value::Int(v)) => *v,
        _ => 512,
    };

    let img = crate::processing::generate::solid_color(w as u32, h as u32, color);
    outputs.insert("image".into(), Value::Image(Arc::new(img)));
    outputs
}

fn gpu_process(
    gpu: &GpuContext,
    _inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let color = match params.get("color") {
        Some(Value::Color(c)) => *c,
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    let w = match params.get("width") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };
    let h = match params.get("height") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };

    // Dummy 1x1 input texture (shader ignores it)
    let dummy_input = GpuTexture::create_empty(&gpu.device, 1, 1);
    let output_tex = GpuTexture::create_empty(&gpu.device, w, h);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        color: [f32; 4],
    }

    let p = Params { color };

    let pipeline = gpu.pipeline(
        "solid_color",
        include_str!("../../gpu/shaders/solid_color.wgsl"),
    );
    let bind_group =
        create_io_params_bind_group(&gpu.device, &pipeline, &dummy_input, &output_tex, &p);
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
