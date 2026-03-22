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
    let range = || Constraint::Range {
        min: -1.0,
        max: 1.0,
    };
    let zero = || Value::Float(0.0);

    registry.register(NodeDef {
        type_id: "color_balance".into(),
        title: "Color Balance".into(),
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
                name: "shadows_r".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "shadows_g".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "shadows_b".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "midtones_r".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "midtones_g".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "midtones_b".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "highlights_r".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "highlights_g".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
            ParamDef {
                name: "highlights_b".into(),
                data_type: DataTypeId::new("float"),
                constraint: range(),
                default: zero(),
                widget_override: None,
            },
        ],
        has_preview: false,
        process: Some(Box::new(process)),
        gpu_process: Some(Box::new(gpu_process)),
    });
}

fn get_float(params: &HashMap<String, Value>, key: &str) -> f32 {
    match params.get(key) {
        Some(Value::Float(v)) => *v,
        _ => 0.0,
    }
}

fn process(
    inputs: &HashMap<String, Value>,
    params: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut outputs = HashMap::new();
    if let Some(Value::Image(img)) = inputs.get("image") {
        let shadows = [
            get_float(params, "shadows_r"),
            get_float(params, "shadows_g"),
            get_float(params, "shadows_b"),
        ];
        let midtones = [
            get_float(params, "midtones_r"),
            get_float(params, "midtones_g"),
            get_float(params, "midtones_b"),
        ];
        let highlights = [
            get_float(params, "highlights_r"),
            get_float(params, "highlights_g"),
            get_float(params, "highlights_b"),
        ];
        let result = nodeimg_processing::color::color_balance(img, shadows, midtones, highlights);
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
        shadows_r: f32,
        shadows_g: f32,
        shadows_b: f32,
        midtones_r: f32,
        midtones_g: f32,
        midtones_b: f32,
        highlights_r: f32,
        highlights_g: f32,
        highlights_b: f32,
        _pad1: f32,
        _pad2: f32,
        _pad3: f32,
    }

    let p = Params {
        shadows_r: get_float(params, "shadows_r"),
        shadows_g: get_float(params, "shadows_g"),
        shadows_b: get_float(params, "shadows_b"),
        midtones_r: get_float(params, "midtones_r"),
        midtones_g: get_float(params, "midtones_g"),
        midtones_b: get_float(params, "midtones_b"),
        highlights_r: get_float(params, "highlights_r"),
        highlights_g: get_float(params, "highlights_g"),
        highlights_b: get_float(params, "highlights_b"),
        _pad1: 0.0,
        _pad2: 0.0,
        _pad3: 0.0,
    };

    let pipeline = gpu.pipeline(
        "color_balance",
        nodeimg_gpu::shaders::COLOR_BALANCE,
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
