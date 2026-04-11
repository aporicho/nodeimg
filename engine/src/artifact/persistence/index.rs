use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use types::{DataType, NodeId};

use crate::artifact::model::{ArtifactError, ArtifactId, ArtifactKind, ArtifactRecord, OutputKey};

use super::index_file::{
    ArtifactEntryFile, ArtifactHistoryFile, ArtifactIndexFile, ArtifactRecordFile,
};

type OutputIndexKey = (NodeId, OutputKey);

#[derive(Clone, Debug, Default)]
pub struct ArtifactIndex {
    entries: HashMap<OutputIndexKey, OutputArtifactState>,
}

#[derive(Clone, Debug, Default)]
pub struct OutputArtifactState {
    pub selected_artifact_id: Option<ArtifactId>,
    pub orphaned: bool,
    pub history: Vec<ArtifactRecord>,
}

impl ArtifactIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn list(&self, node_id: NodeId, output_key: &str) -> Vec<ArtifactRecord> {
        self.entries
            .get(&(node_id, output_key.to_owned()))
            .map(|state| state.history.clone())
            .unwrap_or_default()
    }

    pub fn all_records(&self) -> Vec<ArtifactRecord> {
        self.entries
            .values()
            .flat_map(|state| state.history.iter().cloned())
            .collect()
    }

    pub fn all_outputs(&self) -> Vec<(NodeId, OutputKey, OutputArtifactState)> {
        self.entries
            .iter()
            .map(|((node_id, output_key), state)| (*node_id, output_key.clone(), state.clone()))
            .collect()
    }

    pub fn get(&self, artifact_id: &str) -> Option<&ArtifactRecord> {
        self.entries
            .values()
            .flat_map(|state| state.history.iter())
            .find(|record| record.artifact_id == artifact_id)
    }

    pub fn get_selected(&self, node_id: NodeId, output_key: &str) -> Option<&ArtifactRecord> {
        let state = self.entries.get(&(node_id, output_key.to_owned()))?;
        let artifact_id = state.selected_artifact_id.as_ref()?;
        state
            .history
            .iter()
            .find(|record| &record.artifact_id == artifact_id)
    }

    pub fn is_selected(&self, artifact_id: &str) -> bool {
        self.entries
            .values()
            .any(|state| state.selected_artifact_id.as_deref() == Some(artifact_id))
    }

    pub fn allocate_version(&self, node_id: NodeId, output_key: &str) -> u64 {
        self.entries
            .get(&(node_id, output_key.to_owned()))
            .and_then(|state| state.history.iter().map(|record| record.version).max())
            .unwrap_or(0)
            + 1
    }

    pub fn upsert(&mut self, record: ArtifactRecord, select_as_current: bool) {
        let key = (record.node_id, record.output_key.clone());
        let state = self.entries.entry(key).or_default();

        if let Some(existing) = state
            .history
            .iter_mut()
            .find(|existing| existing.artifact_id == record.artifact_id)
        {
            *existing = record.clone();
        } else {
            state.history.push(record.clone());
            state.history.sort_by_key(|item| item.version);
        }

        state.orphaned = state.orphaned || record.orphaned;

        if select_as_current || state.selected_artifact_id.is_none() {
            state.selected_artifact_id = Some(record.artifact_id);
        }
    }

    pub fn select(&mut self, artifact_id: &str) -> Result<(), ArtifactError> {
        for state in self.entries.values_mut() {
            if state
                .history
                .iter()
                .any(|record| record.artifact_id == artifact_id)
            {
                state.selected_artifact_id = Some(artifact_id.to_owned());
                return Ok(());
            }
        }

        Err(ArtifactError::ArtifactNotFound {
            artifact_id: artifact_id.to_owned(),
        })
    }

    pub fn remove(&mut self, artifact_id: &str) -> Option<ArtifactRecord> {
        let mut empty_keys = Vec::new();
        let mut removed = None;

        for (key, state) in &mut self.entries {
            if let Some(pos) = state
                .history
                .iter()
                .position(|record| record.artifact_id == artifact_id)
            {
                removed = Some(state.history.remove(pos));
                if state.selected_artifact_id.as_deref() == Some(artifact_id) {
                    state.selected_artifact_id = None;
                }
                if state.history.is_empty() {
                    empty_keys.push(key.clone());
                }
                break;
            }
        }

        for key in empty_keys {
            self.entries.remove(&key);
        }

        removed
    }

    pub fn mark_node_orphaned(&mut self, node_id: NodeId, orphaned: bool) {
        for ((entry_node_id, _), state) in &mut self.entries {
            if *entry_node_id == node_id {
                state.orphaned = orphaned;
                for record in &mut state.history {
                    record.orphaned = orphaned;
                }
            }
        }
    }

    pub fn to_file(&self) -> ArtifactIndexFile {
        let mut artifacts = self
            .entries
            .iter()
            .map(|((node_id, output_key), state)| ArtifactEntryFile {
                node_id: node_id.0,
                output_key: output_key.clone(),
                selected_artifact_id: state.selected_artifact_id.clone(),
                orphaned: state.orphaned,
                history: state
                    .history
                    .iter()
                    .cloned()
                    .map(ArtifactRecordFile::from)
                    .collect(),
            })
            .collect::<Vec<_>>();

        artifacts.sort_by(|a, b| {
            a.node_id
                .cmp(&b.node_id)
                .then_with(|| a.output_key.cmp(&b.output_key))
        });

        ArtifactIndexFile {
            version: "1.0".into(),
            artifacts,
        }
    }

    pub fn from_file(file: ArtifactIndexFile) -> Result<Self, ArtifactError> {
        let mut entries = HashMap::new();

        for entry in file.artifacts {
            let key = (NodeId(entry.node_id), entry.output_key.clone());
            let history = entry
                .history
                .into_iter()
                .map(ArtifactRecord::try_from)
                .collect::<Result<Vec<_>, _>>()?;

            entries.insert(
                key,
                OutputArtifactState {
                    selected_artifact_id: entry.selected_artifact_id,
                    orphaned: entry.orphaned,
                    history,
                },
            );
        }

        Ok(Self { entries })
    }
}

