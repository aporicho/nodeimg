use macros::node;

node! {
    name: "load_image",
    title: "Load Image",
    category: "data",
    inputs: [],
    outputs: [image: Image],
    params: [path: String default("")],
    execute: |ctx, inputs| {
        (|| -> Result<std::collections::HashMap<String, types::Value>, Box<dyn std::error::Error + Send + Sync>> {
            let path = match inputs.get("path") {
                Some(types::Value::String(p)) if !p.is_empty() => p.clone(),
                _ => return Ok(std::collections::HashMap::new()),
            };
            let image = ctx.cpu(|cpu| cpu.load_image(&path))
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e })?;
            let mut out = std::collections::HashMap::new();
            out.insert("image".to_string(), types::Value::Image(image));
            Ok(out)
        })()
    },
}
