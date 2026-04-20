use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::{ArtifactKind, CreateArtifactRequest};
use types::{DataType, NodeId, Value};

#[derive(Clone, Debug)]
pub struct ArtifactSinkRequest {
    pub node_id: NodeId,
    pub output_key: String,
    pub value: Value,
    pub param_signature: String,
    pub input_signature: String,
    pub params_snapshot: crate::artifact::model::ArtifactParamsSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactSinkDecision {
    Stored { artifact_id: String },
    SkippedNonPersistable,
}

pub fn persist_if_needed(
    artifact_manager: &mut ArtifactManager,
    request: ArtifactSinkRequest,
) -> Result<ArtifactSinkDecision, crate::artifact::model::ArtifactError> {
    let Some(data_type) = persistable_data_type(&request.value) else {
        return Ok(ArtifactSinkDecision::SkippedNonPersistable);
    };

    let record = artifact_manager.create_artifact(CreateArtifactRequest {
        node_id: request.node_id,
        output_key: request.output_key,
        data_type,
        format: default_format(&request.value).to_string(),
        value: request.value,
        param_signature: request.param_signature,
        input_signature: request.input_signature,
        params_snapshot: request.params_snapshot,
        kind: ArtifactKind::Restorable,
    })?;

    Ok(ArtifactSinkDecision::Stored {
        artifact_id: record.artifact_id,
    })
}

fn persistable_data_type(value: &Value) -> Option<DataType> {
    match value {
        Value::Image(_) => Some(DataType::image()),
        _ => None,
    }
}

fn default_format(value: &Value) -> &'static str {
    match value {
        Value::Image(_) => "png",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, RgbaImage};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    use types::Image;

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

    fn make_manager() -> (ArtifactManager, std::path::PathBuf) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let seq = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("nodeimg-artifact-sink-{unique}-{seq}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        (ArtifactManager::new(root.clone()), root)
    }

    #[test]
    fn persists_image_value_as_restorable_artifact() {
        let (mut manager, root) = make_manager();
        let rgba = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 255, 255]));

        let decision = persist_if_needed(
            &mut manager,
            ArtifactSinkRequest {
                node_id: NodeId(7),
                output_key: "image".into(),
                value: Value::Image(Image::from_cpu(DynamicImage::ImageRgba8(rgba))),
                param_signature: "param-1".into(),
                input_signature: "input-1".into(),
            },
        )
        .expect("persist image");

        match decision {
            ArtifactSinkDecision::Stored { artifact_id } => {
                let record = manager.get_artifact(&artifact_id).expect("stored record");
                assert_eq!(record.node_id, NodeId(7));
                assert_eq!(record.output_key, "image");
            }
            other => panic!("expected stored artifact, got {other:?}"),
        }

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }

    #[test]
    fn skips_non_persistable_values() {
        let (mut manager, root) = make_manager();

        let decision = persist_if_needed(
            &mut manager,
            ArtifactSinkRequest {
                node_id: NodeId(8),
                output_key: "value".into(),
                value: Value::Float(42.0),
                param_signature: "param-1".into(),
                input_signature: "input-1".into(),
            },
        )
        .expect("skip non persistable");

        assert_eq!(decision, ArtifactSinkDecision::SkippedNonPersistable);
        assert!(manager.list_artifacts(NodeId(8), "value").is_empty());

        std::fs::remove_dir_all(root).expect("cleanup temp root");
    }
}
