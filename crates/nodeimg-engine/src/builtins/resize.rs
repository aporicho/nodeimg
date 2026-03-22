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
        type_id: "resize".into(),
        title: "Resize".into(),
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
            ParamDef {
                name: "method".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::Enum {
                    options: vec![
                        ("nearest".into(), "nearest".into()),
                        ("bilinear".into(), "bilinear".into()),
                        ("bicubic".into(), "bicubic".into()),
                        ("lanczos3".into(), "lanczos3".into()),
                    ],
                },
                default: Value::String("bilinear".into()),
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
        let w = match params.get("width") {
            Some(Value::Int(v)) => *v as u32,
            _ => 512,
        };
        let h = match params.get("height") {
            Some(Value::Int(v)) => *v as u32,
            _ => 512,
        };
        let method = match params.get("method") {
            Some(Value::String(s)) => s.as_str(),
            _ => "bilinear",
        };
        let result = nodeimg_processing::transform::resize(img, w, h, method);
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

    let dst_w = match params.get("width") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };
    let dst_h = match params.get("height") {
        Some(Value::Int(v)) => *v as u32,
        _ => 512,
    };

    let method_id: u32 = match params.get("method") {
        Some(Value::String(s)) => match s.as_str() {
            "nearest" => 0,
            "bilinear" => 1,
            "bicubic" => 2,
            "lanczos3" => 3,
            _ => 1,
        },
        _ => 1,
    };

    // Output texture has target dimensions
    let output_tex = GpuTexture::create_empty(&gpu.device, dst_w, dst_h);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        src_width: f32,
        src_height: f32,
        dst_width: f32,
        dst_height: f32,
        method: u32,
        _pad0: u32,
        _pad1: u32,
        _pad2: u32,
    }

    let p = Params {
        src_width: gpu_input.width as f32,
        src_height: gpu_input.height as f32,
        dst_width: dst_w as f32,
        dst_height: dst_h as f32,
        method: method_id,
        _pad0: 0,
        _pad1: 0,
        _pad2: 0,
    };

    let pipeline = gpu.pipeline("resize", nodeimg_gpu::shaders::RESIZE);
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
