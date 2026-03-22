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
        type_id: "rotate".into(),
        title: "Rotate".into(),
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
                name: "angle".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -360.0,
                    max: 360.0,
                },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "fill".into(),
                data_type: DataTypeId::new("color"),
                constraint: Constraint::None,
                default: Value::Color([0.0, 0.0, 0.0, 0.0]),
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
        let angle = match params.get("angle") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let fill = match params.get("fill") {
            Some(Value::Color(c)) => *c,
            _ => [0.0, 0.0, 0.0, 0.0],
        };
        let result = nodeimg_processing::transform::rotate(img, angle, fill);
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

    let angle = match params.get("angle") {
        Some(Value::Float(v)) => *v,
        _ => 0.0,
    };
    let fill = match params.get("fill") {
        Some(Value::Color(c)) => *c,
        _ => [0.0, 0.0, 0.0, 0.0],
    };

    // Convert angle to radians (negative for inverse mapping, same as CPU)
    let angle_rad = -angle.to_radians();

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        angle_rad: f32,
        fill_r: f32,
        fill_g: f32,
        fill_b: f32,
        fill_a: f32,
        center_x: f32,
        center_y: f32,
        _pad: f32,
    }

    let p = Params {
        angle_rad,
        fill_r: fill[0],
        fill_g: fill[1],
        fill_b: fill[2],
        fill_a: fill[3],
        center_x: (gpu_input.width as f32 - 1.0) / 2.0,
        center_y: (gpu_input.height as f32 - 1.0) / 2.0,
        _pad: 0.0,
    };

    let pipeline = gpu.pipeline("rotate", nodeimg_gpu::shaders::ROTATE);
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
