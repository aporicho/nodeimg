use crate::node_manager::NodeManager;
use crate::registry::NodeDef;

impl NodeManager {
    pub fn list_node_defs(&self) -> Vec<&NodeDef> {
        self.nodes().values().collect()
    }
}
