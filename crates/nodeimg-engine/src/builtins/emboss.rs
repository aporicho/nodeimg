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
        type_id: "emboss".into(),
        title: "Emboss".into(),
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
                name: "strength".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 5.0 },
                default: Value::Float(1.0),
                widget_override: None,
            },
            ParamDef {
                name: "angle".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 360.0,
                },
                default: Value::Float(135.0),
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
        _ => 1.0,
    };
    let angle = match params.get("angle") {
        Some(Value::Float(v)) => *v,
        _ => 135.0,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        strength: f32,
        angle_rad: f32,
        _pad0: f32,
        _pad1: f32,
    }

    let p = Params {
        strength,
        angle_rad: angle.to_radians(),
        _pad0: 0.0,
        _pad1: 0.0,
    };

    let pipeline = gpu.pipeline("emboss", nodeimg_gpu::shaders::EMBOSS);
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
