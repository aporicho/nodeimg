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
        type_id: "blur".into(),
        title: "Blur".into(),
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
                name: "radius".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range {
                    min: 0.0,
                    max: 100.0,
                },
                default: Value::Float(5.0),
                widget_override: None,
            },
            ParamDef {
                name: "method".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("gaussian".into(), "gaussian".into()),
                        ("box".into(), "box".into()),
                    ],
                },
                default: Value::String("gaussian".into()),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Some(Box::new(process)),
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

    let radius = match params.get("radius") {
        Some(Value::Float(v)) => *v,
        _ => 5.0,
    };
    let method = match params.get("method") {
        Some(Value::String(s)) => s.clone(),
        _ => "gaussian".into(),
    };

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        radius: f32,
        _pad0: f32,
        _pad1: f32,
        _pad2: f32,
    }
    let p = Params {
        radius,
        _pad0: 0.0,
        _pad1: 0.0,
        _pad2: 0.0,
    };

    let (shader_h, shader_v, name_h, name_v) = if method == "box" {
        (
            nodeimg_gpu::shaders::BOX_BLUR_H,
            nodeimg_gpu::shaders::BOX_BLUR_V,
            "box_blur_h",
            "box_blur_v",
        )
    } else {
        (
            nodeimg_gpu::shaders::GAUSSIAN_BLUR_H,
            nodeimg_gpu::shaders::GAUSSIAN_BLUR_V,
            "gaussian_blur_h",
            "gaussian_blur_v",
        )
    };

    // Pass 1: horizontal
    let mid_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);
    let pipeline_h = gpu.pipeline(name_h, shader_h);
    let bg_h = create_io_params_bind_group(&gpu.device, &pipeline_h, &gpu_input, &mid_tex, &p);
    dispatch_compute(
        &gpu.device,
        &gpu.queue,
        &pipeline_h,
        &bg_h,
        mid_tex.width,
        mid_tex.height,
    );

    // Pass 2: vertical
    let out_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);
    let pipeline_v = gpu.pipeline(name_v, shader_v);
    let bg_v = create_io_params_bind_group(&gpu.device, &pipeline_v, &mid_tex, &out_tex, &p);
    dispatch_compute(
        &gpu.device,
        &gpu.queue,
        &pipeline_v,
        &bg_v,
        out_tex.width,
        out_tex.height,
    );

    outputs.insert("image".into(), Value::GpuImage(Arc::new(out_tex)));
    outputs
}

fn process(
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::Image(img)) = inputs.get("image") {
        let radius = match params.get("radius") {
            Some(Value::Float(v)) => *v,
            _ => 5.0,
        };
        let method = match params.get("method") {
            Some(Value::String(s)) => s.as_str(),
            _ => "gaussian",
        };
        let result = if method == "box" {
            nodeimg_processing::filter::box_blur(img, radius)
        } else {
            nodeimg_processing::filter::gaussian_blur(img, radius)
        };
        outputs.insert("image".into(), Value::Image(Arc::new(result)));
    }
    outputs
}
