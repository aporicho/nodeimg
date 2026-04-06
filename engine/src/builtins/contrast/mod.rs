use macros::node;

node! {
    name: "contrast",
    title: "Contrast",
    category: "color",
    inputs: [image: Image required],
    outputs: [image: Image],
    params: [contrast: Float range(-1.0, 1.0) default(0.0)],
    execute: |ctx, inputs| {
        (|| -> Result<std::collections::HashMap<String, types::Value>, Box<dyn std::error::Error + Send + Sync>> {
            let image = match inputs.get("image") {
                Some(types::Value::Image(img)) => img.clone(),
                _ => return Err("missing image input".into()),
            };
            let contrast = match inputs.get("contrast") {
                Some(types::Value::Float(v)) => *v,
                _ => 0.0,
            };

            ctx.gpu(|gpu| {
                let gpu_tex = image.as_gpu(&gpu.device, &gpu.queue);
                let output = types::GpuTexture::create_empty(&gpu.device, gpu_tex.width, gpu_tex.height);

                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct Params { contrast: f32 }

                let params = Params { contrast };
                let pipeline = gpu.pipeline("contrast", include_str!("shader.wgsl"));
                let bind_group = gpu.create_io_params_bind_group(&pipeline, &gpu_tex, &output, bytemuck::bytes_of(&params));
                gpu.dispatch_compute(&pipeline, &bind_group, output.width, output.height);

                let result_image = types::value::Image::from_gpu(output);
                let mut out = std::collections::HashMap::new();
                out.insert("image".to_string(), types::Value::Image(result_image));
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(out)
            }).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?
        })()
    },
}
