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
        type_id: "distort".into(),
        title: "Distort".into(),
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
                name: "type".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("barrel".into(), "barrel".into()),
                        ("pincushion".into(), "pincushion".into()),
                        ("swirl".into(), "swirl".into()),
                    ],
                },
                default: Value::String("barrel".into()),
                widget_override: None,
            },
            ParamDef {
                name: "strength".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -2.0,
                    max: 2.0,
                },
                default: Value::Float(0.5),
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
        let distort_type = match params.get("type") {
            Some(Value::String(s)) => s.as_str(),
            _ => "barrel",
        };
        let strength = match params.get("strength") {
            Some(Value::Float(v)) => *v,
            _ => 0.5,
        };
        let result = nodeimg_processing::transform::distort(img, distort_type, strength);
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

    let distort_type = match params.get("type") {
        Some(Value::String(s)) => s.as_str(),
        _ => "barrel",
    };
    let strength = match params.get("strength") {
        Some(Value::Float(v)) => *v,
        _ => 0.5,
    };

    let type_val: u32 = match distort_type {
        "barrel" => 0,
        "pincushion" => 1,
        "swirl" => 2,
        _ => 0,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        distort_type: u32,
        strength: f32,
        center_x: f32,
        center_y: f32,
    }

    let p = Params {
        distort_type: type_val,
        strength,
        center_x: (gpu_input.width as f32 - 1.0) / 2.0,
        center_y: (gpu_input.height as f32 - 1.0) / 2.0,
    };

    let pipeline = gpu.pipeline("distort", nodeimg_gpu::shaders::DISTORT);
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
