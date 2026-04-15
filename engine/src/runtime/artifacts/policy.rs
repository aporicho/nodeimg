use crate::execution::EvaluationFidelity;
use crate::node_manager::PinDef;
use crate::node_manager::{ArtifactPolicy, NodeDef, Purity};
use types::DataType;

pub(crate) fn should_restore_artifact(
    def: &NodeDef,
    output: &PinDef,
    fidelity: &EvaluationFidelity,
) -> bool {
    matches!(fidelity, EvaluationFidelity::Full)
        && effective_artifact_policy(def) == ArtifactPolicy::Persist
        && is_restorable_output(output)
}

pub(crate) fn should_persist_artifact(
    def: &NodeDef,
    output: &PinDef,
    fidelity: &EvaluationFidelity,
) -> bool {
    matches!(fidelity, EvaluationFidelity::Full)
        && effective_artifact_policy(def) == ArtifactPolicy::Persist
        && is_restorable_output(output)
}

pub(crate) fn is_restorable_output(output: &PinDef) -> bool {
    output.data_type == DataType::image()
}

fn effective_artifact_policy(def: &NodeDef) -> ArtifactPolicy {
    def.execution.artifact_policy.unwrap_or(match def.purity {
        Purity::Pure => ArtifactPolicy::None,
        Purity::Impure => ArtifactPolicy::Persist,
    })
}
