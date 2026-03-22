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
        type_id: "flip".into(),
        title: "Flip".into(),
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
        params: vec![ParamDef {
            name: "direction".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::Enum {
                options: vec![
                    ("horizontal".into(), "horizontal".into()),
                    ("vertical".into(), "vertical".into()),
                    ("both".into(), "both".into()),
                ],
            },
            default: Value::String("horizontal".into()),
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
        let direction = match params.get("direction") {
            Some(Value::String(s)) => s.as_str(),
            _ => "horizontal",
        };
        let result = nodeimg_processing::transform::flip(img, direction);
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

    let direction = match params.get("direction") {
        Some(Value::String(s)) => s.as_str(),
        _ => "horizontal",
    };
    let dir_val: u32 = match direction {
        "horizontal" => 0,
        "vertical" => 1,
        "both" => 2,
        _ => 0,
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        direction: u32,
        _pad0: u32,
        _pad1: u32,
        _pad2: u32,
    }

    let p = Params {
        direction: dir_val,
        _pad0: 0,
        _pad1: 0,
        _pad2: 0,
    };

    let pipeline = gpu.pipeline("flip", nodeimg_gpu::shaders::FLIP);
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
