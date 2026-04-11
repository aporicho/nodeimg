use crate::node_manager::NodeManager;
use crate::registry::NodeDef;

impl NodeManager {
    pub fn search(&self, query: &str) -> Vec<&NodeDef> {
        crate::node_manager::index::defs_by_search::search_nodes(self.nodes().values(), query)
    }
}
