use types::NodeId;

use crate::artifact::model::{ArtifactError, ArtifactRecord};
use crate::artifact::persistence::index::ArtifactIndex;

pub struct ArtifactSelector;

impl ArtifactSelector {
    pub fn get_selected<'a>(
        index: &'a ArtifactIndex,
        node_id: NodeId,
        output_key: &str,
    ) -> Result<&'a ArtifactRecord, ArtifactError> {
        index.get_selected(node_id, output_key).ok_or_else(|| {
            ArtifactError::SelectedArtifactNotFound {
                artifact_id: format!("{}:{}", node_id.0, output_key),
            }
        })
    }

    pub fn select(index: &mut ArtifactIndex, artifact_id: &str) -> Result<(), ArtifactError> {
        index.select(artifact_id)
    }

    pub fn ensure_not_selected(
        index: &ArtifactIndex,
        artifact_id: &str,
    ) -> Result<(), ArtifactError> {
        if index.is_selected(artifact_id) {
            return Err(ArtifactError::SelectedArtifactDeleteForbidden {
                artifact_id: artifact_id.to_owned(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, UNIX_EPOCH};

    use types::{DataType, NodeId};

    use crate::artifact::model::{ArtifactError, ArtifactKind, ArtifactRecord};
    use crate::artifact::persistence::index::ArtifactIndex;

    use super::ArtifactSelector;

    fn sample_record(version: u64, artifact_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.into(),
            node_id: NodeId(42),
            output_key: "image".into(),
            version,
            created_at: UNIX_EPOCH + Duration::from_secs(version),
            data_type: DataType::image(),
            format: "png".into(),
            path: PathBuf::from(format!("artifacts/42/image/{version:06}.png")),
            param_signature: format!("param-{version}"),
            input_signature: format!("input-{version}"),
            kind: ArtifactKind::Restorable,
            orphaned: false,
        }
    }

    #[test]
    fn test_get_selected_returns_selected_record() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record(1, "art_001"), true);

        let selected =
            ArtifactSelector::get_selected(&index, NodeId(42), "image").expect("selected artifact");

        assert_eq!(selected.artifact_id, "art_001");
    }

    #[test]
    fn test_select_switches_selected_record() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record(1, "art_001"), true);
        index.upsert(sample_record(2, "art_002"), false);

        ArtifactSelector::select(&mut index, "art_002").expect("select artifact");

        let selected =
            ArtifactSelector::get_selected(&index, NodeId(42), "image").expect("selected artifact");
        assert_eq!(selected.artifact_id, "art_002");
    }

    #[test]
    fn test_ensure_not_selected_rejects_current_artifact() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record(1, "art_001"), true);

        let err = ArtifactSelector::ensure_not_selected(&index, "art_001")
            .expect_err("selected artifact should be protected");

        assert!(matches!(
            err,
            ArtifactError::SelectedArtifactDeleteForbidden { .. }
        ));
    }
}