impl From<ArtifactRecord> for ArtifactRecordFile {
    fn from(value: ArtifactRecord) -> Self {
        Self::from(&value)
    }
}

impl From<&ArtifactRecord> for ArtifactRecordFile {
    fn from(value: &ArtifactRecord) -> Self {
        Self {
            artifact_id: value.artifact_id.clone(),
            node_id: value.node_id.0,
            output_key: value.output_key.clone(),
            version: value.version,
            created_at_unix_ms: system_time_to_unix_ms(value.created_at),
            data_type: value.data_type.to_string(),
            format: value.format.clone(),
            path: value.path.to_string_lossy().into_owned(),
            param_signature: value.param_signature.clone(),
            input_signature: value.input_signature.clone(),
            kind: match value.kind {
                ArtifactKind::Restorable => ArtifactHistoryFile::Restorable,
                ArtifactKind::Exported => ArtifactHistoryFile::Exported,
            },
            orphaned: value.orphaned,
        }
    }
}

impl TryFrom<ArtifactRecordFile> for ArtifactRecord {
    type Error = ArtifactError;

    fn try_from(value: ArtifactRecordFile) -> Result<Self, Self::Error> {
        Ok(Self {
            artifact_id: value.artifact_id,
            node_id: NodeId(value.node_id),
            output_key: value.output_key,
            version: value.version,
            created_at: unix_ms_to_system_time(value.created_at_unix_ms)?,
            data_type: DataType(value.data_type),
            format: value.format,
            path: PathBuf::from(value.path),
            param_signature: value.param_signature,
            input_signature: value.input_signature,
            kind: match value.kind {
                ArtifactHistoryFile::Restorable => ArtifactKind::Restorable,
                ArtifactHistoryFile::Exported => ArtifactKind::Exported,
            },
            orphaned: value.orphaned,
        })
    }
}

fn system_time_to_unix_ms(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

fn unix_ms_to_system_time(unix_ms: u128) -> Result<SystemTime, ArtifactError> {
    let millis = u64::try_from(unix_ms).map_err(|_| ArtifactError::DeserializationFailed {
        message: format!("created_at_unix_ms overflow: {unix_ms}"),
    })?;

    Ok(UNIX_EPOCH + Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(version: u64, artifact_id: &str) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: artifact_id.into(),
            node_id: NodeId(42),
            output_key: "image".into(),
            version,
            created_at: UNIX_EPOCH + Duration::from_secs(version),
            data_type: DataType::image(),
            format: "png".into(),
            path: PathBuf::from(format!("artifacts/42/image/{version:06}.png")),
            param_signature: format!("param-{version}"),
            input_signature: format!("input-{version}"),
            kind: ArtifactKind::Restorable,
            orphaned: false,
        }
    }

    #[test]
    fn test_allocate_version_increments_per_output() {
        let mut index = ArtifactIndex::new();
        assert_eq!(index.allocate_version(NodeId(42), "image"), 1);

        index.upsert(sample_record(1, "art_001"), true);
        index.upsert(sample_record(2, "art_002"), false);

        assert_eq!(index.allocate_version(NodeId(42), "image"), 3);
        assert_eq!(index.allocate_version(NodeId(42), "mask"), 1);
    }

    #[test]
    fn test_select_updates_selected_artifact() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record(1, "art_001"), true);
        index.upsert(sample_record(2, "art_002"), false);

        index.select("art_002").expect("select artifact");

        let selected = index
            .get_selected(NodeId(42), "image")
            .expect("selected artifact");
        assert_eq!(selected.artifact_id, "art_002");
    }

    #[test]
    fn test_to_file_and_from_file_roundtrip() {
        let mut index = ArtifactIndex::new();
        index.upsert(sample_record(1, "art_001"), true);
        index.upsert(sample_record(2, "art_002"), false);

        let file = index.to_file();
        let decoded = ArtifactIndex::from_file(file).expect("decode index file");

        assert_eq!(decoded.list(NodeId(42), "image").len(), 2);
        assert_eq!(
            decoded
                .get_selected(NodeId(42), "image")
                .expect("selected artifact")
                .artifact_id,
            "art_001"
        );
    }
}
