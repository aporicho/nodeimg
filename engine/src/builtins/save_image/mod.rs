use macros::node;

node! {
    name: "save_image",
    title: "Save Image",
    category: "data",
    inputs: [image: Image required],
    outputs: [],
    params: [path: String default("")],
    execute: |_ctx, inputs| {
        // SaveImage: For M1, just validate inputs exist.
        // Full implementation needs device/queue for GPU readback,
        // which requires changes to CpuExecutor API (deferred).
        let _path = match inputs.get("path") {
            Some(types::Value::String(p)) if !p.is_empty() => p.clone(),
            _ => String::new(),
        };
        // TODO: ctx.cpu(|cpu| cpu.save_image(&image, &path, device, queue))
        Ok(std::collections::HashMap::new())
    },
}
