use std::fs;
use std::path::{Path, PathBuf};

use types::{NodeId, Value};

use crate::artifact::handler::handler::ArtifactHandler;
use crate::artifact::model::{ArtifactError, ArtifactRecord};

#[derive(Clone, Debug)]
pub struct ArtifactStore {
    project_root: PathBuf,
}

impl ArtifactStore {
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub fn artifact_path(
        &self,
        node_id: NodeId,
        output_key: &str,
        version: u64,
        extension: &str,
    ) -> PathBuf {
        self.project_root
            .join("artifacts")
            .join(node_id.0.to_string())
            .join(output_key)
            .join(format!("{version:06}.{extension}"))
    }

    pub fn index_path(&self) -> PathBuf {
        self.project_root.join("artifacts.json")
    }

    pub fn ensure_parent_dir(&self, path: &Path) -> Result<(), ArtifactError> {
        let Some(parent) = path.parent() else {
            return Err(ArtifactError::StorageIo {
                path: path.to_path_buf(),
                message: "artifact path has no parent directory".into(),
            });
        };

        fs::create_dir_all(parent).map_err(|err| ArtifactError::StorageIo {
            path: parent.to_path_buf(),
            message: err.to_string(),
        })
    }

    pub fn write(
        &self,
        record: &ArtifactRecord,
        value: &Value,
        handler: &dyn ArtifactHandler,
    ) -> Result<(), ArtifactError> {
        self.ensure_parent_dir(&record.path)?;

        let file_name = record
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ArtifactError::StorageIo {
                path: record.path.clone(),
                message: "artifact file name is invalid".into(),
            })?;

        let temp_path = record.path.with_file_name(format!(".{file_name}.tmp"));

        handler.serialize(value, &temp_path)?;
        fs::rename(&temp_path, &record.path).map_err(|err| ArtifactError::StorageIo {
            path: record.path.clone(),
            message: err.to_string(),
        })
    }

    pub fn read(
        &self,
        record: &ArtifactRecord,
        handler: &dyn ArtifactHandler,
    ) -> Result<Value, ArtifactError> {
        handler.deserialize(&record.path)
    }

    pub fn delete(&self, record: &ArtifactRecord) -> Result<(), ArtifactError> {
        fs::remove_file(&record.path).map_err(|err| ArtifactError::StorageIo {
            path: record.path.clone(),
            message: err.to_string(),
        })
    }

    pub fn exists(&self, record: &ArtifactRecord) -> bool {
        record.path.exists()
    }

    pub fn file_size(&self, record: &ArtifactRecord) -> Result<u64, ArtifactError> {
        let metadata = fs::metadata(&record.path).map_err(|err| ArtifactError::StorageIo {
            path: record.path.clone(),
            message: err.to_string(),
        })?;

        Ok(metadata.len())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use image::{DynamicImage, GenericImageView, RgbaImage};
    use types::{DataType, NodeId, Value};

    use crate::artifact::handler::handler::ArtifactHandler;
    use crate::artifact::handler::image::ImageArtifactHandler;
    use crate::artifact::model::{ArtifactKind, ArtifactRecord};

    use super::ArtifactStore;

    fn make_temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nodeimg-artifact-store-{unique}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn sample_record(path: PathBuf) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: "art_001".into(),
            node_id: NodeId(42),
            output_key: "image".into(),
            version: 1,
            created_at: UNIX_EPOCH + Duration::from_secs(1),
            data_type: DataType::image(),
            format: "png".into(),
            path,
            param_signature: "param-1".into(),
            input_signature: "input-1".into(),
            kind: ArtifactKind::Restorable,
            orphaned: false,
        }
    }

    #[test]
    fn test_artifact_path_is_predictable() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());

        let path = store.artifact_path(NodeId(42), "image", 7, "png");

        assert_eq!(path, root.join("artifacts/42/image/000007.png"));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_write_read_delete_roundtrip() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let handler = ImageArtifactHandler::new();
        let path = store.artifact_path(NodeId(42), "image", 1, handler.extension());
        let record = sample_record(path);

        let rgba = RgbaImage::from_pixel(3, 2, image::Rgba([0, 255, 0, 255]));
        let value = Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba)));

        store
            .write(&record, &value, &handler)
            .expect("write artifact");
        assert!(store.exists(&record));
        assert!(store.file_size(&record).expect("file size") > 0);

        let restored = store.read(&record, &handler).expect("read artifact");
        match restored {
            Value::Image(image) => {
                let cpu = image.cpu_data().expect("cpu image data");
                assert_eq!(cpu.dimensions(), (3, 2));
            }
            other => panic!("expected image, got {other:?}"),
        }

        store.delete(&record).expect("delete artifact");
        assert!(!store.exists(&record));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
