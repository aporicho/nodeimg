use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactIndexFile {
    pub version: String,
    pub artifacts: Vec<ArtifactEntryFile>,
}

impl Default for ArtifactIndexFile {
    fn default() -> Self {
        Self {
            version: "1.0".into(),
            artifacts: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactEntryFile {
    pub node_id: u64,
    pub output_key: String,
    pub selected_artifact_id: Option<String>,
    pub orphaned: bool,
    pub history: Vec<ArtifactRecordFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactRecordFile {
    pub artifact_id: String,
    pub node_id: u64,
    pub output_key: String,
    pub version: u64,
    pub created_at_unix_ms: u128,
    pub data_type: String,
    pub format: String,
    pub path: String,
    pub param_signature: String,
    pub input_signature: String,
    pub kind: ArtifactHistoryFile,
    pub orphaned: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactHistoryFile {
    Restorable,
    Exported,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_index_file_is_empty() {
        let index = ArtifactIndexFile::default();
        assert_eq!(index.version, "1.0");
        assert!(index.artifacts.is_empty());
    }

    #[test]
    fn test_index_file_roundtrip_json() {
        let index = ArtifactIndexFile {
            version: "1.0".into(),
            artifacts: vec![ArtifactEntryFile {
                node_id: 42,
                output_key: "image".into(),
                selected_artifact_id: Some("art_001".into()),
                orphaned: false,
                history: vec![ArtifactRecordFile {
                    artifact_id: "art_001".into(),
                    node_id: 42,
                    output_key: "image".into(),
                    version: 1,
                    created_at_unix_ms: 1_744_000_000_000,
                    data_type: "image".into(),
                    format: "png".into(),
                    path: "artifacts/42/image/000001.png".into(),
                    param_signature: "sha256:param".into(),
                    input_signature: "sha256:input".into(),
                    kind: ArtifactHistoryFile::Restorable,
                    orphaned: false,
                }],
            }],
        };

        let json = serde_json::to_string_pretty(&index).expect("serialize index file");
        let decoded: ArtifactIndexFile =
            serde_json::from_str(&json).expect("deserialize index file");

        assert_eq!(decoded, index);
    }
}
