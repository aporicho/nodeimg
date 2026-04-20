use crate::execution::EvaluationFidelity;
use crate::node_manager::{ArtifactPolicy, NodeDef, Purity};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArtifactCarrierKind {
    ImageGeneration,
    VideoGeneration,
}

pub(crate) fn carrier_kind(def: &NodeDef) -> Option<ArtifactCarrierKind> {
    match def.type_id.as_str() {
        "image_gen" => Some(ArtifactCarrierKind::ImageGeneration),
        "ai_video_generate_api" => Some(ArtifactCarrierKind::VideoGeneration),
        _ => None,
    }
}

pub(crate) fn should_restore_artifact(def: &NodeDef, fidelity: &EvaluationFidelity) -> bool {
    matches!(fidelity, EvaluationFidelity::Full)
        && effective_artifact_policy(def) == ArtifactPolicy::Persist
        && carrier_kind(def).is_some()
}

pub(crate) fn should_persist_artifact(def: &NodeDef, fidelity: &EvaluationFidelity) -> bool {
    matches!(fidelity, EvaluationFidelity::Full)
        && effective_artifact_policy(def) == ArtifactPolicy::Persist
        && carrier_kind(def).is_some()
}

fn effective_artifact_policy(def: &NodeDef) -> ArtifactPolicy {
    def.execution.artifact_policy.unwrap_or(match def.purity {
        Purity::Pure => ArtifactPolicy::None,
        Purity::Impure => ArtifactPolicy::Persist,
    })
}
