use types::NodeId;

use crate::artifact::model::{ArtifactError, CleanupReport};
use crate::artifact::persistence::index::ArtifactIndex;
use crate::artifact::persistence::store::ArtifactStore;

pub struct ArtifactCleaner;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupPolicy {
    pub keep_latest_per_output: Option<usize>,
    pub delete_orphaned: bool,
    pub max_total_bytes: Option<u64>,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            keep_latest_per_output: None,
            delete_orphaned: false,
            max_total_bytes: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CleanupReason {
    OldVersion,
    Orphaned,
    OverBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupCandidate {
    pub artifact_id: String,
    pub node_id: NodeId,
    pub output_key: String,
    pub version: u64,
    pub reason: CleanupReason,
    pub estimated_bytes: Option<u64>,
}

impl ArtifactCleaner {
    pub fn plan_cleanup(
        index: &ArtifactIndex,
        store: &ArtifactStore,
        policy: &CleanupPolicy,
    ) -> Vec<CleanupCandidate> {
        let mut candidates = Vec::new();

        for (node_id, output_key, state) in index.all_outputs() {
            let mut history = state.history.clone();
            history.sort_by_key(|record| record.version);

            if policy.delete_orphaned {
                for record in &history {
                    if record.orphaned && !index.is_selected(&record.artifact_id) {
                        candidates.push(CleanupCandidate {
                            artifact_id: record.artifact_id.clone(),
                            node_id,
                            output_key: output_key.clone(),
                            version: record.version,
                            reason: CleanupReason::Orphaned,
                            estimated_bytes: store.file_size(record).ok(),
                        });
                    }
                }
            }

            if let Some(keep_latest) = policy.keep_latest_per_output {
                if history.len() > keep_latest {
                    let to_remove = history.len() - keep_latest;
                    for record in history.iter().take(to_remove) {
                        if index.is_selected(&record.artifact_id) {
                            continue;
                        }
                        if candidates
                            .iter()
                            .any(|c| c.artifact_id == record.artifact_id)
                        {
                            continue;
                        }

                        candidates.push(CleanupCandidate {
                            artifact_id: record.artifact_id.clone(),
                            node_id,
                            output_key: output_key.clone(),
                            version: record.version,
                            reason: CleanupReason::OldVersion,
                            estimated_bytes: store.file_size(record).ok(),
                        });
                    }
                }
            }
        }

        candidates.sort_by(|a, b| {
            a.node_id
                .0
                .cmp(&b.node_id.0)
                .then_with(|| a.output_key.cmp(&b.output_key))
                .then_with(|| a.version.cmp(&b.version))
        });
        candidates
    }

    pub fn execute_cleanup(
        index: &mut ArtifactIndex,
        store: &ArtifactStore,
        policy: &CleanupPolicy,
    ) -> Result<CleanupReport, ArtifactError> {
        let candidates = Self::plan_cleanup(index, store, policy);
        let mut report = CleanupReport::default();

        for candidate in candidates {
            if index.is_selected(&candidate.artifact_id) {
                report.skipped_selected += 1;
                continue;
            }

            let Some(record) = index.get(&candidate.artifact_id).cloned() else {
                report.skipped_errors += 1;
                continue;
            };

            let estimated_bytes = store.file_size(&record).unwrap_or(0);
            match store.delete(&record) {
                Ok(()) => {
                    index.remove(&candidate.artifact_id);
                    report.deleted_artifacts += 1;
                    report.reclaimed_bytes += estimated_bytes;
                }
                Err(_) => {
                    report.skipped_errors += 1;
                }
            }
        }

        Ok(report)
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

    use super::{ArtifactCleaner, CleanupPolicy, CleanupReason};

    fn make_temp_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nodeimg-artifact-cleaner-{unique}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn record(store: &ArtifactStore, version: u64, artifact_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.into(),
            node_id: NodeId(42),
            output_key: "image".into(),
            version,
            created_at: UNIX_EPOCH + Duration::from_secs(version),
            data_type: DataType::image(),
            format: "png".into(),
            path: store.artifact_path(NodeId(42), "image", version, "png"),
            param_signature: format!("param-{version}"),
            input_signature: format!("input-{version}"),
            kind: ArtifactKind::Restorable,
            orphaned: false,
        }
    }

    fn write_image(store: &ArtifactStore, record: &ArtifactRecord) {
        let handler = ImageArtifactHandler::new();
        let rgba = RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 255, 255]));
        let value = Value::Image(types::Image::from_cpu(DynamicImage::ImageRgba8(rgba)));
        store.write(record, &value, &handler).expect("write image");
    }

    #[test]
    fn test_plan_cleanup_keeps_latest_versions() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let mut index = ArtifactIndex::new();

        let r1 = record(&store, 1, "art_001");
        let r2 = record(&store, 2, "art_002");
        let r3 = record(&store, 3, "art_003");
        write_image(&store, &r1);
        write_image(&store, &r2);
        write_image(&store, &r3);

        index.upsert(r1.clone(), false);
        index.upsert(r2.clone(), false);
        index.upsert(r3.clone(), true);

        let candidates = ArtifactCleaner::plan_cleanup(
            &index,
            &store,
            &CleanupPolicy {
                keep_latest_per_output: Some(1),
                delete_orphaned: false,
                max_total_bytes: None,
            },
        );

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].reason, CleanupReason::OldVersion);
        assert_eq!(candidates[0].artifact_id, "art_001");
        assert_eq!(candidates[1].artifact_id, "art_002");

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_plan_cleanup_skips_selected_artifact() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let mut index = ArtifactIndex::new();

        let r1 = record(&store, 1, "art_001");
        let r2 = record(&store, 2, "art_002");
        write_image(&store, &r1);
        write_image(&store, &r2);

        index.upsert(r1.clone(), true);
        index.upsert(r2.clone(), false);

        let candidates = ArtifactCleaner::plan_cleanup(
            &index,
            &store,
            &CleanupPolicy {
                keep_latest_per_output: Some(0),
                delete_orphaned: false,
                max_total_bytes: None,
            },
        );

        assert!(candidates.iter().all(|c| c.artifact_id != "art_001"));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_execute_cleanup_removes_old_versions() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let mut index = ArtifactIndex::new();

        let r1 = record(&store, 1, "art_001");
        let r2 = record(&store, 2, "art_002");
        write_image(&store, &r1);
        write_image(&store, &r2);

        index.upsert(r1.clone(), false);
        index.upsert(r2.clone(), true);

        let report = ArtifactCleaner::execute_cleanup(
            &mut index,
            &store,
            &CleanupPolicy {
                keep_latest_per_output: Some(1),
                delete_orphaned: false,
                max_total_bytes: None,
            },
        )
        .expect("execute cleanup");

        assert_eq!(report.deleted_artifacts, 1);
        assert!(index.get("art_001").is_none());
        assert!(!store.exists(&r1));
        assert!(store.exists(&r2));

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn test_plan_cleanup_includes_orphaned_when_enabled() {
        let root = make_temp_root();
        let store = ArtifactStore::new(root.clone());
        let mut index = ArtifactIndex::new();

        let mut r1 = record(&store, 1, "art_001");
        r1.orphaned = true;
        let r2 = record(&store, 2, "art_002");
        write_image(&store, &r1);
        write_image(&store, &r2);
        index.upsert(r1, false);
        index.upsert(r2, true);

        let candidates = ArtifactCleaner::plan_cleanup(
            &index,
            &store,
            &CleanupPolicy {
                keep_latest_per_output: None,
                delete_orphaned: true,
                max_total_bytes: None,
            },
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].reason, CleanupReason::Orphaned);

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
