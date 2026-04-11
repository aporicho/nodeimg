use std::path::PathBuf;
use std::time::SystemTime;

use types::{DataType, NodeId};

pub type ArtifactId = String;
pub type OutputKey = String;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    Restorable,
    Exported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_id: ArtifactId,
    pub node_id: NodeId,
    pub output_key: OutputKey,
    pub version: u64,
    pub created_at: SystemTime,
    pub data_type: DataType,
    pub format: String,
    pub path: PathBuf,
    pub param_signature: String,
    pub input_signature: String,
    pub kind: ArtifactKind,
    pub orphaned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedArtifact {
    pub node_id: NodeId,
    pub output_key: OutputKey,
    pub artifact_id: ArtifactId,
}
