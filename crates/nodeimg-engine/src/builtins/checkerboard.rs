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
        type_id: "checkerboard".into(),
        title: "Checkerboard".into(),
        category: CategoryId::new("generate"),
        inputs: vec![],
        outputs: vec![PinDef {
            name: "image".into(),
            data_type: DataTypeId::new("image"),
            required: false,
        }],
        params: vec![
            ParamDef {
                name: "color_a".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([1.0, 1.0, 1.0, 1.0]),
                widget_override: None,
            },
            ParamDef {
                name: "color_b".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([0.0, 0.0, 0.0, 1.0]),
                widget_override: None,
            },
            ParamDef {
                name: "size".into(),
                data_type: DataTypeId::new("int"),
                constraint: Constraint::Range {
                    min: 1.0,
                    max: 512.0,
                },
                default: Value::Int(32),
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
        process: None,
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn gpu_process(
    gpu: &GpuContext,
    _inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();

    let color_a = match params.get("color_a") {
        Some(Value::Color(c)) => *c,
        _ => [1.0, 1.0, 1.0, 1.0],
    };
    let color_b = match params.get("color_b") {
        Some(Value::Color(c)) => *c,
        _ => [0.0, 0.0, 0.0, 1.0],
    };
    let size = match params.get("size") {
        Some(Value::Int(v)) => *v as f32,
        _ => 32.0,
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
        color_a: [f32; 4],
        color_b: [f32; 4],
        size: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        color_a,
        color_b,
        size,
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let pipeline = gpu.pipeline(
        "checkerboard",
        nodeimg_gpu::shaders::CHECKERBOARD,
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
