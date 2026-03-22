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
        type_id: "pixelate".into(),
        title: "Pixelate".into(),
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
        params: vec![ParamDef {
            name: "block_size".into(),
            data_type: DataTypeId::new("int"),
            constraint: Constraint::Range {
                min: 1.0,
                max: 256.0,
            },
            default: Value::Int(8),
            widget_override: None,
        }],
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

    let block_size = match params.get("block_size") {
        Some(Value::Int(v)) => *v as f32,
        _ => 8.0,
    };

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        block_size: f32,
        _pad0: f32,
        _pad1: f32,
        _pad2: f32,
    }
    let p = Params {
        block_size,
        _pad0: 0.0,
        _pad1: 0.0,
        _pad2: 0.0,
    };

    let shader = nodeimg_gpu::shaders::PIXELATE;
    let out_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);
    let pipeline = gpu.pipeline("pixelate", shader);
    let bg = create_io_params_bind_group(&gpu.device, &pipeline, &gpu_input, &out_tex, &p);
    dispatch_compute(
        &gpu.device,
        &gpu.queue,
        &pipeline,
        &bg,
        out_tex.width,
        out_tex.height,
    );

    outputs.insert("image".into(), Value::GpuImage(Arc::new(out_tex)));
    outputs
}

