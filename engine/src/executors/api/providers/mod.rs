use std::sync::Arc;

use crate::node_manager::NodeDef;

use super::provider::Provider;

pub mod libtv;

pub fn builtin_providers() -> Vec<Arc<dyn Provider>> {
    vec![Arc::new(libtv::LibTvProvider::new())]
}

pub fn builtin_nodes() -> Vec<NodeDef> {
    builtin_providers()
        .into_iter()
        .flat_map(|provider| provider.nodes())
        .collect()
}
