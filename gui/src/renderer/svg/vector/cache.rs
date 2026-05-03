use std::collections::HashMap;
use std::sync::Arc;

use super::super::{SvgError, SvgSource, SvgSourceKey};
use super::model::SvgVectorDocument;
use super::parse::parse_svg_vector;

#[derive(Default)]
pub(crate) struct SvgVectorCache {
    cache: HashMap<SvgSourceKey, Result<Arc<SvgVectorDocument>, SvgError>>,
}

impl SvgVectorCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn get_or_parse(
        &mut self,
        source: &SvgSource,
    ) -> Result<Arc<SvgVectorDocument>, SvgError> {
        if let Some(entry) = self.cache.get(source.key()) {
            return entry.clone();
        }

        let parsed = parse_svg_vector(source).map(Arc::new);
        self.cache.insert(source.key().clone(), parsed.clone());
        parsed
    }
}
