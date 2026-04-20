use std::path::Path;

use crate::artifact::model::ArtifactError;
use types::{DataType, Value};

pub trait ArtifactHandler: Send + Sync {
    fn data_type(&self) -> DataType;
    fn extension(&self) -> &'static str;
    fn serialize(&self, value: &Value, path: &Path) -> Result<(), ArtifactError>;
    fn deserialize(&self, path: &Path) -> Result<Value, ArtifactError>;
}
