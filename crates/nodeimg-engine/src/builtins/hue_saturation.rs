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
        type_id: "hue_saturation".into(),
        title: "Hue/Saturation".into(),
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
                name: "hue".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -180.0,
                    max: 180.0,
                },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "saturation".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -1.0,
                    max: 1.0,
                },
                default: Value::Float(0.0),
                widget_override: None,
            },
            ParamDef {
                name: "lightness".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: -1.0,
                    max: 1.0,
                },
                default: Value::Float(0.0),
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
        let hue = match params.get("hue") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let saturation = match params.get("saturation") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let lightness = match params.get("lightness") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        };
        let result = nodeimg_processing::color::hsl_adjust(img, hue, saturation, lightness);
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
        hue_shift: f32,
        saturation: f32,
        lightness: f32,
        _pad: f32,
    }

    let p = Params {
        hue_shift: match params.get("hue") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        },
        saturation: match params.get("saturation") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        },
        lightness: match params.get("lightness") {
            Some(Value::Float(v)) => *v,
            _ => 0.0,
        },
        _pad: 0.0,
    };

    let pipeline = gpu.pipeline(
        "hue_saturation",
        nodeimg_gpu::shaders::HUE_SATURATION,
    );
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
