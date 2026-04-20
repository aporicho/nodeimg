use std::collections::HashMap;

use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::{ArtifactError, ArtifactKind, ArtifactRecord, CreateArtifactRequest};
use crate::node_manager::NodeDef;
use types::Value;

use super::identity::NodeArtifactIdentity;
use super::policy::{carrier_kind, ArtifactCarrierKind};

pub(crate) fn persist_artifact_output(
    artifact_manager: &mut ArtifactManager,
    def: &NodeDef,
    node_id: types::NodeId,
    outputs: &HashMap<String, Value>,
    identity: &NodeArtifactIdentity,
) -> Result<Option<ArtifactRecord>, ArtifactError> {
    match carrier_kind(def) {
        Some(ArtifactCarrierKind::ImageGeneration) => persist(
            artifact_manager,
            node_id,
            "image",
            types::DataType::image(),
            String::from("png"),
            outputs.get("image").cloned(),
            identity,
        )
        .map(Some),
        Some(ArtifactCarrierKind::VideoGeneration) => persist(
            artifact_manager,
            node_id,
            "video",
            types::DataType::video(),
            String::from("mp4"),
            outputs.get("video").cloned(),
            identity,
        )
        .map(Some),
        None => Ok(None),
    }
}

fn persist(
    artifact_manager: &mut ArtifactManager,
    node_id: types::NodeId,
    output_key: &str,
    data_type: types::DataType,
    format: String,
    value: Option<Value>,
    identity: &NodeArtifactIdentity,
) -> Result<ArtifactRecord, ArtifactError> {
    let value = value.ok_or_else(|| ArtifactError::SerializationFailed {
        message: format!("missing artifact carrier output '{output_key}'"),
    })?;

    artifact_manager.create_artifact_and_save(CreateArtifactRequest {
        node_id,
        output_key: output_key.to_owned(),
        data_type,
        format,
        value,
        param_signature: identity.param_signature.clone(),
        input_signature: identity.input_signature.clone(),
        params_snapshot: identity.params_snapshot.clone(),
        kind: ArtifactKind::Restorable,
    })
}
