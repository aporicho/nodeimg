use nodeimg_gpu::pipeline::{create_io_params_bind_group, dispatch_compute};
use nodeimg_types::gpu_texture::GpuTexture;
use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use crate::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "gradient".into(),
        title: "Gradient".into(),
        category: CategoryId::new("generate"),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![
            ParamDef {
                name: "color_start".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([0.0, 0.0, 0.0, 1.0]),
                widget_override: None,
            },
            ParamDef {
                name: "color_end".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([1.0, 1.0, 1.0, 1.0]),
                widget_override: None,
            },
            ParamDef {
                name: "direction".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("horizontal".into(), "horizontal".into()),
                        ("vertical".into(), "vertical".into()),
                        ("diagonal".into(), "diagonal".into()),
                    ],
                },
                default: Value::String("horizontal".into()),
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

    let color_start = match params.get("color_start") {
        Some(Value::Color(c)) => *c,
        _ => [0.0, 0.0, 0.0, 1.0],
    };
    let color_end = match params.get("color_end") {
        Some(Value::Color(c)) => *c,
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    let direction = match params.get("direction") {
        Some(Value::String(s)) => s.clone(),
        _ => "horizontal".into(),
    };
    let w = match params.get("width") {
        Some(Value::Int(v)) => *v,
        _ => 512,
    };
    let h = match params.get("height") {
        Some(Value::Int(v)) => *v,
        _ => 512,
    };

    let img = nodeimg_processing::generate::gradient(
        w as u32,
        h as u32,
        color_start,
        color_end,
        &direction,
    );
    outputs.insert("image".into(), Value::Image(Arc::new(img)));
    outputs
}

fn gpu_process(
    gpu: &GpuContext,
    _inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let color_start = match params.get("color_start") {
        Some(Value::Color(c)) => *c,
        _ => [0.0, 0.0, 0.0, 1.0],
    };
    let color_end = match params.get("color_end") {
        Some(Value::Color(c)) => *c,
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    let direction = match params.get("direction") {
        Some(Value::String(s)) => match s.as_str() {
            "vertical" => 1.0_f32,
            "diagonal" => 2.0_f32,
            _ => 0.0_f32, // horizontal
        },
        _ => 0.0_f32,
    };
    let w = match params.get("width") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };
    let h = match params.get("height") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };

    let dummy_input = GpuTexture::create_empty(&gpu.device, 1, 1);
    let output_tex = GpuTexture::create_empty(&gpu.device, w, h);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        color_start: [f32; 4],
        color_end: [f32; 4],
        direction: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        color_start,
        color_end,
        direction,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let pipeline = gpu.pipeline(
        "gradient",
        nodeimg_gpu::shaders::GRADIENT,
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
