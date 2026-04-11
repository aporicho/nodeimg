use std::sync::Arc;
use std::time::Instant;

use types::Value;

use super::GenerationId;

#[derive(Clone, Debug)]
pub struct ResultEntry {
    pub generation: GenerationId,
    pub value: Arc<Value>,
    pub bytes: usize,
    pub updated_at: Instant,
}

impl ResultEntry {
    pub fn new(generation: GenerationId, value: Arc<Value>, bytes: usize) -> Self {
        Self {
            generation,
            value,
            bytes,
            updated_at: Instant::now(),
        }
    }
}
