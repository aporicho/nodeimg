use std::path::PathBuf;

use types::{DataType, NodeId, Value};

use super::record::{ArtifactId, ArtifactKind, OutputKey};

#[derive(Clone, Debug)]
pub struct CreateArtifactRequest {
    pub node_id: NodeId,
    pub output_key: OutputKey,
    pub data_type: DataType,
    pub format: String,
    pub value: Value,
    pub param_signature: String,
    pub input_signature: String,
    pub kind: ArtifactKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListArtifactsRequest {
    pub node_id: NodeId,
    pub output_key: OutputKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetArtifactRequest {
    pub artifact_id: ArtifactId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectArtifactRequest {
    pub artifact_id: ArtifactId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolveForRestoreRequest {
    pub node_id: NodeId,
    pub output_key: OutputKey,
    pub param_signature: String,
    pub input_signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteArtifactRequest {
    pub artifact_id: ArtifactId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportArtifactRequest {
    pub artifact_id: ArtifactId,
    pub target_path: PathBuf,
    pub target_format: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupArtifactsRequest {
    pub keep_latest_per_output: Option<usize>,
    pub delete_orphaned: bool,
}
