use crate::node_manager::NodeDef;
use crate::node_manager::NodeManager;

impl NodeManager {
    pub fn list_nodes_by_category(&self, category: &str) -> Vec<&NodeDef> {
        crate::node_manager::index::defs_by_category::list_nodes_by_category(
            self.nodes().values(),
            category,
        )
    }
}
