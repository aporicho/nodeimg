use crate::node_manager::NodeDef;
use crate::node_manager::NodeManager;

impl NodeManager {
    pub fn list_node_defs(&self) -> Vec<&NodeDef> {
        self.nodes().values().collect()
    }
}
