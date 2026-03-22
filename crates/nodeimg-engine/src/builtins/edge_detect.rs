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
        type_id: "edge_detect".into(),
        title: "Edge Detect".into(),
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
            name: "method".into(),
            data_type: DataTypeId::new("string"),
            constraint: Constraint::Enum {
                options: vec![
                    ("sobel".into(), "sobel".into()),
                    ("canny".into(), "canny".into()),
                    ("laplacian".into(), "laplacian".into()),
                ],
            },
            default: Value::String("sobel".into()),
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

    let method = match params.get("method") {
        Some(Value::String(s)) => s.as_str(),
        _ => "sobel",
    };

    // GPU supports sobel and laplacian; canny falls back to CPU
    if method == "canny" {
        return outputs;
    }

    let method_val: u32 = match method {
        "laplacian" => 1,
        _ => 0, // sobel
    };

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        method: u32,
        _pad0: u32,
        _pad1: u32,
        _pad2: u32,
    }

    let p = Params {
        method: method_val,
        _pad0: 0,
        _pad1: 0,
        _pad2: 0,
    };

    let pipeline = gpu.pipeline(
        "edge_detect",
        nodeimg_gpu::shaders::EDGE_DETECT,
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
