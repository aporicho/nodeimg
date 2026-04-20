use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use super::record::ArtifactId;

#[derive(Debug)]
pub enum ArtifactError {
    ArtifactNotFound { artifact_id: ArtifactId },
    SelectedArtifactNotFound { artifact_id: ArtifactId },
    SelectedArtifactDeleteForbidden { artifact_id: ArtifactId },
    SignatureMismatch { artifact_id: ArtifactId },
    HandlerNotRegistered { data_type: String },
    UnsupportedDataType { data_type: String },
    IndexCorrupted { message: String },
    StorageIo { path: PathBuf, message: String },
    SerializationFailed { message: String },
    DeserializationFailed { message: String },
}

impl Display for ArtifactError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArtifactNotFound { artifact_id } => {
                write!(f, "artifact not found: {artifact_id}")
            }
            Self::SelectedArtifactNotFound { artifact_id } => {
                write!(f, "selected artifact not found: {artifact_id}")
            }
            Self::SelectedArtifactDeleteForbidden { artifact_id } => {
                write!(f, "cannot delete selected artifact: {artifact_id}")
            }
            Self::SignatureMismatch { artifact_id } => {
                write!(f, "artifact signature mismatch: {artifact_id}")
            }
            Self::HandlerNotRegistered { data_type } => {
                write!(
                    f,
                    "artifact handler not registered for data type: {data_type}"
                )
            }
            Self::UnsupportedDataType { data_type } => {
                write!(f, "unsupported artifact data type: {data_type}")
            }
            Self::IndexCorrupted { message } => {
                write!(f, "artifact index corrupted: {message}")
            }
            Self::StorageIo { path, message } => {
                write!(
                    f,
                    "artifact storage io failed at {}: {message}",
                    path.display()
                )
            }
            Self::SerializationFailed { message } => {
                write!(f, "artifact serialization failed: {message}")
            }
            Self::DeserializationFailed { message } => {
                write!(f, "artifact deserialization failed: {message}")
            }
        }
    }
}

impl Error for ArtifactError {}
