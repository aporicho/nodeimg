use std::sync::Arc;

use types::{NodeId, Value};

use crate::cache::model::{CacheError, CacheKey, ExecSignature, GenerationId, ResultEntry};

use super::{bytes::estimate_bytes, lookup::ResultStore};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteResult {
    Written,
    DroppedByGeneration,
}

impl ResultStore {
    pub fn put(
        &self,
        node_id: NodeId,
        output_pin: impl Into<String>,
        exec_signature: ExecSignature,
        current_generation: GenerationId,
        write_generation: GenerationId,
        value: Value,
    ) -> Result<WriteResult, CacheError> {
        let output_pin = output_pin.into();

        if current_generation != write_generation {
            return Ok(WriteResult::DroppedByGeneration);
        }

        let key = CacheKey::new(node_id, output_pin, exec_signature);
        let bytes = estimate_bytes(&value);
        let entry = ResultEntry::new(write_generation, Arc::new(value), bytes);

        let _ = self.insert(key, entry);

        Ok(WriteResult::Written)
    }
}

#[cfg(test)]
mod tests {
    use types::{NodeId, Value};

    use crate::cache::model::{ExecSignature, GenerationId};

    use super::{ResultStore, WriteResult};

    #[test]
    fn put_writes_when_generation_matches() {
        let store = ResultStore::new();
        let result = store.put(
            NodeId(1),
            "image",
            ExecSignature::new(1, 1, 10, 20),
            GenerationId::initial(),
            GenerationId::initial(),
            Value::Int(7),
        );

        assert_eq!(result, Ok(WriteResult::Written));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn put_drops_when_generation_mismatches() {
        let store = ResultStore::new();
        let result = store.put(
            NodeId(1),
            "image",
            ExecSignature::new(1, 1, 10, 20),
            GenerationId(2),
            GenerationId(1),
            Value::Int(7),
        );

        assert_eq!(result, Ok(WriteResult::DroppedByGeneration));
        assert!(store.is_empty());
    }
}
