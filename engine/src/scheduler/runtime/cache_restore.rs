use crate::artifact::lifecycle::restorer::ArtifactRestoreDecision;
use crate::artifact::manager::ArtifactManager;
use crate::artifact::model::ResolveForRestoreRequest;
use crate::cache::manager::CacheManager;
use crate::cache::result_store::WriteResult;
use crate::scheduler::model::RunId;
use types::NodeId;

use crate::cache::model::{ExecSignature, GenerationId};

#[derive(Clone, Debug)]
pub enum CacheRestoreResult {
    Restored {
        artifact_id: String,
        write_result: WriteResult,
    },
    NoSelection,
    SignatureMismatch {
        artifact_id: String,
    },
    NotRestorableKind {
        artifact_id: String,
    },
    Orphaned {
        artifact_id: String,
    },
}

#[derive(Debug)]
pub enum CacheRestoreError {
    Artifact(crate::artifact::model::ArtifactError),
    Cache(crate::cache::model::CacheError),
}

impl std::fmt::Display for CacheRestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => write!(f, "artifact restore failed: {error}"),
            Self::Cache(error) => write!(f, "cache restore failed: {error:?}"),
        }
    }
}

impl std::error::Error for CacheRestoreError {}

impl From<crate::artifact::model::ArtifactError> for CacheRestoreError {
    fn from(value: crate::artifact::model::ArtifactError) -> Self {
        Self::Artifact(value)
    }
}

impl From<crate::cache::model::CacheError> for CacheRestoreError {
    fn from(value: crate::cache::model::CacheError) -> Self {
        Self::Cache(value)
    }
}

#[derive(Clone, Debug)]
pub struct CacheRestoreRequest {
    pub run_id: RunId,
    pub node_id: NodeId,
    pub output_key: String,
    pub exec_signature: ExecSignature,
    pub generation: GenerationId,
    pub param_signature: String,
    pub input_signature: String,
}

pub fn restore_from_artifact(
    cache: &CacheManager,
    artifact: &ArtifactManager,
    request: &CacheRestoreRequest,
) -> Result<CacheRestoreResult, CacheRestoreError> {
    let restore_request = ResolveForRestoreRequest {
        node_id: request.node_id,
        output_key: request.output_key.clone(),
        param_signature: request.param_signature.clone(),
        input_signature: request.input_signature.clone(),
    };

    let decision = artifact.resolve_for_restore(&restore_request);
    match decision {
        ArtifactRestoreDecision::Restorable(record) => {
            let value = artifact.read_artifact(&record.artifact_id)?;
            let write_result = cache.put_result(
                request.node_id,
                &request.output_key,
                request.exec_signature,
                request.generation,
                value,
            )?;

            Ok(CacheRestoreResult::Restored {
                artifact_id: record.artifact_id.clone(),
                write_result,
            })
        }
        ArtifactRestoreDecision::NoSelection => Ok(CacheRestoreResult::NoSelection),
        ArtifactRestoreDecision::SignatureMismatch { artifact_id } => {
            Ok(CacheRestoreResult::SignatureMismatch {
                artifact_id: artifact_id.clone(),
            })
        }
        ArtifactRestoreDecision::NotRestorableKind { artifact_id } => {
            Ok(CacheRestoreResult::NotRestorableKind {
                artifact_id: artifact_id.clone(),
            })
        }
        ArtifactRestoreDecision::Orphaned { artifact_id } => Ok(CacheRestoreResult::Orphaned {
            artifact_id: artifact_id.clone(),
        }),
    }
}
