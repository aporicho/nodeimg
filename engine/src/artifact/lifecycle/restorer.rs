use types::NodeId;

use crate::artifact::model::{ArtifactKind, ArtifactRecord};
use crate::artifact::persistence::index::ArtifactIndex;

pub struct ArtifactRestorer;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactRestoreDecision<'a> {
    Restorable(&'a ArtifactRecord),
    NoSelection,
    SignatureMismatch { artifact_id: String },
    NotRestorableKind { artifact_id: String },
    Orphaned { artifact_id: String },
}

impl ArtifactRestorer {
    pub fn resolve<'a>(
        index: &'a ArtifactIndex,
        node_id: NodeId,
        output_key: &str,
        param_signature: &str,
        input_signature: &str,
    ) -> ArtifactRestoreDecision<'a> {
        let Some(record) = index.get_selected(node_id, output_key) else {
            return ArtifactRestoreDecision::NoSelection;
        };

        if record.kind != ArtifactKind::Restorable {
            return ArtifactRestoreDecision::NotRestorableKind {
                artifact_id: record.artifact_id.clone(),
            };
        }

        if record.orphaned {
            return ArtifactRestoreDecision::Orphaned {
                artifact_id: record.artifact_id.clone(),
            };
        }

        if record.param_signature != param_signature || record.input_signature != input_signature {
            return ArtifactRestoreDecision::SignatureMismatch {
                artifact_id: record.artifact_id.clone(),
            };
        }

        ArtifactRestoreDecision::Restorable(record)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use types::{DataType, NodeId};

    use crate::artifact::model::{ArtifactKind, ArtifactRecord};
    use crate::artifact::persistence::index::ArtifactIndex;

    use super::{ArtifactRestoreDecision, ArtifactRestorer};

    fn sample_record(artifact_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.into(),
            node_id: NodeId(42),
            output_key: "image".into(),
            version: 1,
            created_at: UNIX_EPOCH + Duration::from_secs(1),
            data_type: DataType::image(),
            format: "png".into(),
            path: PathBuf::from("artifacts/42/image/000001.png"),
            param_signature: "param-1".into(),
            input_signature: "input-1".into(),
            kind: ArtifactKind::Restorable,
            orphaned: false,
        }
    }

    #[test]
    fn test_resolve_returns_no_selection_when_selected_missing() {
        let index = ArtifactIndex::new();

        let decision = ArtifactRestorer::resolve(&index, NodeId(42), "image", "param-1", "input-1");

        assert!(matches!(decision, ArtifactRestoreDecision::NoSelection));
    }

    #[test]
    fn test_resolve_rejects_exported_artifact() {
        let mut index = ArtifactIndex::new();
        let mut record = sample_record("art_001");
        record.kind = ArtifactKind::Exported;
        index.upsert(record, true);

        let decision = ArtifactRestorer::resolve(&index, NodeId(42), "image", "param-1", "input-1");

        assert!(matches!(
            decision,
            ArtifactRestoreDecision::NotRestorableKind { .. }
        ));
    }

    #[test]
    fn test_resolve_rejects_orphaned_artifact() {
        let mut index = ArtifactIndex::new();
        let mut record = sample_record("art_001");
        record.orphaned = true;
        index.upsert(record, true);

        let decision = ArtifactRestorer::resolve(&index, NodeId(42), "image", "param-1", "input-1");

        assert!(matches!(decision, ArtifactRestoreDecision::Orphaned { .. }));
    }

    #[test]
    fn test_resolve_rejects_signature_mismatch() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record("art_001"), true);

        let decision = ArtifactRestorer::resolve(&index, NodeId(42), "image", "param-2", "input-1");

        assert!(matches!(
            decision,
            ArtifactRestoreDecision::SignatureMismatch { .. }
        ));
    }

    #[test]
    fn test_resolve_returns_restorable_when_signatures_match() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record("art_001"), true);

        let decision = ArtifactRestorer::resolve(&index, NodeId(42), "image", "param-1", "input-1");

        match decision {
            ArtifactRestoreDecision::Restorable(record) => {
                assert_eq!(record.artifact_id, "art_001");
            }
            other => panic!("expected Restorable, got {other:?}"),
        }
    }
}
