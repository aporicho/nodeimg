use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SvgSourceKey {
    id: String,
    data_hash: u64,
}

impl SvgSourceKey {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SvgSource {
    key: SvgSourceKey,
    bytes: Arc<[u8]>,
}

impl SvgSource {
    pub(crate) fn new(id: impl Into<String>, bytes: Arc<[u8]>) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        bytes.hash(&mut hasher);
        let data_hash = hasher.finish();
        Self {
            key: SvgSourceKey {
                id: id.into(),
                data_hash,
            },
            bytes,
        }
    }

    pub(crate) fn key(&self) -> &SvgSourceKey {
        &self.key
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
