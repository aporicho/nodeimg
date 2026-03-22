use nodeimg_gpu::pipeline::dispatch_compute;
use nodeimg_types::gpu_texture::GpuTexture;
use nodeimg_gpu::GpuContext;
use nodeimg_types::category::CategoryId;
use nodeimg_types::constraint::Constraint;
use crate::registry::{NodeDef, NodeRegistry, ParamDef, PinDef};
use nodeimg_types::data_type::DataTypeId;
use nodeimg_types::value::Value;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub fn register(registry: &mut NodeRegistry) {
    registry.register(NodeDef {
        type_id: "lut_apply".into(),
        title: "LUT Apply".into(),
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
                name: "path".into(),
                data_type: DataTypeId::new("string"),
                constraint: Constraint::FilePath {
                    filters: vec!["cube".into()],
                },
                default: Value::String(String::new()),
                widget_override: None,
            },
            ParamDef {
                name: "intensity".into(),
                data_type: DataTypeId::new("float"),
                constraint: Constraint::Range { min: 0.0, max: 1.0 },
                default: Value::Float(1.0),
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

    let path = match params.get("path") {
        Some(Value::String(s)) => s.clone(),
        _ => return outputs,
    };
    if path.is_empty() {
        return outputs;
    }

    let intensity = match params.get("intensity") {
        Some(Value::Float(v)) => *v,
        _ => 1.0,
    };

    // Parse LUT file
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return outputs,
    };
    let lut = match nodeimg_processing::color::parse_cube_lut(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("LUT parse error: {e}");
            return outputs;
        }
    };

    // Upload LUT data as storage buffer (vec4 per entry for alignment)
    let lut_buffer_data: Vec<[f32; 4]> = lut.data.iter().map(|e| [e[0], e[1], e[2], 0.0]).collect();
    let lut_buffer = gpu
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lut_data"),
            contents: bytemuck::cast_slice(&lut_buffer_data),
            usage: wgpu::BufferUsages::STORAGE,
        });

    #[repr(C)]
    #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
    struct Params {
        lut_size: f32,
        intensity: f32,
        _pad0: f32,
        _pad1: f32,
    }
    let p = Params {
        lut_size: lut.size as f32,
        intensity,
        _pad0: 0.0,
        _pad1: 0.0,
    };
    let params_buffer =
        gpu.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("lut_params"),
                contents: bytemuck::bytes_of(&p),
                usage: wgpu::BufferUsages::UNIFORM,
            });

    let output_tex = GpuTexture::create_empty(&gpu.device, gpu_input.width, gpu_input.height);
    let pipeline = gpu.pipeline("lut_apply", nodeimg_gpu::shaders::LUT_APPLY);

    // Custom bind group: input, output, params uniform, lut storage buffer
    let layout = pipeline.get_bind_group_layout(0);
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lut_apply_bind_group"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&gpu_input.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&output_tex.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: lut_buffer.as_entire_binding(),
            },
        ],
    });

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

