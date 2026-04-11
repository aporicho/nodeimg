use crate::node_manager::NodeDef;
use crate::node_manager::NodeManager;

impl NodeManager {
    pub fn search_nodes(&self, query: &str) -> Vec<&NodeDef> {
        crate::node_manager::index::defs_by_search::search_nodes(self.nodes().values(), query)
    }
}
