use std::fs;
use std::path::{Path, PathBuf};

use crate::artifact::persistence::index::ArtifactIndex;
use crate::artifact::persistence::store::ArtifactStore;

pub struct ArtifactValidator;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationIssue {
    MissingFile {
        artifact_id: String,
        path: PathBuf,
        severity: ValidationSeverity,
    },
    OrphanFile {
        path: PathBuf,
        severity: ValidationSeverity,
    },
    CorruptedRecord {
        artifact_id: String,
        path: PathBuf,
        reason: String,
        severity: ValidationSeverity,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub ok: bool,
    pub issues: Vec<ValidationIssue>,
    pub missing_file_count: usize,
    pub orphan_file_count: usize,
    pub corrupted_record_count: usize,
}

impl ArtifactValidator {
    pub fn validate(index: &ArtifactIndex, store: &ArtifactStore) -> ValidationReport {
        let mut issues = Vec::new();
        let all_records = index.all_records();
        let indexed_paths = all_records
            .iter()
            .map(|record| record.path.clone())
            .collect::<std::collections::HashSet<_>>();

        for record in &all_records {
            if !record
                .path
                .starts_with(store.project_root().join("artifacts"))
            {
                issues.push(ValidationIssue::CorruptedRecord {
                    artifact_id: record.artifact_id.clone(),
                    path: record.path.clone(),
                    reason: "artifact path is outside project artifacts directory".into(),
                    severity: ValidationSeverity::Error,
                });
                continue;
            }

            if !store.exists(record) {
                issues.push(ValidationIssue::MissingFile {
                    artifact_id: record.artifact_id.clone(),
                    path: record.path.clone(),
                    severity: ValidationSeverity::Error,
                });
                continue;
            }

            if !record.path.is_file() {
                issues.push(ValidationIssue::CorruptedRecord {
                    artifact_id: record.artifact_id.clone(),
                    path: record.path.clone(),
                    reason: "artifact path is not a regular file".into(),
                    severity: ValidationSeverity::Error,
                });
                continue;
            }

            match store.file_size(record) {
                Ok(0) => issues.push(ValidationIssue::CorruptedRecord {
                    artifact_id: record.artifact_id.clone(),
                    path: record.path.clone(),
                    reason: "artifact file is empty".into(),
                    severity: ValidationSeverity::Error,
                }),
                Ok(_) => {}
                Err(err) => issues.push(ValidationIssue::CorruptedRecord {
                    artifact_id: record.artifact_id.clone(),
                    path: record.path.clone(),
                    reason: err.to_string(),
                    severity: ValidationSeverity::Error,
                }),
            }
        }

        let artifacts_root = store.project_root().join("artifacts");
        for path in collect_files(&artifacts_root) {
            if !indexed_paths.contains(&path) {
                issues.push(ValidationIssue::OrphanFile {
                    path,
                    severity: ValidationSeverity::Warning,
                });
            }
        }

        ValidationReport {
            ok: issues.is_empty(),
            missing_file_count: issues
                .iter()
                .filter(|issue| matches!(issue, ValidationIssue::MissingFile { .. }))
                .count(),
            orphan_file_count: issues
                .iter()
                .filter(|issue| matches!(issue, ValidationIssue::OrphanFile { .. }))
                .count(),
            corrupted_record_count: issues
                .iter()
                .filter(|issue| matches!(issue, ValidationIssue::CorruptedRecord { .. }))
                .count(),
            issues,
        }
    }
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }

    let mut files = Vec::new();
    collect_files_recursive(root, &mut files);
    files
}

fn collect_files_recursive(root: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, files);
        } else if path.is_file() {
            files.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use image::{DynamicImage, RgbaImage};
    use types::{DataType, NodeId, Value};

    use crate::artifact::handler::image::ImageArtifactHandler;
    use crate::artifact::model::{ArtifactKind, ArtifactRecord};
    use crate::artifact::persistence::index::ArtifactIndex;
    use crate::artifact::persistence::store::ArtifactStore;

    use super::{ArtifactValidator, ValidationIssue};

    fn make_temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nodeimg-artifact-validator-{unique}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn sample_record(path: PathBuf, artifact_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.into(),
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

    fn write_test_image(store: &ArtifactStore, record: &ArtifactRecord) {
        let handler = ImageArtifactHandler::new();
        let rgba = RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 0, 255]));
        let value = Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba)));
        store
            .write(record, &value, &handler)
            .expect("write test image");
    }

    #[test]
    fn test_validate_reports_ok_when_index_matches_disk() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let path = store.artifact_path(NodeId(42), "image", 1, "png");
        let record = sample_record(path, "art_001");
        write_test_image(&store, &record);

        let mut index = ArtifactIndex::new();
        index.upsert(record, true);

        let report = ArtifactValidator::validate(&index, &store);
        assert!(report.ok);
        assert!(report.issues.is_empty());

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_validate_reports_missing_file() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let path = store.artifact_path(NodeId(42), "image", 1, "png");
        let record = sample_record(path, "art_001");

        let mut index = ArtifactIndex::new();
        index.upsert(record, true);

        let report = ArtifactValidator::validate(&index, &store);
        assert_eq!(report.missing_file_count, 1);
        assert!(report
            .issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::MissingFile { .. })));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_validate_reports_orphan_file() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let orphan_path = store.artifact_path(NodeId(42), "image", 1, "png");
        let orphan_record = sample_record(orphan_path, "art_orphan");
        write_test_image(&store, &orphan_record);

        let index = ArtifactIndex::new();
        let report = ArtifactValidator::validate(&index, &store);

        assert_eq!(report.orphan_file_count, 1);
        assert!(report
            .issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::OrphanFile { .. })));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_validate_reports_corrupted_record_for_empty_file() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let path = store.artifact_path(NodeId(42), "image", 1, "png");
        store.ensure_parent_dir(&path).expect("ensure parent dir");
        std::fs::write(&path, []).expect("write empty file");

        let record = sample_record(path, "art_001");
        let mut index = ArtifactIndex::new();
        index.upsert(record, true);

        let report = ArtifactValidator::validate(&index, &store);
        assert_eq!(report.corrupted_record_count, 1);
        assert!(report
            .issues
            .iter()
            .any(|issue| matches!(issue, ValidationIssue::CorruptedRecord { .. })));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
