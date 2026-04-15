use std::collections::HashMap;
use std::path::Path;

use crate::artifact::lifecycle::restorer::ArtifactRestoreDecision;
use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::{ArtifactError, ResolveForRestoreRequest};
use crate::execution::{CookingContext, DimensionValue};
use crate::executors::video::decode_video_frames;
use crate::node_manager::NodeDef;
use types::Value;

use super::identity::NodeArtifactIdentity;
use super::policy::{carrier_kind, ArtifactCarrierKind};

pub(crate) fn restore_artifact_outputs(
    artifact_manager: &ArtifactManager,
    def: &NodeDef,
    node_id: types::NodeId,
    identity: &NodeArtifactIdentity,
    cooking_context: &CookingContext,
) -> Result<Option<HashMap<String, Value>>, ArtifactError> {
    match carrier_kind(def) {
        Some(ArtifactCarrierKind::ImageGeneration) => {
            let restored = restore_selected_artifact(artifact_manager, node_id, "image", identity)?;
            Ok(restored.map(|value| HashMap::from([(String::from("image"), value)])))
        }
        Some(ArtifactCarrierKind::VideoGeneration) => {
            let restored = restore_selected_artifact(artifact_manager, node_id, "video", identity)?;
            match restored {
                Some(Value::String(path)) => {
                    let mut frames = decode_video_frames(Path::new(&path)).map_err(|error| {
                        ArtifactError::DeserializationFailed {
                            message: error.to_string(),
                        }
                    })?;
                    let frame_index = current_frame_index(cooking_context);
                    let preview_frame = if frame_index < frames.len() {
                        frames.remove(frame_index)
                    } else {
                        frames
                            .pop()
                            .ok_or_else(|| ArtifactError::DeserializationFailed {
                                message: format!(
                                    "restored video '{path}' did not contain any frames"
                                ),
                            })?
                    };
                    let fps = identity
                        .params_snapshot
                        .get("fps")
                        .and_then(|fps| fps.parse::<i64>().ok())
                        .ok_or_else(|| ArtifactError::DeserializationFailed {
                            message: String::from("missing fps in artifact params snapshot"),
                        })?;
                    Ok(Some(HashMap::from([
                        (String::from("video"), Value::String(path)),
                        (String::from("fps"), Value::Int(fps)),
                        (
                            String::from("image"),
                            Value::Image(types::Image::from_cpu(preview_frame)),
                        ),
                    ])))
                }
                Some(other) => Err(ArtifactError::DeserializationFailed {
                    message: format!("expected video artifact path, got {other:?}"),
                }),
                None => Ok(None),
            }
        }
        None => Ok(None),
    }
}

fn current_frame_index(context: &CookingContext) -> usize {
    match context.dimensions.get("frame") {
        Some(DimensionValue::Int(value)) if *value >= 0 => *value as usize,
        _ => 0,
    }
}

fn restore_selected_artifact(
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
