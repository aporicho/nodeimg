use types::Value;

pub fn estimate_bytes(value: &Value) -> usize {
    match value {
        Value::Float(_) | Value::Int(_) | Value::Bool(_) | Value::Color(_) => 0,
        Value::String(text) => text.len(),
        Value::Image(image) => image
            .cpu_data()
            .map(|cpu| {
                let rgba = cpu.to_rgba8();
                rgba.width() as usize * rgba.height() as usize * 4
            })
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use image::DynamicImage;
    use types::{Image, Value};

    use super::estimate_bytes;

    #[test]
    fn scalars_estimate_to_zero() {
        assert_eq!(estimate_bytes(&Value::Float(1.0)), 0);
        assert_eq!(estimate_bytes(&Value::Int(1)), 0);
        assert_eq!(estimate_bytes(&Value::Bool(true)), 0);
        assert_eq!(estimate_bytes(&Value::Color([0.0, 0.0, 0.0, 1.0])), 0);
    }

    #[test]
    fn string_estimates_to_utf8_length() {
        assert_eq!(estimate_bytes(&Value::String("nodeimg".into())), 7);
    }

    #[test]
    fn image_estimates_from_cpu_resident_bytes() {
        let image = Image::from_cpu(DynamicImage::new_rgba8(4, 2));

        assert_eq!(estimate_bytes(&Value::Image(image)), 32);
    }
}
