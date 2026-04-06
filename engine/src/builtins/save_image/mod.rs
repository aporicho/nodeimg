use macros::node;

node! {
    name: "save_image",
    title: "Save Image",
    category: "data",
    inputs: [image: Image required],
    outputs: [],
    params: [path: String default("")],
    execute: |ctx, inputs| {
        (|| -> Result<std::collections::HashMap<String, types::Value>, Box<dyn std::error::Error + Send + Sync>> {
            let path = match inputs.get("path") {
                Some(types::Value::String(p)) if !p.is_empty() => p.clone(),
                _ => return Ok(std::collections::HashMap::new()),
            };
            let image = match inputs.get("image") {
                Some(types::Value::Image(img)) => img,
                _ => return Ok(std::collections::HashMap::new()),
            };
            // Try cpu_data() first (no GPU needed)
            if let Some(cpu_img) = image.cpu_data() {
                cpu_img.save(&path)?;
            } else if let Some((device, queue)) = ctx.device_queue() {
                // GPU readback via ensure_cpu
                let cpu_img = image.as_cpu(device, queue);
                cpu_img.save(&path)?;
            } else {
                return Err("No CPU data and no GPU available for readback".into());
            }
            Ok(std::collections::HashMap::new())
        })()
    },
}
