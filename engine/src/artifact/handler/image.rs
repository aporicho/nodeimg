use std::path::Path;

use image::ImageFormat;
use types::{DataType, Image, Value};

use crate::artifact::handler::handler::ArtifactHandler;
use crate::artifact::model::ArtifactError;

pub struct ImageArtifactHandler;

impl ImageArtifactHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ImageArtifactHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactHandler for ImageArtifactHandler {
    fn data_type(&self) -> DataType {
        DataType::image()
    }

    fn extension(&self) -> &'static str {
        "png"
    }

    fn serialize(&self, value: &Value, path: &Path) -> Result<(), ArtifactError> {
        let Value::Image(image) = value else {
            return Err(ArtifactError::SerializationFailed {
                message: "image handler only supports Value::Image".into(),
            });
        };

        let Some(cpu_image) = image.cpu_data() else {
            return Err(ArtifactError::SerializationFailed {
                message: "image handler requires CPU image data".into(),
            });
        };

        cpu_image
            .save_with_format(path, ImageFormat::Png)
            .map_err(|err| ArtifactError::StorageIo {
                path: path.to_path_buf(),
                message: err.to_string(),
            })
    }

    fn deserialize(&self, path: &Path) -> Result<Value, ArtifactError> {
        let image = image::open(path).map_err(|err| ArtifactError::DeserializationFailed {
            message: format!("{}: {}", path.display(), err),
        })?;

        Ok(Value::Image(Image::from_cpu(image)))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::{DynamicImage, GenericImageView, RgbaImage};
    use types::Value;

    use super::{ArtifactHandler, ImageArtifactHandler};

    #[test]
    fn test_image_handler_roundtrip_png() {
        let handler = ImageArtifactHandler::new();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("nodeimg-image-handler-{unique}"));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let path = temp_dir.join("sample.png");

        let rgba = RgbaImage::from_pixel(2, 3, image::Rgba([255, 0, 0, 255]));
        let value = Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba)));

        handler.serialize(&value, &path).expect("serialize image");
        let restored = handler.deserialize(&path).expect("deserialize image");

        match restored {
            Value::Image(image) => {
                let cpu = image.cpu_data().expect("cpu image data");
                assert_eq!(cpu.dimensions(), (2, 3));
            }
            other => panic!("expected image, got {other:?}"),
        }

        fs::remove_dir_all(&temp_dir).expect("cleanup temp dir");
    }
}
