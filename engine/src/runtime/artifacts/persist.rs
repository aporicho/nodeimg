use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::{ArtifactError, ArtifactKind, ArtifactRecord, CreateArtifactRequest};

use super::identity::NodeArtifactIdentity;

pub(crate) fn persist_restorable_image_artifact(
    artifact_manager: &mut ArtifactManager,
    node_id: types::NodeId,
    output_key: &str,
    value: types::Value,
    identity: &NodeArtifactIdentity,
) -> Result<ArtifactRecord, ArtifactError> {
    artifact_manager.create_artifact_and_save(CreateArtifactRequest {
        node_id,
        output_key: output_key.to_owned(),
        data_type: types::DataType::image(),
        format: String::from("png"),
        value,
        param_signature: identity.param_signature.clone(),
        input_signature: identity.input_signature.clone(),
        params_snapshot: identity.params_snapshot.clone(),
        kind: ArtifactKind::Restorable,
    })
}
