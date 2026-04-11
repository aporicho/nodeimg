use crate::node_manager::NodeDef;
use crate::node_manager::NodeManager;

impl NodeManager {
    pub fn get_node_def(&self, type_id: &str) -> Option<&NodeDef> {
        self.nodes().get(type_id)
    }
}
