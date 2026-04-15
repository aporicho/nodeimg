use std::fs;
use std::path::Path;

use types::{DataType, Value};

use crate::artifact::handler::handler::ArtifactHandler;
use crate::artifact::model::ArtifactError;

pub struct VideoArtifactHandler;

impl VideoArtifactHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VideoArtifactHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactHandler for VideoArtifactHandler {
    fn data_type(&self) -> DataType {
        DataType::video()
    }

    fn extension(&self) -> &'static str {
        "mp4"
    }

    fn serialize(&self, value: &Value, path: &Path) -> Result<(), ArtifactError> {
        let Value::String(source_path) = value else {
            return Err(ArtifactError::SerializationFailed {
                message: "video handler only supports Value::String file paths".into(),
            });
        };

        fs::copy(source_path, path).map_err(|error| ArtifactError::StorageIo {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        Ok(())
    }

    fn deserialize(&self, path: &Path) -> Result<Value, ArtifactError> {
        Ok(Value::String(path.to_string_lossy().into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{ArtifactHandler, VideoArtifactHandler};
    use types::Value;

    #[test]
    fn roundtrip_video_file_copy() {
        let handler = VideoArtifactHandler::new();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("nodeimg-video-handler-{unique}"));
        fs::create_dir_all(&temp_dir).expect("create temp dir");
        let source = temp_dir.join("source.mp4");
        let target = temp_dir.join("artifact.mp4");
        fs::write(&source, b"not-a-real-video").expect("write source");

        handler
            .serialize(
                &Value::String(source.to_string_lossy().into_owned()),
                &target,
            )
            .expect("serialize video path");
        let restored = handler
            .deserialize(&target)
            .expect("deserialize video path");

        match restored {
            Value::String(path) => assert_eq!(path, target.to_string_lossy()),
            other => panic!("expected string path, got {other:?}"),
        }

        fs::remove_dir_all(&temp_dir).expect("cleanup temp dir");
    }
}
