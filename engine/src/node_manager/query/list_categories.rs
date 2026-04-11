use crate::node_manager::NodeManager;

impl NodeManager {
    pub fn list_categories(&self) -> Vec<String> {
        crate::node_manager::index::defs_by_category::list_categories(self.nodes().values())
    }
}
