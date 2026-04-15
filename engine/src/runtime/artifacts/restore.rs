use crate::artifact::lifecycle::restorer::ArtifactRestoreDecision;
use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::{ArtifactError, ResolveForRestoreRequest};

use super::identity::NodeArtifactIdentity;

pub(crate) fn restore_selected_artifact(
    artifact_manager: &ArtifactManager,
    node_id: types::NodeId,
    output_key: &str,
    identity: &NodeArtifactIdentity,
) -> Result<Option<types::Value>, ArtifactError> {
    let restore_request = ResolveForRestoreRequest {
        node_id,
        output_key: output_key.to_owned(),
        param_signature: identity.param_signature.clone(),
        input_signature: identity.input_signature.clone(),
    };

    match artifact_manager.resolve_for_restore(&restore_request) {
        ArtifactRestoreDecision::Restorable(record) => {
            Ok(Some(artifact_manager.read_artifact(&record.artifact_id)?))
        }
        ArtifactRestoreDecision::NoSelection
        | ArtifactRestoreDecision::SignatureMismatch { .. }
        | ArtifactRestoreDecision::NotRestorableKind { .. }
        | ArtifactRestoreDecision::Orphaned { .. } => Ok(None),
    }
}
