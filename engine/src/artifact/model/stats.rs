use types::NodeId;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactStats {
    pub total_artifacts: usize,
    pub total_outputs: usize,
    pub total_bytes: u64,
    pub orphaned_artifacts: usize,
    pub selected_outputs: usize,
    pub nodes: Vec<NodeArtifactStats>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeArtifactStats {
    pub node_id: NodeId,
    pub output_count: usize,
    pub artifact_count: usize,
    pub total_bytes: u64,
    pub orphaned_artifacts: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanupReport {
    pub deleted_artifacts: usize,
    pub reclaimed_bytes: u64,
    pub skipped_selected: usize,
    pub skipped_errors: usize,
}
