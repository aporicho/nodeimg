use std::fs;
use std::path::PathBuf;

use types::{DataType, NodeId, Value};

use crate::artifact::handler::{ArtifactHandler, ArtifactRegistry};
use crate::artifact::lifecycle::cleaner::{ArtifactCleaner, CleanupCandidate, CleanupPolicy};
use crate::artifact::lifecycle::restorer::{ArtifactRestoreDecision, ArtifactRestorer};
use crate::artifact::lifecycle::selector::ArtifactSelector;
use crate::artifact::lifecycle::validator::{ArtifactValidator, ValidationReport};
use crate::artifact::model::{
    ArtifactError, ArtifactRecord, CleanupReport, CreateArtifactRequest, ResolveForRestoreRequest,
};
use crate::artifact::persistence::index::ArtifactIndex;
use crate::artifact::persistence::store::ArtifactStore;

pub struct ArtifactManager {
    index: ArtifactIndex,
    store: ArtifactStore,
    registry: ArtifactRegistry,
}

impl ArtifactManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            index: ArtifactIndex::new(),
            store: ArtifactStore::new(project_root),
            registry: ArtifactRegistry::with_defaults(),
        }
    }

    pub fn index(&self) -> &ArtifactIndex {
        &self.index
    }

    pub fn store(&self) -> &ArtifactStore {
        &self.store
    }

    pub fn save_index(&self) -> Result<(), ArtifactError> {
        let index_path = self.store.index_path();
        let json = serde_json::to_string_pretty(&self.index.to_file()).map_err(|err| {
            ArtifactError::SerializationFailed {
                message: err.to_string(),
            }
        })?;

        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).map_err(|err| ArtifactError::StorageIo {
                path: parent.to_path_buf(),
                message: err.to_string(),
            })?;
        }

        fs::write(&index_path, json).map_err(|err| ArtifactError::StorageIo {
            path: index_path,
            message: err.to_string(),
        })
    }

    pub fn load_index(&mut self) -> Result<(), ArtifactError> {
        let index_path = self.store.index_path();
        if !index_path.exists() {
            self.index = ArtifactIndex::new();
            return Ok(());
        }

        let json = fs::read_to_string(&index_path).map_err(|err| ArtifactError::StorageIo {
            path: index_path.clone(),
            message: err.to_string(),
        })?;

        let file =
            serde_json::from_str(&json).map_err(|err| ArtifactError::DeserializationFailed {
                message: err.to_string(),
            })?;

        self.index = ArtifactIndex::from_file(file)?;
        Ok(())
    }

    pub fn validate_store(&self) -> ValidationReport {
        ArtifactValidator::validate(&self.index, &self.store)
    }

    pub fn plan_cleanup(&self, policy: &CleanupPolicy) -> Vec<CleanupCandidate> {
        ArtifactCleaner::plan_cleanup(&self.index, &self.store, policy)
    }

    pub fn cleanup(&mut self, policy: &CleanupPolicy) -> Result<CleanupReport, ArtifactError> {
        let report = ArtifactCleaner::execute_cleanup(&mut self.index, &self.store, policy)?;
        self.save_index()?;
        Ok(report)
    }

    pub fn create_artifact(
        &mut self,
        req: CreateArtifactRequest,
    ) -> Result<ArtifactRecord, ArtifactError> {
        let extension = self.handler_for_data_type(&req.data_type)?.extension();
        let version = self.index.allocate_version(req.node_id, &req.output_key);
        let path = self
            .store
            .artifact_path(req.node_id, &req.output_key, version, extension);

        let record = ArtifactRecord {
            artifact_id: format!("{}:{}:{}", req.node_id.0, req.output_key, version),
            node_id: req.node_id,
            output_key: req.output_key.clone(),
            version,
            created_at: std::time::SystemTime::now(),
            data_type: req.data_type.clone(),
            format: req.format.clone(),
            path,
            param_signature: req.param_signature.clone(),
            input_signature: req.input_signature.clone(),
            kind: req.kind.clone(),
            orphaned: false,
        };

        let handler = self.handler_for_data_type(&req.data_type)?;
        self.store.write(&record, &req.value, handler)?;
        self.index.upsert(record.clone(), true);

        Ok(record)
    }

    pub fn list_artifacts(&self, node_id: NodeId, output_key: &str) -> Vec<ArtifactRecord> {
        self.index.list(node_id, output_key)
    }

    pub fn get_artifact(&self, artifact_id: &str) -> Option<&ArtifactRecord> {
        self.index.get(artifact_id)
    }

    pub fn get_selected_artifact(
        &self,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<&ArtifactRecord, ArtifactError> {
        ArtifactSelector::get_selected(&self.index, node_id, output_key)
    }

    pub fn select_artifact(&mut self, artifact_id: &str) -> Result<(), ArtifactError> {
        ArtifactSelector::select(&mut self.index, artifact_id)
    }

    pub fn read_artifact(&self, artifact_id: &str) -> Result<Value, ArtifactError> {
        let record =
            self.index
                .get(artifact_id)
                .ok_or_else(|| ArtifactError::ArtifactNotFound {
                    artifact_id: artifact_id.to_owned(),
                })?;

        let handler = self.handler_for_data_type(&record.data_type)?;
        self.store.read(record, handler)
    }

    pub fn resolve_for_restore<'a>(
        &'a self,
        req: &ResolveForRestoreRequest,
    ) -> ArtifactRestoreDecision<'a> {
        ArtifactRestorer::resolve(
            &self.index,
            req.node_id,
            &req.output_key,
            &req.param_signature,
            &req.input_signature,
        )
    }

    pub fn delete_artifact(&mut self, artifact_id: &str) -> Result<(), ArtifactError> {
        ArtifactSelector::ensure_not_selected(&self.index, artifact_id)?;

        let record = self.index.get(artifact_id).cloned().ok_or_else(|| {
            ArtifactError::ArtifactNotFound {
                artifact_id: artifact_id.to_owned(),
            }
        })?;

        self.store.delete(&record)?;
        self.index.remove(artifact_id);
        Ok(())
    }

    fn handler_for_data_type(
        &self,
        data_type: &DataType,
    ) -> Result<&dyn ArtifactHandler, ArtifactError> {
        self.registry.get(data_type)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use image::{DynamicImage, GenericImageView, RgbaImage};
    use types::{DataType, NodeId, Value};

    use crate::artifact::lifecycle::cleaner::CleanupPolicy;
    use crate::artifact::lifecycle::restorer::ArtifactRestoreDecision;
    use crate::artifact::model::{ArtifactKind, CreateArtifactRequest, ResolveForRestoreRequest};

    use super::ArtifactManager;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

    fn make_manager() -> (ArtifactManager, std::path::PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let seq = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nodeimg-artifact-manager-{unique}-{seq}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        (ArtifactManager::new(root.clone()), root)
    }

    fn sample_request(param_signature: &str) -> CreateArtifactRequest {
        let rgba = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 255, 255]));
        CreateArtifactRequest {
            node_id: NodeId(42),
            output_key: "image".into(),
            data_type: DataType::image(),
            format: "png".into(),
            value: Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba))),
            param_signature: param_signature.into(),
            input_signature: "input-1".into(),
            kind: ArtifactKind::Restorable,
        }
    }

    #[test]
    fn test_create_and_list_artifacts() {
        let (mut manager, root) = make_manager();

        let record = manager
            .create_artifact(sample_request("param-1"))
            .expect("create artifact");
        let history = manager.list_artifacts(NodeId(42), "image");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].artifact_id, record.artifact_id);

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_select_and_read_artifact() {
        let (mut manager, root) = make_manager();

        let first = manager
            .create_artifact(sample_request("param-1"))
            .expect("create first artifact");
        let second = manager
            .create_artifact(sample_request("param-2"))
            .expect("create second artifact");

        manager
            .select_artifact(&first.artifact_id)
            .expect("select artifact");

        let selected = manager
            .get_selected_artifact(NodeId(42), "image")
            .expect("selected artifact");
        assert_eq!(selected.artifact_id, first.artifact_id);

        let restored = manager
            .read_artifact(&second.artifact_id)
            .expect("read artifact");
        match restored {
            Value::Image(image) => {
                let cpu = image.cpu_data().expect("cpu image data");
                assert_eq!(cpu.dimensions(), (2, 2));
            }
            other => panic!("expected image, got {other:?}"),
        }

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_resolve_for_restore_uses_selected_artifact() {
        let (mut manager, root) = make_manager();

        let record = manager
            .create_artifact(sample_request("param-1"))
            .expect("create artifact");

        let decision = manager.resolve_for_restore(&ResolveForRestoreRequest {
            node_id: NodeId(42),
            output_key: "image".into(),
            param_signature: "param-1".into(),
            input_signature: "input-1".into(),
        });

        match decision {
            ArtifactRestoreDecision::Restorable(selected) => {
                assert_eq!(selected.artifact_id, record.artifact_id);
            }
            other => panic!("expected Restorable, got {other:?}"),
        }

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_delete_artifact_removes_non_selected_record() {
        let (mut manager, root) = make_manager();

        let first = manager
            .create_artifact(sample_request("param-1"))
            .expect("create first artifact");
        let second = manager
            .create_artifact(sample_request("param-2"))
            .expect("create second artifact");

        manager
            .select_artifact(&second.artifact_id)
            .expect("select second artifact");
        manager
            .delete_artifact(&first.artifact_id)
            .expect("delete first artifact");

        assert!(manager.get_artifact(&first.artifact_id).is_none());
        assert!(manager.get_artifact(&second.artifact_id).is_some());

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_save_and_load_index_roundtrip() {
        let (mut manager, root) = make_manager();

        let record = manager
            .create_artifact(sample_request("param-1"))
            .expect("create artifact");
        manager.save_index().expect("save index");

        let mut restored_manager = ArtifactManager::new(root.clone());
        restored_manager.load_index().expect("load index");

        let restored = restored_manager
            .get_artifact(&record.artifact_id)
            .expect("restored artifact");
        assert_eq!(restored.artifact_id, record.artifact_id);
        assert_eq!(restored.node_id, NodeId(42));
        assert_eq!(restored.output_key, "image");

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_validate_store_reports_missing_file_via_manager() {
        let (mut manager, root) = make_manager();

        let record = manager
            .create_artifact(sample_request("param-1"))
            .expect("create artifact");
        std::fs::remove_file(&record.path).expect("remove artifact file");

        let report = manager.validate_store();

        assert!(!report.ok);
        assert_eq!(report.missing_file_count, 1);

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_cleanup_removes_old_versions_and_persists_index() {
        let (mut manager, root) = make_manager();

        let first = manager
            .create_artifact(sample_request("param-1"))
            .expect("create first artifact");
        let second = manager
            .create_artifact(sample_request("param-2"))
            .expect("create second artifact");

        let report = manager
            .cleanup(&CleanupPolicy {
                keep_latest_per_output: Some(1),
                delete_orphaned: false,
                max_total_bytes: None,
            })
            .expect("cleanup artifacts");

        assert_eq!(report.deleted_artifacts, 1);
        assert!(manager.get_artifact(&first.artifact_id).is_none());
        assert!(manager.get_artifact(&second.artifact_id).is_some());
        assert!(manager.store().index_path().exists());

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
