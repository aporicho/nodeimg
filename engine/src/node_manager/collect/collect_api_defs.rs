use crate::node_manager::NodeDef;

pub fn collect_api_defs() -> Vec<NodeDef> {
    crate::executors::api::providers::builtin_nodes()
}
