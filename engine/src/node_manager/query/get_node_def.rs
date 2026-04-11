use crate::node_manager::NodeManager;
use crate::registry::NodeDef;

impl NodeManager {
    pub fn get(&self, type_id: &str) -> Option<&NodeDef> {
        self.nodes().get(type_id)
    }

    pub fn get_node_def(&self, type_id: &str) -> Option<&NodeDef> {
        self.get(type_id)
    }
}
